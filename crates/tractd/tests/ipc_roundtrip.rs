//! End-to-end IPC roundtrip: start a server in one task, drive it as a client
//! in another, on whichever transport is appropriate for the current platform.

use std::time::Duration;

use tempfile::TempDir;
use tokio::time::{sleep, timeout};

// The server/client modules are reused from the binary crate via a small test
// shim — but since this is an integration test, we can only exercise the
// crate's public surface. We re-implement the minimal client locally to avoid
// publishing the internal modules.

#[path = "../src/ipc.rs"]
#[allow(dead_code)]
mod ipc;

#[tokio::test]
async fn ping_and_status_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let cache = tract_fetcher::Cache::in_memory().unwrap();

    #[cfg(unix)]
    let endpoint = tmp.path().join("tractd.sock");
    #[cfg(windows)]
    let endpoint = tmp.path().join("port");

    // Spawn the server. We can't reach the binary's `server::serve_*` directly
    // from an integration test, so we duplicate the listener bootstrap here
    // using the same primitives. This is intentional: the shared logic lives
    // in `ipc::{read_frame, write_frame}`, which IS reused.
    let state_cache = cache.clone();
    let endpoint_for_server = endpoint.clone();
    let server = tokio::spawn(async move {
        run_server(state_cache, endpoint_for_server).await.unwrap();
    });

    // Wait for the listener to be ready.
    wait_ready(&endpoint).await;

    // --- Ping ---
    let resp = call(&endpoint, "ping", serde_json::Value::Null).await;
    assert_eq!(resp["ok"], true);
    assert_eq!(resp["result"]["kind"], "pong");
    assert_eq!(resp["result"]["proto"], tract_proto::PROTO_VERSION);

    // --- Status ---
    let resp = call(&endpoint, "status", serde_json::Value::Null).await;
    assert_eq!(resp["ok"], true);
    assert_eq!(resp["result"]["kind"], "status");
    assert_eq!(resp["result"]["cache_entries"], 0);

    // --- Malformed input gets InvalidRequest ---
    let resp = call_raw(&endpoint, b"not json at all").await;
    assert_eq!(resp["ok"], false);
    assert_eq!(resp["error"]["code"], "invalid_request");

    server.abort();
}

// --- Server bootstrap mirror ---

async fn run_server(
    cache: tract_fetcher::Cache,
    endpoint: std::path::PathBuf,
) -> anyhow::Result<()> {
    let started_at = std::time::Instant::now();
    #[cfg(unix)]
    {
        let listener = tokio::net::UnixListener::bind(&endpoint)?;
        loop {
            let (stream, _) = listener.accept().await?;
            let cache = cache.clone();
            tokio::spawn(handle_one(stream, cache, started_at));
        }
    }
    #[cfg(windows)]
    {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        std::fs::write(&endpoint, port.to_string())?;
        loop {
            let (stream, _) = listener.accept().await?;
            let cache = cache.clone();
            tokio::spawn(handle_one(stream, cache, started_at));
        }
    }
}

async fn handle_one<S>(mut stream: S, cache: tract_fetcher::Cache, started_at: std::time::Instant)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    while let Some(body) = ipc::read_frame(&mut stream).await.unwrap_or(None) {
        let resp = build_response(&body, &cache, started_at).await;
        let resp_body = serde_json::to_vec(&resp).unwrap();
        if ipc::write_frame(&mut stream, &resp_body).await.is_err() {
            return;
        }
    }
}

async fn build_response(
    body: &[u8],
    cache: &tract_fetcher::Cache,
    started_at: std::time::Instant,
) -> tract_proto::Response {
    use tract_proto::{
        ErrorCode, Method, ProtoError, Request, Response, ResultBody, PROTO_VERSION,
    };
    let req: Request = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => {
            return Response::err(
                uuid::Uuid::nil(),
                ProtoError::new(ErrorCode::InvalidRequest, format!("malformed: {e}")),
            );
        }
    };
    let id = req.id;
    match req.method {
        Method::Ping => Response::ok(
            id,
            ResultBody::Pong {
                version: "test".into(),
                proto: PROTO_VERSION,
            },
        ),
        Method::Status => Response::ok(
            id,
            ResultBody::Status {
                version: "test".into(),
                proto: PROTO_VERSION,
                uptime_secs: started_at.elapsed().as_secs(),
                cache_entries: cache.count().await.unwrap(),
            },
        ),
        Method::Fetch(_) => Response::err(id, ProtoError::new(ErrorCode::UnknownMethod, "M3")),
    }
}

// --- Client mirror ---

async fn wait_ready(endpoint: &std::path::Path) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if endpoint.exists() {
            // give the listener a beat to register the file/port
            sleep(Duration::from_millis(20)).await;
            return;
        }
        if tokio::time::Instant::now() > deadline {
            panic!("listener never came up");
        }
        sleep(Duration::from_millis(20)).await;
    }
}

async fn call(
    endpoint: &std::path::Path,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let mut req = serde_json::json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "method": method,
    });
    if !params.is_null() {
        req["params"] = params;
    }
    let body = serde_json::to_vec(&req).unwrap();
    call_raw(endpoint, &body).await
}

async fn call_raw(endpoint: &std::path::Path, body: &[u8]) -> serde_json::Value {
    #[cfg(unix)]
    let mut stream = tokio::net::UnixStream::connect(endpoint).await.unwrap();
    #[cfg(windows)]
    let mut stream = {
        let port: u16 = std::fs::read_to_string(endpoint)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap()
    };
    ipc::write_frame(&mut stream, body).await.unwrap();
    let resp_body = timeout(Duration::from_secs(2), ipc::read_frame(&mut stream))
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    serde_json::from_slice(&resp_body).unwrap()
}
