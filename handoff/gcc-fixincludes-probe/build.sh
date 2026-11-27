#!/bin/bash
# Build only gcc-14's fixincludes driver (fixincl, fixinc.sh) and the libiberty it links.
# Source: Debian trixie gcc-14 14.2.0-19 (orig tarball, checksums match the .dsc).
set -euo pipefail
W=~/work/gcc-fixincludes-probe
S=$W/src/gcc-14.2.0
B=$W/build
TRIPLE=x86_64-pc-linux-gnu
mkdir -p "$B/libiberty" "$B/fixincludes"
cd "$B/libiberty"
"$S/libiberty/configure" --build=$TRIPLE --host=$TRIPLE --disable-shared --disable-multilib
make -j8
cd "$B/fixincludes"
"$S/fixincludes/configure" --build=$TRIPLE --host=$TRIPLE --target=$TRIPLE
make -j8
ls -la "$B/fixincludes/fixincl" "$B/fixincludes/fixinc.sh"
# MACRO_LIST as gcc/Makefile.in's s-macro_list rule builds it (here from the host gcc, not the new xgcc).
echo | gcc -nostdinc -E -dM - | \
  sed -n -e 's/^#define \([^_][a-zA-Z0-9_]*\).*/\1/p' \
         -e 's/^#define \(_[^_A-Z][a-zA-Z0-9_]*\).*/\1/p' | sort -u > "$W/macro_list"
echo "macro_list: $(wc -l < "$W/macro_list") names: $(tr '\n' ' ' < "$W/macro_list")"
