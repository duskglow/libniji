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
import glob
import os
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import header_probe as hp  # noqa: E402  (ROOT, c_integer_names)

ROOT = hp.ROOT
SLICE9 = ["sys/signalfd.h", "sys/timerfd.h", "sys/eventfd.h", "sys/epoll.h", "sys/inotify.h", "sys/fanotify.h", "sys/prctl.h",
          "sys/personality.h", "sys/pidfd.h", "sys/auxv.h", "sys/random.h", "sys/syscall.h", "syscall.h", "sys/ptrace.h",
          "sys/user.h", "sys/reg.h", "sys/procfs.h", "sys/debugreg.h"]
HEADERS = ["features.h", "features-time64.h", "stdc-predef.h", "sys/cdefs.h", "sys/types.h", "sys/bitypes.h",
           "stdint.h", "inttypes.h", "limits.h", "endian.h", "byteswap.h", "errno.h", "sys/errno.h", "alloca.h",
           "gnu-versions.h", "gnu/stubs.h",
           # slice 6
           "stdio.h", "stdlib.h", "string.h", "strings.h", "memory.h", "unistd.h", "sys/unistd.h", "assert.h", "ctype.h",
           "getopt.h", "time.h"] + SLICE9 + [
           # slice 11
           "math.h", "complex.h", "tgmath.h", "fenv.h", "fpu_control.h", "ieee754.h", "values.h"]
COLS = [("isoc", "ISOC"), ("posix", "POSIX"), ("xsi", "XSI"), ("misc", "MISC"), ("gnu", "GNU"), ("lfs64", "LFS64"),
        ("fob64", "FOB64"), ("time64", "TIME64"), ("dynstack", "DYNSTACK"), ("fortify", "FORTIFY"), ("cxx", "CXX")]


# Slice 14 (aio.h and mqueue.h wait for the generation of struct sigevent and struct f_owner_ex).
SLICE14 = ["pwd.h", "grp.h", "shadow.h", "gshadow.h", "utmp.h", "utmpx.h", "lastlog.h", "sys/fsuid.h", "termios.h",
           "sys/termios.h", "termio.h", "sgtty.h", "pty.h", "ttyent.h", "sys/ttychars.h", "sys/ttydefaults.h", "syslog.h",
           "sys/syslog.h", "sys/ipc.h", "sys/msg.h", "sys/sem.h", "sys/shm.h"]
HEADERS += SLICE14
SLICE6 = ["stdio.h", "stdlib.h", "string.h", "strings.h", "memory.h", "unistd.h", "sys/unistd.h", "assert.h", "ctype.h",
          "getopt.h", "time.h", "math.h", "complex.h", "tgmath.h", "fenv.h", "fpu_control.h", "ieee754.h", "values.h"]
FLOAT_SUFFIX = {"float": ("f", 4), "double": ("", 8), "long double": ("L", 16), "_Float32": ("f32", 4), "_Float64": ("f64", 8),
                "_Float128": ("f128", 16), "_Float32x": ("f32x", 8), "_Float64x": ("f64x", 16)}
INT_SUFFIX = {"int": "", "long": "L", "long long": "LL", "unsigned int": "U", "unsigned long": "UL", "unsigned long long": "ULL"}


def expand(configs, ids):
    """The configuration ids a visibility.tsv `configs` cell names."""
    if configs == "*":
        return set(ids)
    if configs.startswith("!"):
        return set(ids) - set(configs[1:].split(","))
    return set(configs.split(","))


def decimal_literal(hexconst):
    """The exact value of a probed hexadecimal floating constant as a decimal literal (60 digits: well above the 36 a
    binary128 value needs, so the compiler reads it back as the same value). Hexadecimal floating constants are not
    tokens in C89 and C++98 to C++14, and the unit is compiled in every mode."""
    import decimal
    neg = hexconst.startswith("-")
    h = hexconst.lstrip("-")
    mant, exp = h[2:].split("p")
    ip, _, fp = mant.partition(".")
    m = int((ip + fp) or "0", 16)
    e = int(exp) - 4 * len(fp)
    with decimal.localcontext() as ctx:
        ctx.prec = 60
        v = decimal.Decimal(m) * (decimal.Decimal(2) ** e)
    return ("-" if neg else "") + format(v, "e")


