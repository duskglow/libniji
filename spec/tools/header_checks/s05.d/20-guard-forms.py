"""Slice 5 checks: guard-forms (header_checks/s05.d/, loaded after s05.py by spec/tools/header_check.py;
header_checks/README.md). With s05.py's `guards` units, which check that each guard is defined and its value."""

# The configurations the forms were probed in (feature_probe.py forms: _GNU_SOURCE), and one C++ mode for the same text.
FORM_CONFIGS = ("gnu", "cxx17")
SENTINEL = "424242"


def units(ctx):
    """No spec clause (black-box probe of the reference, spec/abi/macro-forms.tsv; HEADERS-STAGE2-GAPS-F): for every
    shipped header, each of its include guard names (header_probe.guard_candidates) that the header shows and that
    macro-forms.tsv lists as not replaced, defined by the program to a sentinel before the header, still has the
    sentinel's value after it, with no redefinition diagnostic (the units compile with -Wsystem-headers -Werror).
    features.py's forms check leaves guards out (the reference skips a header whose guard is already defined), so this
    is their form check. Defects: a guard defined outright (a program's value replaced, and a redefinition warning in
    every program that defines it first), as RELEASE-SNAPSHOT-1126A measured for 191 (header, guard) pairs."""
    kept = {(r["header"], r["name"]) for r in ctx.rows("spec/abi/macro-forms.tsv") if not r["result"].startswith("replaced")}
    rows = {}
    for r in ctx.vis:
        rows.setdefault(r["header"], []).append(r)
    out = []
    for h in sorted({r["header"] for r in ctx.slice_rows} - ctx.must_fail):
        hv = rows.get(h, [])
        failing = set().union(*(ctx.expand(r["configs"]) for r in hv if r["kind"] == "include-error"))
        for cid in FORM_CONFIGS:
            if cid in failing:
                continue
            names = [n for n in ctx.hp.guard_candidates(h) if (h, n) in kept
                     and any(r["name"] == n and r["kind"] == "macro" and cid in ctx.expand(r["configs"]) for r in hv)]
            if not names:
                continue
            lines = [f"#define {n} {SENTINEL}" for n in names] + [f"#include <{h}>"]
            lines += [f"#if {n} != {SENTINEL}\n#error \"{n} replaced\"\n#endif" for n in names]
            out.append((cid, f"guard forms {h}", "\n".join(lines) + "\nint niji_guard_form_tu;\n"))
    return out
