#!/usr/bin/env python3
"""Header and reserved-name corpus scan (SL-HEADERS-1).  See validation/headers-corpus-README.md.

Subcommands:
  index  --work DIR                     verify the mirror chain, write the verified Sources / Packages indexes
  run    --mode a|b --work DIR [...]    scan the packages of set (a) or of the whole mirror (b)
  report --mode a|b --work DIR --date D --out-dir DIR   aggregate the per-package results (deterministic)

Everything is read from the project's LAN mirror only.  A path component named lib, gnulib or
gnulib-tests is never read: tar members under it are skipped (the archive stream must still be
decompressed past them, their content is dropped unread) and each skipped directory is logged.
"""
import argparse, concurrent.futures as cf, hashlib, io, json, lzma, os, re, shutil, subprocess
import sys, tarfile, time, urllib.request, zipfile

MIRROR = "http://miko.mgmt.internal/devuan/"
SUITE = "excalibur"
KEYRING = os.path.expanduser("~/keyrings/devuan-archive-keyring.gpg")
SKIP_COMPONENTS = {"lib", "gnulib", "gnulib-tests"}
CODE_EXT = {".c", ".h", ".cc", ".cpp", ".cxx", ".c++", ".hpp", ".hh", ".hxx", ".h++", ".cu", ".cuh", ".m", ".mm",
            ".y", ".l", ".s", ".in", ".inc", ".def", ".ipp", ".tcc", ".ii", ".i", ".gperf", ".xs", ".pgc", ".ec"}
CODE_EXT_UP = {".S", ".C", ".H", ".CC", ".CPP"}
MAX_MEMBER = 48 << 20
# C library source packages, excluded by name (logged with the reason)
CLIB_RE = re.compile(r"(^|-)(e?glibc|musl|dietlibc|klibc|newlib|picolibc|uclibc(-ng)?|avr-libc|bionic)(-|$)")

# R-H7 undecided legacy headers (plan/headers.md 5.1, before this scan)
UNDECIDED = ["a.out.h", "stab.h", "gconv.h", "re_comp.h", "regexp.h", "gnu-versions.h", "netash/ash.h",
             "neteconet/ec.h", "netiucv/iucv.h", "netrom/netrom.h", "nfs/nfs.h", "sys/raw.h", "sys/vlimit.h",
             "sys/vm86.h"]

INC_RE = re.compile(r"^[ \t]*#[ \t]*include(?:_next)?[ \t]*<([^>\s]+)>", re.M)
IDENT_RE = re.compile(
    r"\b(__GLIBC__|__GLIBC_MINOR__|__GLIBC_PREREQ|__GNU_LIBRARY__|__GNU_LIBRARY_MINOR__|__USE_[A-Z0-9_]+"
    r"|__BEGIN_DECLS|__END_DECLS|__THROW|__THROWNL|__NTH|__NTHNL|__P|__PMT|__CONCAT|__STRING|__nonnull|__wur"
    r"|__attribute_[a-z_0-9]+?__|__attribute_[a-z_0-9]+|__glibc_[a-z_]+|__REDIRECT[A-Z_]*|__ASMNAME[0-9]?"
    r"|__fortify_function|__extern_always_inline|__extern_inline|__LDBL_REDIR[A-Z0-9_]*|__bos0?|__bnd"
    r"|__[a-z][a-z0-9_]*_t)\b")
FILE_ANCHOR_RE = re.compile(r"\b(FILE|_IO_FILE|stdin|stdout|stderr)\b")
LAYOUTS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "spec", "abi", "layouts.tsv")


def file_members():
    out = []
    with open(LAYOUTS) as f:
        for ln in f:
            c = ln.rstrip("\n").split("\t")
            if c[0] == "FILE" and c[1]:
                out.append(c[1])
    return sorted(out)


FILE_MEMBERS = file_members()
IO_NAMES = [m for m in FILE_MEMBERS if m.startswith("_IO_")]
PLAIN_NAMES = [m for m in FILE_MEMBERS if not m.startswith("_IO_")]
IO_RE = re.compile(r"\b(" + "|".join(map(re.escape, IO_NAMES)) + r")\b")
ACC_RE = re.compile(r"(?:->|\.)[ \t]*(" + "|".join(map(re.escape, PLAIN_NAMES)) + r")\b")


