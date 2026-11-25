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
    for h in UTMP_HEADERS:
        out += ctx.exact(h, UTMP_PATHS)
    # Spec clause: select(2), the BSD fd_set word names (HEADERS-STAGE2-GAPS-E: libev's ev_select.c sizes its sets in
    # fd_mask words of NFDBITS bits): each exactly where the reference shows it, fd_mask with its C++ spelling (long);
    # the values and sizes are sys/select.h's table P1.
    out += ctx.exact("sys/select.h", FD_WORD_NAMES)
    # The reserved `__u_char` family through <sys/wait.h>'s includes (HEADERS-STAGE2-GAPS-E): the reference shows it in
    # the strict modes too, where <sys/wait.h> does not bring in <signal.h> (a generator that counted <signal.h>'s names
    # as supplied in every configuration left them out there).
    for h in ("sys/wait.h", "wait.h"):
        out += ctx.exact(h, WAIT_U_CHAR_FAMILY)
    return out


FD_WORD_NAMES = ("fd_mask", "__fd_mask", "NFDBITS", "__NFDBITS", "FD_SETSIZE", "__FD_SETSIZE")
WAIT_U_CHAR_FAMILY = ("__u_char", "__u_short", "__u_int", "__u_long", "__quad_t", "__u_quad_t", "__FD_SETSIZE")


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
