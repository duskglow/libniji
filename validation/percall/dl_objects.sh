#!/bin/sh
# LAB-ONLY: the library corpus of validation/percall/dl_objects.c (DL-FIND-OBJECT-FAST).
#   sh validation/percall/dl_objects.sh <dir> [count]   (default 600)
# Builds <dir>/libq1.so .. libq<count>.so: `-nostdlib` libraries without dependencies (one function, one
# variable, PT_GNU_EH_FRAME), loadable under either C library. Then run the spec with PC_DL_DIR=<dir>:
#   PC_DL_DIR=<dir> bash validation/wl_percall.sh --spec validation/percall/dl_objects.c .
set -eu
d=${1:?usage: dl_objects.sh <dir> [count]}
count=${2:-600}
mkdir -p "$d"
k=1
while [ "$k" -le "$count" ]; do
    if [ ! -s "$d/libq$k.so" ]; then
        printf 'int v%d = %d;\nint f%d(void) { return v%d; }\n' "$k" "$k" "$k" "$k" > "$d/libq$k.c"
        cc -O1 -nostdlib -fPIC -shared -Wl,--eh-frame-hdr "-Wl,-soname,libq$k.so" -o "$d/libq$k.so" "$d/libq$k.c"
    fi
    k=$((k + 1))
done
