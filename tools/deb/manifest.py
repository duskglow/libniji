#!/usr/bin/env python3
"""tools/deb/manifest.py: the Debian packages' contents as data (PKG-MANIFEST). No spec clause:
release packaging. The data: tools/deb/manifest.tsv (one row per shipped file) and
tools/deb/control.tsv (the control fields), each documented in its header.

  manifest.py lint [--products DIR]
      The tables are well formed and sorted; every text source and description exists; with
      --products, every product a file row names is in DIR (the build's products, target/niji).
  manifest.py rows FLAVOUR
      The flavour's rows, resolved (both rows mapped to the coexist packages and sysroot):
      package, path, kind, source, mode, strip, tab-separated (build.sh installs them).
  manifest.py packages FLAVOUR | debug-package FLAVOUR | symbols-package FLAVOUR
      The flavour's packages; the one holding the detached debugging information; the one with
      a symbols file (each empty when there is none).
  manifest.py control FLAVOUR PACKAGE --version V --abi A --maint M --installed-size N [--provides P]
      The package's DEBIAN/control.
  manifest.py member FLAVOUR PACKAGE conffiles|triggers
      The member's text (empty: the package has no such member).
  manifest.py check FLAVOUR DEBDIR
      The .debs in DEBDIR against the tables: exactly the flavour's packages; in each, exactly the
      rows' paths and their directories (root:root, the rows' types, modes, link targets, text
      contents), every file row stripped as its strip column says, the control members and their
      contents, the control text, Installed-Size and md5sums. Prints one "check: FAIL ..." line per
      problem and exits 1 when there is any.
"""
import gzip
import hashlib
import io
import os
import re
import subprocess
import sys
import tarfile
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
FLAVOURS = ("both", "coexist", "system")
KINDS = ("file", "text", "gzip", "link", "changelog", "dir")
STRIPS = ("none", "unneeded", "debug", "archive")
SYSROOT = "usr/lib/niji/"
# A both row's package in the coexist flavour (its path goes under SYSROOT).
COEXIST = {"libc6": "niji6", "libc6-dev": "niji6-dev"}
# Control fields after the fixed ones, in the order the build writes them (deb-control(5)).
FIXED = ("Package", "Source", "Version", "Architecture", "Maintainer", "Installed-Size")
ORDER = ("Essential", "Multi-Arch", "Pre-Depends", "Depends", "Recommends", "Suggests", "Provides",
         "Breaks", "Conflicts", "Replaces", "Section", "Priority")
MEMBERS = ("triggers", "symbols", "debug")
REQUIRED = ("Architecture", "Section", "Priority")


def die(msg):
    sys.stderr.write("manifest.py: %s\n" % msg)
    sys.exit(2)


def table(name, ncol):
    """The data rows of tools/deb/NAME: (line number, fields)."""
    rows = []
    with open(os.path.join(HERE, name), encoding="utf-8") as f:
        for n, line in enumerate(f, 1):
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            rows.append((n, line.split("\t")))
    return rows


