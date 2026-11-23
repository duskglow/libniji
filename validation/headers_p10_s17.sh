#!/bin/sh
# P10 for slice 17 (plan/headers.md section 3): build validation/headers_p10_s17.c against the reference's headers and
# against libniji's (libc-include/), at -O0 and -O2, and require identical output. Both link the installed libc (the
# ABI is identical). No spec clause: header macros (GNU C Library manual, Obstacks; queue(7)).
# Run from the repository root. Prints "P10 <opt> identical" per optimisation level; exit 1 on any difference.
set -e
W=$(mktemp -d /dev/shm/p10-s17.XXXXXX)
trap 'rm -rf "$W"' EXIT
GI=$(gcc -print-file-name=include)
for O in -O0 -O2; do
  gcc $O -Wall -Wextra -Werror -o "$W/ref$O" validation/headers_p10_s17.c
  gcc $O -Wall -Wextra -Werror -nostdinc -isystem libc-include -isystem "$GI" -o "$W/niji$O" validation/headers_p10_s17.c
  "$W/ref$O" > "$W/ref$O.out"
  "$W/niji$O" > "$W/niji$O.out"
  if cmp "$W/ref$O.out" "$W/niji$O.out"; then echo "P10 $O identical ($(wc -l < "$W/ref$O.out") lines)"; else echo "P10 $O DIFFERENT"; diff "$W/ref$O.out" "$W/niji$O.out" | head -10; exit 1; fi
done
