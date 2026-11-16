#!/usr/bin/env python3
"""tools/deb/parity.py: package parity audit (SYS-PACKAGE-PARITY-AUDIT; plan/package-parity-audit.md).

Compares the binary packages Debian builds from its C library source (the reference) with libniji's
system-flavour packages (tools/deb/build.sh --flavour system), and ranks every gap by how many
Debian binary packages would notice it. Package metadata, file lists and symbol tables only: no
file's contents are read except ELF headers and dynamic tables (readelf) and the dpkg control
members that are metadata (control, conffiles, triggers, shlibs, the header lines of symbols).
Maintainer-script bodies are never read (name and size only). The reference's glibc-source
package is listed, never unpacked.

usage:
  parity.py index WD [--suite trixie]
      Fetch the suite's InRelease, Packages.xz (main contrib non-free non-free-firmware; suite,
      -updates, -security; later wins) and main Sources.xz into WD; each index is checked against
      the SHA256 list of its InRelease. (InRelease's signature is checked only when --keyring K is
      given; the record says which.)
  parity.py fetch-ref WD
      Download every amd64/all binary package whose Source is glibc (newest version), SHA256-checked.
  parity.py inventory OUT-PREFIX DEB...
      Write OUT-PREFIX-{control,files,elf,dynsyms}.tsv (sorted, one line per fact).
  parity.py compare DIR
      DIR/{ref,ours}-*.tsv -> DIR/gaps.tsv (class, package, item, source line, detail).
  parity.py rank WD DIR [--sample N] [--seed S] [--jobs J]
      Exact reverse-relation counts from WD's indices, and an ELF scan of N packages drawn at
      random (seed S) from the amd64 packages that depend on libc6: DIR/consumers-*.tsv and
      DIR/rank.tsv (each gap with its counts).
"""
import argparse, bisect, concurrent.futures, hashlib, io, json, lzma, os, random, re, shutil
import subprocess, sys, tarfile, tempfile, urllib.request
from collections import Counter, defaultdict

DEBIAN = "http://deb.debian.org/debian"
SECURITY = "http://security.debian.org/debian-security"
COMPS = ["main", "contrib", "non-free", "non-free-firmware"]
REL_FIELDS = ["Pre-Depends", "Depends", "Recommends", "Suggests", "Provides", "Breaks",
              "Replaces", "Conflicts", "Enhances"]
CTRL_FIELDS = ["Package", "Source", "Version", "Architecture", "Multi-Arch", "Section",
               "Priority", "Essential", "Protected", "Build-Essential"] + REL_FIELDS
META_MEMBERS = {"control", "conffiles", "triggers", "shlibs"}  # read whole; symbols: headers only
EXEC_DIR = re.compile(r"^\./(usr/)?(lib|lib64|libx32|lib32|libexec|bin|sbin)/")
NO_ELF = re.compile(r"^\./usr/lib/(debug|locale)/")


# ---------------------------------------------------------------- indices
def fetch(url, dest=None):
    with urllib.request.urlopen(url, timeout=120) as r:
        b = r.read()
    if dest:
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, "wb") as f:
            f.write(b)
    return b


def sha256(b):
    return hashlib.sha256(b).hexdigest()


def stanzas(text):
    cur, key = {}, None
    for line in text.splitlines():
        if not line.strip():
            if cur:
                yield cur
            cur, key = {}, None
        elif line[0] in " \t" and key:
            cur[key] += "\n" + line
        else:
            key, _, val = line.partition(":")
            cur[key] = val.strip()
    if cur:
        yield cur


def suites(base):
    return [(DEBIAN, base), (DEBIAN, base + "-updates"), (SECURITY, base + "-security")]


def release_sums(txt):
    sums, on = {}, False
    for line in txt.splitlines():
        if line.startswith("SHA256:"):
            on = True
        elif on and line.startswith(" "):
            h, _, name = line.split()
            sums[name] = h
        else:
            on = False
    return sums


