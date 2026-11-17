#!/bin/sh
# tools/deb/root-system.sh DEBDIR: install the system packages (tools/deb/build.sh --flavour system)
# into an EMPTY root and run a hello world built against them inside it. Unprivileged: every step
# that needs root runs in a user namespace (unshare -r); nothing outside a temporary directory
# changes. No spec clause: release packaging (plan/system-libc.md section 3; SL-FLAVOUR).
#
# The root starts as the merged-/usr skeleton base-files lays down (bin, lib, lib64, sbin links into
# /usr) and an empty dpkg database, nothing else: no shell, no libraries. dpkg --root installs all
# seven packages, and empty stand-ins for linux-libc-dev, libcrypt-dev and rpcsvc-proto (libc6-dev's
# dependencies from other sources), with --force-script-chrootless (libc-bin's postinst honours DPKG_ROOT, since the root
# has no shell to run it). Then, in the root (chroot inside the namespace):
#   - every package is "install ok installed" (the ldconfig trigger was processed), the conffiles
#     are registered, /etc/nsswitch.conf came from the template, dpkg --verify is clean;
#   - a static hello built with the packaged libc6-dev runs;
#   - the dynamic hello (program interpreter /lib64/ld-linux-x86-64.so.2) starts the packaged
#     loader through the root's /lib64 link, and ld.so --list resolves libc.so.6 to
#     /usr/lib/x86_64-linux-gnu/libc.so.6;
#   - the packaged ldconfig -p lists libc.so.6 from the /etc/ld.so.cache libc-bin's postinst wrote
#     (configure; the ldconfig trigger after later library installs);
#   - the smoke programs hello, threads, devlinks (C; devlinks with -lanl -lBrokenLocale -lmcheck
#     -lg through libc6-dev's development links: SYS-DEV-LINKS) and except, iostream (C++; libstdc++ linked statically)
#     built dynamically with plain gcc/g++ run in the root and print their lines: the loader finds
#     libc.so.6 and libm.so.6 by soname at start (DL-START-SEARCH);
#   - iostream and throwmain (an exception thrown in the shared libthrowlib.so, caught in the
#     program) linked against the build system's shared libstdc++.so.6 and libgcc_s.so.1, copied
#     into the root for the run, start and print their lines (DL-LIBSTDCXX);
#   - libresolv (issue #5): resolv, linked with plain -lresolv, runs and prints its line; krb5, linked
#     with -lgssapi_krb5 -lkrb5 against the build system's Kerberos libraries (copied in like the C++
#     runtime; skipped with an INFO line when the build system has none), whose libkrb5.so.3 needs
#     libresolv.so.2: ld.so --list finds the root's libresolv.so.2 and it runs with LD_BIND_NOW;
#   - purge (libc-bin is Essential: --force-remove-essential) leaves the skeleton, the database and
#     /etc/nsswitch.conf (the administrator's file) and nothing else.
# Prints "root: PASS|FAIL|INFO ..." lines and "root: RESULT ..."; exit 1 when any is FAIL.
set -eu
export LC_ALL=C
[ $# -eq 1 ] || { echo "usage: tools/deb/root-system.sh DEBDIR" >&2; exit 2; }
debdir=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
ma=x86_64-linux-gnu
tmp=$(mktemp -d)
# The namespace's files are owned by the caller outside it; removal needs no privilege.
trap 'chmod -R u+rwX "$tmp" 2>/dev/null; rm -rf "$tmp"' EXIT
npass=0; nfail=0
pass() { echo "root: PASS $*"; npass=$((npass + 1)); }
fail() { echo "root: FAIL $*"; nfail=$((nfail + 1)); }
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
# libc6-dev depends on packages the distribution builds from other sources (kernel headers,
# libcrypt, rpcgen). The empty root has no archive: empty stand-ins of those names meet the
# relations, so dpkg configures libc6-dev without --force-depends.
standins="linux-libc-dev libcrypt-dev rpcsvc-proto"
for s in $standins; do
    mkdir -p "$tmp/standin/$s/DEBIAN"
    printf 'Package: %s\nVersion: 0~niji-standin\nArchitecture: all\nMaintainer: niji root check <root@localhost>\nDescription: empty stand-in (tools/deb/root-system.sh)\n' "$s" \
        > "$tmp/standin/$s/DEBIAN/control"
    dpkg-deb --root-owner-group --build "$tmp/standin/$s" "$tmp/standin/$s.deb" > /dev/null
    set -- "$@" "$tmp/standin/$s.deb"
done

dpkg_r() { ns dpkg --root="$r" --force-script-chrootless --log=/dev/null "$@"; }
if dpkg_r -i "$@" > "$tmp/install.log" 2>&1; then
    pass "dpkg --root installs the seven packages into the empty root"
else
    fail "dpkg --root -i: $(tail -5 "$tmp/install.log" | tr '\n' ' ')"
fi
q() { dpkg-query --admindir="$r/var/lib/dpkg" "$@"; }
bad=$(q -W -f '${Package} ${Status}\n' | grep -v ' install ok installed$' || true)
n=$(q -W -f '${Package} ${Status}\n' | grep -c ' install ok installed$' || true)
if [ -z "$bad" ] && [ "$n" -eq 10 ]; then
    pass "all seven packages (and the three stand-ins) are installed, no trigger pending"
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
"$cc" -O1 -nostartfiles -nodefaultlibs "-Wl,--sysroot=$r" -L"$lib" "$lib/Scrt1.o" "$b" "$here/smoke/threads.c" -lc -lgcc "$e" -o "$r/tmp/threads" > "$tmp/cc.log" 2>&1 ||
    fail "dynamic threads does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
# SYS-DEV-LINKS: -lanl -lBrokenLocale -lmcheck -lg through libc6-dev's development links and archives;
# the program needs libanl.so.1 and libBrokenLocale.so.1, whose MB_CUR_MAX override it prints.
"$cc" -O1 -nostartfiles -nodefaultlibs "-Wl,--sysroot=$r" -L"$lib" "$lib/Scrt1.o" "$b" "$here/smoke/devlinks.c" -Wl,--no-as-needed -lanl -lBrokenLocale -lmcheck -lg -lc -lgcc "$e" -o "$r/tmp/devlinks" > "$tmp/cc.log" 2>&1 ||
    fail "dynamic devlinks does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
# C++ as niji-g++ links it: libstdc++ and the unwinder static (the root has no libstdc++.so.6), the C
# library and libm.so.6 the root's.
cxx=${CXX:-g++}
for p in except iostream; do
    "$cxx" -O1 -nostartfiles -nodefaultlibs "-Wl,--sysroot=$r" -L"$lib" "$lib/Scrt1.o" "$b" "$here/smoke/$p.cc" \
        -Wl,-Bstatic -lstdc++ -Wl,-Bdynamic -lm -lc -lgcc -lgcc_eh "$e" -o "$r/tmp/$p" > "$tmp/cc.log" 2>&1 ||
        fail "dynamic $p does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
done
# C++ against the system's shared libstdc++.so.6 and libgcc_s.so.1 (the build machine's, copied into
# the root's library directory and removed before the purge): iostream again, and throwmain, which
# catches an exception thrown in the shared libthrowlib.so. libstdc++.so.6 defines about a hundred
# STB_GNU_UNIQUE objects, so this is the start-time loading of a shared C++ runtime.
sxx=$(readlink -f "$("$cxx" -print-file-name=libstdc++.so.6)")
sgcc=$(readlink -f "$("$cxx" -print-file-name=libgcc_s.so.1)")
cxxlibs=
if [ -f "$sxx" ] && [ -f "$sgcc" ]; then
    cp "$sxx" "$lib/libstdc++.so.6" && cp "$sgcc" "$lib/libgcc_s.so.1" && cxxlibs="libstdc++.so.6 libgcc_s.so.1"
    info "shared C++ runtime: $sxx $sgcc"
else
    fail "no shared libstdc++.so.6 ($sxx) or libgcc_s.so.1 ($sgcc) beside $cxx"
fi
# shared_cxx OUT SRC AFTER LINK...: a C++ object linked dynamically against the root's C library and
# the copied libstdc++.so.6 and libgcc_s.so.1 (by soname, found at start in the root's library
# directory). LINK... (start files, options) goes before SRC; AFTER (a shared library SRC uses, or
# empty) goes after it: a linker that defaults to --as-needed (Ubuntu's) drops a shared library that
# no object before it references, so a library named before the source is never recorded as needed.
shared_cxx() {
    o=$1; s=$2; a=$3; shift 3
    "$cxx" -O1 -nostdlib "-Wl,--sysroot=$r" -L"$lib" "$@" "$s" ${a:+"$a"} "$lib/libstdc++.so.6" -lm -lc "$lib/libgcc_s.so.1" -lgcc "$e" -o "$o" > "$tmp/cc.log" 2>&1 ||
        fail "$(basename "$o") does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
}
shared_cxx "$lib/libthrowlib.so" "$here/smoke/throwlib.cc" "" -fPIC -shared -Wl,-soname,libthrowlib.so "$b"
shared_cxx "$r/tmp/iostream-so" "$here/smoke/iostream.cc" "" "$lib/Scrt1.o" "$b"
shared_cxx "$r/tmp/throwmain" "$here/smoke/throwmain.cc" "$lib/libthrowlib.so" "$lib/Scrt1.o" "$b"
cxxlibs="$cxxlibs libthrowlib.so"
# libresolv (issue #5): smoke/resolv.c linked with plain -lresolv against the root's libc6-dev (needs
# libresolv.so.2), and smoke/krb5.c linked with -lgssapi_krb5 -lkrb5 against the build system's
# shared Kerberos libraries, whose libkrb5.so.3 needs libresolv.so.2 (ns_initparse, ns_parserr at
# GLIBC_2.9). The Kerberos libraries are copied into the root's library directory by soname and
# removed before the purge, as the C++ runtime above; the link itself resolves libkrb5.so.3's
# libresolv.so.2 symbols against the root's.
"$cc" -O1 -nostartfiles -nodefaultlibs "-Wl,--sysroot=$r" -L"$lib" "$lib/Scrt1.o" "$b" "$here/smoke/resolv.c" -lresolv -lc -lgcc "$e" -o "$r/tmp/resolv" > "$tmp/cc.log" 2>&1 ||
    fail "dynamic resolv (-lresolv) does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
krbdir=/usr/lib/$ma
krblibs=
for f in libgssapi_krb5.so.2 libkrb5.so.3 libk5crypto.so.3 libkrb5support.so.0 libcom_err.so.2 libkeyutils.so.1; do
    [ -f "$krbdir/$f" ] || { krblibs=; info "skip krb5: the build system has no $krbdir/$f"; break; }
    krblibs="$krblibs $f"
done
for f in $krblibs; do cp "$(readlink -f "$krbdir/$f")" "$lib/$f"; done
if [ -n "$krblibs" ]; then
    "$cc" -O1 -nostartfiles -nodefaultlibs "-Wl,--sysroot=$r" -L"$lib" "$lib/Scrt1.o" "$b" "$here/smoke/krb5.c" "$lib/libgssapi_krb5.so.2" "$lib/libkrb5.so.3" -lc -lgcc "$e" -o "$r/tmp/krb5" > "$tmp/cc.log" 2>&1 ||
        fail "krb5 (-lgssapi_krb5 -lkrb5) does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
fi
b=$("$cc" -print-file-name=crtbeginT.o); e=$("$cc" -print-file-name=crtend.o)
"$cc" -O1 -static -nostartfiles -nodefaultlibs -Wl,--eh-frame-hdr -L"$lib" "$lib/crt1.o" "$b" "$here/smoke/hello.c" -Wl,--start-group -lc -lgcc -lgcc_eh -Wl,--end-group "$e" -o "$r/tmp/hello-static" > "$tmp/cc.log" 2>&1 ||
    fail "static hello does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
info "hello's program interpreter: $(readelf -lW "$r/tmp/hello" 2>/dev/null | sed -n 's/.*interpreter: \(.*\)\]$/\1/p')"

# in_root CMD...: run CMD inside the root (60 s guard against a hang).
in_root() { ns timeout 60 chroot "$r" "$@"; }
out=$(in_root /tmp/hello-static 2>&1) && case $out in *"hello from niji"*) true ;; *) false ;; esac &&
    pass "static hello runs in the root: $out" || fail "static hello in the root: $out"
