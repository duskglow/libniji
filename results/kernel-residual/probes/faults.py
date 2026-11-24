#!/usr/bin/env python3
"""KERNEL-RESIDUAL-PROFILE: page faults by the class of mapping they land in, both loaders, mean of 5 runs.

  python3 faults.py OUTDIR      (run after krp.py, which builds the programs into OUTDIR)

Each fault's data address (perf record -e page-faults -c 1 -d) is placed in the latest mapping event that holds it:
  view        a whole-file read-only mapping (offset 0, length the file's size) - libniji's loader views
  text / rodata / data   file mappings r-x / r-- / rw- (data: copy-on-write or private write)
  anon        anonymous mappings and memory without a mapping event (heap, TLS, .bss tails)
  stack       the stack
Black box for the reference: addresses and mapping events only."""
import collections
import os
import re
import subprocess
import sys

sys.argv = [sys.argv[0], "--only", "none"] + sys.argv[1:]
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
MM = re.compile(r"PERF_RECORD_MMAP2?\s+\d+/\d+:\s+\[0x([0-9a-f]+)\(0x([0-9a-f]+)\)\s+@\s+(\S+)[^\]]*\]:\s+([rwxps-]{4})\s+(.*)$")


def classify(argv, env, runs=5):
    c = collections.Counter()
    for _ in range(runs):
        subprocess.run(["perf", "record", "-q", "-e", "page-faults", "-c", "1", "-d", "-o", "pf.data", "--"] + argv,
                       env=env, capture_output=True)
        out = subprocess.run(["perf", "script", "-i", "pf.data", "--show-mmap-events", "-F", "addr"],
                             capture_output=True, text=True).stdout
        maps = []
        for ln in out.splitlines():
            if "PERF_RECORD_MMAP" in ln:
                m = MM.search(ln)
                if m:
                    lo, ln_, pg, perm, nm = int(m.group(1), 16), int(m.group(2), 16), int(m.group(3), 0), m.group(4), m.group(5).split()[-1]
                    is_file = nm.startswith("/") and not nm.startswith("//") and os.path.exists(nm)
                    size = os.stat(nm).st_size if is_file else -1
                    if is_file and pg == 0 and perm.startswith("r--") and ln_ == (size + 4095) & ~4095:
                        k = "view"
                    elif not is_file:
                        k = "stack" if "[stack" in nm else "anon"
                    else:
                        k = {"r-x": "text", "r--": "rodata", "rw-": "data"}.get(perm[:3], "file other")
                    maps.append((lo, lo + ln_, k))
                continue
            s = ln.strip()
            if re.fullmatch(r"[0-9a-f]+", s):
                a = int(s, 16)
                k = next((k for lo, hi, k in reversed(maps) if lo <= a < hi), None)
                c[k or ("stack" if a >= 0x7ff000000000 else "anon")] += 1
    return {k: v / runs for k, v in c.items()}


def main():
    import krp
    os.chdir(krp.OUT)
    print("== faults by mapping class (mean of 5)")
    for name, kit, argv in krp.configs():
        c = classify(argv, krp.ENV)
        print(f"   {name:6} {kit:4} total {sum(c.values()):6.1f}  " + "  ".join(f"{k} {v:.1f}" for k, v in sorted(c.items())))
    os.unlink("pf.data")


main()
