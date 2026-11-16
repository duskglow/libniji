#!/usr/bin/env python3
"""tools/deb/symbols.py: libc6's dpkg `symbols` and `shlibs` control members (plan/system-libc.md
section 3.2, owner ruling R2; tools/deb/system.sh runs it while it builds libc6).

    symbols.py files --versions spec/abi/versions.tsv --native spec/abi/native.tsv
                     --gaps docs/spec-symbol-gaps.tsv --abi 2.39
                     --version <deb version> --libdir DIR --symbols OUT --shlibs OUT --provides OUT

`files` reads every shared library in DIR (the staged libc6's /usr/lib/x86_64-linux-gnu) and
writes, from the specification and the built objects together:

* symbols (deb-symbols(5)): one `<soname> libc6 #MINVER#` block per library. Every exported
  `name@GLIBC_2.x` gets the minimum version `2.x`, the ABI level of the node that defines it, so a
  package built against libniji depends on the newest node it uses, never on the build's own
  version. A node above the packages' ABI level (`--abi`; GLIBC_2.41 while the packages say 2.39,
  ruling R3) gets the build's upstream version instead: `libc6 (>= 2.41)` could not be met by
  libniji's own packages, while every libniji build from this one on has the symbol. Each symbol of
  a libniji-only node `NIJI_<KIND>_<n>` (the kinds in `KINDS`: NIJI_NATIVE_<n>, libniji's own
  interface; NIJI_OWNER_<n>, the opted-in registrations of D-27.29) uses the alternative
  template `| <package> (>= <n>)` on the kind's virtual package (niji-native-abi,
  niji-owner-abi), which only libniji's libc6 provides, so a program that needs such a node
  never installs against another C library. The private node NIJI_PRIVATE (the libraries'
  interface to each other) is one `(symver)` pattern at the build's upstream version. There are no
  GLIBC_PRIVATE lines (R14-12): a library that exports that node is an error. Any other version
  node stops the build with an error naming it: a node without a dependency rule must not ship.
* shlibs (deb-shlibs(5)): `<name> <soversion> libc6 (>= <abi>)` per library, the fallback for
  tools that do not read symbols files.
* provides (`--provides`): libc6's `Provides:` value, each kind's virtual package at the newest
  level the libraries define (`niji-native-abi (= 1), niji-owner-abi (= 1)`).

The library's exports must equal the specification's rows for it (versions.tsv without
GLIBC_PRIVATE, plus native.tsv's functions for libc.so.6): any difference stops the build, so the
file never describes symbols the library lacks or misses ones it has. The one exception is a
default-binding row docs/spec-symbol-gaps.tsv lists as missing in the shared object (`--gaps`): the
build does not have it yet, so the file leaves it out until it does. A NIJI_OWNER node has no
table rows: the product build checks its exact names (`cargo xtask so`, D-27.29), and here each of
its exports must be a non-default version in libc.so.6, so a default link never binds it.
"""
import argparse
import os
import re
import subprocess
import sys

PRIVATE = "GLIBC_PRIVATE"
NIJI_PRIVATE = "NIJI_PRIVATE"
PACKAGE = "libc6"
# The libniji-only version nodes NIJI_<KIND>_<n>, one rule for every kind (a program that uses
# one runs only on libniji: its symbols name `| <package> (>= <n>)`, and libc6 Provides `<package>
# (= <newest n>)`). Per kind: the virtual package, and where the node's exports are specified
# ("native.tsv": exactly the functions of --native; "nondefault-libc": the product build checks
# the names, here each must be a non-default version in libc.so.6). A new kind is one row here
# (and its virtual package in plan/system-libc.md section 3.2).
KINDS = {
    "NATIVE": {"package": "niji-native-abi", "spec": "native.tsv"},
    "OWNER": {"package": "niji-owner-abi", "spec": "nondefault-libc"},
}
NIJI_NODE = re.compile(r"NIJI_([A-Z]+)_([0-9]+)")


def die(msg):
    print(f"symbols.py: {msg}", file=sys.stderr)
    sys.exit(1)


def read_tsv(path):
    with open(path, encoding="utf-8") as f:
        lines = f.read().splitlines()
    head = lines[0].split("\t")
    return [dict(zip(head, line.split("\t"))) for line in lines[1:] if line]


def niji_node(node):
    """(kind, level) of a node NIJI_<KIND>_<n> whose kind KINDS lists, else None."""
    m = NIJI_NODE.fullmatch(node)
    if not m or m.group(1) not in KINDS:
        return None
    return m.group(1), int(m.group(2))


