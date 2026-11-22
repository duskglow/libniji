#!/bin/bash
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# Self-test of the kit table's output column (validation/wl_kits.py, WL-NIGHTLY-1121; no spec clause: measurement
# tooling). Fixture raw rows, no build, under a second:
#   1. equal md5s print SAME;
#   2. a differing output whose fingerprint wl_known_diffs.tsv allows for the workload prints KNOWN(sk,dk) and is not
#      counted as unexplained;
#   3. the same workload with a changed fingerprint prints DIFF and is counted as unexplained;
#   4. a differing output without a fingerprint row (an older raw file) prints DIFF(s!,d!);
#   5. with no --known the repository's wl_known_diffs.tsv is read (its ls-l-enUS row allows its fingerprint).
set -u
here=$(cd "$(dirname "$0")" && pwd)
K=$here/wl_kits.py
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
pass=0; fail=0
check() { if [ "$2" = "$3" ]; then pass=$((pass + 1)); else fail=$((fail + 1)); echo "FAIL $1: expected [$2] got [$3]"; fi; }
T=$'\t'
printf 'workload\tfingerprint\treason\tdecision_ref\n# a comment\nwa\t4:aaaaaaaaaaaa\ttest row\tD-test\n' > "$tmp/known.tsv"
raw=$tmp/raw.tsv
: > "$raw"
for w in same wa wb old; do
    for c in glibc static dynamic; do
        printf 'run\t%s\t%s\t1\t100\t10\t50\t1.0\t1000\n' "$w" "$c" >> "$raw"
    done
done
for c in glibc static dynamic; do printf 'start\tp\t%s\t1\t100\t10\t50\n' "$c" >> "$raw"; done
printf 'md5\tsame\tglibc\tm1\nmd5\tsame\tstatic\tm1\nmd5\tsame\tdynamic\tm1\n' >> "$raw"
printf 'md5\twa\tglibc\tm1\nmd5\twa\tstatic\tm2\nmd5\twa\tdynamic\tm2\ndfp\twa\tstatic\t4:aaaaaaaaaaaa\ndfp\twa\tdynamic\t4:aaaaaaaaaaaa\n' >> "$raw"
printf 'md5\twb\tglibc\tm1\nmd5\twb\tstatic\tm2\nmd5\twb\tdynamic\tm1\ndfp\twb\tstatic\t4:aaaaaaaaaaaa\n' >> "$raw"
printf 'md5\told\tglibc\tm1\nmd5\told\tstatic\tm2\nmd5\told\tdynamic\tm3\n' >> "$raw"
out=$(python3 "$K" "$raw" --known "$tmp/known.tsv")
row() { echo "$out" | awk -v w="$1" '$1 == w {print $NF}'; }
check "equal outputs SAME" "SAME" "$(row same)"
check "known fingerprint KNOWN" "KNOWN(sk,dk)" "$(row wa)"
check "fingerprint allowed for another workload only is unexplained" "DIFF(s!,d=)" "$(row wb)"
check "no fingerprint row DIFF" "DIFF(s!,d!)" "$(row old)"
check "unexplained count" "2  wb, old" "$(echo "$out" | grep '^output differs from glibc, unexplained' | sed 's/.*: //')"
check "known count" "1  wa" "$(echo "$out" | grep '^output differs from glibc, known' | sed 's/.*: //')"
# 5. the repository's list: its ls-l-enUS fingerprint is allowed, one character changed is not.
fp=$(awk -F'\t' '$1 == "ls-l-enUS" {print $2}' "$here/wl_known_diffs.tsv")
sed "s/\twa\t/\tls-l-enUS\t/; /^dfp/d" "$raw" > "$tmp/raw2.tsv"
printf 'dfp\tls-l-enUS\tstatic\t%s\ndfp\tls-l-enUS\tdynamic\t%sx\n' "$fp" "$fp" >> "$tmp/raw2.tsv"
out=$(python3 "$K" "$tmp/raw2.tsv")
check "repository list read by default" "DIFF(sk,d!)" "$(row ls-l-enUS)"
[ -n "$fp" ] || { fail=$((fail + 1)); echo "FAIL no ls-l-enUS row in wl_known_diffs.tsv"; }
echo "wl_kits_test: $pass passed, $fail failed"
[ "$fail" = 0 ]
