#!/usr/bin/env python3
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# The INVIOLABLE workload rule as a verdict (WL-DYNAMIC-GATE; owner ruling #6965, 2026-11-20; D-X.141). It binds the
# DYNAMIC kit, libniji as users run it (validation/wl_lock.sh LINK):
#   * every workload at or below 1.10x glibc in instructions:u and in page faults;
#   * the geomean of instructions:u over all workloads at or below 1.00 (only a run of every workload has one;
#     page faults' geomean is printed, advisory);
#   * a HELD row (validation/wl_holds.tsv: a row over 1.10x for a named cause, today dynamic start-up) is bounded at
#     its recorded level instead (a ratio to glibc; never loosened: the holds check below). A held row measuring at or
#     below 1.10x drops its hold in that run: the 1.10x bound judges it and the run prints HOLD-RELEASE (`--update`
#     removes the row from the file); a held row measuring below its level prints MAY-TIGHTEN. Page faults vary by a
#     few per run, so a held page-fault row drops its hold only when its WORST run is in bound too (the log's "faults
#     min..max": the new column's highest run over glibc's lowest); a release on the median alone would flake back.
# Levels are per CPU class (/proc/cpuinfo "model name"): instruction counts follow the IFUNC variant a CPU selects. A
# held row with no level recorded for this CPU class is reported HELD-UNRECORDED and does not fail (record one).
#
#   wl_verdict.py check LOG [--holds FILE] [--cpu NAME] [--all-count N] [--link dynamic|static] [--update]
#       LOG: a wl_count log (validation/wl_count_run.sh tables). Judges the NEW column. --link static: the same rule,
#       no holds (the static sweep). Exit 0 PASS, 1 FAIL, 2 usage or nothing to judge.
#   wl_verdict.py holds FILE [--base BASEFILE]
#       The holds file's own ratchet: each level EQUAL to the ratio of the counts it records (libniji / glibc, ceil to
#       4 decimals: a level edited without a re-count is refused, D-X.155) and above 1.10, a cause named, no duplicate;
#       against BASEFILE (the merge base's copy), no row added for a CPU class the base already records and no level
#       raised, except a CORRECTION (owner ruling 2026-11-21, D-X.155: a level recorded from an overstated glibc
#       count): the cause names "correction D-X.<n>", the row is recorded again and its libniji count did
#       not rise (so its glibc count fell). Exit 0 ok, 1 not.
import math
import os
import re
import sys

BOUND = 1.10
GEO_BOUND = 1.00
METRICS = {"instructions:u": "instr", "page-faults": "pf"}
FIELDS = ("cpu", "workload", "metric", "glibc", "libniji", "level", "recorded", "cause")


def ceil4(x):
    return math.ceil(x * 10000 - 1e-9) / 10000


def die(msg):
    sys.stderr.write("wl_verdict: %s\n" % msg)
    sys.exit(2)


def read_holds(path):
    rows = []
    if path is None or not os.path.exists(path):
        return rows
    with open(path) as f:
        for n, line in enumerate(f, 1):
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            p = line.split("\t")
            if p[0] == "cpu":
                continue
            if len(p) != len(FIELDS):
                rows.append({"bad": "%s:%d: %d fields, want %d" % (path, n, len(p), len(FIELDS))})
                continue
            r = dict(zip(FIELDS, p))
            r["line"] = n
            rows.append(r)
    return rows


def this_cpu():
    try:
        with open("/proc/cpuinfo") as f:
            for line in f:
                if line.startswith("model name"):
                    return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return "unknown"


def parse_log(path):
    """{workload: {metric: (new, base, glibc)}} from wl_count_run.sh's tables."""
    out, cur = {}, None
    with open(path, errors="replace") as f:
        for line in f:
            if line.startswith("== "):
                cur = line[3:].split()[0]
                out[cur] = {}
                continue
            p = line.split()
            if cur and len(p) >= 5 and p[0] == "faults" and p[1] == "min..max":
                try:  # the new column's highest run and glibc's lowest: a hold drops only when its worst run is in bound
                    out[cur]["pf-worst"] = (float(p[2].split("..")[1]), float(p[4].split("..")[0]))
                except (ValueError, IndexError):
                    pass
                continue
            if cur and p and p[0] in METRICS and len(p) >= 4:
                try:
                    out[cur][METRICS[p[0]]] = (float(p[1]), float(p[2]), float(p[3]))
                except ValueError:
                    out[cur][METRICS[p[0]]] = None
    return out