def http_get(url, dest, tries=5):
    last = None
    for i in range(tries):
        try:
            with urllib.request.urlopen(url, timeout=120) as r, open(dest, "wb") as o:
                shutil.copyfileobj(r, o, 1 << 20)
            return
        except Exception as e:  # noqa
            last = e
            time.sleep(2 * (i + 1))
    raise RuntimeError("download failed %s: %s" % (url, last))


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while True:
            b = f.read(1 << 20)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def cmd_index(a):
    w = a.work
    os.makedirs(w, exist_ok=True)
    http_get(MIRROR + "dists/%s/InRelease" % SUITE, w + "/InRelease.raw")
    r = subprocess.run(["gpgv", "--keyring", KEYRING, "--output", w + "/InRelease", w + "/InRelease.raw"],
                       capture_output=True, text=True)
    open(w + "/gpgv.log", "w").write(r.stderr)
    if r.returncode != 0 or "Good signature" not in r.stderr:
        sys.exit("InRelease does not verify:\n" + r.stderr)
    want = {}
    sec = False
    for ln in open(w + "/InRelease"):
        if ln.startswith("SHA256:"):
            sec = True
            continue
        if sec and ln.startswith(" "):
            h, _, p = ln.split()
            want[p] = h
        elif sec:
            sec = False
    for rel, name in (("main/source/Sources.xz", "Sources.xz"), ("main/binary-amd64/Packages.xz", "Packages.xz")):
        http_get(MIRROR + "dists/%s/%s" % (SUITE, rel), w + "/" + name)
        if sha256(w + "/" + name) != want[rel]:
            sys.exit("hash mismatch for " + rel)
        print("verified", rel, want[rel])
    print(r.stderr.strip())


def stanzas(path):
    with lzma.open(path, "rt", encoding="utf-8", errors="replace") as f:
        txt = f.read()
    for st in txt.split("\n\n"):
        if not st.strip():
            continue
        d, key = {}, None
        for ln in st.split("\n"):
            if ln[:1] in " \t":
                d[key] += "\n" + ln.strip()
            else:
                key, _, v = ln.partition(":")
                d[key] = v.strip()
        yield d


def load_sources(w):
    pk = {}
    for d in stanzas(w + "/Sources.xz"):
        files = []
        for ln in d.get("Checksums-Sha256", "").split("\n"):
            if ln.strip():
                h, sz, nm = ln.split()
                files.append((nm, h, int(sz)))
        pk[d["Package"]] = {"version": d["Version"], "dir": d["Directory"], "files": files}
    return pk


def sets_a(w, smoke_names):
    src_of = {}
    for d in stanzas(w + "/Packages.xz"):
        src_of[d["Package"]] = (d.get("Source", d["Package"]).split()[0], d)
    ess = sorted({s for s, d in src_of.values() if d.get("Essential") == "yes"})
    be = {"build-essential"}
    deps = src_of["build-essential"][1].get("Depends", "")
    for alt in deps.split(","):
        n = alt.split("|")[0].split("(")[0].strip()
        if n in src_of:
            be.add(src_of[n][0])
    # gcc and g++ are stub packages of gcc-defaults: follow them (two levels) to the compiler and binutils sources
    frontier = [n for n in be if n == "gcc-defaults"]
    for _ in range(2):
        nxt = []
        for p, (s, d) in src_of.items():
            if s in frontier or (p in ("gcc", "g++") and not frontier):
                for alt in d.get("Depends", "").split(","):
                    n = alt.split("|")[0].split("(")[0].strip()
                    if n in src_of and re.match(r"^(gcc-\d+|binutils)$", src_of[n][0]):
                        be.add(src_of[n][0])
                        nxt.append(src_of[n][0])
        frontier = nxt
    return {"essential": ess, "build-essential": sorted(be), "smoke": sorted(smoke_names)}