def cmd_index(a):
    meta = {}
    for root, s in suites(a.suite):
        raw = fetch(f"{root}/dists/{s}/InRelease", f"{a.wd}/dists/{s}/InRelease")
        sig = "not checked (no keyring given)"
        if a.keyring:
            r = subprocess.run(["gpgv", "--keyring", a.keyring, f"{a.wd}/dists/{s}/InRelease"],
                               capture_output=True, text=True)
            if r.returncode:
                sys.exit(f"gpgv failed for {s}")
            sig = "gpgv ok"
        txt = raw.decode()
        sums = release_sums(txt)
        date = re.search(r"^Date: (.*)$", txt, re.M).group(1)
        meta[s] = {"root": root, "date": date, "signature": sig, "files": {}}
        names = [f"{c}/binary-amd64/Packages.xz" for c in COMPS]
        names += [f"{c}/source/Sources.xz" for c in COMPS]
        for name in names:
            if name not in sums:
                meta[s]["files"][name] = "absent"
                continue
            b = fetch(f"{root}/dists/{s}/{name}", f"{a.wd}/dists/{s}/{name}")
            if sha256(b) != sums[name]:
                sys.exit(f"sha256 mismatch {s} {name}")
            meta[s]["files"][name] = "sha256 ok"
    with open(f"{a.wd}/index-meta.json", "w") as f:
        json.dump(meta, f, indent=1, sort_keys=True)
    print(json.dumps(meta, indent=1, sort_keys=True))


def dpkg_newer(a, b):
    return subprocess.run(["dpkg", "--compare-versions", a, "gt", b]).returncode == 0


def load_packages(wd, suite):
    """Binary packages, newest version per name over suite, -updates and -security."""
    pk = {}
    for root, s in suites(suite):
        for c in COMPS:
            p = f"{wd}/dists/{s}/{c}/binary-amd64/Packages.xz"
            if not os.path.exists(p):
                continue
            for st in stanzas(lzma.decompress(open(p, "rb").read()).decode("utf-8", "replace")):
                st["_root"], st["_comp"] = root, c
                old = pk.get(st["Package"])
                if old is None or dpkg_newer(st["Version"], old["Version"]):
                    pk[st["Package"]] = st
    return pk


def load_sources(wd, suite):
    src = {}
    for root, s in suites(suite):
        for c in COMPS:
            p = f"{wd}/dists/{s}/{c}/source/Sources.xz"
            if not os.path.exists(p):
                continue
            for st in stanzas(lzma.decompress(open(p, "rb").read()).decode("utf-8", "replace")):
                src[st["Package"]] = st  # later suite wins
    return src


def src_of(st):
    return st.get("Source", st["Package"]).split()[0]


def fetch_deb(st, dest_dir):
    dest = f"{dest_dir}/{os.path.basename(st['Filename'])}"
    b = fetch(f"{st['_root']}/{st['Filename']}")
    if sha256(b) != st["SHA256"]:
        raise RuntimeError("sha256 mismatch " + st["Filename"])
    with open(dest, "wb") as f:
        f.write(b)
    return dest


def cmd_fetch_ref(a):
    pk = load_packages(a.wd, a.suite)
    ref = sorted(n for n, st in pk.items() if src_of(st) == "glibc")
    os.makedirs(f"{a.wd}/ref-debs", exist_ok=True)
    for n in ref:
        print(fetch_deb(pk[n], f"{a.wd}/ref-debs"))


# ---------------------------------------------------------------- inventory
def deb_ar_members(path):
    r = subprocess.run(["ar", "t", path], capture_output=True, text=True, check=True)
    return r.stdout.split()


