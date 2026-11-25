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
    # Spec clause: select(2), the BSD fd_set word names (HEADERS-STAGE2-GAPS-E: libev's ev_select.c sizes its sets in
    # fd_mask words of NFDBITS bits): each exactly where the reference shows it, fd_mask with its C++ spelling (long);
    # the values and sizes are sys/select.h's table P1.
    out += ctx.exact("sys/select.h", FD_WORD_NAMES)
    # The reserved `__u_char` family through <sys/wait.h>'s includes (HEADERS-STAGE2-GAPS-E): the reference shows it in
    # the strict modes too, where <sys/wait.h> does not bring in <signal.h> (a generator that counted <signal.h>'s names
    # as supplied in every configuration left them out there).
    for h in ("sys/wait.h", "wait.h"):
        out += ctx.exact(h, WAIT_U_CHAR_FAMILY)
    # Spec clause: select(2), FD_SET/FD_CLR/FD_ISSET/FD_ZERO; D-11.4 (the word index goes through __fdelt_chk). Every
    # header that defines the fd_set macros itself spells them as <sys/types.h> does, so either include order gives one
    # definition: each one included before and after <sys/types.h> compiles without a redefinition (-Wsystem-headers
    # -Werror) and the macros expand on a set (HEADERS-STAGE2-GAPS-E: <utmp.h> includes <sys/time.h> first). The guards
    # hide a drifted text from that, so in C++11 and later the expansion of FD_SET, FD_CLR and FD_ISSET, whichever
    # header supplied it, must name __fdelt_chk (a constexpr search of the stringified expansion).
    use = ("#ifdef FD_SET\nint niji_s07_fd (fd_set *s) { FD_ZERO (s); FD_SET (3, s); FD_CLR (3, s); "
           "return FD_ISSET (3, s); }\n#endif\nint niji_s07_tu;\n")
    checked = "".join(f"static_assert (niji_s07_has (NIJI_S07_STR ({m} (3, s)), \"__fdelt_chk\"), \"{m} skips __fdelt_chk\");\n"
                      for m in ("FD_SET", "FD_CLR", "FD_ISSET"))
    checked = ("#ifdef FD_SET\n#define NIJI_S07_STR2(x) #x\n#define NIJI_S07_STR(x) NIJI_S07_STR2 (x)\n"
               "constexpr bool niji_s07_at (const char *h, const char *n) { return !*n || (*h == *n && niji_s07_at (h + 1, n + 1)); }\n"
               "constexpr bool niji_s07_has (const char *h, const char *n) { return niji_s07_at (h, n) || (*h && niji_s07_has (h + 1, n)); }\n"
               + checked + "#endif\n")
    for h in FD_MACRO_HEADERS:
        for first, second in ((h, "sys/types.h"), ("sys/types.h", h)):
            body = f"#include <{first}>\n#include <{second}>\n" + use
            out += [(cid, f"fd_set macros {first} then {second}", body + (checked if cid in FD_CHECK_CONFIGS else ""))
                    for cid in ctx.ids]
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


FD_CHECK_CONFIGS = ("cxx11", "cxx14", "cxx17", "cxx20", "cxx23")


FD_MACRO_HEADERS = ("sys/select.h", "sys/time.h", "stdlib.h", "utmp.h", "utmpx.h", "sys/procfs.h")


FD_WORD_NAMES = ("fd_mask", "__fd_mask", "NFDBITS", "__NFDBITS", "FD_SETSIZE", "__FD_SETSIZE")
WAIT_U_CHAR_FAMILY = ("__u_char", "__u_short", "__u_int", "__u_long", "__quad_t", "__u_quad_t", "__FD_SETSIZE")


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
