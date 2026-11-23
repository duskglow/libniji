#!/usr/bin/env python3
"""PERF-GAP-STRUCTURE-AUDIT probe 1 (counts only, no symbols): the reference and libniji, each dynamic and
static, start-up programs and per-call loops. Counters: instructions (user, kernel), cycles (user, advisory),
locked loads (atomics), branch misses, page faults. Writes out1/*.tsv and prints a summary to the job log."""
import os
import statistics
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SHA = "0495c4684330b90a6cecc02271ba1429296536ce"
P = os.path.expanduser("~/wl/products/" + SHA)
ROLE = os.path.expanduser("~/w/s2")
OUT = os.path.join(HERE, "out1")
VARIANTS = ["ref-dyn", "ref-static", "niji-dyn", "niji-static"]
EV = ["instructions:u", "cycles:u", "mem_inst_retired.lock_loads:u", "branch-misses:u", "instructions:k",
      "page-faults"]
SHORT = ["iu", "cu", "lock", "brm", "ik", "pf"]


def sh(cmd, env=None, **kw):
    return subprocess.run(cmd, env=env, capture_output=True, text=True, **kw)


def cc(variant, src, out, defs=()):
    flags = ["-O2"] + list(defs)
    env = None
    if variant == "ref-dyn":
        cmd = ["gcc"] + flags + ["-pthread", src, "-o", out]
    elif variant == "ref-static":
        cmd = ["gcc"] + flags + ["-static", "-pthread", src, "-o", out]
    elif variant == "niji-dyn":
        cmd = ["sh", ROLE + "/validation/wl_dyncc.sh"] + flags + [src, "-o", out]
        env = dict(os.environ, WL_DYN_PRODUCTS=P + "-release-dyn")
    else:
        cmd = ["sh", ROLE + "/scripts/niji-cc"] + flags + [src, "-o", out]
        env = dict(os.environ, NIJI_PRODUCTS=P + "-release")
    r = sh(cmd, env=env)
    if r.returncode != 0:
        print("BUILD-FAIL", variant, out, r.stderr[-800:])
        sys.exit(1)


def stat(argv, env=None):
    """One perf stat run; returns dict short-name -> value (None when not counted) and the min running pct."""
    r = sh(["perf", "stat", "-x,", "-e", ",".join(EV), "--"] + argv, env=env, timeout=600)
    vals, pct = {}, 100.0
    for line in r.stderr.splitlines():
        f = line.split(",")
        if len(f) < 3 or f[2] not in EV:
            continue
        k = SHORT[EV.index(f[2])]
        try:
            vals[k] = float(f[0])
        except ValueError:
            vals[k] = None
        try:
            pct = min(pct, float(f[4]))
        except (ValueError, IndexError):
            pass
    return vals, pct, r.returncode, r.stderr if r.returncode else ""


