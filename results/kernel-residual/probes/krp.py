#!/usr/bin/env python3
"""KERNEL-RESIDUAL-PROFILE probe: where the kernel work of a dynamic start-up goes, libniji's loader against the
reference's, on the empty program, `true`, `ls /` and `sed`.

Black box for the reference: counters, kernel-side samples (kernel call chains only, resolved against a copy of the
running kernel's kallsyms), syscalls as strace decodes them, and the process's own /proc files. Nothing of the
reference's code is symbolised or shown.

  python3 krp.py [--products DIR] [--kallsyms FILE] [--runs N] [--only stat,strace,vmas,record] OUTDIR

Sections:
  stat    perf stat medians (user/kernel instructions and cycles, faults), 21 runs
  strace  syscalls by name, and every mmap/mprotect/munmap/brk with sizes (strace -f -y)
  vmas    VMAs and memory status at exit_group (strace delays exit_group; /proc/<pid>/smaps is read meanwhile),
          and at main for the empty program (maps.c, built with each kit)
  record  kernel samples of instructions:k and cycles:k with kernel call chains, N runs per event and program;
          a fixed period per run (no frequency mode: its first samples are dense and over-weight early code), the
          period varied over runs (a deterministic start-up sampled with one period hits the same instructions every
          run). Each sample weighs its period; per-run estimates are the summed weight over N.
"""
import bisect
import collections
import json
import os
import re
import statistics
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
SHA = "0495c4684330b90a6cecc02271ba1429296536ce"
ARGS = sys.argv[1:]


def opt(name, default):
    if name in ARGS:
        i = ARGS.index(name)
        v = ARGS[i + 1]
        del ARGS[i:i + 2]
        return v
    return default


NLIB = os.path.expanduser(opt("--products", "~/wl/products/" + SHA + "-release-dyn"))
KALLSYMS = os.path.expanduser(opt("--kallsyms", "~/work/kallsyms.txt"))
RUNS = int(opt("--runs", "300"))
ONLY = opt("--only", "stat,strace,vmas,record").split(",")
ROLE = os.path.expanduser(opt("--role", "~/w/s2"))
OUT = os.path.abspath(ARGS[0] if ARGS else "out")
NLD = NLIB + "/ld-linux-x86-64.so.2"
RLD = "/lib64/ld-linux-x86-64.so.2"
SYSLIB = "/lib/x86_64-linux-gnu"
ENV = {"PATH": "/usr/bin:/bin", "HOME": "/tmp", "LANG": "C.UTF-8"}
EV = "instructions:u,instructions:k,cycles:u,cycles:k,page-faults"
# Periods: primes 40,009 .. 69,997 cycled over runs (about 10 to 25 samples per run: below the 26 samples per
# tick at which this host's kernel throttles an event, so no run loses samples).
PERIODS = [p for p in range(40009, 70000, 2) if all(p % d for d in range(3, int(p ** 0.5) + 1, 2))][::97]


def sh(cmd, env=None, timeout=900, **kw):
    return subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=timeout, **kw)


def build():
    os.makedirs(OUT, exist_ok=True)
    os.chdir(OUT)
    with open("empty.c", "w") as f:
        f.write("int main(void) { return 0; }\n")
    with open("words.txt", "w") as f:
        f.write("\n".join(["pear", "apple", "Zebra", "banana", "cherry", "apple"] * 20) + "\n")
    for src in ("empty.c", os.path.join(HERE, "maps.c")):
        base = os.path.basename(src)[:-2]
        r1 = sh(["gcc", "-O2", src, "-o", base + ".ref"])
        r2 = sh(["sh", ROLE + "/validation/wl_dyncc.sh", "-O2", src, "-o", base + ".niji"],
                env=dict(os.environ, WL_DYN_PRODUCTS=NLIB))
        for r in (r1, r2):
            if r.returncode:
                sys.exit("BUILD-FAIL " + src + " " + r.stderr[-600:])


def configs():
    progs = [("true", ["/bin/true"]), ("ls", ["/bin/ls", "/"]), ("sed", ["/bin/sed", "s/a/b/", "words.txt"])]
    out = [("empty", "ref", ["./empty.ref"]), ("empty", "niji", ["./empty.niji"])]
    for name, argv in progs:
        out.append((name, "ref", [RLD, "--library-path", SYSLIB] + argv))
        out.append((name, "niji", [NLD, "--library-path", NLIB + ":" + SYSLIB] + argv))
    return out


def parse_stat(text):
    d = {}
    for ln in text.splitlines():
        f = ln.split(",")
        if len(f) > 3:
            try:
                d[f[2]] = float(f[0])
            except ValueError:
                pass
    return d