def smoke_sources(repo):
    names = []
    for ln in open(os.path.join(repo, "validation", "smoke", "programs.sh")):
        m = re.match(r"^[a-z0-9_]+_src=([A-Za-z0-9.+_-]+)\s*$", ln)
        if m:
            names.append(m.group(1))
    return sorted(set(names))


def is_code(name):
    ext = os.path.splitext(name)[1]
    return ext in CODE_EXT or ext in CODE_EXT_UP


class Acc:
    def __init__(self, mode):
        self.mode = mode
        self.inc = {}      # header -> [occurrences, files]
        self.idn = {}      # identifier -> [occ, files]
        self.fm = {}       # FILE member -> [occ, files]
        self.names = set()  # non-skipped member paths (for own-file check)
        self.skips = []
        self.nfiles = 0
        self.nbig = 0
        self.nbytes = 0
        self.depth = 0
        self.nest = 0
        self.tmp = None

    def add(self, d, k, n):
        e = d.setdefault(k, [0, 0])
        e[0] += n
        e[1] += 1

    def scan_text(self, txt):
        self.nfiles += 1
        self.nbytes += len(txt)
        if self.mode == "b":
            if "include" not in txt:
                return
            found = {}
            for h in INC_RE.findall(txt):
                if h in UNDECIDED_SET:
                    found[h] = found.get(h, 0) + 1
            for h, n in found.items():
                self.add(self.inc, h, n)
            return
        cnt = {}
        for h in INC_RE.findall(txt):
            cnt[h] = cnt.get(h, 0) + 1
        for h, n in cnt.items():
            self.add(self.inc, h, n)
        cnt = {}
        for m in IDENT_RE.findall(txt):
            cnt[m] = cnt.get(m, 0) + 1
        for h, n in cnt.items():
            self.add(self.idn, h, n)
        cnt = {}
        for m in IO_RE.findall(txt):
            cnt[m] = cnt.get(m, 0) + 1
        if FILE_ANCHOR_RE.search(txt):
            for m in ACC_RE.findall(txt):
                cnt[m] = cnt.get(m, 0) + 1
        for h, n in cnt.items():
            self.add(self.fm, h, n)

    def feed(self, path, data):
        self.names.add(path)
        if is_code(path):
            if len(data) > MAX_MEMBER:
                self.nbig += 1
                return
            self.scan_text(data.decode("latin-1"))


UNDECIDED_SET = set(UNDECIDED)
NESTED_RE = re.compile(r"\.(tar\.gz|tgz|tar\.bz2|tbz2?|tar\.xz|txz|tar\.zst)$")
CLIB_NESTED_RE = re.compile(r"^(e?glibc|musl|dietlibc|klibc|newlib|picolibc|uclibc(-ng)?|avr-libc|bionic)[-_.]")


def skip_prefix(comps):
    for i, c in enumerate(comps):
        if c in SKIP_COMPONENTS:
            return "/".join(comps[:i + 1])
    return None


def scan_tar_stream(fobj, acc, label):
    skipped = set()
    with tarfile.open(fileobj=fobj, mode="r|*") as tf:
        for m in tf:
            comps = [c for c in m.name.split("/") if c not in ("", ".")]
            sp = skip_prefix(comps)
            if sp:
                skipped.add(sp)
                continue
            if not m.isfile():
                continue
            path = "/".join(comps)
            if NESTED_RE.search(path) and m.size > 0 and acc.depth < 3:
                acc.names.add(path)
                if CLIB_NESTED_RE.match(comps[-1]):
                    acc.skips.append("%s:%s (nested C library archive, not opened)" % (label, path))
                    continue
                acc.nest += 1
                tmpf = os.path.join(acc.tmp, "nested-%d" % acc.nest + NESTED_RE.search(path).group(0))
                with open(tmpf, "wb") as o:
                    shutil.copyfileobj(tf.extractfile(m), o, 1 << 20)
                acc.depth += 1
                try:
                    scan_file(tmpf, label + ">" + comps[-1], acc)
                finally:
                    acc.depth -= 1
                    os.unlink(tmpf)
                continue
            if not is_code(path):
                acc.names.add(path)
                continue
            if m.size > MAX_MEMBER:
                acc.names.add(path)
                acc.nbig += 1
                continue
            acc.feed(path, tf.extractfile(m).read())
    for sp in sorted(skipped):
        acc.skips.append("%s:%s" % (label, sp))