out=$(in_root /lib64/ld-linux-x86-64.so.2 --list /tmp/hello 2>&1) || true
case $out in
    # /lib is the merged-/usr link to /usr/lib: either spelling is the packaged file (the system
    # build's loader prints /lib/$ma, the directory its search list names).
    *"libc.so.6 => /lib/$ma/libc.so.6 "* | *"libc.so.6 => /usr/lib/$ma/libc.so.6 "*)
        pass "ld.so --list in the root resolves libc.so.6 to the packaged /usr/lib/$ma/libc.so.6" ;;
    *) fail "ld.so --list in the root: $(echo "$out" | tr '\n' ' ')" ;;
esac
out=$(in_root /usr/sbin/ldconfig -p 2>&1) || true
case $out in
    *"libc.so.6 (libc6,x86-64) => "*"/lib/$ma/libc.so.6"*) pass "libc-bin's postinst wrote /etc/ld.so.cache: ldconfig -p in the root lists libc.so.6" ;;
    *) fail "ldconfig -p in the root: $(echo "$out" | head -5 | tr '\n' ' ')" ;;
esac
# The seven dynamic programs: each starts the packaged loader through /lib64 (PT_INTERP), which finds
# libc.so.6 (and libm.so.6, libstdc++.so.6, libgcc_s.so.1, libthrowlib.so, libresolv.so.2,
# libanl.so.1, libBrokenLocale.so.1) by soname at start (DL-START-SEARCH), and prints its line.
for pe in "hello:hello from niji" "threads:threads: total 40000" "devlinks:devlinks: MB_CUR_MAX 4" "except:except: caught" "iostream:iostream: 42" \
    "iostream-so:iostream: 42" "throwmain:throwmain: caught: throwlib: value 3 (throwlib)" \
    "resolv:resolv: a.test A 192.0.2.1 ttl 3600; cut header -1 EMSGSIZE"; do
    p=${pe%%:*}; want=${pe#*:}
    out=$(in_root "/tmp/$p" 2>&1) && rc=0 || rc=$?
    case $rc:$out in
        0:*"$want"*) pass "dynamic $p runs in the root: $(echo "$out" | head -1)" ;;
        *) fail "dynamic $p in the root exits $rc: $(echo "$out" | head -3 | tr '\n' ' ')" ;;
    esac
