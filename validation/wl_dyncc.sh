#!/bin/sh
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# The link driver of the libniji-DYNAMIC workload kit and per-call binaries (WL-DYNAMIC-KIT; validation/wl_lock.sh
# WL_LINK=dynamic). The dynamic counterpart of scripts/niji-cc's static link: the system compiler driver with
# libniji's start files and libraries where an installed libniji puts them (-B/-L the products' lib/ directory, the
# layout of wl_dyn_layout), libniji's ld.so as the interpreter, gcc's own crtbegin/crtend and libgcc as for any
# program. After the link, wl_dynlink.py points the absolute DT_NEEDED of the by-path copies at the shipped
# libc.so.6 / libm.so.6, so the program runs without LD_LIBRARY_PATH.
#
#   WL_DYN_PRODUCTS=<products dir> wl_dyncc.sh <compiler arguments>
#
# Compiling (-c, -S, -E, -M, -MM) and -shared are the real compiler, unchanged. -static, -pthread and the libraries
# glibc folded into libc (-lpthread -lrt -ldl -lutil -lanl -lresolv) and -lmvec are dropped, as niji-cc drops them;
# -lm stays (libniji's libm.so.6). NIJI_CC_EXTRA: words added at the end of the link (gawk's -lm). NIJI_REAL_CC
# (default gcc): the compiler.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
d=${WL_DYN_PRODUCTS:?wl_dyncc: WL_DYN_PRODUCTS names the dynamic products directory}
cc=${NIJI_REAL_CC:-gcc}
for a in "$@"; do
    case $a in -c | -S | -E | -M | -MM | -shared) exec "$cc" "$@" ;; esac
done
for f in "$d/lib/libc.so" "$d/lib/Scrt1.o" "$d/ld-linux-x86-64.so.2"; do
    [ -f "$f" ] || { echo "wl_dyncc: $f is missing (wl_dyn_layout, validation/wl_lock.sh)" >&2; exit 1; }
done
out=
n=$#
i=0
prev=
while [ $i -lt $n ]; do
    a=$1
    shift
    i=$((i + 1))
    [ "$prev" = -o ] && out=$a
    prev=$a
    case $a in
        -lpthread | -lrt | -ldl | -lutil | -lanl | -lresolv | -lmvec | -static | -pthread) ;;
        *) set -- "$@" "$a" ;;
    esac
done
[ -n "$out" ] || { echo "wl_dyncc: no -o in the link" >&2; exit 1; }
# shellcheck disable=SC2086
# -rpath-link: the link resolves libm.so.6's own DT_NEEDED libc.so.6 to libniji's (the products directory), never
# the host's; it records nothing in the program.
"$cc" -B"$d/lib/" -L"$d/lib" "-Wl,--dynamic-linker=$d/ld-linux-x86-64.so.2" "-Wl,-rpath-link,$d" "$@" ${NIJI_CC_EXTRA:-}
python3 "$here/wl_dynlink.py" retarget "$out" "$d" > /dev/null
