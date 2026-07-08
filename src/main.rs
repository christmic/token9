mod config;
mod error;
mod metering;
mod proxy;
mod router;
mod store;

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::store::sqlite::SqliteStore;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<SqliteStore>,
    pub http: reqwest::Client,
}

#[derive(Parser, Debug)]
#[command(name = "token9", about = "Local LLM API router & token meter")]
struct Args {
    /// Path to config.toml
    #[arg(long, default_value = "~/.Oraculo/config/token9/config.toml")]
    config: String,
    /// Override the listen port
    #[arg(long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let mut config = Config::load(&args.config)?;
    if let Some(port) = args.port {
        config.port = port;
    }

    let store = Arc::new(SqliteStore::open(&config.db_path).await?);
    let http = reqwest::Client::builder().build()?;

    let bind = config.bind.clone();
    let port = config.port;
    let state = AppState {
        config: Arc::new(config),
        store,
        http,
    };

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/stats/summary", get(stats_summary))
        .fallback(proxy::proxy)
        .with_state(state);

    let addr = format!("{bind}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(%addr, "token9 listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn stats_summary(State(state): State<AppState>) -> Result<Json<serde_json::Value>, error::AppError> {
    let rows = state.store.summary().await.map_err(|e| {
        error::AppError::new(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    Ok(Json(serde_json::json!({ "by_model": rows })))
}
