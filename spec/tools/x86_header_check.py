#!/usr/bin/env python3
"""Check libniji's <sys/platform/x86.h> against spec/abi/x86-features.tsv (SL-HEADERS-S16-2).

    python3 -I spec/tools/x86_header_check.py [--include DIR]

Compiles with `-nostdinc -isystem <DIR> -isystem <the compiler's own include>` a program that defines its own
`__x86_get_cpuid_feature_leaf` (records the index it is called with, returns a structure it fills) and, for every
row of the table: with only the table's bit set in `cpuid_array` (or `active_array` + the same bit position) the
macros are true and called with the table's index, with every other single bit they are false; a row marked
`active=never` makes `CPU_FEATURE_ACTIVE` false for an all-ones structure. Also: both macros have type `_Bool`
(C11 `_Generic`), `sizeof` and `offsetof` of `struct cpuid_feature` equal layouts.tsv, and a C++17 unit
using every feature name compiles. Prints one FAIL line per problem; exit 1 on any.
"""
import argparse, os, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
REG = ["eax", "ebx", "ecx", "edx"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    a = ap.parse_args()
    rows = []
    for l in open(os.path.join(ROOT, "spec/abi/x86-features.tsv")):
        if l.startswith("#") or l.startswith("name\t") or not l.strip():
            continue
        f = l.rstrip("\n").split("\t")
        rows.append((f[0], int(f[1]), REG.index(f[4]), int(f[5]), f[6]))
    lay = {}
    for l in open(os.path.join(ROOT, "spec/abi/layouts.tsv")):
        f = l.rstrip("\n").split("\t")
        if f[0] == "struct cpuid_feature":
            lay[f[1]] = (int(f[2]), f[3])
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    flags = ["-nostdinc", "-isystem", a.include, "-isystem", gccinc]
    fails = 0
    c = '#include <sys/platform/x86.h>\n#include <string.h>\n'
    c += 'static unsigned last = ~0u; static struct cpuid_feature fake;\n'
    c += 'const struct cpuid_feature *__x86_get_cpuid_feature_leaf (unsigned int i) { last = i; return &fake; }\n'
    c += f'_Static_assert(sizeof(struct cpuid_feature) == {lay[""][0]}, "size");\n_Static_assert(_Alignof(struct cpuid_feature) == {lay[""][1]}, "align");\n'
    c += f'_Static_assert(__builtin_offsetof(struct cpuid_feature, cpuid_array) == {lay["cpuid_array"][0]}, "cpuid_array");\n'
    c += f'_Static_assert(__builtin_offsetof(struct cpuid_feature, active_array) == {lay["active_array"][0]}, "active_array");\n'
    c += '#define TY(e) _Generic((e), _Bool: 1, default: 0)\n'
    c += 'static int set(unsigned w, unsigned b) { memset(&fake, 0, sizeof fake); ((unsigned *)&fake)[w] = 1u << b; last = ~0u; return 0; }\n'
    c += 'static int one(const char *n, unsigned idx, unsigned reg, unsigned bit, int *got, unsigned *i) { (void)n; (void)idx; (void)reg; (void)bit; (void)got; (void)i; return 0; }\n'
    c += 'int main(void) {\n  int bad = 0;\n'
    for n, idx, reg, bit, act in rows:
        c += f'  _Static_assert(TY(CPU_FEATURE_PRESENT({n})) && TY(CPU_FEATURE_ACTIVE({n})), "type {n}");\n'
        c += f'  for (unsigned w = 0; w < 8; w++) for (unsigned b = 0; b < 32; b++) {{\n'
        c += f'    set(w, b); int p = CPU_FEATURE_PRESENT({n}); if (last != {idx}u) bad++; if (p != (w == {reg}u && b == {bit}u)) bad++;\n'
        if act == "yes":
            c += f'    set(w, b); int a = CPU_FEATURE_ACTIVE({n}); if (last != {idx}u) bad++; if (a != (w == {reg + 4}u && b == {bit}u)) bad++;\n'
        else:
            c += f'    set(w, b); int a = CPU_FEATURE_ACTIVE({n}); if (a) bad++;\n'
        c += '  }\n'
    c += '  return bad ? 1 : 0;\n}\n'
    with tempfile.TemporaryDirectory() as d:
        open(f"{d}/x.c", "w").write(c)
        r = subprocess.run(["gcc", "-std=gnu17", "-Wall", "-Wextra", "-Werror", "-Wno-unused-function", "-Wno-unused-parameter"] + flags + [f"{d}/x.c", "-o", f"{d}/x"], capture_output=True, text=True)
        if r.returncode:
            print("FAIL x86 compile:", r.stderr[:1500]); fails += 1
        else:
            r = subprocess.run([f"{d}/x"])
            if r.returncode:
                print(f"FAIL x86 run: macros disagree with x86-features.tsv (exit {r.returncode})"); fails += 1
        cxx = '#include <sys/platform/x86.h>\nint f(void) { return 0'
        cxx += "".join(f' + CPU_FEATURE_PRESENT({n}) + CPU_FEATURE_ACTIVE({n})' for n, *_ in rows) + '; }\n'
        open(f"{d}/y.cc", "w").write(cxx)
        for std in ("c++98", "c++17", "c++23"):
            r = subprocess.run(["g++", f"-std={std}", "-Wall", "-Wextra", "-Werror", "-fsyntax-only"] + flags + [f"{d}/y.cc"], capture_output=True, text=True)
            if r.returncode:
                print(f"FAIL x86 {std}:", r.stderr[:1000]); fails += 1
    print(f"x86_header_check: {len(rows)} features, {fails} failed")
    sys.exit(1 if fails else 0)


main()
