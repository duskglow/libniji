#!/usr/bin/env python3
"""Mechanical filter of the decision-evidence audit (DECISION-EVIDENCE-AUDIT).

Python 3.11+ standard library only. Run from the tree root:

  tools/decision_evidence_audit.py [--rulings PATH] [--out DIR]

Reads
  * records/decisions/**/*.toml           (one item per record),
  * docs/differences-from-glibc.md         (one item per table row; every row is a departure),
  * the owner rulings file (--rulings; default ~/src/liella-ops/state/PH14-OWNER-RULINGS.md, skipped with a
    note when absent), one item per table row, bullet, or paragraph under a heading.

For each item it decides, by regular expression only:
  * SETS     the item sets a default, size, limit, budget, window, reserve or bound;
  * DEPARTS  the item departs from the reference (every differences row; a record or ruling that says so);
  * EVIDENCE the item cites a probe, measurement, job, results file, spec clause or measured count; LEVEL is
             `measured` (a probe, measurement, job or results file), `spec` (a spec clause or standard only: it
             fixes a behaviour, rarely a size) or `none`. A record's `spec` field names its section and is not
             evidence;
  * CLASS    for an item that SETS or DEPARTS: a (a cost every process, thread or call pays), b (a departure),
             c (another value), and the flag PROV when the item is provisional.

It writes DIR/items.tsv (every item that SETS or DEPARTS, sorted by class then id) and prints the counts.
The filter is deliberately coarse: it only decides which items a person must read. An item marked as citing
evidence may cite evidence for something else; the audit's judgement (results/decision-audit/README.md) reads
the flagged items without evidence and the class-a items with it.
"""

import argparse
import os
import re
import sys
import tomllib

DEFAULT_RULINGS = "~/src/liella-ops/state/PH14-OWNER-RULINGS.md"

SETS = re.compile(
    r"\b(defaults?|default value|sized?|limits?|limited to|budgets?|windows?|reserves?|reserved|bounds?|bounded|"
    r"caps?|capped|thresholds?|maximum|minimum|at most|at least|surplus|quota|ceiling|headroom|"
    r"RESERVE_\w+|\d[\d,]*\s*(KiB|MiB|GiB|bytes|pages|entries|slots))\b",
    re.I,
)
DEPARTS = re.compile(
    r"(differences?[- ]row|differences-from-glibc|\bCOMPAT-\d+|\bdepart|\bdeviat|unlike (glibc|the reference)|"
    r"differs? from (glibc|the reference)|(glibc|the reference)('s)? [^.;]{0,60}\b(instead|whereas|but libniji)|"
    r"\bwhere (glibc|the reference)\b|than (glibc|the reference) (does|did))",
    re.I,
)
EVIDENCE = {
    "probe": re.compile(r"\bprob(e|ed|es|ing)\b|black-box", re.I),
    "measure": re.compile(
        r"\bmeasur|\bsurvey|\bcounted\b|\binstr(uctions)?(/call| per call)\b|\bper call\b.{0,40}\d|"
        r"\d[\d,]*\s*(instructions|instr|cycles|faults|ns)\b|\b\d+(\.\d+)?x\b|\bpercall\b|\bwl_count\b|\bbench\b",
        re.I,
    ),
    "job": re.compile(r"\b[a-z0-9]+(-[a-z0-9]+)*-(gate|plants|percall|count|wlc|bench|stress)\d*\b"),
    "results": re.compile(r"\b(results|validation)/[\w./-]+|\b[\w-]+\.(tsv|log)\b"),
    "spec": re.compile(
        r"§\s?\d+|\bs\d+_\d+(_[a-z0-9]+)+\b|\bspec/|\bPOSIX\b|\bISO C\b|\bC11\b|\bC99\b|\bIEEE 754\b|\bpsABI\b|"
        r"\bRFC ?\d+|\bman[- ]page|\bkernel (doc|uapi)|\bUAPI\b",
        re.I,
    ),
}
COST = re.compile(
    r"(start-?up|every process|per[- ]process|per[- ]thread|every thread|per[- ]call|every call|resident|"
    r"\breserve|RESERVE_|layout|\bwindows?\b|\bbudgets?\b|\bbins?\b|static TLS|\bTLS\b|thread stack|stack size|"
    r"guard (page|size)|\barena|surplus|hot path|\bheap\b|\bmmap|page faults?|\bRSS\b|pthread_create)",
    re.I,
)
PROV = re.compile(r"provisional", re.I)


def evidence_kinds(text):
    return [k for k, rx in EVIDENCE.items() if rx.search(text)]


def excerpt(text, n=140):
    """The first `n` characters of `text` on one line. The rulings file lives outside the tree and may name lab
    hosts, so its items get no excerpt (their line number locates them); `cargo xtask static` checks the output."""
    t = re.sub(r"\s+", " ", text).strip().replace("\t", " ")
    t = re.sub(r"/home/[\w.-]+", "~", t)
    t = re.sub(r"\b\d{1,3}(\.\d{1,3}){3}\b", "<address>", t)
    return t[:n]


