//! HTTP engine + SQLite cache for tract.
//!
//! The fetcher's job is narrow: given a URL, return raw bytes from the
//! network (or from cache when fresh). Content extraction lives in
//! `tract-extractor`; orchestration lives in `tractd`.

pub mod cache;
pub mod http;
pub mod url_normalize;

pub use cache::{Cache, CachedResponse};
pub use http::{HttpClient, RawResponse};
pub use url_normalize::normalize_url;

#[derive(Debug, thiserror::Error)]
pub enum FetcherError {
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("cache error: {0}")]
    Cache(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("timeout")]
    Timeout,
}
