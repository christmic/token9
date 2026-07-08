pub mod config_store;
pub mod sqlite;

use crate::config::Dialect;

/// One metered request. Written off the forwarding path.
#[derive(Debug, Clone)]
pub struct RequestRow {
    pub id: String,
    pub ts: i64,
    pub client_protocol: String,
    pub model_id: String,
    pub provider: String,
    pub real_model: String,
    pub stream: bool,
    pub status: Option<i64>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_write_tokens: i64,
    pub cache_read_tokens: i64,
    pub latency_ms: Option<i64>,
    pub ttft_ms: Option<i64>,
    pub error: Option<String>,
}

/// A provider row as stored (api_key kept as-is; encryption is a future ConfigStore concern).
#[derive(Debug, Clone)]
pub struct ProviderRow {
    pub name: String,
    pub base_url: String,
    pub dialect: String,
    pub api_key: Option<String>,
}

/// A logical-model row joined with its provider name.
#[derive(Debug, Clone)]
pub struct ModelRow {
    pub model_id: String,
    pub provider: String,
    pub real_model: String,
    pub inject_usage: bool,
}

/// A fully-resolved route: logical model joined with its provider's connection details.
#[derive(Debug, Clone)]
pub struct ResolvedRoute {
    pub model_id: String,
    pub provider: String,
    pub base_url: String,
    pub dialect: Dialect,
    pub real_model: String,
    pub inject_usage: bool,
    pub api_key: Option<String>,
}

/// One aggregated stats bucket (provider + model + day).
#[derive(Debug, Clone)]
pub struct StatBucket {
    pub provider: String,
    pub real_model: String,
    pub date: String,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
}
