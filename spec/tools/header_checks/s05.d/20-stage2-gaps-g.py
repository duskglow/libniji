"""Slice 5 checks: stage2-gaps-g (header_checks/s05.d/, loaded after s05.py by spec/tools/header_check.py;
header_checks/README.md). Also the checks of this story's <string.h>, <ctype.h>, <stdio.h> (slice 6) and <obstack.h>
(slice 17) changes: those slices have no check module to hang a fragment on."""
import os
import subprocess

from header_check import clang_compilers

ORDER = ("__BYTE_ORDER", "__LITTLE_ENDIAN", "__BIG_ENDIAN", "__PDP_ENDIAN", "__FLOAT_WORD_ORDER")
# The headers that define __bswap_16/32/64 themselves (headers/<h>.d/010-bswap.frag).
BSWAP_HEADERS = ("byteswap.h", "endian.h", "sys/types.h", "stdlib.h", "netinet/in.h")


def shown(ctx, header, name, kind=None):
    return set().union(*(ctx.expand(r["configs"]) for r in ctx.vis
                         if r["header"] == header and r["name"] == name and kind in (None, r["kind"])))


def lang(ctx):
    return {c["id"]: c["language"] for c in ctx.cfgs}


def same(cxx, a, b):
    return f"__is_same ({a}, {b})" if cxx else f"__builtin_types_compatible_p ({a}, {b})"


def units(ctx):
    """No spec clause (reserved names and GNU interfaces; black-box probes of the reference, HEADERS-STAGE2-GAPS-G,
    Yohane stage 2 on 26.1). Defects each unit catches:
    (1) the byte-order names missing where the reference shows them (systemd's -Werror=undef #if) or shown where it
        does not (P1 checks their values);
    (2) __bswap_16/32/64 missing, a macro instead of a function, a wrong result type, or defined twice when two of the
        headers that carry them meet in one translation unit, and taken from the program where not shown;
    (3) strerror_r's XSI form (the configurations whose calls bind to __xpg_strerror_r) returning char * or marked
        warn_unused_result (libssh, bind9, hiredis), and its GNU form not returning char *;
    (4) __toupper_l/__tolower_l not declared as functions of (int, locale_t) where shown (postgresql-17), and not
        macros under _FORTIFY_SOURCE where the reference makes them macros;
    (5) obstack_begin, obstack_specify_allocation(_with_arg), obstack_chunkfun, obstack_freefun, obstack_empty_p and
        obstack_memory_used missing or not taking the documented arguments, obstack_empty_p or obstack_memory_used
        not int (dwarves)."""
    lg = lang(ctx)
    out = ctx.exact("endian.h", ORDER) + ctx.exact("sys/types.h", ORDER)
    every = set.intersection(*(shown(ctx, h, "__bswap_16") for h in BSWAP_HEADERS))
    for cid in ctx.ids:
        cxx = lg[cid] == "c++"
        if cid in every:
            body = [f"#include <{h}>" for h in BSWAP_HEADERS] + [f"#include <{h}>" for h in reversed(BSWAP_HEADERS)]
            body += ["#ifdef __bswap_16\n#error \"__bswap_16 is a macro\"\n#endif"]
            for w, t in ((16, "unsigned short int"), (32, "unsigned int"), (64, "unsigned long int")):
                body.append(f"typedef char niji_bs{w}[{same(cxx, f'__typeof__ (__bswap_{w} (1))', t)} ? 1 : -1];")
                body.append(f"{t} (*niji_bsp{w}) ({t}) = __bswap_{w};")
            out.append((cid, "bswap functions", "\n".join(body) + "\n"))
        elif cid not in shown(ctx, "endian.h", "__bswap_16") and cid in shown(ctx, "endian.h", "__BYTE_ORDER"):
            out.append((cid, "bswap free endian.h", "#include <endian.h>\nint __bswap_16, __bswap_32, __bswap_64;\n"))
    calls = [r for r in ctx.rows("spec/abi/callsites.tsv") if r["name"] == "strerror_r" and not r["configs"].startswith("!")]
    xsi = set().union(*(ctx.expand(r["configs"]) for r in calls if r["symbol"] == "__xpg_strerror_r"))
    gnu = set().union(*(ctx.expand(r["configs"]) for r in calls if r["symbol"] == "strerror_r"))
    for cid in sorted(shown(ctx, "string.h", "strerror_r") & (xsi | gnu)):
        cxx = lg[cid] == "c++"
        # The call's result type (a C++ function type also carries the declaration's noexcept).
        body = ["#include <string.h>", "#include <stddef.h>", "extern char niji_se_b[8];",
                f"typedef char niji_se_r[{same(cxx, '__typeof__ (strerror_r (0, niji_se_b, 8))', 'int' if cid in xsi else 'char *')} ? 1 : -1];"]
        if cid in xsi:
            body += ["typedef char niji_se_w[!__builtin_has_attribute (strerror_r, warn_unused_result) ? 1 : -1];",
                     "typedef char niji_se_n[__builtin_has_attribute (strerror_r, nonnull (2)) ? 1 : -1];"]
            if not cxx:
                body.append(f"typedef char niji_se_t[{same(cxx, '__typeof__ (strerror_r)', 'int (int, char *, size_t)')} ? 1 : -1];")
        out.append((cid, f"strerror_r {'XSI' if cid in xsi else 'GNU'} form", "\n".join(body) + "\n"))
    fn = shown(ctx, "ctype.h", "__toupper_l", "function")
    mac = shown(ctx, "ctype.h", "__toupper_l", "macro")
    for cid in sorted(fn | mac):
        body = ["#include <ctype.h>", f"#if{'n' if cid in mac else ''}def __toupper_l\n#error \"__toupper_l macro\"\n#endif"]
        body += [f"int (*niji_tl{i}) (int, locale_t) = {n};" for i, n in enumerate(("(__toupper_l)", "(__tolower_l)"))]
        body.append("int niji_tl (locale_t l) { return __toupper_l ('a', l) + __tolower_l ('A', l); }")
        out.append((cid, "__toupper_l declared", "\n".join(body) + "\n"))
    for cid in sorted(shown(ctx, "obstack.h", "obstack_begin")):
        cxx = lg[cid] == "c++"
        body = ["#include <obstack.h>", "#include <stdlib.h>", "#define obstack_chunk_alloc malloc",
                "#define obstack_chunk_free free",
                "static void *niji_ca (void *a, long n) { (void) a; return malloc ((size_t) n); }",
                "static void niji_cf (void *a, void *p) { (void) a; free (p); }",
                "int niji_ob (struct obstack *o)\n{\n  int r = obstack_begin (o, 4000);\n"
                "  r += obstack_specify_allocation (o, 0, 0, malloc, free);\n"
                "  r += obstack_specify_allocation_with_arg (o, 0, 16, niji_ca, niji_cf, (void *) 0);\n"
                "  obstack_chunkfun (o, malloc);\n  obstack_freefun (o, free);\n"
                # The result types in C (the probe compared them there; C++ is not asserted).
                + ("" if cxx else
                   f"  typedef char e[{same(cxx, '__typeof__ (obstack_empty_p (o))', 'int')} ? 1 : -1];\n"
                   f"  typedef char m[{same(cxx, '__typeof__ (obstack_memory_used (o))', 'int')} ? 1 : -1];\n"
                   "  r += (int) sizeof (e) + (int) sizeof (m);\n")
                + "  return r + obstack_empty_p (o) + obstack_memory_used (o);\n}"]
        out.append((cid, "obstack interface", "\n".join(body) + "\n"))
    return out


