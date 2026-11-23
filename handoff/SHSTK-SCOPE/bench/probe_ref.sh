#!/bin/bash
# SHSTK-SCOPE: black-box probes of the reference C library's shadow-stack behaviour (N100 host).
cd ~/work/shstk-scope/run/g || exit 2
t() { echo "-- GLIBC_TUNABLES=[$1] $2"; GLIBC_TUNABLES=$1 timeout 10 ${@:2} 2>&1; echo "   rc=$?"; }
for T in "" glibc.cpu.x86_shstk=on glibc.cpu.x86_shstk=permissive glibc.cpu.x86_shstk=off glibc.cpu.hwcaps=SHSTK; do
	t "$T" ./shb status
	t "$T" ./shb_static status
	t "$T" ./shb_unm status
done
t glibc.cpu.x86_shstk=on ./shb dlopen ./libunm.so
t glibc.cpu.x86_shstk=permissive ./shb dlopen ./libunm.so
t glibc.cpu.x86_shstk=on ./shb dlopen ./libmk.so
t glibc.cpu.x86_shstk=on ./shb ssinfo
t glibc.cpu.x86_shstk=on ./shb_static ssinfo
for m in "longjmp 10" "deep 10" "swap 10" "signal 10" "thread 10" "vfork 3"; do
	t glibc.cpu.x86_shstk=on ./shb $m
done
t glibc.cpu.x86_shstk=on ./cxx 10
t glibc.cpu.x86_shstk=on ./main3
echo "-- ulimit -s 1024, on, ssinfo"; (ulimit -s 1024; GLIBC_TUNABLES=glibc.cpu.x86_shstk=on ./shb ssinfo)
