#!/bin/sh
# run.sh - DL-DYNAMIC-RELOC black-box probe of the loaded dynamic array (spec/behaviour/dynlink.md
# "The loaded dynamic array", D-27.60). Builds lib.c and probe.c in SCRATCH (created, must not
# exist) and runs the probe on the reference loader (and a static-pie build) and, given libniji's
# products directory, under libniji's ld.so as the program interpreter.
#   sh tools/probe/dl/dynreloc/run.sh SCRATCH [NIJI_PRODUCTS_DIR]     (NIJI_ONLY=1: libniji only)
set -u
SRC=$(cd "$(dirname "$0")" && pwd)
W=${1:?usage: run.sh SCRATCH [NIJI_PRODUCTS_DIR]}; N=${2:-}
[ -e "$W" ] && { echo "run.sh: $W exists" >&2; exit 2; }
mkdir -p "$W" && W=$(cd "$W" && pwd)
export LC_ALL=C
unset LD_LIBRARY_PATH LD_PRELOAD LD_BIND_NOW GLIBC_TUNABLES
CC=${CC:-cc}
$CC -O1 -fPIC -shared -DNAME=dr_a -Wl,--hash-style=both -Wl,-z,pack-relative-relocs -Wl,-soname,libdrA.so -o "$W/libdrA.so" "$SRC/lib.c" || exit 2
$CC -O1 -fPIC -shared -DNAME=dr_b -DTLSDESC -mtls-dialect=gnu2 -Wl,-z,lazy -Wl,-soname,libdrB.so -o "$W/libdrB.so" "$SRC/lib.c" || exit 2
$CC -O1 -fPIE -pie -Wl,--enable-new-dtags -Wl,-rpath,'$ORIGIN' -o "$W/probe" "$SRC/probe.c" "$W/libdrA.so" || exit 2
if [ -z "${NIJI_ONLY:-}" ]; then
  echo "== reference"
  timeout 30 "$W/probe" "$W"; echo "rc=$?"
  $CC -O1 -static-pie -DSTATIC_PROBE -o "$W/sprobe" "$SRC/probe.c" && { echo "== reference static-pie"; timeout 30 "$W/sprobe" "$W"; echo "rc=$?"; }
fi
if [ -n "$N" ]; then
  echo "== libniji"
  $CC -O1 -fPIE -pie -Wl,--dynamic-linker="$N/ld-linux-x86-64.so.2" -Wl,--enable-new-dtags -Wl,-rpath,'$ORIGIN' -o "$W/probe-niji" "$SRC/probe.c" "$W/libdrA.so" || exit 2
  timeout 30 env LD_LIBRARY_PATH="$N" "$W/probe-niji" "$W"; echo "rc=$?"
fi
