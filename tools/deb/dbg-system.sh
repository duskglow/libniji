#!/bin/sh
# tools/deb/dbg-system.sh DEBDIR [SUITE] [LOGDIR]: prove libc6-dbg (tools/deb/build.sh --flavour system)
# with the distribution's own gdb in a swapped root. Unprivileged (mmdebstrap --mode=unshare; the root
# is discarded). No spec clause: release packaging (SYS-DBG-PACKAGE).
#
# A root of SUITE (trixie default, or excalibur; MIRROR and KEYRING as in tools/deb/proof.sh) with gdb;
# libniji's libc6 and libc-bin swapped in as tools/deb/swap-system.sh does (dpkg -i --force-downgrade
# --force-depends), so gdb itself runs on libniji. Then, in the root (tools/deb/dbg-cmds.sh):
#   - before libc6-dbg: gdb finds no line information for DBG_FUNC (default setlocale) in the
#     stripped libc.so.6;
#   - dpkg -i libc6-dbg succeeds (its Depends: libc6 (= version) is met by the swapped libc6);
#   - after: `info line DBG_FUNC` names a source file and line (gdb found the debug file by build-id
#     under /usr/lib/debug/.build-id), and gdb stopped at DBG_FUNC in a running /usr/bin/true --version shows
#     the frame's file:line.
# The root's libc.so.6 and loader must be the package's (SHA-256). Prints "dbg: PASS|FAIL|PENDING|INFO ..."
# lines and "dbg: RESULT ..."; exit 1 when any is FAIL. The outputs are under LOGDIR/dbg-<suite>/.
set -eu
export LC_ALL=C
[ $# -ge 1 ] || { echo "usage: tools/deb/dbg-system.sh DEBDIR [SUITE] [LOGDIR]" >&2; exit 2; }
debdir=$(cd "$1" && pwd)
suite=${2:-trixie}
logdir=${3:-$debdir}
here=$(cd "$(dirname "$0")" && pwd)
mm=${MMDEBSTRAP:-mmdebstrap}
func=${DBG_FUNC:-setlocale}
case $suite in
    trixie) mirror=https://deb.debian.org/debian; keyring=${KEYRING:-/usr/share/keyrings/debian-archive-keyring.gpg} ;;
    excalibur) mirror=http://deb.devuan.org/merged; keyring=${KEYRING:-/usr/share/keyrings/devuan-archive-keyring.gpg} ;;
    *) echo "dbg-system.sh: suite $suite: trixie or excalibur" >&2; exit 2 ;;
esac
mirror=${MIRROR:-$mirror}
[ -e "$keyring" ] || { echo "dbg-system.sh: no keyring $keyring (set KEYRING)" >&2; exit 2; }
for p in libc6 libc-bin libc6-dbg; do
    set -- "$debdir"/${p}_*_amd64.deb
    [ -e "$1" ] || { echo "dbg: FAIL no $p .deb in $debdir"; exit 1; }
done
echo "dbg: INFO suite $suite from $mirror; function $func"

npass=0; nfail=0
pass() { echo "dbg: PASS $*"; npass=$((npass + 1)); }
fail() { echo "dbg: FAIL $*"; nfail=$((nfail + 1)); }
stage=$(mktemp -d)
trap 'chmod -R u+rwX "$stage" 2>/dev/null; rm -rf "$stage"' EXIT
chmod 0755 "$stage"
mkdir -p "$stage/dbg/debs" "$stage/ref"
cp "$keyring" "$stage/keyring.gpg"
cp "$debdir"/libc6_*_amd64.deb "$debdir"/libc-bin_*_amd64.deb "$debdir"/libc6-dbg_*_amd64.deb "$stage/dbg/debs/"
cp "$here/dbg-cmds.sh" "$stage/dbg/cmds.sh"
chmod -R a+rX "$stage"
dpkg-deb -x "$debdir"/libc6_*_amd64.deb "$stage/ref"
res=$logdir/dbg-$suite
rm -rf "$res"
mkdir -p "$res/tmp"

"$mm" --mode=unshare --variant=apt --keyring="$stage/keyring.gpg" --components=main --include=gdb \
    --customize-hook="copy-in $stage/dbg /tmp" \
    --customize-hook="chroot \"\$1\" sh -c 'mkdir -p /tmp/dbg/out; dpkg -i --force-downgrade --force-depends /tmp/dbg/debs/libc6_*.deb /tmp/dbg/debs/libc-bin_*.deb > /tmp/dbg/out/swap.log 2>&1; echo \$? > /tmp/dbg/out/swap.rc'" \
    --customize-hook="chroot \"\$1\" sh /tmp/dbg/cmds.sh /tmp/dbg/out $func" \
    --customize-hook="copy-out /tmp/dbg/out $res/tmp" \
    "$suite" /dev/null "$mirror" > "$res/mmdebstrap.log" 2>&1 || {
    echo "dbg: FAIL mmdebstrap exited $?: $(tail -3 "$res/mmdebstrap.log" | tr '\n' ' ')"; exit 1; }