def scan_file(path, label, acc):
    n = os.path.basename(path)
    if re.search(r"\.tar\.zst$", n):
        p = subprocess.Popen(["zstd", "-dc", path], stdout=subprocess.PIPE)
        scan_tar_stream(p.stdout, acc, label)
        p.stdout.close()
        if p.wait() != 0:
            raise RuntimeError("zstd failed on " + n)
    elif re.search(r"\.(tar|tar\.gz|tgz|tar\.bz2|tbz2?|tar\.xz|txz|tar\.lzma)$", n):
        with open(path, "rb") as f:
            scan_tar_stream(f, acc, label)
    elif n.endswith(".zip"):
        skipped = set()
        with zipfile.ZipFile(path) as z:
            for zi in z.infolist():
                comps = [c for c in zi.filename.split("/") if c not in ("", ".")]
                sp = skip_prefix(comps)
                if sp:
                    skipped.add(sp)
                    continue
                if zi.is_dir():
                    continue
                p = "/".join(comps)
                if is_code(p) and zi.file_size <= MAX_MEMBER:
                    acc.feed(p, z.read(zi))
                else:
                    acc.names.add(p)
        for sp in sorted(skipped):
            acc.skips.append("%s:%s" % (label, sp))
    elif re.search(r"\.diff\.(gz|xz|bz2)$", n):
        import gzip, bz2
        op = {"gz": gzip.open, "xz": lzma.open, "bz2": bz2.open}[n.rsplit(".", 1)[1]]
        with op(path, "rb") as f:
            acc.scan_text(f.read().decode("latin-1"))
    elif n.endswith(".dsc") or n.endswith(".asc"):
        pass
    else:
        raise RuntimeError("unsupported archive format: " + n)


def process(task):
    name, info, mode, tmp = task
    acc = Acc(mode)
    d = os.path.join(tmp, "%d-%s" % (os.getpid(), name))
    os.makedirs(d, exist_ok=True)
    acc.tmp = d
    try:
        for fn, h, sz in info["files"]:
            dest = os.path.join(d, fn)
            http_get(MIRROR + info["dir"] + "/" + fn, dest)
            if sha256(dest) != h:
                raise RuntimeError("sha256 mismatch " + fn)
            scan_file(dest, fn, acc)
            os.unlink(dest)
        hits = {}
        discarded = []
        for h, (occ, files) in acc.inc.items():
            if mode == "b":
                own = any(p == h or p.endswith("/" + h) for p in acc.names)
                if own:
                    discarded.append(h)
                    continue
            hits[h] = [occ, files]
        rec = {"pkg": name, "version": info["version"], "status": "scanned", "files": acc.nfiles,
               "bytes": acc.nbytes, "big_skipped": acc.nbig, "skips": sorted(acc.skips)}
        if mode == "b":
            rec["hits"] = hits
            rec["own_file_discarded"] = sorted(discarded)
        else:
            rec["inc"] = hits
            rec["idn"] = acc.idn
            rec["fm"] = acc.fm
        return rec
    except Exception as e:  # noqa
        return {"pkg": name, "version": info["version"], "status": "failed", "reason": str(e)[:300]}
    finally:
        shutil.rmtree(d, ignore_errors=True)


