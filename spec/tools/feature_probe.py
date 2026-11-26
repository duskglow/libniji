#!/usr/bin/env python3
"""Probe the reference's feature macros and the form of its macro definitions (HEADERS-FEATURES-FORMS).

    python3 spec/tools/feature_probe.py use   [--include DIR] [--out FILE] [--jobs N]
    python3 spec/tools/feature_probe.py forms [--include DIR] [--out FILE] [--jobs N] [--headers a.h,b.h]

Run on the reference system (Appendix C.0) to write the tables; `--include DIR` runs the same probe on libniji's
headers (`-nostdinc -isystem DIR -isystem <the compiler's own include>`) so the two outputs can be compared.

Clean room (plan/headers.md section 2.2, ruling R-H0): every unit is the probe's own, compiled `-fsyntax-only`;
the probe reads back only the exit status and the LINE NUMBERS of errors located in its own file. Each test is an
`#if` line followed by an `#error` line, so "the `#error` line has an error" means the test held and "the `#if`
line has an error" means the macro is not usable in `#if` there. No `-E` or `-dM` output and no header text is read.

* `use`: for each configuration (header_probe.CONFIGS and EXTRA below) and each candidate name (USE_NAMES), whether
  `<features.h>` defines it and its value: 0..3, `other` (an `#if`-usable value outside 0..3) or `form` (defined,
  not usable in `#if`, as an empty definition). Output columns `config`, `name`, `value` (`-` = undefined).
  Candidates are names software outside the C library tests: the R-H2 corpus scan (results/headers-corpus-*.tsv:
  `__USE_GNU`, `__USE_MISC`, `__USE_LARGEFILE64`, `__USE_FILE_OFFSET64`, `__USE_FORTIFY_LEVEL`,
  `__USE_EXTERN_INLINES`, `__USE_XOPEN2K`, `__USE_XOPEN2K8`, `__USE_POSIX2`, `__USE_ISOC99`, `__USE_ISOC11`,
  `__USE_TIME_BITS64`, ...), the kernel's UAPI headers (`__USE_TIME_BITS64`, `__USE_MISC`, `__USE_GNU`), npth's
  `<npth.h>` (`__USE_UNIX98`, `__USE_XOPEN2K`), and the same `__USE_` + level spelling for every other level of
  feature_test_macros(7) and POSIX XSH 2.2.1, so that the family is probed whole. A candidate the reference never
  defines is simply `-` everywhere.
* `forms`: for each (header, macro) visibility.tsv shows in configuration `gnu`, the unit defines the macro first
  to a sentinel, includes the header, and tests what is in force: `kept` (the program's definition), `replaced`
  (the header's, usable in `#if`), `replaced-form` (the header's, not usable in `#if`) or `undefined` (the header
  removed it). Output columns `header`, `name`, `result`. Each name is probed first in the first header (name
  order) that shows it; only a name not `kept` there is probed in every other header that shows it.
"""
import argparse, csv, os, re, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import header_probe  # noqa: E402

