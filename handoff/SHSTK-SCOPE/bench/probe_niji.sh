#!/bin/bash
# SHSTK-SCOPE: does the libniji prototype run with the shadow stack enabled (NJSHSTK= set)? N100 host.
cd ~/work/shstk-scope/run/n || exit 2
t() { echo "-- [$1] ${*:2}"; timeout 10 env LD_LIBRARY_PATH=$HOME/work/shstk-scope/niji $1 "${@:2}" 2>&1; echo "   rc=$?"; }
for E in "X=1" "NJSHSTK=1"; do
	t $E ./shb status
	t $E ./shb_static status
	t $E ./shb ssinfo
	t $E ./shb_static ssinfo
	for m in "longjmp 10" "deep 10" "swap 10" "signal 10" "thread 10" "vfork 3" "mapss 3"; do
		t $E ./shb $m
		t $E ./shb_static $m
	done
	t $E ./cxx 10
	t $E ./main3
	t $E ./empty
	t $E ./empty_static
	t $E ./shb dlopen ./libunm.so
done
