"""Slice 13 checks: stage2-gaps-m (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md). The configure-probe gaps CONFIGURE-DIFF found (handoff/CONFIGURE-DIFF-report.md, rows
L2-L13 and L15): each check is the form of the reference's answer to that probe (black-box probes of the reference,
HEADERS-STAGE2-GAPS-M; no header text read).

Stand-alone against the build host's own C library headers (the expectations are the reference's, so they hold there
too): python3 spec/tools/header_checks/s13.d/80-stage2-gaps-m.py --reference
"""
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
from header_check import expand, kernel_flags  # noqa: E402

# Every configuration: the stub markers autoconf's AC_CHECK_FUNC tests after <limits.h> (L5), __GNUC_PREREQ in a
# constant expression (L3, gnulib's "glibc-compatible sys/cdefs.h" configure test), the GNU interface versions a
# package's own glob, obstack, regex and getopt compare (L4, GNU make's "working GNU glob" test), d_fileno (L6: bash,
# readline, apr) and struct utmpx's array sizes under their reserved names (L8: procps).
STUBS = ("__stub_chflags", "__stub_fchflags", "__stub_gtty", "__stub_revoke", "__stub_setlogin", "__stub_sigreturn",
         "__stub_stty")
STUB_UNIT = "#include <limits.h>\n" + "".join(f"#ifndef {n}\n#error \"{n} not defined\"\n#endif\n" for n in STUBS)
PREREQ_UNIT = """#include <sys/cdefs.h>
#if !__GNUC_PREREQ (4, 0) || __GNUC_PREREQ (999, 0) || __glibc_clang_prereq (1, 0)
#error "__GNUC_PREREQ or __glibc_clang_prereq gives another answer"
#endif
typedef char niji_gm_prereq[__GNUC_PREREQ (2, 95) && !__GNUC_PREREQ (__GNUC__ + 1, 0) ? 1 : -1];
"""
VERSIONS_UNIT = """#include <gnu-versions.h>
#if _GNU_GLOB_INTERFACE_VERSION != 2 || _GNU_GETOPT_INTERFACE_VERSION != 2 || _GNU_REGEX_INTERFACE_VERSION != 1 \\
  || _GNU_OBSTACK_INTERFACE_VERSION != 1
#error "a GNU interface version differs"
#endif
"""
DIRENT_UNIT = """#include <dirent.h>
#ifndef d_fileno
#error "d_fileno is no macro"
#endif
typedef char niji_gm_dfileno[sizeof (((struct dirent *) 0)->d_fileno) == sizeof (((struct dirent *) 0)->d_ino) ? 1 : -1];
"""


def _utmpx_unit(ctx):
    v = {n: int(ctx.macros[f"UT_{n}"]["value"]) for n in ("HOSTSIZE", "LINESIZE", "NAMESIZE")}
    return f"""#include <utmpx.h>
typedef char niji_gm_ut[__UT_HOSTSIZE == {v['HOSTSIZE']} && __UT_LINESIZE == {v['LINESIZE']} \\
  && __UT_NAMESIZE == {v['NAMESIZE']} ? 1 : -1];
typedef char niji_gm_uts[sizeof (((struct utmpx *) 0)->ut_host) == __UT_HOSTSIZE
  && sizeof (((struct utmpx *) 0)->ut_line) == __UT_LINESIZE ? 1 : -1];
"""


# L12: _REENTRANT and _THREAD_SAFE raise a POSIX level below 1003.1c to it and never lower one or turn the default set
# off; a GNU language mode whose only feature-test macros are ISO C ones keeps POSIX 2008 and defines _POSIX_C_SOURCE,
# _POSIX_SOURCE and _ATFILE_SOURCE but not the MISC set. (configuration, defines before <features.h>, conditions)
FTM_CASES = (
    ("c17", ("_REENTRANT",), "_POSIX_C_SOURCE == 199506L && defined _POSIX_SOURCE && !defined __USE_MISC"
                             " && defined __USE_POSIX199506 && !defined __USE_XOPEN2K"),
    ("c17", ("_THREAD_SAFE",), "_POSIX_C_SOURCE == 199506L && defined _POSIX_SOURCE && !defined __USE_MISC"),
    ("posix2", ("_REENTRANT", "_THREAD_SAFE"), "_POSIX_C_SOURCE == 199506L && defined __USE_POSIX199506"),
    ("posix200809", ("_REENTRANT",), "_POSIX_C_SOURCE == 200809L && defined __USE_XOPEN2K8"),
    ("gnu17", ("_REENTRANT",), "_POSIX_C_SOURCE == 200809L && defined _DEFAULT_SOURCE && defined __USE_MISC"),
    ("gnu17", ("_THREAD_SAFE",), "defined __USE_MISC && defined __USE_XOPEN2K8 && !defined __USE_GNU"),
    ("gnu17", ("_ISOC99_SOURCE",), "_POSIX_C_SOURCE == 200809L && defined _POSIX_SOURCE && defined _ATFILE_SOURCE"
                                   " && defined __USE_XOPEN2K8 && !defined __USE_MISC"),
    ("gnu17", ("_ISOC11_SOURCE", "_REENTRANT"), "_POSIX_C_SOURCE == 200809L && !defined __USE_MISC"),
    ("cxx17", ("_REENTRANT",), "defined __USE_GNU && _POSIX_C_SOURCE == 200809L"),
)