def main():
    os.makedirs(OUT, exist_ok=True)
    os.chdir(OUT)
    have = sh(["locale", "-a"]).stdout.split()
    loc = "en_US.UTF-8" if any(x.lower() in ("en_us.utf8", "en_us.utf-8") for x in have) else "C.UTF-8"
    print("locales available:", " ".join(have[:12]), "| per-call locale:", loc)
    progs = {"empty": "-DP_EMPTY", "fopen": "-DP_FOPEN", "hello": "-DP_HELLO", "setloc": "-DP_SETLOC"}
    for v in VARIANTS:
        for p, d in progs.items():
            cc(v, os.path.join(HERE, "startup.c"), f"{p}.{v}", [d])
        cc(v, os.path.join(HERE, "percall.c"), f"pc.{v}")
    print("built", len(VARIANTS) * 5, "programs")

    # ---- start-up: 21 runs each, median (and min/max of user instructions and faults)
    st = open("startup.tsv", "w")
    st.write("prog\tvariant\t" + "\t".join(SHORT) + "\tiu_min\tiu_max\tpf_min\tpf_max\tminpct\n")
    env_loc = dict(os.environ, LANG="C.UTF-8", LC_ALL="")
    print("\n== START-UP (median of 21 runs; iu/ik = user/kernel instructions, pf = page faults, cu = user cycles)")
    print("prog    variant      iu        ik       pf   cu        lock  [iu min..max] [pf min..max]")
    for p in progs:
        for v in VARIANTS:
            runs, minpct = [], 100.0
            for _ in range(21):
                vals, pct, rc, err = stat(["./" + f"{p}.{v}"], env=env_loc if p == "setloc" else None)
                if rc != 0:
                    print("RUN-FAIL", p, v, rc, err[-300:])
                    break
                runs.append(vals)
                minpct = min(minpct, pct)
            if not runs:
                continue
            med = {k: statistics.median([r[k] for r in runs if r.get(k) is not None] or [float("nan")]) for k in SHORT}
            iu = [r["iu"] for r in runs]
            pf = [r["pf"] for r in runs]
            st.write(f"{p}\t{v}\t" + "\t".join(f"{med[k]:.0f}" for k in SHORT)
                     + f"\t{min(iu):.0f}\t{max(iu):.0f}\t{min(pf):.0f}\t{max(pf):.0f}\t{minpct:.1f}\n")
            print(f"{p:7} {v:11} {med['iu']:9.0f} {med['ik']:9.0f} {med['pf']:4.0f} {med['cu']:9.0f} {med['lock']:5.0f}"
                  f"  [{min(iu):.0f}..{max(iu):.0f}] [{min(pf):.0f}..{max(pf):.0f}] pct={minpct:.0f}")
    st.close()

    # ---- per call: (count(N) - count(0)) / N, 3 repetitions, median
    modes = [("malloc_free", 200000), ("malloc_free_4k", 200000), ("snprintf_d", 200000),
             ("snprintf_mixed", 200000), ("fprintf_null", 200000), ("fopen_fclose", 20000), ("strlen64", 200000),
             ("memcpy64", 200000), ("memcmp64", 200000), ("strcmp64", 200000), ("strchr_miss64", 200000),
             ("strtol", 200000), ("toupper_c", 200000), ("iswalpha_loc", 200000), ("towupper_loc", 200000),
             ("strcoll_loc", 200000), ("mbrtowc_loc", 200000), ("pthread", 2000), ("pthread", 1)]
    env = dict(os.environ, PGSA_LOCALE=loc)
    pt = open("percall.tsv", "w")
    pt.write("mode\tN\tvariant\t" + "\t".join(SHORT) + "\tminpct\n")
    print("\n== PER CALL (median of 3 x (N - 0)/N)")
    print("mode            N      variant      iu/call   cu/call  lock/call brm/call  ik/call  pf/call")
    for mode, n in modes:
        for v in VARIANTS:
            per, minpct, fail = [], 100.0, ""
            for _ in range(3):
                z, p0, rc0, e0 = stat([f"./pc.{v}", mode, "0"], env=env)
                x, p1, rc1, e1 = stat([f"./pc.{v}", mode, str(n)], env=env)
                if rc0 or rc1:
                    fail = f"rc={rc0},{rc1} {(e0 or e1)[-200:]}"
                    break
                minpct = min(minpct, p0, p1)
                per.append({k: ((x[k] - z[k]) / n if x.get(k) is not None and z.get(k) is not None else float("nan"))
                            for k in SHORT})
            if fail:
                print(f"{mode:15} {n:<6} {v:11} FAIL {fail}")
                pt.write(f"{mode}\t{n}\t{v}\tFAIL\n")
                continue
            med = {k: statistics.median([q[k] for q in per]) for k in SHORT}
            pt.write(f"{mode}\t{n}\t{v}\t" + "\t".join(f"{med[k]:.2f}" for k in SHORT) + f"\t{minpct:.1f}\n")
            print(f"{mode:15} {n:<6} {v:11} {med['iu']:9.1f} {med['cu']:9.1f} {med['lock']:9.2f} {med['brm']:8.2f}"
                  f" {med['ik']:8.0f} {med['pf']:8.2f}")
    pt.close()
    print("\n== done")


main()
