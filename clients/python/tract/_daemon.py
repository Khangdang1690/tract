"""Locate, auto-spawn, and connect to ``tractd``.

The SDK looks for the daemon on the platform-appropriate endpoint:

- POSIX: Unix domain socket at ``$XDG_RUNTIME_DIR/tract/tractd.sock`` or the
  per-platform cache directory fallback (``~/Library/Caches/...`` on macOS,
  ``~/.cache/...`` on Linux).
- Windows: loopback TCP on a port recorded in ``%LOCALAPPDATA%\\tract\\port``.

On failure to connect, we acquire a per-user file lock and spawn the daemon
detached, polling for the listener to come up.
"""

from __future__ import annotations

import os
import platform
import socket
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path
from shutil import which

from ._errors import DaemonStartupError

_DEFAULT_STARTUP_TIMEOUT = 5.0


def _is_windows() -> bool:
    return platform.system() == "Windows"


def _is_darwin() -> bool:
    return platform.system() == "Darwin"


def _runtime_endpoint() -> Path:
    """Path to the UDS socket (POSIX) or port file (Windows)."""
    if _is_windows():
        base = Path(os.environ.get("LOCALAPPDATA") or (Path.home() / "AppData" / "Local"))
        return base / "tract" / "port"
    if env := os.environ.get("XDG_RUNTIME_DIR"):
        return Path(env) / "tract" / "tractd.sock"
    if _is_darwin():
        return Path.home() / "Library" / "Caches" / "tract" / "tractd.sock"
    return Path.home() / ".cache" / "tract" / "tractd.sock"


def _open_socket(connect_timeout: float = 0.5) -> socket.socket:
    """Open a connected socket to the daemon, or raise OSError if unreachable."""
    if _is_windows():
        port_file = _runtime_endpoint()
        # Raises FileNotFoundError if no daemon ever ran; ValueError if the
        # file exists but isn't a valid port. Both are caught by callers.
        port = int(port_file.read_text().strip())
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(connect_timeout)
        sock.connect(("127.0.0.1", port))
        return sock
    sock_path = _runtime_endpoint()
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.settimeout(connect_timeout)
    sock.connect(str(sock_path))
    return sock


def _binary_path() -> Path:
    """Locate the daemon binary. Order: env var → bundled → PATH."""
    if env := os.environ.get("TRACTD_BINARY"):
        p = Path(env)
        if not p.exists():
            raise DaemonStartupError(f"TRACTD_BINARY={env} does not exist")
        return p

    bundled = Path(__file__).resolve().parent / "bin" / (
        "tractd.exe" if _is_windows() else "tractd"
    )
    if bundled.exists():
        return bundled

    name = "tractd.exe" if _is_windows() else "tractd"
    found = which(name)
    if found:
        return Path(found)

    raise DaemonStartupError(
        "tractd binary not found. Build with `cargo build --release -p tractd` and "
        "set TRACTD_BINARY, or install tract via a wheel that bundles the binary."
    )


def _spawn_detached() -> subprocess.Popen:
    """Spawn ``tractd serve`` fully detached from the parent process."""
    bin_path = _binary_path()
    common = dict(
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        close_fds=True,
    )
    if _is_windows():
        # DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
        creationflags = 0x00000008 | 0x00000200 | 0x08000000
        return subprocess.Popen(
            [str(bin_path), "serve"], creationflags=creationflags, **common
        )
    return subprocess.Popen(
        [str(bin_path), "serve"], start_new_session=True, **common
    )


def _lock_dir() -> Path:
    if _is_windows():
        base = Path(os.environ.get("LOCALAPPDATA") or (Path.home() / "AppData" / "Local"))
        return base / "tract"
    return Path.home() / ".cache" / "tract"


@contextmanager
def _file_lock():
    """Cross-platform exclusive file lock to serialize spawn attempts."""
    lock_dir = _lock_dir()
    lock_dir.mkdir(parents=True, exist_ok=True)
    lock_path = lock_dir / "spawn.lock"

    if _is_windows():
        import msvcrt

        f = open(lock_path, "ab")
        try:
            # Block until we can lock a single byte. LK_LOCK retries for ~10s.
            f.seek(0)
            while True:
                try:
                    msvcrt.locking(f.fileno(), msvcrt.LK_LOCK, 1)
                    break
                except OSError:
                    time.sleep(0.05)
            try:
                yield
            finally:
                f.seek(0)
                try:
                    msvcrt.locking(f.fileno(), msvcrt.LK_UNLCK, 1)
                except OSError:
                    pass
        finally:
            f.close()
    else:
        import fcntl

        f = open(lock_path, "ab")
        try:
            fcntl.flock(f.fileno(), fcntl.LOCK_EX)
            try:
                yield
            finally:
                try:
                    fcntl.flock(f.fileno(), fcntl.LOCK_UN)
                except OSError:
                    pass
        finally:
            f.close()


def connect(startup_timeout: float = _DEFAULT_STARTUP_TIMEOUT) -> socket.socket:
    """Connect to the daemon. Spawn one if it isn't already running.

    Args:
        startup_timeout: Seconds to wait for a freshly-spawned daemon to come up.

    Raises:
        DaemonStartupError: If the daemon couldn't be located, spawned, or
            reached within ``startup_timeout``.
    """
    try:
        return _open_socket()
    except (FileNotFoundError, ConnectionRefusedError, OSError, ValueError):
        pass

    with _file_lock():
        # A peer may have spawned one while we waited for the lock.
        try:
            return _open_socket()
        except (FileNotFoundError, ConnectionRefusedError, OSError, ValueError):
            pass

        _spawn_detached()

        deadline = time.monotonic() + startup_timeout
        last_err: Exception | None = None
        while time.monotonic() < deadline:
            try:
                return _open_socket()
            except (FileNotFoundError, ConnectionRefusedError, OSError, ValueError) as e:
                last_err = e
                time.sleep(0.05)

        raise DaemonStartupError(
            f"daemon did not become reachable within {startup_timeout}s: {last_err!r}"
        )
