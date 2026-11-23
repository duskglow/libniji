#!/usr/bin/env python3
"""PERF-GAP-STRUCTURE-AUDIT probe 3. (0) removes probe 2's start-up profiles (their sh loop ran on the reference);
(A) start-up instructions against environment size (both libraries); (B) page faults by the faulting address's
mapping (reference: object and permissions only, no symbols; libniji: also the symbol at the address);
(C) resident pages per mapping at the end of main (both); (D) exact gdb single-step split of libniji's dynamic
start-up (libniji only). Uses probe 1's binaries in out1/."""
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


def cc(v, out, defs):
    src = os.path.join(HERE, "startup.c")
    env = None
    if v == "ref-dyn":
        cmd = ["gcc", "-O2", *defs, src, "-o", out]
    elif v == "ref-static":
        cmd = ["gcc", "-O2", "-static", *defs, src, "-o", out]
    elif v == "niji-dyn":
        cmd = ["sh", ROLE + "/validation/wl_dyncc.sh", "-O2", *defs, src, "-o", out]
        env = dict(os.environ, WL_DYN_PRODUCTS=P + "-release-dyn")
    else:
        cmd = ["sh", ROLE + "/scripts/niji-cc", "-O2", *defs, src, "-o", out]
        env = dict(os.environ, NIJI_PRODUCTS=P + "-release")
    r = sh(cmd, env=env)
    if r.returncode:
        print("BUILD-FAIL", v, r.stderr[-600:])


def counts(argv, env):
    r = sh(["perf", "stat", "-x,", "-e", "instructions:u,instructions:k,page-faults", "--"] + argv, env=env)
    d = {}
    for ln in r.stderr.splitlines():
        f = ln.split(",")
        if len(f) > 3:
            try:
                d[f[2]] = float(f[0])
            except ValueError:
                pass
    return d


MM = re.compile(r"PERF_RECORD_MMAP2?\s+\d+/\d+:\s+\[0x([0-9a-f]+)\(0x([0-9a-f]+)\)\s+@\s+\S+[^\]]*\]:\s+([rwxps-]{4})\s+(.*)$")
SAMPLE = re.compile(r"^\s*([0-9a-f]+)\s+(.*?)\s*\(([^()]*)\)")


def faults(prog, args, with_sym):
    data = "pf3." + prog
    sh(["perf", "record", "-q", "-e", "page-faults", "-c", "1", "-d", "-o", data, "--", "./" + prog] + list(args))
    out = sh(["perf", "script", "-i", data, "--show-mmap-events", "-F", "addr"]).stdout
    maps, lab_c, lab_s, total, first_mm = [], collections.Counter(), collections.defaultdict(collections.Counter), 0, None
    for ln in out.splitlines():
        if "PERF_RECORD_MMAP" in ln:
            m = MM.search(ln)
            if m:
                nm = m.group(4).strip().split()[-1]
                maps.append((int(m.group(1), 16), int(m.group(1), 16) + int(m.group(2), 16), m.group(3),
                             os.path.basename(nm) if nm.startswith("/") else nm))
            elif first_mm is None:
                first_mm = ln
            continue
        s = SAMPLE.match(ln)
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
            lab = ("stack" if a >= 0x7FF000000000 else "heap/anon (no mmap event)") + f" [{os.path.basename(s.group(3))}]"
        lab_c[lab] += 1
        if with_sym:
            lab_s[lab][(s.group(2) or "?").split("+")[0]] += 1
    print(f"\n-- faults {prog} {' '.join(args)}: {total} (mmap events parsed {len(maps)})")
    if not maps and first_mm:
        print("   unparsed mmap line:", first_mm[:160])
    for lab, c in lab_c.most_common():
        extra = ("   <- " + ", ".join(f"{k[:44]} {v}" for k, v in lab_s[lab].most_common(5))) if with_sym else ""
        print(f"   {c:4}  {lab[:64]}{extra}")


def smaps(prog):
    out = sh(["./" + prog]).stdout
    agg = collections.OrderedDict()
    cur = None
    for ln in out.splitlines():
        m = re.match(r"^[0-9a-f]+-[0-9a-f]+ ([rwxps-]{4}) \S+ \S+ \d+\s*(.*)$", ln)
        if m:
            nm = m.group(2).strip()
            nm = os.path.basename(nm) if nm.startswith("/") else (nm or "[anon]")
            cur = (nm, m.group(1))
            agg.setdefault(cur, [0, 0, 0])
            continue
        m = re.match(r"^(Size|Rss|Anonymous):\s+(\d+) kB", ln)
        if m and cur:
            agg[cur][("Size", "Rss", "Anonymous").index(m.group(1))] += int(m.group(2))
    tot = [sum(v[i] for v in agg.values()) for i in range(3)]
    print(f"\n-- resident at end of main, {prog}: Rss {tot[1]} KiB (anonymous {tot[2]} KiB), mapped {tot[0]} KiB")
    for (nm, perm), (size, rss, anon) in agg.items():
        if rss:
            print(f"   {nm[:44]:44} {perm}  size {size:7} KiB  rss {rss:5} KiB  anon {anon:5} KiB")


def main():
    os.chdir(OUT)
    gone = [f for f in os.listdir(".") if f.startswith("prof.") and not f.startswith("prof.pc.")]
    for f in gone:
        os.remove(f)
    print("(0) removed", len(gone), "start-up profile files of probe 2 (sh loop)")

    print("\n(A) start-up user instructions against environment size (median of 7; env = PATH, HOME + N vars of ~100 B)")
    envb = sum(len(k) + len(v) + 2 for k, v in os.environ.items())
    print(f"    this job's own environment: {len(os.environ)} variables, {envb} bytes")
    for nv in (0, 20, 80):
        env = {"PATH": "/usr/bin:/bin", "HOME": "/tmp"}
        env.update({f"PGSA_VAR_{i:03d}": "x" * 86 for i in range(nv)})
        row = []
        for v in V4:
            runs = [counts(["./empty." + v], env) for _ in range(7)]
            row.append(f"{v} iu {statistics.median(r.get('instructions:u', 0) for r in runs):9.0f}"
                       f" ik {statistics.median(r.get('instructions:k', 0) for r in runs):8.0f}"
                       f" pf {statistics.median(r.get('page-faults', 0) for r in runs):3.0f}")
        print(f"    N={nv:3}  " + " | ".join(row))

    print("\n(B) page faults by mapping")
    for prog in ("empty", "hello"):
        for v in V4:
            faults(f"{prog}.{v}", (), v.startswith("niji"))
    for v in ("niji-dyn", "ref-dyn", "niji-static", "ref-static"):
        faults(f"pc.{v}", ("pthread", "0"), v.startswith("niji"))
        faults(f"pc.{v}", ("pthread", "1"), v.startswith("niji"))

    print("\n(C) resident pages per mapping at the end of main (empty program + the snapshot)")
    for v in ("ref-dyn", "niji-dyn"):
        cc(v, "smaps." + v, ["-DP_SMAPS"])
        smaps("smaps." + v)

    print("\n(D) libniji dynamic start-up, exact (gdb single-step), env = PATH, HOME")
    r = sh(["env", "-i", "PATH=/usr/bin:/bin", "HOME=/tmp", "gdb", "-q", "-nx", "-batch", "-x",
            os.path.join(HERE, "trace_start.py"), "--args", "./empty.niji-dyn"], timeout=3000)
    print("\n".join(ln for ln in r.stdout.splitlines() if ln.startswith(("TRACE", "  "))))
    if r.returncode:
        print("gdb rc", r.returncode, r.stderr[-500:])
    print("\n== done")


main()
