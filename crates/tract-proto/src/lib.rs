//! IPC schema for tract.
//!
//! Framing on the wire: a 4-byte little-endian length prefix followed by a JSON
//! body. The same schema serves all language SDKs.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PROTO_VERSION: u32 = 1;

/// Maximum frame size accepted by the daemon. Anything larger is a protocol
/// error and the connection is dropped.
pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: Uuid,
    #[serde(flatten)]
    pub method: Method,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Method {
    Ping,
    Status,
    Fetch(FetchParams),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FetchParams {
    pub url: String,
    #[serde(default)]
    pub options: FetchOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FetchOptions {
    #[serde(default)]
    pub force_refresh: bool,
    #[serde(default)]
    pub return_details: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: Uuid,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ResultBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ProtoError>,
}

impl Response {
    pub fn ok(id: Uuid, result: ResultBody) -> Self {
        Self {
            id,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Uuid, error: ProtoError) -> Self {
        Self {
            id,
            ok: false,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResultBody {
    Pong {
        version: String,
        proto: u32,
    },
    Status {
        version: String,
        proto: u32,
        uptime_secs: u64,
        cache_entries: u64,
    },
    Fetch(FetchResult),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub markdown: String,
    pub title: Option<String>,
    pub final_url: String,
    pub fetched_at: i64,
    pub from_cache: bool,
    pub trace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct ProtoError {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    UnknownMethod,
    InvalidUrl,
    FetchFailed,
    ExtractFailed,
    Timeout,
    Internal,
}

impl ProtoError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_request_roundtrips() {
        let req = Request {
            id: Uuid::nil(),
            method: Method::Fetch(FetchParams {
                url: "https://example.com".into(),
                options: FetchOptions {
                    force_refresh: true,
                    return_details: false,
                },
            }),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: Request = serde_json::from_str(&json).unwrap();
        match back.method {
            Method::Fetch(p) => assert_eq!(p.url, "https://example.com"),
            _ => panic!("expected Fetch"),
        }
    }

    #[test]
    fn ping_request_no_params() {
        let req: Request = serde_json::from_str(
            r#"{"id":"00000000-0000-0000-0000-000000000000","method":"ping"}"#,
        )
        .unwrap();
        assert!(matches!(req.method, Method::Ping));
    }

    #[test]
    fn response_ok_shape() {
        let resp = Response::ok(
            Uuid::nil(),
            ResultBody::Pong {
                version: "0.1.0".into(),
                proto: PROTO_VERSION,
            },
        );
        let v: serde_json::Value = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["result"]["kind"], "pong");
        assert!(v.get("error").is_none() || v["error"].is_null());
    }

    #[test]
    fn response_err_shape() {
        let resp = Response::err(
            Uuid::nil(),
            ProtoError::new(ErrorCode::InvalidUrl, "bad scheme"),
        );
        let v: serde_json::Value = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "invalid_url");
    }
}
