#!/usr/bin/env python3
"""P4 of plan/headers.md (slice 20): the C++ check of libniji's headers.

  python3 validation/headers_p4.py [--include DIR] [--uapi DIR] [--jobs N] [--out DIR] [--no-noexcept] [--no-cxx]

1. Every C++ standard library header (the header list of the C++ standard, [headers], C++98 to the C++26
   working draft, with the C library's `<name.h>` forms and the deprecated ones), in every `-std` from
   c++98 to c++26, is compiled (`#include <h>` and an empty `main`, `-fsyntax-only`) once on the build
   host's own headers (the reference) and once with `-nostdinc`, the C++ library's directories (the
   compiler's own search list), libniji's headers (DIR, default target/niji/include), the compiler's
   include directory and the kernel UAPI headers (default target/niji/uapi): the order niji-gcc uses.
   Of the reference only the exit status is read (clean room, plan/headers.md section 2.2): a header the
   reference does not compile in that -std is "n/a". Each pair where the reference compiles and libniji
   does not is a FAIL, printed with up to three of the compiler's errors on libniji's side.
2. noexcept.tsv and callsites.tsv are reproduced: spec/tools/callsite_probe.py --include DIR writes them
   into OUT (default a temporary directory) and they must equal spec/abi's (typedefs.tsv's sizes and
   alignments are P1, spec/tools/header_check.py). --no-noexcept skips this part; --no-cxx skips part 1.
   As P2 and P3 do (spec/tools/probe_compare.py), the rows of the reserved names the generator leaves out by
   policy (headers/reserved.tsv; D-X.107, rulings R-H2 and R-H3) are not compared. Each other difference is a
   line in probe_compare.py's form and a FAIL unless validation/headers_p4.accept (or a slice's
   validation/headers_s*.accept) lists it; a headers_p4.accept line the probe no longer shows is a FAIL (stale).
3. The headers libniji does not ship yet are validation/headers_unshipped.tsv: that list must equal the
   headers the spec tables name with no file under DIR (a FAIL either way). A C++ header whose only errors
   are "<one of them>: No such file or directory" is "known", not a FAIL; the probe skips those headers
   and part 2 compares the tables without their rows.

Prints one line per -std ("P4 c++17: 141 compiled, 3 n/a, 0 known, 0 FAIL") and a summary; exit 1 on any FAIL.
No spec clause: header tooling (plan/headers.md P4).
"""
import argparse, concurrent.futures as cf, glob, os, re, subprocess, sys, tempfile

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "spec", "tools"))
from probe_compare import reserved_omitted  # noqa: E402  (the rule P2 and P3 compare by)

STDS = ["c++98", "c++03", "c++11", "c++14", "c++17", "c++20", "c++23", "c++26"]
# The C++ standard's library headers ([headers], tables "C++ library headers" and "C++ headers for C
# library facilities", C++98 to the C++26 working draft), the deprecated and removed ones included.
CXX = """algorithm any array atomic barrier bit bitset charconv chrono codecvt compare complex concepts
condition_variable coroutine debugging deque exception execution expected filesystem flat_map flat_set format
forward_list fstream functional future generator hazard_pointer initializer_list inplace_vector iomanip ios
iosfwd iostream istream iterator latch limits linalg list locale map mdspan memory memory_resource mutex new
numbers numeric optional ostream print queue random ranges ratio rcu regex scoped_allocator semaphore set
shared_mutex source_location span spanstream sstream stack stacktrace stdexcept stdfloat stop_token streambuf
string string_view strstream syncstream system_error text_encoding thread tuple type_traits typeindex typeinfo
unordered_map unordered_set utility valarray variant vector version
cassert ccomplex cctype cerrno cfenv cfloat cinttypes ciso646 climits clocale cmath csetjmp csignal cstdalign
cstdarg cstdbool cstddef cstdint cstdio cstdlib cstring ctgmath ctime cuchar cwchar cwctype
assert.h complex.h ctype.h errno.h fenv.h float.h inttypes.h iso646.h limits.h locale.h math.h setjmp.h
signal.h stdalign.h stdarg.h stdatomic.h stdbool.h stddef.h stdint.h stdio.h stdlib.h string.h tgmath.h
time.h uchar.h wchar.h wctype.h""".split()


def search_list(cxx):
    """The C++ library's directories of the compiler's own search list (its -v report)."""
    o = subprocess.run([cxx, "-x", "c++", "-E", "-v", "-"], input="", capture_output=True, text=True).stderr
    m = re.search(r"#include <\.\.\.> search starts here:\n(.*?)\nEnd of search list\.", o, re.S)
    return [d.strip() for d in (m.group(1).splitlines() if m else []) if "/c++/" in d]


