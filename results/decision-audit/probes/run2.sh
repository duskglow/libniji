#!/bin/sh
# DECISION-EVIDENCE-AUDIT probe 2: where the hot loops of create_inner and finish are (perf annotate on probe 1's
# profile), the PT_TLS of both libraries, and a one-allocation program's memory (heap first use).
set -u
here=$(cd "$(dirname "$0")" && pwd)
P=${P:?P names the libniji dynamic products directory}
cd "$here"
for s in create_inner finish; do
    sym=$(perf report -i perf.niji --stdio --sort symbol 2>/dev/null | grep -o "_R[A-Za-z0-9_]*thread[0-9]*${#s}${s}\b" | head -1)
    echo "== annotate $s ($sym): lines over 3%"
    perf annotate -i perf.niji --stdio -s "$sym" 2>/dev/null | awk -F: '$1+0 >= 3.0' | head -14
done
echo "== PT_TLS"
for l in /lib/x86_64-linux-gnu/libc.so.6 "$P/libc.so.6"; do
    printf '%s: ' "$l"; readelf -lW "$l" | awk '$1=="TLS" {print "filesz", $5, "memsz", $6, "align", $7}'
done
gcc -O2 one_malloc.c -o om_ref || exit 1
WL_DYN_PRODUCTS=$P sh ~/w/sh1/validation/wl_dyncc.sh -O2 one_malloc.c -o om_niji || exit 1
for b in om_ref om_niji; do
    for a in 0 1; do
        printf '%s malloc=%s: ' "$b" "$a"
        timeout 20 perf stat -x, -e instructions:u,page-faults ./$b $a 2>stat.tmp | tr '\n' ' '
        awk -F, '{printf "%s=%s ", $3, $1} END {print ""}' stat.tmp
    done
done
echo "== done"