def cmd_run(a):
    w = a.work
    pk = load_sources(w)
    smoke = smoke_sources(a.repo)
    sets = sets_a(w, smoke)
    if a.mode == "a":
        todo = sorted({n for v in sets.values() for n in v})
    else:
        todo = sorted(pk)
    resf = os.path.join(w, "results-%s.jsonl" % a.mode)
    done = {}
    if os.path.exists(resf):
        for ln in open(resf):
            r = json.loads(ln)
            done[r["pkg"]] = r
    exclf = os.path.join(w, "excluded-%s.tsv" % a.mode)
    with open(exclf, "w") as ex:
        ex.write("package\tversion\treason\n")
        for n in todo:
            if n not in pk:
                ex.write("%s\t\tnot in Sources of main\n" % n)
            elif CLIB_RE.search(n):
                ex.write("%s\t%s\tC library source package (by name)\n" % (n, pk[n]["version"]))
    todo = [n for n in todo if n in pk and not CLIB_RE.search(n)]
    for attempt in range(a.passes):
        pend = [n for n in todo if done.get(n, {}).get("status") != "scanned"]
        print("pass %d: %d of %d to do" % (attempt + 1, len(pend), len(todo)), flush=True)
        if not pend:
            break
        pend.sort(key=lambda n: -sum(f[2] for f in pk[n]["files"]))
        os.makedirs(a.tmp, exist_ok=True)
        inflight, used = {}, 0
        with cf.ProcessPoolExecutor(max_workers=a.jobs) as ex, open(resf, "a") as out:
            i, nd = 0, 0
            while i < len(pend) or inflight:
                while i < len(pend) and len(inflight) < a.jobs:
                    sz = sum(f[2] for f in pk[pend[i]]["files"])
                    if inflight and used + sz > a.cap:
                        break
                    fut = ex.submit(process, (pend[i], pk[pend[i]], a.mode, a.tmp))
                    inflight[fut] = (pend[i], sz)
                    used += sz
                    i += 1
                dn, _ = cf.wait(list(inflight), return_when=cf.FIRST_COMPLETED)
                for fut in dn:
                    n, sz = inflight.pop(fut)
                    used -= sz
                    try:
                        r = fut.result()
                    except Exception as e:  # noqa
                        r = {"pkg": n, "version": pk[n]["version"], "status": "failed", "reason": "worker: %s" % e}
                    done[n] = r
                    out.write(json.dumps(r, sort_keys=True) + "\n")
                    out.flush()
                    nd += 1
                    if nd % 200 == 0:
                        print("  %d/%d done" % (nd, len(pend)), flush=True)
    bad = [n for n in todo if done.get(n, {}).get("status") != "scanned"]
    print("unfinished after %d passes: %d %s" % (a.passes, len(bad), bad[:20]))
    return 1 if bad else 0


def load_results(w, mode):
    done = {}
    p = os.path.join(w, "results-%s.jsonl" % mode)
    for ln in open(p):
        r = json.loads(ln)
        if r["status"] == "scanned" or r["pkg"] not in done:
            done[r["pkg"]] = r
    return done


