#!/usr/bin/env python3
"""Chain and theme cost against measured gain (CHAIN-COST-AUDIT, goal E4).

Reads the coordinator's ops database READ-ONLY (sqlite URI mode=ro) and the
hand-back reports under handoff/reports/*.json, and writes:

  <out>/chains.tsv     every chain with runs in the window
  <out>/themes.tsv     the same, grouped by theme (first id token)
  <out>/ranked.md      ranked tables (worst chains, themes, red causes)
  <out>/evidence.md    per-run results and red reasons for the worst chains

Usage:
  python3 tools/chain_cost_audit.py --db ~/state/ops.db --since 2026-10-24 \
      --now 2026-11-23T17:30:00Z --out results/chain-cost-audit [--worst 10]

Definitions (also in results/chain-cost-audit/README.md):
  chain     stories joined by continuation edges: an id `X-N` (N in 2..30) and
            its stem `X` (or a sibling `X-k`); events "continued as Y",
            "now waits for Y"; and a Stacks-on parent other than `next` that
            has at most --hub-max followers (planning batches with many
            followers are hubs and are not chain edges).
  cost      runs.cost_usd; for runs without it (before 2026-11-10), tokens x
            the per-model $/token ratio of the runs that have both (marked
            estimated).
  agent-h   sum over runs of ended - started (a live run counts to --now).
  gain      a hand-back impact entry with measured=true whose kind is perf,
            faults, size, accuracy, correctness or tooling (for numeric
            perf/faults/size rows, only when after < before). coverage,
            unblock, none and cost entries are not gains. Impact comes from
            runs.impact (latest per story), else handoff/reports/<ID>.json.
  realized  gains of stories that landed.
  red       a transition into `red` from another state, classified:
            own (the story's own verify/hand-back/precheck failures, a CI red
            attributed to this story alone), incomplete (an INCOMPLETE
            hand-back sent back for a -N continuation), env (wall-time kill,
            API/spawn failure, run without a final result, refused command),
            ci-shared (CI red attributed to several stories or none),
            ci-conflict (merge conflict against next), coordinator (manual).
"""
import argparse
import collections
import datetime as dt
import glob
import json
import math
import os
import re
import sqlite3

GAIN_KINDS = {"perf", "faults", "size", "accuracy", "correctness", "tooling"}
LOWER_IS_BETTER = {"perf", "faults", "size"}
CONT_RE = re.compile(r"^(.*)-(\d+)$")


def ts(s):
    if not s:
        return None
    return dt.datetime.strptime(s[:19], "%Y-%m-%dT%H:%M:%S")


class UF:
    def __init__(self):
        self.p = {}

    def find(self, a):
        self.p.setdefault(a, a)
        while self.p[a] != a:
            self.p[a] = self.p[self.p[a]]
            a = self.p[a]
        return a

    def union(self, a, b):
        ra, rb = self.find(a), self.find(b)
        if ra != rb:
            # keep the shorter (stem) id as the root name
            if (len(rb), rb) < (len(ra), ra):
                ra, rb = rb, ra
            self.p[rb] = ra


def num(x):
    if isinstance(x, bool) or x is None:
        return None
    if isinstance(x, (int, float)):
        return float(x)
    if isinstance(x, str):
        m = re.match(r"^\s*~?([-+]?\d[\d,]*\.?\d*)", x)
        if m:
            try:
                return float(m.group(1).replace(",", ""))
            except ValueError:
                return None
    return None


def gains_of(impact):
    """Return (gain entries, cost entries) from one impact list."""
    g, c = [], []
    if not isinstance(impact, list):
        return g, c
    for e in impact:
        if not isinstance(e, dict) or e.get("measured") is not True:
            continue
        k = e.get("kind")
        if k == "cost":
            c.append(e)
            continue
        if k not in GAIN_KINDS:
            continue
        b, a = num(e.get("before")), num(e.get("after"))
        if k in LOWER_IS_BETTER and b is not None and a is not None:
            if a >= b:
                c.append(e)
                continue
        g.append(e)
    return g, c


