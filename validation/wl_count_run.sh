#!/bin/bash
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# wl_count.sh, Zen 1 host side: deterministic counters of named workloads for glibc / BASE / NEW, one table each.
#
#   wl_count_run.sh WORKLOAD,WORKLOAD,... BASE_SHA NEW_SHA
#
# Kits (~/wl/kit/$(wl_vkey <variant>)/bin: <sha>-release for a commit, validation/wl_lock.sh; with at least the programs the workloads use) are made by wl_prep.sh
# first. Per workload and variant, every run under the RUN SETUP (setarch -R, see wl_stats.sh): one warm-up, FAULT_RUNS
# (default 5) single `perf stat` runs (instructions:u, instructions:k, page-faults of the shell and its children), one
# run through md5sum (output check), two `strace -C -f -y` runs and a third only when they differ, median (syscalls: the
# gating number EXCLUDES the scheduling-dependent calls, reported apart, see below). Page faults are the LOWEST of the
# FAULT_RUNS runs (WLC_ASLR=1: ASLR on and the median, the method before WL-COUNT-HONEST-CHEAP), with min..max
# printed; instructions are the mean (exact). When new and base faults differ by > FAULT_PCT (default 2) % the faults
# of both are re-measured fresh in the same job (not the syscalls or output, which do not vary) and the table says so.
# Each step logs a `wl_phase:` line (PHASES below). Rows are cached in ~/wl/count/<variant>/<workload>.q.tsv (a variant
# is a commit: kit, inputs and the host's glibc do not change); COUNT_FRESH=1 ignores the cache.
# FUTEX (WLCOUNT-FUTEX, 2026-10-21): futex wait/wake counts of threaded workloads (sort-C, sort-utf8) depend on
# scheduling and vary run to run with identical code (3027..3638 in one build). The "syscalls" row is therefore the
# count EXCLUDING futex calls (deterministic, the gating number); the futex calls are the separate advisory row
# "futex (advisory)" and are never a verdict. Single-threaded workloads have futex 0, so their syscalls are unchanged.
# MTMEM (WLCOUNT-MTMEM-2, 2026-10-21; the rule of wl_run.sh, WLRUN-FUTEX, from the same code in wl_syscls.sh): when the run
# created a thread (a clone/clone3 with CLONE_THREAD, found by a second, tiny `strace -f -e trace=clone,clone3` run), ALL its
# mmap/munmap/mremap/brk/mprotect/madvise calls are the separate advisory row "mtmem (advisory)" and not in "syscalls" (a
# worker sometimes gets its own allocator arena, and arena alignment trims depend on addresses the kernel returns while
# other threads map). Single-threaded workloads keep those calls in the gating number.
# PIPERW (WL-PIPE-READS-ADVISORY, D-X.46): when the run created another process (a pipeline such as pipe-mix or tar-gz:
# wl_has_child on the clone trace), its read/write calls on pipes depend on scheduling (how much data waits when a reader
# wakes); they are the advisory row "pipe rw (advisory)" and not in "syscalls". Single-process runs keep them.
# Row: instr_u  instr_sd%  instr_k  page-faults(median)  syscalls(gating)  md5  faults_min  faults_max  futex  mtmem  piperw
# (a cached row with other than 11 fields predates the piperw split and is re-measured.)
# WLC_ENV='NAME=VALUE ...': environment for the measured command in every variant (see wl_count.sh); the
# cache file name carries a hash of it. Stdout of the measured command always goes into a pipe (| cat), never
# straight to /dev/null (GNU grep exits at the first match on /dev/null); cache files of this method end .q.tsv (.p.tsv: WLC_ASLR=1).
set -u
WL=${WL:-$HOME/wl}
list=$1
base=$2
new=$3
. "$(dirname "$0")/wl_workloads.sh"
. "$(dirname "$0")/wl_lock.sh"   # wl_key: a commit's kit and rows are keyed by sha and products profile (WL-PREP-RELEASE-PROFILE)
work=$WL/work
tmp=$WL/tmp.count.$$
mkdir -p "$tmp"
trap 'rm -rf "$tmp"' EXIT
[ -d "$work/tree" ] || { mkdir -p "$work/tree" && tar -xf "$work/tree.tar" -C "$work/tree"; }

