"""Feature-test macros and macro definition forms (HEADERS-FEATURES-FORMS; Yohane stage 2 on snapshot 25.2).

Spec: feature_test_macros(7); the reference's `__USE_*` names, which software outside the C library tests (npth's
<npth.h>, the kernel UAPI headers, R-H2's corpus); black-box probes `spec/tools/feature_probe.py` (tables
spec/abi/use-macros.tsv and spec/abi/macro-forms.tsv). Loaded by spec/tools/header_check.py.

* `units(ctx)` (the harness's flags: -Wall -Wextra -Wsystem-headers -Werror):
  - `FEATURES use <cfg>`: per configuration of ftm-configs.tsv, <features.h> defines exactly the reference's `__USE_*`
    names, with its values; a name the reference leaves undefined is undefined;
  - `FEATURES glibc version`: __GNU_LIBRARY__, __GLIBC__, __GLIBC_MINOR__ and __GLIBC_PREREQ;
  - `FEATURES pkg <package> ...`: the patterns of the packages that failed on 25.2 (npth's `__USE_UNIX98` /
    `__USE_XOPEN2K` test, less's `__USE_GNU` defined before <regex.h>, python's `struct f_owner_ex`, util-linux's
    TIMESPEC_TO_TIMEVAL from every header that shows it at once, fstrm's SUN_LEN, openssl's SOL_TCP).
* `problems(ctx)` (own compiles, the harness's flags without -Wsystem-headers, as a program sees its headers):
  - `FEATURES use <cfg>` for the feature_probe.EXTRA configurations (-std=gnu89, _ISOC23_SOURCE, _TIME_BITS=32, ...);
  - `FEATURES flex SIZE_MAX c|c++`: a program's own `SIZE_MAX (~(size_t)0)` before <stdint.h> is replaced, silently;
  - `FEATURES forms <header>`: every macro visibility.tsv shows there under _GNU_SOURCE, defined first to a sentinel
    (feature_probe.forms_batch, one unit per header), is kept or replaced as macro-forms.tsv says (a pair absent from
    it: replaced). `replaced-form` counts as replaced: whether the definition is usable in `#if` is form.py's check.
    Names in KNOWN (measured differences of hand-written template text, a follow-up) and include guards are left out.

Stand-alone (plants): python3 spec/tools/header_checks/features.py [--include DIR | --reference] [--only REGEX]
`--reference` runs every check against the build host's own C library headers: the expectations are the reference's,
so it must pass there too.
"""
import argparse
import csv
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import feature_probe as fp  # noqa: E402
import header_probe as hp  # noqa: E402

# Guard mismatches measured against the reference (feature_probe.py forms, 2026-11-25) in template text the generator
# rule does not reach yet: the feature-test macros features.h sets under _GNU_SOURCE, macros of hand-written blocks, and
# include guards / macros the reference gets from an included header. Follow-up in handoff/HEADERS-FEATURES-FORMS.md.
KNOWN = {
    "_ATFILE_SOURCE", "_DEFAULT_SOURCE", "_DYNAMIC_STACK_SIZE_SOURCE", "_ISOC11_SOURCE", "_ISOC2X_SOURCE",
    "_ISOC95_SOURCE", "_ISOC99_SOURCE", "_LARGEFILE64_SOURCE", "_LARGEFILE_SOURCE", "_POSIX_C_SOURCE", "_POSIX_SOURCE",
    "_XOPEN_SOURCE", "_XOPEN_SOURCE_EXTENDED", "__WORDSIZE_TIME64_COMPAT32", "_SIGSET_NWORDS", "NULL",
    "IPV6_RECVORIGDSTADDR", "IP_RECVORIGDSTADDR", "IP_RECVRETOPTS", "TEMP_FAILURE_RETRY", "FD_SET", "XATTR_CREATE",
    *(f"__is{c}_l" for c in ("alnum", "alpha", "ascii", "blank", "cntrl", "digit", "graph", "lower", "print", "punct",
                             "space", "upper", "xdigit")), "__toascii_l",
    *(f"MM_NULL{s}" for s in ("ACT", "LBL", "MC", "SEV", "TAG", "TXT")),
    *(f"si_{s}" for s in ("arch", "call_addr", "lower", "pkey", "status", "syscall", "upper", "value")),
    "DEFTAPE", "FIOQSIZE", "PTHREAD_STACK_MIN", "RE_TRANSLATE_TYPE", "R_OK", "TIOCSRS485", "_IOC_DIRBITS", "_IOC_NONE",
    "_IOC_READ", "_IOC_SIZEBITS", "_IOC_WRITE", "_PTY_H", "_SYS_IOCTL_H", "_SYS_MOUNT_H", "_TERMIOS_H",
}

