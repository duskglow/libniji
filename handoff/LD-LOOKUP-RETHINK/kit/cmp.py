#!/usr/bin/env python3
"""LD-LOOKUP-RETHINK: compare meas2.sh counts. usage: cmp.py BASE/counts.txt NEW/counts.txt [N]
Prints each row's libniji user instructions (base, new, change) and glibc's, then per-lookup values (N imports)."""
import re, sys

def rows(path):
    out = {}
    for line in open(path):
        m = re.match(r"row: (\S+) niji user_median (\d+) .*\| glibc user_median (\d+)", line)
        if m:
            out[m.group(1)] = (int(m.group(2)), int(m.group(3)))
    return out

a, b = rows(sys.argv[1]), rows(sys.argv[2])
n = float(sys.argv[3]) if len(sys.argv) > 3 else 200.0
print("| row | libniji base | libniji new | change | glibc 2.39 |")
print("|---|---|---|---|---|")
for k in a:
    if k in b:
        print("| %s | %d | %d | %+d | %d |" % (k, a[k][0], b[k][0], b[k][0] - a[k][0], a[k][1]))
print("| per lookup | base | new | change | glibc |")
for lab, z, m in (("hit-1st", "a0", "a"), ("past-3", "b0", "b"), ("miss-4", "c0", "c"), ("miss-7", "cb0", "cb"),
                  ("hit-ver", "v0", "v"), ("hit-40B", "l0", "l"), ("miss-40B", "c0", "cl"), ("libc", "p0", "p")):
    if all(x in a and x in b for x in (z, m)):
        pa, pb, g = (a[m][0] - a[z][0]) / n, (b[m][0] - b[z][0]) / n, (a[m][1] - a[z][1]) / n
        print("| %s | %.1f | %.1f | %+.1f | %.1f |" % (lab, pa, pb, pb - pa, g))