def cmd_report(a):
    w = a.work
    done = load_results(w, a.mode)
    pk = load_sources(w)
    excl = [ln.rstrip("\n").split("\t") for ln in list(open(os.path.join(w, "excluded-%s.tsv" % a.mode)))[1:]]
    os.makedirs(a.out_dir, exist_ok=True)
    gz = hashlib.sha256()
    if a.mode == "b":
        return report_b(a, done, pk, excl)
    sets = sets_a(w, smoke_sources(a.repo))
    rows, md = [], []
    grp = {"all": sorted({n for v in sets.values() for n in v})}
    grp.update(sets)
    for sname in ["essential", "build-essential", "smoke", "all"]:
        pkgs = [n for n in grp[sname] if n in done and done[n]["status"] == "scanned"]
        for kind in ("inc", "idn", "fm"):
            agg = {}
            for n in pkgs:
                for k, (occ, files) in done[n][kind].items():
                    e = agg.setdefault(k, [0, 0, 0])
                    e[0] += occ
                    e[1] += files
                    e[2] += 1
            for k in sorted(agg):
                kk = kind
                if kind == "inc":
                    kk = "include-bits-gnu" if k.startswith(("bits/", "gnu/")) else "include"
                elif kind == "idn":
                    kk = ("use-" + ("__USE" if k.startswith("__USE_") else
                                    "identity" if k in ("__GLIBC__", "__GLIBC_MINOR__", "__GLIBC_PREREQ", "__GNU_LIBRARY__", "__GNU_LIBRARY_MINOR__") else
                                    "reserved-type" if re.fullmatch(r"__[a-z][a-z0-9_]*_t", k) else "cdefs"))
                else:
                    kk = "file-member"
                rows.append((sname, kk, k, agg[k][0], agg[k][1], agg[k][2]))
    tsv = os.path.join(a.out_dir, "headers-corpus-%s.tsv" % a.date)
    with open(tsv, "w") as f:
        f.write("set\tkind\tname\toccurrences\tfiles\tpackages\n")
        for r in sorted(rows):
            f.write("\t".join(map(str, r)) + "\n")
    # markdown
    def top(sname, kind, n=25):
        rs = [r for r in rows if r[0] == sname and r[1] == kind]
        rs.sort(key=lambda r: (-r[5], -r[3], r[2]))
        return rs[:n]
    L = ["# Header and reserved-name corpus scan (SL-HEADERS-1, part a)", "",
         "Date: %s. Tool: `validation/headers_corpus.py` (README: `validation/headers-corpus-README.md`). TSV: `%s`."
         % (a.date, os.path.basename(tsv)), "",
         "Source: Devuan excalibur `main/source/Sources.xz`, chain verified (InRelease with gpgv and the pinned keyring, then "
         "each index and every source file against its SHA-256). Program sources only: any `lib/`, `gnulib/` or `gnulib-tests/` "
         "path component was skipped unread (log below). Counts are `occurrences`, `files`, `packages` (distinct source packages).", "",
         "## Sets", ""]
    for sname in ("essential", "build-essential", "smoke"):
        ok = [n for n in grp[sname] if n in done and done[n]["status"] == "scanned"]
        L.append("* **%s**: %d source packages named, %d scanned: %s" % (sname, len(grp[sname]), len(ok), ", ".join(grp[sname])))
    L += ["", "Excluded by name or not in main:", ""]
    L += ["* `%s` %s: %s" % (e[0], e[1], e[2]) for e in excl] or ["* none"]
    tf = sum(done[n]["files"] for n in grp["all"] if n in done and done[n]["status"] == "scanned")
    L += ["", "Scanned %d source files in %d packages." % (tf, len([n for n in grp["all"] if n in done and done[n]["status"] == "scanned"])), ""]
    for sname in ("all", "essential", "build-essential", "smoke"):
        L += ["## Set `%s`: top counts" % sname, ""]
        for kind, title in (("include", "`#include <...>` (by packages, top 25)"), ("include-bits-gnu", "direct `bits/` and `gnu/` includes"),
                            ("use-identity", "identity macros"), ("use-__USE", "`__USE_*`"), ("use-cdefs", "`sys/cdefs.h` vocabulary"),
                            ("use-reserved-type", "reserved `__*_t` names (top 25)"), ("file-member", "FILE members")):
            rs = top(sname, kind, 25 if kind in ("include", "use-reserved-type") else 60)
            L += ["### %s" % title, ""]
            if not rs:
                L += ["none", ""]
                continue
            L += ["| name | occurrences | files | packages |", "|---|---|---|---|"]
            L += ["| `%s` | %d | %d | %d |" % (r[2], r[3], r[4], r[5]) for r in rs]
            L.append("")
    L += ["## Skip log (directories never read)", ""]
    sk = []
    for n in sorted(done):
        if done[n]["status"] == "scanned" and n in grp["all"]:
            sk += ["%s %s" % (n, s) for s in done[n]["skips"]]
    L += ["%d skipped directories across the sets." % len(sk), ""] + ["* `%s`" % s for s in sk]
    big = sum(done[n]["big_skipped"] for n in grp["all"] if n in done and done[n]["status"] == "scanned")
    L += ["", "Code-extension members over 48 MiB not scanned: %d." % big, "",
          "## Limits", "",
          "* FILE members other than `_IO_*` are counted only as `->name` or `.name` in files that also mention `FILE`, `stdin`, `stdout` or `stderr`; "
          "generic names (`_flags`, `_mode`, `_offset`, `_lock`) therefore over-count.",
          "* `__*_t` includes names a package defines for itself and compiler names; read the package count with that in mind.",
          "* Identifier counts include mentions in comments and `#ifdef` lines (a test for a name is a use of the name).",
          "* Archives nested inside archives are not opened."]
    open(os.path.join(a.out_dir, "headers-corpus-%s.md" % a.date), "w").write("\n".join(L) + "\n")


