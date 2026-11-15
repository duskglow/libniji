#!/bin/sh
# tools/deb/swap-system.sh DEBDIR [SUITE] [LOGDIR]: swap Liella's system packages (tools/deb/build.sh
# --flavour system) into a Debian root in place of the distribution's C library, run the distribution's
# own programs there (tools/deb/swap-cmds.sh: ls, sort, grep, sed, awk, bash, perl, python3, find on a
# fixed input) and compare each one's output and exit status with the same command in a stock root of
# the same suite. Unprivileged (mmdebstrap --mode=unshare; both roots are discarded). No spec clause:
# release validation (plan/system-libc.md section 3.7).
#
# SUITE: trixie (default) or excalibur. MIRROR and KEYRING as in tools/deb/proof.sh (the index must be
# signed by a key in KEYRING; never trusted=yes).
#
# The swap: Liella's libc6 and libc-bin replace the distribution's with `dpkg -i --force-downgrade
# --force-depends`. Liella's version (2.39+liella...) sorts below the suite's libc6, so this is the
# pinned downgrade of plan section 3.7; --force-depends because packages the base set holds (util-linux)
# depend on a newer libc6 level than 2.39 (plan section 4). The swap is proved by the root's libc.so.6
# and loader being byte-identical to the ones in DEBDIR's libc6.
#
# Prints "swap: PASS|FAIL|INFO ..." lines and "swap: RESULT same=N differ=M"; exit 1 when the swap
# failed or any command differs (the differences are under LOGDIR/swap-<suite>/).
set -eu
export LC_ALL=C
[ $# -ge 1 ] || { echo "usage: tools/deb/swap-system.sh DEBDIR [SUITE] [LOGDIR]" >&2; exit 2; }
debdir=$(cd "$1" && pwd)
suite=${2:-trixie}
logdir=${3:-$debdir}
here=$(cd "$(dirname "$0")" && pwd)
mm=${MMDEBSTRAP:-mmdebstrap}
case $suite in
    trixie) mirror=https://deb.debian.org/debian; keyring=${KEYRING:-/usr/share/keyrings/debian-archive-keyring.gpg} ;;
    excalibur) mirror=http://deb.devuan.org/merged; keyring=${KEYRING:-/usr/share/keyrings/devuan-archive-keyring.gpg} ;;
    *) echo "swap-system.sh: suite $suite: trixie or excalibur" >&2; exit 2 ;;
esac
mirror=${MIRROR:-$mirror}
[ -e "$keyring" ] || { echo "swap-system.sh: no keyring $keyring (set KEYRING)" >&2; exit 2; }
set -- "$debdir"/libc6_*_amd64.deb
[ -e "$1" ] || { echo "swap: FAIL no libc6 .deb in $debdir"; exit 1; }
libc6=$1
set -- "$debdir"/libc-bin_*_amd64.deb
[ -e "$1" ] || { echo "swap: FAIL no libc-bin .deb in $debdir"; exit 1; }
libcbin=$1
echo "swap: INFO suite $suite from $mirror; swapping in $(basename "$libc6") $(basename "$libcbin")"

stage=$(mktemp -d)
trap 'chmod -R u+rwX "$stage" 2>/dev/null; rm -rf "$stage"' EXIT
chmod 0755 "$stage"
mkdir -p "$stage/swap/debs" "$stage/ref"
cp "$keyring" "$stage/keyring.gpg"
cp "$libc6" "$libcbin" "$stage/swap/debs/"
cp "$here/swap-cmds.sh" "$stage/swap/cmds.sh"
chmod -R a+rX "$stage"
dpkg-deb -x "$libc6" "$stage/ref"
res=$logdir/swap-$suite
rm -rf "$res"
mkdir -p "$res"

