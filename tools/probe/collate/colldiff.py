#!/usr/bin/env python3
"""Express one locale's probed collation order as a tailoring of a root order.

usage: colldiff.py <root.tsv> <locale.tsv> [--rules]
Both inputs are collorder outputs (U+XXXX<TAB>level, in collation order).
The elements kept in place are a longest common subsequence of the two orders
(both are permutations of one code point set, so the LCS is a longest
increasing subsequence of root positions, O(n log n)). Every other element is a
moved element: one rule `&<anchor> <level> <cp>` (anchor = the element before
it in the locale's order). A kept element whose level against its predecessor
changed is a level rule `=<cp> <old>-><new>`. Prints a one-line summary; with
--rules, the rules too.  No spec clause (probe tooling).
"""
import bisect
import sys


def load(path):
    seq = []
    with open(path) as f:
        for line in f:
            cp, lv = line.rstrip("\n").split("\t")
            seq.append((int(cp[2:], 16), lv))
    return seq


def lis_mask(vals):
    """Indices (into vals) of one longest strictly increasing subsequence."""
    tails, tails_idx, prev = [], [], [-1] * len(vals)
    for i, v in enumerate(vals):
        j = bisect.bisect_left(tails, v)
        if j == len(tails):
            tails.append(v)
            tails_idx.append(i)
        else:
            tails[j] = v
            tails_idx[j] = i
        prev[i] = tails_idx[j - 1] if j else -1
    keep = set()
    i = tails_idx[-1] if tails_idx else -1
    while i >= 0:
        keep.add(i)
        i = prev[i]
    return keep


def main():
    root, loc = load(sys.argv[1]), load(sys.argv[2])
    rpos = {cp: i for i, (cp, _) in enumerate(root)}
    rlev = dict(root)
    if set(rpos) != {cp for cp, _ in loc}:
        print("code point sets differ", file=sys.stderr)
        return 2
    keep = lis_mask([rpos[cp] for cp, _ in loc])
    rules, moved, relevel = [], 0, 0
    for i, (cp, lv) in enumerate(loc):
        anchor = "U+%04X" % loc[i - 1][0] if i else "BEGIN"
        if i not in keep:
            moved += 1
            rules.append("&%s %s U+%04X" % (anchor, lv, cp))
        else:
            # a kept element whose level against its predecessor changed
            if lv != rlev[cp]:
                relevel += 1
                rules.append("=%s %s U+%04X (root %s)" % (anchor, lv, cp, rlev[cp]))
    name = sys.argv[2].rsplit("/", 1)[-1].rsplit(".", 1)[0]
    print("%s\tn=%d\tmoved=%d\trelevel=%d\trules=%d" % (name, len(loc), moved, relevel, len(rules)))
    if "--rules" in sys.argv:
        for r in rules:
            print(r)
    return 0


if __name__ == "__main__":
    sys.exit(main())
