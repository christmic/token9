use crate::config::Dialect;
use crate::store::ResolvedRoute;

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

impl Target {
    /// Build a target from a DB-resolved route for the given logical model id.
    pub fn from_route(model_id: &str, route: &ResolvedRoute) -> Self {
        Target {
            provider: route.provider.clone(),
            base_url: route.base_url.trim_end_matches('/').to_string(),
            token: route.api_key.clone(),
            real_model: route.real_model.clone(),
            rewrite_model: route.real_model != model_id,
            dialect: route.dialect,
            inject_usage: route.inject_usage,
        }
    }
}
