"""Slice 13 checks: stage2-gaps-l (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md)."""
from header_check import expand

# TTYDEF_{C,I,L,O}FLAG are spelled with the termios flag names (spec/abi/macro-spellings.tsv): where a header does not
# show every name of the spelling they are no integer constant, as the reference's (black-box probe,
# HEADERS-STAGE2-GAPS-L: <sys/ioctl.h> alone, <pty.h> and <termio.h> under strict C for OFLAG and LFLAG). P1 cannot
# evaluate them there; the units below check the value where the names are shown and the spelling elsewhere.
TTYDEF = {"TTYDEF_CFLAG": ("CREAD", "CS7", "PARENB", "HUPCL"),
          "TTYDEF_IFLAG": ("BRKINT", "ISTRIP", "ICRNL", "IMAXBEL", "IXON", "IXANY"),
          "TTYDEF_LFLAG": ("ECHO", "ICANON", "ISIG", "IEXTEN", "ECHOE", "ECHOKE", "ECHOCTL"),
          "TTYDEF_OFLAG": ("OPOST", "ONLCR", "XTABS")}
TTY_HEADERS = ("net/if_ppp.h", "net/if_shaper.h", "pty.h", "sgtty.h", "sys/ioctl.h", "sys/mount.h", "sys/mtio.h",
               "sys/raw.h", "sys/ttydefaults.h", "termio.h", "termios.h", "sys/termios.h")
p1_skip = {(h, n) for h in TTY_HEADERS for n in TTYDEF}

# libstdc++'s configure checks "ISO C99 support in <math.h> for C++98" and "ISO C99 generic macro support in <math.h>
# for C++11" (Yohane's R-H7 repro, built -nostdinc++): every classification and comparison macro is usable in C++.
CXX_MATH = """#include <math.h>
volatile double niji_gl_d1, niji_gl_d2;
volatile int niji_gl_i;
void niji_gl_f (void)
{
  niji_gl_i = fpclassify (niji_gl_d1);
  niji_gl_i = isfinite (niji_gl_d1);
  niji_gl_i = isinf (niji_gl_d1);
  niji_gl_i = isnan (niji_gl_d1);
  niji_gl_i = isnormal (niji_gl_d1);
  niji_gl_i = signbit (niji_gl_d1);
  niji_gl_i = isgreater (niji_gl_d1, niji_gl_d2);
  niji_gl_i = isgreaterequal (niji_gl_d1, niji_gl_d2);
  niji_gl_i = isless (niji_gl_d1, niji_gl_d2);
  niji_gl_i = islessequal (niji_gl_d1, niji_gl_d2);
  niji_gl_i = islessgreater (niji_gl_d1, niji_gl_d2);
  niji_gl_i = isunordered (niji_gl_d1, niji_gl_d2);
}
"""
# math.h.d/190: in C++ the TS 18661-1 names are no macros but functions of the reference's form (black-box probe):
# issignaling and iscanonical one int overload per floating type, iszero a template returning bool (an integer
# argument too), iseqsig a template over both argument types returning int, noexcept.
CXX_TS = """#include <math.h>
#if defined issignaling || defined iszero || defined iscanonical || defined iseqsig
#error "a TS 18661-1 name is a macro in C++"
#endif
int (*niji_gl_ts1) (double) = issignaling;
int (*niji_gl_ts2) (long double) = iscanonical;
bool (*niji_gl_ts3) (double) = iszero;
int (*niji_gl_ts4) (double, double) = iseqsig;
int niji_gl_ts5 = iszero<double> (1.0) + iszero (1) + iseqsig<double> (1.0, 1.0) + iseqsig (1, 1.0f);
"""
CXX_TS11 = """static_assert (noexcept (iseqsig (1.0, 1.0)), "iseqsig noexcept");
static_assert (__is_same (decltype (issignaling (1.0f)), int) && __is_same (decltype (iscanonical (1.0)), int), "int");
static_assert (__is_same (decltype (iszero (1.0L)), bool) && __is_same (decltype (iseqsig (1.0, 1.0L)), int), "form");
"""
# The macros the C++ library's <cmath> undefines and replaces must be macros (it tests each with #ifdef).
CXX_MACROS = ("fpclassify", "isfinite", "isinf", "isnan", "isnormal", "signbit", "isgreater", "isgreaterequal",
              "isless", "islessequal", "islessgreater", "isunordered")

# <printf.h>: the argument types of parse_printf_format, an enumeration without a name (no macros), the values the
# reference's give, combined with the PA_FLAG_* bits as systemd's and elogind's src/basic/stdio-util.h do.
PA = ("PA_INT", "PA_CHAR", "PA_WCHAR", "PA_STRING", "PA_WSTRING", "PA_POINTER", "PA_FLOAT", "PA_DOUBLE", "PA_LAST")
PA_UNIT = "#include <printf.h>\n" + "".join(f"#ifdef {n}\n#error \"{n} is a macro\"\n#endif\n" for n in PA) + "".join(
    f"typedef char niji_gl_pa{i}[({n}) == {i} ? 1 : -1];\n" for i, n in enumerate(PA)) + """\
typedef char niji_gl_pal[(PA_INT | PA_FLAG_LONG) == 512 && (PA_INT | PA_FLAG_LONG_LONG) == 256 ? 1 : -1];
typedef char niji_gl_pad[(PA_DOUBLE | PA_FLAG_LONG_DOUBLE) == 263 && (PA_INT | PA_FLAG_SHORT) == 1024 ? 1 : -1];
typedef char niji_gl_pap[(PA_FLAG_MASK & PA_LAST) == 0 && (PA_STRING | PA_FLAG_PTR) == 2051 ? 1 : -1];
"""
PA_CXX = """#include <printf.h>
typedef char niji_gl_enum[__is_enum (__typeof__ (PA_INT)) && __is_same (__typeof__ (PA_INT), __typeof__ (PA_LAST)) ? 1 : -1];
"""

