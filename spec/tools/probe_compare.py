#!/usr/bin/env python3
"""Compare a probe rerun on libniji's headers with the reference tables (plan/headers.md section 3, P2 and P3).

    python3 spec/tools/probe_compare.py visibility|callsites --got DIR --headers a.h,b.h [--accept FILE]

`--got DIR` holds the visibility.tsv (header_probe.py --include) or callsites.tsv (callsite_probe.py --include)
of the rerun. Only rows of the named headers are compared: a row of spec/abi that the rerun lacks, a row the rerun
adds, and a row whose configurations differ are printed, one line each, and make the exit status 1 unless the
line is listed in `--accept` (one printed line per row, `#` comments; the accepted lines are the differences rows
of docs/differences-from-glibc.md that a header of the slice carries).
"""
import argparse
import csv
import os
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
KEYS = {"visibility": ("header", "name", "kind"), "callsites": ("header", "name", "mode", "symbol")}


def reserved_omitted():
    """Names the generator leaves out by policy (C 7.1.3; owner rulings R-H2, R-H3): `__x` names and include guards,
    except those headers/reserved.tsv emits. They are not part of the comparison."""
    with open(os.path.join(ROOT, "headers", "reserved.tsv"), newline="") as f:
        emit = {r["name"] for r in csv.DictReader((l for l in f if not l.startswith("#")), delimiter="\t") if r["policy"] == "emit"}
    def omitted(n):
        return n not in emit and (n.startswith("__") or (n.startswith("_") and n.endswith("_H")))
    return omitted


def load(path, table, headers):
    omitted = reserved_omitted()
    with open(path, newline="") as f:
        rows = list(csv.DictReader(f, delimiter="\t"))
    out = {}
    for r in rows:
        if r["header"] in headers and not omitted(r["name"]):
            out[tuple(r[k] for k in KEYS[table])] = r["configs"]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("table", choices=sorted(KEYS))
    ap.add_argument("--got", required=True)
    ap.add_argument("--headers", required=True)
    ap.add_argument("--accept", default="")
    args = ap.parse_args()
    headers = set(args.headers.split(","))
    want = load(os.path.join(ROOT, "spec", "abi", args.table + ".tsv"), args.table, headers)
    got = load(os.path.join(args.got, args.table + ".tsv"), args.table, headers)
    accepted = set()
    if args.accept:
        with open(args.accept) as f:
            accepted = {l.rstrip("\n") for l in f if l.strip() and not l.startswith("#")}
    lines = []
    for k in sorted(want.keys() | got.keys()):
        if k not in got:
            lines.append(f"missing {' '.join(k)} [{want[k]}]")
        elif k not in want:
            lines.append(f"extra {' '.join(k)} [{got[k]}]")
        elif want[k] != got[k]:
            lines.append(f"differs {' '.join(k)} table [{want[k]}] libniji [{got[k]}]")
    bad = [l for l in lines if l not in accepted]
    for l in bad:
        print(l)
    print(f"probe_compare {args.table}: {len(bad)} unaccepted of {len(lines)} differences in {len(want)} rows")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
