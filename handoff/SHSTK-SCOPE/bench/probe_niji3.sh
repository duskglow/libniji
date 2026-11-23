#!/bin/bash
# SHSTK-SCOPE: thread exit / cancellation with the shadow stack, and the negative control
# (shb_static_orig: libniji's setjmp/vfork/context code without SSP handling).
D=$HOME/work/shstk-scope/niji
t() { echo "-- [$1] ${*:2}"; timeout 10 env LD_LIBRARY_PATH=$D $1 "${@:2}" 2>&1 | head -3; echo "   rc=${PIPESTATUS[0]}"; }
cd ~/work/shstk-scope/run/n || exit 2
for E in NJSHSTX=1 NJSHSTK=1; do
	for m in "pexit 10" "cancel 10"; do t $E ./shb $m; t $E ./shb_exc $m; t $E ./shb_static $m; done
	for m in "longjmp 3" "deep 3" "swap 3" "vfork 3" "pexit 3" "thread 3" "signal 3"; do t $E ./shb_static_orig $m; done
done
cd ~/work/shstk-scope/run/g || exit 2
for m in "pexit 10" "cancel 10"; do
	echo "-- g on $m"; GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK timeout 10 ./shb $m; echo "   rc=$?"
	echo "-- g on exc $m"; GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK timeout 10 ./shb_exc $m; echo "   rc=$?"
done
