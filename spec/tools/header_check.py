#!/usr/bin/env python3
"""Check libniji's generated headers (plan/headers.md section 3; SL-HEADERS-S05): P5 hygiene, the level model,
and P1 identity of the C 7.20 integer types.

    python3 spec/tools/header_check.py [--work DIR] [--include DIR]

Needs gcc and g++ only (any host; no reference headers are read: every unit is compiled with `-nostdinc
-isystem <DIR> -isystem <the compiler's own include>`, DIR = libc-include/ by default, or the tree `cargo xtask
so` staged, target/niji/include). For each configuration of spec/abi/ftm-configs.tsv:

* levels  a unit that includes features.h and `#error`s unless each `__NIJI_<level>` equals its
          headers/levels.tsv cell; it must compile, or fail for a configuration levels.tsv rejects;
* P5      each header of HEADERS compiles alone and included twice, `-Wall -Wextra -Wsystem-headers -Werror`;
* P1      (ruling #6047) every C 7.20.1 type spelled by header_probe.c_integer_names(): C asserts its size and
          alignment, C++ asserts `__is_same` with the typedefs.tsv `cxx` spelling (the spelling enters C++
          linkage names, so `long` against `long long` is ABI).

Prints one FAIL line per failing unit and `header_check: N failed of M`; exit 1 when any failed.
"""
import argparse
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import header_probe as hp  # noqa: E402  (ROOT, c_integer_names)

ROOT = hp.ROOT
HEADERS = ["features.h", "features-time64.h", "stdc-predef.h", "sys/cdefs.h", "sys/types.h", "sys/bitypes.h",
           "stdint.h", "inttypes.h", "limits.h", "endian.h", "byteswap.h", "errno.h", "sys/errno.h", "alloca.h",
           "gnu-versions.h", "gnu/stubs.h",
           # slice 6
           "stdio.h", "stdlib.h", "string.h", "strings.h", "memory.h", "unistd.h", "sys/unistd.h", "assert.h", "ctype.h",
           "getopt.h", "time.h",
           # slice 12
           "sys/socket.h", "sys/un.h", "netinet/in.h", "arpa/inet.h", "netdb.h", "net/if.h", "ifaddrs.h",
           "netinet/tcp.h", "netinet/udp.h", "resolv.h", "arpa/nameser.h", "arpa/nameser_compat.h", "nss.h",
           "aliases.h", "rpc/netdb.h", "sys/socketvar.h"]
COLS = [("isoc", "ISOC"), ("posix", "POSIX"), ("xsi", "XSI"), ("misc", "MISC"), ("gnu", "GNU"), ("lfs64", "LFS64"),
        ("fob64", "FOB64"), ("time64", "TIME64"), ("dynstack", "DYNSTACK"), ("fortify", "FORTIFY"), ("cxx", "CXX")]


SLICE6 = ["stdio.h", "stdlib.h", "string.h", "strings.h", "memory.h", "unistd.h", "sys/unistd.h", "assert.h", "ctype.h",
          "getopt.h", "time.h"]
SLICE12 = HEADERS[HEADERS.index("sys/socket.h"):]
P1_HEADERS = SLICE6 + SLICE12
INT_SUFFIX = {"int": "", "long": "L", "long long": "LL", "unsigned int": "U", "unsigned long": "UL", "unsigned long long": "ULL"}


P8_CONFIGS = ("gnu17", "default", "gnu", "c11", "cxx17")
P8_PAIRS = [(["netinet/in.h"], ["linux/in.h", "linux/in6.h"]), (["net/if.h"], ["linux/if.h"]),
            (["netinet/in.h", "net/if.h"], ["linux/in.h", "linux/in6.h", "linux/if.h"])]


def p8_units():
    """P8 (plan/headers.md section 3, kernel UAPI): each libniji header with its kernel counterparts in both include
    orders; every unit also uses the shared tags, so a missing or doubled definition fails to compile. Only the libc-first order is
    asserted: the kernel header first fails to compile against the reference's headers as well (--system prints its verdicts),
    so that order is not part of the contract (RULING NEEDED in handoff/SL-HEADERS-S12.md)."""
    out = []
    for libc, kern in P8_PAIRS:
        use = "struct in_addr a; struct in6_addr b; struct sockaddr_in c; struct sockaddr_in6 d;\n" if "netinet/in.h" in libc else ""
        use += "struct ifreq e; struct ifmap f;\n" if "net/if.h" in libc else ""
        li = "".join(f"#include <{h}>\n" for h in libc)
        ki = "".join(f"#include <{h}>\n" for h in kern)
        out.append((f"P8 {'+'.join(libc)} then {'+'.join(kern)}", li + ki + use + "int niji_p8_tu;\n"))
    return out


