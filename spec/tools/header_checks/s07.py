"""Slice 7's own header check (plan/headers.md section 4.1), loaded by spec/tools/header_check.py, which lists this
directory.

`problems` (no spec clause; ruling #6406, SL-HEADERS-S07-TAIL): no generated header guards an include of a header
libniji ships. A `__has_include(<H>)` test around `#include <H>` was the stand-in while H had no template
(`headers/H.in`); once H is generated the include is plain, like every other include of a libniji header, so a
guard left behind means the generated header silently depends on whether H is found on the include path, and on a
system tree it would reach another C library's H.

The P1 of the macros slice 7's headers show but another header owns (`MAXSYMLINKS` in sys/param.h, `TIOCGWINSZ` in
sys/ioctl.h) is the shared table P1's now (header_check.py `p1_units`, HEADERS-GEN-SHARED), for every slice.
"""
import os
import re

HAS_INCLUDE = re.compile(r"__has_include\s*\(\s*<([^>]+)>\s*\)")


def units(ctx):
    """Spec clause: XSH <sys/stat.h>, the file type test macros (HEADERS-SNAPSHOT-GAPS). In every configuration that
    accepts <sys/stat.h>, each S_IS* test the header defines compiles on a mode, also where the S_IF* names are the
    program's (strict POSIX: a build that defined _POSIX_C_SOURCE 200809L failed on S_ISDIR)."""
    tests = ("S_ISBLK", "S_ISCHR", "S_ISDIR", "S_ISFIFO", "S_ISREG", "S_ISLNK", "S_ISSOCK")
    body = "#include <sys/stat.h>\n" + "".join(
        f"#ifdef {t}\nint niji_s07_{t.lower()} (unsigned int m) {{ return {t} (m); }}\n#endif\n" for t in tests)
    out = [(cid, "S_IS* tests of sys/stat.h", body + "int niji_s07_tu;\n") for cid in ctx.ids]
    # Spec clause: utmp(5), utmpx(5), paths.h's _PATH_* names (HEADERS-SNAPSHOT-GAPS): the utmp and wtmp path macros
    # in each header of the family, exactly where the reference shows them, with its values (tree's build used
    # _PATH_WTMP; a header that shows another family member is caught by the "not defined" side).
    # HEADERS-STAGE2-GAPS-C (bsd-mailx: _PATH_DEVNULL): paths.h's other path names, the BSD paths(3) set, the same way.
    for h in UTMP_HEADERS:
        out += ctx.exact(h, UTMP_PATHS + (BSD_PATHS(ctx) if h == "paths.h" else ()))
    # Spec clause: paths(3) / FHS 3.0: the null device's path, from the documents rather than the tables.
    out += [(cid, "S07 _PATH_DEVNULL", "#include <paths.h>\ntypedef char niji_s07_null[__builtin_strcmp (_PATH_DEVNULL, "
             "\"/dev/null\") == 0 ? 1 : -1];\nint niji_s07_pn;\n") for cid in ("c89", "gnu17", "cxx17")]
    # Spec clause: no spec clause (BSD <dirent.h>, man-pages readdir(3) d_type; HEADERS-STAGE2-GAPS-C, capnproto): DTTOIF
    # and IFTODT exactly where the reference shows them, and where shown: a d_type is the file-type bits of a mode
    # shifted down by 12 (DT_DIR 4 <-> S_IFDIR 0040000), the argument is parenthesised (`|` binds looser than `&` and
    # `<<`), the result has the promoted argument's type, and both work in `#if`.
    for h in ("dirent.h", "sys/dir.h"):
        out += ctx.exact(h, ("DTTOIF", "IFTODT"))
        shown = ctx.expand(next((r["configs"] for r in ctx.vis if r["header"] == h and r["name"] == "IFTODT"), "-"))
        body = (f"#include <{h}>\n"
                "#if IFTODT (0200000 | 0040000) != 4 || DTTOIF (4 | 0) != 0040000 || IFTODT (DTTOIF (10)) != 10\n"
                "#error \"IFTODT/DTTOIF in #if\"\n#endif\n"
                "typedef char niji_s07_dt[IFTODT (0200000 | 0100000) == 8 && DTTOIF (2 | 0) == 0020000"
                " && sizeof (IFTODT (0040000L)) == sizeof (long) && sizeof (DTTOIF ((char) 4)) == sizeof (int) ? 1 : -1];\n"
                "int niji_s07_dttu;\n")
        for lang in ("c", "c++"):
            cid = next((c["id"] for c in ctx.cfgs if c["language"] == lang and c["id"] in shown), None)
            if cid:
                out.append((cid, f"S07 {h} DTTOIF/IFTODT", body))
    # Spec clause: XBD <utime.h> (HEADERS-STAGE2-GAPS-C, bzip2, cmake): `struct utimbuf` is complete, with the members
    # `actime` and `modtime` of type time_t, and utime() takes a pointer to it.
    # (<utime.h> shows time_t only from POSIX 2008 on: the unit takes it from <time.h>.)
    ut = ("#include <utime.h>\n#include <time.h>\n"
          "typedef char niji_s07_ut[sizeof (struct utimbuf) == 2 * sizeof (time_t)"
          " && __builtin_offsetof (struct utimbuf, actime) == 0"
          " && __builtin_offsetof (struct utimbuf, modtime) == sizeof (time_t)"
          " && sizeof (((struct utimbuf *) 0)->modtime) == sizeof (time_t) ? 1 : -1];\n"
          "int (*niji_s07_utime) (const char *, const struct utimbuf *) = utime;\n")
    c_type = "typedef char niji_s07_utt[__builtin_types_compatible_p (__typeof__ (((struct utimbuf *) 0)->actime), time_t) ? 1 : -1];\n"
    cxx_type = "static_assert (__is_same (decltype (((utimbuf *) 0)->actime), time_t), \"actime\");\n"
    out += [(cid, "S07 utime.h utimbuf", ut + extra) for cid, extra in
            (("c89", c_type), ("gnu17", c_type), ("time64", c_type), ("cxx98", ""), ("cxx17", cxx_type))]
    return out


def BSD_PATHS(ctx):  # noqa: N802  (a constant spelled as a call: the list lives in header_probe)
    return tuple(n for n in ctx.hp.stage2_names()[0] if n.startswith("_PATH_") and n not in UTMP_PATHS)


UTMP_HEADERS = ("paths.h", "utmp.h", "utmpx.h", "lastlog.h")
UTMP_PATHS = ("_PATH_UTMP", "_PATH_WTMP", "_PATH_UTMPX", "_PATH_WTMPX", "UTMP_FILE", "WTMP_FILE", "UTMPX_FILE", "WTMPX_FILE")


def problems(ctx):
    out = []
    base = os.path.join(ctx.root, "libc-include")
    for d, _, files in sorted(os.walk(base)):
        for f in sorted(files):
            if not f.endswith(".h"):
                continue
            path = os.path.join(d, f)
            with open(path) as fh:
                for n, line in enumerate(fh, 1):
                    for inc in HAS_INCLUDE.findall(line):
                        if os.path.isfile(os.path.join(ctx.root, "headers", inc + ".in")):
                            rel = os.path.relpath(path, ctx.root)
                            out.append(f"{rel}:{n}: guards the include of <{inc}>, which libniji ships (headers/{inc}.in): include it plainly")
    return out
