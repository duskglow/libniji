"""Slice 14 (plan/headers.md section 4): <syslog.h>'s priority macros and its SYSLOG_NAMES tables (HEADERS-NET-SYS-NAMES).

* `units(ctx)`: LOG_PRI, LOG_FAC and LOG_MAKEPRI in every configuration that shows them; with SYSLOG_NAMES defined
  and nothing else included (D-26.3), CODE, the two tables' lengths and INTERNAL_NOPRI / INTERNAL_MARK where the BSD
  names are visible (`__NIJI_MISC`), and none of them elsewhere.
* `problems(ctx)`: a C program built against the tree compares both tables, row by row, with the reference's (black-box
  probe: the dump of a program built on the reference system); a C++ unit takes the tables too.
No spec clause beyond syslog(3) and syslog.conf(5) for the names. Loaded by spec/tools/header_check.py.
"""
import os
import subprocess

# The reference's tables, in order (black-box probe, HEADERS-NET-SYS-NAMES): each ends in a null name with value -1.
PRIORITYNAMES = [("alert", 1), ("crit", 2), ("debug", 7), ("emerg", 0), ("err", 3), ("error", 3), ("info", 6),
                 ("none", 16), ("notice", 5), ("panic", 0), ("warn", 4), ("warning", 4)]
FACILITYNAMES = [("auth", 32), ("authpriv", 80), ("cron", 72), ("daemon", 24), ("ftp", 88), ("kern", 0), ("lpr", 48),
                 ("mail", 16), ("mark", 192), ("news", 56), ("security", 32), ("syslog", 40), ("user", 8), ("uucp", 64),
                 *((f"local{i}", 128 + 8 * i) for i in range(8))]
# Probed values: LOG_MAKEPRI ors (3 | 1 is 3, not 4), each macro is parenthesised as a whole and around its arguments,
# LOG_FAC masks before it shifts (-1 gives 127), and the result has the argument's type (unsigned for 8u).
PRI_TESTS = ["LOG_MAKEPRI(3, 1) == 3", "LOG_MAKEPRI(1, 2) * 2 == 6", "LOG_MAKEPRI(1 ? 8 : 0, 3) == 11",
             "LOG_MAKEPRI(LOG_LOCAL0, LOG_ERR) == 131", "LOG_FAC(0x3ff) == 127", "LOG_FAC(8 | 3) * 2 == 2",
             "LOG_FAC(-1) == 127", "LOG_PRI(0xff) == 7", "LOG_PRI(1 + 2) * 2 == 6", "LOG_PRI(-1) == 7",
             "LOG_FAC(8u) - 2 > 0", "LOG_MAKEPRI(8u, 1) - 10 > 0"]


def expand(configs, ids):
    if configs == "*":
        return set(ids)
    if configs.startswith("!"):
        return set(ids) - set(configs[1:].split(","))
    return set(configs.split(","))


def _shown(ctx, header, name):
    return {cid for r in ctx.vis if r["header"] == header and r["name"] == name for cid in expand(r["configs"], ctx.ids)}


