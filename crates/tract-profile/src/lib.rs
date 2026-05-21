//! Tract profile: HTTP, extractor, cache, verifier, classifier, and browser
//! tuning bundled at compile time from `profiles/default.toml`.
//!
//! The runtime daemon constructs `Arc<Profile>` once at boot via
//! `Profile::default()` and threads sub-profiles to the HTTP client, extractor,
//! and cache. No file I/O happens at runtime; the bundled TOML is
//! `include_str!`'d at compile time.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub name: String,
    pub http: HttpProfile,
    pub extractor: ExtractorProfile,
    pub cache: CacheProfile,
    #[serde(default)]
    pub verifier: VerifierProfile,
    #[serde(default)]
    pub classifier: ClassifierProfile,
    #[serde(default)]
    pub browser: BrowserProfile,
    #[serde(default)]
    pub eval: EvalThresholds,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpProfile {
    pub headers: Vec<(String, String)>,
    pub connect_timeout_secs: u64,
    pub total_timeout_secs: u64,
    pub redirect_limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractorProfile {
    pub candidate_selectors: Vec<String>,
    pub min_text_len: usize,
    pub class_id_positive_tokens: Vec<String>,
    pub class_id_negative_tokens: Vec<String>,
    pub positive_token_bonus: f32,
    pub negative_token_penalty: f32,
    pub paragraph_weight: f32,
    pub heading_weight: f32,
    pub link_density_cap: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheProfile {
    pub default_ttl_secs: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct VerifierProfile {}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ClassifierProfile {}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BrowserProfile {}

#[derive(Debug, Clone, Deserialize)]
pub struct EvalThresholds {
    pub trivial_threshold: f32,
    pub moderate_threshold: f32,
    pub hard_threshold: f32,
}

impl Default for EvalThresholds {
    fn default() -> Self {
        Self {
            trivial_threshold: 0.95,
            moderate_threshold: 0.80,
            hard_threshold: 0.60,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("failed to parse profile: {0}")]
    Parse(#[from] toml::de::Error),
}

const DEFAULT_TOML: &str = include_str!("../../../profiles/default.toml");

impl Profile {
    /// Parse a profile from a TOML string. Returned for tests / future
    /// non-default profiles. The bundled default uses `Profile::default()`.
    pub fn from_toml(s: &str) -> Result<Self, ProfileError> {
        Ok(toml::from_str(s)?)
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self::from_toml(DEFAULT_TOML).expect("bundled default profile must parse")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_parses() {
        let p = Profile::default();
        assert_eq!(p.name, "chrome-stable-windows");
        assert!(
            p.http.headers.len() >= 12,
            "expected ≥12 headers, got {}",
            p.http.headers.len()
        );
        assert_eq!(p.http.connect_timeout_secs, 10);
        assert_eq!(p.http.total_timeout_secs, 30);
        assert_eq!(p.http.redirect_limit, 10);
        assert!(
            p.extractor.candidate_selectors.len() >= 17,
            "expected ≥17 selectors, got {}",
            p.extractor.candidate_selectors.len()
        );
        assert_eq!(p.extractor.min_text_len, 200);
        assert_eq!(p.cache.default_ttl_secs, 3600);
        assert_eq!(p.eval.trivial_threshold, 0.95);
    }

    #[test]
    fn header_pairs_round_trip() {
        let p = Profile::default();
        let ua = p
            .http
            .headers
            .iter()
            .find(|(k, _)| k == "User-Agent")
            .expect("User-Agent present");
        assert!(ua.1.contains("Chrome/131"));
    }

    #[test]
    fn negative_tokens_include_nav() {
        let p = Profile::default();
        assert!(p
            .extractor
            .class_id_negative_tokens
            .iter()
            .any(|t| t == "nav"));
    }
}
