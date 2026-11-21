#!/usr/bin/env python3
"""LAYOUT-TOOL-BOUND-SHARED: compare two pf.py fault lists (A = base, B = other).

    cmp.py <A.faults> <B.faults>

Aligns the two fault sequences on (section, symbol at the address, symbol of the ip) with offsets and Rust hashes
dropped, prints every inserted, deleted or replaced fault with its full lines, then per output section the number
of distinct pages and 64 KiB windows of each list.
"""
import difflib
import re
import sys
from collections import defaultdict

H = re.compile(r"17h[0-9a-f]{16}E")


def load(p):
    return [l.split() for l in open(p) if not l.startswith("#")]


def key(t):
    strip = lambda s: H.sub("", re.sub(r"\+0x[0-9a-f]+$", "", s))
    return f"{t[3]} {strip(t[5])} {strip(t[6])}"


a, b = load(sys.argv[1]), load(sys.argv[2])
sm = difflib.SequenceMatcher(a=[key(t) for t in a], b=[key(t) for t in b], autojunk=False)
for op, i1, i2, j1, j2 in sm.get_opcodes():
    if op == "equal":
        continue
    print(op)
    for t in a[i1:i2]:
        print("  A", " ".join(t)[:260])
    for t in b[j1:j2]:
        print("  B", " ".join(t)[:260])


def summ(rows):
    pages, wins = defaultdict(set), defaultdict(set)
    for t in rows:
        pages[t[3]].add(t[1])
        wins[t[3]].add(t[2])
    return {k: (len(pages[k]), len(wins[k])) for k in pages}


sa, sb = summ(a), summ(b)
print(f"faults A {len(a)} B {len(b)}")
for k in sorted(set(sa) | set(sb)):
    flag = "" if sa.get(k) == sb.get(k) else "   <-- differs"
    print(f"  {k:24} pages/windows A {sa.get(k, (0, 0))} B {sb.get(k, (0, 0))}{flag}")
