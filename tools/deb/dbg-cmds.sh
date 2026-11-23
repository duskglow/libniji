#!/bin/sh
# tools/deb/dbg-cmds.sh OUT FUNC: runs inside the swapped root of tools/deb/dbg-system.sh (copied to
# /tmp/dbg/cmds.sh), after libniji's libc6 and libc-bin replaced the distribution's. Writes into OUT:
#   before.out  gdb's `info line *FUNC` (the line of FUNC's entry address) on libc.so.6 before libc6-dbg
#   dbg.log/rc  dpkg -i of libc6-dbg
#   after.out   the same gdb command after it
#   run.out     gdb stopped at a breakpoint on FUNC in a running program (/usr/bin/true --version, which calls setlocale itself): `frame`,
#               `list`, `bt` and `info sharedlibrary` there
#   dlopen.out  gdb stopped at a pending breakpoint on boot_POSIX, in the library perl dlopens for `-MPOSIX`
#               (perl-base's POSIX.so): `bt` and `info sharedlibrary` there (the debugger rendezvous, DL-23)
#   gnat.out/rc when gnat-14 is installed (DBG_GNAT=1): gnat1's COPY relocation of _r_debug, `gnat1 --version`
#               and `gcc-14 -c` of a one-line Ada unit (GitHub issue #19)
#   info.txt    the SHA-256 of the root's libc.so.6 and loader (the swap is the package's)
# No spec clause: release packaging (SYS-DBG-PACKAGE, DL-23).
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
gdbq -ex "break $func" -ex run -ex frame -ex list -ex bt -ex "info sharedlibrary" --args /usr/bin/true --version > "$out/run.out"
gdbq -ex "set breakpoint pending on" -ex "break boot_POSIX" -ex run -ex bt -ex "info sharedlibrary" \
    --args /usr/bin/perl -MPOSIX -e 1 > "$out/dlopen.out"
gnat1=/usr/libexec/gcc/x86_64-linux-gnu/14/gnat1
if [ -x "$gnat1" ]; then
    {
        readelf -rW "$gnat1" | grep -E 'R_X86_64_COPY .*_r_debug'
        timeout 120 "$gnat1" --version < /dev/null
        echo "gnat1 rc=$?"
        mkdir -p /tmp/dbg/ada && cd /tmp/dbg/ada && printf 'procedure P is begin null; end P;\n' > p.adb &&
            timeout 300 gcc-14 -c p.adb && ls p.o p.ali
    } > "$out/gnat.out" 2>&1
    echo $? > "$out/gnat.rc"
fi
true