def _ftm_unit(defs, cond):
    return "".join(f"#define {d} 1\n" for d in defs) + f"#include <features.h>\n#if !({cond})\n#error \"{cond}\"\n#endif\n"


# L7: <bits/types.h>'s names are the ones the public typedefs are made of (a program may declare with either).
# The header alone first (<sys/types.h> defines some of the same names), then the public names after it.
BITS_TYPES_UNIT = """#include <bits/types.h>
extern __off_t niji_gm_o; extern __uid_t niji_gm_u; extern __pid_t niji_gm_p;
extern __int64_t niji_gm_q; extern __uint32_t niji_gm_w; extern __blksize_t niji_gm_b;
#include <sys/types.h>
extern off_t niji_gm_o; extern uid_t niji_gm_u; extern pid_t niji_gm_p;
"""
# L9: the _NP mutex kinds are enumerators (no macros) with the enumerators.tsv values, equal to the POSIX names.
MUTEX_UNIT = """#include <pthread.h>
#if defined PTHREAD_MUTEX_RECURSIVE_NP || defined PTHREAD_MUTEX_ERRORCHECK_NP
#error "a mutex kind _NP name is a macro"
#endif
typedef char niji_gm_np[PTHREAD_MUTEX_RECURSIVE_NP == 1 && PTHREAD_MUTEX_ERRORCHECK_NP == 2 ? 1 : -1];
"""
MUTEX_GNU = """typedef char niji_gm_npg[PTHREAD_MUTEX_FAST_NP == PTHREAD_MUTEX_NORMAL
  && PTHREAD_MUTEX_RECURSIVE_NP == PTHREAD_MUTEX_RECURSIVE ? 1 : -1];
"""
# L11: __exp10 is declared where visibility.tsv shows it, C++ included (a function of double).
EXP10_UNIT = "#include <math.h>\ndouble (*niji_gm_e10) (double) = __exp10;\n"


def units(ctx):
    """No spec clause beyond the reference's headers (CONFIGURE-DIFF rows; HEADERS-STAGE2-GAPS-M):
    every configuration: (1) the __stub_ markers after <limits.h>; (2) __GNUC_PREREQ and __glibc_clang_prereq in #if
    and in constant expressions; (3) the four GNU interface versions; (4) d_fileno a macro naming d_ino; (5) __UT_*
    with UT_*'s values and struct utmpx's array sizes. Then (6) _REENTRANT / _THREAD_SAFE and ISO C macros in GNU modes:
    the POSIX level and _POSIX_C_SOURCE they give; (7) <bits/types.h> names the public typedefs' types; (8) the _NP
    mutex kinds; (9) __exp10 in C++ (the results __wur checks: problems() below, a full compile).
    Defects: AC_CHECK_FUNC finding a stub, gnulib's cdefs test failing, a package using its own glob over libc's or the
    reverse, d_fileno missing, procps's __UT_HOSTSIZE test failing, _REENTRANT turning the default set off or lowering
    a level, bits/types.h missing or its __off_t another type, an _NP kind a macro or another value, __exp10 hidden from
    C++."""
    shown = {}
    for r in ctx.vis:
        shown.setdefault((r["header"], r["name"]), set()).update(expand(r["configs"], ctx.ids))
    out = []
    for cid in ctx.ids:
        out += [(cid, "GAPS-M stubs", STUB_UNIT), (cid, "GAPS-M gnuc prereq", PREREQ_UNIT),
                (cid, "GAPS-M gnu-versions", VERSIONS_UNIT), (cid, "GAPS-M dirent d_fileno", DIRENT_UNIT),
                (cid, "GAPS-M utmpx sizes", _utmpx_unit(ctx))]
    out += [(cid, f"GAPS-M ftm {'+'.join(defs)}", _ftm_unit(defs, cond)) for cid, defs, cond in FTM_CASES if cid in ctx.ids]
    out += [(cid, "GAPS-M bits/types.h", BITS_TYPES_UNIT) for cid in ("c17", "gnu17", "cxx98", "cxx17")
            if cid in shown.get(("bits/types.h", "__off_t"), ())]
    out += [(cid, "GAPS-M pthread _NP kinds", MUTEX_UNIT + (MUTEX_GNU if cid in ("gnu", "cxx17") else ""))
            for cid in ("c17", "gnu", "cxx17") if cid in shown.get(("pthread.h", "PTHREAD_MUTEX_RECURSIVE_NP"), ())]
    out += [(cid, "GAPS-M math.h __exp10", EXP10_UNIT) for cid in ("gnu", "cxx98", "cxx17")
            if cid in shown.get(("math.h", "__exp10"), ())]
    return out


