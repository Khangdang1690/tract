//! Listener loop and per-connection handler.

use std::path::Path;

use anyhow::Context;
use tokio::io::{AsyncRead, AsyncWrite};
use tract_proto::{ErrorCode, ProtoError, Request, Response};
use uuid::Uuid;

use crate::{handler, ipc, state::AppState};

/// Run the listener until ctrl-c. Picks the platform-appropriate transport.
pub async fn serve(state: AppState) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let path = ipc::default_socket_path()?;
        serve_unix(state, &path).await
    }
    #[cfg(windows)]
    {
        let port_file = ipc::default_port_file()?;
        serve_windows(state, &port_file).await
    }
}

#[cfg(unix)]
pub async fn serve_unix(state: AppState, path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Pre-flight: refuse to start if another daemon is alive at this path.
    if path.exists() {
        if is_alive_unix(path).await {
            anyhow::bail!("daemon already running at {}", path.display());
        }
        std::fs::remove_file(path).context("removing stale socket")?;
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("creating socket dir")?;
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
    }

    let listener = tokio::net::UnixListener::bind(path)
        .with_context(|| format!("binding {}", path.display()))?;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));

    tracing::info!(socket = %path.display(), "tractd serving on Unix socket");
    eprintln!("tractd listening on {}", path.display());

    tokio::select! {
        res = accept_unix(listener, state) => {
            if let Err(e) = res {
                tracing::error!(error = %e, "accept loop ended");
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutdown signal received");
        }
    }

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[cfg(unix)]
async fn accept_unix(listener: tokio::net::UnixListener, state: AppState) -> anyhow::Result<()> {
    loop {
        let (stream, _) = listener.accept().await.context("accept")?;
        let s = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, s).await {
                tracing::warn!(error = %e, "connection error");
            }
        });
    }
}

#[cfg(unix)]
async fn is_alive_unix(path: &Path) -> bool {
    use tokio::time::{timeout, Duration};
    let connect = tokio::net::UnixStream::connect(path);
    matches!(
        timeout(Duration::from_millis(500), connect).await,
        Ok(Ok(_))
    )
}

#[cfg(windows)]
pub async fn serve_windows(state: AppState, port_file: &Path) -> anyhow::Result<()> {
    if let Some(parent) = port_file.parent() {
        std::fs::create_dir_all(parent).context("creating tract data dir")?;
    }

    // Pre-flight: refuse to start if a recorded port is still alive.
    if port_file.exists() {
        if let Ok(s) = std::fs::read_to_string(port_file) {
            if let Ok(port) = s.trim().parse::<u16>() {
                if is_alive_tcp(port).await {
                    anyhow::bail!("daemon already running on 127.0.0.1:{port}");
                }
            }
        }
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    std::fs::write(port_file, addr.port().to_string())
        .with_context(|| format!("writing {}", port_file.display()))?;

    tracing::info!(addr = %addr, port_file = %port_file.display(), "tractd serving on loopback TCP");
    eprintln!(
        "tractd listening on 127.0.0.1:{} (port file: {})",
        addr.port(),
        port_file.display()
    );

    tokio::select! {
        res = accept_tcp(listener, state) => {
            if let Err(e) = res {
                tracing::error!(error = %e, "accept loop ended");
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutdown signal received");
        }
    }

    let _ = std::fs::remove_file(port_file);
    Ok(())
}

#[cfg(windows)]
async fn accept_tcp(listener: tokio::net::TcpListener, state: AppState) -> anyhow::Result<()> {
    loop {
        let (stream, _) = listener.accept().await.context("accept")?;
        let s = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, s).await {
                tracing::warn!(error = %e, "connection error");
            }
        });
    }
}

#[cfg(windows)]
async fn is_alive_tcp(port: u16) -> bool {
    use tokio::time::{timeout, Duration};
    let connect = tokio::net::TcpStream::connect(("127.0.0.1", port));
    matches!(
        timeout(Duration::from_millis(500), connect).await,
        Ok(Ok(_))
    )
}

pub async fn handle_conn<S>(mut stream: S, state: AppState) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    while let Some(body) = ipc::read_frame(&mut stream).await? {
        let resp = match serde_json::from_slice::<Request>(&body) {
            Ok(req) => handler::dispatch(&state, req).await,
            Err(e) => Response::err(
                Uuid::nil(),
                ProtoError::new(ErrorCode::InvalidRequest, format!("malformed request: {e}")),
            ),
        };
        let resp_body = serde_json::to_vec(&resp)?;
        ipc::write_frame(&mut stream, &resp_body).await?;
    }
    Ok(())
}
