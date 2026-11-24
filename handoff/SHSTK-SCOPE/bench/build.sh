#!/bin/bash
# SHSTK-SCOPE: build the probes and benches on the build host. Reference set in out/g (the
# host's C library), libniji set in out/n (dynamic through libniji's ld.so at the N100 path
# $D, static with libniji's start object and libc.a). Usage: build.sh [g|n|all]
set -eu
W=~/work/shstk-scope; S=$W/src; O=$W/out
D=$HOME/work/shstk-scope/niji
N=~/w/n1/target/niji
what=${1:-all}
mkdir -p $O/g $O/n
libs() {
	for i in 1 2 3; do gcc -O2 -shared -fPIC -DNAME=lib${i}_fn -o lib$i.so $S/lib.c; done
	gcc -O2 -shared -fPIC -fcf-protection=none -o libunm.so $S/lib.c
	gcc -O2 -shared -fPIC -o libmk.so $S/lib.c
}
if [ "$what" != n ]; then
	cd $O/g
	libs
	gcc -O2 -o empty $S/empty.c
	gcc -O2 -static -o empty_static $S/empty.c
	gcc -O2 -o shb $S/shb.c
	gcc -O2 -static -o shb_static $S/shb.c
	gcc -O2 -fcf-protection=none -o shb_unm $S/shb.c
	gcc -O2 -fexceptions -o shb_exc $S/shb.c
	g++ -O2 -o cxx $S/cxx.cc
	gcc -O2 -o main3 $S/main3.c -L. -l1 -l2 -l3 -Wl,--disable-new-dtags -Wl,-rpath,'$ORIGIN'
fi
if [ "$what" != g ]; then
	cd $O/n
	libs
	LDN=$(cd $N && ls ld-linux-x86-64.so.2 2>/dev/null || ls ld*.so* | head -1)
	DYN="-Wl,--dynamic-linker=$D/$LDN -Wl,--disable-new-dtags -Wl,-rpath,$D:\$ORIGIN"
	gcc -O2 -o empty $S/empty.c $DYN
	gcc -O2 -o shb $S/shb.c $DYN
	g++ -O2 -o cxx $S/cxx.cc $DYN
	gcc -O2 -o main3 $S/main3.c -L. -l1 -l2 -l3 $DYN
	STA="-static -nostdlib -fno-builtin $N/niji-crt1.o"
	gcc -O2 $STA $S/empty.c $N/libc.a -lgcc -o empty_static
	gcc -O2 $STA $S/shb.c $N/libc.a -lgcc -lgcc_eh -o shb_static
	gcc -O2 -fexceptions -o shb_exc $S/shb.c $DYN
	# negative control: the same static program with the three original .S files (no SSP handling)
	for f in setjmp vfork context; do gcc -c -DNIJI_EXPORT -o orig_$f.o $W/orig/$f.S; done
	gcc -O2 $STA $S/shb.c orig_setjmp.o orig_vfork.o orig_context.o $N/libc.a -lgcc -lgcc_eh -o shb_static_orig
	mkdir -p $O/niji
	cp -L $N/libc.so.6 $N/$LDN $O/niji/
	for f in libm.so.6 libpthread.so.0 libdl.so.2 librt.so.1 libgcc_s.so.1; do [ -e $N/$f ] && cp -L $N/$f $O/niji/ || true; done
fi
cd $O && tar czf $W/out.tgz g n $( [ -d niji ] && echo niji )
ls -la $W/out.tgz