def report_b(a, done, pk, excl):
    todo = [n for n in sorted(pk) if not CLIB_RE.search(n)]
    scanned = [n for n in todo if done.get(n, {}).get("status") == "scanned"]
    failed = [n for n in todo if n not in scanned]
    pairs = sorted((n, h) for n in scanned for h in done[n]["hits"])
    tsv = os.path.join(a.out_dir, "headers-rh7-%s.tsv" % a.date)
    with open(tsv, "w") as f:
        f.write("package\theader\n")
        for n, h in pairs:
            f.write("%s\t%s\n" % (n, h))
    by = {h: [n for n, hh in pairs if hh == h] for h in UNDECIDED}
    disc = {h: [n for n in scanned if h in done[n]["own_file_discarded"]] for h in UNDECIDED}
    skips = sum(len(done[n]["skips"]) for n in scanned)
    L = ["# R-H7 scan of the whole mirror (SL-HEADERS-1, part b)", "",
         "Date: %s. Tool: `validation/headers_corpus.py run --mode b`. TSV (`package<TAB>header`): `%s`." % (a.date, os.path.basename(tsv)), "",
         "Source: Devuan excalibur `main/source/Sources.xz`, chain verified; every source file checked against its SHA-256. "
         "%d source packages in Sources. Accounting: **%d scanned**, **%d excluded** (C library source packages, by name), **%d not scanned** (failed after retries)."
         % (len(pk), len(scanned), len([n for n in pk if CLIB_RE.search(n)]), len(failed)), "",
         "Excluded packages:", ""]
    L += ["* `%s` %s" % (e[0], e[1]) for e in excl] or ["* none"]
    L += ["", "Not scanned: %s" % (", ".join(failed) if failed else "none"), "",
          "A hit is an `#include <header>` in a code-extension file outside any `lib/`, `gnulib/` or `gnulib-tests/` path component. "
          "A hit is discarded when the same package ships a file of that name outside those directories. "
          "%d directories were skipped unread (each is in the per-package results, not repeated here)." % skips, "",
          "| header | source packages with a hit | discarded (own file) | packages |", "|---|---|---|---|"]
    for h in UNDECIDED:
        L.append("| `%s` | %d | %d | %s |" % (h, len(by[h]), len(disc[h]), ", ".join("`%s`" % n for n in by[h][:30]) +
                                              (" ..." if len(by[h]) > 30 else "")))
    L += ["", "Decision (R-H7): a header with a hit is shipped, one with none is dropped."]
    open(os.path.join(a.out_dir, "headers-rh7-%s.md" % a.date), "w").write("\n".join(L) + "\n")
    # skip log as its own file
    with open(os.path.join(a.out_dir, "headers-rh7-%s-skips.txt" % a.date), "w") as f:
        for n in scanned:
            for s in done[n]["skips"]:
                f.write("%s\t%s\n" % (n, s))


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    for nm in ("index", "run", "report"):
        p = sub.add_parser(nm)
        p.add_argument("--work", required=True)
        p.add_argument("--repo", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
        if nm != "index":
            p.add_argument("--mode", choices=["a", "b"], required=True)
        if nm == "run":
            p.add_argument("--jobs", type=int, default=12)
            p.add_argument("--passes", type=int, default=4)
            p.add_argument("--tmp", default="/dev/shm/claude/SL-HEADERS-1")
            p.add_argument("--cap", type=int, default=20 << 30, help="bytes of downloaded sources in flight")
        if nm == "report":
            p.add_argument("--date", required=True)
            p.add_argument("--out-dir", required=True)
    a = ap.parse_args()
    sys.exit({"index": cmd_index, "run": cmd_run, "report": cmd_report}[a.cmd](a) or 0)


if __name__ == "__main__":
    main()