def classify_red(actor, from_state, reason):
    r = reason or ""
    if actor == "ci":
        if r.startswith("CI CONFLICT"):
            return "ci-conflict"
        if r.startswith("CI PRECHECK RED"):
            return "own"
        if "attribution: one" in r:
            return "own"
        return "ci-shared"
    if actor == "coordinator" or r.startswith("manual fail"):
        return "coordinator"
    if r.startswith("verify: incomplete hand-back"):
        return "incomplete"
    env_pat = ("wall-time limit", "run failed", "spawn failed", "refused:",
               "API Error")
    if any(p in r for p in env_pat):
        return "env"
    return "own"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", default=os.path.expanduser("~/state/ops.db"))
    ap.add_argument("--since", required=True)
    ap.add_argument("--now", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--reports", default="handoff/reports")
    ap.add_argument("--worst", type=int, default=10)
    ap.add_argument("--hub-max", type=int, default=3)
    args = ap.parse_args()

    db = sqlite3.connect("file:" + args.db + "?mode=ro", uri=True)
    q = lambda s, *a: db.execute(s, a).fetchall()
    now = ts(args.now)
    hosts = sorted((r[0] for r in q("select name from hosts")), key=len,
                   reverse=True)
    host_re = re.compile(r"\b(" + "|".join(map(re.escape, hosts)) + r")\b",
                         re.IGNORECASE) \
        if hosts else None
    home_re = re.compile(r"/(?:home)/[^/\s]+/")

    def clean(s):
        s = (s or "").replace("\n", " ").replace("|", "/")
        if host_re:
            s = host_re.sub("<host>", s)
        # quoted hand-back text may name a pending record id; the
        # placeholder check would read it as a record of this tree
        s = re.sub(r"\b(D|COMPAT|SR|SAT)(-[A-Z0-9.]+)?-NEW\b",
                   "<pending id>", s)
        return home_re.sub("~/", s)

    stories = {r[0]: r for r in q(
        "select id, state, stacked_on, created, title from stories")}
    runs = q("select story, run_no, model, started, ended, result, tokens, "
             "cost_usd, impact, state from runs where started >= ? and "
             "started < ? order by started", args.since, args.now)
    active = {r[0] for r in runs}

    # cost estimate for runs without cost_usd
    ratio = {m: c / t for m, c, t in q(
        "select model, sum(cost_usd), sum(tokens) from runs where cost_usd "
        "is not null and tokens > 0 group by model")}

    uf = UF()
    for sid in stories:
        uf.find(sid)
    for sid in stories:
        m = CONT_RE.match(sid)
        if m and 2 <= int(m.group(2)) <= 30:
            stem = m.group(1)
            if stem in stories:
                uf.union(stem, sid)
            else:
                for k in range(2, 31):
                    sib = f"{stem}-{k}"
                    if sib != sid and sib in stories:
                        uf.union(sib, sid)
    followers = collections.Counter(
        r[2] for r in stories.values() if r[2] and r[2] != "next")
    hubs = {p for p, n in followers.items() if n > args.hub_max}
    for r in stories.values():
        p = r[2]
        if p and p != "next" and p not in hubs and p in stories:
            uf.union(p, r[0])
    for s, reason in q("select story, reason from events where reason like "
                       "'%continued as %' or reason like '%now waits for %'"):
        m = re.search(r"(?:continued as|now waits for) ([A-Z0-9][A-Z0-9-]+)",
                      reason or "")
        if s in stories and m and m.group(1) in stories:
            uf.union(s, m.group(1))

    # impact per story: latest runs.impact, else the report json
    impact = {}
    for s, imp in q("select story, impact from runs where impact is not null "
                    "order by started"):
        try:
            impact[s] = json.loads(imp)
        except ValueError:
            pass
    for f in glob.glob(os.path.join(args.reports, "*.json")):
        sid = os.path.basename(f)[:-5]
        if sid in impact:
            continue
        try:
            with open(f) as fh:
                d = json.load(fh)
        except (ValueError, OSError):
            continue
        if isinstance(d, dict) and d.get("impact") is not None:
            impact[sid] = d["impact"]

    # session legs (a refresh starts a new leg inside one run)
    legs = collections.Counter(dict(q(
        "select r.story, count(*) from run_legs l join runs r on l.run = "
        "r.id where r.started >= ? and r.started < ? group by r.story",
        args.since, args.now)))

    reds = collections.defaultdict(list)
    for s, actor, fr, reason, t in q(
            "select story, actor, from_state, reason, time from events "
            "where to_state = 'red' and (from_state is null or from_state "
            "!= 'red') and time >= ? and time < ? order by time",
            args.since, args.now):
        reds[s].append((t, classify_red(actor, fr, reason), clean(reason)))

    chains = collections.defaultdict(lambda: {
        "stories": set(), "runs": 0, "cost": 0.0, "est": 0.0, "hours": 0.0,
        "reds": collections.Counter(), "landed": set(), "gain": 0,
        "gain_real": 0, "cost_rows": 0, "first": None, "last": None,
        "runlog": []})
    for (s, rn, model, st, en, res, tok, cost, imp, rstate) in runs:
        c = chains[uf.find(s)]
        if s not in c["stories"]:
            c["legs"] = c.get("legs", 0) + legs[s]
        c["stories"].add(s)
        c["runs"] += 1
        if cost is None and tok:
            cost = tok * ratio.get(model, max(ratio.values()))
            c["est"] += cost
        c["cost"] += cost or 0.0
        sc = c.setdefault("per_story", {}).setdefault(s, [st, 0.0])
        sc[1] += cost or 0.0
        t0, t1 = ts(st), ts(en) or now
        if t0 and t1:
            c["hours"] += max(0.0, (t1 - t0).total_seconds() / 3600)
        c["first"] = min(filter(None, [c["first"], st]))
        c["last"] = max(filter(None, [c["last"], st]))
        c["runlog"].append((st, s, rn, model, round(cost or 0, 2),
                            clean(res)[:220]))
    # stories of each chain not run yet in the window (queued continuations)
    for sid, r in stories.items():
        root = uf.find(sid)
        if root in chains and sid not in chains[root]["stories"] and \
                (r[3] or "") >= args.since:
            chains[root].setdefault("pending", set()).add(sid)

    for root, c in chains.items():
        seen, seen_real = set(), set()
        c["impact_known"] = 0
        for s in c["stories"]:
            for t, kind, reason in reds.get(s, []):
                c["reds"][kind] += 1
                c["runlog"].append((t, s, "red", kind, "", reason[:220]))
            if stories[s][1] == "landed":
                c["landed"].add(s)
            if s in impact:
                c["impact_known"] += 1
            g, co = gains_of(impact.get(s))
            c["cost_rows"] += len(co)
            # the same effect re-measured by a continuation counts once
            for e in g:
                key = (e.get("kind"), str(e.get("what"))[:60])
                seen.add(key)
                if stories[s][1] == "landed":
                    seen_real.add(key)
        c["gain"], c["gain_real"] = len(seen), len(seen_real)
        c["cont"] = len(c["stories"]) - 1
        stems = collections.Counter()
        for s in c["stories"] | c.get("pending", set()):
            m = CONT_RE.match(s)
            stems[m.group(1) if m and 2 <= int(m.group(2)) <= 30 else s] += 1
        c["depth"] = max(stems.values())
        c["stem"] = sorted(stems.items(),
                           key=lambda kv: (-kv[1], len(kv[0]), kv[0]))[0][0]
        c["cpg"] = c["cost"] / c["gain_real"] if c["gain_real"] else math.inf

    # name each chain by its dominant stem (deterministic, sanitized)
    named = {}
    for root, c in sorted(chains.items()):
        name = clean(c["stem"])
        while name in named:
            name += "+"
        named[name] = c
    chains = named

    os.makedirs(args.out, exist_ok=True)
    order = sorted(chains.items(), key=lambda kv: (
        -(kv[1]["cpg"] if kv[1]["cpg"] != math.inf else 1e12),
        -kv[1]["cost"]))
    hdr = ["chain", "stories", "continuations", "depth", "pending", "runs",
           "legs",
           "cost_usd", "cost_estimated_usd", "agent_hours", "reds",
           "red_own", "red_incomplete", "red_env", "red_ci_shared",
           "red_ci_conflict", "red_coordinator", "landed_stories",
           "impact_recorded", "gain_items", "gain_realized", "cost_rows",
           "usd_per_realized_gain", "first_run", "last_run"]

    def row(root, c):
        r = c["reds"]
        cpg = "inf" if c["cpg"] == math.inf else f"{c['cpg']:.0f}"
        if c["cpg"] == math.inf and not c["impact_known"]:
            cpg = "n/r"
        return [root, len(c["stories"]), c["cont"], c["depth"],
                len(c.get("pending", ())), c["runs"], c.get("legs", 0),
                f"{c['cost']:.0f}", f"{c['est']:.0f}", f"{c['hours']:.1f}",
                sum(r.values()), r["own"], r["incomplete"], r["env"],
                r["ci-shared"], r["ci-conflict"], r["coordinator"],
                len(c["landed"]), c["impact_known"], c["gain"],
                c["gain_real"], c["cost_rows"], cpg,
                (c["first"] or "")[:10], (c["last"] or "")[:10]]

    with open(os.path.join(args.out, "chains.tsv"), "w") as f:
        f.write("\t".join(hdr) + "\n")
        for root, c in sorted(chains.items(), key=lambda kv: -kv[1]["cost"]):
            f.write("\t".join(map(str, row(root, c))) + "\n")

    themes = collections.defaultdict(lambda: collections.Counter())
    for root, c in chains.items():
        th = root.split("-")[0]
        t = themes[th]
        t["chains"] += 1
        t["runs"] += c["runs"]
        t["cost"] += c["cost"]
        t["hours"] += c["hours"]
        t["reds"] += sum(c["reds"].values())
        t["landed"] += 1 if c["landed"] else 0
        t["gain_real"] += c["gain_real"]
        t["cont4"] += 1 if c["cont"] >= 3 else 0
    with open(os.path.join(args.out, "themes.tsv"), "w") as f:
        f.write("theme\tchains\tchains_landed\tchains_4plus_stories\truns\t"
                "cost_usd\tagent_hours\treds\tgain_realized\t"
                "usd_per_realized_gain\n")
        for th, t in sorted(themes.items(), key=lambda kv: -kv[1]["cost"]):
            cpg = t["cost"] / t["gain_real"] if t["gain_real"] else math.inf
            f.write(f"{th}\t{t['chains']}\t{t['landed']}\t{t['cont4']}\t"
                    f"{t['runs']}\t{t['cost']:.0f}\t{t['hours']:.1f}\t"
                    f"{t['reds']}\t{t['gain_real']}\t"
                    f"{'inf' if cpg == math.inf else f'{cpg:.0f}'}\n")

    # summary statistics for the tripwire
    multi = [c for c in chains.values() if c["runs"] >= 1]
    tot_cost = sum(c["cost"] for c in multi)
    tot_real = sum(c["gain_real"] for c in multi)
    with open(os.path.join(args.out, "ranked.md"), "w") as f:
        f.write(f"Window: runs started on or after {args.since}, as of "
                f"{args.now}. Chains: {len(multi)}; cost ${tot_cost:.0f}; "
                f"realized gain items {tot_real}; overall "
                f"${tot_cost / max(tot_real, 1):.0f} per realized gain item."
                f" Cost estimated from tokens: "
                f"${sum(c['est'] for c in multi):.0f}. Hubs not used as "
                f"chain edges: {len(hubs)}.\n\n")
        f.write("By size (stories in chain):\n\n| stories | chains | cost $ |"
                " $/chain | landed % | realized gain/chain | $/realized "
                "gain |\n|---|---|---|---|---|---|---|\n")
        bands = [(1, 1), (2, 2), (3, 3), (4, 5), (6, 99)]
        for lo, hi in bands:
            b = [c for c in multi if lo <= len(c["stories"]) <= hi]
            if not b:
                continue
            cs = sum(c["cost"] for c in b)
            gr = sum(c["gain_real"] for c in b)
            lp = 100 * sum(1 for c in b if c["landed"]) / len(b)
            f.write(f"| {lo}-{hi} | {len(b)} | {cs:.0f} | {cs / len(b):.1f} |"
                    f" {lp:.0f} | {gr / len(b):.2f} | "
                    f"{cs / gr if gr else math.inf:.0f} |\n")
        f.write("\nRed causes (all chains): " + ", ".join(
            f"{k} {v}" for k, v in sum(
                (c["reds"] for c in multi), collections.Counter()
            ).most_common()) + "\n\n")
        # evidence counts used by the README's analysis
        merge_re = re.compile(r"\bmerg\w* (origin/)?next", re.IGNORECASE)
        for lo, hi, label in ((1, 2, "chains of 1-2 stories"),
                              (3, 5, "chains of 3-5 stories"),
                              (6, 999, "chains of 6+ stories")):
            rl = [e for c in multi if lo <= len(c["stories"]) <= hi
                  for e in c["runlog"] if e[2] != "red"]
            mg = [e for e in rl if merge_re.search(e[5] or "")]
            f.write(f"Runs in {label}: {len(rl)}; hand-back result "
                    f"mentions merging next: {len(mg)} "
                    f"({100 * len(mg) / max(1, len(rl)):.0f}%), "
                    f"${sum(e[4] for e in mg):.0f}.\n\n")
        tests = collections.Counter()
        for s, lst in reds.items():
            for t, kind, reason in lst:
                if t < "2026-11-16":
                    continue
                for m in re.findall(r"FAILED: ([\w:]+ \w+)", reason):
                    tests[m] += 1
        f.write("Most frequent failing tests named in reds since "
                "2026-11-16: " + "; ".join(
                    f"{k} {v}" for k, v in tests.most_common(6)) + "\n\n")
        weeks = collections.defaultdict(lambda: [0, 0.0])
        for c in multi:
            for e in c["runlog"]:
                if e[2] != "red" and e[0] >= "2026-11-10":
                    w = weeks[e[0][:10] < "2026-11-17" and "11-10..11-16"
                              or "11-17..11-23"]
                    w[0] += 1
                    w[1] += e[4]
        f.write("Cost per run (recorded cost only): " + "; ".join(
            f"{k}: {v[0]} runs, ${v[1] / max(1, v[0]):.2f}/run"
            for k, v in sorted(weeks.items())) + "\n\n")
        f.write("| rank | " + " | ".join(hdr) + " |\n|" + "---|" *
                (len(hdr) + 1) + "\n")
        ranked = [kv for kv in order if kv[1]["cost"] >= 25]
        for i, (root, c) in enumerate(ranked[:40], 1):
            f.write(f"| {i} | " + " | ".join(map(str, row(root, c))) + " |\n")
    # tripwire derivation: walk each chain's stories in order of first run
    def walk(c):
        out = []
        for s, (st, cost) in sorted(c["per_story"].items(),
                                    key=lambda kv: kv[1][0]):
            g, _ = gains_of(impact.get(s))
            out.append((s, cost, len(g), s in c["landed"], s in impact))
        return out

    with open(os.path.join(args.out, "tripwire.md"), "w") as f:
        rec = [c for c in multi if (c["first"] or "") >= "2026-11-13"]
        for title, pop in (("all chains in the window", multi),
                           ("chains first run on or after 2026-11-13 "
                            "(impact recorded on every hand-back)", rec)):
            f.write(f"Depth trigger, {title}: chains that reached N "
                    "stories, and what happened from story N on.\n\n"
                    "| N | chains reaching N | landed from N on | landed a "
                    "gain from N on | cost from story N on $ | of it in "
                    "chains never landed $ |\n|---|---|---|---|---|---|\n")
            for n in range(2, 9):
                reach = [walk(c) for c in pop if len(c["stories"]) >= n]
                if not reach:
                    continue
                land = sum(1 for w in reach if any(x[3] for x in w[n - 1:]))
                landg = sum(1 for w in reach
                            if any(x[3] and x[2] for x in w[n - 1:]))
                after = sum(sum(x[1] for x in w[n - 1:]) for w in reach)
                lost = sum(sum(x[1] for x in w[n - 1:]) for w in reach
                           if not any(x[3] for x in w))
                f.write(f"| {n} | {len(reach)} | {land} | {landg} | "
                        f"{after:.0f} | {lost:.0f} |\n")
            f.write("\n")
        f.write(f"\nCost trigger (chains whose first run is on or after "
                f"2026-11-13, when every hand-back records impact: "
                f"{len(rec)} chains): cumulative cost crosses $X before "
                f"any story of the chain has LANDED with a measured gain "
                f"(a recorded but unlanded gain does not reset it).\n\n"
                "| X $ | chains crossing | later landed anything | later "
                "landed a gain | cost after crossing $ | of it in chains "
                "never landed $ |\n|---|---|---|---|---|---|\n")
        for x in (8, 10, 15, 20, 25, 30, 40):
            cross, gain_l, land_l, after, lost = 0, 0, 0, 0.0, 0.0
            for c in rec:
                cum, hit = 0.0, None
                w = walk(c)
                for i, (s, cost, g, ld, known) in enumerate(w):
                    if cum + cost > x:
                        hit = i
                        break
                    cum += cost
                    if g and ld:
                        break
                if hit is None:
                    continue
                cross += 1
                rest = w[hit:]
                gain_l += any(e[3] for e in rest)
                land_l += any(e[2] and e[3] for e in rest)
                spent = sum(e[1] for e in rest) - (x - cum)
                after += spent
                if not any(e[3] for e in w):
                    lost += spent
            f.write(f"| {x} | {cross} | {gain_l} | {land_l} | {after:.0f} |"
                    f" {lost:.0f} |\n")
        # combined trigger as proposed in the README: fire at the first
        # story that is the chain's Nth, or that takes the cumulative cost
        # over $X, while no story of the chain has landed with a gain.
        f.write("\nCombined trigger on the chains first run on or after "
                "2026-11-13 (fires once per chain; hours are from the "
                "chain's first run to the start of the firing story).\n\n"
                "| N stories | X $ | chains fired | fired per day | landed "
                "a gain after | median hours to fire | cost after firing $ "
                "|\n|---|---|---|---|---|---|---|\n")
        span = max(1.0, (now - ts("2026-11-13T00:00:00")).total_seconds()
                   / 86400)
        for n, x in ((4, 15), (4, 20), (5, 20), (5, 25), (6, 25)):
            fired, landg, after, hrs = 0, 0, 0.0, []
            for c in rec:
                w = walk(c)
                starts = sorted(v[0] for v in c["per_story"].values())
                cum = 0.0
                for i, (s, cost, g, ld, known) in enumerate(w):
                    if i + 1 >= n or cum + cost > x:
                        fired += 1
                        rest = w[i:]
                        landg += any(e[2] and e[3] for e in rest)
                        after += sum(e[1] for e in rest) - max(0.0, x - cum) \
                            if cum + cost > x and i + 1 < n else \
                            sum(e[1] for e in rest)
                        hrs.append((ts(c["per_story"][s][0]) -
                                    ts(starts[0])).total_seconds() / 3600)
                        break
                    cum += cost
                    if g and ld:
                        break
            hrs.sort()
            med = hrs[len(hrs) // 2] if hrs else 0
            f.write(f"| {n} | {x} | {fired} | {fired / span:.1f} | {landg} |"
                    f" {med:.1f} | {after:.0f} |\n")

        legs_hi = [c for c in multi if c.get("legs", 0) >= 6]
        f.write(f"\nChains with 6 or more session legs: {len(legs_hi)}, "
                f"cost ${sum(c['cost'] for c in legs_hi):.0f}, landed "
                f"{sum(1 for c in legs_hi if c['landed'])}.\n")

    with open(os.path.join(args.out, "evidence.md"), "w") as f:
        for root, c in ranked[:args.worst]:
            f.write(f"## {root}\n\nstories: "
                    f"{clean(', '.join(sorted(c['stories'])))}"
                    f"\n\n")
            for e in sorted(c["runlog"], key=lambda e: e[0] or ""):
                f.write("- " + " | ".join(clean(str(x)) for x in e) + "\n")
            f.write("\n")
    print(f"chains {len(multi)} cost {tot_cost:.0f} realized {tot_real}")


if __name__ == "__main__":
    main()
