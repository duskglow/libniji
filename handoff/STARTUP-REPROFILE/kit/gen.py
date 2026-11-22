#!/usr/bin/env python3
"""LD-LOOKUP-COST probe generator: a C program whose main takes the address of N libc functions
through the GOT (one GLOB_DAT each), N names exported at a default version by both C libraries.
usage: gen.py NIJI_LIBC GLIBC_LIBC N > p.c   (N = 0: the empty main)"""
import re, subprocess, sys

def funcs(path):
    out = subprocess.run(["nm", "-D", "--defined-only", path], capture_output=True, text=True, check=True).stdout
    names = set()
    for line in out.splitlines():
        parts = line.split()
        if len(parts) != 3 or parts[1] not in ("T", "W", "i"):
            continue
        name = parts[2]
        if "@@" not in name:
            continue
        base = name.split("@@")[0]
        if re.fullmatch(r"[a-z][a-z0-9_]*", base):
            names.add(base)
    return names

n = int(sys.argv[3])
names = sorted(funcs(sys.argv[1]) & funcs(sys.argv[2]))
names = [x for x in names if x not in ("main",)][:n]
if len(names) < n:
    sys.exit(f"only {len(names)} common names")
for x in names:
    print(f"extern void {x}(void);")
print("int main(void) {\n    void *volatile s;")
for x in names:
    print(f"    s = (void *){x};")
print("    (void)s;\n    return 0;\n}")
