#!/bin/bash
# test_jobwait.sh: run on a host with ~/bin/jobrun installed (JOBWAIT=<path> tests another copy). Private logs dir and
# slots state; nothing outside a mktemp dir is touched. Each case names the defect it guards (TOOLING-PAPERCUTS, 11-20).
set -u
W=${JOBWAIT:-$(dirname "$(readlink -f "$0")")/jobwait}; J=${JOBRUN:-$HOME/bin/jobrun}
T=$(mktemp -d); export JOBRUN_LOGS=$T JOBWAIT_SLOTS_STATE=$T/state JOBWAIT_POLL=1 JOBWAIT_HOST=h1; : >"$T/state"; fail=0
ok() { echo "$1: OK"; }; bad() { echo "$1: FAIL ($2)"; fail=1; }
# run <name> <expected rc> <max seconds> <output regex> -- args...
run() { local n=$1 want=$2 lim=$3 re=$4; shift 5; local s e out r
  s=$(date +%s); out=$("$W" "$@" 2>&1); r=$?; e=$(( $(date +%s) - s ))
  if [ "$r" = "$want" ] && [ "$e" -le "$lim" ] && printf '%s' "$out" | grep -qE "$re"; then ok "$n"; else bad "$n" "rc $r in ${e}s: $(printf '%s' "$out" | tail -n 1)"; fi; }
row() { printf '%s\ttest\t1\t2\t%s\t1\t0\t0\tprocio:ppid\th1\t\t\t\n' "$1" "$2" >>"$T/jobs.tsv"; }
row done0 0; row done7 7
run "help prints usage, no wait"          0 2 'Exit: 0 all done' -- --help
run "-h prints usage"                     0 2 'jobwait \[--max S\]' -- -h
run "unknown option refused"              2 2 'unknown option --foo' -- done0 --foo
run "finished rc 0"                       0 2 'done0' -- done0
run "finished rc != 0"                    1 2 'done7' -- done0 done7
run "host:label on this host"             0 2 'done0' -- h1:done0
run "host:label on another host refused"  2 2 "not h1" -- h2:done0
run "bad label refused"                   2 2 'bad label' -- 'a/b'
# a queued job (slots state W) is alive: --max after the label is honoured (it used to become a label, so the wait ran
# to the 570 s default)
printf '1 2 1 W shared:queued1 N 0 - - 1 N 0 0 -\n' >"$T/state"
JOBWAIT_GRACE=0 run "--max after label honoured"  3 5 'still running: queued1' -- queued1 --max 2
JOBWAIT_GRACE=0 run "--max=S form"                3 5 'still running: queued1' -- --max=2 queued1
# a running job (jobrun --inner holds .locks/<label>) is alive; jobwait reads the lock, never takes it
mkdir -p "$T/.locks"; flock "$T/.locks/held1" sleep 8 & hp=$!; sleep 0.5
JOBWAIT_GRACE=0 run "lock holder is alive"        3 5 'still running: held1' -- --max 2 held1
flock -n "$T/.locks/held1" true && bad "lock not taken by jobwait" "lock free while holder runs" || ok "lock not taken by jobwait"
kill "$hp" 2>/dev/null; wait "$hp" 2>/dev/null
# no row, no queue entry, no lock holder: exit 4 after the grace instead of "still running" until --max
JOBWAIT_GRACE=2 run "unknown label ends early"    4 6 'never started' -- --max 60 nosuch
: >"$T/dead1.log"; : >"$T/.locks/dead1"
JOBWAIT_GRACE=2 run "dead job ends early"         4 6 'ended without its row' -- --max 60 dead1 queued1
# integration with the real jobrun: a short job just started is waited for, not declared dead
JOBRUN_NOQUEUE=1 setsid nohup "$J" jw-int test -- sleep 4 >/dev/null 2>&1 </dev/null &
JOBWAIT_GRACE=3 run "real jobrun job waited for"  0 15 'jw-int' -- --max 30 jw-int
wait
rm -rf "$T"
[ "$fail" = 0 ] && echo "test_jobwait: all OK" || echo "test_jobwait: FAILED"
exit $fail
