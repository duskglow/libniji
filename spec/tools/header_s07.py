#!/usr/bin/env python3
"""Slice 7 header checks beyond header_check.py (plan/headers.md slice 7; SL-HEADERS-S07-3).

    python3 spec/tools/header_s07.py [--include DIR]

Every unit is compiled with `-nostdinc -isystem DIR -isystem <the compiler's own include>` (no reference header is read).

* stat-old   below POSIX 2008 (`-std=c99`, `-D_POSIX_C_SOURCE=200112L`, `-D_XOPEN_SOURCE=600`) `struct timespec` is no name of
             sys/stat.h, fcntl.h, sys/fcntl.h, sys/file.h (the unit defines its own), `struct stat` has `st_atime` and
             `st_atimensec` (and the mtime and ctime pairs) at the offsets of the timespec members, and sizeof 144.
* stat-new   from POSIX 2008 on (`-D_POSIX_C_SOURCE=200809L`, the default) the members are `st_atim`, `st_mtim`, `st_ctim`, the
             macros `st_atime`... name their seconds, the size is the same.
* ctype-masks every class mask the C classification macros expand to is declared (the twelve `_IS*` constants, the table's
             little-endian bit order) and each macro compiles; found by P7 (a package using `isalpha`).
* fob64      under `-D_GNU_SOURCE -D_FILE_OFFSET_BITS=64` a program taking the address of each function below references its
             64-bit twin, without `_FILE_OFFSET_BITS` the function itself (the symbols the reference's headers bind to).

Prints one FAIL line per failing unit and `header_s07: N failed of M`; exit 1 when any failed.
"""
import argparse
import os
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

HEADERS = ["sys/stat.h", "fcntl.h", "sys/fcntl.h", "sys/file.h"]
OLD = [["-std=c99"], ["-std=c99", "-D_POSIX_C_SOURCE=200112L"], ["-std=c99", "-D_XOPEN_SOURCE=600"]]
NEW = [["-std=c99", "-D_POSIX_C_SOURCE=200809L"], ["-std=gnu17"], ["-std=c99", "-D_XOPEN_SOURCE=700"]]

# (name, header(s) that declare it, the symbol a 64-bit file offset binds to)
FOB64 = [
    ("fallocate", ["fcntl.h"], "fallocate64"), ("lockf", ["unistd.h"], "lockf64"), ("prlimit", ["sys/resource.h"], "prlimit64"),
    ("preadv2", ["sys/uio.h"], "preadv64v2"), ("pwritev2", ["sys/uio.h"], "pwritev64v2"),
    ("scandirat", ["dirent.h"], "scandirat64"), ("versionsort", ["dirent.h"], "versionsort64"),
    ("lstat", ["sys/stat.h"], "lstat64"), ("stat", ["sys/stat.h"], "stat64"), ("fseeko", ["stdio.h"], "fseeko64"),
    ("ftello", ["stdio.h"], "ftello64"), ("mkstemp", ["stdlib.h"], "mkstemp64"), ("mkostemp", ["stdlib.h"], "mkostemp64"),
    ("mkostemps", ["stdlib.h"], "mkostemps64"), ("ftruncate", ["unistd.h"], "ftruncate64"), ("truncate", ["unistd.h"], "truncate64"),
    ("pwrite", ["unistd.h"], "pwrite64"), ("open", ["fcntl.h"], "open64"), ("fopen", ["stdio.h"], "fopen64"),
]

OLD_UNIT = """#include <{h}>
struct timespec {{ int niji_own; }};
"""
OLD_STAT = """#include <stddef.h>
_Static_assert(sizeof (struct stat) == 144, "struct stat size");
_Static_assert(offsetof (struct stat, st_atimensec) == offsetof (struct stat, st_atime) + 8, "atime pair");
_Static_assert(offsetof (struct stat, st_mtime) == offsetof (struct stat, st_atime) + 16, "mtime");
_Static_assert(offsetof (struct stat, st_mtimensec) == offsetof (struct stat, st_mtime) + 8, "mtime pair");
_Static_assert(offsetof (struct stat, st_ctimensec) == offsetof (struct stat, st_ctime) + 8, "ctime pair");
"""

NEW_UNIT = """#include <sys/stat.h>
#include <stddef.h>
_Static_assert(sizeof (struct stat) == 144, "struct stat size");
_Static_assert(offsetof (struct stat, st_mtim) == offsetof (struct stat, st_atim) + 16, "mtim");
_Static_assert(offsetof (struct stat, st_ctim) == offsetof (struct stat, st_atim) + 32, "ctim");
long niji_sec (struct stat *s) {{ return s->st_atime + s->st_mtime + s->st_ctime + s->st_atim.tv_nsec; }}
"""


CTYPE_MASKS = [("upper", 256), ("lower", 512), ("alpha", 1024), ("digit", 2048), ("xdigit", 4096), ("space", 8192),
               ("print", 16384), ("graph", 32768), ("blank", 1), ("cntrl", 2), ("punct", 4), ("alnum", 8)]
CTYPE_UNIT = "#include <ctype.h>\n" + "".join(f'_Static_assert(_IS{n} == {v}, "_IS{n}");\n' for n, v in CTYPE_MASKS) + (
    "int niji_all (int c) { return " + " + ".join(f"is{n}(c)" for n, _ in CTYPE_MASKS) + "; }\n")


def fob_unit(name, headers):
    inc = "".join(f"#include <{h}>\n" for h in ["sys/types.h", *headers])
    return inc + f"const void *niji_addr = (const void *) {name};\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    args = ap.parse_args()
    inc = os.path.abspath(args.include)
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    work = tempfile.mkdtemp(prefix="header_s07.")
    base = ["gcc", "-nostdinc", "-isystem", inc, "-isystem", gccinc]
    fails = total = 0

    def run(what, flags, text, check=None, asm=False):
        nonlocal fails, total
        src = os.path.join(work, f"u{total}.c")
        with open(src, "w") as f:
            f.write(text)
        total += 1
        cmd = base + flags + (["-S", "-o", "-"] if asm else ["-fsyntax-only"]) + [src]
        r = subprocess.run(cmd, capture_output=True, text=True)
        ok = r.returncode == 0 and (check is None or check(r.stdout))
        if not ok:
            fails += 1
            msg = " | ".join(r.stderr.strip().splitlines()[:2])[:300] if r.returncode else "wrong symbol"
            print(f"FAIL {what} {' '.join(flags)}: {msg}")

    for flags in OLD:
        for h in HEADERS:
            # struct stat itself is a name of sys/stat.h only, below POSIX 2008 and above.
            run(f"stat-old {h}", flags, OLD_UNIT.format(h=h) + (OLD_STAT if h == "sys/stat.h" else ""))
    for flags in NEW:
        run("stat-new", flags, NEW_UNIT)
    for flags in ([], ["-O2"], ["-D_GNU_SOURCE"]):
        run("ctype-masks", flags, CTYPE_UNIT)
    for name, headers, twin in FOB64:
        text = fob_unit(name, headers)
        words = lambda out: {w.strip(",") for l in out.splitlines() for w in l.replace("\t", " ").split()}
        run(f"fob64 {name}", ["-D_GNU_SOURCE", "-D_FILE_OFFSET_BITS=64"], text, lambda out, t=twin: t in words(out), asm=True)
        run(f"plain {name}", ["-D_GNU_SOURCE"], text, lambda out, n=name, t=twin: n in words(out) and t not in words(out), asm=True)
    print(f"header_s07: {fails} failed of {total}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
