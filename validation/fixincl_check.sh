#!/bin/bash
# fixincl_check.sh: run gcc's fixincludes over libniji's installed header tree and fail on any header it rewrites.
#
#   validation/fixincl_check.sh <gcc build dir> <gcc source dir> <header tree> [<out dir>]
#
# A gcc build runs fixincludes over the target's C library headers and copies every header one of its fixes rewrites
# into include-fixed/, where it shadows the library's own copy (HEADERS-STAGE2-GAPS-I; Yohane R-H7 on 26.3: the copies
# of stdio.h and wchar.h lost va_list). libniji's headers must draw no rewrite: this runs the fixinc.sh of a configured
# gcc build (its fixincludes/ directory, after `make all-fixincludes`) the way gcc's own build step does
# (TARGET_MACHINE x86_64-pc-linux-gnu, the compiler's macro list, VERBOSE=2), over <header tree> (the system
# flavour's usr/include, or libc-include/ and include/ staged together as `cargo xtask so` stages them), and prints
# each "Fixed:" line. Exit 0: no header rewritten; 1: at least one; 2: usage or setup error. The rewritten copies stay
# in <out dir> (default: a new temporary directory, printed) for a diff against the tree.
set -uo pipefail
if [ $# -lt 3 ] || [ $# -gt 4 ]; then
  sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
fi
build=$1 src=$2 tree=$3
out=${4:-$(mktemp -d -t fixincl_check.XXXXXX)}
[ -x "$build/fixincludes/fixincl" ] && [ -f "$build/fixincludes/fixinc.sh" ] || { echo "no fixincludes/fixincl in $build" >&2; exit 2; }
[ -d "$src/gcc" ] || { echo "no gcc/ in $src" >&2; exit 2; }
[ -d "$tree" ] || { echo "no header tree $tree" >&2; exit 2; }
mkdir -p "$out/fixed" || exit 2
# gcc/Makefile's macro_list: the compiler's predefined macros not in the implementation's name space.
"${CC:-gcc}" -E -dM - </dev/null | sed -n -e 's/^#define \([^_][a-zA-Z0-9_]*\).*/\1/p' \
  -e 's/^#define \(_[^_A-Z][a-zA-Z0-9_]*\).*/\1/p' | sort -u > "$out/macro_list" || exit 2
log=$out/fixinc.log
( cd "$build/fixincludes" && TARGET_MACHINE=x86_64-pc-linux-gnu srcdir="$src/gcc" SHELL=/bin/bash \
    MACRO_LIST="$out/macro_list" VERBOSE=2 timeout 900 /bin/bash ./fixinc.sh "$out/fixed" "$(cd "$tree" && pwd)" ) > "$log" 2>&1
rc=$?
[ $rc -eq 0 ] || { echo "fixinc.sh failed (rc $rc), log $log" >&2; tail -5 "$log" >&2; exit 2; }
fixed=$(grep -c "^Fixed:" "$log")
grep "^Fixed:" "$log"
n=$(find "$tree" -name '*.h' | wc -l)
echo "fixincl_check: $n headers scanned, $fixed rewritten (log $log)"
[ "$fixed" -eq 0 ]
