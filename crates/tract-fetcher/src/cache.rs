//! SQLite content cache.

use std::path::Path;
use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::FetcherError;

#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub url: String,
    pub final_url: String,
    pub fetched_at: i64,
    pub expires_at: Option<i64>,
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

impl CachedResponse {
    pub fn is_fresh(&self, now_secs: i64) -> bool {
        match self.expires_at {
            Some(e) => now_secs < e,
            None => false,
        }
    }
}

/// SQLite-backed cache. A single connection wrapped in a Tokio mutex.
/// Good enough for MVP; v0.2 swaps to a connection pool.
#[derive(Clone)]
pub struct Cache {
    conn: Arc<Mutex<Connection>>,
}

impl Cache {
    pub fn open(path: &Path) -> Result<Self, FetcherError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn in_memory() -> Result<Self, FetcherError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn get(&self, url: &str) -> Result<Option<CachedResponse>, FetcherError> {
        let hash = url_hash(url);
        let conn = self.conn.lock().await;
        let row = conn
            .query_row(
                "SELECT url, final_url, fetched_at, expires_at, status, content_type, body \
                 FROM responses WHERE url_hash = ?1",
                params![hash.as_slice()],
                |r| {
                    Ok(CachedResponse {
                        url: r.get(0)?,
                        final_url: r.get(1)?,
                        fetched_at: r.get(2)?,
                        expires_at: r.get(3)?,
                        status: r.get::<_, i64>(4)? as u16,
                        content_type: r.get(5)?,
                        body: r.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub async fn put(&self, entry: &CachedResponse) -> Result<(), FetcherError> {
        let hash = url_hash(&entry.url);
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO responses \
             (url_hash, url, final_url, fetched_at, expires_at, status, content_type, body) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                hash.as_slice(),
                entry.url,
                entry.final_url,
                entry.fetched_at,
                entry.expires_at,
                entry.status as i64,
                entry.content_type,
                entry.body,
            ],
        )?;
        Ok(())
    }

    pub async fn count(&self) -> Result<u64, FetcherError> {
        let conn = self.conn.lock().await;
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM responses", [], |r| r.get(0))?;
        Ok(n as u64)
    }
}

fn url_hash(url: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(url.as_bytes());
    h.finalize().into()
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS responses (
    url_hash     BLOB PRIMARY KEY,
    url          TEXT NOT NULL,
    final_url    TEXT NOT NULL,
    fetched_at   INTEGER NOT NULL,
    expires_at   INTEGER,
    status       INTEGER NOT NULL,
    content_type TEXT,
    body         BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_responses_expires ON responses(expires_at);
"#;

/// Parse `Cache-Control` and return an absolute expiry timestamp, or `None`
/// if the response should not be cached (`no-store`).
pub fn expiry_from_headers(
    headers: &[(String, String)],
    fetched_at: i64,
    default_ttl_secs: i64,
) -> Option<i64> {
    let cc = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("cache-control"))
        .map(|(_, v)| v.to_lowercase());
    if let Some(cc) = cc.as_deref() {
        if cc.contains("no-store") || cc.contains("private") {
            return None;
        }
        for part in cc.split(',') {
            let part = part.trim();
            if let Some(rest) = part.strip_prefix("max-age=") {
                if let Ok(n) = rest.parse::<i64>() {
                    return Some(fetched_at + n.max(0));
                }
            }
            if let Some(rest) = part.strip_prefix("s-maxage=") {
                if let Ok(n) = rest.parse::<i64>() {
                    return Some(fetched_at + n.max(0));
                }
            }
        }
    }
    Some(fetched_at + default_ttl_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn put_then_get_roundtrips() {
        let cache = Cache::in_memory().unwrap();
        let entry = CachedResponse {
            url: "https://example.com".into(),
            final_url: "https://example.com/".into(),
            fetched_at: 1_000,
            expires_at: Some(4_600),
            status: 200,
            content_type: Some("text/html".into()),
            body: b"<html></html>".to_vec(),
        };
        cache.put(&entry).await.unwrap();
        let got = cache.get("https://example.com").await.unwrap().unwrap();
        assert_eq!(got.body, entry.body);
        assert!(got.is_fresh(2_000));
        assert!(!got.is_fresh(5_000));
    }

    #[test]
    fn cache_control_no_store_means_no_cache() {
        let h = vec![("Cache-Control".into(), "no-store".into())];
        assert!(expiry_from_headers(&h, 100, 3600).is_none());
    }

    #[test]
    fn cache_control_max_age_wins_over_default() {
        let h = vec![("Cache-Control".into(), "public, max-age=60".into())];
        assert_eq!(expiry_from_headers(&h, 100, 3600), Some(160));
    }

    #[test]
    fn no_cache_control_uses_default_ttl() {
        let h: Vec<(String, String)> = vec![];
        assert_eq!(expiry_from_headers(&h, 100, 3600), Some(3700));
    }
}
