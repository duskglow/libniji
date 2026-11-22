#!/usr/bin/env python3
"""STARTUP-REPROFILE: per-function difference of two incl.py fn.tsv tables (B minus A), divided by DIV.
usage: fndiff.py A.fn.tsv B.fn.tsv DIV [N]   prints the N largest |inclusive| and |exclusive| differences."""
import sys


def load(p):
    d = {}
    for line in open(p):
        if line.startswith("#"):
            continue
        i, e, s = line.rstrip("\n").split("\t")
        d[s] = (int(i), int(e))
    return d


def short(s):
    return s.replace("niji_boundary::", "b::").replace("niji_ld::", "").replace("<niji_ld::arch::X86_64, niji_boundary::ld::image::ProcImage>", "")[:95]


a, b = load(sys.argv[1]), load(sys.argv[2])
div = float(sys.argv[3])
n = int(sys.argv[4]) if len(sys.argv) > 4 else 25
keys = set(a) | set(b)
d = {k: ((b.get(k, (0, 0))[0] - a.get(k, (0, 0))[0]) / div, (b.get(k, (0, 0))[1] - a.get(k, (0, 0))[1]) / div) for k in keys}
print("-- exclusive difference")
for k in sorted(keys, key=lambda k: -abs(d[k][1]))[:n]:
    print(f"  {d[k][1]:10.0f}  (incl {d[k][0]:10.0f})  {short(k)}")
print("-- inclusive difference")
for k in sorted(keys, key=lambda k: -abs(d[k][0]))[:n]:
    print(f"  {d[k][0]:10.0f}  {short(k)}")