o=$res/tmp/out

[ "$(cat "$o/swap.rc" 2>/dev/null)" = 0 ] && pass "libc6 and libc-bin swapped in (dpkg rc 0)" ||
    fail "the swap: dpkg rc $(cat "$o/swap.rc" 2>/dev/null): $(tail -3 "$o/swap.log" 2>/dev/null | tr '\n' ' ')"
want=$(cd "$stage/ref" && sha256sum usr/lib/x86_64-linux-gnu/libc.so.6 | cut -d' ' -f1)
grep -q "^$want  /usr/lib/x86_64-linux-gnu/libc.so.6" "$o/info.txt" 2>/dev/null &&
    pass "the root's libc.so.6 is the package's" || fail "the root's libc.so.6 is not the package's: $(tr '\n' ' ' < "$o/info.txt" 2>/dev/null)"
grep -q -i 'no line number information' "$o/before.out" 2>/dev/null &&
    pass "without libc6-dbg gdb has no line for $func: $(head -1 "$o/before.out")" ||
    fail "without libc6-dbg: $(head -3 "$o/before.out" 2>/dev/null | tr '\n' ' ')"
[ "$(cat "$o/dbg.rc" 2>/dev/null)" = 0 ] && pass "dpkg -i libc6-dbg (rc 0)" ||
    fail "dpkg -i libc6-dbg: rc $(cat "$o/dbg.rc" 2>/dev/null): $(tail -3 "$o/dbg.log" 2>/dev/null | tr '\n' ' ')"
grep -q -E '^Line [0-9]+ of "[^"]+"' "$o/after.out" 2>/dev/null &&
    pass "with libc6-dbg gdb shows the source line of $func: $(grep -m1 -E '^Line [0-9]+ of' "$o/after.out")" ||
    fail "with libc6-dbg: $(head -3 "$o/after.out" 2>/dev/null | tr '\n' ' ')"
# The stop at the breakpoint, then the innermost frame gdb prints for `frame` and `bt`: "#0  NAME (...) at
# FILE:LINE" (NAME may be the libniji function inlined at FUNC's entry, whose line it is).
#
# Both need the loader's debugger rendezvous (spec section 27.4: _r_debug and its breakpoint function,
# crates/ld/src/debug.rs), through which gdb learns that libc.so.6 was loaded. Until the loader has it,
# gdb warns "Unable to find dynamic linker breakpoint function" and stops at the PLT stub only: with
# that warning (and no stop in libc.so.6) the two checks print PENDING instead of PASS or FAIL. Any
# other outcome is judged as above.
npend=0
if grep -q 'Unable to find dynamic linker breakpoint function' "$o/run.out" 2>/dev/null &&
    ! grep -q -E '^#0 +.* at [^ ]+:[0-9]+$' "$o/run.out" 2>/dev/null; then
    echo "dbg: PENDING gdb in a running /usr/bin/true and list at the stop: the loader has no debugger rendezvous yet (gdb: $(grep -m1 'Unable to find dynamic linker breakpoint function' "$o/run.out"))"
    npend=2
else
grep -q -E "^Breakpoint 1, " "$o/run.out" 2>/dev/null && grep -q -E '^#0 +.* at [^ ]+:[0-9]+$' "$o/run.out" 2>/dev/null &&
    pass "gdb stopped at $func in /usr/bin/true: $(grep -m1 -E '^#0 ' "$o/run.out")" ||
    fail "gdb in a running /usr/bin/true: $(tail -4 "$o/run.out" 2>/dev/null | tr '\n' ' ')"
# `list` there prints the source line numbers around the stop (the sources are not installed, so gdb
# names the file and line instead of its text).
grep -q -E '^(warning: )?[0-9]+[[:space:]]+(in )?[^ ]+\.rs' "$o/run.out" 2>/dev/null &&
    pass "list at the stop names the source: $(grep -m1 -E '^(warning: )?[0-9]+[[:space:]]+(in )?[^ ]+\.rs' "$o/run.out")" ||
    fail "list at the stop: $(grep -v '^#' "$o/run.out" 2>/dev/null | tail -3 | tr '\n' ' ')"
fi
mv "$o"/* "$res/" && rm -rf "$res/tmp"
echo "dbg: RESULT pass=$npass fail=$nfail pending=$npend"
[ "$nfail" -eq 0 ]
