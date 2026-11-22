#!/bin/bash
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# Statistics helpers for wl_count_run.sh (WL-FAULTS-STAT). Source this file.
#
# Page faults are not exact: on a NUMA host with numa_balancing one outlier run can double them, and the
# mean of `perf stat -r 3` then shows a false regression. So faults are reported as the MEDIAN of N runs
# (with min..max beside it); instructions stay a mean (they are exact, sd% is printed).
#
# (sources wl_syscls.sh for the syscall classification)
#   wl_stats      stdin: one run per line "instr_u instr_k page_faults"
#                 stdout: "instr_u_mean instr_u_sd% instr_k_mean faults_median faults_min faults_max"
#                 (even counts: the two middle values are averaged, rounded down)
#   wl_faults_differ NEW BASE [PCT]   exit 0 when |NEW-BASE| > PCT% of BASE (default 2), or BASE is 0 and
#                                      NEW is not; exit 1 otherwise
. "${WL_SYSCLS:-$(dirname "${BASH_SOURCE[0]}")/wl_syscls.sh}"

wl_stats() {
    awk '
        { n++; iu[n] = $1 + 0; ik[n] = $2 + 0; pf[n] = $3 + 0 }
        END {
            if (n == 0) { print "- - - - - -"; exit }
            for (i = 1; i <= n; i++) { su += iu[i]; sk += ik[i] }
            mu = su / n
            for (i = 1; i <= n; i++) v += (iu[i] - mu) ^ 2
            sd = (n > 1 && mu > 0) ? sqrt(v / (n - 1)) / mu * 100 : 0
            for (i = 2; i <= n; i++) { x = pf[i]; j = i - 1; while (j >= 1 && pf[j] > x) { pf[j + 1] = pf[j]; j-- } pf[j + 1] = x }
            med = (n % 2) ? pf[(n + 1) / 2] : int((pf[n / 2] + pf[n / 2 + 1]) / 2)
            printf "%.0f %.2f %.0f %d %d %d\n", mu, sd, sk / n, med, pf[1], pf[n]
        }'
}

# RUN SETUP (WL-COUNT-HONEST-CHEAP, D-X.151): every measured run of every column (glibc, base, new) starts under
#   setarch -R
# (one address layout per program) and the page-fault number of a row is the LOWEST of its FAULT_RUNS runs (median and
# highest still printed). Measured on the Zen 1 host (probes of WL-COUNT-HONEST-CHEAP, 5..10 runs per condition):
# - ASLR moves the stack, heap and mapping bases across page edges, so faults wander run to run: seq glibc 136..142 and
#   libniji 165..168 with ASLR, 133 and 163 in every run without; gzip-c glibc 271..274, libniji 302..307 with, 269
#   and 300 without. That spread was most of the noise and is gone.
# - What is left are rare positive excursions on runs of two seconds or more (gzip-c glibc 403 once in 10 runs, sed-s
#   +10..+12 in 2 of 8): minor faults only, no major faults, no THP (the host's THP is madvise-only), not removed by
#   binding the run to one NUMA node (numactl --cpunodebind --membind: same excursions), so most likely automatic NUMA
#   balancing's hinting faults, which are time driven. They only ever ADD faults, so the lowest run is the program's
#   own number and the median (the method before, 3 good runs of 5 needed) is not.
# WLC_ASLR=1 measures as before (ASLR on, the median; rows cached apart as .p.tsv).
# FIXED ENVIRONMENT (WL-LEVELS-RERECORD, D-X.155): in one address layout the size of the process environment decides
# where the stack starts, so a row moved by about one fault per process with the caller's environment (an ssh session's
# addresses and session id, the job label, cargo's variables). Every measured run therefore starts from `env -i` with
# WL_RUN_ENV: the same variables, values and total size on every run and host, whoever starts the count. LANG and
# LC_CTYPE keep the values every count before this one ran with (an ssh session's); HOME and PATH are fixed literals.
# Rows measured so are cached as .r.tsv (wl_count_run.sh).
#   wl_run_prefix       stdout: the prefix (`env -i WL_RUN_ENV setarch -R`; empty for WLC_ASLR=1, which measures as
#                       before in the caller's environment); exit 1 when setarch is missing
WL_RUN_ENV='HOME=/ PATH=/usr/bin:/bin LANG=en_US.UTF-8 LC_CTYPE=C.UTF-8'
#   wl_fault_field      stdout: the field of wl_stats' output that is the row's page-fault number: 5 (lowest), or 4
#                       (median) for WLC_ASLR=1
#   wl_clone_lines FILE stdout: the clone/clone3/fork/vfork CALL lines of an `strace -f` trace (pid first, or no pid),
#                       for wl_has_thread / wl_has_child: a write() of text naming CLONE_THREAD (grep over
#                       /usr/include) is a trace line too and must not mark the run threaded
wl_run_prefix() {
    [ "${WLC_ASLR:-0}" = 1 ] && return 0
    command -v setarch > /dev/null || { echo "wl_count: setarch is needed for the run setup (WLC_ASLR=1 measures without it)" >&2; return 1; }
    echo "env -i $WL_RUN_ENV setarch -R"
}

