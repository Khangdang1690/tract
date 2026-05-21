"""IPC client and the public ``fetch()`` entry point."""

from __future__ import annotations

import json
import socket
import struct
import uuid
from dataclasses import dataclass
from typing import Optional, Union

from . import _daemon
from ._errors import FetchFailedError, TractError, TractTimeoutError

MAX_FRAME_BYTES = 16 * 1024 * 1024


@dataclass
class FetchResult:
    markdown: str
    title: Optional[str]
    final_url: Optional[str]
    fetched_at: Optional[int]
    from_cache: bool
    trace_id: Optional[str]


def fetch(
    url: str,
    *,
    force_refresh: bool = False,
    return_details: bool = False,
    timeout: float = 35.0,
) -> Union[str, FetchResult]:
    """Fetch a URL via the tract daemon and return its content.

    Args:
        url: The URL to fetch.
        force_refresh: If ``True``, bypass the cache and re-fetch from network.
        return_details: If ``True``, return a :class:`FetchResult` with metadata;
            otherwise return just the markdown string.
        timeout: Maximum seconds to wait for the daemon's response.

    Returns:
        Either the markdown string or a :class:`FetchResult` depending on
        ``return_details``.

    Raises:
        FetchFailedError: When the daemon returned a typed error.
        TractTimeoutError: When the daemon did not respond in time.
        DaemonStartupError: When the daemon could not be reached at all.
    """
    sock = _daemon.connect()
    sock.settimeout(timeout)
    try:
        req = {
            "id": str(uuid.uuid4()),
            "method": "fetch",
            "params": {
                "url": url,
                "options": {
                    "force_refresh": force_refresh,
                    "return_details": return_details,
                },
            },
        }
        _write_frame(sock, json.dumps(req).encode("utf-8"))
        try:
            resp_bytes = _read_frame(sock)
        except socket.timeout as e:
            raise TractTimeoutError(
                f"daemon did not respond within {timeout}s"
            ) from e
    finally:
        try:
            sock.close()
        except OSError:
            pass

    resp = json.loads(resp_bytes)
    if not resp.get("ok"):
        err = resp.get("error") or {}
        raise FetchFailedError(
            str(err.get("code", "unknown")), str(err.get("message", ""))
        )
    result = resp.get("result") or {}
    if result.get("kind") != "fetch":
        raise TractError(f"unexpected response kind: {result.get('kind')!r}")

    if return_details:
        return FetchResult(
            markdown=result.get("markdown", ""),
            title=result.get("title"),
            final_url=result.get("final_url"),
            fetched_at=result.get("fetched_at"),
            from_cache=bool(result.get("from_cache", False)),
            trace_id=result.get("trace_id"),
        )
    return result.get("markdown", "")


def _write_frame(sock: socket.socket, body: bytes) -> None:
    if len(body) > MAX_FRAME_BYTES:
        raise TractError(f"outgoing frame too large: {len(body)} bytes")
    sock.sendall(struct.pack("<I", len(body)) + body)


def _read_frame(sock: socket.socket) -> bytes:
    head = _read_exact(sock, 4)
    (length,) = struct.unpack("<I", head)
    if length > MAX_FRAME_BYTES:
        raise TractError(f"incoming frame too large: {length} bytes")
    return _read_exact(sock, length)


def _read_exact(sock: socket.socket, n: int) -> bytes:
    buf = bytearray()
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            raise TractError("daemon closed connection unexpectedly")
        buf.extend(chunk)
    return bytes(buf)