# Headers where the reference keeps a program's definition of names other headers replace (it gets them through a
# header that guards them; libniji defines them outright there): the macros the reference keeps are left out there.
KNOWN_KEPT = {"net/if_ppp.h", "net/if_shaper.h", "sys/mount.h", "sys/mtio.h", "sys/raw.h"}
# An include guard: defining it first makes the reference skip the header, which is not a form question.
GUARD = re.compile(r"^_[A-Z0-9_]*_H$")

GLIBC = """#include <features.h>
#if __GNU_LIBRARY__ != 6 || __GLIBC__ != 2 || __GLIBC_MINOR__ != 39
#error "version macros"
#endif
#if !__GLIBC_PREREQ (2, 39) || __GLIBC_PREREQ (2, 40) || !__GLIBC_PREREQ (1, 99) || __GLIBC_PREREQ (3, 0)
#error "__GLIBC_PREREQ"
#endif
"""
# (configurations, title, unit): the 25.2 failures, each as the package's own source has it.
PACKAGES = [
    (("gnu17", "gnu"), "npth rwlock", "#include <pthread.h>\n#if defined(__USE_UNIX98) || defined(__USE_XOPEN2K)\n"
     "typedef pthread_rwlock_t niji_npth_rwlock_t;\nniji_npth_rwlock_t niji_rw;\n#else\n"
     "#error \"npth.h would leave npth_rwlock_t undeclared\"\n#endif\n"),
    (("gnu17",), "less regex", "#include <stdio.h>\n#define __USE_GNU 1\n#include <regex.h>\n"
     "int niji_less (void);\nint niji_less (void) { return (int) re_set_syntax (0); }\n"),
    (("gnu", "cxx17"), "python f_owner_ex", "#include <fcntl.h>\nstruct f_owner_ex niji_owner = { F_OWNER_TID, 0 };\n"
     "int niji_cmds[] = { F_GETOWN_EX, F_SETOWN_EX, F_OWNER_PID, F_OWNER_PGRP };\n"),
    (("gnu", "cxx17"), "util-linux timespec", "#include <sys/time.h>\n"
     "void niji_tv (struct timeval *tv, struct timespec *ts);\n"
     "void niji_tv (struct timeval *tv, struct timespec *ts) { TIMESPEC_TO_TIMEVAL (tv, ts); TIMEVAL_TO_TIMESPEC (tv, ts); }\n"),
    # Every header that shows them, at once: their definitions are token-identical (no redefinition diagnostic).
    (("gnu", "cxx17"), "timespec every header", "#include <utmp.h>\n#include <utmpx.h>\n#include <sys/timex.h>\n"
     "#include <sys/procfs.h>\n#include <sys/profil.h>\n#include <sys/time.h>\n"
     "void niji_tv (struct timeval *tv, struct timespec *ts);\n"
     "void niji_tv (struct timeval *tv, struct timespec *ts) { TIMESPEC_TO_TIMEVAL (tv, ts); TIMEVAL_TO_TIMESPEC (tv, ts); }\n"),
    (("default", "gnu17"), "fstrm SUN_LEN", "#include <sys/un.h>\nunsigned long niji_len (struct sockaddr_un *a);\n"
     "unsigned long niji_len (struct sockaddr_un *a) { return SUN_LEN (a); }\n"),
    (("gnu17",), "openssl SOL_TCP", "#include <sys/socket.h>\n#include <netinet/in.h>\n#include <netinet/tcp.h>\n"
     "int niji_opt[] = { SOL_TCP, TCP_NODELAY };\n"),
]
# flex's scanners define SIZE_MAX before <stdint.h> (gcc-14's gengtype-lex.cc); the reference's definition is the one
# in force afterwards, with no diagnostic unless -Wsystem-headers.
FLEX = "#define SIZE_MAX (~(size_t)0)\n#include <stdint.h>\n#if SIZE_MAX > 0xffffffffUL\nint niji_wide;\n#endif\n"


