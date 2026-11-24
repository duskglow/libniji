#!/bin/sh
# SL-HEADERS-S07-TAIL P6 and P7 in the role tree (plan/headers.md): conformance programs and the smoke programs built
# against libniji's staged headers alone, then the undefined-symbol comparison with the reference-header build.
# Usage: p67.sh <role dir> <out dir> <ram work dir>. Full outputs in <out dir>/p6.out, p7.out, undef.out.
cd "$1" || exit 2
. ~/.cargo/env
O=$2
W=$3
mkdir -p "$O" "$W" || exit 2
echo "== products (NIJI_FULL_PROFILE=1 cargo xtask so)"
NIJI_FULL_PROFILE=1 cargo xtask so > "$O/so.out" 2>&1; rc=$?; echo "rc=$rc"; tail -2 "$O/so.out"
[ $rc -eq 0 ] || exit 1
echo "== P6 conformance --niji-headers"
sh conformance/run.sh --niji-headers > "$O/p6.out" 2>&1; echo "rc=$?"; tail -3 "$O/p6.out"
echo "== P7 smoke --niji-headers zlib xz_utils sqlite3"
sh validation/smoke/smoke.sh --work "$W" --niji-headers zlib xz_utils sqlite3 > "$O/p7.out" 2>&1; echo "rc=$?"
cat "$W/results.tsv" 2>/dev/null | head -20
echo "== P7 undef-compare"
sh validation/smoke/undef-compare.sh "$W" zlib xz_utils sqlite3 > "$O/undef.out" 2>&1; echo "rc=$?"; tail -2 "$O/undef.out"
echo "== done"