ROOT = header_probe.ROOT
EXTRA = [
    ("gnu89", "c", "-std=gnu89", "GNU C90 mode, no feature-test macro"),
    ("gnu99", "c", "-std=gnu99", "GNU C99 mode, no feature-test macro"),
    ("gnu11", "c", "-std=gnu11", "GNU C11 mode, no feature-test macro"),
    ("c89gnu", "c", "-std=c89 -D_GNU_SOURCE", "_GNU_SOURCE under strict C90"),
    ("c2xdefault", "c", "-std=c2x -D_DEFAULT_SOURCE", "_DEFAULT_SOURCE under strict C2X"),
    ("posixsource", "c", "-std=c17 -D_POSIX_SOURCE", "_POSIX_SOURCE"),
    ("atfile", "c", "-std=c17 -D_ATFILE_SOURCE", "_ATFILE_SOURCE under strict C17"),
    ("lfs", "c", "-std=c17 -D_LARGEFILE_SOURCE", "_LARGEFILE_SOURCE under strict C17"),
    ("lfs64strict", "c", "-std=c17 -D_LARGEFILE64_SOURCE", "_LARGEFILE64_SOURCE under strict C17"),
    ("fob64strict", "c", "-std=c17 -D_FILE_OFFSET_BITS=64", "_FILE_OFFSET_BITS=64 under strict C17"),
    ("fob32", "c", "-std=gnu17 -D_FILE_OFFSET_BITS=32", "_FILE_OFFSET_BITS=32"),
    ("time32", "c", "-std=gnu17 -D_FILE_OFFSET_BITS=64 -D_TIME_BITS=32", "_TIME_BITS=32"),
    ("isoc95", "c", "-std=c89 -D_ISOC95_SOURCE", "_ISOC95_SOURCE under C90"),
    ("isoc23", "c", "-std=c17 -D_ISOC23_SOURCE", "_ISOC23_SOURCE under C17"),
    ("c99posix", "c", "-std=c99 -D_POSIX_C_SOURCE=200112L", "POSIX.1-2001 under strict C99"),
    ("posixxopen", "c", "-std=c17 -D_POSIX_C_SOURCE=200809L -D_XOPEN_SOURCE=600", "both, different levels"),
    ("xopen700default", "c", "-std=c17 -D_XOPEN_SOURCE=700 -D_DEFAULT_SOURCE", "_XOPEN_SOURCE=700 and _DEFAULT_SOURCE"),
    ("posixdefault", "c", "-std=c17 -D_POSIX_C_SOURCE=200112L -D_DEFAULT_SOURCE", "_POSIX_C_SOURCE and _DEFAULT_SOURCE"),
    ("xopenext500", "c", "-std=c17 -D_XOPEN_SOURCE=500 -D_XOPEN_SOURCE_EXTENDED", "SUSv2 and _XOPEN_SOURCE_EXTENDED"),
    ("reentrantstrict", "c", "-std=c17 -D_REENTRANT", "_REENTRANT under strict C17"),
    ("threadsafe", "c", "-std=gnu17 -D_THREAD_SAFE", "_THREAD_SAFE"),
    ("o2", "c", "-std=gnu17 -O2", "-O2, no feature-test macro"),
    ("os", "c", "-std=gnu17 -Os", "-Os"),
    ("o2noinline", "c", "-std=gnu17 -O2 -fno-inline", "-O2 -fno-inline"),
    ("fortify2o0", "c", "-std=gnu17 -D_FORTIFY_SOURCE=2", "_FORTIFY_SOURCE=2 without optimization"),
    ("cxx17o2", "c++", "-std=c++17 -O2", "C++17 at -O2"),
    ("cxx11fob64", "c++", "-std=c++11 -D_FILE_OFFSET_BITS=64", "C++11, _FILE_OFFSET_BITS=64"),
]
LEVELS = ["POSIX", "POSIX2", "POSIX199309", "POSIX199506", "XOPEN", "XOPEN_EXTENDED", "UNIX98", "XOPEN2K", "XOPEN2KXSI",
          "XOPEN2K8", "XOPEN2K8XSI", "XOPEN2K24", "XOPEN2K24XSI", "POSIX2024", "ISOC95", "ISOC99", "ISOC11", "ISOC2X",
          "ISOC23", "ISOC2Y", "ISOCXX11", "ISOCXX", "LARGEFILE", "LARGEFILE64", "FILE_OFFSET64", "TIME_BITS64", "ATFILE",
          "MISC", "GNU", "BSD", "SVID", "REENTRANT", "DYNAMIC_STACK_SIZE", "FORTIFY_LEVEL", "EXTERN_INLINES",
          "STRING_INLINES", "POSIX_IMPLICITLY", "C2X_STRTOL", "ISOC2X_STRTOL", "LIB_EXT2", "IEC_60559_BFP_EXT",
          "IEC_60559_FUNCS_EXT", "IEC_60559_TYPES_EXT", "IEC_60559_EXT", "IEC_60559_BFP_EXT_C2X",
          "IEC_60559_FUNCS_EXT_C2X", "IEC_60559_TYPES_EXT_C2X", "XOPEN2K8_GNU"]
USE_NAMES = ["__USE_" + n for n in LEVELS]
SENTINEL = 987654321


