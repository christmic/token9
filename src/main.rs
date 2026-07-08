mod admin;
mod cli;
mod config;
mod error;
mod hosts;
mod metering;
mod proxy;
mod router;
mod routetable;
mod stats;
mod store;

use std::sync::Arc;

use axum::Router;
use axum::routing::{delete, get, post};
use clap::Parser;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Command};
use crate::config::Config;
use crate::routetable::RouteTable;
use crate::store::sqlite::SqliteStore;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<SqliteStore>,
    pub routes: Arc<RwLock<RouteTable>>,
    pub http: reqwest::Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();
    let mut config = Config::load(&cli.config)?;
    if let Some(port) = cli.port {
        config.port = port;
    }

    match cli.command {
        None | Some(Command::Serve) => serve(config).await,
        Some(Command::Provider { action }) => {
            let store = SqliteStore::open(&config.db_path).await?;
            cli::run_provider(&store, action).await
        }
        Some(Command::Model { action }) => {
            let store = SqliteStore::open(&config.db_path).await?;
            cli::run_model(&store, action).await
        }
        Some(Command::Hosts { action }) => cli::run_hosts(&config.domain, action),
    }
}

async fn serve(config: Config) -> anyhow::Result<()> {
    let store = Arc::new(SqliteStore::open(&config.db_path).await?);
    let routes = RouteTable::load(&store).await?;
    info!(count = routes.len(), "loaded routes");
    let http = reqwest::Client::builder().build()?;

    let bind = config.bind.clone();
    let port = config.port;
    let domain = config.domain.clone();
    let state = AppState {
        config: Arc::new(config),
        store,
        routes: Arc::new(RwLock::new(routes)),
        http,
    };

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/stats/summary", get(stats::summary))
        .route("/admin/providers", get(admin::list_providers).post(admin::create_provider))
        .route("/admin/providers/{name}", delete(admin::delete_provider))
        .route("/admin/models", get(admin::list_models).post(admin::create_model))
        .route("/admin/models/{model_id}", delete(admin::delete_model))
        .route("/admin/reload", post(admin::reload_routes))
        .fallback(proxy::proxy)
        .with_state(state);

    let addr = format!("{bind}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(%addr, %domain, "token9 listening (try http://{domain}:{port})");
    axum::serve(listener, app).await?;
    Ok(())
}