UNSHIPPED = "validation/headers_unshipped.tsv"
ACCEPT = "validation/headers_p4.accept"
TABLES = ("visibility.tsv", "typedefs.tsv", "callsites.tsv", "noexcept.tsv")
NOT_FOUND = re.compile(r": fatal error: (\S+): No such file or directory$")


def tsv_rows(path):
    """(column names, rows as lists) of a tab-separated table; `#` lines are comments."""
    with open(path) as f:
        lines = [l.rstrip("\n").split("\t") for l in f if not l.startswith("#")]
    return lines[0], lines[1:]


def unshipped_check(inc, listed):
    """The FAIL lines of part 3: the listed headers against the named headers missing under inc."""
    named = set()
    for t in TABLES:
        head, rows = tsv_rows(os.path.join("spec/abi", t))
        named |= {r[head.index("header")] for r in rows if r[head.index("header")].endswith(".h")}
    missing = {h for h in named if not os.path.isfile(os.path.join(inc, h))}
    out = [f"P4 FAIL <{h}>: named by spec/abi, not shipped and not in {UNSHIPPED}" for h in sorted(missing - listed)]
    out += [f"P4 FAIL <{h}>: in {UNSHIPPED} but shipped (or named by no table): remove its row"
            for h in sorted(listed - missing)]
    return out


def table_lines(t, ref_rows, mine_rows, head, omitted):
    """Part 2's difference lines of table t, one per row: callsites.tsv in probe_compare.py's form (`missing|extra
    <header> <name> <mode> <symbol> [<configs>]`, `differs ... table [<configs>] libniji [<configs>]`), noexcept.tsv
    keyed by header and name (`differs noexcept <header> <name> table [<cxx17> <variadic> <cxx98>] libniji [...]`).
    Rows of the names the generator leaves out by policy are not compared; returns (lines, rows left out)."""
    ix = {c: i for i, c in enumerate(head)}
    if t == "callsites.tsv":
        key, val, tag = ("header", "name", "mode", "symbol"), ("configs",), ""
    else:
        key, val, tag = ("header", "name"), ("cxx17", "variadic", "cxx98"), "noexcept "
    def index(rows):
        return {tuple(r[ix[k]] for k in key): " ".join(r[ix[v]] for v in val) for r in rows if not omitted(r[ix["name"]])}
    want, got = index(ref_rows), index(mine_rows)
    left_out = sum(1 for r in ref_rows if omitted(r[ix["name"]]))
    lines = []
    for k in sorted(want.keys() | got.keys()):
        if k not in got:
            lines.append(f"missing {tag}{' '.join(k)} [{want[k]}]")
        elif k not in want:
            lines.append(f"extra {tag}{' '.join(k)} [{got[k]}]")
        elif want[k] != got[k]:
            lines.append(f"differs {tag}{' '.join(k)} table [{want[k]}] libniji [{got[k]}]")
    return lines, left_out


def accept_lines(path):
    with open(path) as f:
        return {l.rstrip("\n") for l in f if l.strip() and not l.startswith("#")}


