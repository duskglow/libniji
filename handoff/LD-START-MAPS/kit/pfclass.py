#!/usr/bin/env python3
"""LD-START-MAPS: classify page-fault samples (perf script -F comm,ip,addr,sym,dso --show-mmap-events)
by the mapping holding the faulting data address. Anonymous mappings are not in perf's mmap events,
so the strace log (same program, ASLR off) supplies every mmap with its arguments.
usage: pfclass.py PF STRACE"""
import re
import sys

pf, st = sys.argv[1], sys.argv[2]
maps = []  # (start, end, label), later ones override earlier ones
for line in open(st):
    m = re.search(r'mmap\((\w+), (\d+), ([A-Z_|]+), ([A-Z_|]+), (-?\d+), (\w+)\) = (0x[0-9a-f]+)', line)
    if m:
        at = int(m.group(7), 16)
        ln = int(m.group(2))
        maps.append((at, at + ln, f"mmap {m.group(3)} fd{m.group(5)} off{m.group(6)} len{ln}"))
for line in open(pf):
    m = re.search(r'PERF_RECORD_MMAP2 \S+ \[(0x[0-9a-f]+)\((0x[0-9a-f]+)\) @ (\S+) .*\]: (\S+) (\S+)', line)
    if m:
        at = int(m.group(1), 16)
        maps.append((at, at + int(m.group(2), 16), f"{m.group(5).split('/')[-1]} {m.group(4)} @{m.group(3)}"))
counts = {}
rows = []
for line in open(pf):
    if 'PERF_RECORD' in line:
        continue
    f = line.split()
    if len(f) < 3:
        continue
    try:
        addr = int(f[1], 16)
    except ValueError:
        continue
    label = 'stack/other'
    if addr >= 0x7ffffffde000:
        label = 'stack'
    for s, e, lab in maps:
        if s <= addr < e:
            label = lab
    rows.append((addr, label, ' '.join(f[2:])))
    counts[label] = counts.get(label, 0) + 1
for label, n in sorted(counts.items(), key=lambda x: -x[1]):
    print(f"{n:4d} {label}")
print(f"total {len(rows)}")
if len(sys.argv) > 3:
    for addr, label, rest in rows:
        print(f"  {addr:#x} {label} | {rest}")