def ctrl_inventory(path):
    """(control stanza, [(member, size)], {member: text}) from the control tar."""
    t = subprocess.run(["dpkg-deb", "--ctrl-tarfile", path], capture_output=True, check=True).stdout
    members, texts = [], {}
    with tarfile.open(fileobj=io.BytesIO(t)) as tf:
        for m in tf.getmembers():
            if not m.isfile():
                continue
            name = m.name.lstrip("./")
            members.append((name, m.size))
            if name in META_MEMBERS:
                texts[name] = tf.extractfile(m).read().decode("utf-8", "replace")
            elif name == "symbols":
                body = tf.extractfile(m).read().decode("utf-8", "replace").splitlines()
                heads = [l for l in body if l and not l[0].isspace() and not l.startswith(("|", "*", "#"))]
                texts[name] = "\n".join(heads) + f"\n({sum(1 for l in body if l.startswith(' '))} symbol lines)"
    ctl = next(stanzas(texts["control"]))
    return ctl, members, texts


def readelf(path, *opts):
    return subprocess.run(["readelf", "-W", *opts, path], capture_output=True, text=True,
                          errors="replace").stdout


def elf_dynamic(path):
    """(soname, [needed], [(symbol, version, '@@'|'@'|'', type)] defined, [(lib, sym, ver)] undefined)."""
    d = readelf(path, "-d")
    soname = (re.findall(r"\(SONAME\).*\[(.*)\]", d) or [""])[0]
    needed = re.findall(r"\(NEEDED\).*\[(.*)\]", d)
    v = readelf(path, "-V")
    # .gnu.version_r: "File: libm.so.6  Cnt: 2" then "Name: GLIBC_2.2.5  Flags: none  Version: 3"
    vlib, cur = {}, None
    for line in v.splitlines():
        m = re.search(r"File: (\S+)", line)
        if m:
            cur = m.group(1)
            continue
        m = re.search(r"Name: (\S+)\s+Flags: \S+\s+Version: (\d+)", line)
        if m and cur:
            vlib[m.group(2)] = cur
    defined, undef = [], []
    for line in readelf(path, "--dyn-syms").splitlines():
        f = line.split()
        if len(f) < 8 or not f[0].endswith(":") or f[4] == "LOCAL":
            continue
        name = f[7]
        m = re.match(r"([^@]+)(@@?)([^ ]*)$", name)
        sym, at, ver = (m.group(1), m.group(2), m.group(3)) if m else (name, "", "")
        if f[6] == "UND":
            idx = f[8].strip("()") if len(f) > 8 else ""
            undef.append((vlib.get(idx, ""), sym, ver))
        else:
            defined.append((sym, ver, at, f[3]))
    return soname, needed, defined, undef


def is_x86_64(body):
    """ELFCLASS64 and EM_X86_64 (62); x32 is ELFCLASS32 with the same machine."""
    return body[4] == 2 and int.from_bytes(body[18:20], "little") == 62


def elf_class(body):
    return {(1, 3): "i386", (1, 62): "x32"}.get((body[4], int.from_bytes(body[18:20], "little")), "other")


def is_elf_candidate(name):
    return bool(EXEC_DIR.match(name)) and not NO_ELF.match(name)


def data_members(path, want_elf):
    """Yield (type, size, name, linkname, elf_bytes_or_None) for the data tar."""
    p = subprocess.Popen(["dpkg-deb", "--fsys-tarfile", path], stdout=subprocess.PIPE)
    with tarfile.open(fileobj=p.stdout, mode="r|") as tf:
        for m in tf:
            kind = "d" if m.isdir() else "l" if m.issym() else "h" if m.islnk() else "f" if m.isfile() else "?"
            body = None
            if kind == "f" and want_elf and is_elf_candidate(m.name):
                fh = tf.extractfile(m)
                if fh.read(4) == b"\x7fELF":
                    body = b"\x7fELF" + fh.read()
            yield kind, m.size, m.name, m.linkname, body
    p.wait()


