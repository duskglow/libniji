#!/bin/bash
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# Self-test of validation/wl_stats.sh (median / min / max and the 2% re-measure trigger).
#   bash validation/wl_stats_test.sh          -> exit 0 and "wl_stats_test: N passed" or exit 1
#   WL_STATS_PLANT=mean bash validation/wl_stats_test.sh   -> plants a mean-based faults column; MUST fail
#   WL_STATS_PLANT=futex|offby1|nofutex|first|nomt|allmt bash validation/wl_stats_test.sh  -> plants of wl_strace_sys (the syscall split); each MUST fail (validation/wl_stats_plants.sh runs them all)
set -u
here=$(cd "$(dirname "$0")" && pwd)
. "$here/wl_stats.sh"
if [ "${WL_STATS_PLANT:-}" = mean ]; then
    # the old behaviour: mean of the faults instead of the median
    wl_stats() { awk '{ n++; a += $1; b += $2; c += $3; p[n] = $3 }
        END { printf "%.0f 0.00 %.0f %.0f %d %d\n", a / n, b / n, c / n, p[1], p[n] }'; }
fi
pass=0
fail=0
check() { # name expected actual
    if [ "$2" = "$3" ]; then pass=$((pass + 1)); else fail=$((fail + 1)); echo "FAIL $1: expected [$2] got [$3]"; fi
}
checkrc() { # name expected_rc command...
    local name=$1 want=$2
    shift 2
    "$@"
    check "$name" "$want" "$?"
}

# the gzip-c incident: one outlier (479) among stable 309-310 runs
out=$(printf '%s\n' "1000 5 309" "1000 5 479" "1000 5 310" "1000 5 309" "1000 5 310" | wl_stats)
check outlier_median "1000 0.00 5 310 309 479" "$out"
# the mean would have been 363.4 -> the median is what must be reported
check three_runs "1000 0.00 5 310 309 479" "$(printf '%s\n' "1000 5 479" "1000 5 310" "1000 5 309" | wl_stats)"
# even count: middle two averaged, rounded down
check even_count "1000 0.00 5 310 309 312" "$(printf '%s\n' "1000 5 312" "1000 5 309" "1000 5 311" "1000 5 310" | wl_stats)"
# single run, all equal
check single "7 0.00 1 42 42 42" "$(printf '%s\n' "7 1 42" | wl_stats)"
# instructions: mean and sd
check instr_sd "150 47.14 5 3 3 3" "$(printf '%s\n' "100 5 3" "200 5 3" | wl_stats)"
check empty "- - - - - -" "$(printf '' | wl_stats)"

checkrc differ_big 0 wl_faults_differ 479 310
checkrc differ_same 1 wl_faults_differ 310 310
checkrc differ_edge_2pct 1 wl_faults_differ 102 100
checkrc differ_over_2pct 0 wl_faults_differ 103 100
checkrc differ_below 0 wl_faults_differ 97 100
checkrc differ_base0 0 wl_faults_differ 5 0
checkrc differ_both0 1 wl_faults_differ 0 0

# wl_strace_sys / wl_strace_median (WLCOUNT-FUTEX, WLCOUNT-MTMEM-2): "GATING FUTEX MTMEM" from `strace -c -f` summaries, with and
# without the errors column; the classification is wl_syscls.sh's, the same code wl_run.sh uses.
case "${WL_STATS_PLANT:-}" in
    futex) wl_strace_sys() { sed 's/^ *//; s/  */ /g' "$1" | awk -v mt="${2:-0}" '$NF == "total" { print $4, 0, 0 }'; } ;;   # the old total incl. futex and mtmem
    offby1) wl_strace_sys() { local l; l=$(wl_classify "$1" "${2:-0}") || return; echo "$(echo "$l" | awk -F'\t' '{ print $1 - 1, $10, $11 }')"; } ;;   # the gating count one too low
    first) wl_strace_median() { shift 2; wl_strace_sys "$1" 1; } ;;   # one strace run instead of the median of three
    nomt) wl_strace_median() { local mt=$1 mp=$2; shift 2; mt=0; for f in "$@"; do wl_strace_sys "$f" 0 "$mp"; done | awk '{ n++; a[n] = $1; b[n] = $2; c[n] = $3; d[n] = $4 } END { print a[(n + 1) / 2], b[(n + 1) / 2], c[(n + 1) / 2], d[(n + 1) / 2] }'; } ;;   # thread flag ignored
    allmt) wl_strace_sys() { local l; l=$(wl_classify "$1" 1); echo "$l" | awk -F'\t' '{ print $1, $10, $11, $12 }'; } ;;   # every run treated as threaded
    nofutex) WL_CLASSIFY=${WL_CLASSIFY//'s == "futex"'/'s == "nothing"'} ;;   # futex never recognised
esac
t=$(mktemp -d) && trap 'rm -rf "$t"' EXIT
printf '%s\n' '% time     seconds  usecs/call     calls    errors syscall' '------ ----------- ----------- --------- --------- ----------------' \
    ' 60.00    0.000100           1       300        12 futex' ' 30.00    0.000050           1       100           read' \
    ' 10.00    0.000010           1        50         3 openat' '  5.00    0.000010           1        20           fstat' \
    '  2.00    0.000010           1        30           mmap' '  1.00    0.000010           1         8           munmap' '------ ----------- ----------- --------- --------- ----------------' \
    '100.00    0.000160           1       508        15 total' > "$t/e.txt"
check strace_errors_col_single "208 300 0 0" "$(wl_strace_sys "$t/e.txt" 0)"        # mmap/munmap stay in the gate: 100 + 50 + 20 + 38
check strace_errors_col_threaded "170 300 38 0" "$(wl_strace_sys "$t/e.txt" 1)"
check strace_default_single "208 300 0 0" "$(wl_strace_sys "$t/e.txt")"
printf '%s\n' '% time     seconds  usecs/call     calls syscall' '------ ----------- ----------- --------- ----------------' \
    ' 90.00    0.000100           1       300 futex' ' 10.00    0.000050           1       100 read' \
    '------ ----------- ----------- --------- ----------------' '100.00    0.000160           1       400 total' > "$t/n.txt"
check strace_no_errors_col "100 300 0 0" "$(wl_strace_sys "$t/n.txt" 1)"
printf '%s\n' '% time     seconds  usecs/call     calls    errors syscall' ' 50.00    0.0           1       100           read' '100.00    0.0           1       100 total' > "$t/z.txt"
check strace_no_futex "100 0 0 0" "$(wl_strace_sys "$t/z.txt" 1)"
# a threaded run: two runs that differ only in futex waits and mapping calls give the same gating number (the point of MTMEM-2)
sed 's/ 300        12 futex/ 777        12 futex/; s/ 30 *mmap/ 31           mmap/; s/ 8 *munmap/ 5           munmap/; s/ 508 .* total/ 983 15 total/' "$t/e.txt" > "$t/e2.txt"
check strace_threaded_two_runs_same_gating "$(wl_strace_sys "$t/e.txt" 1 | cut -d' ' -f1)" "$(wl_strace_sys "$t/e2.txt" 1 | cut -d' ' -f1)"
check strace_threaded_two_runs_differ_advisory "170 777 36 0" "$(wl_strace_sys "$t/e2.txt" 1)"
# one more deterministic call (a code change adding a syscall per call) must show, threaded or not: read 100 -> 101
sed 's/ 100           read/ 101           read/; s/ 508 .* total/ 509 15 total/' "$t/e.txt" > "$t/p.txt"
check strace_plus_one_threaded "171 300 38 0" "$(wl_strace_sys "$t/p.txt" 1)"
check strace_plus_one_single "209 300 0 0" "$(wl_strace_sys "$t/p.txt" 0)"
# median of three: gating 807 / 804 / 804 -> 804; futex 862 / 846 / 943 -> 862; mtmem 5 / 9 / 7 -> 7
for n in 1 2 3; do printf '%s\n' '% time seconds usecs/call calls syscall' '------ ------ ------ ------ ------' ' 60.00    0.0           1       X          futex' ' 40.00    0.0           1       Y           mmap' ' 10.00    0.0           1       W           write' '100.00    0.0           1       Z total' > "$t/m$n.txt"; done
sed -i 's/X/862/; s/Y/5/; s/W/807/; s/Z/1674/' "$t/m1.txt"; sed -i 's/X/846/; s/Y/9/; s/W/804/; s/Z/1659/' "$t/m2.txt"; sed -i 's/X/943/; s/Y/7/; s/W/804/; s/Z/1754/' "$t/m3.txt"
check strace_median_threaded "804 862 7 0" "$(wl_strace_median 1 0 "$t/m1.txt" "$t/m2.txt" "$t/m3.txt")"
check strace_median_single "812 862 0 0" "$(wl_strace_median 0 0 "$t/m1.txt")"
check strace_median_one_threaded "807 862 5 0" "$(wl_strace_median 1 0 "$t/m1.txt")"
check strace_empty "- - - -" "$(: > "$t/0.txt"; wl_strace_sys "$t/0.txt" 1)"
# wl_count_run.sh takes its syscall numbers (thread and process flags included) from wl_strace_adaptive over `strace -C -f
# -y` traces (pipe fds), runs every measured run under the run setup, and writes the lowest run (field ff) into its row
check wl_count_run_uses_adaptive 1 "$(grep -c 'sys=$(wl_strace_adaptive "$tmp" st_pass)' "$here/wl_count_run.sh")"
check wl_count_run_traces_pipe_fds 1 "$(grep -c 'strace -C -f -y -o "$tmp/strace$1.txt"' "$here/wl_count_run.sh")"
check wl_count_run_setup_every_run 4 "$(grep -c '&& $pin \(sh -c\|perf stat\|strace\) ' "$here/wl_count_run.sh")"
check wl_count_run_row_fault_field 2 "$(grep -c '\$ff, sys\|wl_row_refaults "\$st" "\$row" "\$ff"' "$here/wl_count_run.sh")"
# rows of the fixed-layout method are cached apart from rows of the method before (never served to each other)
check wl_count_run_cache_apart 2 "$(grep -c '^msuf=.q.tsv$\|^\[ "${WLC_ASLR:-0}" = 1 \] && msuf=.p.tsv$' "$here/wl_count_run.sh")"
# a faults re-measure takes instructions and faults from the new runs and keeps syscalls, md5 and advisory fields
check row_refaults "$(printf '1000\t0.10\t50\t300\t77\tabcdef\t300\t320\t1\t2\t3')" \
    "$(wl_row_refaults "1000 0.10 50 300 300 320" "$(printf '999\t0.20\t40\t400\t77\tabcdef\t390\t410\t1\t2\t3')" 5)"
check row_refaults_aslr_median "$(printf '1000\t0.10\t50\t310\t77\tabcdef\t300\t320\t1\t2\t3')" \
    "$(wl_row_refaults "1000 0.10 50 310 300 320" "$(printf '999\t0.20\t40\t400\t77\tabcdef\t390\t410\t1\t2\t3')" 4)"
check wl_count_run_row_fields 1 "$(grep -c "print NF }' \"\$cache\")\" = 11 \]" "$here/wl_count_run.sh")"
# PIPERW (D-X.46): an `strace -C -f -y` output of a two-process pipeline; only the pipe calls move between runs
pipetrace() { # pipe_reads file_reads -> file
    local i
    { echo '100 pipe2([3<pipe:[77]>, 4<pipe:[77]>], 0) = 0'
      for ((i = 0; i < $1; i++)); do echo "101 read(0<pipe:[77]>, \"x\", 4096) = 1"; done
      for ((i = 0; i < 10; i++)); do echo "100 write(1<pipe:[77]>, \"x\", 1) = 1"; done
      for ((i = 0; i < $2; i++)); do echo "100 read(5</tmp/in>, \"x\", 4096) = 1"; done
      echo '% time     seconds  usecs/call     calls    errors syscall'
      echo '------ ----------- ----------- --------- --------- ----------------'
      echo " 90.00    0.0           1       $(($1 + 10 + $2))           read"
      echo ' 10.00    0.0           1         1           pipe2'
      echo '------ ----------- ----------- --------- --------- ----------------'
      echo "100.00    0.0           1       $(($1 + 11 + $2)) total"; }
}
pipetrace 20 5 > "$t/q1.txt"; pipetrace 47 5 > "$t/q2.txt"; pipetrace 20 6 > "$t/q3.txt"
check pipe_multi_process "6 0 0 30" "$(wl_strace_sys "$t/q1.txt" 0 1)"
check pipe_reads_differ_gate_same "$(wl_strace_sys "$t/q1.txt" 0 1 | cut -d' ' -f1)" "$(wl_strace_sys "$t/q2.txt" 0 1 | cut -d' ' -f1)"
check pipe_reads_differ_advisory "6 0 0 57" "$(wl_strace_sys "$t/q2.txt" 0 1)"
check pipe_file_read_differs_gate_moves "7 0 0 30" "$(wl_strace_sys "$t/q3.txt" 0 1)"
check pipe_single_process_kept "36 0 0 0" "$(wl_strace_sys "$t/q1.txt" 0 0)"
check pipe_median "6 0 0 30" "$(wl_strace_median 0 1 "$t/q1.txt" "$t/q2.txt" "$t/q1.txt")"

