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
check wl_count_run_cache_apart 2 "$(grep -c '^msuf=.r.tsv$\|^\[ "${WLC_ASLR:-0}" = 1 \] && msuf=.p.tsv$' "$here/wl_count_run.sh")"
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
    check run_prefix_fixed_layout "env -i HOME=/ PATH=/usr/bin:/bin LANG=en_US.UTF-8 LC_CTYPE=C.UTF-8 setarch -R" "$(wl_run_prefix)"
    # FIXED ENVIRONMENT (WL-LEVELS-RERECORD, D-X.155; no spec clause, lab tooling): what a measured program sees does
    # not depend on the caller. Two callers with different environments (one large, one empty but for PATH) start the
    # same measured command; it must see the same variables, values and total size, and nothing of the caller's.
    envseen() { $(wl_run_prefix) sh -c 'env | LC_ALL=C sort'; }
    e1=$(env WL_STATS_CALLER=1 SSH_CONNECTION="caller-a 2 caller-b 4" JOBRUN_LABEL=a-long-job-label-0123456789 bash -c "$(declare -f wl_run_prefix envseen); WL_RUN_ENV='$WL_RUN_ENV'; envseen")
    e2=$(env -i PATH=/usr/bin:/bin bash -c "$(declare -f wl_run_prefix envseen); WL_RUN_ENV='$WL_RUN_ENV'; envseen")
    check run_env_same_for_any_caller "$e1" "$e2"
    check run_env_same_size "$(printf '%s' "$e1" | wc -c)" "$(printf '%s' "$e2" | wc -c)"
    check run_env_no_caller_vars 0 "$(printf '%s\n' "$e1" | grep -c 'WL_STATS_CALLER\|SSH_CONNECTION\|JOBRUN_LABEL')"
    check run_env_fixed_vars 4 "$(printf '%s\n' "$e1" | grep -c '^HOME=/$\|^PATH=/usr/bin:/bin$\|^LANG=en_US.UTF-8$\|^LC_CTYPE=C.UTF-8$')"
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