def units(ctx):
    res = []
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    misc = {r["id"]: r["misc"] == "1" for r in ctx.rows("headers/levels.tsv")}
    for hdr in ("syslog.h", "sys/syslog.h"):
        cids = _shown(ctx, hdr, "LOG_MAKEPRI") & _shown(ctx, hdr, "LOG_FAC") & _shown(ctx, hdr, "LOG_PRI")
        if not cids:
            res.append((ctx.ids[0], f"LOG_PRI {hdr}", "#error visibility.tsv does not show LOG_PRI, LOG_FAC, LOG_MAKEPRI\n"))
        for cid in sorted(cids):
            body = f"#include <{hdr}>\n" + "".join(f"typedef char niji_lp{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(PRI_TESTS))
            res.append((cid, f"LOG_PRI {hdr}", body))
    for cid in sorted(_shown(ctx, "syslog.h", "LOG_MAKEPRI")):
        if lang[cid] != "c":
            continue  # c_name is `char *`: C++ warns on a string literal there (problems() takes the tables in C++)
        body = ["#define SYSLOG_NAMES", "#include <syslog.h>"]
        if misc[cid]:
            body += ["typedef char niji_sn_np[sizeof prioritynames / sizeof prioritynames[0] == %d ? 1 : -1];" % (len(PRIORITYNAMES) + 1),
                     "typedef char niji_sn_nf[sizeof facilitynames / sizeof facilitynames[0] == %d ? 1 : -1];" % (len(FACILITYNAMES) + 1),
                     "typedef char niji_sn_t[__builtin_types_compatible_p(__typeof__(prioritynames), CODE[]) && "
                     "__builtin_types_compatible_p(__typeof__(facilitynames[0].c_name), char *) && "
                     "__builtin_types_compatible_p(__typeof__(facilitynames[0].c_val), int) && sizeof(struct _code) == 16 ? 1 : -1];",
                     "typedef char niji_sn_i[INTERNAL_NOPRI == 0x10 && INTERNAL_MARK == 192 ? 1 : -1];"]
            res.append((cid, "SYSLOG_NAMES syslog.h", "\n".join(body) + "\n"))
        else:
            body += ["#if defined INTERNAL_NOPRI || defined INTERNAL_MARK", "#error INTERNAL_* where __NIJI_MISC is 0", "#endif",
                     "int prioritynames, facilitynames;", "typedef int CODE;", "struct _code { int niji_c; };"]
            res.append((cid, "SYSLOG_NAMES syslog.h (not shown)", "\n".join(body) + "\n"))
    return res


def problems(ctx):
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    flags = ["-nostdinc", "-isystem", ctx.inc, "-isystem", gccinc]

    def table(name, rows):
        lit = ", ".join(f'{{ "{n}", {v} }}' for n, v in rows)
        return (f"  {{ static const struct {{ const char *n; int v; }} want[] = {{ {lit}, {{ 0, -1 }} }};\n"
                f"    if (sizeof {name} != sizeof want / sizeof want[0] * sizeof(CODE)) return 1;\n"
                f"    for (unsigned i = 0; i < sizeof want / sizeof want[0]; i++) {{\n"
                f"      if ({name}[i].c_val != want[i].v || !{name}[i].c_name != !want[i].n) return 2;\n"
                f"      for (unsigned j = 0; want[i].n && (j == 0 || want[i].n[j - 1]); j++)\n"
                f"        if ({name}[i].c_name[j] != want[i].n[j]) return 3;\n    }} }}\n")
    src = os.path.join(ctx.work, "s14_syslog_names.c")
    with open(src, "w") as f:
        f.write("#define SYSLOG_NAMES\n#include <syslog.h>\nint main(void) {\n" + table("prioritynames", PRIORITYNAMES)
                + table("facilitynames", FACILITYNAMES) + "  return 0;\n}\n")
    exe = src[:-2]
    out = []
    b = subprocess.run(["gcc", "-std=gnu17", "-Wall", "-Wextra", "-Werror", *flags, src, "-o", exe], capture_output=True, text=True)
    if b.returncode:
        out.append(f"SYSLOG_NAMES tables: build: {' | '.join(b.stderr.strip().splitlines()[:3])[:400]}")
    else:
        rc = subprocess.run([exe]).returncode
        if rc:
            out.append(f"SYSLOG_NAMES tables: rc {rc} (1 length, 2 value or null name, 3 name)")
    cxx = subprocess.run(["g++", "-std=c++17", "-w", "-fsyntax-only", *flags, "-x", "c++", "-"], capture_output=True, text=True,
                         input="#define SYSLOG_NAMES\n#include <syslog.h>\nint niji_x() { return prioritynames[0].c_val + facilitynames[0].c_val; }\n")
    if cxx.returncode:
        out.append(f"SYSLOG_NAMES syslog.h in C++: {' | '.join(cxx.stderr.strip().splitlines()[:3])[:400]}")
    return out


if __name__ == "__main__":
    # The tables' comparison alone (header_check.py --only runs units, not problems): a plant's command, seconds.
    import argparse
    import sys
    import tempfile
    from types import SimpleNamespace
    ap = argparse.ArgumentParser(description="the SYSLOG_NAMES table comparison of slice 26")
    ap.add_argument("--include", default="libc-include")
    a = ap.parse_args()
    res = problems(SimpleNamespace(inc=os.path.abspath(a.include), work=tempfile.mkdtemp(prefix="s26.")))
    for line in res:
        print("FAIL " + line)
    print(f"s26: {len(res)} failed")
    sys.exit(1 if res else 0)
