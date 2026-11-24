#!/bin/bash
# STARTUP-REPROFILE: dynamic start counts of libniji products against the host's C library (glibc 2.39), same host.
# Programs (kit form, as STARTUP-STRUCTURE-REVIEW and LD-LOOKUP-COST): m0 empty main, m3 three one-function
# libraries, p0/p200 LD-LOOKUP-COST's empty main and 200 GLOB_DAT imports, /bin/true with PT_INTERP rewritten.
# libniji programs get LD_LIBRARY_PATH as their whole environment; host programs an empty one. ASLR off.
# Per program: stc (user instr median/min/max, kernel instr median, faults), strace system-call count.
# usage: meas.sh PRODUCTS OUTDIR [pt]   (pt: Intel PT exclusive and inclusive per-symbol tables, libniji only)
set -e
P=$(readlink -f "$1"); O=$2; MODE=$3; RUNS=${RUNS:-101}
K=${K:-~/work/ld-start-maps/kit}
mkdir -p "$O"
lib=/tmp/niji-srp-$$
rm -rf "$lib"; mkdir -p "$lib/d" "$lib/b"
trap '[ -n "$KEEP" ] || rm -rf "$lib"' EXIT; echo "lib: $lib"
B=$lib/b
cp "$P/ld-linux-x86-64.so.2" "$P/libc.so.6" "$lib/"
cc -O2 -o "$B/stc" "$K/stc.c"
for k in 1 2 3; do cc -O1 -shared -fPIC "$K/progs/d$k.c" -o "$lib/d/libd$k.so"; done
SYS=$(readlink -f /lib/x86_64-linux-gnu/libc.so.6)
python3 -I "$K/gen.py" "$P/libc.so.6" "$SYS" 0 > "$B/p0.c"
python3 -I "$K/gen.py" "$P/libc.so.6" "$SYS" 200 > "$B/p200.c"
lk() {
  src=$1; out=$2; shift 2
  cc -std=gnu17 -O1 -nostdlib -fno-builtin -w -fPIE -pie "-Wl,--dynamic-linker=$lib/ld-linux-x86-64.so.2" \
    "$P/niji-crt1.o" "$(cc -print-file-name=crtbeginS.o)" "$src" "$@" "$P/libc_nonshared.a" "$lib/libc.so.6" \
    "$lib/ld-linux-x86-64.so.2" -lgcc "$(cc -print-file-name=crtendS.o)" -o "$out"
}
D3="$lib/d/libd1.so $lib/d/libd2.so $lib/d/libd3.so"
lk "$K/progs/m0.c" "$B/n0"; lk "$K/progs/m3.c" "$B/n3" $D3; lk "$B/p0.c" "$B/np0"; lk "$B/p200.c" "$B/n200"
cc -O1 -fno-builtin -w -fPIE -pie "$K/progs/m0.c" -o "$B/g0"
cc -O1 -fno-builtin -w -fPIE -pie "$K/progs/m3.c" -L"$lib/d" -ld1 -ld2 -ld3 -Wl,-rpath,"$lib/d" -o "$B/g3"
cc -O1 -fno-builtin -w -fPIE -pie "$B/p0.c" -o "$B/gp0"; cc -O1 -fno-builtin -w -fPIE -pie "$B/p200.c" -o "$B/g200"
# /bin/true with PT_INTERP rewritten in place (same length) to /tmp/Z/ld-linux-x86-64.so.2, /tmp/Z -> $lib
python3 -I -c 'import sys; b=open("/bin/true","rb").read(); o=b"/lib64/ld-linux-x86-64.so.2\0"; assert b.count(o)==1; open(sys.argv[1],"wb").write(b.replace(o,b"/tmp/Z/ld-linux-x86-64.so.2\0"))' "$B/t"
chmod +x "$B/t"
ln -sfn "$lib" /tmp/Z
LP="$lib:$lib/d"
echo "model: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2)  products: $P  runs $RUNS"
echo "symbolic relocations (GLOB_DAT/JUMP_SLOT/64): n0 $(readelf -rW "$B/n0" | grep -c -E 'GLOB_DAT|JUMP_SLO|R_X86_64_64') n200 $(readelf -rW "$B/n200" | grep -c GLOB_DAT) libc.so.6 $(readelf -rW "$lib/libc.so.6" | grep -c -E 'GLOB_DAT|JUMP_SLO|R_X86_64_64|TPOFF|DTPMOD') relative $(readelf -rW "$lib/libc.so.6" | grep -c RELATIVE) relr $(readelf -SW "$lib/libc.so.6" | grep -c -i relr)"
row() {
  name=$1; L=$2; prog=$3
  if [ -n "$L" ]; then c=$(cd / && env -i setarch -R "$B/stc" "$RUNS" -L "$L" "$prog"); else c=$(cd / && env -i setarch -R "$B/stc" "$RUNS" "$prog"); fi
  if [ -n "$L" ]; then (cd / && env -i strace -f -qq -E "LD_LIBRARY_PATH=$L" -o "$O/$name.st" "$prog" >/dev/null 2>&1); else (cd / && env -i strace -f -qq -o "$O/$name.st" "$prog" >/dev/null 2>&1); fi
  sc=$(wc -l < "$O/$name.st"); mm=$(grep -c -E '^[0-9]+ +(mmap|mprotect|munmap)\(' "$O/$name.st" || true)
  op=$(grep -c -E '^[0-9]+ +(openat|open|read|pread64|fstat|newfstatat|close)\(' "$O/$name.st" || true)
  echo "row: $name $c syscalls $sc map $mm file $op"
}
row n0 "$LP" "$B/n0"; row n3 "$LP" "$B/n3"; row t "$LP" "$B/t"; row np0 "$LP" "$B/np0"; row n200 "$LP" "$B/n200"
row g0 "" "$B/g0"; row g3 "" "$B/g3"; row true "" /bin/true; row gp0 "" "$B/gp0"; row g200 "" "$B/g200"
if [ "$MODE" = pt ]; then
  for p in n0 n3 t np0 n200; do
    (cd / && setarch -R perf record -q -e intel_pt//u -o "$lib/pt-$p.data" -- env -i LD_LIBRARY_PATH="$LP" "$B/$p" >/dev/null 2>&1) || true
    perf script -i "$lib/pt-$p.data" --itrace=i1ig256 --comms "$p" -F comm,ip,sym 2>/dev/null | c++filt | sed -E "s/\\[[0-9a-f]{12,16}\\]//g" | python3 -I "$K/incl.py" "$p" "$O/pt-$p"
  done
fi
rm -f /tmp/Z
cp "$B"/*.c "$O/"
