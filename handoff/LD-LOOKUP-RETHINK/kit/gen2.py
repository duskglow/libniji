#!/usr/bin/env python3
"""LD-LOOKUP-RETHINK scenario sources. usage: gen2.py OUTDIR N
Libraries (no C library dependency, built -nostdlib), N functions each unless said:
  la_s  short names (8 bytes, 'sa%06d'), no versions        la_l  long names (40 bytes), no versions
  la_v  short names 'sv%06d' at version VA_1 (version script la_v.map)
  lx1..lx3  unrelated short names (fillers a lookup walks past)
  lm   N GLOB_DAT references to weak undefined short names (every lookup misses); lml: long names; lm0: none
Programs: a0 (empty main, no imports), a/l/v (main takes the address of the N functions of la_s/la_l/la_v
through the GOT: one GLOB_DAT each)."""
import os, sys

out, n = sys.argv[1], int(sys.argv[2])
os.makedirs(out, exist_ok=True)

def names(kind):
    if kind == "s": return ["sa%06d" % i for i in range(n)]
    if kind == "v": return ["sv%06d" % i for i in range(n)]
    if kind == "l": return ["lb_" + "q" * 30 + "%07d" % i for i in range(n)]
    if kind.startswith("x"): return ["x%s%06d" % (kind[1], i) for i in range(n)]
    if kind == "m": return ["mz%06d" % i for i in range(n)]
    if kind == "ml": return ["mw_" + "k" * 30 + "%07d" % i for i in range(n)]
    raise SystemExit(kind)

def write(name, text):
    with open(os.path.join(out, name), "w") as f:
        f.write(text)

for kind, lib in (("s", "la_s"), ("l", "la_l"), ("v", "la_v"), ("x1", "lx1"), ("x2", "lx2"), ("x3", "lx3")):
    write(lib + ".c", "".join("int %s(void) { return %d; }\n" % (x, i) for i, x in enumerate(names(kind))))
write("la_v.map", "VA_1 { global: sv*; local: *; };\n")

def refs(ns, weak):
    decl = "".join("extern void %s(void)%s;\n" % (x, " __attribute__((weak))" if weak else "") for x in ns)
    body = "".join("    s = (void *)%s;\n" % x for x in ns)
    return decl, body

for kind, lib in (("m", "lm"), ("ml", "lml")):
    decl, body = refs(names(kind), True)
    write(lib + ".c", decl + "void *%s_take(void) {\n    void *volatile s = 0;\n%s    return s;\n}\n" % (lib, body))
write("lm0.c", "void *lm0_take(void) { return 0; }\n")

write("a0.c", "int main(void) { return 0; }\n")
for kind, prog in (("s", "a"), ("l", "l"), ("v", "v")):
    decl, body = refs(names(kind), False)
    write(prog + ".c", decl + "int main(void) {\n    void *volatile s;\n%s    (void)s;\n    return 0;\n}\n" % body)
