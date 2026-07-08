use std::str::FromStr;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;

use crate::AppState;
use crate::config::Dialect;
use crate::error::AppError;
use crate::routetable::RouteTable;

/// Rebuild the in-memory route cache from the DB. Called after every mutation.
async fn reload(state: &AppState) -> Result<usize, AppError> {
    let rt = RouteTable::load(&state.store)
        .await
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let n = rt.len();
    *state.routes.write().await = rt;
    Ok(n)
}

/// Mask a secret for display: keep last 4 chars.
fn mask(key: &Option<String>) -> serde_json::Value {
    match key {
        Some(k) if k.len() > 4 => serde_json::Value::String(format!("****{}", &k[k.len() - 4..])),
        Some(_) => serde_json::Value::String("****".into()),
        None => serde_json::Value::Null,
    }
}

// ---- providers ----

#[derive(Debug, Deserialize)]
pub struct ProviderInput {
    pub name: String,
    pub base_url: String,
    pub dialect: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

pub async fn list_providers(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ps = state
        .store
        .list_providers()
        .await
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let rows: Vec<_> = ps
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "base_url": p.base_url,
                "dialect": p.dialect,
                "api_key": mask(&p.api_key),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "providers": rows })))
}

pub async fn create_provider(
    State(state): State<AppState>,
    Json(input): Json<ProviderInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dialect = Dialect::from_str(&input.dialect).map_err(AppError::bad_request)?;
    state
        .store
        .add_provider(&input.name, &input.base_url, dialect, input.api_key.as_deref())
        .await
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let routes = reload(&state).await?;
    Ok(Json(serde_json::json!({ "ok": true, "routes": routes })))
}

pub async fn delete_provider(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let removed = state
        .store
        .remove_provider(&name)
        .await
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let routes = reload(&state).await?;
    Ok(Json(serde_json::json!({ "removed": removed, "routes": routes })))
}

// ---- models ----

#[derive(Debug, Deserialize)]
pub struct ModelInput {
    pub model_id: String,
    pub provider: String,
    pub real_model: String,
    #[serde(default)]
    pub inject_usage: bool,
}

pub async fn list_models(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ms = state
        .store
        .list_models()
        .await
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let rows: Vec<_> = ms
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "model_id": m.model_id,
                "provider": m.provider,
                "real_model": m.real_model,
                "inject_usage": m.inject_usage,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "models": rows })))
}

pub async fn create_model(
    State(state): State<AppState>,
    Json(input): Json<ModelInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .store
        .add_model(&input.model_id, &input.provider, &input.real_model, input.inject_usage)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let routes = reload(&state).await?;
    Ok(Json(serde_json::json!({ "ok": true, "routes": routes })))
}

pub async fn delete_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let removed = state
        .store
        .remove_model(&model_id)
        .await
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let routes = reload(&state).await?;
    Ok(Json(serde_json::json!({ "removed": removed, "routes": routes })))
}

// ---- reload ----

pub async fn reload_routes(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let routes = reload(&state).await?;
    Ok(Json(serde_json::json!({ "ok": true, "routes": routes })))
}