def expand(configs, ids):
    """The configuration ids a visibility.tsv `configs` cell names."""
    if configs == "*":
        return set(ids)
    if configs.startswith("!"):
        return set(ids) - set(configs[1:].split(","))
    return set(configs.split(","))


def p1_units(ids, vis, macros, typedefs, layouts, members, foreign):
    """P1 of slice 6 (plan/headers.md section 3): per (header, configuration) one unit that includes the header and
    asserts, for what the header makes visible there, every macros.tsv integer value, every typedef size and
    alignment of typedefs.tsv and every structure size and alignment of layouts.tsv. A false condition is a
    negative array size, which every language mode rejects (C89 and C++98 have no static assertion)."""
    out = {}
    for r in vis:
        h = r["header"]
        if h not in P1_HEADERS or r["kind"] not in ("macro", "typedef", "struct"):
            continue
        n = r["name"]
        if h in SLICE12 and n in foreign:
            continue  # a name another slice's header owns (P1 runs there; signal.h and sys/param.h are slices 7 and 8)
        if n.startswith("__") or (n.startswith("_") and n.endswith("_H")):
            continue  # reserved names are emitted only where headers/reserved.tsv says so (owner rulings R-H2, R-H3)
        tests = []
        if r["kind"] == "macro" and n in macros and macros[n]["type"] in INT_SUFFIX and macros[n]["header"] == h:
            m = macros[n]
            v = int(m["value"])
            lit = ("(%d%s)" % (v, INT_SUFFIX[m["type"]])) if v >= 0 else "(%d%s)" % (v, INT_SUFFIX[m["type"]])
            tests.append(f"(({n}) == {lit})")
        elif r["kind"] == "typedef" and n in typedefs and typedefs[n]["size"]:
            t = typedefs[n]
            tests.append(f"sizeof({n}) == {t['size']}")
            tests.append(f"__alignof__({n}) == {t['align']}")
        elif r["kind"] == "struct" and ("struct " + n) in layouts:
            t = layouts["struct " + n]
            tests.append(f"sizeof(struct {n}) == {t['offset_or_size']}")
            tests.append(f"__alignof__(struct {n}) == {t['align']}")
            # offsets (plan section 3, P1): every member row that is not a bit-field, nested names by member path
            for m in members.get("struct " + n, []):
                tests.append(f"__builtin_offsetof(struct {n}, {m['member']}) == {m['offset_or_size']}")
        for cid in expand(r["configs"], ids):
            out.setdefault((h, cid), []).extend(tests)
    units = {}
    for (h, cid), tests in out.items():
        if tests:
            body = f"#include <{h}>\n" + "".join(f"typedef char niji_p1_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests))
            units.setdefault(cid, []).append((f"P1 {h}", body))
    return units


def rows(rel):
    out, head = [], None
    with open(os.path.join(ROOT, rel)) as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            cells = line.rstrip("\n").split("\t")
            if head is None:
                head = cells
            else:
                out.append(dict(zip(head, cells)))
    return out


def identity_unit(cxx, types):
    """P1 lines for the C 7.20.1 types (typedefs.tsv rows), C or C++."""
    lines = ["#include <stdint.h>"]
    for name, r in types:
        if cxx:
            lines.append(f"static_assert(__is_same({name}, {r['cxx']}), \"{name} is {r['cxx']}\");")
        else:
            lines.append(f"_Static_assert(sizeof({name}) == {r['size']} && _Alignof({name}) == {r['align']}, \"{name}\");")
    return "\n".join(lines) + "\n"


def p10(inc, work):
    """P10 of slice 12: the same program, built on the system's headers and on libniji's (`-nostdinc`), must print the
    same bytes (CMSG_*, IN6_IS_ADDR_*, the byte-order conversions)."""
    src = os.path.join(ROOT, "spec", "tools", "p10_net.c")
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    outs = {}
    for tag, extra in (("reference", []), ("libniji", ["-nostdinc", "-isystem", inc, "-isystem", gccinc])):
        exe = os.path.join(work, "p10_" + tag)
        c = subprocess.run(["gcc", "-std=gnu17", "-O2", "-Wall", "-Wextra", "-Werror", *extra, src, "-o", exe], capture_output=True, text=True)
        if c.returncode:
            print(f"FAIL P10 {tag} build: {' | '.join(c.stderr.strip().splitlines()[:3])[:400]}")
            return 1
        outs[tag] = subprocess.run([exe], capture_output=True, text=True).stdout
    ok = outs["reference"] == outs["libniji"] and outs["reference"].count("\n") > 1000
    print(f"P10 net macros: {outs['reference'].count(chr(10))} lines, {'identical' if ok else 'DIFFERENT'}")
    if not ok:
        a, b = outs["reference"].splitlines(), outs["libniji"].splitlines()
        for i, (x, y) in enumerate(zip(a, b)):
            if x != y:
                print(f"FAIL P10 line {i}: reference `{x}` libniji `{y}`")
                break
    return 0 if ok else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--work", default="")
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    ap.add_argument("--system", action="store_true", help="compile only the P8 units against the system's own headers (the reference's verdicts)")
    ap.add_argument("--p10", action="store_true", help="run only P10: spec/tools/p10_net.c on the reference's headers and on libniji's, outputs compared")
    ap.add_argument("--kernel", default="", help="a staging of the kernel UAPI headers (linux/, asm/, asm-generic/): enables P8")
    args = ap.parse_args()
    inc = os.path.abspath(args.include)
    if not os.path.isfile(os.path.join(inc, "features.h")):
        sys.exit(f"header_check: no features.h under {inc}")
    work = args.work or tempfile.mkdtemp(prefix="header_check.")
    os.makedirs(work, exist_ok=True)
    if args.p10:
        return p10(inc, work)
    cfgs = rows("spec/abi/ftm-configs.tsv")
    levels = {r["id"]: r for r in rows("headers/levels.tsv")}
    typedefs = {r["name"]: r for r in rows("spec/abi/typedefs.tsv") if r["kind"] == "typedef"}
    wanted = [n for n in hp.c_integer_names() if n.islower()] + ["intmax_t", "uintmax_t", "intptr_t", "uintptr_t"]
    missing = [n for n in wanted if n not in typedefs]
    types = [(n, typedefs[n]) for n in wanted if n in typedefs]
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    vis = rows("spec/abi/visibility.tsv")
    macros = {r["name"]: r for r in rows("spec/abi/macros.tsv")}
    layouts, members = {}, {}
    for r in rows("spec/abi/layouts.tsv"):
        if not r["member"]:
            layouts[r["type"]] = r
        elif not r["bitfield"] and r["offset_or_size"].isdigit():
            members.setdefault(r["type"], []).append(r)
    others = set(HEADERS[:HEADERS.index("sys/socket.h")]) | {"signal.h", "sys/param.h"}
    foreign = {r["name"] for r in vis if r["header"] in others}
    p1s = p1_units([c["id"] for c in cfgs], vis, macros, typedefs, layouts, members, foreign)
    fails, total = len(missing), len(missing)
    for n in missing:
        print(f"FAIL P1 {n}: no typedefs.tsv row")
    for c in cfgs:
        cxx = c["language"] == "c++"
        ext = "cc" if cxx else "c"
        if args.system:
            if c["id"] in P8_CONFIGS:
                for i, (what, text) in enumerate(p8_units()):
                    src = os.path.join(work, f"{c['id']}-{i}.{ext}")
                    with open(src, "w") as f:
                        f.write(text)
                    r = subprocess.run(["g++" if cxx else "gcc", *c["flags"].split(), "-fsyntax-only", "-Wall", "-Wextra", "-Werror", src], capture_output=True, text=True)
                    print(f"REF {what} {c['id']}: {'OK' if r.returncode == 0 else 'FAIL'}")
            continue
        base = ["g++" if cxx else "gcc", *c["flags"].split(), "-nostdinc", "-isystem", inc,
                "-isystem", gccinc, *(["-isystem", os.path.abspath(args.kernel)] if args.kernel else []), "-fsyntax-only", "-Wall", "-Wextra", "-Wsystem-headers", "-Werror"]
        lv = levels[c["id"]]
        rejected = lv["reject"] == "1"
        units = [("levels", "#include <features.h>\n" + "".join(
            f"#if __NIJI_{mac} != {lv[col]}\n#error \"__NIJI_{mac} is not {lv[col]}\"\n#endif\n" for col, mac in COLS))]
        if not rejected:
            units += [(f"P5 {h}", f"#include <{h}>\n#include <{h}>\nint niji_check_tu;\n") for h in HEADERS]
            # static_assert is C++11 and _Static_assert C11 (C89 mode has neither as a keyword).
            if c["id"] not in ("cxx98", "c89", "isoc99"):
                units.append(("P1 stdint", identity_unit(cxx, types)))
        if not rejected:
            units += p1s.get(c["id"], [])
        if not rejected and args.kernel and c["id"] in P8_CONFIGS:
            units += p8_units()
        for i, (what, text) in enumerate(units):
            src = os.path.join(work, f"{c['id']}-{i}.{ext}")
            with open(src, "w") as f:
                f.write(text)
            r = subprocess.run(base + [src], capture_output=True, text=True)
            total += 1
            ok = (r.returncode != 0) if (rejected and what == "levels") else (r.returncode == 0)
            if not ok:
                fails += 1
                print(f"FAIL {what} {c['id']}: rc {r.returncode}: {' | '.join(r.stderr.strip().splitlines()[:3])[:400]}")
    print(f"header_check: {fails} failed of {total}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