def lint_errors(products=None):
    errs = []
    man = table("manifest.tsv", 8)
    ctl = table("control.tsv", 3)
    for name, rows, ncol in (("manifest.tsv", man, 8), ("control.tsv", ctl, 3)):
        prev = None
        for n, f in rows:
            line = "\t".join(f)
            if len(f) != ncol or any(x == "" for x in f):
                errs.append("%s:%d: %d non-empty tab-separated fields expected, %d given" % (name, n, ncol, len(f)))
            if prev is not None and line.encode() <= prev.encode():
                errs.append("%s:%d: not sorted (LC_ALL=C sort) or a duplicate: %s" % (name, n, line[:80]))
            prev = line
    seen = {}
    for n, f in man:
        if len(f) != 8:
            continue
        fl, pkg, path, kind, src, mode, strip, _note = f
        where = "manifest.tsv:%d" % n
        if fl not in FLAVOURS:
            errs.append("%s: flavour %s (%s)" % (where, fl, ", ".join(FLAVOURS)))
        if fl == "both" and pkg not in COEXIST:
            errs.append("%s: a both row's package is one of %s, not %s" % (where, ", ".join(COEXIST), pkg))
        if not re.fullmatch(r"[a-z0-9][a-z0-9+.-]+", pkg):
            errs.append("%s: package name %s" % (where, pkg))
        if path.startswith("/") or path.endswith("/") or ".." in path.split("/") or "//" in path or path.startswith("./"):
            errs.append("%s: path %s is not a relative path without . or .. parts" % (where, path))
        if path.startswith("DEBIAN/") and (path.count("/") != 1 or kind != "text"):
            errs.append("%s: a maintainer script is DEBIAN/<name> of kind text" % where)
        if kind not in KINDS:
            errs.append("%s: kind %s (%s)" % (where, kind, ", ".join(KINDS)))
        if strip not in STRIPS:
            errs.append("%s: strip %s (%s)" % (where, strip, ", ".join(STRIPS)))
        if kind != "file" and strip != "none":
            errs.append("%s: strip %s on a %s row (only a file is stripped)" % (where, strip, kind))
        if kind == "link":
            if mode != "-":
                errs.append("%s: a link's mode is -, not %s" % (where, mode))
        elif not re.fullmatch(r"0[0-7]{3}", mode):
            errs.append("%s: mode %s is not four octal digits" % (where, mode))
        if kind == "file":
            if "/" in src or src == "-":
                errs.append("%s: a file row's source is a product name, not %s" % (where, src))
            elif products is not None and not os.path.isfile(os.path.join(products, src)):
                errs.append("%s: product %s is not in %s (cargo xtask so writes no such file)" % (where, src, products))
        elif kind in ("text", "gzip"):
            if not os.path.isfile(os.path.join(HERE, src)) or ".." in src.split("/"):
                errs.append("%s: %s source %s is not a file under tools/deb/" % (where, kind, src))
        elif kind in ("changelog", "dir") and src != "-":
            errs.append("%s: a %s row's source is -, not %s" % (where, kind, src))
        for flav in (("coexist", "system") if fl == "both" else (fl,)):
            if flav not in ("coexist", "system"):
                continue
            p, q = resolve(flav, fl, pkg, path)
            if (flav, p, q) in seen:
                errs.append("%s: %s %s %s again (first at line %d)" % (where, flav, p, q, seen[(flav, p, q)]))
            seen[(flav, p, q)] = n
    for (flav, p, q), n in seen.items():
        parent = q
        while "/" in parent:
            parent = parent.rsplit("/", 1)[0]
            if (flav, p, parent) in seen:
                errs.append("manifest.tsv:%d: %s is a row and a directory of %s" % (seen[(flav, p, parent)], parent, q))
    pkgs = {p for (_f, p, _q) in seen}
    cpkgs = set()
    for n, f in ctl:
        if len(f) != 3:
            continue
        pkg, field, _value = f
        cpkgs.add(pkg)
        if field not in ORDER + MEMBERS and field != "Architecture":
            errs.append("control.tsv:%d: field %s (Architecture, %s; members %s)" % (n, field, ", ".join(ORDER), ", ".join(MEMBERS)))
        if field == "symbols" and not os.path.isfile(os.path.join(HERE, "..", "..", f[2])):
            errs.append("control.tsv:%d: symbols generator %s does not exist" % (n, f[2]))
    for p in sorted(pkgs | cpkgs):
        if p not in pkgs:
            errs.append("control.tsv: package %s has no manifest row" % p)
            continue
        if p not in cpkgs:
            errs.append("control.tsv: package %s (manifest) has no control fields" % p)
            continue
        have = {f[1] for _n, f in ctl if f[0] == p}
        for r in REQUIRED:
            if r not in have:
                errs.append("control.tsv: package %s has no %s" % (p, r))
        if not os.path.isfile(os.path.join(HERE, "description", p)):
            errs.append("tools/deb/description/%s is missing" % p)
    return errs


def resolve(flavour, fl, pkg, path):
    if fl == "both" and flavour == "coexist":
        return COEXIST[pkg], SYSROOT + path
    return pkg, path


def rows(flavour):
    out = []
    for _n, f in table("manifest.tsv", 8):
        fl, pkg, path, kind, src, mode, strip, _note = f
        if fl == flavour or fl == "both":
            p, q = resolve(flavour, fl, pkg, path)
            out.append((p, q, kind, src, mode, strip))
    return out


def packages(flavour):
    return sorted({r[0] for r in rows(flavour)})


def fields(pkg):
    return [(f[1], f[2]) for _n, f in table("control.tsv", 3) if f[0] == pkg]


