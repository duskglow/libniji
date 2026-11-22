#!/usr/bin/env python3
"""STARTUP-REPROFILE: inclusive and exclusive instruction counts per function from Intel PT samples with
synthesized call chains (perf script --itrace=i1ig256 -F comm,ip,sym), one sample per instruction.
usage: perf script ... | incl.py COMM OUTPREFIX
writes OUTPREFIX.fn.tsv (incl, excl, function; every function with incl >= 200) and OUTPREFIX.edge.tsv
(incl, callee, caller: inclusive count of callee under that caller, outermost occurrence; >= 200)."""
import collections
import sys

comm, out = sys.argv[1], sys.argv[2]
incl = collections.Counter()
excl = collections.Counter()
edge = collections.Counter()
total = 0
chain = []
keep = False


def flush():
    global total
    if not keep or not chain:
        return
    total += 1
    excl[chain[0]] += 1
    seen = set()
    # chain[0] is the leaf; walk from the root down so the outermost occurrence wins
    n = len(chain)
    for i in range(n - 1, -1, -1):
        s = chain[i]
        if s in seen:
            continue
        seen.add(s)
        incl[s] += 1
        caller = chain[i + 1] if i + 1 < n else "-"
        edge[(s, caller)] += 1


for line in sys.stdin:
    if not line.strip():
        continue
    if not line[0].isspace():
        flush()
        chain = []
        keep = line.split()[0] == comm
        continue
    parts = line.split(None, 1)
    sym = parts[1].strip() if len(parts) > 1 else "?"
    # drop a +0x offset if perf printed one
    sym = sym.split("+0x")[0]
    chain.append(sym)
flush()

with open(out + ".fn.tsv", "w") as f:
    f.write(f"# total samples (instructions) of comm {comm}: {total}\n")
    for s, c in incl.most_common():
        if c < 200:
            break
        f.write(f"{c}\t{excl[s]}\t{s}\n")
with open(out + ".edge.tsv", "w") as f:
    for (s, caller), c in edge.most_common():
        if c < 200:
            break
        f.write(f"{c}\t{s}\t{caller}\n")
print(f"{comm}: {total} samples, {len(incl)} functions")