def check_node(so, node):
    """Stop on a version node no rule covers (what the file would say about it is unknown)."""
    if node in (PRIVATE, NIJI_PRIVATE) or niji_node(node) is not None:
        return
    if node.startswith("GLIBC_") and re.fullmatch(r"\d+(\.\d+)*", node[len("GLIBC_"):]):
        return
    die(f"{so}: version node {node} has no dependency rule: a libniji-only node NIJI_<KIND>_<n> "
        f"needs its kind in KINDS (tools/deb/symbols.py; plan/system-libc.md section 3.2), any other "
        f"node a rule beside GLIBC_<x> and {NIJI_PRIVATE} in min_version")


def version_key(v):
    """The numeric parts of an ABI level (2.2.5 -> (2, 2, 5)); compared component by component."""
    if not re.fullmatch(r"\d+(\.\d+)*", v):
        die(f"ABI level '{v}' is not dotted numbers")
    return tuple(int(x) for x in v.split("."))


def run(*argv):
    return subprocess.run(argv, check=True, capture_output=True, text=True).stdout


def soname(path):
    for line in run("readelf", "-dW", path).splitlines():
        m = re.search(r"\(SONAME\)\s.*\[(.*)\]$", line)
        if m:
            return m.group(1)
    return None


def exports(path):
    """{(name, node)} of the defined, visible, versioned symbols, the version-node names, and the
    {(name, node)} that are the name's default version (`name@@node`)."""
    syms, nodes, unversioned, defaults = set(), set(), [], set()
    # The version definitions (the BASE entry is the file's own name). readelf prints a node's own
    # ABS symbol without the @NODE suffix.
    defined = set()
    for line in run("readelf", "-VW", path).splitlines():
        m = re.search(r"Flags: (\S+)\s+Index: \d+\s+Cnt: \d+\s+Name: (\S+)", line)
        if m and m.group(1) != "BASE":
            defined.add(m.group(2))
    for line in run("readelf", "--dyn-syms", "-W", path).splitlines():
        f = line.split()
        if len(f) < 8 or not f[0].rstrip(":").isdigit():
            continue
        typ, bind, vis, ndx, name = f[3], f[4], f[5], f[6], f[7]
        if ndx == "UND" or bind == "LOCAL" or vis in ("HIDDEN", "INTERNAL") or typ in ("FILE", "SECTION"):
            continue
        if "@" not in name:
            if ndx == "ABS" and name in defined:
                nodes.add(name)
            else:
                unversioned.append(name)
            continue
        sym, node = re.split(r"@@?", name, maxsplit=1)
        if sym == node and ndx == "ABS":
            nodes.add(node)
        else:
            syms.add((sym, node))
            if "@@" in name:
                defaults.add((sym, node))
    if nodes != defined:
        die(f"{path}: version nodes {sorted(defined)}, node symbols {sorted(nodes)}")
    if unversioned:
        die(f"{path}: unversioned exports: {' '.join(sorted(unversioned)[:5])}")
    return syms, nodes, defaults


def spec_exports(versions, natives, so):
    """(required, allowed): the default-binding rows a library must export, and every row it may
    export (compat-binding rows serve only programs linked long ago; the build may leave them out)."""
    rows = [r for r in versions if r["library"] == so and r["version"] != PRIVATE]
    want = {(r["symbol"], r["version"]) for r in rows if r["binding"] == "default"}
    if so == "libc.so.6":
        want |= {(r["name"], r["node"]) for r in natives if r["kind"] == "fn"}
    return want, want | {(r["symbol"], r["version"]) for r in rows}


def nondefault_exports(so, got, defaults):
    """(exports at the "nondefault-libc" kinds' nodes, problems): such a node's exports are the
    product build's to name (D-27.29); each must be a non-default version, in libc.so.6 only."""
    kinds = {k for k, v in KINDS.items() if v["spec"] == "nondefault-libc"}
    mine = {(s, n) for s, n in got if (niji_node(n) or ("", 0))[0] in kinds}
    problems = [f"{so}: exports {s}@{n}, a node only libc.so.6 may define" for s, n in sorted(mine) if so != "libc.so.6"]
    problems += [f"{so}: exports {s}@@{n} as the default version (a default link would bind it)" for s, n in sorted(mine & defaults)]
    return mine, problems


