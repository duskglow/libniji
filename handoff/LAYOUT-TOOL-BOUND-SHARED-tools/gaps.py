#!/usr/bin/env python3
"""LAYOUT-TOOL-BOUND-SHARED: gaps.py <map> <lo> <hi> [min_gap]: the input sections of a GNU ld map in [lo, hi) that
follow a gap of at least min_gap bytes (alignment padding), with the gap, plus the section count and bytes."""
import re
import sys

mapf, lo, hi = sys.argv[1], int(sys.argv[2], 16), int(sys.argv[3], 16)
min_gap = int(sys.argv[4]) if len(sys.argv) > 4 else 256
ins, pend = [], None
for line in open(mapf, errors="replace"):
    m = re.match(r"^ (\.\S+)\s*$", line)
    if m:
        pend = m.group(1)
        continue
    m = re.match(r"^ (\.\S+)?\s+0x([0-9a-f]+)\s+0x([0-9a-f]+)\s+(\S+)", line)
    if m and (m.group(1) or pend):
        a, n = int(m.group(2), 16), int(m.group(3), 16)
        if a and n:
            ins.append((a, n, m.group(1) or pend))
    pend = None
ins.sort()
prev_end, count, total = None, 0, 0
for a, n, name in ins:
    if lo <= a < hi:
        count += 1
        total += n
        if prev_end is not None and a - prev_end >= min_gap:
            name = re.sub(r"17h[0-9a-f]{16}E", "", name)[:110]
            print(f"  gap {a - prev_end:#8x} before {a:#x} size {n:#7x} {name}")
    prev_end = a + n if prev_end is None or a + n > prev_end else prev_end
print(f"  {count} sections, {total:#x} bytes in [{lo:#x}, {hi:#x})")
