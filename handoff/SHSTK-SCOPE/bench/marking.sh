#!/bin/bash
# SHSTK-SCOPE: CET marking of the installed ELF objects (programs and shared libraries).
# One line per object: <class> <path>, class = shstk | noshstk (property note without SHSTK) | noprop.
out=~/work/shstk-scope/marking.txt
: > $out
for f in /usr/bin/* /usr/sbin/* /usr/lib/x86_64-linux-gnu/*.so* /usr/lib/x86_64-linux-gnu/*/*.so* /usr/lib/python3*/lib-dynload/*.so /usr/libexec/*/* ; do
	[ -f "$f" ] && [ ! -L "$f" ] || continue
	head -c 4 "$f" 2>/dev/null | grep -q "ELF" || continue
	n=$(readelf -n "$f" 2>/dev/null | grep -a "x86 feature:" | head -1)
	if [ -z "$n" ]; then c=noprop; elif echo "$n" | grep -q SHSTK; then c=shstk; else c=noshstk; fi
	echo "$c $f" >> $out
done
awk '{n[$1]++} END{for (k in n) print k, n[k]}' $out
echo "unmarked libraries (sample):"; grep -v "^shstk" $out | grep "\.so" | head -40
echo "unmarked programs (sample):"; grep -v "^shstk" $out | grep -v "\.so" | head -40
