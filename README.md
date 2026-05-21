# tract

> Give your agent the page, the way a human would read it.

`tract` is an open-source primitive that gives agents human-equivalent web
access through a single function call.

```python
from tract import fetch

content = fetch("https://example.com")
```

You get back clean LLM-ready markdown — not navigation, not ads, not banners,
not a CAPTCHA page that returned `200 OK`.

## Status

**Pre-alpha.** This repo is being built top-down from a design doc; the MVP
slice is in progress.

- See [docs/DESIGN.md](docs/DESIGN.md) for the full design — 12 components,
  the four irreducible operations, and how each binding constraint forces an
  architectural decision.
- The active MVP plan covers a thin vertical slice: Rust daemon + Python SDK
  + HTTP engine + SQLite cache + extractor. TLS/JA3 fingerprinting, browser
  engine, verifier, selector, and learning loop are explicitly v0.2.

## Architecture (MVP)

```
┌────────────────────────────────────┐
│ Python script                      │
│   from tract import fetch          │
└──────────────┬─────────────────────┘
               │ JSON over UDS / loopback TCP
┌──────────────▼─────────────────────┐
│ tractd (Rust async daemon)         │
│   IPC → orchestrator → cache       │
│              → http → extractor    │
└────────────────────────────────────┘
```

Four Rust crates:

| Crate | Role |
|---|---|
| `tract-proto` | IPC schema, shared with all SDKs |
| `tract-fetcher` | HTTP engine + SQLite cache |
| `tract-extractor` | HTML → readable markdown |
| `tractd` | Daemon binary: IPC listener + orchestrator |

Plus `clients/python/` — a thin Python SDK that auto-spawns the bundled
daemon binary on first use.

## Building from source

```
# Rust workspace
cargo build --workspace --release

# Python SDK (inside a venv — never install into system Python)
python -m venv .venv
.\.venv\Scripts\Activate.ps1     # Windows
# or: source .venv/bin/activate  # POSIX
pip install -e clients/python
```

## License

Apache-2.0 OR MIT.