CLANG_SRC = """#define _GNU_SOURCE 1
#include <stdio.h>
#include <wchar.h>
#include <syslog.h>
#include <fcntl.h>
extern char niji_b[8];
extern wchar_t niji_w[8];
int niji_f (char **p, int x)
{
  syslog (LOG_ERR, "%d", x);
  return printf ("%d", x) + fprintf (stderr, "%d", x) + sprintf (niji_b, "%d", x) + snprintf (niji_b, 8, "%d", x)
    + asprintf (p, "%d", x) + dprintf (1, "%d", x) + wprintf (L"%d", x) + fwprintf (stderr, L"%d", x)
    + swprintf (niji_w, 8, L"%d", x) + printf ("x") + open ("f", O_RDONLY) + open ("f", O_CREAT | O_WRONLY, 0600);
}
"""
# The `_chk` symbols a clang C translation unit at _FORTIFY_SOURCE=2 references (the reference's, black-box probe).
CLANG_CHK = ("__printf_chk", "__fprintf_chk", "__sprintf_chk", "__snprintf_chk", "__asprintf_chk", "__dprintf_chk",
             "__wprintf_chk", "__fwprintf_chk", "__swprintf_chk", "__syslog_chk")


def problems(ctx):
    """No spec clause beyond the GCC manual's __builtin_va_arg_pack, which clang does not have (HEADERS-STAGE2-GAPS-G,
    llvm-toolchain-19): under clang at _FORTIFY_SOURCE=2 the printf family compiles in C and C++ (-Werror), the C
    object references each CLANG_CHK symbol and no __builtin_va_arg_pack, and the open family keeps its plain call.
    Defects: a GCC-only wrapper under clang (C++ fails; C references an undefined __builtin_va_arg_pack), the clang
    route dropped (no _chk call), a wrong argument order. Skipped, with the reason printed, without clang."""
    cc = clang_compilers()
    if not cc:
        print("header_check: stage2-gaps-g clang fortify skipped: no clang-19 or clang on PATH, NIJI_CLANG unset")
        return []
    res = subprocess.run([cc[0], "-print-resource-dir"], capture_output=True, text=True, check=True).stdout.strip()
    fails = []
    for comp, ext in ((cc[0], "c"), (cc[1], "cc")):
        src = os.path.join(ctx.work, f"s2g-clang-fortify.{ext}")
        obj = src + ".o"
        with open(src, "w") as f:
            f.write(CLANG_SRC)
        r = subprocess.run([comp, "-O2", "-D_FORTIFY_SOURCE=2", "-nostdinc", "-isystem", os.path.join(res, "include"),
                            "-isystem", ctx.inc, "-Wall", "-Werror", "-c", src, "-o", obj], capture_output=True, text=True)
        if r.returncode != 0:
            fails.append(f"clang fortify {ext}: rc {r.returncode}: {' | '.join(r.stderr.strip().splitlines()[:3])[:400]}")
            continue
        und = set(subprocess.run(["nm", "-u", obj], capture_output=True, text=True, check=True).stdout.split())
        if ext == "c":
            fails += [f"clang fortify c: {s} not referenced" for s in CLANG_CHK if s not in und]
        fails += [f"clang fortify {ext}: references {s}" for s in und if "va_arg_pack" in s or s.startswith("__open")]
    return fails
