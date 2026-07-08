use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::Row;
use tracing::warn;

use super::{ModelRow, ProviderRow, RateLimitRow, ResolvedRoute};
use crate::config::Dialect;
use crate::ratelimit::RateLimitSnapshot;
use crate::store::sqlite::SqliteStore;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Provider + logical-model persistence. Single choke-point for `api_key`
/// (a future encryption layer wraps read/write here without schema churn).
impl SqliteStore {
    // ---- providers ----

    /// Insert or update a provider by name.
    pub async fn add_provider(
        &self,
        name: &str,
        base_url: &str,
        dialect: Dialect,
        api_key: Option<&str>,
    ) -> anyhow::Result<()> {
        let ts = now_ms();
        sqlx::query(
            r#"INSERT INTO providers (name, base_url, dialect, api_key, created_at, updated_at)
               VALUES (?,?,?,?,?,?)
               ON CONFLICT(name) DO UPDATE SET
                 base_url = excluded.base_url,
                 dialect  = excluded.dialect,
                 api_key  = excluded.api_key,
                 updated_at = excluded.updated_at"#,
        )
        .bind(name)
        .bind(base_url)
        .bind(dialect.as_str())
        .bind(api_key)
        .bind(ts)
        .bind(ts)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_providers(&self) -> anyhow::Result<Vec<ProviderRow>> {
        let rows = sqlx::query(
            "SELECT name, base_url, dialect, api_key FROM providers ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ProviderRow {
                name: r.get("name"),
                base_url: r.get("base_url"),
                dialect: r.get("dialect"),
                api_key: r.get("api_key"),
            })
            .collect())
    }

    /// Remove a provider (cascades to its logical models). Returns rows affected.
    pub async fn remove_provider(&self, name: &str) -> anyhow::Result<u64> {
        let res = sqlx::query("DELETE FROM providers WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }

    // ---- logical models ----

    /// Insert or update a logical model. Fails if the provider name is unknown.
    pub async fn add_model(
        &self,
        model_id: &str,
        provider_name: &str,
        real_model: &str,
        inject_usage: bool,
    ) -> anyhow::Result<()> {
        let provider_id: i64 = sqlx::query("SELECT id FROM providers WHERE name = ?")
            .bind(provider_name)
            .fetch_optional(&self.pool)
            .await?
            .map(|r| r.get::<i64, _>("id"))
            .ok_or_else(|| anyhow::anyhow!("unknown provider `{provider_name}`"))?;

        let ts = now_ms();
        sqlx::query(
            r#"INSERT INTO logical_models
               (model_id, provider_id, real_model, inject_usage, created_at, updated_at)
               VALUES (?,?,?,?,?,?)
               ON CONFLICT(model_id) DO UPDATE SET
                 provider_id  = excluded.provider_id,
                 real_model   = excluded.real_model,
                 inject_usage = excluded.inject_usage,
                 updated_at   = excluded.updated_at"#,
        )
        .bind(model_id)
        .bind(provider_id)
        .bind(real_model)
        .bind(inject_usage as i64)
        .bind(ts)
        .bind(ts)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_models(&self) -> anyhow::Result<Vec<ModelRow>> {
        let rows = sqlx::query(
            r#"SELECT m.model_id, p.name AS provider, m.real_model, m.inject_usage
               FROM logical_models m
               JOIN providers p ON p.id = m.provider_id
               ORDER BY m.model_id"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ModelRow {
                model_id: r.get("model_id"),
                provider: r.get("provider"),
                real_model: r.get("real_model"),
                inject_usage: r.get::<i64, _>("inject_usage") != 0,
            })
            .collect())
    }

    pub async fn remove_model(&self, model_id: &str) -> anyhow::Result<u64> {
        let res = sqlx::query("DELETE FROM logical_models WHERE model_id = ?")
            .bind(model_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }

    // ---- routing ----

    /// Load all routes (logical model joined with provider connection details).
    /// Rows with an unparseable dialect are skipped with a warning.
    pub async fn load_routes(&self) -> anyhow::Result<Vec<ResolvedRoute>> {
        let rows = sqlx::query(
            r#"SELECT m.model_id, p.name AS provider, p.base_url, p.dialect,
                      m.real_model, m.inject_usage, p.api_key
               FROM logical_models m
               JOIN providers p ON p.id = m.provider_id"#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let dialect_str: String = r.get("dialect");
            let dialect = match Dialect::from_str(&dialect_str) {
                Ok(d) => d,
                Err(e) => {
                    warn!(model = %r.get::<String, _>("model_id"), error = %e, "skipping route with bad dialect");
                    continue;
                }
            };
            out.push(ResolvedRoute {
                model_id: r.get("model_id"),
                provider: r.get("provider"),
                base_url: r.get("base_url"),
                dialect,
                real_model: r.get("real_model"),
                inject_usage: r.get::<i64, _>("inject_usage") != 0,
                api_key: r.get("api_key"),
            });
        }
        Ok(out)
    }

    // ---- rate limits ----

    /// Upsert the latest rate-limit snapshot for a provider.
    pub async fn upsert_rate_limit(
        &self,
        provider: &str,
        snap: &RateLimitSnapshot,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO provider_rate_limits
               (provider, updated_at, requests_limit, requests_remaining, requests_reset,
                tokens_limit, tokens_remaining, tokens_reset, raw)
               VALUES (?,?,?,?,?,?,?,?,?)
               ON CONFLICT(provider) DO UPDATE SET
                 updated_at         = excluded.updated_at,
                 requests_limit     = excluded.requests_limit,
                 requests_remaining = excluded.requests_remaining,
                 requests_reset     = excluded.requests_reset,
                 tokens_limit       = excluded.tokens_limit,
                 tokens_remaining   = excluded.tokens_remaining,
                 tokens_reset       = excluded.tokens_reset,
                 raw                = excluded.raw"#,
        )
        .bind(provider)
        .bind(now_ms())
        .bind(snap.requests_limit)
        .bind(snap.requests_remaining)
        .bind(snap.requests_reset.as_deref())
        .bind(snap.tokens_limit)
        .bind(snap.tokens_remaining)
        .bind(snap.tokens_reset.as_deref())
        .bind(&snap.raw)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_rate_limits(&self) -> anyhow::Result<Vec<RateLimitRow>> {
        let rows = sqlx::query(
            r#"SELECT provider, updated_at, requests_limit, requests_remaining, requests_reset,
                      tokens_limit, tokens_remaining, tokens_reset
               FROM provider_rate_limits ORDER BY provider"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| RateLimitRow {
                provider: r.get("provider"),
                updated_at: r.get("updated_at"),
                requests_limit: r.get("requests_limit"),
                requests_remaining: r.get("requests_remaining"),
                requests_reset: r.get("requests_reset"),
                tokens_limit: r.get("tokens_limit"),
                tokens_remaining: r.get("tokens_remaining"),
                tokens_reset: r.get("tokens_reset"),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store() -> SqliteStore {
        SqliteStore::open(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn provider_and_model_crud() {
        let s = store().await;
        s.add_provider("anthropic", "https://api.anthropic.com", Dialect::Anthropic, Some("sk-1"))
            .await
            .unwrap();
        s.add_provider("deepseek", "https://api.deepseek.com", Dialect::OpenaiChat, Some("sk-2"))
            .await
            .unwrap();

        let ps = s.list_providers().await.unwrap();
        assert_eq!(ps.len(), 2);

        s.add_model("claude-opus-4-6", "anthropic", "claude-opus-4-6", false)
            .await
            .unwrap();
        s.add_model("my-cheap-coder", "deepseek", "deepseek-chat", true)
            .await
            .unwrap();

        let ms = s.list_models().await.unwrap();
        assert_eq!(ms.len(), 2);

        let routes = s.load_routes().await.unwrap();
        assert_eq!(routes.len(), 2);
        let coder = routes.iter().find(|r| r.model_id == "my-cheap-coder").unwrap();
        assert_eq!(coder.provider, "deepseek");
        assert_eq!(coder.real_model, "deepseek-chat");
        assert!(coder.inject_usage);
        assert_eq!(coder.api_key.as_deref(), Some("sk-2"));
        assert_eq!(coder.dialect, Dialect::OpenaiChat);
    }

    #[tokio::test]
    async fn cascade_delete() {
        let s = store().await;
        s.add_provider("p", "https://x", Dialect::Anthropic, None).await.unwrap();
        s.add_model("m", "p", "m", false).await.unwrap();
        assert_eq!(s.remove_provider("p").await.unwrap(), 1);
        assert_eq!(s.list_models().await.unwrap().len(), 0); // cascaded
    }

    #[tokio::test]
    async fn add_model_unknown_provider_fails() {
        let s = store().await;
        assert!(s.add_model("m", "ghost", "m", false).await.is_err());
    }

    #[tokio::test]
    async fn upsert_provider() {
        let s = store().await;
        s.add_provider("p", "https://a", Dialect::Anthropic, Some("k1")).await.unwrap();
        s.add_provider("p", "https://b", Dialect::Anthropic, Some("k2")).await.unwrap();
        let ps = s.list_providers().await.unwrap();
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].base_url, "https://b");
        assert_eq!(ps[0].api_key.as_deref(), Some("k2"));
    }
}