def compiler(lang, include):
    cc = header_probe.COMPILER[lang]
    flags = [cc] + header_probe.BASE
    if include:
        own = subprocess.run([cc, "-print-file-name=include"], capture_output=True, text=True).stdout.strip()
        flags += ["-nostdinc", "-isystem", include, "-isystem", own]
    return flags


def error_lines(cmd, src, suffix):
    """Compile `src`; the set of line numbers of errors in the unit's own file."""
    with tempfile.TemporaryDirectory(prefix="niji-fp-") as d:
        path = os.path.join(d, "u" + suffix)
        with open(path, "w") as f:
            f.write(src)
        r = subprocess.run(cmd + [path], capture_output=True, text=True, timeout=300)
        pat = re.compile(r"^" + re.escape(path) + r":(\d+):(?:\d+:)? (?:fatal )?error")
        return {int(m.group(1)) for line in r.stderr.splitlines() for m in [pat.match(line)] if m}


def use_unit(cfg, include):
    cid, lang, flags, _ = cfg
    lines, tests = ["#include <features.h>"], []
    for n in USE_NAMES:
        lines += [f"#ifdef {n}", "#error", "#endif"]
        tests.append((n, "defined", len(lines) - 2))
        for v in range(4):
            lines += [f"#if defined {n} && ({n} * 1 + 0) == {v}", "#error", "#endif"]
            tests.append((n, v, len(lines) - 2))
    errs = error_lines(compiler(lang, include) + flags.split(), "\n".join(lines) + "\n", ".c" if lang == "c" else ".cc")
    out = []
    for n in USE_NAMES:
        mine = [(what, ln) for m, what, ln in tests if m == n]
        if mine[0][1] + 1 not in errs:
            out.append((cid, n, "-"))
            continue
        vals = [str(what) for what, ln in mine[1:] if ln + 1 in errs]
        form = any(ln in errs for _, ln in mine[1:])
        out.append((cid, n, "form" if form else (vals[0] if vals else "other")))
    return out


def forms_unit(header, name, include):
    src = (f"#define {name} {SENTINEL}\n#include <{header}>\n#ifndef {name}\n#error\n#elif ({name} * 1 + 0) == {SENTINEL}\n"
           "#error\n#endif\n")
    errs = error_lines(compiler("c", include) + ["-std=gnu17", "-D_GNU_SOURCE"], src, ".c")
    if 4 in errs:
        return "undefined"
    if 5 in errs:
        return "replaced-form"
    return "kept" if 6 in errs else "replaced"


def forms_batch(header, names, include, flags=("-std=gnu17", "-D_GNU_SOURCE")):
    """`forms_unit` for many names of one header in one unit: every name is defined to the sentinel first, then the
    header is included once and each name tested by its own `#ifndef` / `#elif` block. {name: result}. Only for
    names whose early definition does not change what the header declares (no include guard, no feature-test macro):
    header_checks/features.py batches so, and its `--reference` run checks the batched results against the one-name
    results of macro-forms.tsv."""
    lines = [f"#define {n} {SENTINEL}" for n in names] + [f"#include <{header}>"]
    at = {}
    for n in names:
        lines += [f"#ifndef {n}", "#error", f"#elif ({n} * 1 + 0) == {SENTINEL}", "#error", "#endif"]
        at[n] = len(lines) - 4
    errs = error_lines(compiler("c", include) + list(flags), "\n".join(lines) + "\n", ".c")
    out = {}
    for n, ln in at.items():
        out[n] = ("undefined" if ln + 1 in errs else "replaced-form" if ln + 2 in errs
                  else "kept" if ln + 3 in errs else "replaced")
    return out