wl_fault_field() { if [ "${WLC_ASLR:-0}" = 1 ]; then echo 4; else echo 5; fi; }

# wl_row_refaults ST ROW FF -> ROW (11 tab-separated fields, wl_count_run.sh) with its instruction and fault fields taken
#   from ST (wl_stats output; FF = wl_fault_field) and its syscall, md5 and advisory fields kept: a faults re-measure
#   never re-runs strace (syscalls and output do not vary run to run; printf-loop's strace pass is minutes)
wl_row_refaults() {
    printf '%s\n%s\n' "$1" "$2" | awk -v OFS='\t' -v ff="$3" 'NR == 1 { split($0, s, " ") }
        NR == 2 { split($0, r, "\t"); print s[1], s[2], s[3], s[ff], r[5], r[6], s[5], s[6], r[9], r[10], r[11] }'
}

wl_clone_lines() { grep -E '^([0-9]+ +)?(clone3?|v?fork)\(' "$1"; }

# wl_trace_flags FILE... -> "MT MP": MT = 1 when any trace's clone/fork call lines show a thread (CLONE_THREAD), MP = 1
#   when any shows a new process (wl_has_thread / wl_has_child on wl_clone_lines)
wl_trace_flags() {
    local f mt=0 mp=0
    for f in "$@"; do
        wl_has_thread <(wl_clone_lines "$f") && mt=1
        wl_has_child <(wl_clone_lines "$f") && mp=1
    done
    echo "$mt $mp"
}

# wl_strace_adaptive DIR RUN... -> "GATING FUTEX MTMEM PIPERW PASSES" (WL-COUNT-HONEST-CHEAP). `RUN... K` writes the
#   K-th `strace -C -f -y` trace to DIR/straceK.txt. Each number is the median of three passes; when passes 1 and 2
#   agree on all four numbers the third cannot move any median and is not run (PASSES 2), else pass 3 runs and the
#   median of the three is printed (PASSES 3). The thread and process flags come from the traces themselves
#   (wl_trace_flags over every pass), not from a separate clone-only strace run.
wl_strace_adaptive() {
    local d=$1 a b mt mp
    shift
    "$@" 1 && "$@" 2 || return 1
    read -r mt mp < <(wl_trace_flags "$d/strace1.txt" "$d/strace2.txt")
    a=$(wl_strace_sys "$d/strace1.txt" "$mt" "$mp")
    b=$(wl_strace_sys "$d/strace2.txt" "$mt" "$mp")
    if [ "$a" = "$b" ]; then echo "$a 2"; return 0; fi
    "$@" 3 || return 1
    read -r mt mp < <(wl_trace_flags "$d/strace1.txt" "$d/strace2.txt" "$d/strace3.txt")
    echo "$(wl_strace_median "$mt" "$mp" "$d/strace1.txt" "$d/strace2.txt" "$d/strace3.txt") 3"
}

