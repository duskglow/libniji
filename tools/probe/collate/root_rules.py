#!/usr/bin/env python3
"""Derive spec/locale/collate-root.txt (the shared root of libniji's generated
LC_COLLATE, coordinator decision #7958) from one probe of the reference.

usage: root_rules.py <allkeys.txt> <UnicodeData.txt> <probed.tsv> <out.txt>

<probed.tsv> is collorder's output for the reference's en_US.UTF-8 over
U+0001-U+D7FF and U+E000-U+10FFFF (one line `U+XXXX<TAB>level`, see README.md).
The model the rules complete (the generator in crates/collate-root implements it):
  * levels 1-3: DUCET 15.1 weights with variable weighting "shifted" (UTS #10
    section 4); missing code points take the implicit weights of UTS #10
    section 10.1;
  * level 4: code point order among characters equal at levels 1-3;
  * class han-variant-block: a code point whose first primary is an implicit Han
    primary and that is not in U+4E00-U+9FFF takes a primary before every
    implicit Han primary, then its own;
  * class format-first: all-zero DUCET weights and Cc, Cf or Variation_Selector
    first at level 4, in code point order.
What the model does not reproduce becomes a rule:
  * undefined: candidates (DUCET singles and Unified_Ideograph) the probe puts
    with the undefined characters, merged into ranges that never span a
    defined code point;
  * after: elements off one longest non-decreasing subsequence of the probed
    order under the model's keys (placed after their probed predecessor at the
    probed level), then elements whose level against the predecessor still
    differs, then level-4 ties the class order gets wrong.
The derivation simulates the generator and prints the order and level
mismatches left (0 and 0 for the committed file, apart from the probe's
level-5 lines, which are expansions next to the element).

SETTLED lists the lines the single code point order cannot decide; they were
settled by crates/locale/tests/reference.rs::generated_root_collation_is_the_
references_en_us (strings and fillers against the reference's compiled en_US)
and are written in place of the derived lines they replace.
Results: results/locale-collate-2026-11-23.md section 7. no spec clause: this is
a probe tool, evidence for the derivation, not a test.
"""
import bisect
import re
import sys
from collections import Counter, defaultdict

ELEM = re.compile(r"\[([.*])([0-9A-F]{4,5})\.([0-9A-F]{4})\.([0-9A-F]{4})\]")
MIN = (-1,)  # the minimal step appended to a weight at a level
VS = set(range(0x180B, 0x180E)) | {0x180F} | set(range(0xFE00, 0xFE10)) | set(range(0xE0100, 0xE01F0))

# Lines the test settled (see the docstring): derived `after` lines they replace,
# and the lines written instead, per output group.
SETTLED_REPLACE = {
    0x0675: "expand 0675 0627 0621",
    0x0676: "expand 0676 0648 0621",
    0x0678: "expand 0678 064A 0621",
}
SETTLED_APPEND = {"latin": ["same A784 A785 2"]}
SETTLED_LEVEL = {0xA784}  # code points whose probed level a SETTLED line gives (the `same` line)
CONTRACTIONS = """
# Contractions. Evidence: the generated_root test: the reference compares l+U+00B7 and L+U+00B7
# equal to U+0140 and U+013F (strcoll 0), and orders them so in words.
contract 006C 00B7 0140
contract 004C 00B7 013F
"""


def load(path):
    """single code point DUCET entries: cp -> [(variable, p, s, t)]"""
    raw = {}
    for line in open(path):
        line = line.split("#", 1)[0].strip()
        if not line or line.startswith("@"):
            continue
        cps, ce = line.split(";", 1)
        cps = cps.split()
        if len(cps) != 1:
            continue
        raw[int(cps[0], 16)] = [(v == "*", int(p, 16), int(s, 16), int(t, 16))
                                for v, p, s, t in ELEM.findall(ce)]
    return raw


def uni(cp):
    """(core, other) Unified_Ideograph classes of UTS #10 section 10.1"""
    core = (0x4E00 <= cp <= 0x9FFF) or cp in (
        0xFA0E, 0xFA0F, 0xFA11, 0xFA13, 0xFA14, 0xFA1F, 0xFA21, 0xFA23, 0xFA24, 0xFA27, 0xFA28, 0xFA29)
    other = any(lo <= cp <= hi for lo, hi in (
        (0x3400, 0x4DBF), (0x20000, 0x2A6DF), (0x2A700, 0x2B739), (0x2B740, 0x2B81D), (0x2B820, 0x2CEA1),
        (0x2CEB0, 0x2EBE0), (0x2EBF0, 0x2EE5D), (0x30000, 0x3134A), (0x31350, 0x323AF)))
    return core, other