def check(args):
    log = args[0]
    holds_path, cpu, allc, link, update = None, None, None, "dynamic", False
    i = 1
    while i < len(args):
        a = args[i]
        if a == "--update":
            update = True
            i += 1
            continue
        if i + 1 >= len(args):
            die("%s needs a value" % a)
        v = args[i + 1]
        if a == "--holds":
            holds_path = v
        elif a == "--cpu":
            cpu = v
        elif a == "--all-count":
            allc = int(v)
        elif a == "--link":
            link = v
        else:
            die("unknown option %s" % a)
        i += 2
    if link not in ("dynamic", "static"):
        die("--link %s: want dynamic or static" % link)
    cpu = cpu or this_cpu()
    rows = read_holds(holds_path) if link == "dynamic" else []
    bad = [r["bad"] for r in rows if "bad" in r]
    if bad:
        die("holds file: " + "; ".join(bad))
    held_any = {(r["workload"], r["metric"]) for r in rows}
    held = {(r["workload"], r["metric"]): r for r in rows if r["cpu"] == cpu}
    data = parse_log(log)
    if not data:
        die("%s: no workload table" % log)
    fails, notes, release = [], [], []
    ratios = {"instr": [], "pf": []}
    print("wl_verdict: link %s, CPU class '%s', %d workloads, bound %.2fx glibc per row, geomean (instructions:u) %.2f"
          % (link, cpu, len(data), BOUND, GEO_BOUND))
    for w, m in data.items():
        for k in ("instr", "pf"):
            v = m.get(k)
            if not v or v[2] <= 0:
                fails.append("%s %s: no measurement" % (w, k))
                continue
            new, _, g = v
            r = new / g
            ratios[k].append(r)
            h = held.get((w, k))
            if r <= BOUND:
                wr = m.get("pf-worst") if k == "pf" else None
                if h and wr and wr[1] > 0 and wr[0] / wr[1] > BOUND:
                    notes.append("HELD %s %s: median %.4f <= %.2f but its worst run %.4f is over: the hold stays (it drops"
                                 " when every run is at or below %.2f)" % (w, k, r, BOUND, wr[0] / wr[1], BOUND))
                    continue
                if h:
                    release.append(h)
                    notes.append("HOLD-RELEASE %s %s: %.4f <= %.2f: the hold (level %s) is dropped; the %.2fx bound applies"
                                 % (w, k, r, BOUND, h["level"], BOUND))
                continue
            if h:
                lv = float(h["level"])
                if r > lv:
                    fails.append("%s %s: %.0f is %.6f x glibc, over its held level %s (%s; recorded %s)"
                                 % (w, k, new, r, h["level"], h["cause"], h["recorded"]))
                elif ceil4(r) < lv:
                    notes.append("MAY-TIGHTEN %s %s: %.4f under its held level %s" % (w, k, r, h["level"]))
                else:
                    notes.append("HELD %s %s: %.4f <= level %s (%s)" % (w, k, r, h["level"], h["cause"]))
            elif (w, k) in held_any:
                notes.append("HELD-UNRECORDED %s %s: %.4f x glibc; held on another CPU class, no level recorded for '%s'"
                             " (record one in validation/wl_holds.tsv)" % (w, k, r, cpu))
            else:
                fails.append("%s %s: %.0f is %.4f x glibc, over %.2f" % (w, k, new, r, BOUND))
    geo = {k: math.exp(sum(math.log(x) for x in v) / len(v)) if v else float("nan") for k, v in ratios.items()}
    full = allc is not None and len(data) >= allc
    if full and geo["instr"] > GEO_BOUND:
        fails.append("geomean instructions:u %.4f over %.2f" % (geo["instr"], GEO_BOUND))
    print("wl_verdict: geomean over %d workloads: instructions:u %.4f%s, page faults %.4f (advisory)"
          % (len(data), geo["instr"], "" if full else " (advisory: not every workload ran)", geo["pf"]))
    for n in notes:
        print("wl_verdict: " + n)
    for f in fails:
        print("wl_verdict: FAIL " + f)
    if update and release and holds_path:
        drop = {r["line"] for r in release}
        with open(holds_path) as f:
            lines = f.readlines()
        with open(holds_path, "w") as f:
            f.writelines(x for n, x in enumerate(lines, 1) if n not in drop)
        print("wl_verdict: %s: %d released hold(s) removed" % (holds_path, len(drop)))
    print("wl_verdict: %s (%d failing)" % ("FAIL" if fails else "PASS", len(fails)))
    return 1 if fails else 0


