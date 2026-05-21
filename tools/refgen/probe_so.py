"""Throwaway probe: find the article-body container in the Stack Overflow fixture."""
import re

with open("tests/corpus/fixtures/stackoverflow-question.html", "r", encoding="utf-8") as f:
    html = f.read()

ids = set(re.findall(r'id="([^"]{1,40})"', html))
candidates = sorted(
    i for i in ids
    if not i.startswith(("comments-", "answer-", "post-", "user", "answers"))
    and "avatar" not in i and i and not i[0].isdigit()
)
print("ids:")
for i in candidates[:40]:
    print(" ", i)

classes = re.findall(r'class="([^"]+)"', html)
class_tokens = set()
for cls in classes:
    for tok in cls.split():
        class_tokens.add(tok)
interesting = [t for t in class_tokens if any(k in t.lower() for k in ("post", "answer", "question", "body", "main", "content", "prose"))]
print("\ninteresting class tokens:")
for c in sorted(interesting)[:40]:
    print(" ", c)
