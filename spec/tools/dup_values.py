#!/usr/bin/env python3
"""DUPVAL (HEADERS-STAGE2-GAPS-J): the value of every macro that libniji defines in more than one installed header
(headers/dup-listed.tsv), in each of those headers, against the reference.

    python3 spec/tools/dup_values.py [--include DIR]   # DIR: libniji's tree (default libc-include/)

Run on the reference system (Appendix C.0). For each listed header (not bits/ ones, which are not included
directly), one C unit per tree (`-std=gnu17 -D_GNU_SOURCE`, the header alone) holds one static object per listed
macro initialised with its value; the values are read back from the compiler's assembly (`.quad`), never from header
text. libniji's unit is compiled with `-nostdinc -isystem DIR`, the compiler's own include and the build host's
kernel UAPI headers (as header_check.py). A name that gives no integer constant on the reference (not a constant,
not defined there) is skipped; one the reference gives and libniji does not is NIJI-ONLY-FAIL; a different value is
DIFF. Prints one line per finding and `dupval: RESULT headers=H same=S diff=D niji-only-fail=F`; exit 1 when D or F
is not 0. Later snapshots rerun it (RELEASE-SNAPSHOT-1127A found DELETE in <protocols/talkd.h> and ABORT and NOP in
<scsi/scsi.h> this way; spec/abi/macro-values.tsv now gives their per-header values).
"""
import argparse
import collections
import os
import re
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(ROOT, "spec", "tools", "header_checks"))
from dup import LISTED, kernel_dir  # noqa: E402


def listed():
    by_hdr = collections.defaultdict(set)
    for line in open(os.path.join(ROOT, LISTED)):
        f = line.rstrip("\n").split("\t")
        if line.startswith("#") or len(f) < 3 or f[0] != "macro" or f[1].startswith("_"):
            continue
        for h in f[2].split(","):
            if not (h.startswith("bits/") or "/bits/" in h):
                by_hdr[h].add(f[1])
    return by_hdr


def values(work, hdr, names, flags):
    """{name: value} of the names that give an integer constant with `hdr` alone (failing lines dropped, retried)."""
    live = sorted(names)
    for _ in range(8):
        src = "#define _GNU_SOURCE 1\n#include <%s>\n" % hdr + "".join(
            "__attribute__ ((used)) static const long long niji_v%d = (long long) (%s);\n" % (i, n) for i, n in enumerate(live))
        p, s = os.path.join(work, "p.c"), os.path.join(work, "p.s")
        open(p, "w").write(src)
        r = subprocess.run(["gcc", "-std=gnu17", "-w", "-fmax-errors=0", "-S", "-o", s, p] + flags, capture_output=True, text=True, timeout=300)
        if r.returncode == 0:
            out, cur = {}, None
            for line in open(s):
                m = re.match(r"niji_v(\d+):", line)
                if m:
                    cur = int(m.group(1))
                    continue
                m = re.match(r"\s+\.(quad|zero)\s+(-?\d+)", line)
                if m and cur is not None:
                    out[live[cur]] = 0 if m.group(1) == "zero" else int(m.group(2))
                    cur = None
            return out
        bad = {int(m.group(1)) - 3 for m in re.finditer(r"p\.c:(\d+):\d+: error", r.stderr)}
        live = [n for i, n in enumerate(live) if i not in bad]
        if not bad or not live:
            return {}
    return {}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    a = ap.parse_args()
    work = tempfile.mkdtemp(prefix="dupval-")
    gcc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    niji = ["-nostdinc", "-isystem", os.path.abspath(a.include), "-isystem", gcc, "-idirafter", kernel_dir(work)]
    by_hdr = listed()
    same = diff = fail = 0
    for hdr in sorted(by_hdr):
        ref = values(work, hdr, by_hdr[hdr], [])
        got = values(work, hdr, ref, niji) if ref else {}
        for n in sorted(ref):
            if n not in got:
                fail += 1
                print(f"dupval: NIJI-ONLY-FAIL {n} {hdr} reference={ref[n]}")
            elif got[n] != ref[n]:
                diff += 1
                print(f"dupval: DIFF {n} {hdr} reference={ref[n]} libniji={got[n]}")
            else:
                same += 1
    print(f"dupval: RESULT headers={len(by_hdr)} same={same} diff={diff} niji-only-fail={fail}")
    return 1 if diff or fail else 0


if __name__ == "__main__":
    sys.exit(main())
