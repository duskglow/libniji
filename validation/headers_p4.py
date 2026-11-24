#!/usr/bin/env python3
"""P4 of plan/headers.md (slice 20): the C++ check of libniji's headers.

  python3 validation/headers_p4.py [--include DIR] [--uapi DIR] [--jobs N] [--out DIR] [--no-noexcept]

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
   alignments are P1, spec/tools/header_check.py). --no-noexcept skips this part.

Prints one line per -std ("P4 c++17: 141 compiled, 3 n/a, 0 FAIL") and a summary; exit 1 on any FAIL.
No spec clause: header tooling (plan/headers.md P4).
"""
import argparse, concurrent.futures as cf, filecmp, os, re, subprocess, sys, tempfile

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
    a = ap.parse_args()
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
    if compile_one([cxx, "-std=c++26"], "int main() { return 0; }\n")[0] != 0:
        STDS[STDS.index("c++26")] = "c++2c"
    niji = ["-nostdinc"] + [w for d in dirs for w in ("-isystem", d)] + ["-isystem", inc, "-isystem", gccinc, "-isystem", uapi]
    jobs = {}
    fails = 0
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        for std in STDS:
            for h in CXX:
                text = f"#include <{h}>\nint main() {{ return 0; }}\n"
                jobs[(std, h, "ref")] = ex.submit(compile_one, [cxx, f"-std={std}"], text)
                jobs[(std, h, "niji")] = ex.submit(compile_one, [cxx, f"-std={std}"] + niji, text)
        for std in STDS:
            ok = na = bad = 0
            for h in CXX:
                ref, _ = jobs[(std, h, "ref")].result()
                rc, err = jobs[(std, h, "niji")].result()
                if ref != 0:
                    na += 1
                    if rc == 0:
                        print(f"P4 NOTE {std} <{h}>: compiles on libniji's headers only")
                elif rc == 0:
                    ok += 1
                else:
                    bad += 1
                    errs = [l for l in err.splitlines() if ": error:" in l and l.startswith(inc)][:3]
                    print(f"P4 FAIL {std} <{h}>: " + (" | ".join(errs) if errs else "(no error in libniji's headers; "
                          + (err.splitlines() or ["?"])[0][:200] + ")"))
            fails += bad
            print(f"P4 {std}: {ok} compiled, {na} n/a, {bad} FAIL")
    if not a.no_noexcept:
        out = a.out or tempfile.mkdtemp(prefix="p4-")
        os.makedirs(out, exist_ok=True)
        p = subprocess.run([sys.executable, "spec/tools/callsite_probe.py", "--include", inc, "--out", out,
                            "--jobs", str(a.jobs)], capture_output=True, text=True)
        if p.returncode != 0:
            print("P4 FAIL callsite_probe.py --include: " + (p.stderr.strip().splitlines() or ["?"])[-1])
            fails += 1
        for t in ("noexcept.tsv", "callsites.tsv"):
            mine, ref = os.path.join(out, t), os.path.join("spec/abi", t)
            if os.path.isfile(mine) and filecmp.cmp(mine, ref, shallow=False):
                print(f"P4 {t}: reproduced on libniji's headers")
            else:
                fails += 1
                print(f"P4 FAIL {t}: differs from spec/abi/{t} (diff {ref} {mine})")
    print(f"P4: {'FAIL ' + str(fails) if fails else 'OK'}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
