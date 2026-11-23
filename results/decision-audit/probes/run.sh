#!/bin/sh
# DECISION-EVIDENCE-AUDIT probe runner (a build host; the role tree at ~/w/sh1). Builds thread_cost.c against the reference and against the
# primed libniji-dynamic products of the branch base, then counts instructions and faults with perf stat.
set -u
here=$(cd "$(dirname "$0")" && pwd)
P=${P:?P names the libniji dynamic products directory}
cd "$here"
gcc -O2 -pthread thread_cost.c -o tc_ref || exit 1
WL_DYN_PRODUCTS=$P sh ~/w/sh1/validation/wl_dyncc.sh -O2 thread_cost.c -o tc_niji || exit 1
echo "== interpreter"; readelf -l tc_niji | grep -i interpreter
for b in tc_ref tc_niji; do
    echo "== $b maps"; timeout 20 ./$b maps
    for n in 0 1000 2000; do
        for rep in 1 2 3; do
            timeout 120 perf stat -x, -e instructions:u,instructions:k,page-faults ./$b $n 2>stat.tmp
            rc=$?
            printf '%s n=%s rep=%s rc=%s ' "$b" "$n" "$rep" "$rc"
            awk -F, '{printf "%s=%s ", $3, $1} END {print ""}' stat.tmp
        done
    done
done
echo "== niji profile (user instructions, 2000 pairs)"
timeout 120 perf record -q -e instructions:u -c 2000 -o perf.niji ./tc_niji 2000 >/dev/null 2>&1
perf report -i perf.niji --stdio --sort dso,symbol 2>/dev/null | grep -v '^#' | grep -v '^$' | head -30
echo "== ref profile (user instructions, 2000 pairs)"
timeout 120 perf record -q -e instructions:u -c 2000 -o perf.ref ./tc_ref 2000 >/dev/null 2>&1
perf report -i perf.ref --stdio --sort dso,symbol 2>/dev/null | grep -v '^#' | grep -v '^$' | head -15
echo "== done"