# <unistd.h> declares __environ (libc.so.6 exports it) in every configuration; a program's own declaration agrees.
ENVIRON = """#include <unistd.h>
extern char **__environ;
char **niji_gl_env (void) { return __environ; }
"""

# headers/nested-tags.tsv: in C++ the tags of struct icmp's and struct ip6_hdr's union members are nested (the
# reference's g++ shows none at namespace scope), so a program's own struct of the same tag after the header is a new
# type, and from C++11 the nested one is named through the member's type.
NESTED = {"netinet/ip_icmp.h": (("icmp", "icmp_hun", "ih_idseq", 4), ("icmp", "icmp_hun", "ih_pmtu", 4),
                                ("icmp", "icmp_hun", "ih_rtradv", 4)),
          "netinet/ip6.h": (("ip6_hdr", "ip6_ctlun", "ip6_hdrctl", 8),)}


def _nested(h, cid):
    body = f"#include <{h}>\n"
    for i, (outer, member, tag, size) in enumerate(NESTED[h]):
        body += f"struct {tag} {{ char niji[3]; }};\ntypedef char niji_gl_nt{i}[sizeof (struct {tag}) == 3 ? 1 : -1];\n"
        if cid != "cxx98":
            body += f"static_assert (sizeof (decltype ({outer}::{member})::{tag}) == {size}, \"{tag}\");\n"
    return body


def units(ctx):
    """No spec clause beyond the reference's headers (HEADERS-STAGE2-GAPS-L; Yohane R-H7 on snapshot 27.2: gcc-14's
    libstdc++ configure and Modula-2 wrapc.c, systemd 257 and elogind 255 stdio-util.h, fakechroot setenv.c):
    (1) <math.h> in C++ defines the classification and comparison macros, libstdc++'s configure tests compile;
    (2) the PA_* argument types of <printf.h>: values, combinations with PA_FLAG_*, no macros, one unnamed enumeration;
    (3) __environ after <unistd.h>; (4) TTYDEF_{C,I,L,O}FLAG: the value where the header shows the flag names, and
    otherwise the flag names' spelling (a program's own definitions of the names, after the header, give the value);
    (5) C++: issignaling, iszero, iscanonical, iseqsig are functions of the reference's form, not macros;
    (6) C++: the union members' tags of struct icmp and struct ip6_hdr are nested, not at namespace scope.
    Defects: the math macros hidden from C++ (configure then disables C99 math in libstdc++), a PA_* value off or a
    macro, __environ undeclared or declared with another type, TTYDEF_* evaluable where the reference's is not (a
    number) or spelled with other names, a TS 18661-1 name a macro in C++ or of another type, a nested tag at namespace scope (a program's own struct of that tag is a
    redefinition)."""
    shown = {}
    for r in ctx.vis:
        shown.setdefault((r["header"], r["name"]), set()).update(expand(r["configs"], ctx.ids))
    cxx = [c for c in ("cxx98", "cxx11", "cxx17", "cxx20") if c in ctx.ids]
    out = []
    macros = "#include <math.h>\n" + "".join(f"#ifndef {n}\n#error \"{n} is no macro\"\n#endif\n" for n in CXX_MACROS)
    out += [(cid, "GAPS-L C++ math.h configure tests", CXX_MATH) for cid in cxx]
    out += [(cid, "GAPS-L C++ math.h macros", macros) for cid in cxx]
    out += [(cid, "GAPS-L C++ math.h TS 18661-1 forms", CXX_TS + ("" if cid == "cxx98" else CXX_TS11)) for cid in cxx
            if cid in shown.get(("math.h", "iseqsig"), ())]
    pick = ("c89", "c17", "gnu17", "cxx98", "cxx17")
    out += [(cid, "GAPS-L printf.h PA types", PA_UNIT) for cid in pick if cid in shown.get(("printf.h", "PA_INT"), ())]
    out += [(cid, "GAPS-L printf.h PA enum", PA_CXX) for cid in ("cxx98", "cxx17") if cid in shown.get(("printf.h", "PA_INT"), ())]
    out += [(cid, "GAPS-L unistd.h __environ", ENVIRON) for cid in pick if cid in shown.get(("unistd.h", "__environ"), ())]
    for h, rows in NESTED.items():
        out += [(cid, f"GAPS-L C++ nested tags {h}", _nested(h, cid)) for cid in cxx
                if cid in shown.get((h, rows[0][0]), ())]
    for h in TTY_HEADERS:
        for cid in ("c17", "xopen700", "gnu17", "cxx17"):
            body = ""
            for i, (n, parts) in enumerate(sorted(TTYDEF.items())):
                if cid not in shown.get((h, n), ()):
                    continue
                if all(cid in shown.get((h, p), ()) for p in parts):
                    v = int(ctx.macros[n]["value"])
                    body += f"typedef char niji_gl_tty{i}[({n}) == {v} ? 1 : -1];\n"
                    continue
                # The names the header does not show, defined by the program: distinct bits, so the sum proves each
                # name of the spelling is used once. A shown name keeps its value (its own macros.tsv row).
                total = []
                for j, p in enumerate(parts):
                    if cid in shown.get((h, p), ()):
                        total.append(f"({p})")
                    else:
                        body += f"#define {p} (1L << {32 + 8 * i + j})\n"
                        total.append(f"(1L << {32 + 8 * i + j})")
                body += f"typedef char niji_gl_tty{i}[({n}) == ({' | '.join(total)}) ? 1 : -1];\n"
            if body:
                out.append((cid, f"GAPS-L TTYDEF {h}", f"#include <{h}>\n{body}"))
    return out
