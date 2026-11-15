#!/bin/sh
# tools/deb/root-system.sh DEBDIR: install the system packages (tools/deb/build.sh --flavour system)
# into an EMPTY root and run a hello world built against them inside it. Unprivileged: every step
# that needs root runs in a user namespace (unshare -r); nothing outside a temporary directory
# changes. No spec clause: release packaging (plan/system-libc.md section 3; SL-FLAVOUR).
#
# The root starts as the merged-/usr skeleton base-files lays down (bin, lib, lib64, sbin links into
# /usr) and an empty dpkg database, nothing else: no shell, no libraries. dpkg --root installs all
# six packages with --force-script-chrootless (libc-bin's postinst honours DPKG_ROOT, since the root
# has no shell to run it). Then, in the root (chroot inside the namespace):
#   - every package is "install ok installed" (the ldconfig trigger was processed), the conffiles
#     are registered, /etc/nsswitch.conf came from the template, dpkg --verify is clean;
#   - a static hello built with the packaged libc6-dev runs;
#   - the dynamic hello (program interpreter /lib64/ld-linux-x86-64.so.2) starts the packaged
#     loader through the root's /lib64 link, and ld.so --list resolves libc.so.6 to
#     /usr/lib/x86_64-linux-gnu/libc.so.6;
#   - the packaged ldconfig -p lists libc.so.6 from the /etc/ld.so.cache the ldconfig trigger wrote;
#   - the dynamic hello runs: PASS when it prints its line; WAIT while the loader's start-time
#     soname search is not on the start path (story DL-8; the loader says so);
#   - purge (libc-bin is Essential: --force-remove-essential) leaves the skeleton, the database and
#     /etc/nsswitch.conf (the administrator's file) and nothing else.
# Prints "root: PASS|FAIL|WAIT|INFO ..." lines and "root: RESULT ..."; exit 1 when any is FAIL.
set -eu
export LC_ALL=C
[ $# -eq 1 ] || { echo "usage: tools/deb/root-system.sh DEBDIR" >&2; exit 2; }
debdir=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
ma=x86_64-linux-gnu
tmp=$(mktemp -d)
# The namespace's files are owned by the caller outside it; removal needs no privilege.
trap 'chmod -R u+rwX "$tmp" 2>/dev/null; rm -rf "$tmp"' EXIT
npass=0; nfail=0; nwait=0
pass() { echo "root: PASS $*"; npass=$((npass + 1)); }
fail() { echo "root: FAIL $*"; nfail=$((nfail + 1)); }
wait_() { echo "root: WAIT $*"; nwait=$((nwait + 1)); }
info() { echo "root: INFO $*"; }
# ns CMD...: run CMD as root of a new user namespace (the caller's uid mapped to 0).
ns() { unshare -r "$@"; }

r=$tmp/root
mkdir -p "$r/usr/bin" "$r/usr/lib" "$r/usr/lib64" "$r/usr/sbin" "$r/etc" "$r/tmp" "$r/var/lib/dpkg/info" "$r/var/lib/dpkg/updates" "$r/var/lib/dpkg/triggers"
for d in bin lib lib64 sbin; do ln -s "usr/$d" "$r/$d"; done
: > "$r/var/lib/dpkg/status"
: > "$r/var/lib/dpkg/available"
chmod 1777 "$r/tmp"
(cd "$r" && find . | sort) > "$tmp/before"
set -- "$debdir"/*.deb
[ -e "$1" ] || { echo "root: FAIL no .deb in $debdir"; exit 1; }
info "installing $(echo "$@" | tr ' ' '\n' | sed 's|.*/||' | tr '\n' ' ')into an empty root"

dpkg_r() { ns dpkg --root="$r" --force-script-chrootless --log=/dev/null "$@"; }
if dpkg_r -i "$@" > "$tmp/install.log" 2>&1; then
    pass "dpkg --root installs the six packages into the empty root"
else
    fail "dpkg --root -i: $(tail -5 "$tmp/install.log" | tr '\n' ' ')"
fi
q() { dpkg-query --admindir="$r/var/lib/dpkg" "$@"; }
bad=$(q -W -f '${Package} ${Status}\n' | grep -v ' install ok installed$' || true)
n=$(q -W -f '${Package} ${Status}\n' | grep -c ' install ok installed$' || true)
if [ -z "$bad" ] && [ "$n" -eq 6 ]; then
    pass "all six packages are installed, no trigger pending"
else
    fail "package states: $(q -W -f '${Package}=${Status}; ' 2>&1)"
fi
cf=$(q -W -f '${Conffiles}\n' libc6 libc-bin | awk 'NF { print $1 }' | sort | tr '\n' ' ')
[ "$cf" = "/etc/gai.conf /etc/ld.so.conf /etc/ld.so.conf.d/libc.conf /etc/ld.so.conf.d/$ma.conf " ] &&
    pass "conffiles registered: $cf" || fail "conffiles registered: '$cf'"
cmp -s "$r/etc/nsswitch.conf" "$r/usr/share/libc-bin/nsswitch.conf" &&
    pass "libc-bin's postinst installed /etc/nsswitch.conf from its template" || fail "no /etc/nsswitch.conf from the template"
v=$(ns dpkg --root="$r" --verify 2>&1 || true)
[ -z "$v" ] && pass "dpkg --verify is clean" || fail "dpkg --verify: $(echo "$v" | head -5 | tr '\n' ' ')"

# Hello world built with the system flavour (the root's own libc6-dev; gcc's default interpreter).
lib=$r/usr/lib/$ma
cc=${CC:-gcc}
b=$("$cc" -print-file-name=crtbeginS.o); e=$("$cc" -print-file-name=crtendS.o)
"$cc" -O1 -nostartfiles -nodefaultlibs "-Wl,--sysroot=$r" -L"$lib" "$lib/Scrt1.o" "$b" "$here/smoke/hello.c" -lc -lgcc "$e" -o "$r/tmp/hello" > "$tmp/cc.log" 2>&1 ||
    fail "dynamic hello does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
b=$("$cc" -print-file-name=crtbeginT.o); e=$("$cc" -print-file-name=crtend.o)
"$cc" -O1 -static -nostartfiles -nodefaultlibs -Wl,--eh-frame-hdr -L"$lib" "$lib/crt1.o" "$b" "$here/smoke/hello.c" -Wl,--start-group -lc -lgcc -lgcc_eh -Wl,--end-group "$e" -o "$r/tmp/hello-static" > "$tmp/cc.log" 2>&1 ||
    fail "static hello does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
info "hello's program interpreter: $(readelf -lW "$r/tmp/hello" 2>/dev/null | sed -n 's/.*interpreter: \(.*\)\]$/\1/p')"

# in_root CMD...: run CMD inside the root (60 s guard against a hang).
in_root() { ns timeout 60 chroot "$r" "$@"; }
out=$(in_root /tmp/hello-static 2>&1) && case $out in *"hello from liella"*) true ;; *) false ;; esac &&
    pass "static hello runs in the root: $out" || fail "static hello in the root: $out"
