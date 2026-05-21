//! HTTP engine.
//!
//! MVP: plain reqwest + rustls with browser-shaped default headers. Real
//! TLS/JA3/JA4 fingerprint matching is deferred to v0.2; see DESIGN.md §11.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};

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
    pub fn new() -> Result<Self, FetcherError> {
        let inner = reqwest::Client::builder()
            .use_rustls_tls()
            .http2_adaptive_window(true)
            .gzip(true)
            .brotli(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10))
            .default_headers(default_headers())
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

fn default_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    let pairs: &[(&str, &str)] = &[
        ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"),
        ("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"),
        ("Accept-Language", "en-US,en;q=0.9"),
        ("Accept-Encoding", "gzip, deflate, br"),
        ("Sec-Ch-Ua", "\"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\", \"Google Chrome\";v=\"131\""),
        ("Sec-Ch-Ua-Mobile", "?0"),
        ("Sec-Ch-Ua-Platform", "\"Windows\""),
        ("Sec-Fetch-Dest", "document"),
        ("Sec-Fetch-Mode", "navigate"),
        ("Sec-Fetch-Site", "none"),
        ("Sec-Fetch-User", "?1"),
        ("Upgrade-Insecure-Requests", "1"),
    ];
    for (k, v) in pairs {
        if let Ok(val) = HeaderValue::from_str(v) {
            h.insert(*k, val);
        }
    }
    h
}