def compile_one(cmd, text):
    p = subprocess.run(cmd + ["-x", "c++", "-fsyntax-only", "-"], input=text, capture_output=True, text=True)
    return p.returncode, p.stderr


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--include", default="target/niji/include")
    ap.add_argument("--uapi", default="target/niji/uapi")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--out", default="")
    ap.add_argument("--no-noexcept", action="store_true")
    ap.add_argument("--no-cxx", action="store_true")
    a = ap.parse_args()
    stds = [] if a.no_cxx else STDS
    inc, uapi = os.path.abspath(a.include), os.path.abspath(a.uapi)
    if not os.path.isfile(os.path.join(inc, "features.h")) or not os.path.isdir(uapi):
        print(f"P4: FAIL {inc} or {uapi} is missing (cargo xtask so)")
        return 1
    cxx = "g++"
    gccinc = subprocess.run([cxx, "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    dirs = search_list(cxx)
    if not dirs:
        print("P4: FAIL the compiler lists no C++ library directory")
        return 1
    # A compiler older than the name c++26 (GCC 13) knows the draft as c++2c.
    # A compiler that knows neither name (GCC 13 has neither) says so once, instead of 145 "n/a" lines.
    if compile_one([cxx, "-std=c++26"], "int main() { return 0; }\n")[0] != 0:
        if compile_one([cxx, "-std=c++2c"], "int main() { return 0; }\n")[0] == 0:
            STDS[STDS.index("c++26")] = "c++2c"
        else:
            STDS.remove("c++26")
            print("P4 c++26: not supported by this compiler (neither -std=c++26 nor -std=c++2c)")
    niji = ["-nostdinc"] + [w for d in dirs for w in ("-isystem", d)] + ["-isystem", inc, "-isystem", gccinc, "-isystem", uapi]
    listed = {r[0] for r in tsv_rows(UNSHIPPED)[1]}
    jobs = {}
    lines = unshipped_check(inc, listed)
    for line in lines:
        print(line)
    fails = len(lines)
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        for std in stds:
            for h in CXX:
                text = f"#include <{h}>\nint main() {{ return 0; }}\n"
                jobs[(std, h, "ref")] = ex.submit(compile_one, [cxx, f"-std={std}"], text)
                jobs[(std, h, "niji")] = ex.submit(compile_one, [cxx, f"-std={std}"] + niji, text)
        for std in stds:
            ok = na = known = bad = 0
            for h in CXX:
                ref, _ = jobs[(std, h, "ref")].result()
                rc, err = jobs[(std, h, "niji")].result()
                errs = [l for l in err.splitlines() if "error:" in l]
                gone = {m.group(1) for m in map(NOT_FOUND.search, errs) if m}
                if ref != 0:
                    na += 1
                    if rc == 0:
                        print(f"P4 NOTE {std} <{h}>: compiles on libniji's headers only")
                elif rc == 0:
                    ok += 1
                elif errs and gone <= listed and all(NOT_FOUND.search(l) for l in errs):
                    known += 1
                    print(f"P4 KNOWN {std} <{h}>: needs <{'>, <'.join(sorted(gone))}>, not shipped yet ({UNSHIPPED})")
                else:
                    bad += 1
                    errs = [l for l in err.splitlines() if ": error:" in l and l.startswith(inc)][:3]
                    print(f"P4 FAIL {std} <{h}>: " + (" | ".join(errs) if errs else "(no error in libniji's headers; "
                          + (err.splitlines() or ["?"])[0][:200] + ")"))
            fails += bad
            print(f"P4 {std}: {ok} compiled, {na} n/a, {known} known, {bad} FAIL")
    if not a.no_noexcept:
        out = a.out or tempfile.mkdtemp(prefix="p4-")
        os.makedirs(out, exist_ok=True)
        p = subprocess.run([sys.executable, "spec/tools/callsite_probe.py", "--include", inc, "--out", out,
                            "--jobs", str(a.jobs)], capture_output=True, text=True, env={**os.environ, "NIJI_UAPI": uapi})
        if p.returncode != 0:
            print("P4 FAIL callsite_probe.py --include: " + (p.stderr.strip().splitlines() or ["?"])[-1])
            fails += 1
        omitted = reserved_omitted()
        mine_accept = accept_lines(ACCEPT)
        accepted = set(mine_accept)
        for p in sorted(glob.glob("validation/headers_s*.accept")):
            accepted |= accept_lines(p)
        seen = set()
        for t in ("noexcept.tsv", "callsites.tsv"):
            # The reference's rows of the headers libniji ships: the probe skips the unshipped ones.
            head, rows = tsv_rows(os.path.join("spec/abi", t))
            ref = os.path.join(out, "spec-" + t)
            with open(ref, "w", newline="\n") as f:
                f.writelines("\t".join(r) + "\n" for r in [head] + [r for r in rows if r[1] not in listed])
            mine = os.path.join(out, t)
            if not os.path.isfile(mine):
                fails += 1
                print(f"P4 FAIL {t}: the probe wrote no {mine}")
                continue
            lines, left_out = table_lines(t, [r for r in rows if r[1] not in listed], tsv_rows(mine)[1], head, omitted)
            seen |= set(lines)
            bad = [l for l in lines if l not in accepted]
            for l in bad:
                print(f"P4 FAIL {t}: {l}")
            fails += len(bad)
            print(f"P4 {t}: {len(lines) - len(bad)} accepted and {len(bad)} other differences from spec/abi/{t} "
                  f"({len(rows)} rows; {len(listed)} headers not shipped; {left_out} rows of reserved names the headers "
                  f"leave out by policy, headers/reserved.tsv)")
        # The ratchet: a difference this list accepts that the probe no longer shows is a stale line.
        for l in sorted(mine_accept - seen):
            fails += 1
            print(f"P4 FAIL {ACCEPT}: stale line (no longer a difference): {l}")
    print(f"P4: {'FAIL ' + str(fails) if fails else 'OK'}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
