#!/bin/bash
# STARTUP-DATA-WINDOWS (D-X.50): write $NIJI_HOT_LISTS/startup-data.keys from the products of <tree>
# (the lists of one product build, LT-5: never committed).
# usage: NIJI_HOT_LISTS=<dir> tools/hottext/startup_data.sh <tree with built products>
#   (STARTUP_DATA_PRODUCTS=<dir>: read the products from <dir>, a saved copy, instead of <tree>/target/niji)
#   Links crates/core/tests/oracle/page_faults.c as crates/core/tests/hot_text_products.rs links it and
#   writes the data objects start-up may touch, declared by rule, after checking that `page_faults true`
#   touches no other (writable ones by access, in a second link with every writable data section on its own
#   64 KiB boundary; tools/hottext/startup_data.py, LAYOUT-CANONICAL-SET), for the profile
#   in the products' .profile; this build's objects only. regen.sh runs it after the
#   start-up text trace (it needs startup-text.keys of the same profile). Needs perf (page-faults events).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
T=$(cd "$1" && pwd); shift
L=${NIJI_HOT_LISTS:-}
if [ -z "$L" ] || [ ! -d "$L" ]; then
    echo "startup_data: NIJI_HOT_LISTS must name the lists directory of this build (cargo xtask so sets it; no list is committed, LT-5)" >&2
    exit 1
fi
P=${STARTUP_DATA_PRODUCTS:-$T/target/niji}
profile=$(cat "$P/.profile" 2>/dev/null || true)
case $profile in fast|release) ;; *) echo "startup_data: $P/.profile: want fast or release, got '$profile'" >&2; exit 1;; esac
work=$(mktemp -d "${TMPDIR:-/tmp}/startupdata.XXXXXX")
trap 'rm -rf "$work"' EXIT
# The probe compiled alone first: its own functions' operands are declared too (startup_data.py, clause 2).
cc -std=gnu17 -O1 -fno-builtin -c "$T/crates/core/tests/oracle/page_faults.c" -o "$work/page_faults.o"
cc -std=gnu17 -O1 -static -nostdlib -fno-builtin "$P/niji-crt1.o" "$work/page_faults.o" \
    "$P/libc.a" -lgcc -o "$work/page_faults"
# The isolated link (startup_data.py, source 2): the default script with every writable data input
# section on a 64 KiB boundary of its own, so each object start-up touches faults at its own address.
ld --verbose | sed -n '/^=====/,/^=====/p' | sed '1d;$d' \
    | sed -E 's/^([[:space:]]*\.(data|data1|bss|lbss)[[:space:]]+:)/\1 SUBALIGN(65536)/' > "$work/isolated.ld"
[ "$(grep -c 'SUBALIGN(65536)' "$work/isolated.ld")" -ge 3 ] || { echo "startup_data: ld --verbose: .data/.bss output sections not found" >&2; exit 1; }
cc -std=gnu17 -O1 -static -nostdlib -fno-builtin -Wl,-T,"$work/isolated.ld" "$P/niji-crt1.o" \
    "$work/page_faults.o" "$P/libc.a" -lgcc -o "$work/page_faults_isolated"
python3 -B "$here/startup_data.py" "$work/page_faults" "$work/page_faults_isolated" "$L/startup-text.keys" "$profile" \
    "$L/startup-data.keys" "$work/page_faults.o" "$@"
