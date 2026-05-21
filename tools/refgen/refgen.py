"""Reference-corpus fixture generator.

Reads ../../tests/corpus/urls.toml. For each entry whose URL has a real scheme
(http/https), launches a headed Chromium instance, navigates, waits for the
network to settle, and writes page.content() to ../../tests/corpus/fixtures/<id>.html.

`synthetic://...` URLs are skipped — their HTML is authored locally and must
already exist in fixtures/.

`expected/<id>.md` is NEVER overwritten. If missing, refgen writes a stub
containing a single TODO marker for a human to fill in.

Usage:
    pip install playwright tomli
    python -m playwright install chromium
    python refgen.py
    python refgen.py --only example-com,wikipedia-erlang
"""

from __future__ import annotations

import argparse
import asyncio
import sys
from pathlib import Path
from typing import Iterable

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore[no-redef]

from playwright.async_api import async_playwright

REPO_ROOT = Path(__file__).resolve().parents[2]
CORPUS_DIR = REPO_ROOT / "tests" / "corpus"
URLS_TOML = CORPUS_DIR / "urls.toml"
FIXTURES_DIR = CORPUS_DIR / "fixtures"
EXPECTED_DIR = CORPUS_DIR / "expected"

SETTLE_MS = 2_000
NAV_TIMEOUT_MS = 30_000


def load_entries() -> list[dict]:
    with URLS_TOML.open("rb") as f:
        data = tomllib.load(f)
    return list(data.get("entries", []))


def select(entries: Iterable[dict], only: set[str] | None) -> list[dict]:
    if only is None:
        return list(entries)
    return [e for e in entries if e["id"] in only]


async def fetch_one(browser, entry: dict) -> tuple[str, bool, str]:
    """Return (id, ok, message)."""
    eid: str = entry["id"]
    url: str = entry["url"]

    if url.startswith("synthetic://"):
        return eid, True, "skipped (synthetic; HTML authored locally)"

    page = await browser.new_page()
    try:
        # Two-stage wait: try networkidle (best for static pages), fall back
        # to load + extra settle for ad/tracker-heavy pages where networkidle
        # never converges (Stack Overflow, news sites, ecommerce).
        try:
            await page.goto(url, wait_until="networkidle", timeout=15_000)
        except Exception:
            await page.goto(url, wait_until="load", timeout=NAV_TIMEOUT_MS)
        await page.wait_for_timeout(SETTLE_MS)
        html = await page.content()
    except Exception as exc:  # noqa: BLE001
        return eid, False, f"fetch error: {exc!s}"
    finally:
        await page.close()

    out = FIXTURES_DIR / f"{eid}.html"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(html, encoding="utf-8")
    return eid, True, f"wrote {out.relative_to(REPO_ROOT)} ({len(html):,} bytes)"


def ensure_expected_stub(eid: str) -> None:
    """Create an empty expected/<id>.md with a TODO marker if missing.

    Never overwrites an existing file — the curated content lives there.
    """
    EXPECTED_DIR.mkdir(parents=True, exist_ok=True)
    stub = EXPECTED_DIR / f"{eid}.md"
    if stub.exists():
        return
    stub.write_text(
        f"<!-- TODO: hand-curate expected markdown for `{eid}` -->\n",
        encoding="utf-8",
    )


async def amain(only: set[str] | None) -> int:
    entries = select(load_entries(), only)
    if not entries:
        print("no matching entries", file=sys.stderr)
        return 1

    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=False)
        try:
            failures = 0
            for entry in entries:
                eid, ok, msg = await fetch_one(browser, entry)
                ensure_expected_stub(eid)
                status = "OK " if ok else "FAIL"
                print(f"[{status}] {eid}: {msg}")
                if not ok:
                    failures += 1
        finally:
            await browser.close()

    return 1 if failures else 0


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate corpus HTML fixtures.")
    parser.add_argument(
        "--only",
        type=str,
        default=None,
        help="Comma-separated entry ids to fetch (default: all)",
    )
    args = parser.parse_args()
    only = set(s.strip() for s in args.only.split(",")) if args.only else None
    rc = asyncio.run(amain(only))
    sys.exit(rc)


if __name__ == "__main__":
    main()
