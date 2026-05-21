"""One-off helper: extract a single rooted-by-id element as a flattened text
outline so I can read the article body without slogging through 400KB of
minified HTML. Not a checked-in tool — used during corpus curation.

Usage:
    python inspect_body.py <fixture.html> [<root_id>]
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from html.parser import HTMLParser


CHROME_TAGS = {
    "nav", "aside", "footer", "script", "style", "svg",
    "noscript", "form", "button", "select", "input", "header",
}


class Extractor(HTMLParser):
    def __init__(self, root_id: str | None):
        super().__init__()
        self.root_id = root_id
        self.depth = 0      # depth inside the root element
        self.skip = 0       # skip-tag stack
        self.out: list[str] = []
        self.activated = self.root_id is None  # if no id, treat whole body as root

    def _match_root(self, attrs):
        if not self.root_id:
            return False
        a = dict(attrs)
        if self.root_id.startswith("."):
            cls = a.get("class", "")
            return self.root_id[1:] in cls.split()
        return a.get("id") == self.root_id

    def handle_starttag(self, tag, attrs):
        if not self.depth and self._match_root(attrs):
            self.depth = 1
            self.out.append(f"<{tag}>")
            return
        # When no root_id provided, root is <body>.
        if self.root_id is None and tag == "body":
            self.depth = 1
            self.out.append(f"<{tag}>")
            return
        if not self.depth:
            return
        if tag in CHROME_TAGS:
            self.skip += 1
            return
        if self.skip:
            return
        # Track nested depth so we close the root correctly.
        self.depth += 1
        self.out.append(f"<{tag}>")

    def handle_endtag(self, tag):
        if not self.depth:
            return
        if self.skip and tag in CHROME_TAGS:
            self.skip -= 1
            return
        if self.skip:
            return
        self.out.append(f"</{tag}>")
        self.depth -= 1
        if self.depth == 0:
            # Stop after closing the root element.
            self.activated = False

    def handle_data(self, data):
        if not self.depth or self.skip:
            return
        d = re.sub(r"\s+", " ", data).strip()
        if d:
            self.out.append(d)


def main():
    if len(sys.argv) < 2:
        print("usage: inspect_body.py <fixture.html> [<root_id>]", file=sys.stderr)
        sys.exit(2)
    path = Path(sys.argv[1])
    root_id = sys.argv[2] if len(sys.argv) > 2 else None

    p = Extractor(root_id)
    p.feed(path.read_text(encoding="utf-8"))

    # Write to a sibling .txt so we don't trip Windows console codec.
    out = path.with_suffix(".body.txt")
    out.write_text("\n".join(p.out), encoding="utf-8")
    print(f"wrote {out}  ({sum(len(s) for s in p.out):,} chars)")


if __name__ == "__main__":
    main()
