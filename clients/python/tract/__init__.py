"""tract — human-equivalent web fetch for agents.

>>> from tract import fetch
>>> content = fetch("https://example.com")
"""

from ._client import FetchResult, fetch
from ._errors import (
    DaemonStartupError,
    FetchFailedError,
    TractError,
    TractTimeoutError,
)

__version__ = "0.1.0"

__all__ = [
    "fetch",
    "FetchResult",
    "TractError",
    "DaemonStartupError",
    "FetchFailedError",
    "TractTimeoutError",
]