def implicit(cp):
    core, other = uni(cp)
    base = 0xFB40 if core else 0xFB80 if other else 0xFBC0
    return [(False, base + (cp >> 15), 0x20, 0x2), (False, (cp & 0x7FFF) | 0x8000, 0, 0)]


def shifted_key(ces):
    """UTS #10 section 4 shifted: variable CEs -> (0,0,0, L4=primary); ignorables
    after a variable -> all zero; others L4 = FFFF (or 0 if completely ignorable)."""
    out = []
    after_var = False
    for var, p, s, t in ces:
        if var:
            out.append((0, 0, 0, p))
            after_var = True
        elif p == 0 and s == 0 and t == 0:
            out.append((0, 0, 0, 0))
        elif p == 0 and after_var:
            out.append((0, 0, 0, 0))
        else:
            out.append((p, s, t, 0xFFFF))
            after_var = False
    return tuple(tuple(e[i] for e in out if e[i]) for i in range(4))


def lnds_keep(vals):
    """indices of one longest non-decreasing subsequence"""
    tails, ti, prev = [], [], [-1] * len(vals)
    for i, v in enumerate(vals):
        j = bisect.bisect_right(tails, v)
        if j == len(tails):
            tails.append(v)
            ti.append(i)
        else:
            tails[j] = v
            ti[j] = i
        prev[i] = ti[j - 1] if j else -1
    keep = set()
    i = ti[-1] if ti else -1
    while i >= 0:
        keep.add(i)
        i = prev[i]
    return keep


def level(a, b):
    for i in range(3):
        if a[i] != b[i]:
            return i + 1
    return 4


def place(w, a, lv):
    """the key of an element placed after anchor a at level lv"""
    nk = list(w[a])
    if lv < 4:
        nk[lv - 1] = w[a][lv - 1] + MIN
    return tuple(nk)


