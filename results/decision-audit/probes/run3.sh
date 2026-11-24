#!/bin/sh
# DECISION-EVIDENCE-AUDIT probe 3: does a static libniji process fault the heap reserve's page (RESERVE_MEM, the
# `.lbss` page D-X.138's start-up data budget charges)? Records every user page fault's data address and checks it
# against the symbol's range. Also lists the faulting sections (by the program's section headers).
set -u
here=$(cd "$(dirname "$0")" && pwd)
PS=${PS:?PS names the libniji static release products directory}
cd "$here"
NIJI_PRODUCTS=$PS sh ~/w/sh1/scripts/niji-cc -O2 one_malloc.c -o om_static || exit 1
nm om_static | grep -i "RESERVE_MEM" | head -3
r=$(nm -S om_static | awk '/RESERVE_MEM/ {print $1, $2; exit}')
echo "reserve: $r"
for a in 0 1; do
    timeout 20 perf record -q -e page-faults:u -c 1 -d -o pf.$a ./om_static $a >/dev/null 2>&1
    echo "== malloc=$a faults: $(perf script -i pf.$a -F addr 2>/dev/null | wc -l)"
    perf script -i pf.$a -F addr 2>/dev/null | awk -v r="$r" '
        BEGIN { split(r, x, " "); lo = strtonum("0x" x[1]); hi = lo + strtonum("0x" x[2]) }
        { a = strtonum("0x" $1); if (a >= lo && a < hi) n++ }
        END { printf "faults inside RESERVE_MEM [%x, %x): %d\n", lo, hi, n + 0 }'
    readelf -SW om_static | awk '$2 ~ /^\./ && $3 != "NULL" {print $2, $4, $6}' > sections.txt
    perf script -i pf.$a -F addr 2>/dev/null | awk '
        NR == FNR { s[NR] = $1; lo[NR] = strtonum("0x" $2); sz[NR] = strtonum("0x" $3); n = NR; next }
        { a = strtonum("0x" $1); hit = "other";
          for (i = 1; i <= n; i++) if (lo[i] > 0 && a >= lo[i] && a < lo[i] + sz[i]) { hit = s[i]; break }
          c[hit]++ }
        END { for (k in c) printf "  %s %d\n", k, c[k] }' sections.txt - | sort
done
echo "== done"