def _tsv(ctx, rel):
    with open(os.path.join(ctx.root, rel), newline="") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def _use_expect(ctx):
    exp = {}
    for r in _tsv(ctx, "spec/abi/use-macros.tsv"):
        exp.setdefault(r["config"], {})[r["name"]] = r["value"]
    return exp


def _use_unit(values):
    lines = ["#include <features.h>"]
    for n, v in sorted(values.items()):
        if v == "-":
            lines += [f"#ifdef {n}", f"#error \"{n} is defined\"", "#endif"]
        elif v.isdigit():
            lines += [f"#if !defined {n} || {n} != {v}", f"#error \"{n} is not {v}\"", "#endif"]
        else:
            lines += [f"#ifndef {n}", f"#error \"{n} is not defined\"", "#endif"]
    return "\n".join(lines) + "\n"


def units(ctx):
    exp = _use_expect(ctx)
    res = [(cid, f"FEATURES use {cid}", _use_unit(exp[cid])) for cid in ctx.ids if cid in exp]
    res += [(cid, "FEATURES glibc version", GLIBC) for cid in ("c89", "gnu", "cxx98")]
    res += [(cid, f"FEATURES pkg {what}", body) for cids, what, body in PACKAGES for cid in cids]
    return res


def _compile(ctx, lang, flags, body, sysw):
    cc = hp.COMPILER[lang]
    cmd = [cc, *flags, "-fsyntax-only", "-Wall", "-Wextra", "-Werror", *(["-Wsystem-headers"] if sysw else [])]
    if ctx.inc:
        own = subprocess.run([cc, "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
        cmd += ["-nostdinc", "-isystem", ctx.inc, "-isystem", own]
    else:
        cmd.append("-Wno-cpp")  # the reference's own `#warning` for _BSD_SOURCE / _SVID_SOURCE (deprecated)
    with tempfile.TemporaryDirectory(prefix="niji-features-") as d:
        src = os.path.join(d, "u.c" if lang == "c" else "u.cc")
        with open(src, "w") as f:
            f.write(body)
        r = subprocess.run(cmd + [src], capture_output=True, text=True, timeout=300)
    return r.returncode, " | ".join(r.stderr.strip().splitlines()[:3])[:400]


def _forms(ctx, only):
    """FAIL lines of the forms check, and a report line."""
    expect = {(r["header"], r["name"]): ("kept" if r["result"] == "kept" else "replaced")
              for r in _tsv(ctx, "spec/abi/macro-forms.tsv")}
    # Two batches per header, the names the reference keeps and those it replaces: an early definition must not change
    # what the header declares, so include guards (`_STDIO_H`: the reference skips the header) and KNOWN (the
    # feature-test macros among them) stay out.
    per = {}
    for h, n in fp.gnu_pairs():
        want = expect.get((h, n), "replaced")
        if n not in KNOWN and not GUARD.match(n) and not (h in KNOWN_KEPT and want == "kept"):
            per.setdefault((h, want), []).append(n)
    keys = [k for k in sorted(per) if not only or only.search(f"FEATURES forms {k[0]}")]
    ctx.selected += len(keys)
    with ThreadPoolExecutor(min(16, os.cpu_count() or 2)) as ex:
        got = list(ex.map(lambda k: fp.forms_batch(k[0], per[k], ctx.inc), keys))
    merged = {}
    for k, res in zip(keys, got):
        merged.setdefault(k[0], {}).update(res)
    hs = sorted(merged)
    fails = []
    for h, res in ((h, merged[h]) for h in hs):
        bad = []
        for n, r in sorted(res.items()):
            r = "replaced" if r == "replaced-form" else r
            if r != expect.get((h, n), "replaced"):
                bad.append(f"{n} {r}, reference {expect.get((h, n), 'replaced')}")
        if bad:
            fails.append(f"FEATURES forms {h}: {len(bad)} differ: {'; '.join(bad[:8])}")
    print(f"features: forms {len(hs)} headers, {sum(len(v) for v in merged.values())} pairs, {len(fails)} headers "
          f"differ (KNOWN names and include guards not probed)")
    return fails


def problems(ctx, only=None):
    out = []
    exp = _use_expect(ctx)
    ctx.selected = 0
    for cfg in fp.EXTRA:
        cid, lang, flags, _ = cfg
        if cid not in exp or (only and not only.search(f"FEATURES use {cid}")):
            continue
        ctx.selected += 1
        got = {n: v for _, n, v in fp.use_unit(cfg, ctx.inc)}
        bad = [f"{n} {got.get(n, '-')} (reference {v})" for n, v in sorted(exp[cid].items()) if got.get(n, "-") != v]
        if bad:
            out.append(f"FEATURES use {cid}: {'; '.join(bad[:8])}")
    for lang, flags in (("c", ["-std=gnu17"]), ("c++", ["-std=c++17"])):
        if only and not only.search(f"FEATURES flex SIZE_MAX {lang}"):
            continue
        ctx.selected += 1
        rc, err = _compile(ctx, lang, flags, FLEX, False)
        if rc:
            out.append(f"FEATURES flex SIZE_MAX {lang}: rc {rc}: {err}")
    if not only or only.search("FEATURES forms"):
        out += _forms(ctx, only)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--include", default=os.path.join(hp.ROOT, "libc-include"))
    ap.add_argument("--reference", action="store_true", help="the build host's own C library headers")
    ap.add_argument("--only", default="", metavar="REGEX")
    a = ap.parse_args()
    only = re.compile(a.only) if a.only else None
    import types
    with open(os.path.join(hp.ROOT, "spec/abi/ftm-configs.tsv"), newline="") as f:
        cfgs = {r["id"]: r for r in csv.DictReader(f, delimiter="\t")}
    ctx = types.SimpleNamespace(root=hp.ROOT, ids=list(cfgs), inc=None if a.reference else os.path.abspath(a.include))
    fails = problems(ctx, only)
    total = ctx.selected
    with open(os.path.join(hp.ROOT, "headers/levels.tsv"), newline="") as f:
        rejected = {r["id"] for r in csv.DictReader((x for x in f if not x.startswith("#")), delimiter="\t")
                    if r["reject"] == "1"}
    for cid, what, body in units(ctx):
        # As the harness: no unit in a configuration the headers reject (_TIME_BITS=64 alone); -Wsystem-headers on
        # libniji's tree only (the reference warns of its own deprecated _BSD_SOURCE / _SVID_SOURCE).
        if (only and not only.search(what)) or cid in rejected:
            continue
        c = cfgs[cid]
        rc, err = _compile(ctx, c["language"], c["flags"].split(), body, not a.reference)
        total += 1
        if rc:
            fails.append(f"{what} {cid}: rc {rc}: {err}")
    for f in fails:
        print(f"FAIL {f}")
    if only and total == 0:
        print(f"FAIL --only {a.only}: nothing selected")
        return 1
    print(f"features: {len(fails)} failed")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