def member_package(flavour, member):
    pk = [p for p in packages(flavour) if any(k == member for k, _v in fields(p))]
    if len(pk) > 1:
        die("%s: more than one package has a %s row: %s" % (flavour, member, " ".join(pk)))
    return pk[0] if pk else ""


def expand(text, subst):
    for k in sorted(subst, key=len, reverse=True):
        text = text.replace("$" + k, subst[k])
    if re.search(r"\$[a-z]", text):
        die("an unknown placeholder in: %s" % text)
    return text


def control(flavour, pkg, version, abi, maint, size, provides):
    # The administrator's programs (usr/sbin) first, then usr/bin, each sorted by name.
    progs = [os.path.basename(q) for (_d, q) in sorted(
        (q.split("/")[1] != "sbin", q) for (p, q, kind, _s, _m, _t) in rows(flavour)
        if p == pkg and kind == "file" and q.split("/")[:2] in (["usr", "bin"], ["usr", "sbin"]))]
    subst = {"version": version, "abi": abi, "provides": provides, "programs": ", ".join(progs)}
    fs = dict(fields(pkg))
    if "Provides" in fs and "$provides" in fs["Provides"] and not provides:
        die("%s: Provides names $provides and none was given" % pkg)
    lines = ["Package: " + pkg, "Source: niji", "Version: " + version, "Architecture: " + fs["Architecture"],
             "Maintainer: " + maint, "Installed-Size: " + str(size)]
    lines += ["%s: %s" % (k, expand(fs[k], subst)) for k in ORDER if k in fs]
    with open(os.path.join(HERE, "description", pkg), encoding="utf-8") as f:
        desc = expand(f.read(), subst)
    return "\n".join(lines) + "\nDescription: " + desc


def member(flavour, pkg, name):
    if name == "conffiles":
        return "".join(sorted("/%s\n" % q for (p, q, _k, _s, _m, _t) in rows(flavour) if p == pkg and q.startswith("etc/")))
    if name == "triggers":
        return "".join(v + "\n" for k, v in fields(pkg) if k == "triggers")
    die("member %s: conffiles or triggers" % name)
    return ""


def sections(data):
    """The section names readelf reports for an ELF file or an archive's members."""
    with tempfile.NamedTemporaryFile() as t:
        t.write(data)
        t.flush()
        out = subprocess.run(["readelf", "-SW", t.name], capture_output=True, text=True, check=False).stdout
    return set(re.findall(r"\]\s+(\.[A-Za-z0-9_.]+)\s", out))


def strip_fault(strip, data):
    secs = sections(data)
    if not secs:
        # An archive without members (libg.a, ...) has no sections to strip.
        return "" if strip == "archive" and data.startswith(b"!<arch>\n") else "readelf reports no sections"
    bad = sorted(s for s in secs if s.startswith(".debug_") or s == ".comment" or (strip != "archive" and s == ".symtab"))
    return "has %s" % " ".join(bad) if bad else ""


def deb_tar(deb, what):
    r = subprocess.run(["dpkg-deb", what, deb], capture_output=True, check=False)
    if r.returncode != 0:
        return None
    return tarfile.open(fileobj=io.BytesIO(r.stdout))


def norm(name):
    name = name[2:] if name.startswith("./") else name
    return name.rstrip("/")


def check(flavour, debdir):
    fails = []
    fail = fails.append
    for e in lint_errors():
        fail("manifest: " + e)
    if fails:
        return fails
    debs = sorted(os.path.join(debdir, d) for d in os.listdir(debdir) if d.endswith(".deb"))
    by_pkg = {}
    for deb in debs:
        p = subprocess.run(["dpkg-deb", "-f", deb, "Package"], capture_output=True, text=True, check=False).stdout.strip()
        by_pkg[p] = deb
    want = packages(flavour)
    if sorted(by_pkg) != want:
        fail("packages are '%s', not the manifest's '%s'" % (" ".join(sorted(by_pkg)), " ".join(want)))
    dbg = member_package(flavour, "debug")
    ndebug = sum(1 for r in rows(flavour) if r[5] == "debug")
    for pkg in want:
        if pkg not in by_pkg:
            continue
        check_package(flavour, pkg, by_pkg[pkg], pkg == dbg, ndebug, fail)
    return fails


