//! IPC framing and endpoint discovery.
//!
//! Wire format: 4-byte little-endian length prefix + JSON body. Frames over
//! [`tract_proto::MAX_FRAME_BYTES`] are protocol errors and drop the connection.
//!
//! Endpoint conventions:
//! - POSIX: Unix domain socket under `$XDG_RUNTIME_DIR/tract/tractd.sock`,
//!   falling back to `$XDG_CACHE_HOME/tract/tractd.sock`.
//! - Windows: ephemeral loopback TCP port; the actual port is recorded in
//!   `%LOCALAPPDATA%\tract\port` so clients can find it.

use std::path::PathBuf;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tract_proto::MAX_FRAME_BYTES;

/// Read one length-prefixed JSON frame. Returns `Ok(None)` on clean EOF
/// (the peer closed without starting a new frame).
pub async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> anyhow::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match r.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    anyhow::ensure!(len <= MAX_FRAME_BYTES, "frame too large: {len} bytes");
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    Ok(Some(buf))
}

pub async fn write_frame<W: AsyncWrite + Unpin>(w: &mut W, body: &[u8]) -> anyhow::Result<()> {
    let len = u32::try_from(body.len()).map_err(|_| anyhow::anyhow!("body too large"))?;
    w.write_all(&len.to_le_bytes()).await?;
    w.write_all(body).await?;
    w.flush().await?;
    Ok(())
}

#[cfg(unix)]
pub fn default_socket_path() -> anyhow::Result<PathBuf> {
    let base = dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .ok_or_else(|| anyhow::anyhow!("no runtime or cache dir available"))?;
    Ok(base.join("tract").join("tractd.sock"))
}

#[cfg(windows)]
pub fn default_port_file() -> anyhow::Result<PathBuf> {
    let base = dirs::cache_dir().ok_or_else(|| anyhow::anyhow!("no cache dir available"))?;
    Ok(base.join("tract").join("port"))
}

pub fn default_data_dir() -> anyhow::Result<PathBuf> {
    if let Some(custom) = std::env::var_os("TRACTD_DATA_DIR") {
        return Ok(PathBuf::from(custom));
    }
    Ok(dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("no data dir available"))?
        .join("tract"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn frame_roundtrip() {
        let (mut a, mut b) = duplex(64 * 1024);
        let body = br#"{"hello":"world"}"#.to_vec();
        let body_clone = body.clone();
        let write = tokio::spawn(async move {
            write_frame(&mut a, &body_clone).await.unwrap();
            drop(a);
        });
        let got = read_frame(&mut b).await.unwrap().unwrap();
        write.await.unwrap();
        assert_eq!(got, body);
        // second read returns clean EOF
        assert!(read_frame(&mut b).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn rejects_oversized_frame() {
        let (mut a, mut b) = duplex(64);
        let bad_len: u32 = (MAX_FRAME_BYTES + 1) as u32;
        tokio::spawn(async move {
            let _ = a.write_all(&bad_len.to_le_bytes()).await;
            // never write a body; reader should bail after seeing the len.
        });
        let res = read_frame(&mut b).await;
        assert!(res.is_err());
    }
}
