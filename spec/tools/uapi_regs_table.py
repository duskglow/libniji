#!/usr/bin/env python3
"""spec/abi rows for sys/reg.h (register indexes) and sys/debugreg.h (DR_ names), SL-HEADERS-S09 (ruling #6399).

Both headers are kernel ABI, so the kernel wins and the names come from the kernel, never from a libc header:

* sys/reg.h      the index of each member of `struct user_regs_struct`, named by the member in upper case
                 (`r15` is `R15`, `orig_rax` is `ORIG_RAX`): offset / 8 of the member rows of layouts.tsv, whose
                 first 21 members are the kernel UAPI `struct pt_regs` of asm/ptrace.h in the same order.
* sys/debugreg.h every `#define DR_*` / `DR6_*` of the kernel UAPI header asm/debugreg.h, with the value and the type
                 the compiler gives it there.

usage: uapi_regs_table.py emit  [--uapi DIR]   print the rows (name, header, type, value) that macros.tsv/constants.tsv take
       uapi_regs_table.py apply [--uapi DIR]   splice the rows the probe accepts into spec/abi/macros.tsv and constants.tsv
       uapi_regs_table.py probe [--uapi DIR]   black-box check of the reference values: for each row compile a unit that
                                                includes the reference's header and asserts the macro exists, its value
                                                and its type; only the exit status is read. A row it does not accept is listed.

DIR defaults to /usr/include/x86_64-linux-gnu (the kernel UAPI of the linux-libc-dev package).
"""
import argparse
import os
import re
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ABI = os.path.join(ROOT, "spec", "abi")
TYPES = [("int", "int"), ("unsigned int", "unsigned int"), ("long", "long"), ("unsigned long", "unsigned long")]


def tsv(name):
    with open(os.path.join(ABI, name), encoding="utf-8") as f:
        return [l.rstrip("\n").split("\t") for l in f if l.strip() and not l.startswith("#")]


def reg_rows():
    rows = []
    for r in tsv("layouts.tsv"):
        if r[0] == "struct user_regs_struct" and r[1]:
            rows.append((r[1].upper(), "sys/reg.h", "int", str(int(r[2]) // 8)))
    return rows


def debugreg_rows(uapi):
    path = os.path.join(uapi, "asm", "debugreg.h")
    names = []
    for line in open(path, encoding="utf-8"):
        m = re.match(r"#define\s+(DR6?_[A-Za-z0-9_]+)\s", line)
        if m and m.group(1) not in names:
            names.append(m.group(1))
    gen = ["#include <stdio.h>", "#include <asm/debugreg.h>", "int main(void) {"]
    for n in names:
        sel = "_Generic((%s), int: 0, unsigned int: 1, long: 2, unsigned long: 3, default: 9)" % n
        gen.append('printf("%s %%d %%llu\\n", %s, (unsigned long long)(%s));' % (n, sel, n))
    gen += ["return 0; }"]
    with tempfile.TemporaryDirectory() as d:
        src, exe = os.path.join(d, "u.c"), os.path.join(d, "u")
        open(src, "w").write("\n".join(gen) + "\n")
        subprocess.run(["gcc", "-I", uapi, "-w", "-o", exe, src], check=True)
        out = subprocess.run([exe], capture_output=True, text=True, check=True).stdout
    rows = []
    for line in out.splitlines():
        n, t, v = line.split()
        rows.append((n, "sys/debugreg.h", TYPES[int(t)][1], v))
    return rows


def constant_value(typ, value):
    """constants.tsv holds the value as a signed 64-bit integer."""
    v = int(value)
    return str(v - (1 << 64) if v >= 1 << 63 else v)


def all_rows(uapi):
    return reg_rows() + debugreg_rows(uapi)


def splice(rel, rows, cols):
    path = os.path.join(ABI, rel)
    lines = open(path, encoding="utf-8").read().split("\n")
    assert lines[-1] == ""
    head, body = lines[0], lines[1:-1]
    keys = [l.split("\t")[0].encode() for l in body]
    assert keys == sorted(keys), rel + " is not sorted by name"
    have = {l.split("\t")[0]: l for l in body}
    for r in rows:
        line = "\t".join(r)
        if r[0] in have and have[r[0]] != line:
            sys.exit("%s: %s is already a different row: %s" % (rel, r[0], have[r[0]]))
        have[r[0]] = line
    body = [have[k] for k in sorted(have, key=lambda k: k.encode())]
    open(path, "w", encoding="utf-8").write("\n".join([head] + body) + "\n")


def probe(rows):
    """The reference's own headers are compiled against; the exit status is the only thing read."""
    ref = {"sys/reg.h": "sys/reg.h", "sys/debugreg.h": "sys/debugreg.h"}
    bad = []
    for name, header, typ, value in rows:
        code = {"int": 0, "unsigned int": 1, "long": 2, "unsigned long": 3}[typ]
        with tempfile.TemporaryDirectory() as d:
            src = os.path.join(d, "p.c")
            open(src, "w").write(
                "#include <%s>\n#ifndef %s\n#error absent\n#endif\n"
                "_Static_assert((unsigned long long)(%s) == %sULL, \"v\");\n"
                "_Static_assert(_Generic((%s), int: 0, unsigned int: 1, long: 2, unsigned long: 3, default: 9) == %d, \"t\");\n"
                % (ref[header], name, name, value, name, code))
            rc = subprocess.run(["gcc", "-std=gnu17", "-c", "-o", os.devnull, src], capture_output=True).returncode
        if rc != 0:
            bad.append((name, header, typ, value))
    return bad


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("mode", choices=["emit", "apply", "probe"])
    ap.add_argument("--uapi", default="/usr/include/x86_64-linux-gnu")
    args = ap.parse_args()
    rows = all_rows(args.uapi)
    if args.mode == "emit":
        for r in rows:
            print("\t".join(r))
    elif args.mode == "probe":
        bad = probe(rows)
        for r in bad:
            print("not accepted by the reference: " + "\t".join(r))
        print("uapi_regs_table: %d rows, %d not accepted" % (len(rows), len(bad)))
        sys.exit(1 if bad else 0)
    else:
        # The reference is the ABI: a kernel name its header does not give (with this value and type) stays out.
        bad = set(probe(rows))
        rows = [r for r in rows if r not in bad]
        for r in sorted(bad):
            print("left out (the reference does not give it): " + "\t".join(r))
        splice("macros.tsv", rows, 4)
        splice("constants.tsv", [(n, constant_value(t, v)) for n, h, t, v in rows], 2)
        print("uapi_regs_table: %d rows spliced" % len(rows))


if __name__ == "__main__":
    main()