# wl_phase_summary LOG -> the cost summary of one wl_count run from its `wl_phase: <seconds> <step> ...` lines
#   (wl_prep.sh: lockwait, build <key> built|cached, relink <key> <n> programs; wl_count_run.sh: count <workload>
#   <column> hit|miss perf=Ns md5=Ns strace=Ns passes=N, refaults <workload> <column>): seconds per step, per column
#   with its cache hits and misses, and the five slowest rows.
wl_phase_summary() {
    awk '$1 == "wl_phase:" {
            s = $2 + 0; k = $3; tot[k] += s
            if (k == "build") nb[$5]++
            if (k == "relink") prog += $5
            if (k == "count") {
                c = $5; cs[c] += s; seen[c] = 1
                if ($6 == "hit") hit[c]++
                else { miss[c]++; for (i = 7; i <= NF; i++) { split($i, kv, "="); v = kv[2]; sub(/s$/, "", v); part[c, kv[1]] += v + 0 } }
            }
            if (k == "refaults") { rf[$5] += s; nrf[$5]++ }
        }
        END {
            printf "wl_cost: steps: build %d s (built %d, cached %d), lock wait %d s, relink %d s (%d programs), count %d s, faults re-measure %d s\n",
                tot["build"], nb["built"], nb["cached"], tot["lockwait"], tot["relink"], prog, tot["count"], tot["refaults"]
            split("glibc base new", order, " ")
            for (j = 1; j <= 3; j++) {
                c = order[j]
                if (!seen[c]) continue
                printf "wl_cost: column %s: %d rows, %d cached (hit), %d measured (miss); %d s (perf %d, md5 %d, strace %d in %d passes); faults re-measured %d rows, %d s\n",
                    c, hit[c] + miss[c], hit[c], miss[c], cs[c], part[c, "perf"], part[c, "md5"], part[c, "strace"], part[c, "passes"], nrf[c], rf[c]
            }
        }' "$1"
    awk '$1 == "wl_phase:" && ($3 == "count" || $3 == "refaults") { print $2, $4, $5, $3 }' "$1" | sort -k1,1nr | head -5 \
        | awk '{ s = s sprintf("%s%s %s %s %d s", (NR > 1 ? ", " : ""), $2, $3, ($4 == "refaults" ? "(faults re-measure)" : ""), $1) } END { if (NR) print "wl_cost: slowest: " s }'
}

wl_faults_differ() {
    awk -v a="$1" -v b="$2" -v p="${3:-2}" 'BEGIN {
        d = a - b; if (d < 0) d = -d
        exit (b + 0 > 0 ? (d * 100 > p * b ? 0 : 1) : (d > 0 ? 0 : 1))
    }'
}

# wl_strace_sys FILE [MT] [MP] -> "GATING FUTEX MTMEM PIPERW" from an `strace -C -f -y` (or `strace -c -f`) output (WLCOUNT-FUTEX, WLCOUNT-MTMEM-2). It takes the
# classification from wl_syscls.sh (the same code wl_run.sh uses, so both tools agree): GATING = all calls minus the
# scheduling-dependent ones; FUTEX = futex calls; MTMEM = for a run that created threads (MT=1, found by wl_has_thread on a
# clone trace) its mmap/munmap/mremap/brk/mprotect/madvise calls. Futex counts and a threaded program's arena/stack mappings
# depend on scheduling and vary run to run with identical code, so only GATING is a deterministic counter; the other two
# are advisory. PIPERW = for a run that created another process (MP=1, wl_has_child) its read/write calls on pipes, advisory
# too (WL-PIPE-READS-ADVISORY, D-X.46). Prints "- - - -" when the file has no total line.
wl_strace_sys() {
    if grep -q 'total' "$1"; then
        wl_classify "$1" "${2:-0}" "${3:-0}" | awk -F'\t' '{ printf "%d %d %d %d\n", $1, $10, $11, $12 }'
    else
        echo "- - - -"
    fi
}

# wl_strace_median MT MP FILE... -> "GATING FUTEX MTMEM PIPERW": the median over several `strace -c -f` summaries of each number.
# Kept from WLCOUNT-FUTEX: it costs two extra runs and guards against a one-off strace hiccup in the gating number.
wl_strace_median() {
    local mt=$1 mp=$2 f
    shift 2
    for f in "$@"; do wl_strace_sys "$f" "$mt" "$mp"; done | awk '
        { n++; a[n] = $1; b[n] = $2; c[n] = $3; d[n] = $4 }
        function med(x, m,   i, j, v) { for (i = 2; i <= m; i++) { v = x[i]; j = i - 1; while (j >= 1 && x[j] > v) { x[j + 1] = x[j]; j-- } x[j + 1] = v } return (m % 2) ? x[(m + 1) / 2] : int((x[m / 2] + x[m / 2 + 1]) / 2) }
        END { if (n == 0 || a[1] == "-") print "- - - -"; else printf "%d %d %d %d\n", med(a, n), med(b, n), med(c, n), med(d, n) }'
}