out=$(in_root /lib64/ld-linux-x86-64.so.2 --list /tmp/hello 2>&1) || true
case $out in
    *"libc.so.6 => /usr/lib/$ma/libc.so.6 "*) pass "ld.so --list in the root resolves libc.so.6 to /usr/lib/$ma/libc.so.6" ;;
    *) fail "ld.so --list in the root: $(echo "$out" | tr '\n' ' ')" ;;
esac
out=$(in_root /usr/sbin/ldconfig -p 2>&1) || true
case $out in
    *"libc.so.6 (libc6,x86-64) => "*"/lib/$ma/libc.so.6"*) pass "the ldconfig trigger wrote /etc/ld.so.cache: ldconfig -p in the root lists libc.so.6" ;;
    *) fail "ldconfig -p in the root: $(echo "$out" | head -5 | tr '\n' ' ')" ;;
esac
out=$(in_root /tmp/hello 2>&1) && rc=0 || rc=$?
case $rc:$out in
    0:*"hello from liella"*) pass "dynamic hello runs in the root: $out" ;;
    *"library search is not available"*) wait_ "dynamic hello: the packaged loader starts through /lib64 (PT_INTERP) and stops at the start-time soname search (DL-8): $out" ;;
    *) fail "dynamic hello in the root exits $rc: $out" ;;
esac

# Purge everything; only the skeleton, the database and the administrator's nsswitch.conf remain.
if dpkg_r --force-remove-essential --force-depends -P libc6-dev libc-dev-bin locales libc-l10n libc-bin libc6 > "$tmp/purge.log" 2>&1; then
    (cd "$r" && find . | sort) | grep -v -e '^\./var/lib/dpkg' -e '^\./tmp/hello' -e '^\./etc/nsswitch.conf$' > "$tmp/after"
    left=$(comm -13 "$tmp/before" "$tmp/after" | tr '\n' ' ')
    [ -z "$left" ] && pass "purge leaves the empty root (and /etc/nsswitch.conf)" || fail "left after purge: $left"
else
    fail "dpkg --root -P: $(tail -5 "$tmp/purge.log" | tr '\n' ' ')"
fi

echo "root: RESULT pass=$npass fail=$nfail wait=$nwait"
[ "$nfail" -eq 0 ]
