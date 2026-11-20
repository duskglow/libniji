#!/usr/bin/env python3
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# The table of validation/wl_kits.sh (WL-DYNAMIC-KIT): static vs dynamic libniji vs glibc per workload.
#
#   wl_kits.py RAW.tsv
#
# RAW rows (tab-separated): run W COL ROUND instr_u instr_k faults task_clock_ms wall_us | md5 W COL MD5 |
# exec W PROG N | start PROG COL ROUND instr_u instr_k faults. Every number shown is the median over the rounds.
# Ratios are to glibc. "start-up share" = sum over the workload's program starts of (dynamic - static) instructions of
# one `<prog> --version`, divided by the workload's dynamic - static instructions:u: how much of the dynamic column's
# extra cost is paid once per start (loader, relocation, IFUNC resolution) rather than per call (PLT).
# "dyn@glibc-start" = the dynamic column with each start priced at glibc's per-start instructions instead: the level
# the dynamic kit would reach if libniji's dynamic start-up matched glibc's (what DYN-START-SPEED works on).
import math
import statistics
import sys
from collections import defaultdict

COLS = ("glibc", "static", "dynamic")


def num(x):
    try:
        return float(x)
    except ValueError:
        return float("nan")


def med(xs):
    xs = [x for x in xs if not math.isnan(x)]
    return statistics.median(xs) if xs else float("nan")


def geo(xs):
    xs = [x for x in xs if x > 0 and not math.isnan(x)]
    return math.exp(sum(math.log(x) for x in xs) / len(xs)) if xs else float("nan")


def main(path):
    runs = defaultdict(lambda: defaultdict(list))   # (w, col) -> field -> values
    md5 = {}
    execs = defaultdict(dict)
    start = defaultdict(lambda: defaultdict(list))  # (prog, col) -> field -> values
    order = []
    for line in open(path):
        f = line.rstrip("\n").split("\t")
        if f[0] == "run":
            w, col = f[1], f[2]
            if w not in order:
                order.append(w)
            for k, v in zip(("iu", "ik", "pf", "tc", "wall"), f[4:9]):
                runs[(w, col)][k].append(num(v))
        elif f[0] == "md5":
            md5[(f[1], f[2])] = f[3]
        elif f[0] == "exec":
            execs[f[1]][f[2]] = int(f[3])
        elif f[0] == "start":
            for k, v in zip(("iu", "ik", "pf"), f[4:7]):
                start[(f[1], f[2])][k].append(num(v))
    m = {key: {k: med(v) for k, v in d.items()} for key, d in runs.items()}
    s = {key: {k: med(v) for k, v in d.items()} for key, d in start.items()}

    print("Per start (`<prog> --version`, median of 3): instructions:u / page faults")
    print("%-8s %12s %12s %12s %8s %8s %8s" % ("prog", "glibc", "static", "dynamic", "pf g", "pf s", "pf d"))
    for p in sorted({k[0] for k in s}):
        r = [s.get((p, c), {}) for c in COLS]
        print("%-8s %12.0f %12.0f %12.0f %8.0f %8.0f %8.0f" % ((p,) + tuple(x.get("iu", float("nan")) for x in r) + tuple(x.get("pf", float("nan")) for x in r)))
    print()
    hdr = ("workload", "iu glibc", "iu static", "iu dyn", "s/g", "d/g", "d/s", "pf g", "pf s", "pf d", "pf s/g", "pf d/g",
           "tc s/g", "tc d/g", "starts", "start share", "dyn@g-start/g", "out")
    print("%-12s %13s %13s %13s %6s %6s %6s %7s %7s %7s %6s %6s %6s %6s %6s %6s %8s %s" % hdr)
    ratios = defaultdict(list)
    over = defaultdict(lambda: defaultdict(list))
    for w in order:
        g, st, dy = (m.get((w, c), {}) for c in COLS)
        r = {}
        for k in ("iu", "pf", "tc"):
            r["s" + k] = st.get(k, float("nan")) / g.get(k, float("nan"))
            r["d" + k] = dy.get(k, float("nan")) / g.get(k, float("nan"))
            ratios["s" + k].append(r["s" + k])
            ratios["d" + k].append(r["d" + k])
        nstarts = sum(execs[w].values())
        extra = sum(n * (s[(p, "dynamic")]["iu"] - s[(p, "static")]["iu"]) for p, n in execs[w].items() if (p, "dynamic") in s and (p, "static") in s)
        diff = dy["iu"] - st["iu"]
        share = extra / diff if diff > 0 else float("nan")
        excess = sum(n * (s[(p, "dynamic")]["iu"] - s[(p, "glibc")]["iu"]) for p, n in execs[w].items() if (p, "dynamic") in s and (p, "glibc") in s)
        atg = (dy["iu"] - excess) / g["iu"]
        ratios["datg"].append(atg)
        out = "SAME" if md5.get((w, "static")) == md5.get((w, "glibc")) == md5.get((w, "dynamic")) else \
            "DIFF(s%s,d%s)" % ("=" if md5.get((w, "static")) == md5.get((w, "glibc")) else "!", "=" if md5.get((w, "dynamic")) == md5.get((w, "glibc")) else "!")
        for col, key in (("static", "s"), ("dynamic", "d")):
            for k in ("iu", "pf", "tc"):
                v = r[key + k]
                if v > 1.10:
                    over[(col, k)]["1.10"].append("%s %.3f" % (w, v))
                if v > 1.00:
                    over[(col, k)]["1.00"].append("%s %.3f" % (w, v))
        print("%-12s %13.0f %13.0f %13.0f %6.3f %6.3f %6.3f %7.0f %7.0f %7.0f %6.3f %6.3f %6.3f %6.3f %6d %6s %8.3f %s" % (
            w, g["iu"], st["iu"], dy["iu"], r["siu"], r["diu"], dy["iu"] / st["iu"], g["pf"], st["pf"], dy["pf"], r["spf"], r["dpf"],
            r["stc"], r["dtc"], nstarts, ("%.0f%%" % (100 * share)) if not math.isnan(share) else "-", atg, out))
    print()
    print("geomean over %d workloads (ratio to glibc): instructions:u static %.4f dynamic %.4f (dynamic at glibc's per-start cost %.4f); "
          "page faults static %.4f dynamic %.4f; task-clock (advisory) static %.4f dynamic %.4f" % (
              len(order), geo(ratios["siu"]), geo(ratios["diu"]), geo(ratios["datg"]), geo(ratios["spf"]), geo(ratios["dpf"]),
              geo(ratios["stc"]), geo(ratios["dtc"])))
    names = {"iu": "instructions:u", "pf": "page faults", "tc": "task-clock (advisory)"}
    for col in ("static", "dynamic"):
        for k in ("iu", "pf", "tc"):
            for lim in ("1.10", "1.00"):
                lst = over[(col, k)][lim]
                print("%-7s %-22s > %s: %2d  %s" % (col, names[k], lim, len(lst), ", ".join(lst)))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.stderr.write("usage: wl_kits.py RAW.tsv\n")
        sys.exit(2)
    main(sys.argv[1])
