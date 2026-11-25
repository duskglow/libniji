#!/usr/bin/env python3
"""Write spec/abi/kernel-spelled.tsv: the macros libniji's headers define that a kernel UAPI header also defines with
different replacement text (comments and whitespace normalised), one row per name with the first kernel header.

Why (HEADERS-FEATURES-FORMS): the generator defines a macro outright where the reference's definition replaces a
program's earlier one (macro-forms.tsv). A kernel header that defines the same name first in another spelling (one
libniji's header includes, like <asm/ioctls.h> from <sys/ioctl.h>, or one a program includes first) would then meet a
second outright definition: a redefinition. These names stay under `#ifndef` (the kernel's definition, when present,
is the one in force). Input: the kernel UAPI headers `cargo xtask so` stages (plan/headers.md 2.4); every definition in
libc-include/, guarded or not, so the table does not depend on the generator's own choice.

    python3 spec/tools/kernel_spelled.py [--uapi DIR]   (default target/niji/uapi; run after `cargo xtask so`)
"""
import argparse
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DEF = re.compile(r"^\s*#\s*define\s+([A-Za-z_]\w*)(\([^)]*\))?\s*(.*?)\s*$")


def norm(v):
    return " ".join(re.sub(r"/\*.*?\*/", " ", v).replace("\\", " ").split())


def scan(base):
    out = {}
    for d, dirs, fs in os.walk(base):
        dirs.sort()
        for f in sorted(x for x in fs if x.endswith(".h")):
            p = os.path.join(d, f)
            with open(p, errors="replace") as fh:
                for line in fh:
                    m = DEF.match(line)
                    if m:
                        out.setdefault(m.group(1), []).append((os.path.relpath(p, base), (m.group(2) or "") + norm(m.group(3))))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--uapi", default=os.path.join(ROOT, "target/niji/uapi"))
    a = ap.parse_args()
    niji, kern = scan(os.path.join(ROOT, "libc-include")), scan(a.uapi)
    rows = []
    for n in sorted(set(niji) & set(kern)):
        if {v for _, v in niji[n]} - {v for _, v in kern[n]}:
            rows.append(f"{n}\t{sorted(h for h, _ in kern[n])[0]}")
    with open(os.path.join(ROOT, "spec/abi/kernel-spelled.tsv"), "w") as f:
        f.write("name\tkernel_header\n" + "".join(r + "\n" for r in rows))
    print(f"kernel-spelled: {len(rows)} names")


if __name__ == "__main__":
    main()
