"""SDK smoke test against a local in-process HTTP mock server.

Sanity check: the daemon auto-spawns, the IPC framing roundtrips, the
extractor produces sensible markdown, and the cache serves a warm hit.

We DO NOT hit the real web here — that's flaky in CI. The mock server below
is a 30-line HTTP/1.1 responder that reqwest happily talks to.
"""

from __future__ import annotations

import http.server
import os
import platform
import socketserver
import subprocess
import threading
import time
from contextlib import contextmanager
from pathlib import Path

import pytest

from tract import FetchResult, fetch
from tract._daemon import _runtime_endpoint


HTML = """<!doctype html>
<html lang="en">
<head><title>Smoke Article</title></head>
<body>
  <nav>home about</nav>
  <article>
    <h1>Smoke Article</h1>
    <p>This is the article body. It needs to be long enough to clear the
       readability threshold so the extractor picks the article over the
       page chrome.</p>
    <p>A second paragraph with <strong>bold</strong> and <em>italic</em>.</p>
    <ul><li>alpha</li><li>beta</li><li>gamma</li></ul>
  </article>
  <footer>copyright</footer>
</body></html>
"""


@pytest.fixture(scope="session", autouse=True)
def _ensure_clean_daemon():
    """Kill any lingering daemon and clean up the port file before the suite.

    Tests share a daemon (cheaper than spawning per-test); when the suite
    ends we kill it so a stale daemon doesn't haunt the developer's machine.
    """
    _stop_daemon()
    yield
    _stop_daemon()


@pytest.fixture(scope="module")
def mock_url():
    with _spawn_mock(HTML) as port:
        yield f"http://127.0.0.1:{port}/article"


def test_fetch_returns_markdown_then_serves_cache(mock_url):
    md = fetch(mock_url)
    assert isinstance(md, str)
    assert "# Smoke Article" in md, md
    assert "**bold**" in md
    assert "- alpha" in md
    assert "home" not in md, "nav leaked"
    assert "copyright" not in md, "footer leaked"

    detailed = fetch(mock_url, return_details=True)
    assert isinstance(detailed, FetchResult)
    assert detailed.title == "Smoke Article"
    assert detailed.from_cache is True, "second fetch should hit cache"

    forced = fetch(mock_url, return_details=True, force_refresh=True)
    assert forced.from_cache is False, "force_refresh should bypass cache"


def test_fetch_invalid_url_raises_typed_error(mock_url):
    from tract import FetchFailedError

    with pytest.raises(FetchFailedError) as ei:
        fetch("not a url at all")
    assert ei.value.code == "invalid_url"


def test_auto_spawn_recovers_when_daemon_crashes(mock_url):
    """Kill the daemon mid-session; the next fetch should auto-respawn it."""
    _stop_daemon()
    md = fetch(mock_url)
    assert "Smoke Article" in md


# --- helpers ---


def _stop_daemon():
    """Stop any running tractd and delete the port/socket file."""
    if platform.system() == "Windows":
        # SIGKILL-equivalent; ctrl-c handler doesn't run, so we clean up the
        # port file ourselves below.
        subprocess.run(
            ["taskkill", "/F", "/IM", "tractd.exe"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    else:
        subprocess.run(
            ["pkill", "-9", "-x", "tractd"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    endpoint = _runtime_endpoint()
    try:
        endpoint.unlink()
    except FileNotFoundError:
        pass
    # Brief settle for OS-level socket teardown.
    time.sleep(0.1)


@contextmanager
def _spawn_mock(html: str):
    handler = _make_handler(html.encode("utf-8"))
    server = socketserver.ThreadingTCPServer(("127.0.0.1", 0), handler)
    port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield port
    finally:
        server.shutdown()
        server.server_close()


def _make_handler(body: bytes):
    class H(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_args, **_kwargs):
            pass

    return H
