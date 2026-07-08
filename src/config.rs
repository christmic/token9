use std::collections::HashMap;

use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_db")]
    pub db_path: String,
    #[serde(default)]
    pub providers: HashMap<String, Provider>,
    #[serde(default)]
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Provider {
    pub base_url: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    pub dialect: Dialect,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    pub model_id: String,
    pub provider: String,
    /// Real upstream model name. Defaults to `model_id` (no body rewrite).
    #[serde(default)]
    pub real_model: Option<String>,
    /// Opt-in: inject `stream_options.include_usage` for OpenAI Chat streaming.
    #[serde(default)]
    pub inject_usage: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dialect {
    Anthropic,
    OpenaiChat,
    OpenaiResponses,
}

impl Dialect {
    pub fn as_str(self) -> &'static str {
        match self {
            Dialect::Anthropic => "anthropic",
            Dialect::OpenaiChat => "openai_chat",
            Dialect::OpenaiResponses => "openai_responses",
        }
    }

    /// True for OpenAI-family dialects (Authorization: Bearer auth).
    pub fn is_openai(self) -> bool {
        matches!(self, Dialect::OpenaiChat | Dialect::OpenaiResponses)
    }
}

fn default_port() -> u16 {
    9527
}
fn default_bind() -> String {
    "127.0.0.1".to_string()
}
fn default_db() -> String {
    "~/.Oraculo/config/token9/token9.db".to_string()
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let path = expand_tilde(path);
        let cfg: Config = Figment::new()
            .merge(Toml::file(&path))
            .merge(Env::prefixed("TOKEN9_"))
            .extract()?;
        Ok(cfg)
    }
}

/// Expand a leading `~` to the user's home directory.
pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}