def check_package(flavour, pkg, deb, is_dbg, ndebug, fail):
    mine = [r for r in rows(flavour) if r[0] == pkg]
    data = deb_tar(deb, "--fsys-tarfile")
    ctrl = deb_tar(deb, "--ctrl-tarfile")
    if data is None or ctrl is None:
        fail("%s: dpkg-deb cannot read %s" % (pkg, deb))
        return
    # The expected data entries: path -> row (None: a directory a path needs).
    exp = {}
    for r in mine:
        if r[1].startswith("DEBIAN/"):
            continue
        exp[r[1]] = r
        d = r[1]
        while "/" in d:
            d = d.rsplit("/", 1)[0]
            exp.setdefault(d, None)
    members = {}
    size = 0
    files = {}
    ndbg = 0
    for m in data.getmembers():
        name = norm(m.name)
        if name in ("", "."):
            continue
        size += (m.size + 1023) // 1024 if m.isreg() else 1
        if m.uid != 0 or m.gid != 0 or m.uname != "root" or m.gname != "root":
            fail("%s: %s is owned by %s/%s (%d/%d), not root/root" % (pkg, name, m.uname, m.gname, m.uid, m.gid))
        if m.isreg():
            files[name] = data.extractfile(m).read()
        members[name] = m
        if is_dbg and re.fullmatch(r"usr/lib(/debug(/\.build-id(/[0-9a-f]{2}(/[0-9a-f]{2,}\.debug)?)?)?)?", name):
            if name.endswith(".debug"):
                ndbg += 1
                if not m.isreg() or m.mode & 0o7777 != 0o644:
                    fail("%s: %s is not a file of mode 0644" % (pkg, name))
            elif not m.isdir() or m.mode & 0o7777 != 0o755:
                fail("%s: %s is not a directory of mode 0755" % (pkg, name))
            continue
        if name not in exp:
            fail("%s: %s is shipped and the manifest does not list it" % (pkg, name))
            continue
        r = exp[name]
        if r is None or r[2] == "dir":
            if not m.isdir() or m.mode & 0o7777 != 0o755:
                fail("%s: %s is not a directory of mode 0755" % (pkg, name))
            continue
        _p, path, kind, src, mode, strip = r
        if kind == "link":
            if not m.issym() or m.linkname != src:
                fail("%s: %s is not a link to %s (%s)" % (pkg, path, src, m.linkname if m.issym() else "type %s" % m.type.decode()))
            continue
        if not m.isreg():
            fail("%s: %s is not a regular file" % (pkg, path))
            continue
        if m.mode & 0o7777 != int(mode, 8):
            fail("%s: %s has mode %04o, not %s" % (pkg, path, m.mode & 0o7777, mode))
        body = files[name]
        if kind == "text" and body != open(os.path.join(HERE, src), "rb").read():
            fail("%s: %s differs from tools/deb/%s" % (pkg, path, src))
        if kind == "gzip" and gzip.decompress(body) != open(os.path.join(HERE, src), "rb").read():
            fail("%s: %s is not tools/deb/%s compressed" % (pkg, path, src))
        if kind == "changelog" and not gzip.decompress(body).startswith(b"niji ("):
            fail("%s: %s is not a niji changelog" % (pkg, path))
        if kind == "file" and strip != "none":
            why = strip_fault(strip, body)
            if why:
                fail("%s: %s is not stripped (strip %s): %s" % (pkg, path, strip, why))
    for name in sorted(set(exp) - set(members)):
        fail("%s: the manifest lists %s and the package does not ship it" % (pkg, name))
    if is_dbg and ndbg != ndebug:
        fail("%s: %d detached debugging files, the manifest has %d strip debug rows" % (pkg, ndbg, ndebug))
    # Control members.
    cm = {norm(m.name): m for m in ctrl.getmembers() if norm(m.name) not in ("", ".")}
    want = {"control", "md5sums"}
    if member(flavour, pkg, "conffiles"):
        want.add("conffiles")
    if member(flavour, pkg, "triggers"):
        want.add("triggers")
    if any(k == "symbols" for k, _v in fields(pkg)):
        want |= {"symbols", "shlibs"}
    scripts = {r[1].split("/", 1)[1]: r for r in mine if r[1].startswith("DEBIAN/")}
    want |= set(scripts)
    if set(cm) != want:
        fail("%s: control members '%s', not '%s'" % (pkg, " ".join(sorted(cm)), " ".join(sorted(want))))

    def text(name):
        return ctrl.extractfile(cm[name]).read().decode() if name in cm else ""
    for name, r in scripts.items():
        if name in cm:
            if cm[name].mode & 0o7777 != int(r[4], 8):
                fail("%s: DEBIAN/%s has mode %04o, not %s" % (pkg, name, cm[name].mode & 0o7777, r[4]))
            if text(name).encode() != open(os.path.join(HERE, r[3]), "rb").read():
                fail("%s: DEBIAN/%s differs from tools/deb/%s" % (pkg, name, r[3]))
    for name in sorted(set(cm) - set(scripts)):
        if cm[name].mode & 0o7777 != 0o644:
            fail("%s: DEBIAN/%s has mode %04o, not 0644" % (pkg, name, cm[name].mode & 0o7777))
    for name in ("conffiles", "triggers"):
        if name in want and text(name) != member(flavour, pkg, name):
            fail("%s: %s is '%s', not '%s'" % (pkg, name, text(name).strip().replace("\n", " "), member(flavour, pkg, name).strip().replace("\n", " ")))
    got = text("control")
    f = dict(re.findall(r"^([A-Za-z-]+): (.*)$", got, re.M))
    version = f.get("Version", "")
    abi = version.split("+", 1)[0] if "+niji" in version else "2.39"
    expect = control(flavour, pkg, version, abi, f.get("Maintainer", ""), f.get("Installed-Size", ""), f.get("Provides", "") or "-")
    if got != expect:
        g, e = got.splitlines(), expect.splitlines()
        i = next((k for k in range(max(len(g), len(e))) if k >= len(g) or k >= len(e) or g[k] != e[k]), 0)
        fail("%s: control line %d is '%s', not '%s'" % (pkg, i + 1, g[i] if i < len(g) else "(none)", e[i] if i < len(e) else "(none)"))
    if f.get("Installed-Size") != str(size):
        fail("%s: Installed-Size %s, the data is %d KiB" % (pkg, f.get("Installed-Size"), size))
    sums = {}
    for line in text("md5sums").splitlines():
        h, _sp, path = line.partition("  ")
        sums[path] = h
    if sums != {k: hashlib.md5(v).hexdigest() for k, v in files.items()}:
        fail("%s: md5sums does not list exactly the package's files with their sums" % pkg)


