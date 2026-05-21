//! URL normalization for stable cache keys.

use url::Url;

use crate::FetcherError;

/// Normalize a URL so equivalent inputs hash to the same cache key.
///
/// - Lowercase scheme + host.
/// - Strip default ports.
/// - Sort query params lexicographically.
/// - Drop fragment.
pub fn normalize_url(input: &str) -> Result<String, FetcherError> {
    let mut u = Url::parse(input).map_err(|_| FetcherError::InvalidUrl(input.to_string()))?;
    u.set_fragment(None);

    if let Some(host) = u.host_str() {
        let lower = host.to_ascii_lowercase();
        // set_host can fail for some schemes; ignore failure and leave as-is.
        let _ = u.set_host(Some(&lower));
    }

    if let Some(port) = u.port() {
        let is_default = matches!((u.scheme(), port), ("http", 80) | ("https", 443));
        if is_default {
            let _ = u.set_port(None);
        }
    }

    if u.query().is_some() {
        let mut pairs: Vec<(String, String)> = u
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        pairs.sort();
        let mut q = u.query_pairs_mut();
        q.clear();
        for (k, v) in &pairs {
            q.append_pair(k, v);
        }
        drop(q);
    }

    Ok(u.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_host_case_and_fragment() {
        let got = normalize_url("https://Example.COM/Path#section").unwrap();
        assert_eq!(got, "https://example.com/Path");
    }

    #[test]
    fn strips_default_port() {
        let got = normalize_url("https://example.com:443/x").unwrap();
        assert_eq!(got, "https://example.com/x");
    }

    #[test]
    fn keeps_nondefault_port() {
        let got = normalize_url("http://example.com:8080/x").unwrap();
        assert_eq!(got, "http://example.com:8080/x");
    }

    #[test]
    fn sorts_query_params() {
        let a = normalize_url("https://example.com/?b=2&a=1").unwrap();
        let b = normalize_url("https://example.com/?a=1&b=2").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_invalid() {
        assert!(normalize_url("not a url").is_err());
    }
}