# EXCURSION RE-MEASURE (D-X.157; no spec clause: lab tooling). Rows as wl_count_run.sh stores them (11 fields).
xrow() { printf '%s\t0.00\t%s\t%s\t81201\tabcdef123456\t%s\t%s\t0\t0\t16125' "$@"; }   # instr_u instr_k pf min max
# the Zen 1 count k2's tar-gz rows after its 2 % faults re-measure: new 758..786 (every run of both batches high),
# base 629..921, glibc 523..814
k2n=$(xrow 9178315593 416678362 758 758 786)
k2b=$(xrow 9178316180 402169277 629 629 921)
checkrc long_row_tar_gz 0 wl_long_row "$k2n" "$k2b"
checkrc long_row_short_pipeline 1 wl_long_row "$(xrow 61769826 5 476 476 480)" "$(xrow 61495127 5 465 465 470)"
checkrc long_row_either_column 0 wl_long_row "$(xrow 2475742619 5 276 276 276)" "$(xrow 5689349866 5 276 276 276)"
check excursion_new_high new "$(wl_excursion_side "$k2n" "$k2b")"
check excursion_base_high base "$(wl_excursion_side "$(xrow 9178315593 5 637 637 782)" "$(xrow 9178316180 5 765 765 877)")"
check excursion_normal_spread "" "$(wl_excursion_side "$(xrow 9 5 648 648 917)" "$(xrow 9 5 629 629 629)")"
check excursion_edge_7pct "" "$(wl_excursion_side "$(xrow 9 5 107 107 107)" "$(xrow 9 5 100 100 100)")"
check excursion_over_7pct new "$(wl_excursion_side "$(xrow 9 5 108 108 108)" "$(xrow 9 5 100 100 100)")"
check excursion_not_a_number "" "$(wl_excursion_side "$(xrow 9 5 - - -)" "$(xrow 9 5 100 100 100)")"
check row_lowest_joins "$(xrow 9178315593 416678362 631 631 786)" "$(wl_row_lowest "$k2n" "9178315000 0.00 416000000 640 631 655")"
check row_lowest_keeps_lower "$(xrow 9178315593 416678362 758 758 801)" "$(wl_row_lowest "$k2n" "9178315000 0.00 416000000 790 770 801")"
# replay: MEASURE is a stub printing XBATCH (a wl_stats line) and logging the column it was asked for
xstub() { echo "$1" >> "$t/xcalls"; echo "$XBATCH"; }
xverdict() { # new_row base_row -> wl_verdict check of a tar-gz table against the recorded Zen 1 levels
    { echo "== tar-gz   (new a, base b)"
      echo "instructions:u $(echo "$1" | cut -f1) $(echo "$2" | cut -f1) 9177796840"
      echo "page-faults $(echo "$1" | cut -f4) $(echo "$2" | cut -f4) 523"
      echo "faults min..max $(echo "$1" | cut -f7)..$(echo "$1" | cut -f8) $(echo "$2" | cut -f7)..$(echo "$2" | cut -f8) 523..814"; } > "$t/x.log"
    python3 "$here/wl_verdict.py" check "$t/x.log" --holds "$here/wl_holds.tsv" --cpu "AMD EPYC 7551 32-Core Processor" > "$t/x.out"
}
# (a) k2 as recorded: without a further batch the row fails its unchanged level 1.2371 (758 / 523 = 1.4493)
checkrc k2_tar_gz_fails_as_recorded 1 xverdict "$k2n" "$k2b"
# (b) the column measured again finds its own number (a batch in the 629..647 range of the other 8 columns): the new
# column becomes 631 and the row passes the same level, one batch, nothing else moved
: > "$t/xcalls"; WL_RN=$k2n WL_RB=$k2b XBATCH="9178315000 0.00 416000000 640 631 655"
wl_excursions xstub
check k2_excursion_one_batch new "$(paste -sd' ' "$t/xcalls")"
check k2_excursion_new_lowest 631 "$(echo "$WL_RN" | cut -f4)"
check k2_excursion_base_kept "$k2b" "$WL_RB"
check k2_excursion_note "excursion re-measure (D-X.157): round 1: new 758, batch 631..655 -> 631; within 7 %" "$WL_XNOTE"
checkrc k2_tar_gz_passes_after_remeasure 0 xverdict "$WL_RN" "$WL_RB"
check k2_level_unchanged 1 "$(grep -c "^AMD EPYC 7551 32-Core Processor	tar-gz	pf	523	647	1.2371	" "$here/wl_holds.tsv")"
# (c) PLANTED REGRESSION of that size: the new column faults 758..786 in every run (as both of k2's batches did), so no
# batch can lower it: 3 batches, still over, and the verdict fails exactly as before
: > "$t/xcalls"; WL_RN=$k2n WL_RB=$k2b XBATCH="9178315000 0.00 416000000 770 758 786"
wl_excursions xstub
check regression_three_batches "new new new" "$(paste -sd' ' "$t/xcalls")"
check regression_stays 758 "$(echo "$WL_RN" | cut -f4)"
check regression_note_says_real 1 "$(echo "$WL_XNOTE" | grep -c 'new still over 7 % after 3 batches: reported as measured')"
checkrc regression_still_fails 1 xverdict "$WL_RN" "$WL_RB"
check regression_failure_named 1 "$(grep -c 'FAIL tar-gz pf: 758 is 1.449' "$t/x.out")"
# a short row and the ASLR (median) method are never measured again; a pair within 7 % is not either
: > "$t/xcalls"; WL_RN=$(xrow 61769826 5 560 560 560) WL_RB=$(xrow 61495127 5 465 465 470)
wl_excursions xstub
check short_row_not_remeasured "0 " "$(wc -l < "$t/xcalls") $WL_XNOTE"
WL_RN=$k2n WL_RB=$k2b
WLC_ASLR=1 wl_excursions xstub
check aslr_not_remeasured "0 " "$(wc -l < "$t/xcalls") $WL_XNOTE"
WL_RN=$(xrow 9178315593 5 647 647 650) WL_RB=$k2b
wl_excursions xstub
check within_threshold_not_remeasured "0 " "$(wc -l < "$t/xcalls") $WL_XNOTE"
check wl_count_run_calls_excursions 1 "$(grep -c '^        wl_excursions xmeasure || ' "$here/wl_count_run.sh")"

echo "wl_stats_test: $pass passed, $fail failed"
[ $fail = 0 ]
