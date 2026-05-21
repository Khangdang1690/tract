//! Fetch orchestration: cache → HTTP → extractor → cache write,
//! with single-flight de-duplication on identical concurrent URLs.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::{broadcast, Mutex};
use tract_extractor::extract;
use tract_fetcher::{
    cache::{expiry_from_headers, CachedResponse},
    Cache, HttpClient,
};
use tract_proto::{ErrorCode, FetchOptions, FetchResult, ProtoError};
use uuid::Uuid;

pub const DEFAULT_TTL_SECS: i64 = 3600;

type FetchOutcome = Result<FetchResult, ProtoError>;

pub struct Orchestrator {
    cache: Cache,
    http: HttpClient,
    inflight: Mutex<HashMap<String, broadcast::Sender<Arc<FetchOutcome>>>>,
}

impl Orchestrator {
    pub fn new(cache: Cache, http: HttpClient) -> Self {
        Self {
            cache,
            http,
            inflight: Mutex::new(HashMap::new()),
        }
    }

    pub async fn cache_count(&self) -> u64 {
        self.cache.count().await.unwrap_or(0)
    }

    /// Resolve `url` and return the extracted result. Concurrent calls for
    /// the same normalized URL share one underlying fetch.
    pub async fn fetch(&self, url: &str, opts: FetchOptions) -> FetchOutcome {
        let normalized = tract_fetcher::normalize_url(url)
            .map_err(|_| ProtoError::new(ErrorCode::InvalidUrl, format!("invalid url: {url}")))?;

        let now = current_time();

        // Fresh cache hit, unless caller forced refresh.
        if !opts.force_refresh {
            if let Ok(Some(entry)) = self.cache.get(&normalized).await {
                if entry.is_fresh(now) {
                    return self.serve_from_cache(entry);
                }
            }
        }

        // Single-flight check.
        let mut guard = self.inflight.lock().await;
        if let Some(tx) = guard.get(&normalized).cloned() {
            drop(guard);
            let mut rx = tx.subscribe();
            return match rx.recv().await {
                Ok(arc) => (*arc).clone(),
                Err(_) => Err(ProtoError::new(
                    ErrorCode::Internal,
                    "in-flight fetch dropped before completing",
                )),
            };
        }
        let (tx, _rx) = broadcast::channel::<Arc<FetchOutcome>>(16);
        guard.insert(normalized.clone(), tx.clone());
        drop(guard);

        let result = self.do_fetch(&normalized, opts).await;
        let arc = Arc::new(result.clone());
        let _ = tx.send(arc);

        let mut guard = self.inflight.lock().await;
        guard.remove(&normalized);

        result
    }

    async fn do_fetch(&self, normalized: &str, _opts: FetchOptions) -> FetchOutcome {
        let raw = self
            .http
            .fetch(normalized)
            .await
            .map_err(|e| ProtoError::new(ErrorCode::FetchFailed, e.to_string()))?;

        if raw.status >= 400 {
            return Err(ProtoError::new(
                ErrorCode::FetchFailed,
                format!("HTTP {}", raw.status),
            ));
        }

        let body_text = match std::str::from_utf8(&raw.body) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(&raw.body).into_owned(),
        };

        let extracted = extract(&body_text, &raw.final_url)
            .map_err(|e| ProtoError::new(ErrorCode::ExtractFailed, e.to_string()))?;

        let now = current_time();
        let expires_at = expiry_from_headers(&raw.headers, now, DEFAULT_TTL_SECS);
        let entry = CachedResponse {
            url: normalized.to_string(),
            final_url: raw.final_url.clone(),
            fetched_at: now,
            expires_at,
            status: raw.status,
            content_type: raw.content_type.clone(),
            body: raw.body.clone(),
        };
        if let Err(e) = self.cache.put(&entry).await {
            tracing::warn!(error = %e, "cache write failed");
        }

        Ok(FetchResult {
            markdown: extracted.markdown,
            title: extracted.title,
            final_url: raw.final_url,
            fetched_at: now,
            from_cache: false,
            trace_id: Uuid::new_v4().to_string(),
        })
    }

    fn serve_from_cache(&self, entry: CachedResponse) -> FetchOutcome {
        let body_text = match std::str::from_utf8(&entry.body) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(&entry.body).into_owned(),
        };
        let extracted = extract(&body_text, &entry.final_url)
            .map_err(|e| ProtoError::new(ErrorCode::ExtractFailed, e.to_string()))?;
        Ok(FetchResult {
            markdown: extracted.markdown,
            title: extracted.title,
            final_url: entry.final_url,
            fetched_at: entry.fetched_at,
            from_cache: true,
            trace_id: Uuid::new_v4().to_string(),
        })
    }
}

fn current_time() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
