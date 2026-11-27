#!/bin/bash
# Compile small probes against: 26.3 as shipped, 26.3 behind its fixincludes output, the __need_ variant, and
# the variant behind its own fixincludes output. gcc-14's own <stdarg.h>/<stddef.h> (ginclude) come first, as in
# a built gcc (include, then include-fixed, then the system headers).
set -u
W=~/work/gcc-fixincludes-probe
P=$W/probe
mkdir -p "$P/gcc14inc" "$P/kern"
# kernel UAPI headers only (linux-libc-dev), never the host C library's headers
for k in linux asm-generic; do ln -sfn "/usr/include/$k" "$P/kern/$k"; done
ln -sfn /usr/include/x86_64-linux-gnu/asm "$P/kern/asm"
rm -rf "$W/variant" "$W/out-variant"
cp "$W/src/gcc-14.2.0/gcc/ginclude/stdarg.h" "$W/src/gcc-14.2.0/gcc/ginclude/stddef.h" "$P/gcc14inc/"
T=$W/trees/20261126.3/usr/include
V=$W/variant/usr/include
[ -d "$V" ] || { mkdir -p "$W/variant/usr" && python3 -I "$W/scripts/variant.py" "$T" "$V"; } || exit 1
OUTTAG=variant bash "$W/scripts/run_trees.sh" "variant=$V"
echo "variant fixed files: $(find "$W/out-variant/variant/fixed" -type f | wc -l)"
HOSTINC=$(gcc -print-file-name=include)
printf '#include <stdio.h>\n#include <stdarg.h>\nint f(const char *s, ...) { va_list a; va_start(a, s); int r = vfprintf(stderr, s, a); va_end(a); return r; }\n' > "$P/va_stdarg.c"
printf '#include <stdio.h>\nint g(const char *s, va_list a) { return vprintf(s, a); }\n' > "$P/va_stdio_only.c"
printf '#include <wchar.h>\n#include <stdarg.h>\nint h(const wchar_t *s, ...) { va_list a; va_start(a, s); int r = vwprintf(s, a); va_end(a); return r; }\n' > "$P/va_wchar.c"
printf '#include <stdlib.h>\n#include <unistd.h>\n#include <stddef.h>\n#include <string.h>\nsize_t n(const char *s) { return strlen(s); }\nwchar_t w; ptrdiff_t d;\n' > "$P/size_t.c"
printf '#include <stdarg.h>\n#include <stdio.h>\n#include <wchar.h>\n#include <stdlib.h>\nint k(const char *s, ...) { va_list a; va_start(a, s); char b[8]; int r = vsnprintf(b, sizeof b, s, a); va_end(a); return r; }\n' > "$P/stdarg_first.c"
run() { # label sysdirs...
  local label=$1; shift
  local inc=(-nostdinc -isystem "$P/gcc14inc" -isystem "$HOSTINC")
  for d in "$@"; do inc+=(-isystem "$d"); done
  local base=${!#}
  inc+=(-isystem "$base/x86_64-linux-gnu" -idirafter "$P/kern")
  for src in va_stdarg va_stdio_only va_wchar size_t stdarg_first; do
    for mode in "gcc -std=gnu17" "gcc -std=c99 -pedantic-errors" "g++ -x c++ -std=c++98" "g++ -x c++ -std=c++11"; do
      if $mode "${inc[@]}" -fsyntax-only "$P/$src.c" 2> "$P/err.txt"; then r=ok; else r="FAIL: $(grep -m1 error "$P/err.txt" | sed "s|$W/||g")"; fi
      echo "$label | $src | $mode | $r" >> "$P/matrix.txt"
      [ "$r" = ok ] || echo "$label | $src | $mode | $r"
    done
  done
}
: > "$P/matrix.txt"
run shipped "$T"
run shipped+fixed "$W/out-v2/s20261126.3/fixed" "$T"
run variant "$V"
run variant+fixed "$W/out-variant/variant/fixed" "$V"
echo "matrix: $(grep -c '| ok$' "$P/matrix.txt") ok of $(wc -l < "$P/matrix.txt")"
for l in shipped shipped+fixed variant variant+fixed; do echo "$l: $(grep -c "^$l | .*| ok$" "$P/matrix.txt") ok of $(grep -c "^$l |" "$P/matrix.txt")"; done
for c in /usr/lib/llvm-*/lib/clang/*/include; do
  [ -d "$c" ] || continue
  echo "clang headers $c: __need_size_t=$(grep -c __need_size_t "$c/stddef.h") __need___va_list=$(grep -c __need___va_list "$c/stdarg.h")"
done