def do_stat():
    print("\n== stat: medians of 21 (env PATH HOME LANG=C.UTF-8)")
    res = {}
    for name, kit, argv in configs():
        runs = [parse_stat(sh(["perf", "stat", "-x,", "-e", EV, "--"] + argv, env=ENV).stderr) for _ in range(21)]
        m = {k: statistics.median(r.get(k, float("nan")) for r in runs) for k in EV.split(",")}
        res[f"{name}.{kit}"] = m
        print(f"   {name:6} {kit:4}  iu {m['instructions:u']:9.0f}  ik {m['instructions:k']:9.0f}  cu {m['cycles:u']:9.0f}"
              f"  ck {m['cycles:k']:9.0f}  pf {m['page-faults']:4.0f}")
    for name in ("empty", "true", "ls", "sed"):
        a, b = res[name + ".ref"], res[name + ".niji"]
        print(f"   {name:6} niji-ref  ik {b['instructions:k'] - a['instructions:k']:+9.0f}  ck {b['cycles:k'] - a['cycles:k']:+9.0f}"
              f"  cu+ck {(b['cycles:u'] + b['cycles:k']) / (a['cycles:u'] + a['cycles:k']):.3f}  pf {b['page-faults'] - a['page-faults']:+4.0f}")
    json.dump(res, open("stat.json", "w"), indent=1)


SYSLINE = re.compile(r"^(\d+)\s+(\w+)\((.*)\)\s+=\s+(-?\w+)")


def do_strace():
    print("\n== strace -f -y: syscalls by name (niji - ref), then the mapping calls")
    allc = {}
    for name, kit, argv in configs():
        log = f"strace.{name}.{kit}"
        sh(["strace", "-f", "-y", "-qq", "-s", "0", "-o", log, "--"] + argv, env=ENV)
        c = collections.Counter()
        maps = []
        for ln in open(log):
            m = SYSLINE.match(ln)
            if not m:
                continue
            c[m.group(2)] += 1
            if m.group(2) in ("mmap", "munmap", "mprotect", "brk", "mremap", "madvise"):
                maps.append(f"{m.group(2)}({m.group(3)}) = {m.group(4)}")
        allc[(name, kit)] = c
        with open(log + ".maps", "w") as f:
            f.write("\n".join(maps) + "\n")
    for name in ("empty", "true", "ls", "sed"):
        a, b = allc[(name, "ref")], allc[(name, "niji")]
        keys = sorted(set(a) | set(b), key=lambda k: (-(b[k] - a[k]), k))
        print(f"   {name:6} total {sum(a.values())} -> {sum(b.values())}: " +
              ", ".join(f"{k} {a[k]}->{b[k]}" for k in keys if a[k] != b[k]))


def smaps(pid):
    vmas, cur = [], None
    for ln in open(f"/proc/{pid}/smaps"):
        f = ln.split()
        if re.match(r"^[0-9a-f]+-[0-9a-f]+ ", ln):
            lo, hi = (int(x, 16) for x in f[0].split("-"))
            cur = {"lo": lo, "hi": hi, "perm": f[1], "name": f[5] if len(f) > 5 else "", "kb": {}}
            vmas.append(cur)
        elif cur is not None and len(f) == 3 and f[2] == "kB":
            cur["kb"][f[0].rstrip(":")] = int(f[1])
    return vmas


def status(pid_or_text):
    text = pid_or_text if "\n" in pid_or_text else open(f"/proc/{pid_or_text}/status").read()
    keep = ("VmSize", "VmRSS", "RssAnon", "RssFile", "VmPTE", "VmData", "VmStk")
    return {ln.split(":")[0]: ln.split(":")[1].strip() for ln in text.splitlines() if ln.split(":")[0] in keep}


def vma_summary(vmas):
    tot = collections.Counter()
    for v in vmas:
        tot["vmas"] += 1
        tot["size_kb"] += (v["hi"] - v["lo"]) >> 10
        for k in ("Rss", "Private_Dirty", "Anonymous"):
            tot[k] += v["kb"].get(k, 0)
        if v["perm"].startswith("---"):
            tot["none_vmas"] += 1
            tot["none_kb"] += (v["hi"] - v["lo"]) >> 10
    return tot


