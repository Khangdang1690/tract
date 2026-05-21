//! End-to-end fetch test: spin up a tiny in-process HTTP/1.1 server,
//! drive `Orchestrator::fetch` against it, verify the extracted markdown,
//! re-fetch and verify the cache served the second hit.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;

use std::sync::Arc;
use tractd::orchestrator;

#[tokio::test]
async fn fetch_extracts_markdown_then_serves_from_cache() {
    let html = r#"<!doctype html>
        <html lang="en">
        <head><title>Example Article</title></head>
        <body>
            <nav>home</nav>
            <article>
                <h1>Example Article</h1>
                <p>Paragraph one is informative and long enough to count
                   as content for the readability scorer.</p>
                <p>Paragraph two has <strong>bold</strong> and <em>italic</em>.</p>
                <ul><li>alpha</li><li>beta</li></ul>
            </article>
            <footer>copyright</footer>
        </body></html>"#;

    let (port, _stop) = spawn_mock(html.to_string()).await;

    let cache = tract_fetcher::Cache::in_memory().unwrap();
    let profile = Arc::new(tract_profile::Profile::default());
    let http = tract_fetcher::HttpClient::new(&profile.http).unwrap();
    let orch = orchestrator::Orchestrator::new(profile, cache, http);

    let url = format!("http://127.0.0.1:{port}/article");
    let result = timeout(
        Duration::from_secs(10),
        orch.fetch(&url, tract_proto::FetchOptions::default()),
    )
    .await
    .expect("fetch did not complete in 10s")
    .expect("fetch returned an error");

    assert_eq!(result.title.as_deref(), Some("Example Article"));
    assert!(!result.from_cache, "first fetch must not be cached");
    assert!(
        result.markdown.contains("# Example Article"),
        "markdown missing title heading:\n{}",
        result.markdown
    );
    assert!(
        result.markdown.contains("**bold**"),
        "markdown missing bold:\n{}",
        result.markdown
    );
    assert!(
        result.markdown.contains("- alpha"),
        "markdown missing list:\n{}",
        result.markdown
    );
    assert!(
        !result.markdown.contains("home"),
        "nav leaked into markdown:\n{}",
        result.markdown
    );
    assert!(
        !result.markdown.contains("copyright"),
        "footer leaked into markdown:\n{}",
        result.markdown
    );

    // Second fetch should hit the cache.
    let second = orch
        .fetch(&url, tract_proto::FetchOptions::default())
        .await
        .expect("second fetch failed");
    assert!(second.from_cache, "second fetch must be served from cache");
    assert_eq!(second.markdown, result.markdown);

    // force_refresh bypasses cache.
    let forced = orch
        .fetch(
            &url,
            tract_proto::FetchOptions {
                force_refresh: true,
                return_details: false,
            },
        )
        .await
        .expect("forced fetch failed");
    assert!(!forced.from_cache, "force_refresh must hit network again");
}

#[tokio::test]
async fn fetch_invalid_url_returns_invalid_url_error() {
    let cache = tract_fetcher::Cache::in_memory().unwrap();
    let profile = Arc::new(tract_profile::Profile::default());
    let http = tract_fetcher::HttpClient::new(&profile.http).unwrap();
    let orch = orchestrator::Orchestrator::new(profile, cache, http);
    let err = orch
        .fetch("not://a real url at all", Default::default())
        .await
        .expect_err("must error");
    assert_eq!(err.code, tract_proto::ErrorCode::InvalidUrl);
}

#[tokio::test]
async fn fetch_404_returns_fetch_failed() {
    let (port, _stop) = spawn_mock_status(404, "<h1>nope</h1>".into()).await;
    let cache = tract_fetcher::Cache::in_memory().unwrap();
    let profile = Arc::new(tract_profile::Profile::default());
    let http = tract_fetcher::HttpClient::new(&profile.http).unwrap();
    let orch = orchestrator::Orchestrator::new(profile, cache, http);
    let err = orch
        .fetch(
            &format!("http://127.0.0.1:{port}/missing"),
            Default::default(),
        )
        .await
        .expect_err("must error");
    assert_eq!(err.code, tract_proto::ErrorCode::FetchFailed);
}

// --- Mock HTTP/1.1 server ---

/// Holds the accept task; dropping it stops the server.
struct StopHandle(#[allow(dead_code)] tokio::task::JoinHandle<()>);

async fn spawn_mock(body: String) -> (u16, StopHandle) {
    spawn_mock_status(200, body).await
}

async fn spawn_mock_status(status: u16, body: String) -> (u16, StopHandle) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let body = body.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let _ = stream.read(&mut buf).await;
                let reason = match status {
                    200 => "OK",
                    404 => "Not Found",
                    500 => "Internal Server Error",
                    _ => "Status",
                };
                let resp = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });
    (port, StopHandle(handle))
}
