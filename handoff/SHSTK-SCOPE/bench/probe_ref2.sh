#!/bin/bash
# SHSTK-SCOPE: reference probes with the shadow stack requested (glibc.cpu.hwcaps=SHSTK).
cd ~/work/shstk-scope/run/g || exit 2
H=glibc.cpu.hwcaps=SHSTK
t() { echo "-- GLIBC_TUNABLES=[$1] ${*:2}"; GLIBC_TUNABLES=$1 timeout 10 "${@:2}" 2>&1; echo "   rc=$?"; }
t "$H" ./shb dlopen ./libunm.so
t "$H:glibc.cpu.x86_shstk=on" ./shb dlopen ./libunm.so
t "$H:glibc.cpu.x86_shstk=permissive" ./shb dlopen ./libunm.so
t "$H:glibc.cpu.x86_shstk=off" ./shb status
t "$H" ./shb dlopen ./libmk.so
t "$H" ./shb ssinfo
t "$H" ./shb_static ssinfo
for m in "longjmp 10" "deep 10" "swap 10" "signal 10" "thread 10" "vfork 3" "mapss 3"; do
	t "$H" ./shb $m
done
t "$H" ./cxx 10
t "$H" ./main3
cp libunm.so libX.so 2>/dev/null
echo "-- ulimit -s 1024"; (ulimit -s 1024; GLIBC_TUNABLES=$H ./shb ssinfo)
echo "-- ulimit -s unlimited"; (ulimit -s unlimited; GLIBC_TUNABLES=$H ./shb ssinfo)
echo "-- LD_PRELOAD unmarked"; GLIBC_TUNABLES=$H LD_PRELOAD=./libunm.so ./shb status
