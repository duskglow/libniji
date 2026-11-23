#!/bin/sh
# validation/smoke/undef-compare.sh WORK PROGRAM...: plan/headers.md P7, the symbol half. For each program smoke.sh built
# (WORK/build/<program>/{glibc,niji}), every ELF program or shared object that exists on both sides is compared by
# `nm -D --undefined-only`: the names with and without their version tags. Prints one line per object and
# `undef-compare: N objects, D differ`; exit 1 when any differs.
set -u
work=${1:?usage: undef-compare.sh WORK PROGRAM...}; shift
n=0; d=0
for p in "$@"; do
    p=$(printf '%s' "$p" | tr '-' '_')
    g=$work/build/$p/glibc; l=$work/build/$p/niji
    [ -d "$g" ] && [ -d "$l" ] || { echo "undef-compare: $p: no build trees"; continue; }
    (cd "$g" && find . -type f | sort) | while read -r f; do
        [ -f "$l/$f" ] || continue
        [ "$(head -c4 "$g/$f" | od -An -c | tr -d ' ')" = '177ELF' ] || continue
        nm -D --undefined-only "$g/$f" > "$work/undef.g" 2>/dev/null || continue
        nm -D --undefined-only "$l/$f" > "$work/undef.n" 2>/dev/null || continue
        sed 's/@.*//' "$work/undef.g" | sort -u > "$work/undef.g0"; sed 's/@.*//' "$work/undef.n" | sort -u > "$work/undef.n0"
        if cmp -s "$work/undef.g0" "$work/undef.n0"; then
            if cmp -s "$work/undef.g" "$work/undef.n"; then echo "same $p $f"; else echo "same-names-other-versions $p $f"; fi
        else
            echo "DIFF $p $f: only glibc: $(comm -23 "$work/undef.g0" "$work/undef.n0" | tr -s ' \n' ' ') only libniji: $(comm -13 "$work/undef.g0" "$work/undef.n0" | tr -s ' \n' ' ')"
        fi
    done
done > "$work/undef-compare.txt"
cat "$work/undef-compare.txt"
n=$(wc -l < "$work/undef-compare.txt"); d=$(grep -c '^DIFF' "$work/undef-compare.txt")
echo "undef-compare: $n objects, $d differ"
[ "$d" -eq 0 ]
