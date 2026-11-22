#!/usr/bin/env python3
"""STARTUP-REPROFILE: print, for each callee pattern, its inclusive count split by caller (from incl.py's edge.tsv),
and the children of each pattern (callees whose caller matches).
usage: edges.py EDGE_TSV PATTERN..."""
import sys

rows = []
for line in open(sys.argv[1]):
    c, callee, caller = line.rstrip("\n").split("\t")
    rows.append((int(c), callee, caller))


def short(s):
    return s.replace("niji_boundary::", "b::").replace("niji_ld::", "").replace("<niji_ld::arch::X86_64, b::ld::image::ProcImage>", "").replace("::<X86_64>", "")[:90]


for pat in sys.argv[2:]:
    print(f"== {pat}: callers")
    for c, callee, caller in rows:
        if pat in callee:
            print(f"  {c:7d} {short(callee)}  <- {short(caller)}")
    print(f"== {pat}: children")
    for c, callee, caller in rows:
        if pat in caller and c >= 500:
            print(f"  {c:7d} {short(callee)}")
