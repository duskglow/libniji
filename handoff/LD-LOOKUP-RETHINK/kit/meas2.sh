#!/bin/bash
# LD-LOOKUP-RETHINK: symbol-lookup scenarios and start-up rows, libniji against the host C library (glibc 2.39),
# same host, ASLR off. usage: meas2.sh PRODUCTS OUTDIR [trace]   env: RUNS (21), N (200), SYMLD (ld.so with symbols)
# Counts: user instructions exec to exit (STARTUP-REPROFILE's stc, median of RUNS) for both loaders. With `trace`:
# sstep traces of the libniji programs and attr.py tables (libniji's loader only; the reference is only counted).
# libniji programs get LD_LIBRARY_PATH as their whole environment, host programs an empty one (kit form).
set -e
P=$(readlink -f "$1"); O=$(readlink -f -m "$2"); MODE=$3; RUNS=${RUNS:-21}; N=${N:-200}
K=~/work/ld-lookup-rethink; R=$K/srp
mkdir -p "$O"
lib=/tmp/niji-llr-$$
rm -rf "$lib"; mkdir -p "$lib/d" "$lib/b"
cleanup() { rm -rf "$lib"; if [ -L /tmp/Y ] && [ "$(readlink /tmp/Y)" = "$lib" ]; then rm -f /tmp/Y; fi; }
trap cleanup EXIT
B=$lib/b; D=$lib/d
cp "$P/ld-linux-x86-64.so.2" "$P/libc.so.6" "$lib/"
sha256sum "$P/ld-linux-x86-64.so.2" "$P/libc.so.6" > "$O/products.sha256"
cc -O2 -o "$B/stc" "$R/stc.c"; cc -O2 -o "$B/sstep" "$K/sstep.c"
python3 -I "$K/gen2.py" "$B" "$N"
for l in la_s la_l lx1 lx2 lx3 lm lml lm0; do cc -O1 -shared -fPIC -nostdlib -Wl,-soname,lib$l.so "$B/$l.c" -o "$D/lib$l.so"; done
cc -O1 -shared -fPIC -nostdlib -Wl,-soname,libla_v.so -Wl,--version-script="$B/la_v.map" "$B/la_v.c" -o "$D/libla_v.so"
for k in 1 2 3; do cc -O1 -shared -fPIC -Wl,-soname,libd$k.so "$R/progs/d$k.c" -o "$D/libd$k.so"; done
SYS=$(readlink -f /lib/x86_64-linux-gnu/libc.so.6)
python3 -I "$R/gen.py" "$P/libc.so.6" "$SYS" 0 > "$B/p0.c"
python3 -I "$R/gen.py" "$P/libc.so.6" "$SYS" "$N" > "$B/p.c"
cp "$R/progs/m0.c" "$R/progs/m3.c" "$B/"
lk() {  # libniji program: SRC OUT LIB...
  local src=$1 out=$2; shift 2
  cc -std=gnu17 -O1 -nostdlib -fno-builtin -w -fPIE -pie "-Wl,--dynamic-linker=$lib/ld-linux-x86-64.so.2" \
    "$P/niji-crt1.o" "$(cc -print-file-name=crtbeginS.o)" "$src" -Wl,--no-as-needed "-L$D" "$@" "$P/libc_nonshared.a" \
    "$lib/libc.so.6" "$lib/ld-linux-x86-64.so.2" -lgcc "$(cc -print-file-name=crtendS.o)" -o "$out"
}
gk() {  # host program: SRC OUT LIB...
  local src=$1 out=$2; shift 2
  cc -O1 -fno-builtin -w -fPIE -pie "$src" -Wl,--no-as-needed "-L$D" "$@" -Wl,-rpath,"$D" -o "$out"
}
PROGS="a0:a0:la_s a:a:la_s l0:a0:la_l l:l:la_l v0:a0:la_v v:v:la_v b0:a0:lx1,lx2,lx3,la_s b:a:lx1,lx2,lx3,la_s
c0:a0:lm0 c:a0:lm cl:a0:lml cb0:a0:lx1,lx2,lx3,lm0 cb:a0:lx1,lx2,lx3,lm p0:p0: p:p: m0:m0: m3:m3:d1,d2,d3"
for spec in $PROGS; do
  name=${spec%%:*}; rest=${spec#*:}; src=${rest%%:*}; libs=${rest#*:}
  set --; for l in ${libs//,/ }; do set -- "$@" "-l$l"; done
  lk "$B/$src.c" "$B/n-$name" "$@"; gk "$B/$src.c" "$B/g-$name" "$@"
done
# /bin/true with PT_INTERP rewritten (same length) to /tmp/Y/ld-linux-x86-64.so.2, /tmp/Y -> $lib
if [ -e /tmp/Y ] || [ -L /tmp/Y ]; then echo "t: /tmp/Y exists, /bin/true row skipped"; else
  python3 -I -c 'import sys; b=open("/bin/true","rb").read(); o=b"/lib64/ld-linux-x86-64.so.2\0"; assert b.count(o)==1; open(sys.argv[1],"wb").write(b.replace(o,b"/tmp/Y/ld-linux-x86-64.so.2\0"))' "$B/n-t"
  chmod +x "$B/n-t"; ln -sn "$lib" /tmp/Y; fi
LP="$lib:$D"
echo "model: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2)  products: $P  runs $RUNS  N $N"
readelf -rW "$B/n-p" | grep -c GLOB_DAT | sed 's/^/GLOB_DAT in n-p: /'
[ "$MODE" = traceonly ] && PROGS_COUNT= || PROGS_COUNT="$PROGS t:t:"
for spec in $PROGS_COUNT; do
  name=${spec%%:*}
  [ -x "$B/n-$name" ] || continue
  c=$(cd / && env -i setarch -R "$B/stc" "$RUNS" -L "$LP" "$B/n-$name")
  if [ "$name" = t ]; then g=$(cd / && env -i setarch -R "$B/stc" "$RUNS" /bin/true); else g=$(cd / && env -i setarch -R "$B/stc" "$RUNS" "$B/g-$name"); fi
  echo "row: $name niji $c | glibc $g"
done | tee -a "$O/counts.txt"
if [ "$MODE" = trace ] || [ "$MODE" = traceonly ]; then
  SYMS=(); [ -n "$SYMLD" ] && SYMS=("ld-linux-x86-64.so.2=$SYMLD")
  for name in a0 a b0 b c0 c cb0 cb cl v0 v l0 l p0 p m0 t; do
    [ -x "$B/n-$name" ] || continue
    s=$(cd / && env -i setarch -R "$B/sstep" "$lib/tr-$name" -L "$LP" "$B/n-$name")
    echo "trace $name: $s"
    python3 -I "$K/attr.py" "$lib/tr-$name" "$O/$name.json" "${SYMS[@]}"
    cp "$lib/tr-$name.maps" "$O/$name.maps"; rm -f "$lib/tr-$name.seq"
  done
  for pr in a0:a b0:b c0:c cb0:cb c0:cl v0:v l0:l p0:p; do
    for kind in inl self incl pair; do python3 -I "$K/diff.py" "$O/${pr%%:*}.json" "$O/${pr##*:}.json" "$N" $kind 45 > "$O/d-${pr##*:}-$kind.txt"; done
    python3 -I "$K/diff.py" "$O/${pr%%:*}.json" "$O/${pr##*:}.json" "$N" line 90 > "$O/d-${pr##*:}-line.txt"
  done
fi
cp "$B"/*.c "$B"/*.map "$O/" 2>/dev/null || true
echo "done: $O"
