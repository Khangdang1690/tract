//! HTTP engine.
//!
//! All tuning (headers, timeouts, redirect limit) comes from `HttpProfile`.
//! TLS fingerprinting lands in Phase D; the current backend is reqwest+rustls.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tract_profile::HttpProfile;

use crate::FetcherError;

#[derive(Debug, Clone)]
pub struct RawResponse {
    pub final_url: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
}

#[derive(Clone)]
pub struct HttpClient {
    inner: reqwest::Client,
}

impl HttpClient {
    pub fn new(profile: &HttpProfile) -> Result<Self, FetcherError> {
        let inner = reqwest::Client::builder()
            .use_rustls_tls()
            .http2_adaptive_window(true)
            .gzip(true)
            .brotli(true)
            .connect_timeout(Duration::from_secs(profile.connect_timeout_secs))
            .timeout(Duration::from_secs(profile.total_timeout_secs))
            .redirect(reqwest::redirect::Policy::limited(profile.redirect_limit))
            .default_headers(headers_from_profile(profile))
            .build()?;
        Ok(Self { inner })
    }

    pub async fn fetch(&self, url: &str) -> Result<RawResponse, FetcherError> {
        let resp = self.inner.get(url).send().await?;
        let final_url = resp.url().to_string();
        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.bytes().await?.to_vec();
        Ok(RawResponse {
            final_url,
            status,
            headers,
            body,
            content_type,
        })
    }
}

fn headers_from_profile(profile: &HttpProfile) -> HeaderMap {
    let mut h = HeaderMap::new();
    for (name, value) in &profile.headers {
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            h.insert(n, v);
        }
    }
    h
}