def main(argv):
    if not argv:
        die("usage: see the header of tools/deb/manifest.py")
    cmd, args = argv[0], argv[1:]
    if cmd == "lint":
        products = None
        if args[:1] == ["--products"] and len(args) == 2:
            products = args[1]
        elif args:
            die("usage: manifest.py lint [--products DIR]")
        errs = lint_errors(products)
        for e in errs:
            print("manifest: FAIL " + e)
        if errs:
            return 1
        print("manifest: OK (%d rows, %d control rows)" % (len(table("manifest.tsv", 8)), len(table("control.tsv", 3))))
        return 0
    if not args or args[0] not in ("coexist", "system"):
        die("%s: the flavour (coexist or system) first" % cmd)
    flavour = args[0]
    if cmd in ("rows", "packages", "debug-package", "symbols-package", "control", "member") and lint_errors():
        die("the tables are malformed: run tools/deb/manifest.py lint")
    if cmd == "rows":
        for r in rows(flavour):
            print("\t".join(r))
    elif cmd == "packages":
        print(" ".join(packages(flavour)))
    elif cmd == "debug-package":
        print(member_package(flavour, "debug"))
    elif cmd == "symbols-package":
        print(member_package(flavour, "symbols"))
    elif cmd == "control":
        opts = dict(zip(args[2::2], args[3::2]))
        need = ("--version", "--abi", "--maint", "--installed-size")
        if len(args) < 2 or any(k not in opts for k in need):
            die("usage: manifest.py control FLAVOUR PACKAGE --version V --abi A --maint M --installed-size N [--provides P]")
        sys.stdout.write(control(flavour, args[1], opts["--version"], opts["--abi"], opts["--maint"], opts["--installed-size"], opts.get("--provides", "")))
    elif cmd == "member":
        if len(args) != 3:
            die("usage: manifest.py member FLAVOUR PACKAGE conffiles|triggers")
        sys.stdout.write(member(flavour, args[1], args[2]))
    elif cmd == "check":
        if len(args) != 2:
            die("usage: manifest.py check FLAVOUR DEBDIR")
        fails = check(flavour, args[1])
        for f in fails:
            print("check: FAIL " + f)
        return 1 if fails else 0
    else:
        die("unknown command %s" % cmd)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