# The programs of the comparison: the Essential set (bash, coreutils, findutils, grep, sed, perl-base,
# dash), mawk, python3-minimal and fakeroot (a preloaded library: LD_PRELOAD and RTLD_NEXT). Same
# package set in both roots.
mmroot() { # mmroot MODE HOOK...: build a root, run the hooks, copy /tmp/swap/out out to $res/MODE.
    mode=$1; shift
    mkdir -p "$res/$mode-tmp"
    "$mm" --mode=unshare --variant=apt --keyring="$stage/keyring.gpg" --components=main \
        --include=mawk,python3-minimal,fakeroot \
        --customize-hook="copy-in $stage/swap /tmp" \
        "$@" \
        --customize-hook="chroot \"\$1\" sh /tmp/swap/cmds.sh /tmp/swap/out" \
        --customize-hook="copy-out /tmp/swap/out $res/$mode-tmp" \
        "$suite" /dev/null "$mirror" > "$res/mmdebstrap-$mode.log" 2>&1 || {
        echo "swap: FAIL mmdebstrap ($mode root) exited $?: $(tail -3 "$res/mmdebstrap-$mode.log" | tr '\n' ' ')"; return 1; }
    mv "$res/$mode-tmp/out" "$res/$mode"
    rmdir "$res/$mode-tmp"
}
mmroot stock || exit 1
# The swap's own output stays in the root's /tmp/swap/out (dpkg.log, dpkg.rc) and is checked below.
mmroot liella \
    --customize-hook="chroot \"\$1\" sh -c 'mkdir -p /tmp/swap/out; dpkg -i --force-downgrade --force-depends /tmp/swap/debs/*.deb > /tmp/swap/out/dpkg.log 2>&1; echo \$? > /tmp/swap/out/dpkg.rc; dpkg --audit >> /tmp/swap/out/dpkg.log 2>&1; true'" ||
    exit 1

nsame=0; ndiff=0; bad=0
s=$res/stock; l=$res/liella
if [ "$(cat "$l/dpkg.rc" 2>/dev/null)" = 0 ]; then
    echo "swap: PASS dpkg -i --force-downgrade --force-depends swaps libc6 and libc-bin in (rc 0)"
else
    echo "swap: FAIL the swap: dpkg rc $(cat "$l/dpkg.rc" 2>/dev/null): $(tail -5 "$l/dpkg.log" 2>/dev/null | tr '\n' ' ')"; bad=1
fi
echo "swap: INFO dpkg notes: $(grep -i -E 'warning|depend|break' "$l/dpkg.log" | sort -u | head -6 | tr '\n' ' ')"
want=$(cd "$stage/ref" && sha256sum usr/lib/x86_64-linux-gnu/libc.so.6 | cut -d' ' -f1)
wantld=$(cd "$stage/ref" && sha256sum usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 | cut -d' ' -f1)
if grep -q "^$want  /usr/lib/x86_64-linux-gnu/libc.so.6" "$l/info.txt" && grep -q "^$wantld  /usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2" "$l/info.txt"; then
    echo "swap: PASS the swapped root's libc.so.6 and ld-linux-x86-64.so.2 are the package's ($(head -1 "$l/info.txt"))"
else
    echo "swap: FAIL the swapped root does not run the package's libc.so.6 and loader: $(tr '\n' ' ' < "$l/info.txt")"; bad=1
fi
echo "swap: INFO stock root: $(head -1 "$s/info.txt")"
for f in "$s"/*.rc; do
    id=$(basename "$f" .rc)
    if [ -f "$l/$id.rc" ] && cmp -s "$f" "$l/$id.rc" && cmp -s "$s/$id.out" "$l/$id.out"; then
        echo "swap: PASS $id: same output and status ($(cat "$f"))"; nsame=$((nsame + 1))
    else
        diff "$s/$id.out" "$l/$id.out" > "$res/$id.diff" 2>&1 || true
        echo "swap: FAIL $id: status $(cat "$f") vs $(cat "$l/$id.rc" 2>/dev/null || echo none); $(head -6 "$res/$id.diff" | tr '\n' ' ')"
        ndiff=$((ndiff + 1))
    fi
done
[ "$nsame" -gt 0 ] || { echo "swap: FAIL no command ran"; bad=1; }
for id in fakeroot-1 fakeroot-2; do
    echo "swap: INFO $id in the Liella root: $(tr '\n' ' ' < "$l/$id.out" 2>/dev/null)"
done
echo "swap: RESULT same=$nsame differ=$ndiff"
[ "$bad" -eq 0 ] && [ "$ndiff" -eq 0 ]
