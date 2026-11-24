#!/bin/bash
# LD-START-MAPS: attribute a start's kernel work and faults (kit form programs n0 / g0).
# usage: attr.sh PRODUCTS OUTDIR [REPS]
# Per program: strace with arguments; perf page-fault samples with data addresses and mmap events;
# perf kernel-instruction samples (every 500) with the user registers at kernel entry (ip, di, si),
# repeated REPS times, so each kernel instruction sample is charged to the user instruction that
# entered the kernel (a system call site or a faulting access).
set -e
P=$(readlink -f "$1"); O=$(readlink -f "$2"); REPS=${3:-20}
K=${K:-~/work/ld-start-maps/kit}
mkdir -p "$O"
lib=/tmp/niji-lsm-$$
rm -rf "$lib"; mkdir -p "$lib/b"
trap 'rm -rf "$lib"' EXIT
B=$lib/b
cp "$P/ld-linux-x86-64.so.2" "$P/libc.so.6" "$lib/"
lk() {
  src=$1; out=$2; shift 2
  cc -std=gnu17 -O1 -nostdlib -fno-builtin -w -fPIE -pie "-Wl,--dynamic-linker=$lib/ld-linux-x86-64.so.2" \
    "$P/niji-crt1.o" "$(cc -print-file-name=crtbeginS.o)" "$src" "$@" "$P/libc_nonshared.a" "$lib/libc.so.6" \
    "$lib/ld-linux-x86-64.so.2" -lgcc "$(cc -print-file-name=crtendS.o)" -o "$out"
}
lk "$K/progs/m0.c" "$B/n0"
cc -O1 -fno-builtin -w -fPIE -pie "$K/progs/m0.c" -o "$B/g0"
cc -O2 -o "$B/stc" "$K/stc.c"
for p in n0 g0; do
  if [ $p = n0 ]; then E="LD_LIBRARY_PATH=$lib"; L="-L $lib"; else E="X=1"; L=; fi
  (cd / && env -i setarch -R "$B/stc" 101 $L "$B/$p") > "$O/$p.stc"
  (cd / && env -i setarch -R strace -f -qq -o "$O/$p.st" -E "$E" "$B/$p" >/dev/null 2>&1) || true
  (cd / && env -i setarch -R perf record -q -d -e page-faults -c 1 -o "$lib/pf-$p.data" -- env -i "$E" "$B/$p" >/dev/null 2>&1) || true
  perf script -i "$lib/pf-$p.data" --show-mmap-events --comms $p -F comm,ip,addr,sym,dso 2>/dev/null > "$O/$p.pf" || true
  : > "$O/$p.ki"
  for r in $(seq 1 "$REPS"); do
    (cd / && env -i setarch -R perf record -q -e instructions:k -c ${PERIOD:-10000} --user-regs=ip,di,si -o "$lib/ki-$p.data" -- env -i "$E" "$B/$p" >/dev/null 2>&1) || true
    perf script -i "$lib/ki-$p.data" --show-mmap-events --comms $p -F comm,ip,uregs 2>/dev/null >> "$O/$p.ki" || true
  done
done
cp "$B/n0" "$B/g0" "$O/"; cp "$lib/ld-linux-x86-64.so.2" "$lib/libc.so.6" "$O/"
echo done