# --- WL-COUNT-HONEST-CHEAP (D-X.151): the run setup, the lowest-run statistic, flags from the traces, adaptive passes,
# the cost summary. no spec clause (lab tooling).
# The row's page faults are the LOWEST run in the fixed layout: the outlier set above reports 309, not 310 or 479.
check fault_field_lowest 5 "$(wl_fault_field)"
check fault_field_aslr_median 4 "$(WLC_ASLR=1 wl_fault_field)"
check lowest_of_outlier_set 309 "$(printf '%s\n' "1000 5 309" "1000 5 479" "1000 5 310" "1000 5 309" "1000 5 310" | wl_stats | cut -d' ' -f"$(wl_fault_field)")"
if command -v setarch > /dev/null; then
    check run_prefix_fixed_layout "setarch -R" "$(wl_run_prefix)"
else
    echo "wl_stats_test: skip run_prefix_fixed_layout: setarch is not installed here"
fi
check run_prefix_aslr_none "" "$(WLC_ASLR=1 wl_run_prefix)"
ctrace() { # reads thread child text -> an `strace -C -f -y` trace; text = 1 adds writes of text naming clone calls
    local i
    { echo '100 execve("/bin/sh", ["sh", "-c", "x"], 0x7ffc /* 9 vars */) = 0'
      [ "$2" = 1 ] && echo '100 clone3({flags=CLONE_VM|CLONE_FS|CLONE_FILES|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM|CLONE_SETTLS|CLONE_PARENT_SETTID|CLONE_CHILD_CLEARTID, child_tid=0x7f, parent_tid=0x7f, exit_signal=0, stack=0x7f, stack_size=0x7fff00, tls=0x7f} => {parent_tid=[101]}, 88) = 101'
      [ "$3" = 1 ] && echo '100 clone(child_stack=NULL, flags=CLONE_CHILD_CLEARTID|CLONE_CHILD_SETTID|SIGCHLD, child_tidptr=0x7f) = 102'
      [ "$4" = 1 ] && { echo '100 write(1</dev/null>, "#define CLONE_THREAD\t0x00010000\n", 32) = 32'
                        echo '100 write(1</dev/null>, "see 5 clone(2) and 7 vfork(2)\n", 30) = 30'; }
      for ((i = 0; i < $1; i++)); do echo "100 read(5</tmp/in>, \"x\", 4096) = 1"; done
      for ((i = 0; i < 3; i++)); do echo "100 mmap(NULL, 4096, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0) = 0x7f0000"; done
      echo '% time     seconds  usecs/call     calls    errors syscall'
      echo '------ ----------- ----------- --------- --------- ----------------'
      echo " 90.00    0.0           1       $1           read"
      echo '  5.00    0.0           1         3           mmap'
      echo ' 10.00    0.0           1         1           execve'
      [ "$2" = 1 ] && echo '  0.00    0.0           1         1           clone3'
      [ "$3" = 1 ] && echo '  0.00    0.0           1         1           clone'
      [ "$4" = 1 ] && echo '  0.00    0.0           1         2           write'
      echo '------ ----------- ----------- --------- --------- ----------------'
      echo "100.00    0.0           1       $(($1 + 4 + $2 + $3 + 2 * $4)) total"; }
}
ctrace 5 0 0 1 > "$t/c_text.txt"; ctrace 5 1 0 0 > "$t/c_thread.txt"; ctrace 5 0 1 0 > "$t/c_child.txt"
check flags_text_naming_clone_is_not_a_call "0 0" "$(wl_trace_flags "$t/c_text.txt")"
check flags_thread "1 0" "$(wl_trace_flags "$t/c_thread.txt")"
check flags_child "0 1" "$(wl_trace_flags "$t/c_child.txt")"
check flags_any_pass "1 1" "$(wl_trace_flags "$t/c_text.txt" "$t/c_thread.txt" "$t/c_child.txt")"
mkdir -p "$t/ad"
fakepass() { cp "$t/src$1.txt" "$t/ad/strace$1.txt" && echo "$1" >> "$t/ad/ran"; }
# two passes agree: the third is not run, the numbers are pass 1's
ctrace 7 0 0 0 > "$t/src1.txt"; cp "$t/src1.txt" "$t/src2.txt"; ctrace 9 0 0 0 > "$t/src3.txt"; rm -f "$t/ad/ran"
check adaptive_agree "$(wl_strace_sys "$t/src1.txt" 0 0) 2" "$(wl_strace_adaptive "$t/ad" fakepass)"
check adaptive_agree_two_passes "1 2" "$(paste -sd' ' "$t/ad/ran")"
# two passes differ: the third runs and each number is the median of three (7, 11, 9 reads -> 9)
ctrace 11 0 0 0 > "$t/src2.txt"; rm -f "$t/ad/ran"
check adaptive_differ_median "$(wl_strace_median 0 0 "$t/src1.txt" "$t/src2.txt" "$t/src3.txt") 3" "$(wl_strace_adaptive "$t/ad" fakepass)"
check adaptive_differ_three_passes "1 2 3" "$(paste -sd' ' "$t/ad/ran")"
check adaptive_differ_median_value 13 "$(wl_strace_adaptive "$t/ad" fakepass | cut -d' ' -f1)"
# a thread seen only in pass 3 marks every pass threaded (the flags come from all passes: its mmap calls leave the gate)
ctrace 9 1 0 0 > "$t/src3.txt"; rm -f "$t/ad/ran"
check adaptive_thread_flag_matters 1 "$([ "$(wl_strace_median 1 0 "$t/src1.txt" "$t/src2.txt" "$t/src3.txt")" != "$(wl_strace_median 0 0 "$t/src1.txt" "$t/src2.txt" "$t/src3.txt")" ] && echo 1)"
check adaptive_flags_from_all_passes "$(wl_strace_median 1 0 "$t/src1.txt" "$t/src2.txt" "$t/src3.txt") 3" "$(wl_strace_adaptive "$t/ad" fakepass)"
# the cost summary of a job log
{ echo 'wl_phase: 0 lockwait abc-release-dyn'
  echo 'wl_phase: 812 build abc-release-dyn built'
  echo 'wl_phase: 0 build def-release-dyn cached'
  echo 'wl_phase: 3 relink abc-release-dyn 18 programs'
  echo 'wl_phase: 0 count seq glibc hit'
  echo 'wl_phase: 40 count seq base miss perf=10s md5=2s strace=28s passes=2'
  echo 'wl_phase: 300 count printf-loop new miss perf=100s md5=20s strace=180s passes=3'
  echo 'wl_phase: 12 refaults seq new'
  echo 'an ordinary log line'; } > "$t/phase.log"
wl_phase_summary "$t/phase.log" > "$t/phase.out"
check phase_steps "wl_cost: steps: build 812 s (built 1, cached 1), lock wait 0 s, relink 3 s (18 programs), count 340 s, faults re-measure 12 s" "$(sed -n 1p "$t/phase.out")"
check phase_glibc_hit "wl_cost: column glibc: 1 rows, 1 cached (hit), 0 measured (miss); 0 s (perf 0, md5 0, strace 0 in 0 passes); faults re-measured 0 rows, 0 s" "$(sed -n 2p "$t/phase.out")"
check phase_new_miss "wl_cost: column new: 1 rows, 0 cached (hit), 1 measured (miss); 300 s (perf 100, md5 20, strace 180 in 3 passes); faults re-measured 1 rows, 12 s" "$(sed -n 4p "$t/phase.out")"
check phase_slowest "wl_cost: slowest: printf-loop new  300 s, seq base  40 s, seq new (faults re-measure) 12 s, seq glibc  0 s" "$(sed -n 5p "$t/phase.out")"

echo "wl_stats_test: $pass passed, $fail failed"
[ $fail = 0 ]
