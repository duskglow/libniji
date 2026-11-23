#!/usr/bin/env python3
"""LD-LOOKUP-RETHINK: per-lookup difference of two attr.py outputs. usage: diff.py BASE.json MORE.json N KIND [TOP]
KIND: self, incl, inl, line, pair, obj. Prints (MORE - BASE) / N per key, largest first, and the total."""
import json, sys

a, b = json.load(open(sys.argv[1])), json.load(open(sys.argv[2]))
n, kind = float(sys.argv[3]), sys.argv[4]
top = int(sys.argv[5]) if len(sys.argv) > 5 else 40
d = {k: (b[kind].get(k, 0) - a[kind].get(k, 0)) / n for k in set(a[kind]) | set(b[kind])}
print("%s: total per unit %.1f  (%d -> %d)" % (kind, (b["total"] - a["total"]) / n, a["total"], b["total"]))
shown = 0.0
for k, v in sorted(d.items(), key=lambda kv: -abs(kv[1]))[:top]:
    if abs(v) < 0.5:
        break
    shown += v
    print("%8.1f  %s" % (v, k[:150]))
print("%8.1f  (shown)" % shown)
