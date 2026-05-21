"""Exception types raised by the tract SDK."""

from __future__ import annotations


class TractError(Exception):
    """Base class for all tract SDK errors."""


class DaemonStartupError(TractError):
    """Raised when the daemon could not be located, spawned, or reached."""


class FetchFailedError(TractError):
    """Raised when the daemon returned an error response.

    Attributes:
        code: One of the ProtoError codes (``invalid_url``, ``fetch_failed``, etc.).
        message: Human-readable message from the daemon.
    """

    def __init__(self, code: str, message: str) -> None:
        self.code = code
        self.message = message
        super().__init__(f"{code}: {message}")


class TractTimeoutError(TractError):
    """Raised when the daemon did not respond within the configured timeout."""