def derive(allkeys, unicodedata, probed):
    raw = load(allkeys)
    gc = {}
    for line in open(unicodedata):
        f = line.split(";")
        gc[int(f[0], 16)] = f[2]
    ref = []
    for line in open(probed):
        cp, lv = line.rstrip("\n").split("\t")
        ref.append((int(cp[2:], 16), lv))
    # the undefined characters lead the probe (equal runs and their class steps)
    u = 1
    while u < len(ref) and (ref[u][1] == "=" or (
            ref[u][1] in "45" and u + 1 < len(ref) and ref[u + 1][1] == "=")):
        u += 1
    d = ref[u:]
    dset = {c for c, _ in d}
    # 1. undefined: candidates the probe does not define
    cand = set(raw) | {c for c in range(0x110000) if any(uni(c))}
    cand = {c for c in cand if not (0xD800 <= c <= 0xDFFF) and c != 0}
    assert dset <= cand, sorted(dset - cand)[:5]
    defl = sorted(dset)
    ranges = []
    for c in sorted(cand - dset):
        if ranges and bisect.bisect_left(defl, c) == bisect.bisect_left(defl, ranges[-1][1]):
            ranges[-1][1] = c
        else:
            ranges.append([c, c])
    # 2. the model's keys (levels 1-3), with class han-variant-block
    key = {c: shifted_key(raw.get(c) or implicit(c))[:3] for c in dset}
    nvar = 0
    for c in dset:
        k1 = key[c][0]
        if k1 and 0xFB40 <= k1[0] < 0xFBC0 and not 0x4E00 <= c <= 0x9FFF:
            key[c] = ((0xFB3F,) + k1,) + key[c][1:]
            nvar += 1
    seq = [c for c, _ in d]
    lev = dict(d)
    keep = lnds_keep([key[c] for c in seq])
    overrides = []
    for i, c in enumerate(seq):
        if i not in keep:
            overrides.append((c, seq[i - 1], int(lev[c]) if lev[c] in "1234" else 1))
    w = dict(key)
    ov = {c: (a, lv) for c, a, lv in overrides}
    for c in seq:
        if c in ov:
            w[c] = place(w, *ov[c])
    for i in range(1, len(seq)):
        a, c = seq[i - 1], seq[i]
        if c in ov or lev[c] == "5":
            continue
        got = level(w[a], w[c])
        want = int(lev[c])
        if want != got and (want < got or want == 4):
            w[c] = place(w, a, want)
            overrides.append((c, a, want))
            ov[c] = (a, want)
    # 3. level 4: class format-first, then code point order; ties it gets wrong
    zero = {c for c in dset if c in raw and all(p == s == t == 0 for _, p, s, t in raw[c])}
    fmt = {c for c in zero if gc.get(c) in ("Cc", "Cf") or c in VS}

    def tie(c):
        return (0 if c in fmt else 1, c)

    pos = {c: i for i, c in enumerate(seq)}
    groups = defaultdict(list)
    for c in seq:
        groups[w[c]].append(c)
    ties = []
    for m in groups.values():
        if len(m) < 2:
            continue
        r = sorted(m, key=lambda c: pos[c])
        want = {c: i for i, c in enumerate(sorted(m, key=tie))}
        kk = lnds_keep([want[c] for c in r])
        for i, c in enumerate(r):
            if i not in kk and c not in ov:
                assert i, "U+%04X: a level-4 move to the front of its group" % c
                ties.append((c, r[i - 1]))
    # simulate the generator: sort by the model, then apply the level-4 moves
    order = sorted(seq, key=lambda c: (w[c], tie(c)))
    moves = ties + [(c, a) for c, (a, lv) in ov.items() if lv == 4]
    moves.sort(key=lambda x: pos[x[0]])
    for c, a in moves:
        order.remove(c)
        order.insert(order.index(a) + 1, c)
    bad = sum(1 for i in range(len(seq)) if order[i] != seq[i])
    lv_bad = Counter()
    for i in range(1, len(order)):
        a, c = order[i - 1], order[i]
        got = str(level(w[a], w[c]))
        if got != lev[c] and lev[c] != "5":
            settled = c in SETTLED_LEVEL
            if not settled:
                lv_bad[(got, lev[c])] += 1
            print("level mismatch: U+%04X after U+%04X: model %s, probe %s%s"
                  % (c, a, got, lev[c], " (settled by a SETTLED line)" if settled else ""))
    print("defined=%d undefined-ranges=%d han-variants=%d format-first=%d after=%d ties=%d"
          % (len(dset), len(ranges), nvar, len(fmt), len(overrides), len(ties)))
    print("simulated: order mismatches=%d level mismatches=%s" % (bad, dict(lv_bad)))
    after = [(c, a, lv) for c, a, lv in overrides] + [(c, a, 4) for c, a in ties]
    return ranges, after, bad + sum(lv_bad.values())


GROUPS = ("han", "arabic", "latin", "marks", "format", "other")
LATIN = (0x20A7, 0xA75A, 0xA75B, 0x1DE3, 0x1D89, 0x1E9B, 0x1D98, 0xA785)
FORMAT = (0x0600, 0x0601, 0x0602, 0x0603, 0x0604, 0x0605, 0x06DD, 0x2061, 0x2062, 0x2063, 0x2064, 0x110BD)
HEAD = {
    "han": "U+FFFD sorts after the last CJK variant and before the unified ideographs (level 1).",
    "arabic": "Arabic letters with a high hamza and their presentation forms: one letter each, after the "
              "letter named (DUCET 15.1 expands them).",
    "latin": "Latin letters and the peseta sign whose DUCET 15.1 weights differ from the reference's "
             "(r rotunda, insular s, long s with dot, r with palatal hook, peseta sign as one currency symbol).",
    "format": "Prepended concatenation marks and invisible operators: Cf with all-zero DUCET weights that "
              "the reference orders by code point among the variable characters, not with class format-first.",
    "marks": "Combining marks the reference weights differently from DUCET 15.1 (level 4: same weights as "
             "the anchor; level 2: a secondary of their own).",
    "other": "Letters of other scripts placed after the letter named (Phaistos, Kharoshthi, Myanmar, "
             "Javanese, Hentaigana, Bopomofo).",
}
EVID = ("probe: tools/probe/collate/collorder.c over U+0001-U+D7FF and U+E000-U+10FFFF in the reference's "
        "en_US.UTF-8 (Zen 1 host, 2026-11-23; results/locale-collate-2026-11-23.md section 7)")
