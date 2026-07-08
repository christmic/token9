use std::collections::HashMap;

use crate::router::Target;
use crate::store::sqlite::SqliteStore;

/// In-memory routing cache built from the DB. Read on the hot path.
#[derive(Debug, Default, Clone)]
pub struct RouteTable {
    map: HashMap<String, Target>,
}

impl RouteTable {
    /// Rebuild the cache from persisted routes.
    pub async fn load(store: &SqliteStore) -> anyhow::Result<Self> {
        let routes = store.load_routes().await?;
        let mut map = HashMap::with_capacity(routes.len());
        for route in &routes {
            map.insert(route.model_id.clone(), Target::from_route(&route.model_id, route));
        }
        Ok(RouteTable { map })
    }

    /// Resolve a logical model id to its target. `None` when unknown.
    pub fn resolve(&self, model_id: &str) -> Option<Target> {
        self.map.get(model_id).cloned()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Dialect;

    async fn store_with_routes() -> SqliteStore {
        let s = SqliteStore::open(":memory:").await.unwrap();
        s.add_provider("anthropic", "https://api.anthropic.com/", Dialect::Anthropic, Some("sk-a"))
            .await
            .unwrap();
        s.add_provider("deepseek", "https://api.deepseek.com", Dialect::OpenaiChat, Some("sk-d"))
            .await
            .unwrap();
        s.add_model("claude-opus-4-6", "anthropic", "claude-opus-4-6", false)
            .await
            .unwrap();
        s.add_model("my-cheap-coder", "deepseek", "deepseek-chat", true)
            .await
            .unwrap();
        s
    }

    #[tokio::test]
    async fn passthrough_no_rewrite() {
        let s = store_with_routes().await;
        let rt = RouteTable::load(&s).await.unwrap();
        let t = rt.resolve("claude-opus-4-6").unwrap();
        assert_eq!(t.provider, "anthropic");
        assert_eq!(t.base_url, "https://api.anthropic.com"); // trailing slash trimmed
        assert_eq!(t.real_model, "claude-opus-4-6");
        assert!(!t.rewrite_model);
        assert_eq!(t.token.as_deref(), Some("sk-a"));
        assert_eq!(t.dialect, Dialect::Anthropic);
    }

    #[tokio::test]
    async fn alias_triggers_rewrite() {
        let s = store_with_routes().await;
        let rt = RouteTable::load(&s).await.unwrap();
        let t = rt.resolve("my-cheap-coder").unwrap();
        assert_eq!(t.real_model, "deepseek-chat");
        assert!(t.rewrite_model);
        assert!(t.inject_usage);
        assert!(t.dialect.is_openai());
        assert_eq!(t.token.as_deref(), Some("sk-d"));
    }

    #[tokio::test]
    async fn unknown_model_is_none() {
        let s = store_with_routes().await;
        let rt = RouteTable::load(&s).await.unwrap();
        assert!(rt.resolve("nope").is_none());
    }
}
