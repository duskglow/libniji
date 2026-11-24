#!/usr/bin/env python3
"""KERNEL-RESIDUAL-PROFILE: the kernel cost of each loader's mapping calls, and of variants of libniji's pattern.

  python3 replay.py OUTDIR      (OUTDIR: a krp.py output directory holding strace.<program>.<kit>)

From each strace log (one process), the mmap/munmap/mprotect/brk calls become a replay script (replay.c); every
address is re-expressed as an earlier mapping's result plus an offset (calls on the kernel's own mappings of the
program, the loader and the stack are not replayable and are left out, counted). The replay runs in a fresh
static process with the mappings torn down at its exit, so the count is the calls' work plus their exit teardown,
without page faults (nothing is touched). Each count is perf stat's median of 21 of `replay OPS 1` minus that of
`replay OPS 0` (parse and open only).

Variants of libniji's script, each removing one pattern (the replay keeps every other call as recorded):
  noview     no whole-file views: no view arena (its reservation, the views mapped into it, its unmap) and no
             view mapped alone (and its unmap)
  noreserve  no PROT_NONE reservation per object: the first segment is mapped over the whole span from the file
             (offset 0) and the other segments MAP_FIXED over it
  exact      no over-sized mapping trimmed afterwards: a mapping later cut at its start or end is made at its
             final extent (the heap's first region, aligned reservations)
  all        the three together
"""
import os
import re
import statistics
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "out")
PROT = {"PROT_NONE": 0, "PROT_READ": 1, "PROT_WRITE": 2, "PROT_EXEC": 4}
MAPF = {"MAP_SHARED": 1, "MAP_PRIVATE": 2, "MAP_FIXED": 0x10, "MAP_ANONYMOUS": 0x20, "MAP_GROWSDOWN": 0x100,
        "MAP_DENYWRITE": 0x800, "MAP_NORESERVE": 0x4000, "MAP_POPULATE": 0x8000, "MAP_STACK": 0x20000,
        "MAP_FIXED_NOREPLACE": 0x100000}
RE_MMAP = re.compile(r"^\d+\s+mmap\((\w+), (\d+), ([\w|]+), ([\w|]+), (-1|\d+(?:<([^>]*)>)?), (\w+)\)\s+= (0x[0-9a-f]+)")
RE_UNMAP = re.compile(r"^\d+\s+munmap\((0x[0-9a-f]+), (\d+)\)\s+= 0")
RE_PROT = re.compile(r"^\d+\s+mprotect\((0x[0-9a-f]+), (\d+), ([\w|]+)\)\s+= 0")
RE_BRK = re.compile(r"^\d+\s+brk\((\w+)\)\s+= (0x[0-9a-f]+)")


def bits(s, table):
    v = 0
    for t in s.split("|"):
        v |= table.get(t, 0) if not t.startswith("0x") else int(t, 16)
    return v


def parse(log):
    ops, brk0 = [], None
    for ln in open(log):
        m = RE_MMAP.match(ln)
        if m:
            addr = 0 if m.group(1) == "NULL" else int(m.group(1), 16)
            ops.append({"k": "m", "addr": addr, "len": int(m.group(2)), "prot": bits(m.group(3), PROT),
                        "flags": bits(m.group(4), MAPF), "path": m.group(6), "pgoff": int(m.group(7), 0),
                        "ret": int(m.group(8), 16), "shift": 0})
            continue
        m = RE_UNMAP.match(ln)
        if m:
            ops.append({"k": "u", "addr": int(m.group(1), 16), "len": int(m.group(2))})
            continue
        m = RE_PROT.match(ln)
        if m:
            ops.append({"k": "p", "addr": int(m.group(1), 16), "len": int(m.group(2)), "prot": bits(m.group(3), PROT)})
            continue
        m = RE_BRK.match(ln)
        if m:
            if m.group(1) == "NULL":
                brk0 = brk0 or int(m.group(2), 16)
            elif brk0:
                ops.append({"k": "b", "len": int(m.group(1), 16) - brk0})
    for i, o in enumerate(ops):
        o["i"] = i
    return ops


def owner(ops, upto, addr):
    """The latest mapping before op `upto` whose recorded range holds `addr`."""
    for o in reversed(ops[:upto]):
        if o["k"] == "m" and not o.get("drop") and o["ret"] <= addr < o["ret"] + o["len"] + 4095 & ~4095:
            return o
    return None


def emit(ops, path):
    files, lines, skipped = {}, [], 0
    for o in ops:
        if o.get("drop"):
            continue
        if o["k"] == "b":
            lines.append(f"brk {o['len']}")
            continue
        if o["k"] == "m":
            fd = -1
            if o["path"]:
                fd = files.setdefault(o["path"], len(files))
            if o["flags"] & (MAPF["MAP_FIXED"] | MAPF["MAP_FIXED_NOREPLACE"]):
                b = owner(ops, o["i"], o["addr"])
                if b is None:
                    skipped += 1
                    continue
                base, off = b["i"], o["addr"] - (b["ret"] + b["shift"])
            else:
                base, off = -1, 0
            lines.append(f"mmap {o['i']} {base} {off} {o['len']} {o['prot']} {o['flags']} {fd} {o['pgoff']}")
            continue
        b = owner(ops, o["i"], o["addr"])
        if b is None:
            skipped += 1
            continue
        off = o["addr"] - (b["ret"] + b["shift"])
        if o["k"] == "u":
            lines.append(f"munmap {b['i']} {off} {o['len']}")
        else:
            lines.append(f"mprotect {b['i']} {off} {o['len']} {o['prot']}")
    with open(path, "w") as f:
        for p, i in files.items():
            f.write(f"open {i} {p}\n")
        f.write("\n".join(lines) + "\n")
    return len(lines), skipped


