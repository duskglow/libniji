#!/usr/bin/env python3
"""PERF-GAP-STRUCTURE-AUDIT probe 8: who writes libniji's extra anonymous and stack pages. /bin/ls / under libniji's
loader only (libniji ld.so and libc.so.6; no reference code in the process): page-fault samples with a DWARF call
graph; per fault, the mapping of the faulting address and the first user-space frame of the call chain (and the
first frame inside libniji's objects)."""
import collections
import os
import re
import subprocess

SHA = "0495c4684330b90a6cecc02271ba1429296536ce"
P = os.path.expanduser("~/wl/products/" + SHA)
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "out5")
ENV = {"PATH": "/usr/bin:/bin", "HOME": "/tmp", "LANG": "C.UTF-8"}
MM = re.compile(r"PERF_RECORD_MMAP2?\s+\d+/\d+:\s+\[0x([0-9a-f]+)\(0x([0-9a-f]+)\)\s+@\s+\S+[^\]]*\]:\s+([rwxps-]{4})\s+(.*)$")
FRAME = re.compile(r"^\s+([0-9a-f]+)\s+(.*?)\s+\(([^()]*)\)\s*$")


def short(sym):
    s = re.sub(r"\+0x[0-9a-f]+$", "", sym)
    m = re.search(r"(niji_[a-z_]+?\d+[a-z_]+|niji_[a-z_0-9]+)", s)
    return (m.group(1) if m and s.startswith("_R") else s)[:60]


def main(prog, args, label):
    os.chdir(OUT)
    data = "pf8." + label
    argv = [P + "-release-dyn/ld-linux-x86-64.so.2", "--library-path", P + "-release-dyn:/lib/x86_64-linux-gnu", prog] + args
    subprocess.run(["perf", "record", "-q", "-e", "page-faults", "-c", "1", "-d", "--call-graph", "dwarf,16384", "-o",
                    data, "--"] + argv, env=ENV, capture_output=True, timeout=300)
    out = subprocess.run(["perf", "script", "-i", data, "--show-mmap-events", "-F", "addr,ip,sym,dso"],
                         capture_output=True, text=True, timeout=600).stdout
    maps, agg, total = [], collections.defaultdict(collections.Counter), collections.Counter()
    for block in out.split("\n\n"):
        lines = [ln for ln in block.splitlines() if ln.strip()]
        if not lines:
            continue
        for ln in lines:
            m = MM.search(ln)
            if m:
                nm = m.group(4).strip().split()[-1]
                maps.append((int(m.group(1), 16), int(m.group(1), 16) + int(m.group(2), 16), m.group(3),
                             os.path.basename(nm) if nm.startswith("/") else nm))
        head = next((ln for ln in lines if "PERF_RECORD" not in ln), None)
        if head is None:
            continue
        h = re.match(r"^\s*([0-9a-f]+)\b", head)
        if not h:
            continue
        a = int(h.group(1), 16)
        lab = "stack" if a >= 0x7FF000000000 else "heap / anonymous (no mmap event)"
        for lo, hi, perm, nm in reversed(maps):
            if lo <= a < hi:
                lab = f"{nm} {perm}"
                break
        frames = [FRAME.match(ln) for ln in lines[1:]]
        user = [f for f in frames if f and int(f.group(1), 16) < 0x800000000000]
        first = short(user[0].group(2)) + " [" + os.path.basename(user[0].group(3)) + "]" if user else "(no user frame)"
        own = next((short(f.group(2)) for f in user if os.path.basename(f.group(3)) in ("libc.so.6", "ld-linux-x86-64.so.2")), "-")
        total[lab] += 1
        agg[lab][first if own == "-" or own in first else first + " <- " + own] += 1
    print(f"\n== {label}: faults by mapping, then by first user frame (<- first libniji frame when different)")
    for lab, c in total.most_common():
        print(f"  {c:4}  {lab}")
        if lab.startswith(("anon", "stack", "[stack]", "heap", "libc.so.6 rw", "libc.so.6 r-x")):
            for k, v in agg[lab].most_common(8):
                print(f"          {v:3}  {k[:150]}")


main("/bin/ls", ["/"], "ls")
main("/bin/true", [], "true")
print("== done")
