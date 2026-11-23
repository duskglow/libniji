"""SHSTK-SCOPE: tables from a measure.sh log. Usage: python3 -I parse.py <log> [<log> ...]
Several logs: each value is the median over the logs (one log = one perf -r mean)."""
import statistics
import sys

rows = {}
for path in sys.argv[1:]:
    for line in open(path):
        if not line.startswith("ROW "):
            continue
        head, _, tail = line.partition("|")
        _, impl, arm, name, n, reps = head.split()
        vals = {}
        for tok in tail.split():
            v, _, ev = tok.partition(",")
            try:
                vals[ev] = float(v)
            except ValueError:
                vals[ev] = float("nan")
        rows.setdefault((impl, arm, name, int(n)), []).append(vals)

EVS = ["instructions:u", "instructions:k", "page-faults", "task-clock"]


def get(impl, arm, name, n):
    vs = rows.get((impl, arm, name, n))
    if not vs:
        return None
    return {e: statistics.median(v[e] for v in vs) for e in EVS}


def fmt(x, nd=0):
    return "-" if x is None else f"{x:,.{nd}f}"


print("## start-up (per process; dynamic rows minus env's own cost)")
print("| impl | row | arm | user instr | kernel instr | faults | ms (advisory) |")
print("|---|---|---|---:|---:|---:|---:|")
deltas = []
for impl in ("n", "g"):
    for name in ("empty_static", "empty", "main3"):
        per = {}
        for arm in ("off", "on"):
            r = get(impl, arm, name, 0)
            if r is None:
                continue
            if name != "empty_static":
                eb, ed = get(impl, arm, "envbase", 0), get(impl, arm, "envbase_direct", 0)
                r = {e: r[e] - (eb[e] - ed[e]) for e in EVS}
            per[arm] = r
            print(f"| {impl} | {name} | {arm} | {fmt(r[EVS[0]])} | {fmt(r[EVS[1]])} | {fmt(r[EVS[2]])} | {fmt(r[EVS[3]], 3)} |")
        if len(per) == 2:
            d = {e: per["on"][e] - per["off"][e] for e in EVS}
            deltas.append((impl, name, d, per["off"]))
print()
print("| impl | row | d user | d kernel | d faults | d ms | d kernel % |")
print("|---|---|---:|---:|---:|---:|---:|")
for impl, name, d, off in deltas:
    print(f"| {impl} | {name} | {fmt(d[EVS[0]])} | {fmt(d[EVS[1]])} | {fmt(d[EVS[2]], 1)} | {fmt(d[EVS[3]], 3)} | {100 * d[EVS[1]] / off[EVS[1]]:.1f}% |")

print()
print("## per call ((row n=N) - (row n=0)) / N")
print("| impl | row | N | off user | on user | off kernel | on kernel | off faults | on faults | off ns | on ns |")
print("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
names = []
for k in rows:
    if k[3] > 0 and k[2] not in names:
        names.append(k[2])
for impl in ("n", "g"):
    for name in names:
        ns = sorted({k[3] for k in rows if k[0] == impl and k[2] == name and k[3] > 0})
        for N in ns:
            out = {}
            for arm in ("off", "on"):
                a, b = get(impl, arm, name, N), get(impl, arm, name, 0)
                if a and b:
                    out[arm] = {e: (a[e] - b[e]) / N for e in EVS}
            if not out:
                continue
            o, f = out.get("off"), out.get("on")
            g = lambda r, e, nd=1: fmt(r[e], nd) if r else "-"
            print(f"| {impl} | {name} | {N} | {g(o, EVS[0])} | {g(f, EVS[0])} | {g(o, EVS[1])} | {g(f, EVS[1])} | {g(o, EVS[2], 3)} | {g(f, EVS[2], 3)} | "
                  f"{fmt(o[EVS[3]] * 1e6, 0) if o else '-'} | {fmt(f[EVS[3]] * 1e6, 0) if f else '-'} |")

print()
print("## real programs through libniji's loader (whole process)")
print("| row | off user | on user | off kernel | on kernel | d kernel % | off faults | on faults | off ms | on ms |")
print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
for name in sorted({k[2] for k in rows if k[2].startswith("prog_")}):
    o, f = get("n", "off", name, 0), get("n", "on", name, 0)
    if o and f:
        print(f"| {name} | {fmt(o[EVS[0]])} | {fmt(f[EVS[0]])} | {fmt(o[EVS[1]])} | {fmt(f[EVS[1]])} | {100 * (f[EVS[1]] - o[EVS[1]]) / o[EVS[1]]:.1f}% | {fmt(o[EVS[2]])} | {fmt(f[EVS[2]])} | {fmt(o[EVS[3]], 2)} | {fmt(f[EVS[3]], 2)} |")