def p1_units(ids, vis, macros, typedefs, layouts):
    """P1 of slice 6 (plan/headers.md section 3): per (header, configuration) one unit that includes the header and
    asserts, for what the header makes visible there, every macros.tsv integer value, every typedef size and
    alignment of typedefs.tsv and every structure size and alignment of layouts.tsv. A false condition is a
    negative array size, which every language mode rejects (C89 and C++98 have no static assertion)."""
    out = {}
    # A template with `@@emit-c` (slice 11) decides a name by its C configurations only: the C++ names of such a header
    # come from the C++ standard library's wrappers, so a name with no C row is not the header's (same filter as headers-gen).
    emit_c = set()
    for t in glob.glob(os.path.join(ROOT, "headers", "**", "*.h.in"), recursive=True):
        with open(t) as f:
            if any(l.startswith("@@emit-c") for l in f):
                emit_c.add(os.path.relpath(t, os.path.join(ROOT, "headers"))[:-3])
    c_names = {(r["header"], r["name"]) for r in vis if any(not c.startswith("cxx") for c in re.split(r"[,!]", r["configs"]) if c)}
    for r in vis:
        h = r["header"]
        if h not in SLICE6 + SLICE9 + SLICE14 or r["kind"] not in ("macro", "typedef", "struct", "union"):
            continue
        n = r["name"]
        if h in emit_c and (h, n) not in c_names:
            continue
        if n.startswith("__") or (n.startswith("_") and n.endswith("_H")):
            continue  # reserved names are emitted only where headers/reserved.tsv says so (owner rulings R-H2, R-H3)
        tests = []
        if r["kind"] == "macro" and n in macros and macros[n]["type"] in INT_SUFFIX and macros[n]["header"] == h:
            m = macros[n]
            v = int(m["value"])
            sfx = INT_SUFFIX[m["type"]]
            # The most negative value has no literal (its magnitude is unsigned): (-max - 1).
            lit = "(%d%s)" % (v, sfx) if v > -(1 << 31) or (v != -(1 << 31) and v != -(1 << 63)) else "(%d%s-1)" % (v + 1, sfx)
            tests.append(f"(({n}) == {lit})")
        elif r["kind"] == "macro" and n in macros and macros[n]["type"] in FLOAT_SUFFIX and macros[n]["header"] == h and macros[n]["value"].startswith("0x"):
            # A floating macro (C 7.12, TS 18661): its size is its type's and its value the exact probed constant.
            m = macros[n]
            suffix, size = FLOAT_SUFFIX[m["type"]]
            tests.append(f"sizeof({n}) == {size}")
            tests.append(f"F:(({n}) == {decimal_literal(m['value'])}{suffix})")
        elif r["kind"] == "typedef" and n in typedefs and typedefs[n]["size"]:
            t = typedefs[n]
            tests.append(f"sizeof({n}) == {t['size']}")
            tests.append(f"__alignof__({n}) == {t['align']}")
        elif r["kind"] in ("struct", "union") and (r["kind"] + " " + n) in layouts:
            t = layouts[r["kind"] + " " + n]
            tests.append(f"sizeof({r['kind']} {n}) == {t['offset_or_size']}")
            tests.append(f"__alignof__({r['kind']} {n}) == {t['align']}")
        for cid in expand(r["configs"], ids):
            out.setdefault((h, cid), []).extend(tests)
    units = {}
    for (h, cid), tests in out.items():
        if tests:
            # A floating comparison is not an integer constant expression: those use _Static_assert (C11, C++11).
            body = f"#include <{h}>\n" + "".join(f"typedef char niji_p1_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests) if not t.startswith("F:"))
            ft = [t[2:] for t in tests if t.startswith("F:")]
            if ft:
                body += "#if defined __cplusplus && __cplusplus >= 201103L\n"
                body += "".join(f"static_assert({t}, \"niji_p1_float\");\n" for t in ft)
                body += "#elif !defined __cplusplus && defined __STDC_VERSION__ && __STDC_VERSION__ >= 201112L\n"
                body += "".join(f"_Static_assert({t}, \"niji_p1_float\");\n" for t in ft) + "#endif\n"
            units.setdefault(cid, []).append((f"P1 {h}", body))
    # sys/syscall.h (SL-HEADERS-S09): SYS_<name> is the number of spec/abi/syscalls.tsv, which is the kernel's __NR_<name>.
    sysrows = rows("spec/abi/syscalls.tsv")
    body = "#include <sys/syscall.h>\n" + "".join(f"typedef char niji_p1_sys_{r['name']}[(SYS_{r['name']} == {r['number']}) ? 1 : -1];\n" for r in sysrows)
    for cid in ids:
        units.setdefault(cid, []).append(("P1 sys/syscall.h", body))
    return units


def uapi_flags():
    """`-isystem` for the kernel UAPI directories (linux-libc-dev) the headers of slice 9 include: $NIJI_UAPI (the tree
    `cargo xtask so` stages at target/niji/uapi) or, when unset, the build host's multiarch and plain include
    directories (their glibc headers stay out of reach: the units use -nostdinc and the libniji tree comes first)."""
    d = os.environ.get("NIJI_UAPI")
    if d:
        return ["-isystem", d]
    return ["-isystem", "/usr/include/x86_64-linux-gnu", "-isystem", "/usr/include"] if os.path.isdir("/usr/include/linux") else []


# P8 (plan/headers.md section 3, slice 9): a libniji header and the kernel UAPI header that defines the same names. Both
# include orders must compile, as C and as C++, with and without _GNU_SOURCE; a pair whose kernel header is not installed
# is skipped.
P8_PAIRS = [("sys/ptrace.h", "linux/ptrace.h"), ("sys/ptrace.h", "asm/ptrace.h"), ("sys/prctl.h", "linux/prctl.h"),
            ("sys/inotify.h", "linux/inotify.h"), ("sys/fanotify.h", "linux/fanotify.h"), ("sys/epoll.h", "linux/eventpoll.h"),
            ("sys/random.h", "linux/random.h"), ("sys/pidfd.h", "linux/pidfd.h"), ("sys/auxv.h", "linux/auxvec.h"),
            ("sys/auxv.h", "asm/auxvec.h"), ("sys/personality.h", "linux/personality.h"), ("sys/eventfd.h", "linux/eventfd.h"),
            ("sys/timerfd.h", "linux/timerfd.h"), ("sys/signalfd.h", "linux/signalfd.h"), ("sys/syscall.h", "asm/unistd.h"),
            ("syscall.h", "asm/unistd.h"), ("sys/debugreg.h", "asm/debugreg.h"), ("sys/user.h", "asm/user.h")]
P8_MODES = [("c", "-std=c17"), ("c", "-std=c17 -D_GNU_SOURCE"), ("c", "-std=gnu17"), ("c++", "-std=c++17 -D_GNU_SOURCE")]


def reference_works(work):
    """Whether the system's own C library headers compile here (the reference of black-box probes)."""
    src = os.path.join(work, "ref.c")
    with open(src, "w") as f:
        f.write("#include <stdio.h>\n#include <sys/types.h>\nint niji_ref_tu;\n")
    return subprocess.run(["gcc", "-fsyntax-only", src], capture_output=True).returncode == 0


def p8(inc, gccinc, work):
    """(failures, units) of the P8 include-order units. The kernel's own headers conflict with a libc's for most
    of these pairs (only `linux/libc-compat.h`'s set has a protocol); a unit that does not compile with the reference
    system's headers either is printed as a NOTE and is not a failure (black-box parity, plan/headers.md 2.2)."""
    ref = reference_works(work)
    uapi = uapi_flags()
    dirs = [uapi[i + 1] for i in range(0, len(uapi), 2)]
    fails = total = 0
    for a, b in P8_PAIRS:
        if not any(os.path.isfile(os.path.join(d, b)) for d in dirs):
            continue
        for lang, flags in P8_MODES:
            for order in ((a, b), (b, a)):
                src = os.path.join(work, f"p8-{total}.{'cc' if lang == 'c++' else 'c'}")
                with open(src, "w") as f:
                    f.write("".join(f"#include <{h}>\n" for h in order) + "int niji_p8_tu;\n")
                cmd = ["g++" if lang == "c++" else "gcc", *flags.split(), "-nostdinc", "-isystem", inc, "-isystem", gccinc, *uapi,
                       "-fsyntax-only", "-Wall", "-Wextra", "-Werror", src]  # a program's flags: no -Wsystem-headers (P5 has it)
                r = subprocess.run(cmd, capture_output=True, text=True)
                total += 1
                if r.returncode != 0 and ref:
                    rr = subprocess.run(["g++" if lang == "c++" else "gcc", *flags.split(), "-fsyntax-only", "-Wall", "-Wextra", "-Werror", src],
                                        capture_output=True)
                    if rr.returncode != 0:
                        print(f"NOTE P8 {order[0]} then {order[1]} [{flags}]: the reference system does not compile it either")
                        continue
                if r.returncode != 0:
                    fails += 1
                    print(f"FAIL P8 {order[0]} then {order[1]} [{flags}]: rc {r.returncode}: {' | '.join(r.stderr.strip().splitlines()[:3])[:400]}")
    return fails, total


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


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--work", default="")
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    args = ap.parse_args()
    inc = os.path.abspath(args.include)
    if not os.path.isfile(os.path.join(inc, "features.h")):
        sys.exit(f"header_check: no features.h under {inc}")
    work = args.work or tempfile.mkdtemp(prefix="header_check.")
    os.makedirs(work, exist_ok=True)
    cfgs = rows("spec/abi/ftm-configs.tsv")
    levels = {r["id"]: r for r in rows("headers/levels.tsv")}
    typedefs = {r["name"]: r for r in rows("spec/abi/typedefs.tsv") if r["kind"] == "typedef"}
    wanted = [n for n in hp.c_integer_names() if n.islower()] + ["intmax_t", "uintmax_t", "intptr_t", "uintptr_t"]
    missing = [n for n in wanted if n not in typedefs]
    types = [(n, typedefs[n]) for n in wanted if n in typedefs]
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    vis = rows("spec/abi/visibility.tsv")
    macros = {r["name"]: r for r in rows("spec/abi/macros.tsv")}
    layouts = {}
    for r in rows("spec/abi/layouts.tsv"):
        if not r["member"]:
            layouts[r["type"]] = r
    p1s = p1_units([c["id"] for c in cfgs], vis, macros, typedefs, layouts)
    fails, total = len(missing), len(missing)
    for n in missing:
        print(f"FAIL P1 {n}: no typedefs.tsv row")
    # The kernel's UAPI headers (linux-libc-dev; libniji ships none): a directory of links to the build host's
    # `linux`, `asm-generic` and `asm`, so the C library's own headers of the host stay out of the units.
    kern = os.path.join(work, "kernel-uapi")
    os.makedirs(kern, exist_ok=True)
    for d in ("linux", "asm-generic", "asm"):
        for base_dir in ("/usr/include", "/usr/include/x86_64-linux-gnu"):
            src = os.path.join(base_dir, d)
            if os.path.isdir(src) and not os.path.lexists(os.path.join(kern, d)):
                os.symlink(src, os.path.join(kern, d))
    for c in cfgs:
        cxx = c["language"] == "c++"
        ext = "cc" if cxx else "c"
        base = ["g++" if cxx else "gcc", *c["flags"].split(), "-nostdinc", "-isystem", inc,
                "-isystem", gccinc, *(uapi_flags() if os.environ.get("NIJI_UAPI") else ["-isystem", kern]),
                "-fsyntax-only", "-Wall", "-Wextra", "-Wsystem-headers", "-Werror"]
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
        if c["id"] == "gnu":
            with open(os.path.join(ROOT, "spec", "tools", "tgmath_types.c")) as f:
                units.append(("P6 tgmath", f.read()))
            with open(os.path.join(ROOT, "spec", "tools", "ieee754_fields.c")) as f:
                units.append(("P6 ieee754 fields", f.read()))
        for i, (what, text) in enumerate(units):
            src = os.path.join(work, f"{c['id']}-{i}.{ext}")
            with open(src, "w") as f:
                f.write(text)
            # A unit whose first lines say `niji: codegen` needs the optimiser (its checks are folded, not evaluated by the parser).
            cmd = [("-O1" if a == "-fsyntax-only" else a) for a in base] + ["-S", "-o", os.devnull] if "niji: codegen" in text else base
            r = subprocess.run(cmd + [src], capture_output=True, text=True)
            total += 1
            ok = (r.returncode != 0) if (rejected and what == "levels") else (r.returncode == 0)
            if not ok:
                fails += 1
                print(f"FAIL {what} {c['id']}: rc {r.returncode}: {' | '.join(r.stderr.strip().splitlines()[:3])[:400]}")
    f8, t8 = p8(inc, gccinc, work)
    fails, total = fails + f8, total + t8
    print(f"header_check: {fails} failed of {total}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