def is_view(o):
    if o["k"] != "m" or not o["path"] or o["pgoff"] or o["prot"] != 1:
        return False
    try:
        return o["len"] == os.stat(o["path"]).st_size
    except OSError:
        return False


def noview(ops):
    for o in ops:
        if o["k"] == "m" and o["path"] is None and o["prot"] == 0 and o["flags"] & MAPF["MAP_NORESERVE"] and o["len"] >= 64 << 20:
            o["drop"] = True
        elif is_view(o):
            o["drop"] = True
    # A view mapped alone sits in an aligned reservation of its own: no view, no reservation.
    for r in ops:
        if r["k"] == "m" and not r.get("drop") and r["path"] is None and r["prot"] == 0:
            inside = [o for o in ops[r["i"] + 1:] if o["k"] == "m" and o["flags"] & MAPF["MAP_FIXED"]
                      and r["ret"] <= o["addr"] < r["ret"] + r["len"]]
            if inside and all(o.get("drop") for o in inside):
                r["drop"] = True
    for o in ops:
        if o["k"] in "up":
            b = owner([x for x in ops], o["i"], o["addr"])
            if b is None:
                for x in reversed(ops[:o["i"]]):
                    if x["k"] == "m" and x.get("drop") and x["ret"] <= o["addr"] < x["ret"] + x["len"]:
                        o["drop"] = True
                        break


def exact(ops):
    for o in ops:
        if o["k"] != "u" or o.get("drop"):
            continue
        b = owner(ops, o["i"], o["addr"])
        if b is None or b["flags"] & MAPF["MAP_FIXED"]:
            continue
        start, end = b["ret"] + b["shift"], b["ret"] + b["len"]
        if o["addr"] == start and o["addr"] + o["len"] < end:
            b["shift"] += o["len"]
            b["len"] -= o["len"]
            b["ret_eff"] = True
            o["drop"] = True
        elif o["addr"] > start and o["addr"] + o["len"] >= end:
            b["len"] -= end - o["addr"]
            o["drop"] = True


def noreserve(ops):
    for r in ops:
        if r["k"] != "m" or r.get("drop") or r["path"] or r["prot"] != 0 or r["flags"] & MAPF["MAP_NORESERVE"]:
            continue
        start = r["ret"] + r["shift"]
        segs = [o for o in ops[r["i"] + 1:] if o["k"] == "m" and not o.get("drop") and o["flags"] & MAPF["MAP_FIXED"]
                and o["path"] and o["addr"] == start]
        if not segs:
            continue
        s = segs[0]
        r.update(path=s["path"], prot=s["prot"], pgoff=s["pgoff"], flags=MAPF["MAP_PRIVATE"])
        s["drop"] = True


def counts(ops_file):
    res = {}
    for mode in ("1", "0"):
        vals = []
        for _ in range(21):
            r = subprocess.run(["perf", "stat", "-x,", "-e", "instructions:k,cycles:k", "--", "./replay", ops_file, mode],
                               capture_output=True, text=True)
            d = {}
            for ln in r.stderr.splitlines():
                f = ln.split(",")
                if len(f) > 3:
                    try:
                        d[f[2]] = float(f[0])
                    except ValueError:
                        pass
            vals.append(d)
        res[mode] = {k: statistics.median(v[k] for v in vals) for k in ("instructions:k", "cycles:k")}
        res["rc" + mode] = r.returncode
    return {k: res["1"][k] - res["0"][k] for k in res["1"]}, res["rc1"]


def main():
    os.chdir(OUT)
    r = subprocess.run(["gcc", "-O2", "-static", os.path.join(HERE, "replay.c"), "-o", "replay"], capture_output=True, text=True)
    if r.returncode:
        sys.exit("BUILD-FAIL " + r.stderr[-500:])
    print("== replay: kernel instructions / cycles of the mapping calls alone (median of 21, minus the baseline)")
    for name in ("empty", "true", "ls", "sed"):
        rows = []
        for kit, variant in (("ref", "as recorded"), ("niji", "as recorded"), ("niji", "noview"), ("niji", "noreserve"),
                             ("niji", "exact"), ("niji", "all")):
            ops = parse(f"strace.{name}.{kit}")
            if variant in ("noview", "all"):
                noview(ops)
            if variant in ("exact", "all"):
                exact(ops)
            if variant in ("noreserve", "all"):
                noreserve(ops)
            f = f"replay.{name}.{kit}.{variant.replace(' ', '_')}"
            n, skipped = emit(ops, f)
            c, rc = counts(f)
            rows.append((kit, variant, n, skipped, c, rc))
        base = rows[1][4]["instructions:k"]
        for kit, variant, n, skipped, c, rc in rows:
            print(f"   {name:6} {kit:4} {variant:12} calls {n:3} (skipped {skipped})  ik {c['instructions:k']:9.0f}"
                  f"  ck {c['cycles:k']:9.0f}  vs niji {c['instructions:k'] - base:+9.0f}  rc {rc}")
    print("\n== done")


main()
