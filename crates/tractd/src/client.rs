//! Client used by `tractd status` and other in-process subcommands that
//! need to talk to a running daemon.

use std::path::Path;

use anyhow::Context;
use tract_proto::{Method, Request, Response};
use uuid::Uuid;

use crate::ipc;

/// Send a single request to the default daemon endpoint and return its response.
pub async fn send(method: Method) -> anyhow::Result<Response> {
    #[cfg(unix)]
    {
        let path = ipc::default_socket_path()?;
        send_unix(&path, method).await
    }
    #[cfg(windows)]
    {
        let port_file = ipc::default_port_file()?;
        send_windows(&port_file, method).await
    }
}

#[cfg(unix)]
pub async fn send_unix(path: &Path, method: Method) -> anyhow::Result<Response> {
    let stream = tokio::net::UnixStream::connect(path)
        .await
        .with_context(|| format!("connect to {}", path.display()))?;
    request(stream, method).await
}

#[cfg(windows)]
pub async fn send_windows(port_file: &Path, method: Method) -> anyhow::Result<Response> {
    let port_str = std::fs::read_to_string(port_file)
        .with_context(|| format!("reading port file {}", port_file.display()))?;
    let port: u16 = port_str
        .trim()
        .parse()
        .with_context(|| format!("parsing port from {}", port_file.display()))?;
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .with_context(|| format!("connect to 127.0.0.1:{port}"))?;
    request(stream, method).await
}

async fn request<S>(mut stream: S, method: Method) -> anyhow::Result<Response>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let req = Request {
        id: Uuid::new_v4(),
        method,
    };
    let body = serde_json::to_vec(&req)?;
    ipc::write_frame(&mut stream, &body).await?;
    let resp_body = ipc::read_frame(&mut stream)
        .await?
        .ok_or_else(|| anyhow::anyhow!("daemon closed connection without responding"))?;
    let resp: Response = serde_json::from_slice(&resp_body)?;
    Ok(resp)
}
