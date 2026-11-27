#!/bin/bash
# Run gcc-14's fixinc.sh over each header tree named on the command line (name=dir pairs).
# Output: per tree, the fixinc.sh log on stdout between "=== TREE <name>" markers,
# and the fixed copies under $W/out/<name>/fixed (stock's copies are never read).
set -uo pipefail
W=~/work/gcc-fixincludes-probe
S=$W/src/gcc-14.2.0
B=$W/build
for pair in "$@"; do
  name=${pair%%=*}; dir=${pair#*=}
  out=$W/out-${OUTTAG:-v2}/$name
  if [ -e "$out" ]; then echo "=== SKIP $name (exists)"; continue; fi
  mkdir -p "$out/fixed"
  echo "=== TREE $name $dir"
  ( cd "$B/fixincludes" && \
    TARGET_MACHINE=x86_64-pc-linux-gnu srcdir="$S/gcc" SHELL=/bin/bash MACRO_LIST="$W/macro_list" \
    VERBOSE=${VERBOSE:-2} timeout 600 /bin/bash ./fixinc.sh "$out/fixed" "$dir" )
  echo "=== END $name rc=$?"
done
