#!/bin/sh
# tools/deb/dbg-cmds.sh OUT FUNC: runs inside the swapped root of tools/deb/dbg-system.sh (copied to
# /tmp/dbg/cmds.sh), after libniji's libc6 and libc-bin replaced the distribution's. Writes into OUT:
#   before.out  gdb's `info line *FUNC` (the line of FUNC's entry address) on libc.so.6 before libc6-dbg
#   dbg.log/rc  dpkg -i of libc6-dbg
#   after.out   the same gdb command after it
#   run.out     gdb stopped at a breakpoint on FUNC in a running program (/usr/bin/true --version, which calls setlocale itself): `frame`,
#               `list` and `bt` there
#   info.txt    the SHA-256 of the root's libc.so.6 and loader (the swap is the package's)
# No spec clause: release packaging (SYS-DBG-PACKAGE).
out=$1
func=$2
lib=/usr/lib/x86_64-linux-gnu
mkdir -p "$out"
gdbq() { timeout 300 gdb -batch -nx "$@" 2>&1; }
sha256sum "$lib/libc.so.6" "$lib/ld-linux-x86-64.so.2" > "$out/info.txt"
gdbq -ex "info line *$func" "$lib/libc.so.6" > "$out/before.out"
dpkg -i /tmp/dbg/debs/libc6-dbg_*.deb > "$out/dbg.log" 2>&1
echo $? > "$out/dbg.rc"
gdbq -ex "info line *$func" "$lib/libc.so.6" > "$out/after.out"
gdbq -ex "break $func" -ex run -ex frame -ex list -ex bt --args /usr/bin/true --version > "$out/run.out"
true