def classify(text, departs, sets):
    if sets and COST.search(text):
        return "a"
    if departs:
        return "b"
    return "c"


def records():
    out = []
    for root, _, files in os.walk("records/decisions"):
        for f in sorted(files):
            if not f.endswith(".toml"):
                continue
            path = os.path.join(root, f)
            with open(path, "rb") as fh:
                rec = tomllib.load(fh)
            out.append(
                {
                    "source": "record",
                    "id": rec["id"],
                    "where": path,
                    "text": rec.get("body", ""),
                    "departs_always": False,
                }
            )
    return out


def differences():
    out = []
    path = "docs/differences-from-glibc.md"
    section = ""
    with open(path, encoding="utf-8") as fh:
        lines = fh.readlines()
    for n, line in enumerate(lines, 1):
        if line.startswith("#"):
            section = line.strip("# \n")
            continue
        if not line.startswith("|") or line.startswith("|---") or is_header(lines, n):
            continue
        out.append(
            {
                "source": "difference",
                "id": "DIFF:%d" % n,
                "where": "%s:%d (%s)" % (path, n, section),
                "text": line,
                "departs_always": True,
            }
        )
    return out


def is_header(lines, n):
    """Line `n` (1-based) is a table's header row: the next line is its `|---` separator."""
    return n < len(lines) and lines[n].startswith("|---")


def rulings(path):
    out = []
    heading = ""
    para = []
    para_start = 0

    def flush():
        nonlocal para
        if para:
            text = " ".join(para)
            out.append(
                {
                    "source": "ruling",
                    "id": "RUL:%d" % para_start,
                    "where": "PH14-OWNER-RULINGS.md:%d" % para_start,
                    "text": heading + " " + text,
                    "departs_always": False,
                }
            )
        para = []

    with open(path, encoding="utf-8") as fh:
        lines = fh.readlines()
    for n, line in enumerate(lines, 1):
        s = line.rstrip("\n")
        if s.startswith("#"):
            flush()
            heading = s.strip("# ")
            continue
        if not s.strip():
            flush()
            continue
        if s.startswith("|---") or is_header(lines, n):
            continue
        if s.startswith("|") or re.match(r"^\s*([-*]|\d+\.)\s", s):
            flush()
            para_start = n
            para = [s]
            if s.startswith("|"):
                flush()
            continue
        if not para:
            para_start = n
        para.append(s)
    flush()
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--rulings", default=DEFAULT_RULINGS)
    ap.add_argument("--out", default="results/decision-audit")
    args = ap.parse_args()

    items = records() + differences()
    rpath = os.path.expanduser(args.rulings)
    if os.path.exists(rpath):
        items += rulings(rpath)
    else:
        print("note: rulings file %s absent, skipped" % args.rulings, file=sys.stderr)

    rows = []
    counts = {}
    for it in items:
        t = it["text"]
        sets = bool(SETS.search(t))
        departs = it["departs_always"] or bool(DEPARTS.search(t))
        prov = bool(PROV.search(t))
        # The record's `spec` field names the section it belongs to; that is not evidence for a value.
        ev = evidence_kinds(t)
        level = "measured" if set(ev) & MEASURED else ("spec" if ev else "none")
        src = it["source"]
        c = counts.setdefault(src, {k: 0 for k in KEYS})
        c["items"] += 1
        if not (sets or departs):
            continue
        cls = classify(t, departs, sets)
        c["flagged"] += 1
        c[level] += 1
        c[cls] += 1
        if level != "measured":
            c[cls + "_unmeasured"] += 1
        if level == "none":
            c[cls + "_none"] += 1
        if prov:
            c["prov"] += 1
        rows.append(
            (cls, src, it["id"], it["where"], "Y" if sets else "-", "Y" if departs else "-",
             "PROV" if prov else "-", level, ",".join(ev) if ev else "-",
             "(see the rulings file)" if src == "ruling" else excerpt(t))
        )

    order = {"a": 0, "b": 1, "c": 2}
    lorder = {"none": 0, "spec": 1, "measured": 2}
    rows.sort(key=lambda r: (order[r[0]], lorder[r[7]], r[1], r[2]))
    os.makedirs(args.out, exist_ok=True)
    with open(os.path.join(args.out, "items.tsv"), "w", encoding="utf-8") as fh:
        fh.write("class\tsource\tid\twhere\tsets\tdeparts\tprovisional\tlevel\tevidence\texcerpt\n")
        for r in rows:
            fh.write("\t".join(r) + "\n")

    print("\t".join(["source"] + KEYS))
    tot = {k: 0 for k in KEYS}
    for src, c in counts.items():
        print("\t".join([src] + [str(c[k]) for k in KEYS]))
        for k, v in c.items():
            tot[k] += v
    print("\t".join(["total"] + [str(tot[k]) for k in KEYS]))
    return 0


MEASURED = {"probe", "measure", "job", "results"}
KEYS = ["items", "flagged", "measured", "spec", "none", "prov", "a", "b", "c",
        "a_unmeasured", "b_unmeasured", "c_unmeasured", "a_none", "b_none", "c_none"]


if __name__ == "__main__":
    sys.exit(main())
