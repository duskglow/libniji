#!/usr/bin/env python3
"""PERF-GAP-STRUCTURE-AUDIT probe 4. (1) kernel unit costs (kunit, a libniji static program; the kernel's cost does
not depend on the C library): instructions:k and cycles:k per zero-fill fault, per fault-around read fault, per
copy-on-write fault, per mmap+munmap. (2) page faults by the faulting address's mapping (reference: mapping only;
libniji: also the symbol at the address). (3) the largest stack frames in libniji's ld.so (its own disassembly).
(4) relocation entries of the probe programs themselves (gcc output, both kits)."""
import collections
import os
import re
import statistics
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
SHA = "0495c4684330b90a6cecc02271ba1429296536ce"
P = os.path.expanduser("~/wl/products/" + SHA)
ROLE = os.path.expanduser("~/w/s2")
OUT = os.path.join(HERE, "out1")
V4 = ("ref-dyn", "ref-static", "niji-dyn", "niji-static")


def sh(cmd, env=None, timeout=1200):
    return subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=timeout)


def counts(argv):
    r = sh(["perf", "stat", "-x,", "-e", "instructions:k,cycles:k,page-faults", "--"] + argv)
    d = {}
    for ln in r.stderr.splitlines():
        f = ln.split(",")
        if len(f) > 3:
            try:
                d[f[2]] = float(f[0])
            except ValueError:
                pass
    return d, r.returncode


MM = re.compile(r"PERF_RECORD_MMAP2?\s+\d+/\d+:\s+\[0x([0-9a-f]+)\(0x([0-9a-f]+)\)\s+@\s+\S+[^\]]*\]:\s+([rwxps-]{4})\s+(.*)$")
WITH_SYM = re.compile(r"^\s*([0-9a-f]+)\s+(.*?)\s*\(([^()]*)\)")
BARE = re.compile(r"^\s*([0-9a-f]+)\s*$")


def faults(prog, args, with_sym):
    data = "pf4." + prog
    sh(["perf", "record", "-q", "-e", "page-faults", "-c", "1", "-d", "-o", data, "--", "./" + prog] + list(args))
    fields = "ip,sym,dso,addr" if with_sym else "addr"
    out = sh(["perf", "script", "-i", data, "--show-mmap-events", "-F", fields]).stdout
    maps, lab_c, lab_s, total = [], collections.Counter(), collections.defaultdict(collections.Counter), 0
    for ln in out.splitlines():
        if "PERF_RECORD_MMAP" in ln:
            m = MM.search(ln)
            if m:
                nm = m.group(4).strip().split()[-1]
                maps.append((int(m.group(1), 16), int(m.group(1), 16) + int(m.group(2), 16), m.group(3),
                             os.path.basename(nm) if nm.startswith("/") else nm))
            continue
        s = (WITH_SYM if with_sym else BARE).match(ln)
        if not s:
            continue
        total += 1
        a = int(s.group(1), 16)
        lab = None
        for lo, hi, perm, nm in reversed(maps):
            if lo <= a < hi:
                lab = f"{nm} {perm}"
                break
        if lab is None:
            lab = "stack" if a >= 0x7FF000000000 else "heap / anonymous (no mmap event)"
        lab_c[lab] += 1
        if with_sym:
            lab_s[lab][(s.group(2) or "?").split("+")[0]] += 1
    print(f"\n-- faults {prog} {' '.join(args)}: {total}")
    for lab, c in lab_c.most_common():
        extra = ("   <- " + ", ".join(f"{k[:40]} {v}" for k, v in lab_s[lab].most_common(4))) if with_sym else ""
        print(f"   {c:4}  {lab[:62]}{extra}")


def main():
    os.chdir(OUT)
    r = sh(["sh", ROLE + "/scripts/niji-cc", "-O2", os.path.join(HERE, "kunit.c"), "-o", "kunit"],
           env=dict(os.environ, NIJI_PRODUCTS=P + "-release"))
    if r.returncode:
        print("BUILD-FAIL kunit", r.stderr[-500:])
        return
    big = P + "-release-dyn/libc.so.6"
    with open(big, "rb") as f:  # page cache warm: every fault below is a minor fault
        while f.read(1 << 20):
            pass
    print("(1) kernel unit costs (median of 5 x (n - 0)/n; file = an 18 MB library file, page cache warm)")
    for mode, n in (("anon", 2000), ("fileread64k", 200), ("cow", 1000), ("mmapfile", 2000)):
        per = []
        for _ in range(5):
            z, rc0 = counts(["./kunit", mode, "0", big])
            x, rc1 = counts(["./kunit", mode, str(n), big])
            if rc0 or rc1:
                print("   RUN-FAIL", mode, rc0, rc1)
                break
            per.append({k: (x.get(k, 0) - z.get(k, 0)) / n for k in ("instructions:k", "cycles:k", "page-faults")})
        if per:
            print(f"   {mode:12} per unit: instructions:k {statistics.median(p['instructions:k'] for p in per):8.0f}"
                  f"  cycles:k {statistics.median(p['cycles:k'] for p in per):8.0f}"
                  f"  faults {statistics.median(p['page-faults'] for p in per):5.2f}")

    print("\n(2) page faults by mapping (mapping permissions as first mapped)")
    for prog in ("empty", "hello"):
        for v in V4:
            faults(f"{prog}.{v}", (), v.startswith("niji"))
    for v in V4:
        for k in ("0", "1"):
            faults(f"pc.{v}", ("pthread", k), v.startswith("niji"))

    print("\n(3) largest stack frames in libniji's ld.so (sub $N,%rsp in its own disassembly)")
    dis = sh(["objdump", "-d", "--no-show-raw-insn", P + "-release-dyn/ld-linux-x86-64.so.2"]).stdout
    fn, frames = "?", collections.Counter()
    for ln in dis.splitlines():
        m = re.match(r"^[0-9a-f]+ <(.+)>:$", ln)
        if m:
            fn = m.group(1)
            continue
        m = re.search(r"\bsub\s+\$0x([0-9a-f]+),%rsp", ln)
        if m:
            frames[fn] = max(frames[fn], int(m.group(1), 16))
    for f, b in frames.most_common(15):
        print(f"   {b:7} B  {f[:150]}")

    print("\n(4) relocation entries of the probe program itself (empty), by type")
    for v in ("ref-dyn", "niji-dyn"):
        t = collections.Counter(re.findall(r"\b(R_X86_64_[A-Z0-9_]+)", sh(["readelf", "-rW", f"empty.{v}"]).stdout))
        print(f"   empty.{v}: " + ", ".join(f"{k} {c}" for k, c in t.most_common()))
    print("\n== done")


main()
