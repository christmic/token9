use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

use super::RequestRow;
use crate::config::expand_tilde;

const CREATE_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS requests (
  id                 TEXT PRIMARY KEY,
  ts                 INTEGER NOT NULL,
  client_protocol    TEXT NOT NULL,
  model_id           TEXT NOT NULL,
  provider           TEXT NOT NULL,
  real_model         TEXT NOT NULL,
  stream             INTEGER NOT NULL,
  status             INTEGER,
  input_tokens       INTEGER NOT NULL DEFAULT 0,
  output_tokens      INTEGER NOT NULL DEFAULT 0,
  cache_write_tokens INTEGER NOT NULL DEFAULT 0,
  cache_read_tokens  INTEGER NOT NULL DEFAULT 0,
  latency_ms         INTEGER,
  ttft_ms            INTEGER,
  error              TEXT
);
CREATE INDEX IF NOT EXISTS idx_requests_ts ON requests(ts);
CREATE INDEX IF NOT EXISTS idx_requests_provider ON requests(provider, real_model);
"#;

#[derive(Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn open(db_path: &str) -> anyhow::Result<Self> {
        let path = expand_tilde(db_path);
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let opts = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(opts).await?;
        sqlx::raw_sql(CREATE_TABLE).execute(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn record(&self, row: RequestRow) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO requests
               (id, ts, client_protocol, model_id, provider, real_model, stream, status,
                input_tokens, output_tokens, cache_write_tokens, cache_read_tokens,
                latency_ms, ttft_ms, error)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#,
        )
        .bind(row.id)
        .bind(row.ts)
        .bind(row.client_protocol)
        .bind(row.model_id)
        .bind(row.provider)
        .bind(row.real_model)
        .bind(row.stream as i64)
        .bind(row.status)
        .bind(row.input_tokens)
        .bind(row.output_tokens)
        .bind(row.cache_write_tokens)
        .bind(row.cache_read_tokens)
        .bind(row.latency_ms)
        .bind(row.ttft_ms)
        .bind(row.error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Aggregate totals grouped by provider + real_model.
    pub async fn summary(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            r#"SELECT provider, real_model,
                      COUNT(*)                  AS requests,
                      SUM(input_tokens)         AS input_tokens,
                      SUM(output_tokens)        AS output_tokens,
                      SUM(cache_read_tokens)    AS cache_read_tokens,
                      SUM(cache_write_tokens)   AS cache_write_tokens
               FROM requests
               GROUP BY provider, real_model
               ORDER BY requests DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;

        let out = rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "provider": r.get::<String, _>("provider"),
                    "real_model": r.get::<String, _>("real_model"),
                    "requests": r.get::<i64, _>("requests"),
                    "input_tokens": r.get::<Option<i64>, _>("input_tokens").unwrap_or(0),
                    "output_tokens": r.get::<Option<i64>, _>("output_tokens").unwrap_or(0),
                    "cache_read_tokens": r.get::<Option<i64>, _>("cache_read_tokens").unwrap_or(0),
                    "cache_write_tokens": r.get::<Option<i64>, _>("cache_write_tokens").unwrap_or(0),
                })
            })
            .collect();
        Ok(out)
    }
}