def cmd_inventory(a):
    ctl_rows, file_rows, elf_rows, sym_rows = [], [], [], []
    for deb in a.debs:
        ctl, members, texts = ctrl_inventory(deb)
        pkg = ctl["Package"]
        for k in CTRL_FIELDS:
            if k in ctl:
                ctl_rows.append((pkg, k, " ".join(ctl[k].split())))
        for name, size in members:
            ctl_rows.append((pkg, "member:" + name, str(size)))
        for name, text in sorted(texts.items()):
            if name == "control":
                continue
            for line in text.splitlines():
                if line.strip() and not line.startswith("#"):
                    ctl_rows.append((pkg, name, " ".join(line.split())))
        if pkg == "glibc-source":  # the reference's source: listed by name only, never read
            out = subprocess.run(["dpkg-deb", "-c", deb], capture_output=True, text=True).stdout
            for line in out.splitlines():
                f = line.split(None, 5)
                file_rows.append((pkg, f[0][0].replace("-", "f"), f[2], f[5].split(" -> ")[0], ""))
            continue
        want = not pkg.endswith("-dbg")
        with tempfile.TemporaryDirectory() as td:
            for kind, size, name, link, body in data_members(deb, want):
                file_rows.append((pkg, kind, str(size if kind == "f" else 0), name, link or ""))
                if body is None:
                    continue
                tmp = f"{td}/x"
                with open(tmp, "wb") as f:
                    f.write(body)
                soname, needed, defined, _ = elf_dynamic(tmp)
                x64 = is_x86_64(body)
                elf_rows.append((pkg, name, soname if x64 else soname + " (" + elf_class(body) + ")",
                                 ",".join(needed)))
                if soname and x64:  # multilib (i386, x32) objects reuse the sonames
                    for sym, ver, at, typ in defined:
                        sym_rows.append((soname, sym, ver, at, typ))
    for suffix, rows in (("control", ctl_rows), ("files", file_rows), ("elf", elf_rows),
                         ("dynsyms", sorted(set(sym_rows)))):
        with open(f"{a.prefix}-{suffix}.tsv", "w") as f:
            for r in sorted(rows):
                f.write("\t".join(r) + "\n")
        print(f"{a.prefix}-{suffix}.tsv: {len(rows)} lines")


# ---------------------------------------------------------------- compare
def read_tsv(path):
    rows = []
    with open(path) as f:
        for i, line in enumerate(f, 1):
            rows.append((i, line.rstrip("\n").split("\t")))
    return rows


def ver_key(v):
    m = re.match(r"GLIBC_(\d+)\.(\d+)(?:\.(\d+))?$", v)
    return tuple(int(x or 0) for x in m.groups()) if m else None


def norm_path(p):
    """Group per-build names: build-id debug files, versioned names."""
    p = re.sub(r"/\.build-id/[0-9a-f]{2}/[0-9a-f]+\.debug$", "/.build-id/*/*.debug", p)
    return p


