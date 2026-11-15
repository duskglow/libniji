#!/usr/bin/env python3
"""LAYOUT-TOOL-BOUND-SHARED: count and attribute the coreutils-shaped ("tool") probe's page faults of one tree.

    pf.py <tree> <tag> <outdir> [mode]

Links crates/core/tests/oracle/page_faults.c exactly as crates/core/tests/hot_text_products.rs links it (and a second
time with -Wl,-Map, which must give the same bytes), runs `relay <mode>` as the test does (ADDR_NO_RANDOMIZE, a
fixed-length private path, argv0 page_faults, cwd /, env LANG=en_US.UTF-8; one warm-up and 5 runs, lowest kept), then
records the same run with `perf record -e page-faults -c 1 -d` and attributes every fault of the counted (relayed)
process: page, 64 KiB window, output section, input section (map file), symbol at the address, symbol of the ip.
"""
import bisect
import ctypes
import os
import re
import subprocess
import sys

tree, tag, out = sys.argv[1:4]
mode = sys.argv[4] if len(sys.argv) > 4 else "tool"
os.makedirs(out, exist_ok=True)
crt1 = f"{tree}/target/liella/liella-crt1.o"
src = f"{tree}/crates/core/tests/oracle/page_faults.c"
liba = f"{tree}/target/liella/libc.a"
exe = f"{out}/{tag}.pf"
mapf = f"{out}/{tag}.map"
CC = ["cc", "-std=gnu17", "-O1", "-static", "-nostdlib", "-fno-builtin"]


def link(extra, o):
    subprocess.run(CC + extra + [crt1, src, liba, "-lgcc", "-o", o], check=True)


link([], exe)
link([f"-Wl,-Map={mapf}"], exe + ".m")
with open(exe, "rb") as a, open(exe + ".m", "rb") as b:
    same = a.read() == b.read()
os.unlink(exe + ".m")
print(f"pf: {tag}: map link identical to the test link: {same}", flush=True)

libc = ctypes.CDLL(None, use_errno=True)
libc.personality.restype = ctypes.c_int
cur = libc.personality(ctypes.c_ulong(0xFFFFFFFF))
libc.personality(ctypes.c_ulong(cur | 0x0040000))  # ADDR_NO_RANDOMIZE, inherited by every child

d = f"/tmp/liella-hot-text-{8000000000 + os.getpid():010}-0"
os.makedirs(d, exist_ok=True)
run = f"{d}/p"
subprocess.run(["cp", "--", exe, run], check=True, stdin=subprocess.DEVNULL)
seen = []
for _ in range(6):
    o = subprocess.run(["page_faults", "relay", mode], executable=run, cwd="/", env={"LANG": "en_US.UTF-8"},
                       stdin=subprocess.DEVNULL, capture_output=True, text=True)
    assert o.returncode == 0, (o.returncode, o.stderr)
    seen.append(int(o.stderr.strip().split()[1]))
print(f"pf: {tag}: minflt {mode}: warm-up {seen[0]}, runs {seen[1:]}, counted {min(seen[1:])}", flush=True)

data = f"{out}/{tag}.{mode}.perf.data"
helper = "import os,sys; os.execve(sys.argv[1], ['page_faults', 'relay', sys.argv[2]], {'LANG': 'en_US.UTF-8'})"
p = subprocess.run(["perf", "record", "-q", "-e", "page-faults", "-c", "1", "-d", "-o", data, "--", "python3", "-c",
                    helper, run, mode], cwd="/", stdin=subprocess.DEVNULL, capture_output=True, text=True)
print(f"pf: {tag}: perf record rc={p.returncode} minflt under perf: {p.stderr.strip()[-40:]!r}", flush=True)
s = subprocess.run(["perf", "script", "-i", data, "-F", "pid,addr,ip"], capture_output=True, text=True, check=True)
ev = []
for line in s.stdout.splitlines():
    t = line.split()
    if len(t) >= 3:
        ev.append((int(t[0]), int(t[1], 16), int(t[2], 16)))
pids = []
for e in ev:
    if e[0] not in pids:
        pids.append(e[0])
child = pids[-1]

secs = []
for line in subprocess.run(["readelf", "-SW", exe], capture_output=True, text=True, check=True).stdout.splitlines():
    m = re.match(r"\s*\[\s*\d+\]\s+(\S+)\s+\S+\s+([0-9a-f]{16})\s+[0-9a-f]+\s+([0-9a-f]+)\s", line)
    if m and int(m.group(2), 16):
        secs.append((int(m.group(2), 16), int(m.group(3), 16), m.group(1)))
secs.sort()
syms = []
for line in subprocess.run(["nm", "-n", "-S", "--defined-only", exe], capture_output=True, text=True, check=True).stdout.splitlines():
    t = line.split()
    if len(t) == 4:
        syms.append((int(t[0], 16), int(t[1], 16), t[3]))
    elif len(t) == 3:
        syms.append((int(t[0], 16), 0, t[2]))
syms.sort()
saddr = [x[0] for x in syms]
ins = []
pend = None
for line in open(mapf, errors="replace"):
    m = re.match(r"^ (\.\S+)\s*$", line)
    if m:
        pend = m.group(1)
        continue
    m = re.match(r"^ (\.\S+)?\s+0x([0-9a-f]+)\s+0x([0-9a-f]+)\s+(\S+)", line)
    if m and (m.group(1) or pend):
        a, n = int(m.group(2), 16), int(m.group(3), 16)
        if a and n:
            ins.append((a, n, m.group(1) or pend, os.path.basename(m.group(4))))
    pend = None
ins.sort()
iaddr = [x[0] for x in ins]
bss_end = max(a + n for a, n, _ in secs)


def where(a):
    for s_a, s_n, name in secs:
        if s_a <= a < s_a + s_n:
            return name
    if bss_end <= a < bss_end + (1 << 30):
        return "brk"
    if a >= 0x7FFFFFFD0000:
        return "stack"
    return "mmap"


def sym(a):
    i = bisect.bisect_right(saddr, a) - 1
    if i < 0:
        return "-"
    s_a, s_n, name = syms[i]
    off = a - s_a
    if s_n and off < s_n:
        return f"{name}+{off:#x}"
    return f"~{name}+{off:#x}"


def inp(a):
    i = bisect.bisect_right(iaddr, a) - 1
    if i < 0:
        return "-"
    i_a, i_n, name, f = ins[i]
    return f"{name}({f})" if a < i_a + i_n else f"~{name}({f})"


with open(f"{out}/{tag}.{mode}.faults", "w") as f:
    f.write("# n page win64k section input-section symbol@addr symbol@ip\n")
    n = 0
    for pid, addr, ip in ev:
        if pid != child:
            continue
        n += 1
        w = where(addr)
        inexe = w not in ("brk", "stack", "mmap")
        f.write(f"{n} {addr >> 12 << 12:#x} {addr >> 16 << 16:#x} {w} {inp(addr) if inexe else '-'} "
                f"{sym(addr) if inexe else '-'} {sym(ip)}\n")
print(f"pf: {tag}: perf events of the counted process (pid {child} of {pids}): {n}", flush=True)
with open(f"{out}/{tag}.sections", "w") as f:
    for s_a, s_n, name in secs:
        f.write(f"{s_a:#x} {s_n:#x} {name}\n")
subprocess.run(["rm", "-rf", "--", d], check=True)
