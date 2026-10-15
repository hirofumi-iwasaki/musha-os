# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
"""Fail-closed lookup for the pinned compiler's measurement-only assembly."""
import re


def body(source, suffix, namespace=None):
    matches = list(re.finditer(r'^([^\s:]*' + re.escape(suffix) + r'):\n', source, re.M))
    if namespace is not None:
        # Rust v0: the length-prefixed crate/module path must be followed by
        # another nested-path component (N), not a longer module identifier.
        path = re.compile(re.escape(namespace) + r'(?=N)')
        matches = [match for match in matches if path.search(match.group(1))]
    assert len(matches) == 1, (namespace, suffix, len(matches))
    return source[matches[0].end():].split('\n\t.def\t', 1)[0]