# problems(): compiles that must fail or warn, as a program sees its headers (no -Wsystem-headers).
# L13 (#8815): _BSD_SOURCE / _SVID_SOURCE without _DEFAULT_SOURCE give the deprecation #warning in C and C++, whatever
# else is defined; with _DEFAULT_SOURCE none. (language, flags, warned)
DEPRECATION = (
    ("c", "-std=c17 -D_BSD_SOURCE", True),
    ("c", "-std=gnu17 -D_SVID_SOURCE", True),
    ("c", "-std=gnu17 -D_GNU_SOURCE -D_BSD_SOURCE", True),
    ("c", "-std=c17 -D_POSIX_C_SOURCE=200809L -D_SVID_SOURCE", True),
    ("c++", "-std=c++17 -D_BSD_SOURCE", True),
    ("c", "-std=c17 -D_BSD_SOURCE -D_DEFAULT_SOURCE", False),
    ("c", "-std=gnu17 -D_SVID_SOURCE -D_BSD_SOURCE -D_DEFAULT_SOURCE", False),
)
# L2: GNU strerror_r's result and __wur's are not checked without _FORTIFY_SOURCE (the configure probes compile
# -Werror, no -O); with it they are, and __attribute_warn_unused_result__'s always. GCC reports an unused result after
# parsing, so these compile to an object (-fsyntax-only shows none). (language, flags, unit)
WUR_FREE = (
    ("c", "-std=gnu17 -D_GNU_SOURCE", "#include <string.h>\nextern int niji_gm_r (void) __wur;\n"
     "void niji_gm_f (void) { char b[16]; strerror_r (1, b, sizeof b); niji_gm_r (); }\n"),
    ("c++", "-std=c++17", "#include <string.h>\nextern int niji_gm_r (void) __wur;\n"
     "void niji_gm_f (void) { char b[16]; strerror_r (1, b, sizeof b); niji_gm_r (); }\n"),
    # XSI strerror_r (the headers/redirects.tsv form, bound to __xpg_strerror_r): unchecked under fortification too.
    ("c", "-std=c17 -D_POSIX_C_SOURCE=200809L -O2 -D_FORTIFY_SOURCE=2", "#include <string.h>\n"
     "void niji_gm_f (void) { char b[16]; strerror_r (1, b, sizeof b); }\n"),
)
WUR_CHECKED = (
    ("c", "-std=gnu17 -O2 -D_FORTIFY_SOURCE=2", "#include <sys/cdefs.h>\nextern int niji_gm_r (void) __wur;\n"
     "void niji_gm_f (void) { niji_gm_r (); }\n"),
    ("c", "-std=gnu17", "#include <sys/cdefs.h>\nextern int niji_gm_r (void) __attribute_warn_unused_result__;\n"
     "void niji_gm_f (void) { niji_gm_r (); }\n"),
    # diagattrs.tsv wur `fortify` on libc's own declarations.
    ("c", "-std=gnu17 -O2 -D_FORTIFY_SOURCE=2", "#include <unistd.h>\n"
     "void niji_gm_f (void) { char b[1]; read (0, b, 1); chdir (\"/\"); }\n"),
)