def gnu_pairs(headers=None):
    """The (header, macro) pairs visibility.tsv shows in configuration `gnu`, sorted."""
    with open(os.path.join(header_probe.ABI, "visibility.tsv"), newline="") as f:
        vis = list(csv.DictReader(f, delimiter="\t"))
    ids = [c[0] for c in header_probe.CONFIGS]
    return sorted({(r["header"], r["name"]) for r in vis if r["kind"] == "macro" and r["header"] != "-"
                   and (headers is None or r["header"] in headers)
                   and "gnu" in header_probe_expand(r["configs"], ids)})


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("what", choices=["use", "forms"])
    ap.add_argument("--include")
    ap.add_argument("--out")
    ap.add_argument("--jobs", type=int, default=os.cpu_count())
    ap.add_argument("--headers")
    ap.add_argument("--compact", action="store_true", help="forms: write spec/abi/macro-forms.tsv's rows (see compact)")
    ap.add_argument("--compact-from", help="forms: compact this full forms output instead of probing")
    ap.add_argument("--pairs", help="forms: probe exactly the (header, name) rows of this TSV (another run's output)")
    a = ap.parse_args()
    inc = os.path.abspath(a.include) if a.include else None
    with ThreadPoolExecutor(a.jobs) as ex:
        if a.what == "use":
            rows = [r for rs in ex.map(lambda c: use_unit(c, inc), header_probe.CONFIGS + EXTRA) for r in rs]
            head = ["config", "name", "value"]
        else:
            with open(os.path.join(header_probe.ABI, "visibility.tsv"), newline="") as f:
                vis = list(csv.DictReader(f, delimiter="\t"))
            ids = [c[0] for c in header_probe.CONFIGS]
            want = set(a.headers.split(",")) if a.headers else None
            pairs = sorted({(r["header"], r["name"]) for r in vis if r["kind"] == "macro" and r["header"] != "-"
                            and (want is None or r["header"] in want)
                            and "gnu" in header_probe_expand(r["configs"], ids)})
            # Stage 1: each name in the first header (name order) that shows it; stage 2: every other header showing
            # a name whose first result is not `kept` (a name kept there is the same definition, guarded, elsewhere).
            if a.pairs:
                with open(a.pairs, newline="") as f:
                    one = [(r["header"], r["name"]) for r in csv.DictReader(f, delimiter="\t")]
                rows = [(h, n, r) for (h, n), r in zip(one, ex.map(lambda p: forms_unit(p[0], p[1], inc), one))]
                pairs = []
            first = {}
            for h, n in pairs:
                first.setdefault(n, h)
            one = sorted((h, n) for n, h in first.items())
            probe = (lambda p: "from-file") if a.compact_from else (lambda p: forms_unit(p[0], p[1], inc))
            res = dict(zip(one, ex.map(probe, one)))
            rest = [p for p in pairs if p not in res and res[(first[p[1]], p[1])] != "kept"]
            res.update(zip(rest, ex.map(probe, rest)))
            if not a.pairs:
                rows = sorted((h, n, r) for (h, n), r in res.items())
                if a.compact or a.compact_from:
                    rows = compact(rows, pairs, a.compact_from)
            head = ["header", "name", "result"]
    out = open(a.out, "w") if a.out else sys.stdout
    out.write("\t".join(head) + "\n")
    for r in rows:
        out.write("\t".join(map(str, r)) + "\n")


def compact(rows, pairs, full=None):
    """The rows of spec/abi/macro-forms.tsv: every probed pair whose result is not `replaced`, and each pair of a name
    kept in its first header that stage 2 did not probe, as `kept`. A pair visible under `_GNU_SOURCE` and absent
    from the table is `replaced` (the generator's rule). `full`: take the results from a full forms output."""
    if full:
        with open(full, newline="") as f:
            got = {(r["header"], r["name"]): r["result"] for r in csv.DictReader(f, delimiter="	")}
    else:
        got = {(h, n): r for h, n, r in rows}
    first = {}
    for h, n in pairs:
        first.setdefault(n, h)
    out = []
    for h, n in pairs:
        r = got.get((h, n)) or ("kept" if got.get((first[n], n)) == "kept" else None)
        if r is None:
            raise SystemExit(f"{h} {n}: not probed")
        if r != "replaced":
            out.append((h, n, r))
    return out


def header_probe_expand(configs, ids):
    if configs == "*":
        return set(ids)
    if configs.startswith("!"):
        return set(ids) - set(configs[1:].split(","))
    return set(configs.split(","))


if __name__ == "__main__":
    main()
