#!/usr/bin/env python3
"""STARTUP-REPROFILE leg 2: release-product phase rows that phase.py cannot name (inlined in release).
usage: relphase.py FASTDIR RELDIR prog...
Object::new is a function in release (with_style + GnuHash::symbol_count inlined into it).
check_versions is inlined into link in release: reconstructed as
  (release link exclusive - fast link exclusive) + Verdefs::next called from link
  + fast check_versions' other callees (Dynamic::once, bytes_from, equal, Verneeds/Vernauxes::next)."""
import sys


def load(p):
    d, e, tot = {}, {}, 0
    for line in open(p + ".fn.tsv"):
        if line.startswith("#"):
            tot = int(line.split()[-1])
            continue
        i, x, s = line.rstrip("\n").split("\t")
        d[s] = (int(i), int(x))
    for line in open(p + ".edge.tsv"):
        f = line.rstrip("\n").split("\t")
        if len(f) == 3:
            e[(f[1], f[2])] = e.get((f[1], f[2]), 0) + int(f[0])
    return d, e, tot


LINK = "niji_boundary::ld::link::link"
CV = "niji_ld::reloc::check_versions"
VD = "<niji_ld::elf::Verdefs as core::iter::traits::iterator::Iterator>::next"
fastdir, reldir = sys.argv[1], sys.argv[2]
for p in sys.argv[3:]:
    fd, fe, ft = load(f"{fastdir}/pt-{p}")
    rd, re_, rt = load(f"{reldir}/pt-{p}")
    on = rd.get("<niji_ld::symbols::Object>::new", (0, 0))
    ws = fd.get("<niji_ld::symbols::Object>::with_style", (0, 0))
    sc = fd.get("<niji_ld::elf::GnuHash>::symbol_count", (0, 0))
    other = sum(v for (callee, caller), v in fe.items() if caller == CV and callee != VD)
    cv = rd[LINK][1] - fd[LINK][1] + re_.get((VD, LINK), 0) + other
    print(f"{p}: total fast {ft:,} release {rt:,}; Object::new incl {on[0]:,} own {on[1]:,} "
          f"(fast with_style incl {ws[0]:,} own {ws[1]:,} + symbol_count {sc[0]:,} = own {ws[1] + sc[0]:,}); "
          f"check_versions fast {fd.get(CV, (0, 0))[0]:,} release reconstructed {cv:,}; "
          f"link own fast {fd[LINK][1]:,} release {rd[LINK][1]:,}")