done
# krb5: the loader finds libresolv.so.2 in the root for libkrb5.so.3, and with LD_BIND_NOW every
# symbol of every library (libkrb5.so.3's ns_initparse@GLIBC_2.9 among them) resolves before main.
if [ -n "$krblibs" ]; then
    out=$(in_root /lib64/ld-linux-x86-64.so.2 --list /tmp/krb5 2>&1) || true
    case $out in
        # /lib is the merged-/usr link to /usr/lib: either spelling is the packaged file.
        *"libresolv.so.2 => /lib/$ma/libresolv.so.2 "* | *"libresolv.so.2 => /usr/lib/$ma/libresolv.so.2 "*)
            pass "ld.so --list in the root resolves libkrb5.so.3's libresolv.so.2 to the packaged /usr/lib/$ma/libresolv.so.2" ;;
        *) fail "ld.so --list /tmp/krb5 in the root: $(echo "$out" | tr '\n' ' ')" ;;
    esac
    out=$(LD_BIND_NOW=1 in_root /tmp/krb5 2>&1) && rc=0 || rc=$?
    case $rc:$out in
        0:"krb5: started"*) pass "krb5 (libgssapi_krb5.so.2, libkrb5.so.3 -> libresolv.so.2) runs in the root with LD_BIND_NOW: $(echo "$out" | head -1)" ;;
        *) fail "krb5 in the root (LD_BIND_NOW) exits $rc: $(echo "$out" | head -3 | tr '\n' ' ')" ;;
    esac
fi
for f in $cxxlibs $krblibs; do rm -f "$lib/$f"; done

# Purge everything; only the skeleton, the database and the administrator's nsswitch.conf remain.
if dpkg_r --force-remove-essential --force-depends -P libc6-dbg libc6-dev $standins libc-dev-bin locales libc-l10n libc-bin libc6 > "$tmp/purge.log" 2>&1; then
    (cd "$r" && find . | sort) | grep -v -e '^\./var/lib/dpkg' -e '^\./tmp/' -e '^\./etc/nsswitch.conf$' > "$tmp/after"
    left=$(comm -13 "$tmp/before" "$tmp/after" | tr '\n' ' ')
    [ -z "$left" ] && pass "purge leaves the empty root (and /etc/nsswitch.conf)" || fail "left after purge: $left"
else
    fail "dpkg --root -P: $(tail -5 "$tmp/purge.log" | tr '\n' ' ')"
fi

echo "root: RESULT pass=$npass fail=$nfail"
[ "$nfail" -eq 0 ]
