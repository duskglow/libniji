#!/usr/bin/env python3
"""Order a probed code point set by the DUCET (UTS #10 allkeys.txt), in
collorder's output format, so colldiff can compare a probed root against it.

usage: ducet_order.py <allkeys.txt> <probed.tsv> <out.tsv> [shifted]
Only code points with a non-zero primary in the probed order (those after its
first level-1 line) and an explicit or implicit DUCET weight are kept; the same
set is written to <out>.probed.tsv in probed order so the two are comparable.
Implicit weights for Han and unassigned code points follow UTS #10 section 10.1
(Unified_Ideograph core/other bases FB40/FB80, FBC0 otherwise).
Variable weighting: non-ignorable by default; 'shifted' makes variable
collation elements ignorable at levels 1-3 (UTS #10 section 4, without the
fourth-level weight).  No spec clause (probe tooling).
"""
import re
import sys

ELEM = re.compile(r"\[([.*])([0-9A-F]{4,5})\.([0-9A-F]{4})\.([0-9A-F]{4})\]")


def load_ducet(path, shifted):
    keys = {}
    with open(path) as f:
        for line in f:
            line = line.split("#", 1)[0].strip()
            if not line or line.startswith("@"):
                continue
            cps, ce = line.split(";", 1)
            cps = cps.split()
            if len(cps) != 1:
                continue
            elems = []
            for var, p, s, t in ELEM.findall(ce):
                p, s, t = int(p, 16), int(s, 16), int(t, 16)
                if shifted and var == "*":
                    p = s = t = 0
                elems.append((p, s, t))
            keys[int(cps[0], 16)] = elems
    return keys


def unified_ideograph(cp):
    core = (0x4E00 <= cp <= 0x9FFF) or cp in (
        0xFA0E, 0xFA0F, 0xFA11, 0xFA13, 0xFA14, 0xFA1F, 0xFA21, 0xFA23, 0xFA24, 0xFA27, 0xFA28, 0xFA29)
    other = (0x3400 <= cp <= 0x4DBF) or (0x20000 <= cp <= 0x323AF)
    return core, other


def implicit(cp):
    core, other = unified_ideograph(cp)
    base = 0xFB40 if core else 0xFB80 if other else 0xFBC0
    return [(base + (cp >> 15), 0x20, 0x2), ((cp & 0x7FFF) | 0x8000, 0, 0)]


def sortkey(elems):
    return tuple(tuple(e[i] for e in elems if e[i]) for i in range(3))


def main():
    shifted = len(sys.argv) > 4 and sys.argv[4] == "shifted"
    ducet = load_ducet(sys.argv[1], shifted)
    probed = []
    with open(sys.argv[2]) as f:
        for line in f:
            cp, lv = line.rstrip("\n").split("\t")
            probed.append((int(cp[2:], 16), lv))
    first = next(i for i, (_, lv) in enumerate(probed) if lv == "1" and i > 0)
    mode = sys.argv[5] if len(sys.argv) > 5 else ""
    if mode == "ref-defined":
        # the probed order starts with the undefined classes: runs of '=' whose
        # heads differ from each other only at level 4; stop at the first
        # element that is neither a member nor a head followed by a member
        u = 1
        while u < len(probed) and (probed[u][1] == "=" or (
                probed[u][1] in "45" and u + 1 < len(probed) and probed[u + 1][1] == "=")):
            u += 1
        undefined = {cp for cp, _ in probed[:u]}
        print("reference-undefined=%d" % len(undefined))
        keep = [(cp, lv) for cp, lv in probed[u:] if cp in ducet or any(unified_ideograph(cp))]
        missing = [cp for cp, _ in probed[u:] if not (cp in ducet or any(unified_ideograph(cp)))]
        print("reference-defined but not in DUCET: %d e.g. %s" % (
            len(missing), " ".join("U+%04X" % c for c in missing[:8])))
        ext = [cp for cp in undefined if cp in ducet]
        print("in DUCET but reference-undefined: %d e.g. %s" % (
            len(ext), " ".join("U+%04X" % c for c in sorted(ext)[:8])))
    elif mode == "all-defined":
        # every code point DUCET defines (explicitly, or Han by implicit weight)
        keep = [(cp, lv) for cp, lv in probed if cp in ducet or any(unified_ideograph(cp))]
    else:
        keep = [(cp, lv) for cp, lv in probed[first:]]
    out = []
    for cp, _ in keep:
        ce = ducet.get(cp) or implicit(cp)
        out.append((sortkey(ce), cp))
    out.sort()
    with open(sys.argv[3], "w") as f:
        prev = None
        for k, cp in out:
            if prev is None:
                lv = "1"
            elif k == prev:
                lv = "="
            else:
                lv = next(str(i + 1) for i in range(3) if k[i] != prev[i])
            f.write("U+%04X\t%s\n" % (cp, lv))
            prev = k
    with open(sys.argv[3] + ".probed.tsv", "w") as f:
        for cp, lv in keep:
            f.write("U+%04X\t%s\n" % (cp, lv))
    print("kept=%d dropped-before-first-primary=%d in-ducet=%d" % (
        len(keep), first, sum(1 for cp, _ in keep if cp in ducet)))


if __name__ == "__main__":
    sys.exit(main())
