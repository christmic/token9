pub mod sqlite;

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