PREAMBLE = """# The shared root of libniji's generated LC_COLLATE (LOCALE-COLLATE-TAILORINGS-2; coordinator
# decisions #7890, #7958; owner ruling #7791: locale data by black-box probe). Input of
# crates/collate-root (niji-collate-root), which builds the root from the DUCET
# (spec/unicode/15.1.0/allkeys.txt, UTS #10) and this file:
#
#   * levels 1-3: the DUCET's weights with variable weighting "shifted" (UTS #10 section 4: a
#     variable collation element and the ignorables after it weigh nothing at levels 1-3);
#   * level 4: the character itself, characters equal at levels 1-3 in code point order;
#   * the class rules and the placements below, each with the probe that shows it.
#
# Every rule was derived from, and is checked against, one probe of the reference:
# @EVID@.
# tools/probe/collate/root_rules.py derives the undefined ranges and placements from that output.
#
# Formats:
#   class <name>                 a class rule the generator implements (described above it)
#   undefined <lo> <hi>          code points the DUCET or Unified_Ideograph covers that the root
#                                leaves undefined (they collate as UNDEFINED: first)
#   after <cp> <anchor> <level>  <cp> sorts directly after <anchor>, differing from it at
#                                <level> (1-3: a weight of its own just after the anchor's at that
#                                level, the anchor's below it; 4: the anchor's weights at 1-3)
#   same <cp> <anchor> <n>       <cp> has the anchor's weights at levels 1 to <n>
#   expand <cp> <cp1> <cp2>...   <cp> has the weights of the string <cp1><cp2>... at levels 1-3
#   contract <cp1> <cp2> <as>    the string <cp1><cp2> is one element weighing as <as> at all
#                                four levels
#
# The expand, same and contract lines were settled by the generated_root test (strings and
# fillers against the reference's compiled en_US: U+0675, U+0676 and U+0678 weigh as the letter
# and U+0621; U+017F, U+1DE5 and U+A785 are equal at levels 1-3 and U+A784 differs from them at
# level 3 only), not by the single code point order alone.

# Class han-variant-block. Evidence: the probe orders every CJK radical, Kangxi radical, CJK
# compatibility ideograph (U+F900-U+FAFF, U+2F800-U+2FA1F), parenthesized, circled, squared and
# kanbun ideograph after all other scripts and before U+4E00, in the DUCET's order among
# themselves (1,508 code points, no exception). Rule: a code point whose first DUCET primary is
# an implicit Han primary (UTS #10 section 10.1) and that is not in U+4E00-U+9FFF takes a
# primary before every implicit Han primary, then its DUCET primaries.
class han-variant-block

# Class format-first. Evidence: in the probe, the code points with all-zero DUCET weights that
# are controls (Cc), format characters (Cf) or variation selectors (PropList Variation_Selector)
# come first at level 4, in code point order, before the variable characters (467 code points;
# the exceptions are the `after` lines of the format group below).
class format-first

# Undefined. Evidence: the probe puts these code points with the undefined characters (they sort
# first, equal to each other within a UTF-8 length): the reference's table does not define them.
"""


def group(c, lv):
    if c == 0xFFFD:
        return "han"
    if c in LATIN:
        return "latin"
    if lv == 4 and c in FORMAT:
        return "format"
    if 0x0600 <= c <= 0x06FF or (0xFB50 <= c <= 0xFEFF and lv != 4):
        return "arabic"
    if lv == 4 or c == 0x101FD:
        return "marks"
    return "other"


def write(out, ranges, after):
    by = {k: [] for k in GROUPS}
    for c, a, lv in after:
        by[group(c, lv)].append(SETTLED_REPLACE.get(c, "after %04X %04X %d" % (c, a, lv)))
    for k, lines in SETTLED_APPEND.items():
        by[k] += lines
    with open(out, "w") as f:
        f.write(PREAMBLE.replace("@EVID@", EVID))
        for lo, hi in ranges:
            f.write("undefined %04X %04X\n" % (lo, hi))
        for k in GROUPS:
            if by[k]:
                f.write("\n# Placements: %s Evidence: the probe's order and level for each line.\n" % HEAD[k])
                f.write("".join(line + "\n" for line in by[k]))
            if k == "latin":
                f.write(CONTRACTIONS)


def main():
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    ranges, after, left = derive(*sys.argv[1:4])
    write(sys.argv[4], ranges, after)
    sys.exit(1 if left else 0)


main()
