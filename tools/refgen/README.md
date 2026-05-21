# refgen — corpus fixture generator

Reads `tests/corpus/urls.toml`, launches headed Chromium via Playwright, and
writes one `tests/corpus/fixtures/<id>.html` per entry. `synthetic://` URLs
are skipped — their HTML lives in `fixtures/` already (authored by hand).

Re-running overwrites HTML fixtures but never touches `expected/`. The
`expected/<id>.md` files are hand-curated and are what the eval harness scores
against.

## Setup

Always inside a `.venv` — never `pip install` globally.

```powershell
# from tools/refgen/
python -m venv .venv
.venv\Scripts\Activate.ps1
pip install --upgrade pip
pip install playwright tomli
python -m playwright install chromium
```

## Usage

```powershell
# All entries
python refgen.py

# A subset
python refgen.py --only example-com,wikipedia-erlang
```

## Curating `expected/<id>.md`

Refgen writes a stub `expected/<id>.md` if one doesn't exist, containing a
`<!-- TODO: hand-curate -->` marker. The human's job: open
`fixtures/<id>.html` in a browser, decide what a careful human would call
"the article," and write that as markdown in `expected/<id>.md`.

The eval harness scores extractor output against these files. Garbage in,
garbage out — take the time.
