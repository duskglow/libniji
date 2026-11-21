#!/bin/sh
# Build and run the glibc reference benchmarks (benchmarks/glibc_ref.c) for progress.md.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
out=${TMPDIR:-/tmp}/niji-glibc-ref
cc -O2 -pthread -o "$out" "$here/glibc_ref.c"
"$out"