CORRECTION = re.compile(r"\bcorrection (D-X\.[0-9]+)\b")


def correction_refused(r, b):
    """Why a raised level is not an accepted CORRECTION, or "" when it is (owner ruling 2026-11-21, D-X.155: a level
    recorded from an overstated glibc count is corrected from honest counts, never raised because libniji got worse).
    Accepted only when the row names the decision that corrects it ("correction D-X.<n>" in its cause), was recorded
    again (its `recorded` differs from the base's), and its libniji count did not rise (so its glibc count fell)."""
    if not CORRECTION.search(r["cause"]):
        return "no 'correction D-X.<n>' in its cause"
    if r["recorded"] == b["recorded"]:
        return "a correction is recorded from new counts: its recorded revision and date are the base's"
    # (a raise whose libniji count did not rise has a lower glibc count: the overstated column is what moved)
    if float(r["libniji"]) > float(b["libniji"]):
        return "libniji's count rose (%s, base %s): a level never rises because libniji got worse" % (r["libniji"], b["libniji"])
    return ""


def holds(args):
    path = args[0]
    base = args[2] if len(args) >= 3 and args[1] == "--base" else None
    rows = read_holds(path)
    errs = [r["bad"] for r in rows if "bad" in r]
    seen = set()
    for r in (x for x in rows if "bad" not in x):
        key = (r["cpu"], r["workload"], r["metric"])
        where = "%s:%d %s %s" % (path, r["line"], r["workload"], r["metric"])
        if key in seen:
            errs.append("%s: duplicate row" % where)
        seen.add(key)
        if r["metric"] not in ("instr", "pf"):
            errs.append("%s: metric must be instr or pf" % where)
            continue
        if not r["cause"].strip():
            errs.append("%s: no cause named" % where)
        try:
            g, n, lv = float(r["glibc"]), float(r["libniji"]), float(r["level"])
        except ValueError:
            errs.append("%s: glibc, libniji and level must be numbers" % where)
            continue
        if g <= 0 or lv > ceil4(n / g):
            errs.append("%s: level %s is above the measurement it records (%.0f / %.0f = %.6f): a hold is never loosened"
                        % (where, r["level"], n, g, n / g if g > 0 else float("nan")))
        elif lv != ceil4(n / g):
            # D-X.155: a level is the ratio of the counts it records, so a level edited without a re-count (lowered
            # or raised) is caught here; tightening means recording the new counts too.
            errs.append("%s: level %s is not the ratio of the counts it records (%.0f / %.0f = %.4f, rounded up): "
                        "re-count and record the counts with the level" % (where, r["level"], n, g, ceil4(n / g)))
        if lv <= BOUND:
            errs.append("%s: level %s is at or below %.2f: the row needs no hold (drop it)" % (where, r["level"], BOUND))
    if base is not None:
        brows = [x for x in read_holds(base) if "bad" not in x]
        bcpu = {x["cpu"] for x in brows}
        bkey = {(x["cpu"], x["workload"], x["metric"]): x for x in brows}
        for r in (x for x in rows if "bad" not in x):
            key = (r["cpu"], r["workload"], r["metric"])
            where = "%s:%d %s %s" % (path, r["line"], r["workload"], r["metric"])
            b = bkey.get(key)
            if b is None and r["cpu"] in bcpu:
                errs.append("%s: a new hold on a CPU class the base records: holds are never added (the 1.10x bound binds)" % where)
            elif b is not None:
                try:
                    if float(r["level"]) > float(b["level"]):
                        why = correction_refused(r, b)
                        if why:
                            errs.append("%s: level %s raised from the base's %s: a hold is never loosened (%s)"
                                        % (where, r["level"], b["level"], why))
                except ValueError:
                    pass
    for e in errs:
        print("wl_verdict holds: " + e)
    print("wl_verdict holds: %d rows, %s" % (len([x for x in rows if "bad" not in x]), "ok" if not errs else "%d errors" % len(errs)))
    return 1 if errs else 0


def main(argv):
    if len(argv) >= 3 and argv[1] == "check":
        return check(argv[2:])
    if len(argv) >= 3 and argv[1] == "holds":
        return holds(argv[2:])
    sys.stderr.write("usage: wl_verdict.py check LOG [--holds FILE] [--cpu NAME] [--all-count N] [--link L] [--update]\n"
                     "       wl_verdict.py holds FILE [--base BASEFILE]\n")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
