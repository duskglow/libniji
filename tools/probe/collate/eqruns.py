#!/usr/bin/env python3
"""Summarise runs of '=' (strcoll-equal) elements in a collorder output:
each run as first..last code point (in order), length, and whether the run's
code points are consecutive. usage: eqruns.py <tsv> [min-run]"""
import sys

seq = []
with open(sys.argv[1]) as f:
    for line in f:
        cp, lv = line.rstrip("\n").split("\t")
        seq.append((int(cp[2:], 16), lv))
minrun = int(sys.argv[2]) if len(sys.argv) > 2 else 50
runs, i = [], 0
while i < len(seq):
    j = i
    while j + 1 < len(seq) and seq[j + 1][1] == "=":
        j += 1
    if j > i:
        runs.append((i, j))
    i = j + 1
tot = sum(j - i + 1 for i, j in runs)
print("runs=%d members=%d" % (len(runs), tot))
for i, j in runs:
    if j - i + 1 >= minrun:
        cps = [seq[k][0] for k in range(i, j + 1)]
        print("pos %d len %d U+%04X..U+%04X min U+%04X max U+%04X" % (i, j - i + 1, cps[0], cps[-1], min(cps), max(cps)))
small = [(i, j) for i, j in runs if j - i + 1 < minrun]
print("small runs=%d, e.g.:" % len(small))
for i, j in small[:8]:
    print("  " + " ".join("U+%04X" % seq[k][0] for k in range(i, min(j + 1, i + 6))))
