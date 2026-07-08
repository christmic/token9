use crate::config::{Config, Dialect};

/// Resolved forwarding target for a request.
#[derive(Debug, Clone)]
pub struct Target {
    pub provider: String,
    pub base_url: String,
    pub token: Option<String>,
    pub real_model: String,
    /// True when `real_model != model_id`, i.e. the body `model` field must be rewritten.
    pub rewrite_model: bool,
    pub dialect: Dialect,
    pub inject_usage: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error("no route for model_id `{0}`")]
    NoRoute(String),
    #[error("route `{0}` points to unknown provider `{1}`")]
    UnknownProvider(String, String),
}

/// Resolve a logical `model_id` to a forwarding target.
/// Reads (never mutates) the incoming model to pick provider/endpoint/credential.
pub fn resolve(config: &Config, model_id: &str) -> Result<Target, RouteError> {
    let route = config
        .routes
        .iter()
        .find(|r| r.model_id == model_id)
        .ok_or_else(|| RouteError::NoRoute(model_id.to_string()))?;

    let provider = config
        .providers
        .get(&route.provider)
        .ok_or_else(|| RouteError::UnknownProvider(route.model_id.clone(), route.provider.clone()))?;

    let real_model = route.real_model.clone().unwrap_or_else(|| model_id.to_string());
    let rewrite_model = real_model != model_id;

    let token = provider
        .api_key_env
        .as_ref()
        .and_then(|env| std::env::var(env).ok());

    Ok(Target {
        provider: route.provider.clone(),
        base_url: provider.base_url.trim_end_matches('/').to_string(),
        token,
        real_model,
        rewrite_model,
        dialect: provider.dialect,
        inject_usage: route.inject_usage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Provider, Route};
    use std::collections::HashMap;

    fn config() -> Config {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            Provider {
                base_url: "https://api.anthropic.com/".to_string(),
                api_key_env: None,
                dialect: Dialect::Anthropic,
            },
        );
        providers.insert(
            "deepseek".to_string(),
            Provider {
                base_url: "https://api.deepseek.com".to_string(),
                api_key_env: None,
                dialect: Dialect::OpenaiChat,
            },
        );
        Config {
            port: 9527,
            bind: "127.0.0.1".to_string(),
            db_path: ":memory:".to_string(),
            providers,
            routes: vec![
                Route {
                    model_id: "claude-opus-4-6".to_string(),
                    provider: "anthropic".to_string(),
                    real_model: None,
                    inject_usage: false,
                },
                Route {
                    model_id: "my-cheap-coder".to_string(),
                    provider: "deepseek".to_string(),
                    real_model: Some("deepseek-chat".to_string()),
                    inject_usage: true,
                },
                Route {
                    model_id: "orphan".to_string(),
                    provider: "ghost".to_string(),
                    real_model: None,
                    inject_usage: false,
                },
            ],
        }
    }

    #[test]
    fn passthrough_no_rewrite() {
        let t = resolve(&config(), "claude-opus-4-6").unwrap();
        assert_eq!(t.provider, "anthropic");
        assert_eq!(t.base_url, "https://api.anthropic.com"); // trailing slash trimmed
        assert_eq!(t.real_model, "claude-opus-4-6");
        assert!(!t.rewrite_model);
        assert_eq!(t.dialect, Dialect::Anthropic);
    }

    #[test]
    fn alias_triggers_rewrite() {
        let t = resolve(&config(), "my-cheap-coder").unwrap();
        assert_eq!(t.provider, "deepseek");
        assert_eq!(t.real_model, "deepseek-chat");
        assert!(t.rewrite_model);
        assert!(t.inject_usage);
        assert!(t.dialect.is_openai());
    }

    #[test]
    fn no_route() {
        assert!(matches!(
            resolve(&config(), "unknown"),
            Err(RouteError::NoRoute(_))
        ));
    }

    #[test]
    fn unknown_provider() {
        assert!(matches!(
            resolve(&config(), "orphan"),
            Err(RouteError::UnknownProvider(..))
        ));
    }
}
