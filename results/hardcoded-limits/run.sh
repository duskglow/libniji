#!/bin/bash
# AUDIT-HARDCODED-LIMITS probe driver (a build host with a compiler). usage: run.sh <workdir> <niji products dir>
# Builds the probes and N tiny shared objects, then runs every probe against the reference (the
# installed C library) and against libniji's products (through libniji's loader). Every run has a
# timeout. Output: one line per probe and library.
set -u
W=$1; NIJI=$2
mkdir -p "$W/lib" && cd "$W" || exit 1
cc -O1 -o probes probes.c -ldl -lanl -lpthread || exit 1
# 600 objects, each with a thread-local variable (dynamic TLS) and one function.
for i in $(seq 0 599); do echo "$i"; done | xargs -P 16 -I{} sh -c \
  'printf "__thread int t;\nint f{}(void) { return t; }\n" > lib/p{}.c && cc -shared -fPIC -o lib/libp{}.so lib/p{}.c'
# A library needing 100 of them, and programs needing 100 and 300 of them.
needs() { n=$1; s=""; for i in $(seq 0 $((n - 1))); do s="$s -lp$i"; done; echo "$s"; }
echo 'int wide(void) { return 0; }' > lib/wide.c
cc -shared -fPIC -o lib/libwide.so lib/wide.c -Llib -Wl,--no-as-needed $(needs 100)
echo 'int main(void) { return 0; }' > need.c
cc -o need100 need.c -Llib -Wl,--no-as-needed $(needs 100)
cc -o need300 need.c -Llib -Wl,--no-as-needed $(needs 300)
# The same programs linked against libniji's products (as xtask so links its utilities).
nl() { out=$1; shift; cc -pie -nostdlib -Wl,-z,now -Wl,--dynamic-linker="$NIJI/ld-linux-x86-64.so.2" \
  "$NIJI/niji-crt1.o" "$NIJI/niji-crti.o" "$(cc -print-file-name=crtbeginS.o)" "$@" "$NIJI/libc.so.6" "$NIJI/libc_nonshared.a" -lgcc "$(cc -print-file-name=crtendS.o)" "$NIJI/niji-crtn.o" -o "$out"; }
cc -O1 -fPIE -c -o probes.o probes.c && nl probes.niji probes.o
cc -fPIE -c -o need.o need.c
nl need100.niji need.o -Llib -Wl,--no-as-needed $(needs 100)
nl need300.niji need.o -Llib -Wl,--no-as-needed $(needs 300)
# 1100 locale names, each a link to the installed C.utf8 directory.
mkdir -p loc && for i in $(seq 0 1099); do ln -sfn /usr/lib/locale/C.utf8 "loc/l$i"; done

ref() { timeout 120 "$@" 2>&1 | tail -2 | sed 's/^/  ref:   /'; echo "  ref rc ${PIPESTATUS[0]}"; }
niji() { p=$1; shift; timeout 120 env LD_LIBRARY_PATH="$NIJI:$W/lib" "$p.niji" "$@" 2>&1 | tail -2 | sed 's/^/  niji:  /'; echo "  niji rc ${PIPESTATUS[0]}"; }
both() { echo "== $*"; ref "$@"; niji "$@"; }

echo "reference: $(ldd --version | head -1)"
echo "libniji products: $NIJI ($(stat -c %y "$NIJI/libc.so.6" | cut -c1-16))"
export LD_LIBRARY_PATH="$W/lib"
both ./probes dlopen_many 600 "$W/lib"
both ./probes dlopen_wide "$W/lib/libwide.so"
both ./need100
both ./need300
echo "== ldd need300 (count of listed lines)"
echo "  ref:   $(timeout 60 ldd ./need300 2>&1 | wc -l) lines, rc ${PIPESTATUS[0]}"
echo "  niji:  $(timeout 60 "$NIJI/ldd" ./need300.niji 2>&1 | wc -l) lines; tail: $(timeout 60 "$NIJI/ldd" ./need300.niji 2>&1 | tail -1)"
both ./probes sem_many 300
both ./probes severity_many 300
both ./probes gai_many 5000
LOCPATH="$W/loc" both ./probes locale_many 1100
both ./probes printf_registry 20 300
both ./probes atexit_many 1100000
both ./probes fnmatch_depth 65
both ./probes fnmatch_depth 200
both ./probes wordexp_depth 129
both ./probes wordexp_depth 300
exit 0
