"""Macro alias probe (HEADERS-STAGE2-GAPS-B; the form rule of 2026-11-24 applied to a macro's spelling): a macro the
reference defines as another macro's name (`AF_INET` as `PF_INET`, `AF_FILE` as `AF_LOCAL`) has that form in libniji's
header too, not only its value. Generators that read a header's macros one step deep (systemd's address-family list,
apparmor's) keep only the names whose definition is such a name.

Black box: for each header and each integer macro Y that shares its value with another macro X of that header, a unit
includes the header, then `#undef Y` / `#define Y <sentinel>`, and asserts `(X) != <sentinel>` for every such X; an
error on X's line means X expands to exactly Y (directly or through other names). A baseline unit first asserts each
X equal to its macros.tsv value (a line that fails there, X undefined or of another value on that side, is not
probed). Only exit status and the line numbers of the probe unit's own errors are read (form.error_lines), never a
header's text. X's direct alias is the name Y it follows whose own followers are all X follows besides Y.

    python3 spec/tools/header_checks/aliases.py table [--headers H,...]   write spec/abi/macro-aliases.tsv (reference)
    python3 spec/tools/header_checks/aliases.py check [--include DIR] [--headers H,...]   compare libniji's tree

As a header_check.py module: compares the two sides for the ENFORCED headers (a difference fails the run), or for
every header with NIJI_ALIAS_ALL=1 (the others report mode: one probe side of every header takes about 4 minutes on
a Zen 1 host, 16 compilers). NIJI_ALIAS_PROBE=0 skips it.
"""
import argparse
import concurrent.futures
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.dirname(HERE))
import form  # noqa: E402

ROOT = form.ROOT
TABLE = os.path.join(ROOT, "spec", "abi", "macro-aliases.tsv")
SENTINEL = "0x7eadbee"
INT = {"int", "unsigned int", "long", "unsigned long"}
LITERAL = re.compile(r"-?(0x[0-9a-fA-F]+|[0-9]+)")
# Headers whose aliases must match (FAIL): systemd's and apparmor's address-family lists read <sys/socket.h>.
ENFORCED = {"sys/socket.h"}
PRE = 3  # the unit's first lines: the include, then the #undef and #define of the probed name (blank in the baseline)


def tables():
    def rows(path):
        with open(os.path.join(ROOT, path)) as f:
            head = f.readline().rstrip("\n").split("\t")
            return [dict(zip(head, line.rstrip("\n").split("\t"))) for line in f]
    macros = {r["name"]: r for r in rows("spec/abi/macros.tsv")}
    by_header = {}
    for r in rows("spec/abi/visibility.tsv"):
        if r["kind"] != "macro" or r["name"].startswith("__"):
            continue
        m = macros.get(r["name"])
        if m and m["type"] in INT and LITERAL.fullmatch(m["value"]):
            by_header.setdefault(r["header"], {})[r["name"]] = int(m["value"], 0)
    return by_header


def groups(names):
    """The probed pairs of one header: value -> the names sharing it (two or more)."""
    by = {}
    for n, v in names.items():
        by.setdefault(v, []).append(n)
    return {v: sorted(ns) for v, ns in by.items() if len(ns) > 1}


def compile_lines(side, header, pre, lines, work, tag):
    src = os.path.join(work, tag + ".c")
    with open(src, "w") as f:
        f.write("\n".join([f"#include <{header}>"] + pre + lines) + "\n")
    p = subprocess.run(side.compiler("c") + side.flags() + ["-D_GNU_SOURCE", "-w", "-fsyntax-only",
                       "-fno-diagnostics-show-caret", src], capture_output=True, text=True, stdin=subprocess.DEVNULL)
    return {ln - PRE - 1 for ln in form.error_lines(p.stderr, src)}


def probe_header(side, header, names, work):
    """{X: direct alias} for one header on one side."""
    gs = groups(names)
    members = sorted(n for ns in gs.values() for n in ns)
    if not members:
        return {}
    d = tempfile.mkdtemp(prefix="alias.", dir=work)
    bad = compile_lines(side, header, ["", ""], [f"_Static_assert (({x}) == ({names[x]}), \"\");" for x in members], d, "base")
    ok = {x for i, x in enumerate(members) if i not in bad}
    follows = {x: set() for x in ok}
    for v, ns in gs.items():
        ns = [n for n in ns if n in ok]
        for y in ns:
            xs = [x for x in ns if x != y]
            if not xs:
                continue
            err = compile_lines(side, header, [f"#undef {y}", f"#define {y} {SENTINEL}"],
                                [f"_Static_assert (({x}) != {SENTINEL}, \"\");" for x in xs], d, "y")
            for i, x in enumerate(xs):
                if i in err:
                    follows[x].add(y)
    out = {}
    for x, fs in follows.items():
        direct = [y for y in fs if follows.get(y, set()) == fs - {y}]
        if len(direct) == 1:
            out[x] = direct[0]
    return out


