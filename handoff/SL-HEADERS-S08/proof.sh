#!/bin/bash
# slice 8 proofs in the role tree on mei: headers-gen, then P1/P5, P2, P3. Full outputs to ~/work/SL-HEADERS-S08-3/*.out
cd ~/w/s2 || exit 2
. ~/.cargo/env
O=~/work/SL-HEADERS-S08-3
H=signal.h,sys/signal.h,setjmp.h,ucontext.h,sys/ucontext.h,pthread.h,semaphore.h,threads.h,sched.h,sys/single_threaded.h,sys/rseq.h
D=/mnt/zram/claude/SL-HEADERS-S08-3/proof-$1
mkdir -p $D/vis $D/call $O
if [ "$GEN" = 1 ]; then
  echo "== headers-gen"
  cargo xtask headers-gen 2>&1 | tail -${GEN_TAIL:-15}
fi
echo "== P1/P5 header_check"
python3 spec/tools/header_check.py > $O/hc.out 2>&1; echo "rc=$?"; tail -1 $O/hc.out
echo "== P2 header_probe"
python3 spec/tools/header_probe.py --include libc-include --headers $H --out $D/vis --jobs 16 2>&1 | tail -3
python3 spec/tools/probe_compare.py visibility --got $D/vis --headers $H > $O/p2.out 2>&1; echo "rc=$?"; tail -2 $O/p2.out
echo "== P3 callsite_probe"
python3 spec/tools/callsite_probe.py --include libc-include --headers $H --out $D/call --jobs 16 2>&1 | tail -3
python3 spec/tools/probe_compare.py callsites --got $D/call --headers $H > $O/p3.out 2>&1; echo "rc=$?"; tail -2 $O/p3.out
echo "== done"
