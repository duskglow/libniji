#!/usr/bin/env python3
"""CONFIGURE-DIFF: group diff.tsv by probe name across packages (the directory part of the key dropped).

Usage: group.py <run dir> [--all]   Prints one line per probe name: packages, stock -> libniji values.
Without --all only primary probes (autoconf cache, meson, cmake, perl) are listed; generated-header defines
(h:) and autoconf output variables (acout:) mostly repeat a primary probe's answer.
"""
import collections
import sys

run = sys.argv[1]
allk = "--all" in sys.argv
g = collections.OrderedDict()
with open(run + "/diff.tsv") as f:
    next(f)
    for line in f:
        pkg, kl, key, s, n = line.rstrip("\n").split("\t")
        kind, rest = key.split(":", 1)
        if not allk and kind in ("h", "acout"):
            continue
        name = kind + ":" + rest.split(":", 1)[-1] if kind != "meson" else key.split(":", 2)[0] + ":" + key.split(":", 2)[2]
        e = g.setdefault(name, [kl, set(), set()])
        e[1].add(pkg)
        e[2].add("%s -> %s" % (s[:60], n[:60]))
for name, (kl, pk, vals) in sorted(g.items(), key=lambda x: (x[1][0], x[0])):
    print("%-6s %-60s %-40s %s" % (kl, name[:60], ",".join(sorted(pk))[:40], " | ".join(sorted(vals))[:150]))