def min_version(node, abi, upstream):
    if node.startswith("GLIBC_"):
        level = node[len("GLIBC_"):]
        return level if version_key(level) <= version_key(abi) else upstream
    if niji_node(node) is not None:
        return abi
    die(f"no minimum version rule for node {node}")
    return None


def generate(args):
    versions = read_tsv(args.versions)
    natives = read_tsv(args.native)
    # The specification's exports the build does not have yet (docs/spec-symbol-gaps.tsv, rows
    # missing in the shared object): left out of the file, which describes the built library.
    gaps = {}
    for r in read_tsv(args.gaps):
        if "so" in r["missing_in"].split("+"):
            for v in r["missing_versions"].split(","):
                gaps.setdefault(r["library"], set()).add((r["symbol"], v))
    upstream = args.version.rsplit("-", 1)[0]
    libs = {}
    for f in sorted(os.listdir(args.libdir)):
        p = os.path.join(args.libdir, f)
        if os.path.islink(p) or not os.path.isfile(p) or ".so" not in f:
            continue
        sn = soname(p)
        if sn is None:
            die(f"{p}: no SONAME")
        libs[sn] = p
    if "libc.so.6" not in libs:
        die(f"{args.libdir}: no libc.so.6")
    sym_out, shl_out, errors, provides = [], [], [], {}
    for so in sorted(libs):
        got, nodes, defaults = exports(libs[so])
        for n in sorted(nodes | {n for _, n in got}):
            check_node(so, n)
        if PRIVATE in nodes or any(n == PRIVATE for _, n in got):
            errors.append(f"{so} exports the {PRIVATE} node (R14-12)")
        public = {(s, n) for s, n in got if n != NIJI_PRIVATE}
        build_named, problems = nondefault_exports(so, public, defaults)
        errors += problems
        want, allowed = spec_exports(versions, natives, so)
        if not allowed:
            errors.append(f"{so}: no rows in {args.versions}")
        for s, n in sorted(public - allowed - build_named)[:10]:
            errors.append(f"{so}: exports {s}@{n}, which the specification does not list")
        for s, n in sorted(want - public - gaps.get(so, set()))[:10]:
            errors.append(f"{so}: does not export {s}@{n} (specification; not a gap in {args.gaps})")
        # One alternative template per (virtual package, level) the library's symbols use.
        kind_levels = {niji_node(n) for _, n in public} - {None}
        levels = sorted({(KINDS[kind]["package"], lvl) for kind, lvl in kind_levels})
        template = {pl: i + 1 for i, pl in enumerate(levels)}
        sym_out.append(f"{so} {PACKAGE} #MINVER#")
        for pkg, lvl in levels:
            sym_out.append(f"| {pkg} (>= {lvl})")
            provides[pkg] = max(provides.get(pkg, 0), lvl)
        sym_out.append(f"* Build-Depends-Package: {PACKAGE}-dev")
        if NIJI_PRIVATE in nodes:
            sym_out.append(f" (symver){NIJI_PRIVATE} {upstream}")
        lines = [(n, n) for n in nodes if n != NIJI_PRIVATE] + sorted(public)
        for s, n in sorted(lines, key=lambda x: (x[0], x[1])):
            if n == PRIVATE:
                continue
            line = f" {s}@{n} {min_version(n, args.abi, upstream)}"
            kl = niji_node(n)
            if kl is not None:
                line += f" {template[(KINDS[kl[0]]['package'], kl[1])]}"
            sym_out.append(line)
        m = re.fullmatch(r"(.+)\.so\.(\d+)", so)
        if not m:
            errors.append(f"{so}: not <name>.so.<n>")
            continue
        shl_out.append(f"{m.group(1)} {m.group(2)} {PACKAGE} (>= {args.abi})")
    if errors:
        die("the built libraries and the specification differ:\n  " + "\n  ".join(errors))
    with open(args.symbols, "w", encoding="utf-8") as f:
        f.write("\n".join(sym_out) + "\n")
    with open(args.shlibs, "w", encoding="utf-8") as f:
        f.write("\n".join(shl_out) + "\n")
    with open(args.provides, "w", encoding="utf-8") as f:
        f.write(", ".join(f"{pkg} (= {lvl})" for pkg, lvl in sorted(provides.items())) + "\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("files")
    for opt in ("versions", "native", "gaps", "abi", "version", "libdir", "symbols", "shlibs", "provides"):
        g.add_argument(f"--{opt}", required=True)
    args = ap.parse_args()
    version_key(args.abi)
    generate(args)


if __name__ == "__main__":
    main()