def sides(inc, work):
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    kern = os.path.join(work, "kernel-uapi")
    os.makedirs(kern, exist_ok=True)
    for d in ("linux", "asm-generic", "asm"):
        for base_dir in ("/usr/include", "/usr/include/x86_64-linux-gnu"):
            src = os.path.join(base_dir, d)
            if os.path.isdir(src) and not os.path.lexists(os.path.join(kern, d)):
                os.symlink(src, os.path.join(kern, d))
    return form.Side("reference", None, gccinc, None), form.Side("libniji", inc, gccinc, kern)


def run(side, headers, by_header, work, jobs):
    with concurrent.futures.ThreadPoolExecutor(jobs) as ex:
        futs = {h: ex.submit(probe_header, side, h, by_header.get(h, {}), work) for h in headers}
        return {h: f.result() for h, f in futs.items()}


def compare(inc, work, jobs, only=None, skip=()):
    by_header = tables()
    headers = sorted(h for h in by_header if h not in skip and (not only or h in only)
                     and os.path.exists(os.path.join(inc, h)))
    ref_side, niji_side = sides(inc, work)
    ref = run(ref_side, headers, by_header, work, jobs)
    got = run(niji_side, headers, by_header, work, jobs)
    diffs = []
    for h in headers:
        for x in sorted(set(ref[h]) | set(got[h])):
            if ref[h].get(x) != got[h].get(x):
                diffs.append((h, x, ref[h].get(x, "-"), got[h].get(x, "-")))
    return headers, ref, diffs


def problems(ctx):
    """header_check.py entry: FAIL lines for the ENFORCED headers' differences; the others are NOTE lines."""
    if os.environ.get("NIJI_ALIAS_PROBE") == "0":
        print("NOTE aliases: skipped (NIJI_ALIAS_PROBE=0)")
        return []
    inc = getattr(ctx, "inc", os.path.join(ROOT, "libc-include"))
    work = tempfile.mkdtemp(prefix="aliases.", dir=getattr(ctx, "work", None))
    if not form.reference_compiles(work):
        print("NOTE aliases: skipped, the build host's own C library headers do not compile (no reference)")
        return []
    jobs = int(os.environ.get("NIJI_FORM_JOBS", min(16, os.cpu_count() or 1)))
    skip = {x for _, mod in ctx.mods for x in getattr(mod, "must_fail", ())} if hasattr(ctx, "mods") else form.must_fail_headers()
    only = None if os.environ.get("NIJI_ALIAS_ALL") == "1" else ENFORCED
    headers, ref, diffs = compare(inc, work, jobs, only=only, skip=skip)
    fails = []
    for h, x, a, b in diffs:
        line = f"ALIAS-DIFF {h}: {x} is {a} in the reference, {b} in libniji (- = not another macro's name)"
        if h in ENFORCED:
            fails.append(line)
        else:
            print("NOTE " + line)
    print(f"NOTE aliases: {len(headers)} headers, {sum(len(r) for r in ref.values())} reference aliases, {len(diffs)} differences")
    return fails


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("mode", choices=["table", "check"])
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    ap.add_argument("--headers", default="")
    ap.add_argument("--work", default="")
    ap.add_argument("--jobs", type=int, default=min(16, os.cpu_count() or 1))
    args = ap.parse_args()
    work = args.work or tempfile.mkdtemp(prefix="aliases.")
    os.makedirs(work, exist_ok=True)
    only = set(args.headers.split(",")) if args.headers else None
    if args.mode == "table":
        by_header = tables()
        inc = os.path.abspath(args.include)
        headers = sorted(h for h in by_header if (not only or h in only) and os.path.exists(os.path.join(inc, h))
                         and h not in form.must_fail_headers())
        ref = run(sides(inc, work)[0], headers, by_header, work, args.jobs)
        with open(TABLE, "w") as f:
            f.write("header\tname\talias\n")
            for h in headers:
                for x in sorted(ref[h]):
                    f.write(f"{h}\t{x}\t{ref[h][x]}\n")
        print(f"aliases: {len(headers)} headers, {sum(len(r) for r in ref.values())} aliases written to {TABLE}")
        return 0
    headers, ref, diffs = compare(os.path.abspath(args.include), work, args.jobs, only, form.must_fail_headers())
    for h, x, a, b in diffs:
        print(f"{'FAIL' if h in ENFORCED else 'NOTE'} ALIAS-DIFF {h}: {x} is {a} in the reference, {b} in libniji")
    print(f"aliases: {len(headers)} headers, {sum(len(r) for r in ref.values())} reference aliases, {len(diffs)} differences")
    return 1 if any(h in ENFORCED for h, *_ in diffs) else 0


if __name__ == "__main__":
    sys.exit(main())
