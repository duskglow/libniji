#!/usr/bin/env python3
"""PERF-GAP-STRUCTURE-AUDIT probe 2 (attribution). Uses the binaries probe 1 built in out1/.
libniji: perf symbol profiles (start-up and per call), fault addresses with the faulting function, relocation
census of its ld.so and libc.so.6. The reference: black box only (fault addresses by mapping and the object whose
code faulted, syscalls, LD_DEBUG=statistics, program headers); no symbol of the reference is sampled or shown."""
import bisect
import collections
import os
import re
import struct
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
SHA = "0495c4684330b90a6cecc02271ba1429296536ce"
P = os.path.expanduser("~/wl/products/" + SHA)
OUT = os.path.join(HERE, "out1")


def sh(cmd, env=None, shell=False, timeout=900):
    return subprocess.run(cmd, env=env, shell=shell, capture_output=True, text=True, timeout=timeout)


def head(text, n):
    return "\n".join(text.splitlines()[:n])


def report(data, n, comm=None, sort="dso,sym"):
    cmd = ["perf", "report", "-i", data, "--stdio", "--sort", sort, "--percent-limit", "0.3"]
    if comm:
        cmd += ["--comm", comm]
    out = sh(cmd).stdout
    lines = [ln for ln in out.splitlines() if ln.strip() and not ln.startswith("#")]
    return "\n".join(lines[:n])


# ---------------------------------------------------------------- relocation census (libniji objects only)
def elf(path):
    d = open(path, "rb").read()
    shoff = struct.unpack_from("<Q", d, 0x28)[0]
    shentsize, shnum, shstrndx = struct.unpack_from("<HHH", d, 0x3A)
    secs = []
    for i in range(shnum):
        name, typ, flags, addr, off, size, link, info, align, ent = struct.unpack_from("<IIQQQQIIQQ", d, shoff + i * shentsize)
        secs.append(dict(name=name, type=typ, flags=flags, addr=addr, off=off, size=size, link=link, ent=ent))
    so = secs[shstrndx]["off"]
    for s in secs:
        o = so + s["name"]
        s["name"] = d[o:d.index(b"\0", o)].decode()
    return d, secs


