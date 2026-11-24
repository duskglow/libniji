#!/bin/sh
# Run collcontr over the Latin candidate set for the named prebuilt locales.
# usage: contr_run.sh <locpath> <outdir> <locale>...
lp=$1; out=$2; shift 2
mkdir -p "$out"
set_=$(python3 -c '
r=list(range(0x41,0x5b))+list(range(0x61,0x7b))+[c for c in range(0xc0,0x100) if c not in (0xd7,0xf7)]+list(range(0x100,0x180))
print(" ".join("%X"%c for c in r))')
for l in "$@"; do
  LOCPATH=$lp timeout 600 "$(dirname "$0")/collcontr" "$l.UTF-8" $set_ >"$out/$l.contr" 2>"$out/$l.contr.err"
  cat "$out/$l.contr.err"
done
