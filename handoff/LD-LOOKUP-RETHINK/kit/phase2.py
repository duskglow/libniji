#!/usr/bin/env python3
"""LD-LOOKUP-RETHINK: per-lookup phase table from attr.py 'pair' keys ("outer fn | innermost inlined fn").
usage: phase2.py DIR N base:more[:label] ...   (DIR holds attr.py JSON per program)
Phases are assigned by the first matching rule (outer = the real function, inner = innermost inlined one)."""
import json, re, sys

VERS = r"reference_want_origin|with_memo|memo_key|memo_name|memo_fill|version_name|has_hashed|Verdefs|Vern|def_name|need_name|reference_name|Versions"
RULES = [
    ("store: target-address check + write", None, r"Loads>::inside|^put<"),
    ("store: target-address check + write", r"Loads>::inside|ProcImage>", None),
    ("versions (reference + definition side)", VERS, None),
    ("versions (reference + definition side)", None, VERS),
    ("name hash (GNU)", r"reloc::target$", r"hashed_string|wrapping_mul|wrapping_add|gnu_hash"),
    ("reference symbol, name, request", r"reloc::target$", None),
    ("relocation decode + dispatch", r"reloc::pass1|reloc::one", None),
    ("scope walk (per object: loop, objects.get, Result)", r"symbols::lookup$|symbols::resolve$", None),
    ("per-object filter (call, bloom, bucket, chain)", r"Object>::lookup$", None),
    ("candidate check (symbol, kind, compare, versym)", r"Object>::lookup::\{closure#0\}|niji_machine_equal|string_is", None),
]

def phase(key):
    outer, _, inner = key.partition(" | ")
    for name, o, i in RULES:
        if (o is None or re.search(o, outer)) and (i is None or re.search(i, inner)):
            return name
    return "other: " + outer.split("::")[-1][:40]

d, n = sys.argv[1], float(sys.argv[2])
cols, table = [], {}
for spec in sys.argv[3:]:
    p = spec.split(":")
    a, b = (json.load(open("%s/%s.json" % (d, x))) for x in p[:2])
    label = p[2] if len(p) > 2 else p[1]
    cols.append((label, (b["total"] - a["total"]) / n))
    for k in set(a["pair"]) | set(b["pair"]):
        v = (b["pair"].get(k, 0) - a["pair"].get(k, 0)) / n
        ph = phase(k)
        table.setdefault(ph, {}).setdefault(label, 0.0)
        table[ph][label] += v
print("| phase | " + " | ".join(c for c, _ in cols) + " |")
print("|---" * (len(cols) + 1) + "|")
order = [r[0] for r in RULES]
rows = sorted(table, key=lambda ph: (order.index(ph) if ph in order else 99, -abs(sum(table[ph].values()))))
other = {c: 0.0 for c, _ in cols}
for ph in rows:
    vals = table[ph]
    if ph.startswith("other") and max(abs(v) for v in vals.values()) < 3:
        for c in vals:
            other[c] += vals[c]
        continue
    print("| %s | " % ph + " | ".join("%.0f" % vals.get(c, 0.0) for c, _ in cols) + " |")
print("| other (each < 3) | " + " | ".join("%.0f" % other[c] for c, _ in cols) + " |")
print("| **total per lookup** | " + " | ".join("%.0f" % t for _, t in cols) + " |")
