# tract — Python SDK

A thin Python wrapper over `tractd`, the daemon that fetches and extracts
clean LLM-ready markdown for any URL.

```python
from tract import fetch

content = fetch("https://example.com")
```

See the [main repo README](https://github.com/khangdang1690/tract) for the
full design and project status.

## Auto-spawn

On first use the SDK looks for a running daemon at the platform-default
endpoint and, if none is running, spawns one from the bundled binary or
from `$TRACTD_BINARY`. Concurrent first uses are serialized through a
per-user file lock.

## Errors

- `FetchFailedError(code, message)` — daemon returned a typed error (`invalid_url`, `fetch_failed`, `extract_failed`, etc.).
- `TractTimeoutError` — daemon didn't respond in time.
- `DaemonStartupError` — daemon couldn't be located or reached.
- `TractError` — base class for all of the above.
