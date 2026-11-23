#!/bin/bash
# SHSTK-SCOPE: shadow stack off/on counts on the N100 host. One line per row:
#   ROW <impl> <arm> <name> <n> <reps> | perf -x, lines (instructions:u, instructions:k, page-faults, task-clock)
# impl n = libniji prototype (NJSHSTK= enables; NJSHSTX= is the same-length off switch),
# impl g = the host C library (glibc.cpu.hwcaps=SHSTK enables, =-SHSTK off).
# Dynamic rows run as `env <vars> prog` (libniji's libc is found through LD_LIBRARY_PATH, R14-3),
# so they include env's own cost: the row envbase (env <vars> <static empty>) minus the direct
# static empty measures it. Static rows run directly with the switch in the environment.
R=~/work/shstk-scope/run; D=$HOME/work/shstk-scope/niji
EV="instructions:u,instructions:k,page-faults,task-clock"
row() { # impl arm name n reps cmd...
	local i=$1 a=$2 nm=$3 n=$4 r=$5; shift 5
	local out; out=$(${PE:+env $PE} perf stat -r $r -x, -e $EV -- "$@" 2>&1 >/dev/null)
	echo "ROW $i $a $nm $n $r | $(echo "$out" | grep -E 'instructions|page-faults|task-clock' | cut -d, -f1,3 | tr '\n' ' ')"
	${PE:+env $PE} "$@" >/dev/null 2>&1; local rc=$?; [ $rc = 0 ] || echo "RC $i $a $nm $n rc=$rc"
}
for i in n g; do
	for a in off on; do
		if [ $i = n ]; then
			[ $a = on ] && V="NJSHSTK=1" || V="NJSHSTX=1"; VV="$V LD_LIBRARY_PATH=$D"
		else
			[ $a = on ] && V="GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK" || V="GLIBC_TUNABLES=glibc.cpu.hwcaps=-SHSTK"; VV="$V"
		fi
		cd $R/$i
		# start-up
		row $i $a envbase 0 50 env $VV $R/n/empty_static
		PE="$V" row $i $a empty_static 0 50 ./empty_static
		PE="$V" row $i $a envbase_direct 0 50 $R/n/empty_static
		row $i $a empty 0 50 env $VV ./empty
		row $i $a main3 0 50 env $VV ./main3
		# per-call rows: n = 0 and n = N
		for spec in "thread 2000" "pexit 2000" "cancel 2000" "longjmp 1000000" "deep 1000000" "swap 200000" \
			"signal 100000" "mapss 10000" "vfork 2000"; do
			set -- $spec
			row $i $a $1 0 5 env $VV ./shb $1 0
			row $i $a $1 $2 5 env $VV ./shb $1 $2
		done
		for spec in "pexit 2000" "cancel 2000"; do
			set -- $spec
			row $i $a exc_$1 0 5 env $VV ./shb_exc $1 0
			row $i $a exc_$1 $2 5 env $VV ./shb_exc $1 $2
		done
		PE="$V" row $i $a static_thread 0 5 ./shb_static thread 0
		PE="$V" row $i $a static_thread 2000 5 ./shb_static thread 2000
		row $i $a cxx 0 5 env $VV ./cxx 0
		row $i $a cxx 100000 5 env $VV ./cxx 100000
	done
done
# real programs through libniji's loader (program mode), off/on
cd $R; seq 200000 | shuf --random-source=/dev/zero > $R/sortin.txt 2>/dev/null || seq 200000 > $R/sortin.txt
for a in off on; do
	[ $a = on ] && V="NJSHSTK=1" || V="NJSHSTX=1"
	L="env $V LD_LIBRARY_PATH=$D $D/ld-linux-x86-64.so.2"
	row n $a prog_sort 0 5 $L /usr/bin/sort -n $R/sortin.txt
	row n $a prog_gzip 0 5 $L /usr/bin/gzip -c $R/sortin.txt
	row n $a prog_sed 0 5 $L /usr/bin/sed -e s/1/x/g $R/sortin.txt
	row n $a prog_python 0 5 $L /usr/bin/python3 -c "import json, threading; t = threading.Thread(target=print); t.start(); t.join()"
	row n $a prog_perl 0 5 $L /usr/bin/perl -e 'my %h; $h{$_} = $_ for 1..100000;'
	row n $a prog_find 0 5 $L /usr/bin/find /usr/share/doc -name "*.gz"
done
# syscall counts of the start-up rows (lines after the program's own execve)
for i in n g; do for a in off on; do
	if [ $i = n ]; then [ $a = on ] && V="NJSHSTK=1" || V="NJSHSTX=1"; VV="$V LD_LIBRARY_PATH=$D"
	else [ $a = on ] && V="GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK" || V="GLIBC_TUNABLES=glibc.cpu.hwcaps=-SHSTK"; VV="$V"; fi
	cd $R/$i
	for p in empty main3 empty_static; do
		strace -f -qq -o /tmp/shstk-scope-st.txt env $VV ./$p
		echo "SYSCALLS $i $a $p $(awk '/execve\(/{n=0; next} {n++} END{print n}' /tmp/shstk-scope-st.txt) $(grep -c 'arch_prctl(0x500\|ARCH_SHSTK\|map_shadow' /tmp/shstk-scope-st.txt) $(grep -o 'arch_prctl([^)]*' /tmp/shstk-scope-st.txt | tr '\n' ' ')"
	done
done; done
rm -f /tmp/shstk-scope-st.txt
echo DONE
