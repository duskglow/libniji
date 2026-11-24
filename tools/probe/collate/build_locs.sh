#!/bin/sh
# Build reference locales with the reference localedef into a private LOCPATH
# (black-box use of the installed tool and its installed data; nothing read).
# usage: build_locs.sh <outdir> <locale>...   (locale like sv_SE; charmap UTF-8)
out=$1; shift
mkdir -p "$out"
for l in "$@"; do
  if [ -d "$out/$l.UTF-8" ]; then continue; fi
  s=$(date +%s.%N)
  timeout 300 localedef -i "$l" -f UTF-8 "$out/$l.UTF-8" >"$out/$l.log" 2>&1
  rc=$?
  e=$(date +%s.%N)
  printf '%s\trc=%s\t%.2fs\n' "$l" "$rc" "$(echo "$e - $s" | bc)"
done
