#!/usr/bin/env python3
"""LD-START-MAPS: kernel instructions per start charged to the user instruction that entered the kernel.
Input: perf script -F comm,ip,uregs --show-mmap-events of REPS runs, sample period PERIOD.
usage: ki.py KI PERIOD REPS DIR   (DIR holds the objects, symbolized with nm by file name)"""
import bisect
import os
import re
import subprocess
import sys

ki, period, reps, d = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
syms = {}


def symbolize(path, off):
    name = os.path.basename(path)
    local = os.path.join(d, name)
    if not path.startswith('/tmp/niji-lsm') or not os.path.exists(local):
        local = path
    if local not in syms:
        try:
            out = subprocess.run(['nm', '-n', '--defined-only', local], capture_output=True, text=True).stdout
        except OSError:
            out = ''
        rows = []
        for line in out.splitlines():
            f = line.split()
            if len(f) == 3 and f[1] in 'tTwW':
                rows.append((int(f[0], 16), f[2]))
        syms[local] = rows
    rows = syms[local]
    i = bisect.bisect_right([r[0] for r in rows], off) - 1
    return f"{name}:{rows[i][1][:70]}" if i >= 0 else f"{name}:{off:#x}"


maps = []
counts = {}
for line in open(ki):
    m = re.search(r'PERF_RECORD_MMAP2 \S+ \[(0x[0-9a-f]+)\((0x[0-9a-f]+)\) @ (0x[0-9a-f]+|0) .*\]: r-xp (\S+)', line)
    if m:
        maps.append((int(m.group(1), 16), int(m.group(2), 16), int(m.group(3), 16), m.group(4)))
        continue
    m = re.search(r'IP:0x([0-9a-f]+)', line)
    if not m:
        continue
    ip = int(m.group(1), 16)
    label = f"?{ip:#x}"
    for start, ln, pg, path in reversed(maps):
        if start <= ip < start + ln:
            label = symbolize(path, ip - start + pg) if not path.startswith('[') else path
            break
    if 'niji_sys_' in label:
        si = re.search(r'SI:0x([0-9a-f]+)', line)
        label += f" len={int(si.group(1), 16) if si else -1}"
    counts[label] = counts.get(label, 0) + 1
tot = sum(counts.values())
print(f"total kernel instructions per run ~{tot * period // reps}  ({tot} samples)")
for label, n in sorted(counts.items(), key=lambda x: -x[1])[:40]:
    print(f"{n * period // reps:8d} {label}")