def _cc(ctx, lang, flags, body, work, obj=False):
    cc = "g++" if lang == "c++" else "gcc"
    mode = ["-c", "-o", os.path.join(work, "gm.o")] if obj else ["-fsyntax-only"]
    cmd = [cc, *flags.split(), *mode, "-Wall", "-Wextra", "-Werror"]
    if ctx.inc:
        own = subprocess.run([cc, "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
        cmd += ["-nostdinc", "-isystem", ctx.inc, "-isystem", own, *kernel_flags(work)]
    src = os.path.join(work, "gm.cc" if lang == "c++" else "gm.c")
    with open(src, "w") as f:
        f.write(body)
    r = subprocess.run(cmd + [src], capture_output=True, text=True, timeout=300)
    return r.returncode, r.stderr


def problems(ctx):
    """No spec clause beyond feature_test_macros(7) and the reference's headers (HEADERS-STAGE2-GAPS-M, #8815): (1) the
    _BSD_SOURCE / _SVID_SOURCE deprecation warning per combination; (2) GNU strerror_r's and __wur's results unchecked
    without _FORTIFY_SOURCE, checked with it, __attribute_warn_unused_result__'s always; (3) <signal.h> and <sys/signal.h> do not contain the text SIGTSTP (perl's
    Configure decides d_bsd by searching them) while <signal.h> still defines SIGTSTP. Defects: the warning missing
    (dhcpcd's configure) or given with _DEFAULT_SOURCE, a fortified build not checking results, perl's d_bsd answer."""
    out = []
    with tempfile.TemporaryDirectory(prefix="niji-gaps-m-") as work:
        for lang, flags, warned in DEPRECATION:
            rc, err = _cc(ctx, lang, flags, "#include <features.h>\nint niji_gm_tu;\n", work)
            got = rc != 0 and "deprecated" in err and "_DEFAULT_SOURCE" in err
            if got != warned or (not warned and rc != 0):
                out.append(f"GAPS-M deprecation warning [{lang} {flags}]: rc {rc}, expected "
                           f"{'the deprecation #warning' if warned else 'no diagnostic'}: {err.strip()[:300]}")
        for lang, flags, body in WUR_FREE:
            rc, err = _cc(ctx, lang, flags, body, work, obj=True)
            if rc != 0:
                out.append(f"GAPS-M wur unchecked [{lang} {flags}]: rc {rc}: {err.strip()[:300]}")
        for lang, flags, body in WUR_CHECKED:
            rc, err = _cc(ctx, lang, flags, body, work, obj=True)
            calls = [n for n in ("niji_gm_r", "read", "chdir", "strerror_r") if f" {n} (" in body]
            if rc == 0 or err.count("unused-result]") < len(calls):
                out.append(f"GAPS-M wur checked [{lang} {flags}]: rc {rc}, expected -Wunused-result: {err.strip()[:300]}")
        rc, err = _cc(ctx, "c", "-std=c17", "#include <signal.h>\nint niji_gm_s = SIGTSTP;\n", work)
        if rc != 0:
            out.append(f"GAPS-M SIGTSTP defined: rc {rc}: {err.strip()[:300]}")
    base = ctx.inc or "/usr/include"
    for h in ("signal.h", "sys/signal.h"):
        p = os.path.join(base, h)
        if not os.path.isfile(p):
            p = os.path.join("/usr/include/x86_64-linux-gnu", h) if not ctx.inc else p
        with open(p, encoding="utf-8", errors="replace") as f:
            if "SIGTSTP" in f.read():
                out.append(f"GAPS-M SIGTSTP text: {h} contains the text SIGTSTP (perl's d_bsd test)")
    return out


if __name__ == "__main__":
    import types
    import csv
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "..")

    def tsv(rel):
        with open(os.path.join(root, rel), newline="") as f:
            return list(csv.DictReader((x for x in f if not x.startswith("#")), delimiter="\t"))
    cfgs = {r["id"]: r for r in tsv("spec/abi/ftm-configs.tsv")}
    ref = "--reference" in sys.argv
    ctx = types.SimpleNamespace(ids=list(cfgs), vis=tsv("spec/abi/visibility.tsv"), inc=None if ref else
                                os.path.abspath(os.path.join(root, "libc-include")),
                                macros={r["name"]: r for r in tsv("spec/abi/macros.tsv")})
    rejected = {r["id"] for r in tsv("headers/levels.tsv") if r["reject"] == "1"}
    fails = problems(ctx)
    with tempfile.TemporaryDirectory(prefix="niji-gaps-m-u-") as w:
        for cid, what, body in units(ctx):
            if cid in rejected:
                continue
            c = cfgs[cid]
            flags = c["flags"] + (" -Wno-cpp" if cid in ("bsd", "svid") else "")
            rc, err = _cc(ctx, c["language"], flags, body, w)
            if rc:
                fails.append(f"{what} {cid}: rc {rc}: {' | '.join(err.strip().splitlines()[:3])[:300]}")
    for f in fails:
        print(f"FAIL {f}")
    print(f"gaps-m: {len(fails)} failed")
    sys.exit(1 if fails else 0)
