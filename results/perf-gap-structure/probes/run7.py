#!/usr/bin/env python3
"""PERF-GAP-STRUCTURE-AUDIT probe 7: page faults of /bin/ls / under each loader (invoked explicitly), by the faulting
address's mapping. The system libraries (libselinux, libpcre2) are the same files for both loaders, so a difference
on their mappings is the loader's own access pattern. Mapping labels only; no symbols."""
import collections
import os
import re
import subprocess

SHA = "0495c4684330b90a6cecc02271ba1429296536ce"
P = os.path.expanduser("~/wl/products/" + SHA)
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "out5")
ENV = {"PATH": "/usr/bin:/bin", "HOME": "/tmp", "LANG": "C.UTF-8"}
MM = re.compile(r"PERF_RECORD_MMAP2?\s+\d+/\d+:\s+\[0x([0-9a-f]+)\(0x([0-9a-f]+)\)\s+@\s+(\S+)[^\]]*\]:\s+([rwxps-]{4})\s+(.*)$")


def run(label, argv):
    data = "pf7." + label
    subprocess.run(["perf", "record", "-q", "-e", "page-faults", "-c", "1", "-d", "-o", data, "--"] + argv,
                   env=ENV, capture_output=True, timeout=300)
    out = subprocess.run(["perf", "script", "-i", data, "--show-mmap-events", "-F", "addr"], capture_output=True,
                         text=True, timeout=300).stdout
    maps, c = [], collections.Counter()
    for ln in out.splitlines():
        if "PERF_RECORD_MMAP" in ln:
            m = MM.search(ln)
            if m:
                nm = m.group(5).strip().split()[-1]
                maps.append((int(m.group(1), 16), int(m.group(1), 16) + int(m.group(2), 16), m.group(4),
                             os.path.basename(nm) if nm.startswith("/") else nm, m.group(3)))
            continue
        s = re.match(r"^\s*([0-9a-f]+)\s*$", ln)
        if not s:
            continue
        a = int(s.group(1), 16)
        lab = "stack" if a >= 0x7FF000000000 else "heap / anonymous (no mmap event)"
        for lo, hi, perm, nm, off in reversed(maps):
            if lo <= a < hi:
                lab = f"{nm} {perm}"
                break
        c[lab] += 1
    return c


os.chdir(OUT)
ref = run("ls.ref", ["/lib64/ld-linux-x86-64.so.2", "--library-path", "/lib/x86_64-linux-gnu", "/bin/ls", "/"])
nij = run("ls.niji", [P + "-release-dyn/ld-linux-x86-64.so.2", "--library-path",
                      P + "-release-dyn:/lib/x86_64-linux-gnu", "/bin/ls", "/"])
print(f"/bin/ls / faults: reference loader {sum(ref.values())}, libniji loader {sum(nij.values())}")
print(f"{'mapping':52} {'ref':>5} {'niji':>5}")
for k in sorted(set(ref) | set(nij), key=lambda k: -(ref[k] + nij[k])):
    print(f"{k[:52]:52} {ref[k]:5} {nij[k]:5}")
print("== done")