def cmd_compare(a):
    D = a.dir
    out = []
    # (d) packages
    rc = read_tsv(f"{D}/ref-control.tsv")
    oc = read_tsv(f"{D}/ours-control.tsv")
    rpk = {r[0]: i for i, r in rc if r[1] == "Package"}
    opk = {r[0] for _, r in oc if r[1] == "Package"}
    arch = {r[0]: r[2] for _, r in rc if r[1] == "Architecture"}
    for p, i in sorted(rpk.items()):
        if p not in opk:
            out.append(("d", p, p, f"ref-control.tsv:{i}", "Architecture " + arch.get(p, "?")))
    # (e) control fields and metadata members
    rmap, omap = defaultdict(lambda: defaultdict(list)), defaultdict(lambda: defaultdict(list))
    for i, r in rc:
        rmap[r[0]][r[1]].append((r[2], i))
    for i, r in oc:
        omap[r[0]][r[1]].append((r[2], i))
    for p in sorted(rpk):
        if p not in opk:
            continue
        keys = sorted(set(rmap[p]) | set(omap[p]))
        for k in keys:
            if k in ("Version", "Source", "Installed-Size") or k.startswith("member:") and k[7:] not in (
                    "conffiles", "triggers", "shlibs", "symbols", "md5sums", "postinst", "preinst",
                    "postrm", "prerm", "config", "templates"):
                continue
            rv = [v for v, _ in rmap[p].get(k, [])]
            ov = [v for v, _ in omap[p].get(k, [])]
            if k.startswith("member:"):
                if bool(rv) != bool(ov):
                    line = (rmap[p].get(k) or omap[p].get(k))[0][1]
                    src = "ref" if rv else "ours"
                    out.append(("e", p, k, f"{src}-control.tsv:{line}",
                                "only in " + ("reference" if rv else "ours")))
                continue
            if k in REL_FIELDS:
                rs = {x.strip() for v in rv for x in v.split(",") if x.strip()}
                os_ = {x.strip() for v in ov for x in v.split(",") if x.strip()}
                for x in sorted(rs - os_):
                    out.append(("e", p, f"{k}: {x}", f"ref-control.tsv:{rmap[p][k][0][1]}", "reference only"))
                for x in sorted(os_ - rs):
                    out.append(("e", p, f"{k}: {x}", f"ours-control.tsv:{omap[p][k][0][1]}", "ours only"))
            elif sorted(rv) != sorted(ov):
                for v, i in rmap[p].get(k, []):
                    if v not in ov:
                        out.append(("e", p, f"{k}: {v}", f"ref-control.tsv:{i}", "reference only"))
                for v, i in omap[p].get(k, []):
                    if v not in rv:
                        out.append(("e", p, f"{k}: {v}", f"ours-control.tsv:{i}", "ours only"))
    # (a) files, by package; directories and links included
    rf = read_tsv(f"{D}/ref-files.tsv")
    of = read_tsv(f"{D}/ours-files.tsv")
    have_any = {norm_path(r[3]) for _, r in of}
    seen = set()
    for i, r in rf:
        if r[0] not in opk and r[0] in rpk:
            continue  # whole package missing: class d
        n = norm_path(r[3])
        if n in have_any or (r[0], n) in seen:
            continue
        seen.add((r[0], n))
        out.append(("a", r[0], n, f"ref-files.tsv:{i}", {"d": "directory", "l": "link -> " + r[4],
                                                           "h": "hard link"}.get(r[1], "file " + r[2])))
    # (b) sonames
    re_ = read_tsv(f"{D}/ref-elf.tsv")
    oe = read_tsv(f"{D}/ours-elf.tsv")
    osn = {r[2] for _, r in oe if len(r) > 2 and r[2]}
    for i, r in re_:
        if len(r) > 2 and r[2] and r[2] not in osn:
            out.append(("b", r[0], r[2], f"ref-elf.tsv:{i}", r[1]))
    # (c) (symbol, version) pairs per soname present in both
    rs = read_tsv(f"{D}/ref-dynsyms.tsv")
    os2 = {(r[0], r[1], r[2]) for _, r in read_tsv(f"{D}/ours-dynsyms.tsv")}
    odef = {(r[0], r[1], r[2]): r[3] for _, r in read_tsv(f"{D}/ours-dynsyms.tsv")}
    nver = Counter((r[0], r[1]) for _, r in rs if r[2])
    for i, r in rs:
        soname, sym, ver, at = r[0], r[1], r[2], r[3]
        if soname not in osn or not ver or sym == ver:
            continue  # version-node definitions themselves
        if (soname, sym, ver) in os2:
            if odef[(soname, sym, ver)] != at:
                out.append(("c", soname, f"{sym}{at}{ver}", f"ref-dynsyms.tsv:{i}",
                            f"default-ness differs (ours {odef[(soname, sym, ver)] or 'unversioned'})"))
            continue
        k = ver_key(ver)
        why = ("GLIBC_PRIVATE" if ver == "GLIBC_PRIVATE" else
               "after 2.39" if k and k > (2, 39) else
               "multi-version (SYS-COMPAT-VERSIONS)" if nver[(soname, sym)] > 1 else "missing")
        out.append(("c", soname, f"{sym}{at}{ver}", f"ref-dynsyms.tsv:{i}", why))
    with open(f"{D}/gaps.tsv", "w") as f:
        f.write("# class\tpackage-or-soname\titem\tinventory line\tdetail (parity.py compare)\n")
        for r in out:
            f.write("\t".join(r) + "\n")
    print(Counter((r[0], r[4] if r[0] in "c" else "") for r in out))


