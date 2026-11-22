#!/bin/bash
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# Self-test of the INVIOLABLE workload verdict (validation/wl_verdict.py, WL-DYNAMIC-GATE, owner ruling #6965, D-X.141;
# no spec clause: measurement tooling) and of the recorded holds (validation/wl_holds.tsv). Seconds, no build:
#   1. a row is bound at 1.10x glibc (1.10 passes, above fails), in instructions:u and in page faults;
#   2. a held row is bound at its recorded level: the level passes, one instruction more fails;
#   3. a held row measuring at or below 1.10x drops its hold (HOLD-RELEASE; --update removes the row); a page-fault
#      row only when its worst run is in bound too;
#   4. the geomean of instructions:u binds at 1.00 on a run of every workload, advisory on a partial run;
#   5. a row held only on another CPU class is reported, not failed;
#   6. the holds ratchet: a level above the measurement it records, a raised level and a new row on a recorded CPU
#      class are refused, and the repository's wl_holds.tsv passes against the merge base's copy when one is on hand;
#   7. wl_count.sh runs the verdict on the dynamic release kit and fails on it.
set -u
here=$(cd "$(dirname "$0")" && pwd)
V=$here/wl_verdict.py
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
pass=0; fail=0
check() { if [ "$2" = "$3" ]; then pass=$((pass + 1)); else fail=$((fail + 1)); echo "FAIL $1: expected [$2] got [$3]"; fi; }
CPU="Test CPU 1"
H="cpu	workload	metric	glibc	libniji	level	recorded	cause"
row() { # name instr_new instr_glibc pf_new pf_glibc [pf_new_min..max pf_glibc_min..max]
    printf '== %s   (new aaaaaaaaa, base bbbbbbbbb)\n' "$1"
    printf '%-18s %16s %16s %16s %9s %9s\n' counter new base glibc new/glibc new/base
    printf '%-18s %16s %16s %16s %9s %9s\n' instructions:u "$2" "$2" "$3" - -
    printf '%-18s %16s %16s %16s %9s %9s\n' instructions:k 1 1 1 - -
    printf '%-18s %16s %16s %16s %9s %9s\n' page-faults "$4" "$4" "$5" - -
    [ $# -ge 7 ] && printf 'faults min..max    %16s %16s %16s   (table shows the median)\n' "$6" "$6" "$7"
    printf 'output             new vs glibc: SAME   new vs base: SAME\n'
}
verdict() { python3 "$V" check "$@" --cpu "$CPU" > "$tmp/out" 2>&1; echo $?; }
printf '%s\n%s\n' "$H" "$CPU	printf-loop	instr	1000	3646	3.6460	fixture	dynamic start-up" > "$tmp/holds.tsv"

# 1. the 1.10x bound per row
row seq 1100 1000 110 100 > "$tmp/a.log"
check "1.10x passes" 0 "$(verdict "$tmp/a.log" --holds "$tmp/holds.tsv")"
row seq 1101 1000 100 100 > "$tmp/b.log"
check "instructions over 1.10x fail" 1 "$(verdict "$tmp/b.log" --holds "$tmp/holds.tsv")"
check "the failure names the row" 1 "$(grep -c 'FAIL seq instr: 1101 is 1.1010 x glibc, over 1.10' "$tmp/out")"
row seq 1000 1000 111 100 > "$tmp/c.log"
check "page faults over 1.10x fail" 1 "$(verdict "$tmp/c.log" --holds "$tmp/holds.tsv")"

# 2. a held row at its level; one instruction more fails
row printf-loop 3646 1000 100 100 > "$tmp/d.log"
check "held row at its level passes" 0 "$(verdict "$tmp/d.log" --holds "$tmp/holds.tsv")"
check "held row reported" 1 "$(grep -c '^wl_verdict: HELD printf-loop instr' "$tmp/out")"
row printf-loop 3647 1000 100 100 > "$tmp/e.log"
check "held row one instruction over its level fails" 1 "$(verdict "$tmp/e.log" --holds "$tmp/holds.tsv")"
check "the failure names the held level" 1 "$(grep -c 'over its held level 3.6460 (dynamic start-up' "$tmp/out")"
row printf-loop 3600 1000 100 100 > "$tmp/e2.log"
check "held row under its level passes" 0 "$(verdict "$tmp/e2.log" --holds "$tmp/holds.tsv")"
check "and may tighten" 1 "$(grep -c '^wl_verdict: MAY-TIGHTEN printf-loop instr' "$tmp/out")"

# 3. a held row at or below 1.10x drops its hold
cp "$tmp/holds.tsv" "$tmp/holds2.tsv"
row printf-loop 1100 1000 100 100 > "$tmp/f.log"
check "released row passes" 0 "$(verdict "$tmp/f.log" --holds "$tmp/holds2.tsv" --update)"
check "release reported" 1 "$(grep -c '^wl_verdict: HOLD-RELEASE printf-loop instr' "$tmp/out")"
check "--update removed the hold" 0 "$(grep -c printf-loop "$tmp/holds2.tsv")"
check "--update kept the header" 1 "$(grep -c '^cpu	workload' "$tmp/holds2.tsv")"
row printf-loop 1101 1000 100 100 > "$tmp/g.log"
check "after release the 1.10x bound binds" 1 "$(verdict "$tmp/g.log" --holds "$tmp/holds2.tsv")"
# 3b. a held page-fault row drops its hold only when its worst run is in bound too (median alone would flake back)
printf '%s\n%s\n' "$H" "$CPU	sed-s	pf	251	304	1.2112	fixture	dynamic start-up" > "$tmp/holds3.tsv"
row sed-s 1000 1000 108 100 100..120 100..104 > "$tmp/f2.log"
check "held pf row, median in bound, worst run over: passes" 0 "$(verdict "$tmp/f2.log" --holds "$tmp/holds3.tsv" --update)"
check "held pf row with a worst run over keeps its hold" 1 "$(grep -c '^wl_verdict: HELD sed-s pf: median 1.0800 <= 1.10 but its worst run 1.2000 is over' "$tmp/out")"
check "and --update leaves it in the file" 1 "$(grep -c sed-s "$tmp/holds3.tsv")"
row sed-s 1000 1000 108 100 100..110 100..104 > "$tmp/f3.log"
check "held pf row with every run in bound: released" 0 "$(verdict "$tmp/f3.log" --holds "$tmp/holds3.tsv" --update)"
check "pf release reported" 1 "$(grep -c '^wl_verdict: HOLD-RELEASE sed-s pf' "$tmp/out")"
check "and --update removed it" 0 "$(grep -c sed-s "$tmp/holds3.tsv")"

# 4. geomean of instructions:u: binding on a run of every workload, advisory otherwise
{ row w1 1050 1000 100 100; row w2 1000 1000 100 100; } > "$tmp/h.log"
check "geomean over 1.00 on a full run fails" 1 "$(verdict "$tmp/h.log" --all-count 2)"
check "the failure names the geomean" 1 "$(grep -c 'FAIL geomean instructions:u 1.0247 over 1.00' "$tmp/out")"
check "geomean on a partial run is advisory" 0 "$(verdict "$tmp/h.log" --all-count 30)"
{ row w1 1050 1000 100 100; row w2 900 1000 100 100; } > "$tmp/i.log"
check "geomean at or below 1.00 passes" 0 "$(verdict "$tmp/i.log" --all-count 2)"

# 5. held only on another CPU class: reported, not failed
check "unrecorded CPU class: held row reported (rc)" 0 "$(python3 "$V" check "$tmp/e.log" --holds "$tmp/holds.tsv" --cpu "Other CPU" > "$tmp/out" 2>&1; echo $?)"
check "unrecorded CPU class named" 1 "$(grep -c '^wl_verdict: HELD-UNRECORDED printf-loop instr' "$tmp/out")"
check "static link: no holds" 1 "$(verdict "$tmp/d.log" --holds "$tmp/holds.tsv" --link static)"

# 6. the holds ratchet
hc() { python3 "$V" holds "$@" > "$tmp/out" 2>&1; echo $?; }
check "fixture holds consistent" 0 "$(hc "$tmp/holds.tsv")"
printf '%s\n%s\n' "$H" "$CPU	printf-loop	instr	1000	3646	3.6470	fixture	dynamic start-up" > "$tmp/loose.tsv"
check "level above its measurement refused" 1 "$(hc "$tmp/loose.tsv")"
printf '%s\n%s\n' "$H" "$CPU	printf-loop	instr	1000	3700	3.7000	fixture	dynamic start-up" > "$tmp/raised.tsv"
check "raised level refused against the base" 1 "$(hc "$tmp/raised.tsv" --base "$tmp/holds.tsv")"
check "the refusal says raised" 1 "$(grep -c 'raised from the base' "$tmp/out")"
printf '%s\n%s\n%s\n' "$H" "$CPU	printf-loop	instr	1000	3646	3.6460	fixture	dynamic start-up" "$CPU	seq	pf	100	130	1.3000	fixture	dynamic start-up" > "$tmp/added.tsv"
check "new hold on a recorded CPU class refused" 1 "$(hc "$tmp/added.tsv" --base "$tmp/holds.tsv")"
printf '%s\n%s\n%s\n' "$H" "$CPU	printf-loop	instr	1000	3646	3.6460	fixture	dynamic start-up" "Other CPU	seq	pf	100	130	1.3000	fixture	dynamic start-up" > "$tmp/newcpu.tsv"
check "first levels of a new CPU class accepted" 0 "$(hc "$tmp/newcpu.tsv" --base "$tmp/holds.tsv")"
# D-X.155 (WL-LEVELS-RERECORD; no spec clause, lab tooling): a level is recomputed from the counts it records, so a
# level edited without a re-count is refused whichever way it moves; a raised level is accepted only as a correction of
# an overstated glibc count (named decision, recorded again, libniji did not rise, so glibc fell).
printf '%s\n%s\n' "$H" "$CPU	printf-loop	instr	1000	3646	3.6000	fixture	dynamic start-up" > "$tmp/edited.tsv"
check "level lowered without a re-count refused" 1 "$(hc "$tmp/edited.tsv")"
check "the refusal names the recorded counts" 1 "$(grep -c 'not the ratio of the counts it records (3646 / 1000 = 3.6460' "$tmp/out")"
printf '%s\n%s\n' "$H" "$CPU	printf-loop	instr	1000	3600	3.6000	fixture2	dynamic start-up" > "$tmp/tight.tsv"
check "level lowered with its new counts accepted" 0 "$(hc "$tmp/tight.tsv" --base "$tmp/holds.tsv")"
cor() { printf '%s\n%s\n' "$H" "$CPU	printf-loop	instr	$1	$2	$3	$4	$5" > "$tmp/cor.tsv"; hc "$tmp/cor.tsv" --base "$tmp/holds.tsv"; }
check "correction of an overstated glibc count accepted" 0 "$(cor 900 3600 4.0000 fixture2 'dynamic start-up; correction D-X.155')"
check "correction without a named decision refused" 1 "$(cor 900 3600 4.0000 fixture2 'dynamic start-up')"
check "correction not recorded again refused" 1 "$(cor 900 3600 4.0000 fixture 'dynamic start-up; correction D-X.155')"
check "correction with libniji's count risen refused" 1 "$(cor 900 3700 4.1112 fixture2 'dynamic start-up; correction D-X.155')"
check "the refusal says libniji got worse" 1 "$(grep -c 'libniji got worse' "$tmp/out")"
printf '%s\n%s\n' "$H" "$CPU	seq	pf	100	105	1.0500	fixture	dynamic start-up" > "$tmp/low.tsv"
check "a hold at or below 1.10 refused" 1 "$(hc "$tmp/low.tsv")"
printf '%s\n%s\n' "$H" "$CPU	seq	pf	100	130	1.3000	fixture	 " > "$tmp/nocause.tsv"
check "a hold without a cause refused" 1 "$(hc "$tmp/nocause.tsv")"
# The repository's holds against the merge base's copy: git (merge-base with origin/next) or a role directory
# (.synced merge_base, its source primed under ~/wl/src). No base copy (the file is new there): consistency only.
base=
root=$(cd "$here/.." && pwd)
if git -C "$root" rev-parse --git-dir > /dev/null 2>&1; then
    mb=$(git -C "$root" merge-base HEAD origin/next 2>/dev/null) && git -C "$root" show "$mb:validation/wl_holds.tsv" > "$tmp/base.tsv" 2>/dev/null && base=$tmp/base.tsv
elif [ -f "$root/.synced" ]; then
    mb=$(sed -n 's/^merge_base=//p' "$root/.synced")
    [ -n "$mb" ] && [ -f "$HOME/wl/src/$mb/validation/wl_holds.tsv" ] && base=$HOME/wl/src/$mb/validation/wl_holds.tsv
fi
if [ -n "$base" ]; then
    check "wl_holds.tsv against the merge base ($base)" 0 "$(hc "$here/wl_holds.tsv" --base "$base")"
else
    echo "wl_verdict_test: no merge-base copy of wl_holds.tsv (new there, or the base source is not primed): consistency only"
    check "wl_holds.tsv consistent" 0 "$(hc "$here/wl_holds.tsv")"
fi
[ "$fail" = 0 ] || cat "$tmp/out"

# 7. wl_count.sh gates on the verdict of the dynamic release kit
check "wl_count runs the verdict with the holds" 1 "$(grep -c 'wl_verdict.py" check ~/logs/$label.log --holds "$here/wl_holds.tsv" --all-count "${#WL_DEFS\[@\]}" || rc=1' "$here/wl_count.sh")"

echo "wl_verdict_test: $pass passed, $fail failed"
[ $fail = 0 ]