def show_vmas(tag, vmas, st):
    t = vma_summary(vmas)
    print(f"   {tag}: {t['vmas']} VMAs ({t['none_vmas']} PROT_NONE, {t['none_kb']} KiB), size {t['size_kb']} KiB,"
          f" Rss {t['Rss']}, Private_Dirty {t['Private_Dirty']}, Anonymous {t['Anonymous']} KiB; " +
          " ".join(f"{k} {v}" for k, v in st.items()))
    with open("vmas." + tag.replace(" ", "."), "w") as f:
        for v in vmas:
            f.write(f"{v['lo']:x}-{v['hi']:x} {v['perm']} {(v['hi'] - v['lo']) >> 10:7}K rss {v['kb'].get('Rss', 0):4}"
                    f" dirty {v['kb'].get('Private_Dirty', 0):4} anon {v['kb'].get('Anonymous', 0):4} {v['name']}\n")


def do_vmas():
    print("\n== VMAs at main (maps.c, each kit)")
    for kit in ("ref", "niji"):
        out = sh(["./maps." + kit], env=ENV).stdout
        parts = out.split("== /proc/self/status\n")
        with open("smaps.main." + kit, "w") as f:
            f.write(parts[0])
        vmas, cur = [], None
        for ln in parts[0].splitlines():
            f = ln.split()
            if re.match(r"^[0-9a-f]+-[0-9a-f]+ ", ln):
                lo, hi = (int(x, 16) for x in f[0].split("-"))
                cur = {"lo": lo, "hi": hi, "perm": f[1], "name": f[5] if len(f) > 5 else "", "kb": {}}
                vmas.append(cur)
            elif cur is not None and len(f) == 3 and f[2] == "kB":
                cur["kb"][f[0].rstrip(":")] = int(f[1])
        show_vmas(f"main {kit}", vmas, status(parts[1] if len(parts) > 1 else "\n"))
    print("\n== VMAs at exit_group (strace delays exit_group by 3 s; smaps read after 1.5 s)")
    for name, kit, argv in configs():
        p = subprocess.Popen(["strace", "-qq", "-o", "/dev/null", "-e", "trace=exit_group",
                              "-e", "inject=exit_group:delay_enter=3s", "--"] + argv, env=ENV,
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(1.5)
        kids = open(f"/proc/{p.pid}/task/{p.pid}/children").read().split()
        if len(kids) != 1:
            print(f"   {name} {kit}: children {kids}, skipped")
            p.wait()
            continue
        pid = kids[0]
        try:
            nr = open(f"/proc/{pid}/syscall").read().split()[0]
        except OSError as e:
            nr = "unreadable " + str(e.errno)
        show_vmas(f"exit {name} {kit} (syscall {nr})", smaps(pid), status(pid))
        p.wait()


def load_kallsyms():
    addrs, names = [], []
    for ln in open(KALLSYMS):
        f = ln.split()
        if len(f) >= 3 and f[1] in "tTwW":
            a = int(f[0], 16)
            if a:
                addrs.append(a)
                names.append(f[2])
    order = sorted(range(len(addrs)), key=addrs.__getitem__)
    return [addrs[i] for i in order], [names[i] for i in order]


KA, KN = None, None


def ksym(a):
    i = bisect.bisect_right(KA, a) - 1
    return KN[i] if i >= 0 else "?"


FAULT_KIND = [("do_anonymous_page", "zero-fill"), ("do_wp_page", "copy-on-write"), ("wp_page_copy", "copy-on-write"),
              ("do_cow_fault", "private-file-write"), ("do_shared_fault", "shared-file-write"),
              ("do_read_fault", "file-read"), ("filemap_map_pages", "file-read"), ("do_swap_page", "swap")]


def category(chain):
    """chain: kernel function names, outermost first."""
    if not chain:
        return "none"
    root = chain[0]
    if root.startswith(("asm_sysvec", "asm_common_interrupt", "irq_", "sysvec")) or "interrupt" in root:
        return "irq"
    if any(f.startswith(("perf_event_mmap", "perf_iterate", "perf_event_exec", "perf_event_comm",
                         "perf_event_task", "perf_event_namespaces")) for f in chain):
        return "perf-overhead"
    sysname = next((f[len("__x64_sys_"):] for f in chain if f.startswith("__x64_sys_")), None)
    fault = "asm_exc_page_fault" in chain
    kind = next((k for fn, k in FAULT_KIND if fn in chain), "fault-common")
    if sysname:
        if sysname == "exit_group":
            sub = "exit_mmap" if "exit_mmap" in chain else "other"
            return f"exit_group/{sub}"
        return f"sys:{sysname}" + (f"/fault:{kind}" if fault else "")
    if fault:
        return f"fault:{kind}"
    if root.startswith("entry_SYSCALL") or root.startswith("do_syscall"):
        return "sys:entry-exit"
    return "other:" + root


def record_one(argv, ev, period, data):
    r = sh(["perf", "record", "-q", "-e", ev, "-c", str(period), "-g", "-o", data, "--"] + argv, env=ENV)
    out = sh(["perf", "script", "-i", data, "-F", "tid,period,ip"]).stdout
    # A sample is a header line "<tid> <period>" (no leading tab) followed by its call chain, one frame per
    # line, each starting with a tab; kernel frames only are kept (the user part of the chain is not used).
    samples, cur = [], None
    for ln in out.splitlines():
        if not ln.strip():
            continue
        f = ln.split()
        if ln.startswith("\t"):
            if cur is not None:
                a = int(f[0], 16)
                if a >= 0xffff800000000000:
                    cur[1].append(ksym(a))
            continue
        if len(f) >= 2 and f[-1].isdigit():
            cur = (int(f[-1]), [])
            samples.append(cur)
    return r.returncode, samples


def do_record():
    global KA, KN
    KA, KN = load_kallsyms()
    print(f"\n== record: {RUNS} runs per event and program, kernel call chains, periods {PERIODS[0]}..{PERIODS[-1]}"
          f" ({len(PERIODS)} values)")
    stat = json.load(open("stat.json")) if os.path.exists("stat.json") else {}
    agg = {}
    for ev in ("instructions:k", "cycles:k"):
        for name, kit, argv in configs():
            cat, incl, leaf, chains = (collections.Counter() for _ in range(4))
            nsamp, rcs = 0, collections.Counter()
            for i in range(RUNS):
                per = PERIODS[i % len(PERIODS)]
                rc, samples = record_one(argv, ev, per, "rec.data")
                rcs[rc] += 1
                for w, frames in samples:
                    ch = list(reversed(frames))
                    nsamp += 1
                    cat[category(ch)] += w
                    for fn in set(ch):
                        incl[fn] += w
                    if ch:
                        leaf[ch[-1]] += w
                    chains[";".join(ch)] += w
            key = f"{ev}|{name}|{kit}"
            agg[key] = {"cat": cat, "incl": incl, "leaf": leaf, "chains": chains, "samples": nsamp, "rc": dict(rcs)}
            tot = sum(cat.values()) / RUNS
            ref = stat.get(f"{name}.{kit}", {}).get(ev, float("nan"))
            print(f"   {ev:14} {name:6} {kit:4} samples {nsamp:6}  sampled {tot:9.0f}/run  perf stat {ref:9.0f}"
                  f"  coverage {tot / ref if ref == ref else float('nan'):.3f}  rc {dict(rcs)}")
    json.dump(agg, open("record.json", "w"))
    report(agg)


def report(agg):
    for ev in ("instructions:k", "cycles:k"):
        for name in ("empty", "true", "ls", "sed"):
            a, b = agg[f"{ev}|{name}|ref"], agg[f"{ev}|{name}|niji"]
            ta, tb = sum(a["cat"].values()) / RUNS, sum(b["cat"].values()) / RUNS
            print(f"\n-- {ev} {name}: per run ref {ta:,.0f} niji {tb:,.0f} diff {tb - ta:+,.0f}; categories by |diff|")
            keys = sorted(set(a["cat"]) | set(b["cat"]), key=lambda k: -abs(b["cat"][k] - a["cat"][k]))
            for k in keys[:16]:
                print(f"   {k[:44]:44} {a['cat'][k] / RUNS:9,.0f} {b['cat'][k] / RUNS:9,.0f} {(b['cat'][k] - a['cat'][k]) / RUNS:+9,.0f}")
            keys = sorted(set(a["incl"]) | set(b["incl"]), key=lambda k: -abs(b["incl"][k] - a["incl"][k]))
            print("   inclusive functions by |diff|:")
            for k in keys[:22]:
                print(f"   {k[:44]:44} {a['incl'][k] / RUNS:9,.0f} {b['incl'][k] / RUNS:9,.0f} {(b['incl'][k] - a['incl'][k]) / RUNS:+9,.0f}")


HOLDER = """
import mmap, os, sys, time
keep = []
for p in sys.argv[1:]:
    fd = os.open(p, os.O_RDONLY)
    m = mmap.mmap(fd, 0, prot=mmap.PROT_READ)
    s = 0
    for off in range(0, len(m), 4096):
        s += m[off]
    keep.append(m)
print("ready", flush=True)
sys.stdin.read()
"""


def start_holder(paths):
    """A process that keeps every page of `paths` mapped (each page read once through a shared view), as every
    other process on a system using that library would: a page already mapped elsewhere skips the kernel's
    first-map accounting when another process maps it, and its last-unmap accounting at exit."""
    p = subprocess.Popen([sys.executable, "-c", HOLDER] + paths, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    if p.stdout.readline().strip() != "ready":
        sys.exit("holder failed")
    return p


def shared_configs(copydir):
    progs = [("empty", ["./empty.ref"], ["./empty.niji"]), ("true", ["/bin/true"], ["/bin/true"]),
             ("ls", ["/bin/ls", "/"], ["/bin/ls", "/"]), ("sed", ["/bin/sed", "s/a/b/", "words.txt"],
                                                         ["/bin/sed", "s/a/b/", "words.txt"])]
    out = []
    for name, ra, na in progs:
        out.append((name, "ref", [RLD, "--library-path", SYSLIB] + ra))
        out.append((name, "refcopy", [copydir + "/ld-linux-x86-64.so.2", "--library-path", copydir + ":" + SYSLIB] + ra))
        out.append((name, "niji", [NLD, "--library-path", NLIB + ":" + SYSLIB] + na))
        out.append((name, "nijiheld", [NLD, "--library-path", NLIB + ":" + SYSLIB] + na))
    return out


def do_shared():
    """The same programs, all through an explicitly invoked loader: the reference, the reference with fresh
    copies of its loader and libc (mapped by no other process), libniji, and libniji while a holder keeps its
    loader's and libc's pages mapped."""
    global KA, KN
    KA, KN = load_kallsyms()
    copydir = os.path.join(OUT, "refcopy")
    os.makedirs(copydir, exist_ok=True)
    for f in ("libc.so.6", "ld-linux-x86-64.so.2"):
        src = os.path.realpath(os.path.join(SYSLIB, f))
        dst = os.path.join(copydir, f)
        if os.path.exists(dst):
            os.unlink(dst)
        with open(src, "rb") as a, open(dst, "wb") as b:
            b.write(a.read())
        os.chmod(dst, 0o755)
    cfgs = shared_configs(copydir)
    res, agg = {}, {}
    print("\n== shared: perf stat medians of 21, then kernel samples (runs as --runs; cycles at twice the period)")
    for phase in ("plain", "held"):
        holder = start_holder([NLIB + "/libc.so.6", os.path.realpath(NLD)]) if phase == "held" else None
        try:
            for name, kit, argv in cfgs:
                if (kit == "nijiheld") != (phase == "held"):
                    continue
                runs = [parse_stat(sh(["perf", "stat", "-x,", "-e", EV, "--"] + argv, env=ENV).stderr) for _ in range(21)]
                res[f"{name}.{kit}"] = {k: statistics.median(r.get(k, float("nan")) for r in runs) for k in EV.split(",")}
                for ev, mult in (("instructions:k", 1), ("cycles:k", 2)):
                    chains = collections.Counter()
                    for i in range(RUNS):
                        _, samples = record_one(argv, ev, PERIODS[i % len(PERIODS)] * mult, "rec.data")
                        for w, frames in samples:
                            chains[";".join(reversed(frames))] += w
                    agg[f"{ev}|{name}|{kit}"] = {"chains": chains}
        finally:
            if holder:
                holder.stdin.close()
                holder.wait()
    json.dump(res, open("stat_shared.json", "w"), indent=1)
    json.dump(agg, open("record_shared.json", "w"))
    for name in ("empty", "true", "ls", "sed"):
        r = {k: res[f"{name}.{k}"] for k in ("ref", "refcopy", "niji", "nijiheld")}
        print(f"   {name:6} " + "  ".join(f"{k} ik {v['instructions:k']:9.0f} ck {v['cycles:k']:9.0f} pf {v['page-faults']:4.0f}"
                                         for k, v in r.items()))
        print(f"   {name:6} niji-ref ik {r['niji']['instructions:k'] - r['ref']['instructions:k']:+9.0f}"
              f"  nijiheld-ref ik {r['nijiheld']['instructions:k'] - r['ref']['instructions:k']:+9.0f}"
              f" ck {r['nijiheld']['cycles:k'] - r['ref']['cycles:k']:+9.0f}"
              f"  refcopy-ref ik {r['refcopy']['instructions:k'] - r['ref']['instructions:k']:+9.0f}"
              f"  niji-refcopy ik {r['niji']['instructions:k'] - r['refcopy']['instructions:k']:+9.0f}")


def main():
    build()
    if "shared" in ONLY:
        do_shared()
    if "stat" in ONLY:
        do_stat()
    if "strace" in ONLY:
        do_strace()
    if "vmas" in ONLY:
        do_vmas()
    if "record" in ONLY:
        do_record()
    print("\n== done")


main()
