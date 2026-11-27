#!/usr/bin/env python3
"""CONFIGURE-DIFF diff: compare <run>/probes/<pkg>.{stockA,stockB,niji}.tsv per probe.

  noisy      stockA != stockB (the stock-vs-stock control): the probe is not deterministic, never counted as a
             libniji difference (listed in noise.tsv so a later normalisation can remove it)
  differing  stockA == stockB != niji (a missing key counts as "<absent>")
Writes <run>/diff.tsv (pkg, class, key, stock, libniji), <run>/noise.tsv, <run>/summary.txt; prints the summary.
Class is a first guess from the probe's name (header / decl / func / type / lib / other); the report says what each
libniji row really is after reading the probe's test program.
Usage: diff.py <run dir> [--sides A,B]   (--sides stockA,stockB = the stock-vs-stock run, expected 0)
"""
import os
import re
import sys

ABSENT = "<absent>"
VOLATILE = re.compile(r"^(ac:[^:]*:ac_cv_env_|ac:[^:]*:(ac_cv_build|ac_cv_host|ac_cv_target)$)")


def load(path):
    d = {}
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            k, _, v = line.rstrip("\n").partition("\t")
            d[k] = v
    return d


def klass(key):
    var = key.rsplit(":", 1)[-1]
    lk = key.lower()
    if re.search(r"ac_cv_header_|has header|_h$|^have_.*_h$", lk) or re.search(r"HAVE_\w+_H$", var):
        return "header"
    if re.search(r"have_decl_|has symbol|value of define|_decl$|decl_", lk):
        return "decl"
    if re.search(r"ac_cv_type_|sizeof|size of|alignment of|ac_cv_member_|has member|_type$|type \"|alignof", lk):
        return "type"
    if re.search(r"ac_cv_lib_|ac_cv_search_|^acout:|library ", lk):
        return "lib"
    if re.search(r"ac_cv_func_|checking for function|have_[a-z0-9_]+$|^perl:.*:d_", lk) or var.startswith("HAVE_"):
        return "func"
    return "other"


def status(run, p):
    st = {}
    path = os.path.join(run, "pkgs", p, "out", "status.txt")
    if os.path.exists(path):
        for line in open(path, errors="replace"):
            m = re.match(r"cdiff: configure (\w+) rc=(\d+) files=(\d+)", line)
            if m:
                st[m.group(1)] = "rc=%s files=%s" % (m.group(2), m.group(3))
            m = re.match(r"cdiff: source .* got=(\S*)", line)
            if m:
                st["version"] = m.group(1)
            if line.startswith("cdiff: swap rc="):
                st["swap"] = line.split()[2]
    return st


def main():
    run = sys.argv[1]
    sides = ("stockA", "stockB", "niji")
    tag = ""
    if "--sides" in sys.argv:
        a, b = sys.argv[sys.argv.index("--sides") + 1].split(",")
        sides = (a, a, b)
        tag = "-%s-%s" % (a, b)
    pkgs = sorted(os.listdir(os.path.join(run, "pkgs")))
    rows, noise, summ = [], [], []
    tot = [0, 0, 0, 0]
    for p in pkgs:
        A, B, N = (load(os.path.join(run, "probes", "%s.%s.tsv" % (p, s))) for s in sides)
        st = status(run, p)
        if A is None or N is None:
            summ.append("%-28s NODATA %s" % (p, " ".join("%s:%s" % kv for kv in sorted(st.items()))))
            continue
        B = B if B is not None else A
        keys = sorted(set(A) | set(B) | set(N))
        nn = nd = 0
        for k in keys:
            if VOLATILE.match(k):
                continue
            a, b, n = A.get(k, ABSENT), B.get(k, ABSENT), N.get(k, ABSENT)
            if a != b:
                nn += 1
                noise.append((p, k, a, b))
            elif n != a:
                nd += 1
                rows.append((p, klass(k), k, a, n))
        prim = sum(1 for r in rows if r[0] == p and r[2].split(":", 1)[0] in ("ac", "meson", "cmake", "perl"))
        tot[0] += 1; tot[1] += len(keys); tot[2] += nn; tot[3] += nd
        summ.append("%-28s probes=%-6d noisy=%-4d differing=%-4d primary=%-4d %s" % (
            p, len(keys), nn, nd, prim, " ".join("%s:%s" % kv for kv in sorted(st.items()))))
    with open(os.path.join(run, "diff%s.tsv" % tag), "w") as f:
        f.write("pkg\tclass\tprobe\tstock\tlibniji\n")
        for r in rows:
            f.write("\t".join(r) + "\n")
    with open(os.path.join(run, "noise%s.tsv" % tag), "w") as f:
        for r in noise:
            f.write("\t".join(r) + "\n")
    summ.append("TOTAL packages=%d probes=%d noisy=%d differing=%d" % tuple(tot))
    with open(os.path.join(run, "summary%s.txt" % tag), "w") as f:
        f.write("\n".join(summ) + "\n")
    print("\n".join(summ[-60:]))


if __name__ == "__main__":
    main()
