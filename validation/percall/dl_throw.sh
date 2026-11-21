#!/bin/sh
# LAB-ONLY (DL-FIND-OBJECT-FAST): instructions per C++ throw (validation/percall/dl_throw.cc), through 10 frames in
# the program (the first object) or in the last object loaded, with the C++ start set (+1) and with 104 objects.
#   sh validation/percall/dl_throw.sh <corpus dir> <out dir> <glibc | libniji products dir>...
# <corpus dir>: validation/percall/dl_objects.sh's. A products dir is a dynamic product set (~/wl/products/<sha>-release,
# as validation/wl_percall.sh makes it); the program links against it with validation/wl_dyncc.sh. Per throw =
# (I(N throws) - I(0 throws)) / N, perf stat -r 5, instructions:u (deterministic); the setup (dlopen) cancels.
set -eu
q=${1:?usage: dl_throw.sh <corpus dir> <out dir> <glibc|products dir>...}
o=${2:?usage}
shift 2
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$o"
g++ -O2 -fPIC -shared -DDL_THROW_LIB -o "$o/libdlthrow.so" "$here/dl_throw.cc"
N=2000
count() { # binary throws objects where -> instructions:u, mean of 5
    perf stat -r 5 -x, -e instructions:u "$1" "$2" "$3" "$4" "$q" "$o/libdlthrow.so" 2>&1 >/dev/null |
        awk -F, '$3 ~ /^instructions/ { print $1; exit }'
}
printf '%-40s %-5s %8s %12s\n' variant where objects instr/throw
for v in "$@"; do
    if [ "$v" = glibc ]; then
        b=$o/dl_throw.glibc
        g++ -O2 -Wl,-z,now -o "$b" "$here/dl_throw.cc"
    else
        b=$o/dl_throw.$(basename "$v")
        WL_DYN_PRODUCTS=$v NIJI_REAL_CC=g++ sh "$here/../wl_dyncc.sh" -O2 -Wl,-z,now -o "$b" "$here/dl_throw.cc"
    fi
    for want in 8 104; do
        for where in prog lib; do
            "$b" 1 "$want" "$where" "$q" "$o/libdlthrow.so" > "$o/check.txt" || { echo "dl_throw: $v $want $where failed" >&2; exit 1; }
            objs=$(awk '{ print $2 }' "$o/check.txt")
            i0=$(count "$b" 0 "$want" "$where")
            i1=$(count "$b" "$N" "$want" "$where")
            printf '%-40s %-5s %8s %12s\n' "$(basename "$v")" "$where" "$objs" "$(( (i1 - i0) / N ))"
        done
    done
done
