use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use crate::AppState;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// Inclusive lower bound, YYYY-MM-DD.
    pub from: Option<String>,
    /// Inclusive upper bound, YYYY-MM-DD.
    pub to: Option<String>,
}

/// GET /stats/summary — usage grouped by provider + model + date.
/// Dimensions: provider, model, requests, input/output tokens, cache_ratio, date.
pub async fn summary(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let buckets = state
        .store
        .stats(q.from.as_deref(), q.to.as_deref())
        .await
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows: Vec<serde_json::Value> = buckets
        .into_iter()
        .map(|b| {
            let denom = b.input_tokens + b.cache_read_tokens;
            let cache_ratio = if denom > 0 {
                b.cache_read_tokens as f64 / denom as f64
            } else {
                0.0
            };
            serde_json::json!({
                "provider": b.provider,
                "model": b.real_model,
                "date": b.date,
                "requests": b.requests,
                "input_tokens": b.input_tokens,
                "output_tokens": b.output_tokens,
                "cache_read_tokens": b.cache_read_tokens,
                "cache_write_tokens": b.cache_write_tokens,
                "cache_ratio": (cache_ratio * 10000.0).round() / 10000.0,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "buckets": rows })))
}
