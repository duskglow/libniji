#!/bin/sh
# tools/deb/lint.sh DEBDIR SUITE [LOGDIR]: run lintian --pedantic on the .debs in DEBDIR inside an
# unprivileged chroot of SUITE (mmdebstrap --mode=unshare, discarded at the end; nothing is
# installed from DEBDIR, lintian only reads the files). For build hosts without lintian. No spec
# clause: release packaging.
#
# SUITE: trixie or excalibur; MIRROR and KEYRING as in tools/deb/proof.sh (the index must be signed
# by a key in KEYRING). Prints "lint: TAG ..." lines (lintian's own lines), the suite's base-files
# version, and "lint: RESULT errors=E warnings=W"; exit 1 when lintian could not run.
set -eu
[ $# -ge 2 ] || { echo "usage: tools/deb/lint.sh DEBDIR SUITE [LOGDIR]" >&2; exit 2; }
debdir=$(cd "$1" && pwd)
suite=$2
logdir=${3:-$debdir}
mm=${MMDEBSTRAP:-mmdebstrap}
case $suite in
    trixie) mirror=https://deb.debian.org/debian; keyring=${KEYRING:-/usr/share/keyrings/debian-archive-keyring.gpg} ;;
    excalibur) mirror=http://deb.devuan.org/merged; keyring=${KEYRING:-/usr/share/keyrings/devuan-archive-keyring.gpg} ;;
    *) echo "lint.sh: suite $suite: trixie or excalibur" >&2; exit 2 ;;
esac
mirror=${MIRROR:-$mirror}
[ -e "$keyring" ] || { echo "lint.sh: no keyring $keyring" >&2; exit 2; }
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
chmod 0755 "$stage"
mkdir -p "$stage/lint/debs" "$logdir"
cp "$keyring" "$stage/keyring.gpg"
cp "$debdir"/*.deb "$stage/lint/debs/"
# What runs in the chroot (single quotes: nothing expands here).
printf '%s\n' 'set -e' \
    'echo "lint: INFO base-files $(dpkg-query -W -f="\${Version}" base-files), lintian $(dpkg-query -W -f="\${Version}" lintian)"' \
    'cd /tmp/lint/debs' \
    'lintian --pedantic --no-tag-display-limit ./*.deb | sed "s/^/lint: /" || true' \
    'echo "lint: DONE"' > "$stage/lint/inside.sh"
chmod -R a+rX "$stage"
log=$logdir/lint-$suite.log
echo "lint: INFO suite $suite from $mirror"
"$mm" --mode=unshare --variant=apt --keyring="$stage/keyring.gpg" --components=main --include=lintian \
    --customize-hook="copy-in $stage/lint /tmp" \
    --customize-hook='chroot "$1" sh /tmp/lint/inside.sh' \
    "$suite" /dev/null "$mirror" > "$log" 2>&1 || echo "lint: FAIL mmdebstrap exited $? (see $log)" >> "$log"
grep '^lint:' "$log" || true
grep -q '^lint: DONE' "$log" || exit 1
echo "lint: RESULT errors=$(grep -c '^lint: E:' "$log" || true) warnings=$(grep -c '^lint: W:' "$log" || true)"
