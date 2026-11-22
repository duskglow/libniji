#!/bin/bash
# SL-HEADERS-S07-TAIL proofs in the role tree: P1/P5 (header_check), P2, P3, P10 for the five headers and the six
# headers whose output this branch changes (slice 12's guards, slice 15's sys/ioctl.h includes).
# Usage: proof.sh <role dir> <out dir> <ram dir>. Full outputs in <out dir>/*.out.
cd "$1" || exit 2
. ~/.cargo/env
O=$2
D=$3
H=sys/wait.h,wait.h,sys/param.h,spawn.h,sys/ioctl.h,arpa/nameser.h,netdb.h,resolv.h,sys/raw.h,sys/mtio.h,sys/mount.h
mkdir -p "$D/vis" "$D/call" "$D/p10" "$O"
echo "== P1/P5 header_check"
python3 -I spec/tools/header_check.py > "$O/hc.out" 2>&1; echo "rc=$?"; tail -1 "$O/hc.out"
echo "== P10 header_p10"
python3 -I spec/tools/header_p10.py --work "$D/p10" > "$O/p10.out" 2>&1; echo "rc=$?"; tail -1 "$O/p10.out"
echo "== P2 header_probe"
python3 -I spec/tools/header_probe.py --include libc-include --headers $H --out "$D/vis" --jobs 16 2>&1 | tail -3
cat handoff/SL-HEADERS-S07-TAIL/p2-accept.txt handoff/SL-HEADERS-S15/p2-accept.txt > "$O/p2-accept.txt"
python3 -I spec/tools/probe_compare.py visibility --got "$D/vis" --headers $H --accept "$O/p2-accept.txt" > "$O/p2.out" 2>&1; echo "rc=$?"; tail -2 "$O/p2.out"
echo "== P3 callsite_probe"
python3 -I spec/tools/callsite_probe.py --include libc-include --headers $H --out "$D/call" --jobs 16 2>&1 | tail -3
python3 -I spec/tools/probe_compare.py callsites --got "$D/call" --headers $H > "$O/p3.out" 2>&1; echo "rc=$?"; tail -2 "$O/p3.out"
echo "== done"
