use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use bytes::Bytes;
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::AppState;
use crate::error::AppError;
use crate::metering::{self, MeterMeta};
use crate::router::Target;
use crate::store::RequestRow;

/// Headers we never forward as-is (transport / auth are handled explicitly).
fn is_hop_or_auth(name: &str) -> bool {
    matches!(
        name,
        "host" | "content-length" | "connection" | "x-api-key" | "authorization"
    )
}

/// Generic transparent forwarding handler.
/// Reads `model` to route, swaps endpoint URL + credential, forwards the body
/// (verbatim unless a mutation is configured), and tees the response off-path for metering.
pub async fn proxy(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    let start = Instant::now();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let id = uuid::Uuid::now_v7().to_string();

    // Parse just enough to read the routing key. Reading != modifying (§1.5 #2).
    let json: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|_| AppError::bad_request("request body is not valid JSON"))?;
    let model_id = json
        .get("model")
        .and_then(|m| m.as_str())
        .ok_or_else(|| AppError::bad_request("request body has no `model` field"))?
        .to_string();

    let target = {
        let rt = state.routes.read().await;
        rt.resolve(&model_id)
    }
    .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, format!("no route for model `{model_id}`")))?;

    let stream = json.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    // Build the outbound body. Verbatim unless a mutation is actually required.
    let inject = target.inject_usage
        && matches!(target.dialect, crate::config::Dialect::OpenaiChat)
        && stream;
    let out_body: Bytes = if target.rewrite_model || inject {
        let mut j = json.clone();
        if target.rewrite_model {
            j["model"] = serde_json::Value::String(target.real_model.clone());
        }
        if inject {
            let so = j
                .as_object_mut()
                .and_then(|o| o.entry("stream_options").or_insert(serde_json::json!({})).as_object_mut());
            if let Some(so) = so {
                so.insert("include_usage".into(), serde_json::Value::Bool(true));
            }
        }
        Bytes::from(serde_json::to_vec(&j).map_err(|e| {
            AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("serialize error: {e}"))
        })?)
    } else {
        body.clone()
    };

    // Upstream URL: real base_url + original path (+ query).
    let path = uri.path();
    let upstream_url = match uri.query() {
        Some(q) => format!("{}{}?{}", target.base_url, path, q),
        None => format!("{}{}", target.base_url, path),
    };

    // Build the upstream request with header passthrough + credential swap.
    let mut req = state.http.request(method, upstream_url);
    for (k, v) in headers.iter() {
        if is_hop_or_auth(k.as_str()) {
            continue;
        }
        req = req.header(k.clone(), v.clone());
    }
    if let Some(token) = &target.token {
        if target.dialect.is_openai() {
            req = req.header("authorization", format!("Bearer {token}"));
        } else {
            req = req.header("x-api-key", token.clone());
        }
    }
    req = req.body(out_body);

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            record_error(&state, &id, ts, start, &target, &model_id, stream, &e.to_string());
            return Err(AppError::bad_gateway(format!("upstream request failed: {e}")));
        }
    };

    let status = resp.status();
    let resp_headers = resp.headers().clone();
    let upstream = resp.bytes_stream();

    // Off-path metering: tee a cheap clone of each chunk into an async task (§5.1).
    let (tx, rx) = mpsc::unbounded_channel::<Bytes>();
    let meta = MeterMeta {
        id,
        ts,
        start,
        dialect: target.dialect,
        model_id,
        provider: target.provider.clone(),
        real_model: target.real_model.clone(),
        stream,
        status: status.as_u16() as i64,
    };
    tokio::spawn(metering::run(rx, meta, state.store.clone()));

    let teed = upstream.map(move |item| {
        if let Ok(ref chunk) = item {
            let _ = tx.send(chunk.clone());
        }
        item
    });

    // Build the client response: same status, passthrough headers (minus length/encoding framing).
    let mut response = Response::builder()
        .status(status)
        .body(Body::from_stream(teed))
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let out_headers = response.headers_mut();
    for (k, v) in resp_headers.iter() {
        let name = k.as_str();
        if name == "content-length" || name == "transfer-encoding" {
            continue;
        }
        out_headers.insert(k.clone(), v.clone());
    }

    Ok(response)
}

#[allow(clippy::too_many_arguments)]
fn record_error(
    state: &AppState,
    id: &str,
    ts: i64,
    start: Instant,
    target: &Target,
    model_id: &str,
    stream: bool,
    err: &str,
) {
    let row = RequestRow {
        id: id.to_string(),
        ts,
        client_protocol: target.dialect.as_str().to_string(),
        model_id: model_id.to_string(),
        provider: target.provider.clone(),
        real_model: target.real_model.clone(),
        stream,
        status: None,
        input_tokens: 0,
        output_tokens: 0,
        cache_write_tokens: 0,
        cache_read_tokens: 0,
        latency_ms: Some(start.elapsed().as_millis() as i64),
        ttft_ms: None,
        error: Some(err.to_string()),
    };
    let store = state.store.clone();
    tokio::spawn(async move {
        let _ = store.record(row).await;
    });
}
