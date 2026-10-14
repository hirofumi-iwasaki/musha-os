#!/usr/bin/env python3
"""Check local Markdown file links in README and docs (not URLs or anchors).

Run from any directory. Fenced code and inline code are ignored. Reference-style
links are checked at their definitions. Fragment targets are outside this check.
"""
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]

def links(text):
    text = re.sub(r'^\s*(```|~~~).*?^\s*\1\s*$', '', text, flags=re.M | re.S)
    text = re.sub(r'(`+).*?\1', '', text)
    pattern = r'!?\[[^\]\n]*\]\(\s*(<[^>]+>|[^\s)]+)(?:\s+[^)]*)?\)'
    for match in re.finditer(pattern, text):
        yield text.count('\n', 0, match.start()) + 1, match.group(1).strip('<>')
    for match in re.finditer(r'^\s*\[[^\]]+\]:\s*(<[^>]+>|\S+)', text, re.M):
        yield text.count('\n', 0, match.start()) + 1, match.group(1).strip('<>')

def main():
    files = [ROOT / 'README.md', *sorted((ROOT / 'docs').rglob('*.md'))]
    checked = 0
    errors = []
    for file in files:
        for line, target in links(file.read_text()):
            url = urlsplit(target)
            if url.scheme or url.netloc or not url.path:
                continue
            checked += 1
            path = Path(unquote(url.path))
            destination = path if path.is_absolute() else file.parent / path
            if not destination.exists():
                errors.append(f'{file.relative_to(ROOT)}:{line}: {target}')
    for error in errors:
        print(error)
    print(f'{len(files)} Markdown files, {checked} local file links, {len(errors)} missing targets (anchors and remote URLs excluded)')
    return bool(errors)

if __name__ == '__main__':
    sys.exit(main())