. "$(dirname "$0")/wl_stats.sh"
nruns=${FAULT_RUNS:-5}
TIME_DEPENDENT="ls-lR ls-l-enUS"   # workloads whose output depends on the current date (see measure)

ev() { echo "$1" | grep ",$2," | tail -1 | cut -d, -f1; }

wenv=${WLC_ENV:-}
etag=
[ -n "$wenv" ] && etag=.e$(printf '%s' "$wenv" | md5sum | cut -c1-8)
# RUN SETUP (WL-COUNT-HONEST-CHEAP, D-X.151; wl_stats.sh): every measured run (warm-up, perf, md5, strace) of every column
# starts under $pin (env -i with the fixed WL_RUN_ENV, D-X.155, then setarch -R: one environment, one address layout)
# and a row's page faults are its LOWEST run (ff = the wl_stats field). Rows measured so end .r.tsv (.q.tsv rows were
# measured in the caller's environment and are never served); WLC_ASLR=1 measures as before (ASLR, the median, the
# caller's environment) and reads/writes .p.tsv rows.
pin=$(wl_run_prefix) || exit 2
ff=$(wl_fault_field)
msuf=.r.tsv
[ "${WLC_ASLR:-0}" = 1 ] && msuf=.p.tsv
if [ -n "$pin" ]; then
    echo "wl_count_run: run setup: $pin, page faults = the lowest of $nruns runs"
else
    echo "wl_count_run: run setup: ASLR on, page faults = the median of $nruns runs (WLC_ASLR=1)"
fi
# PHASES (WL-COUNT-HONEST-CHEAP): every step logs `wl_phase: <seconds> <step> ...` on stderr (the job log); wl_count.sh
# sums them. count lines name the column (glibc, base, new) and say hit (cached row) or miss (measured).
col() { case $1 in glibc) echo glibc ;; "$new") echo new ;; "$base") echo base ;; *) echo "$1" ;; esac; }

# perf_runs VARIANT_CMD -> "iu sd% ik pf_med pf_min pf_max": one warm-up, then FAULT_RUNS single perf runs
perf_runs() {
    local c=$1 runs="" i out
    (cd "$work" && $pin sh -c "$c" 2>/dev/null | cat > /dev/null)
    for ((i = 0; i < nruns; i++)); do
        (cd "$work" && $pin perf stat -x, -o "$tmp/perf.txt" -e instructions:u,instructions:k,page-faults sh -c "$c" 2>/dev/null | cat > /dev/null)
        out=$(cat "$tmp/perf.txt")
        runs+="$(ev "$out" instructions:u) $(ev "$out" instructions:k) $(ev "$out" page-faults)"$'\n'
    done
    printf '%s' "$runs" | wl_stats   # iu sd% ik pf_med pf_min pf_max
}

