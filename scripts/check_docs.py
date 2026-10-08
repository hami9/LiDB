#!/usr/bin/env python3
"""Validate local Markdown link targets without network dependencies."""
from pathlib import Path
import re
import sys
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
errors = []
count = 0

for md in sorted(ROOT.rglob("*.md")):
    if any(part in {"target", "build", ".git"} for part in md.relative_to(ROOT).parts):
        continue
    text = md.read_text(encoding="utf-8")
    for raw in LINK.findall(text):
        target = raw.strip().split(" ", 1)[0].strip("<>")
        if not target or target.startswith(("https://", "http://", "mailto:", "#")):
            continue
        path = unquote(target.split("#", 1)[0].split("?", 1)[0])
        if not path:
            continue
        count += 1
        resolved = (md.parent / path).resolve()
        if not resolved.is_relative_to(ROOT) or not resolved.exists():
            errors.append(f"{md.relative_to(ROOT)}: broken link {raw}")

print(f"Checked {count} local Markdown links across project documentation.")
if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
print("Local Markdown links: PASS")
