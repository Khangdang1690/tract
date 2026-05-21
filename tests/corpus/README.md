# tract reference corpus

The eval harness (`cargo run -p eval`) scores extractor output against this
corpus. Coverage is the §2.3 commitment of Phase A.

## Layout

- `urls.toml` — entries: id, url, bucket, expected_kind, notes.
- `fixtures/<id>.html` — captured HTML for each entry.
- `expected/<id>.md` — hand-curated markdown for what the extractor *should*
  return. Authored by a careful human reading `fixtures/<id>.html`.
- `baseline.json` — written by `cargo run -p eval`. Committed.

## Status (Phase A seed)

Seeded (committed):

| bucket       | committed | source                                                  |
|--------------|-----------|---------------------------------------------------------|
| trivial      | 2 / 6     | `example-com`, `static-blog-post` (synthetic)           |
| moderate     | 0 / 10    | needs refgen + curation                                 |
| hard         | 0 / 6     | needs refgen + curation (most will fail until Phase D/E)|
| adversarial  | 4 / 4     | synthetic block/challenge/stub/wrong-page pages         |

**The remaining 19 entries require `tools/refgen` + hand-curated `expected/<id>.md`.**

## Generating fixtures

See `tools/refgen/README.md`. Short version:

```powershell
cd tools/refgen
pip install playwright tomli
python -m playwright install chromium
python refgen.py                          # all entries
python refgen.py --only wikipedia-erlang  # one entry
```

Refgen writes `fixtures/<id>.html` and (if missing) a stub `expected/<id>.md`
with a TODO marker. **You must replace the stub with a careful human's
markdown.** The eval is only as good as the expected files.

## What to write in `expected/<id>.md`

Open `fixtures/<id>.html` in a browser. Decide what a literate human would
call "the article" — usually:

- the title (one `# Heading`)
- the article body, in reading order
- inline links and emphasis preserved
- headings preserved at their nesting level
- nav, footer, sidebar, comments, ads, "related posts" excluded
- tables as GitHub-flavored pipe tables (or a `Table:` caption + rows for
  complex tables, post-Phase B)

It is not extractor output. If you copy-paste extractor output, the eval is
circular and tells you nothing.