# refaults workload cmd variant row -> the row with its instruction and fault fields measured again (fresh), its
# syscall, md5 and advisory fields kept: only faults vary run to run, so a faults re-measure never re-runs strace
# (printf-loop: one strace pass of 20000 dynamic starts is minutes).
refaults() {
    local name=$1 cmd=$2 v=$3 row=$4 c cache st s0=$SECONDS
    cache=$WL/count/$(wl_vkey "$v")/$name$etag$msuf
    c=${cmd//@/$WL/kit/$(wl_vkey "$v")/bin}
    [ -n "$wenv" ] && c="env $wenv $c"
    st=$(perf_runs "$c")
    wl_row_refaults "$st" "$row" "$ff" | tee "$cache"
    echo "wl_phase: $((SECONDS - s0)) refaults $name $(col "$v")" >&2
}

measure() { # workload cmd variant [fresh] [noenv] -> row on stdout; fresh=1 ignores the cache, noenv=1 drops WLC_ENV
    local name=$1 cmd=$2 v=$3 fresh=${4:-0} c cache e=$wenv t=$etag s0=$SECONDS s1 s2
    [ "${5:-0}" = 1 ] && { e=; t=; }
    cache=$WL/count/$(wl_vkey "$v")/$name$t$msuf
    # Time-dependent output (ls -l prints HH:MM for files younger than ~6 months, the year for older ones, so the
    # md5 changes when a file crosses that boundary): a row cached on an earlier day has a stale md5 and would
    # report DIFF against freshly measured rows (LSLR-OUTPUT-DIFF). Such a row is used only on the day it was made.
    case " $TIME_DEPENDENT " in *" $name "*)
        [ -s "$cache" ] && [ "$(date -r "$cache" +%F)" != "$(date +%F)" ] && fresh=1 ;;
    esac
    if [ "${COUNT_FRESH:-0}" != 1 ] && [ "$fresh" != 1 ] && [ -s "$cache" ] \
        && [ "$(awk -F'\t' 'NR == 1 { print NF }' "$cache")" = 11 ]; then
        echo "wl_phase: 0 count $name $(col "$v") hit" >&2
        cat "$cache"; return
    fi
    c=${cmd//@/$WL/kit/$(wl_vkey "$v")/bin}
    [ -n "$e" ] && c="env $e $c"
    local sum sys st
    st=$(perf_runs "$c")
    s1=$SECONDS
    sum=$(cd "$work" && $pin sh -c "$c" 2>/dev/null | md5sum | cut -c1-12)
    s2=$SECONDS
    # STRACE PASSES (WL-COUNT-HONEST-CHEAP, wl_stats.sh wl_strace_adaptive): the median of three `strace -C -f -y` runs,
    # the third only when the first two differ; thread and process flags from these traces (the separate clone-only
    # strace run that did this before was one more full pass per column).
    local np
    st_pass() { (cd "$work" && $pin strace -C -f -y -o "$tmp/strace$1.txt" sh -c "$c" 2>/dev/null | cat > /dev/null); }
    sys=$(wl_strace_adaptive "$tmp" st_pass) || sys="- - - - 0"
    np=${sys##* }
    sys=${sys% *}   # "GATING FUTEX MTMEM PIPERW"
    rm -f "$tmp"/strace[123].txt
    echo "wl_phase: $((SECONDS - s0)) count $name $(col "$v") miss perf=$((s1 - s0))s md5=$((s2 - s1))s strace=$((SECONDS - s2))s passes=$np" >&2
    local fx mx pr
    read -r _ fx mx pr <<< "$sys"
    # The row's own directory (its key, <sha>-release): `mkdir $WL/count/$v` made <sha>, so `tee` could not write a
    # release row and every run re-measured the base column.
    mkdir -p "${cache%/*}"
    echo "$st" | awk -v sys="${sys%% *}" -v fx="$fx" -v mx="$mx" -v pr="$pr" -v sum="$sum" -v OFS='\t' -v ff="$ff" '{ print $1, $2, $3, $ff, sys, sum, $5, $6, fx, mx, pr }' | tee "$cache"
}

IFS=, read -r -a want <<< "$list"
for w in "${want[@]}"; do
    found=0
    for d in "${WL_DEFS[@]}"; do
        name=${d%%|*}
        [ "$name" = "$w" ] || continue
        found=1
        cmd=${d#*|}
        rg=$(measure "$name" "$cmd" glibc)
        rb=$(measure "$name" "$cmd" "$base")
        rn=$(measure "$name" "$cmd" "$new")
        note=
        envck=
        if [ -n "$wenv" ]; then   # self-check: did the env reach glibc? (its instructions must move)
            r0=$(measure "$name" "$cmd" glibc 0 1)
            if [ "$(echo "$r0" | cut -f1)" = "$(echo "$rg" | cut -f1)" ]; then
                envck="WARNING: glibc column unchanged under WLC_ENV='$wenv' ($(echo "$rg" | cut -f1) instr:u both) - the env did not reach glibc"
            else
                envck="glibc instr:u $(echo "$rg" | cut -f1) under WLC_ENV='$wenv' vs $(echo "$r0" | cut -f1) without: changed (ok)"
            fi
        fi
        if wl_faults_differ "$(echo "$rn" | cut -f4)" "$(echo "$rb" | cut -f4)" "${FAULT_PCT:-2}"; then
            first="new $(echo "$rn" | cut -f4) [$(echo "$rn" | cut -f7)..$(echo "$rn" | cut -f8)], base $(echo "$rb" | cut -f4) [$(echo "$rb" | cut -f7)..$(echo "$rb" | cut -f8)]"
            rb=$(refaults "$name" "$cmd" "$base" "$rb")
            rn=$(refaults "$name" "$cmd" "$new" "$rn")
            note="faults differed by > 2% (first measurement: $first): faults and instructions of new and base re-measured fresh in this job (syscalls and output kept)"
        fi
        printf '%s\n%s\n%s\n' "$rn" "$rb" "$rg" | awk -F'\t' -v name="$name" -v nsha="${new:0:9}" -v bsha="${base:0:9}" -v note="$note" -v envck="$envck" -v stat="$([ "$ff" = 5 ] && echo lowest || echo median)" '
            function ratio(a, b) { return (b + 0 > 0) ? sprintf("%.4f", a / b) : "-" }
            { for (i = 1; i <= 11; i++) f[NR, i] = $i }
            END {
                printf "== %s   (new %s, base %s)\n", name, nsha, bsha
                printf "%-18s %16s %16s %16s %9s %9s\n", "counter", "new", "base", "glibc", "new/glibc", "new/base"
                split("instructions:u instructions:k page-faults syscalls-gating", lab, " ")
                idx[1] = 1; idx[2] = 3; idx[3] = 4; idx[4] = 5
                for (k = 1; k <= 4; k++) {
                    i = idx[k]
                    printf "%-18s %16s %16s %16s %9s %9s\n", lab[k], f[1, i], f[2, i], f[3, i], ratio(f[1, i], f[3, i]), ratio(f[1, i], f[2, i])
                }
                printf "%-18s %16s %16s %16s %9s %9s\n", "futex (advisory)", f[1, 9], f[2, 9], f[3, 9], ratio(f[1, 9], f[3, 9]), ratio(f[1, 9], f[2, 9])
                printf "%-18s %16s %16s %16s %9s %9s\n", "mtmem (advisory)", f[1, 10], f[2, 10], f[3, 10], ratio(f[1, 10], f[3, 10]), ratio(f[1, 10], f[2, 10])
                printf "%-18s %16s %16s %16s %9s %9s\n", "pipe rw (advisory)", f[1, 11], f[2, 11], f[3, 11], ratio(f[1, 11], f[3, 11]), ratio(f[1, 11], f[2, 11])
                printf "instr:u sd%%        %16s %16s %16s\n", f[1, 2], f[2, 2], f[3, 2]
                printf "faults min..max     %16s %16s %16s   (table shows the %s)\n", f[1, 7] ".." f[1, 8], f[2, 7] ".." f[2, 8], f[3, 7] ".." f[3, 8], stat
                if (note != "") printf "note               %s\n", note
                if (envck != "") printf "env check          %s\n", envck
                printf "output             new vs glibc: %s   new vs base: %s\n", (f[1, 6] == f[3, 6] ? "SAME" : "DIFF"), (f[1, 6] == f[2, 6] ? "SAME" : "DIFF")
            }'
    done
    [ $found = 1 ] || echo "wl_count_run: unknown workload $w"
done