# ---------------------------------------------------------------- rank
def rel_names(v):
    out = set()
    for alt in re.split(r"[,|]", v or ""):
        m = re.match(r"\s*([a-z0-9][a-z0-9+.-]*)", alt)
        if m:
            out.add(m.group(1).split(":")[0])
    return out


def scan_one(st, td):
    """NEEDED sonames and GLIBC-family imports (lib, sym, ver) of one package's ELF files."""
    deb = fetch_deb(st, td)
    sonames, imports = set(), set()
    try:
        for kind, size, name, link, body in data_members(deb, True):
            if body is None or not is_x86_64(body):
                continue
            fd, tmp = tempfile.mkstemp(dir=td)
            with os.fdopen(fd, "wb") as f:
                f.write(body)
            _, needed, _, undef = elf_dynamic(tmp)
            os.unlink(tmp)
            sonames.update(needed)
            imports.update(u for u in undef if u[2].startswith("GLIBC_"))
    finally:
        os.unlink(deb)
    return sonames, imports


def cmd_rank(a):
    pk = load_packages(a.wd, a.suite)
    src = load_sources(a.wd, a.suite)
    D = a.dir
    ref_pkgs = sorted({r[0] for _, r in read_tsv(f"{D}/ref-control.tsv")})
    # exact relation counts: binary packages (other sources) naming each reference package
    rel = defaultdict(lambda: defaultdict(set))
    for n, st in pk.items():
        if src_of(st) == "glibc":
            continue
        for k in ("Pre-Depends", "Depends", "Recommends", "Suggests"):
            for x in rel_names(st.get(k)) & set(ref_pkgs):
                rel[x][k].add(n)
    for n, st in src.items():
        if n == "glibc":
            continue
        for k in ("Build-Depends", "Build-Depends-Arch", "Build-Depends-Indep"):
            for x in rel_names(st.get(k)) & set(ref_pkgs):
                rel[x]["Build-Depends*"].add(n)
    with open(f"{D}/consumers-packages.tsv", "w") as f:
        f.write("# reference package\trelation\tcount (binary packages, or source packages for Build-Depends*)\n")
        for x in ref_pkgs:
            for k in ("Pre-Depends", "Depends", "Recommends", "Suggests", "Build-Depends*"):
                f.write(f"{x}\t{k}\t{len(rel[x][k])}\n")
    # ELF sample: amd64 packages from other sources that depend on libc6
    pop = sorted(n for n, st in pk.items() if st.get("Architecture") == "amd64" and src_of(st) != "glibc"
                 and "libc6" in rel_names(st.get("Depends", "") + "," + st.get("Pre-Depends", "")))
    rnd = random.Random(a.seed)
    sample = pop[:]
    rnd.shuffle(sample)
    sample = sample[:a.sample]
    # the gap items whose users are named: missing sonames (x86-64) and missing (symbol, version)
    gap_son, gap_pair = set(), set()
    for _, r in read_tsv(f"{D}/gaps.tsv"):
        if r[0] == "b" and "(" not in r[2]:
            gap_son.add(r[2])
        elif r[0] == "c":
            m = re.match(r"(.*?)(@@?)(.*)$", r[2])
            gap_pair.add((r[1], m.group(1), m.group(3)))
    users = defaultdict(set)
    son, imp, done, failed = Counter(), Counter(), 0, []
    td = tempfile.mkdtemp(dir=a.tmp)
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
        futs = {ex.submit(scan_one, pk[n], td): n for n in sample}
        for fu in concurrent.futures.as_completed(futs):
            try:
                s, i = fu.result()
            except Exception as e:  # recorded, never hidden
                failed.append((futs[fu], str(e)[:120]))
                continue
            done += 1
            son.update(s)
            imp.update(i)
            for x in s & gap_son:
                users[("soname", x)].add(futs[fu])
            for x in i & gap_pair:
                users[("symbol", f"{x[0]} {x[1]}@{x[2]}")].add(futs[fu])
            if done % 250 == 0:
                print(f"rank: {done}/{len(sample)} scanned", flush=True)
    shutil.rmtree(td, ignore_errors=True)
    meta = {"population": len(pop), "sample": len(sample), "scanned": done, "seed": a.seed,
            "failed": failed}
    with open(f"{D}/consumers-meta.json", "w") as f:
        json.dump(meta, f, indent=1, sort_keys=True)
    with open(f"{D}/consumers-sonames.tsv", "w") as f:
        f.write(f"# NEEDED soname\tpackages of {done} scanned (population {len(pop)})\n")
        for s, c in sorted(son.items(), key=lambda x: (-x[1], x[0])):
            f.write(f"{s}\t{c}\n")
    with open(f"{D}/consumers-imports.tsv", "w") as f:
        f.write(f"# library\tsymbol\tversion\tpackages of {done} scanned (population {len(pop)})\n")
        for (l, s, v), c in sorted(imp.items(), key=lambda x: (-x[1], x[0])):
            f.write(f"{l}\t{s}\t{v}\t{c}\n")
    # one level of reverse dependencies (Depends, Pre-Depends; every architecture and source)
    rdep = defaultdict(set)
    for n, st in pk.items():
        for x in rel_names(st.get("Depends", "") + "," + st.get("Pre-Depends", "")):
            rdep[x].add(n)
    with open(f"{D}/consumers-users.tsv", "w") as f:
        f.write(f"# kind\tgap item\tusers of {done} scanned\tpackages that depend on a user\tusers (first 25)\n")
        for (kind, item), us in sorted(users.items(), key=lambda x: (-len(x[1]), x[0])):
            rd = set().union(*(rdep[u] for u in us)) - us
            f.write(f"{kind}\t{item}\t{len(us)}\t{len(rd)}\t{','.join(sorted(us)[:25])}\n")
    print(json.dumps({k: v for k, v in meta.items() if k != "failed"}), len(failed), "failed")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = ap.add_subparsers(dest="cmd", required=True)
    p = sp.add_parser("index"); p.add_argument("wd"); p.add_argument("--suite", default="trixie")
    p.add_argument("--keyring")
    p = sp.add_parser("fetch-ref"); p.add_argument("wd"); p.add_argument("--suite", default="trixie")
    p = sp.add_parser("inventory"); p.add_argument("prefix"); p.add_argument("debs", nargs="+")
    p = sp.add_parser("compare"); p.add_argument("dir")
    p = sp.add_parser("rank"); p.add_argument("wd"); p.add_argument("dir")
    p.add_argument("--suite", default="trixie"); p.add_argument("--sample", type=int, default=3000)
    p.add_argument("--seed", type=int, default=20261116); p.add_argument("--jobs", type=int, default=12)
    p.add_argument("--tmp", default="/tmp")
    a = ap.parse_args()
    {"index": cmd_index, "fetch-ref": cmd_fetch_ref, "inventory": cmd_inventory,
     "compare": cmd_compare, "rank": cmd_rank}[a.cmd](a)


if __name__ == "__main__":
    main()
