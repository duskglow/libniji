#!/usr/bin/env python3
"""Summarise collcontr outputs: one line per locale with out-of-order pairs,
`<locale><TAB><count><TAB><pairs as text>`.  usage: contr_summary.py <file>...
No spec clause (probe tooling)."""
import os
import sys

for path in sorted(sys.argv[1:]):
    pairs = []
    with open(path) as f:
        for line in f:
            p = line.split()
            if p and p[0] == "CONTRACTION":
                pairs.append(chr(int(p[1][2:], 16)) + chr(int(p[2][2:], 16)))
    if pairs:
        name = os.path.basename(path).rsplit(".", 1)[0]
        print("%s\t%d\t%s" % (name, len(pairs), " ".join(pairs)))