def census(path):
    d, secs = elf(path)
    alloc = sorted([s for s in secs if s["flags"] & 2 and s["size"]], key=lambda s: s["addr"])
    starts = [s["addr"] for s in alloc]

    def sec_of(a):
        i = bisect.bisect_right(starts, a) - 1
        if i >= 0 and a < alloc[i]["addr"] + alloc[i]["size"]:
            return alloc[i]
        return None

    def word(a):
        s = sec_of(a)
        if not s or s["type"] == 8:
            return 0
        return struct.unpack_from("<Q", d, s["off"] + a - s["addr"])[0]

    rs = re.compile(rb"^[A-Za-z0-9_./\\-]{3,200}?\.rs")

    def tclass(t):
        s = sec_of(t)
        if not s:
            return "outside"
        if s["flags"] & 4:
            return "code pointer"
        if not s["flags"] & 1:
            b = d[s["off"] + t - s["addr"]: s["off"] + t - s["addr"] + 220] if s["type"] != 8 else b""
            return "rodata: .rs path (panic location)" if rs.match(b) else "rodata: other"
        return "writable: " + s["name"]

    rel = []  # (location, target)
    for s in secs:
        if s["type"] == 19 or s["name"] == ".relr.dyn":
            base = 0
            for i in range(s["size"] // 8):
                e = struct.unpack_from("<Q", d, s["off"] + 8 * i)[0]
                if not e & 1:
                    rel.append((e, word(e)))
                    base = e + 8
                else:
                    e >>= 1
                    j = 0
                    while e:
                        if e & 1:
                            a = base + 8 * j
                            rel.append((a, word(a)))
                        e >>= 1
                        j += 1
                    base += 63 * 8
    types = collections.Counter()
    selfsym = collections.Counter()
    dynsym = next((s for s in secs if s["type"] == 11), None)
    dynstr = secs[dynsym["link"]] if dynsym else None
    for s in secs:
        if s["type"] != 4:
            continue
        for i in range(s["size"] // 24):
            off, info, add = struct.unpack_from("<QQq", d, s["off"] + 24 * i)
            typ, sym = info & 0xFFFFFFFF, info >> 32
            types[(s["name"], typ)] += 1
            if typ == 8:
                rel.append((off, add))
            elif sym and dynsym:
                shndx = struct.unpack_from("<H", d, dynsym["off"] + 24 * sym + 6)[0]
                selfsym[(typ, "defined in this object" if shndx else "undefined here")] += 1
    print(f"\n== RELOCATION CENSUS {path}")
    print("sections (alloc, >= 4 KiB):", ", ".join(f"{s['name']} {s['size']}" for s in alloc if s["size"] >= 4096))
    print("RELA entries by (section, type):", dict(types))
    print("symbolic relocations by (type, symbol):", dict(selfsym))
    byloc = collections.Counter()
    pages = collections.defaultdict(set)
    bytgt = collections.Counter()
    page_cls = collections.defaultdict(collections.Counter)
    for a, t in rel:
        s = sec_of(a)
        ln = s["name"] if s else "?"
        tc = tclass(t)
        byloc[ln] += 1
        pages[ln].add(a >> 12)
        bytgt[tc] += 1
        page_cls[a >> 12][tc] += 1
    print(f"relative relocations: {len(rel)} words, {len({a >> 12 for a, _ in rel})} distinct 4 KiB pages")
    for ln, c in byloc.most_common():
        sz = next((s["size"] for s in secs if s["name"] == ln), 0)
        print(f"  at {ln:24} {c:7} words  {len(pages[ln]):5} pages  (section {sz} bytes, {8 * c * 100 // max(sz, 1)}% of it relocated words)")
    print("  by target:", ", ".join(f"{k}: {v}" for k, v in bytgt.most_common()))
    only_rs = sum(1 for p, c in page_cls.items() if set(c) == {"rodata: .rs path (panic location)"})
    print(f"  pages whose every relocation targets a .rs path: {only_rs}")
    symtab = next((s for s in secs if s["type"] == 2), None)
    if symtab:
        strt = secs[symtab["link"]]
        syms = []
        for i in range(symtab["size"] // 24):
            nm, inf, oth, shn, val, size = struct.unpack_from("<IBBHQQ", d, symtab["off"] + 24 * i)
            if size and shn and (inf & 0xF) == 1:
                o = strt["off"] + nm
                syms.append((val, size, d[o:d.index(b"\0", o)].decode(errors="replace")))
        syms.sort()
        vs = [v for v, _, _ in syms]
        bysym = collections.Counter()
        for a, _ in rel:
            i = bisect.bisect_right(vs, a) - 1
            nm = syms[i][2] if i >= 0 and a < syms[i][0] + syms[i][1] else "(no object symbol)"
            bysym[nm] += 1
        print("  top objects holding relocated words:")
        for nm, c in bysym.most_common(25):
            print(f"    {c:6}  {nm[:110]}")
    else:
        print("  (no .symtab: per-object split not available)")


# ---------------------------------------------------------------- fault addresses by mapping
MM = re.compile(r"PERF_RECORD_MMAP2?\s+\d+/\d+:\s+\[0x([0-9a-f]+)\(0x([0-9a-f]+)\)\s+@\s+\S+[^\]]*\]:\s+([rwxps-]{4})\s+(.*)$")
SAMPLE = re.compile(r"^\s*([0-9a-f]+)\s+([0-9a-f]+)\s*(.*?)\s*\(([^()]*)\)\s*$")


def faults(prog, with_sym, args=()):
    data = f"pf.{prog}"
    r = sh(["perf", "record", "-q", "-e", "page-faults", "-c", "1", "-d", "-o", data, "--", "./" + prog] + list(args))
    fields = "ip,sym,dso,addr" if with_sym else "ip,dso,addr"
    out = sh(["perf", "script", "-i", data, "--show-mmap-events", "-F", fields]).stdout
    maps, bylabel, bysym, bydso, bad = [], collections.Counter(), collections.defaultdict(collections.Counter), collections.Counter(), []
    total = 0
    for ln in out.splitlines():
        m = MM.search(ln)
        if m:
            name = m.group(4).strip().split()[-1]
            maps.append((int(m.group(1), 16), int(m.group(1), 16) + int(m.group(2), 16), m.group(3),
                         os.path.basename(name) if name.startswith("/") else name))
            continue
        s = SAMPLE.match(ln)
        if not s:
            if ln.strip() and "PERF_RECORD" not in ln:
                bad.append(ln)
            continue
        total += 1
        addr, sym, dso = int(s.group(1), 16), s.group(3), os.path.basename(s.group(4))
        lab = None
        for lo, hi, perm, name in reversed(maps):
            if lo <= addr < hi:
                lab = f"{name} {perm}"
                break
        if lab is None:
            lab = "stack (no mapping event)" if addr >= 0x7FF000000000 else "brk heap / unmapped (no mapping event)"
        bylabel[lab] += 1
        bydso[dso] += 1
        if with_sym:
            bysym[lab][sym.split("+")[0] if sym else "?"] += 1
    print(f"\n== FAULTS {prog} {' '.join(args)}: {total} (rc {r.returncode}) by faulting address's mapping:")
    for lab, c in bylabel.most_common():
        extra = ""
        if with_sym:
            extra = "   <- " + ", ".join(f"{k[:48]} {v}" for k, v in bysym[lab].most_common(6))
        print(f"  {c:4}  {lab[:60]}{extra}")
    print("  by object whose code faulted:", ", ".join(f"{k} {v}" for k, v in bydso.most_common()))
    if bad:
        print("  unparsed lines:", len(bad), "first:", bad[0][:160])


def syscalls(prog, args=()):
    f = f"st.{prog}.txt"
    sh(["strace", "-f", "-o", f, "./" + prog] + list(args))
    c = collections.Counter()
    for ln in open(f):
        m = re.match(r"^(?:\d+\s+)?([a-z_0-9]+)\(", ln)
        if m:
            c[m.group(1)] += 1
    return c


def main():
    os.chdir(OUT)
    env = dict(os.environ)
    print("== start-up cycles, user and kernel (perf stat -r 21, mean; counters, not wall time)")
    for p in ("empty", "hello"):
        for v in ("ref-dyn", "ref-static", "niji-dyn", "niji-static"):
            r = sh(["perf", "stat", "-r", "21", "-x,", "-e", "cycles:u,cycles:k,instructions:u,instructions:k,page-faults",
                    "./" + f"{p}.{v}"])
            vals = {ln.split(",")[2]: ln.split(",")[0] for ln in r.stderr.splitlines() if len(ln.split(",")) > 3}
            print(f"  {p:6} {v:11} " + "  ".join(f"{k}={vals.get(k, '?')}" for k in
                  ("cycles:u", "cycles:k", "instructions:u", "instructions:k", "page-faults")))
    print("\n== objects")
    for v in ("ref-dyn", "niji-dyn"):
        r = sh(["readelf", "-lW", f"empty.{v}"]).stdout
        interp = re.search(r"interpreter: ([^\]]+)\]", r)
        need = re.findall(r"\(NEEDED\)\s+Shared library: \[([^\]]+)\]", sh(["readelf", "-dW", f"empty.{v}"]).stdout)
        print(v, "interpreter", interp.group(1) if interp else "?", "needed", need)
    ref_libc = [ln.split("=>")[1].split()[0] for ln in sh(["ldd", "empty.ref-dyn"]).stdout.splitlines() if "libc.so.6" in ln]
    ref_ld = re.search(r"interpreter: ([^\]]+)\]", sh(["readelf", "-lW", "empty.ref-dyn"]).stdout).group(1)
    niji_ld = re.search(r"interpreter: ([^\]]+)\]", sh(["readelf", "-lW", "empty.niji-dyn"]).stdout).group(1)
    niji_need = re.findall(r"\(NEEDED\)\s+Shared library: \[([^\]]+)\]", sh(["readelf", "-dW", "empty.niji-dyn"]).stdout)
    niji_libc = next((x for x in niji_need if "libc" in x), P + "-release-dyn/libc.so.6")
    if not niji_libc.startswith("/"):
        niji_libc = P + "-release-dyn/libc.so.6"
    for lab, path in (("ref ld.so", ref_ld), ("ref libc.so.6", ref_libc[0] if ref_libc else "?"),
                      ("niji ld.so", niji_ld), ("niji libc.so.6", niji_libc)):
        r = sh(["readelf", "-lW", path]).stdout
        segs = [" ".join(ln.split()) for ln in r.splitlines() if re.match(r"\s+(LOAD|GNU_RELRO|TLS)\b", ln)]
        nsym = len([ln for ln in sh(["readelf", "--dyn-syms", "-W", path]).stdout.splitlines() if re.match(r"\s+\d+:", ln)])
        print(f"-- {lab} {path} size {os.path.getsize(path)} dynsyms {nsym}")
        for sg in segs:
            print("   ", sg)

    print("\n== LD_DEBUG=statistics")
    for v in ("ref-dyn", "niji-dyn"):
        r = sh(["./empty." + v], env=dict(env, LD_DEBUG="statistics"))
        print(f"-- {v} rc {r.returncode}")
        print(head(r.stderr, 14))

    print("\n== syscalls (strace -f), dynamic programs")
    for p, args in (("empty", ()), ("fopen", ()), ("hello", ()), ("pc", ("pthread", "100"))):
        a = syscalls(f"{p}.ref-dyn", args)
        b = syscalls(f"{p}.niji-dyn", args)
        keys = sorted(set(a) | set(b), key=lambda k: -(a[k] + b[k]))
        print(f"-- {p} {' '.join(args)}: total ref {sum(a.values())} niji {sum(b.values())}")
        print("   " + ", ".join(f"{k} {a[k]}/{b[k]}" for k in keys))

    for path in (niji_ld, niji_libc):
        try:
            census(path)
        except Exception as e:  # report and go on
            print("census failed", path, e)

    for prog in ("empty", "hello", "setloc"):
        for v in ("niji-dyn", "ref-dyn", "niji-static", "ref-static"):
            faults(f"{prog}.{v}", with_sym=v.startswith("niji"))
    print("\n== first thread: faults of pc pthread 1 (compare with pc pthread 0)")
    for v in ("niji-dyn", "ref-dyn", "niji-static", "ref-static"):
        faults(f"pc.{v}", with_sym=v.startswith("niji"), args=("pthread", "0"))
        faults(f"pc.{v}", with_sym=v.startswith("niji"), args=("pthread", "1"))

    print("\n== libniji start-up profiles (perf record instructions:u -c 1000, 40 runs; libniji only)")
    for prog in ("empty", "fopen", "hello", "setloc"):
        for v in ("niji-dyn", "niji-static"):
            if v == "niji-static" and prog == "empty":
                continue
            b = f"{prog}.{v}"
            data = f"prof.{b}"
            loop = f"i=0; while [ $i -lt 40 ]; do LANG=C.UTF-8 ./{b} >/dev/null; i=$((i+1)); done"
            sh(["perf", "record", "-q", "-e", "instructions:u", "-c", "1000", "-o", data, "--", "sh", "-c", loop])
            print(f"\n-- {b} by object")
            print(report(data, 8, comm=b[:15], sort="dso"))
            print(f"-- {b} by symbol")
            print(report(data, 45 if v == "niji-dyn" else 20, comm=b[:15]))

    print("\n== libniji per-call profiles (perf record instructions:u -c 1000; libniji only)")
    for mode, n in (("malloc_free", 200000), ("pthread", 2000), ("iswalpha_loc", 200000), ("towupper_loc", 200000),
                    ("strcoll_loc", 200000), ("snprintf_d", 200000), ("fopen_fclose", 20000), ("memcmp64", 200000),
                    ("strcmp64", 200000), ("fprintf_null", 200000)):
        for v in ("niji-dyn", "niji-static"):
            data = f"prof.pc.{mode}.{v}"
            sh(["perf", "record", "-q", "-e", "instructions:u", "-c", "1000", "-o", data, "--", f"./pc.{v}", mode, str(n)],
               env=dict(env, PGSA_LOCALE=os.environ.get("PGSA_LOCALE", "en_US.UTF-8")))
            print(f"\n-- {mode} {v}")
            print(report(data, 14))
    print("\n== done")


main()
