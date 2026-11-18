#!/bin/sh
# tools/deb/check-system.sh DIR: check the system packages in DIR (tools/deb/build.sh --flavour system)
# without installing them. No spec clause: release packaging (plan/system-libc.md sections 3.1-3.3,
# owner rulings R1 and R2; records/pending/SL-FLAVOUR-1.toml).
#
# 1. Contents (PKG-MANIFEST): `tools/deb/manifest.py check system DIR`: exactly the packages of
#    tools/deb/manifest.tsv's system and both rows; in each exactly the rows' paths and their
#    directories (owners root:root, types, modes, link targets, text contents, strip state), the
#    control members (conffiles, triggers, maintainer scripts, symbols and shlibs where
#    tools/deb/control.tsv names them; their contents: tools/deb/check-symbols.sh), the control
#    text from control.tsv and tools/deb/description/, Installed-Size and md5sums. libc6-dbg
#    holds one .build-id file per strip debug row. Identity (R1, R2): every package Source: niji,
#    the project's Maintainer, one version <abi>+niji<YYYYMMDD>.<n>-<rev> (no epoch).
# 2. The products: the loader was built with prefix / (no sysroot path in it); each shared
#    library's SONAME is its file name; each of libc6-dev's development links resolves to a file.
# 3. libc6-dbg (SYS-DBG-PACKAGE), below.
# 4. Linking: plain gcc with the unpacked libc6-dev (its start files and libc.so script, --sysroot
#    the unpacked tree) makes a dynamic hello whose program interpreter is
#    /lib64/ld-linux-x86-64.so.2 (gcc's default) and which needs libc.so.6 alone with no
#    RPATH/RUNPATH; a static hello runs and prints its line. The development links and archives
#    (SYS-DEV-LINKS): -lanl -lBrokenLocale -lmcheck -lg link dynamically (each shared library
#    needed) and statically (the program runs; libBrokenLocale.a's MB_CUR_MAX override is the one
#    linked). libresolv (issue #5): smoke/resolv.c linked with `-lresolv` needs libresolv.so.2 and
#    libc.so.6 and binds ns_initparse and ns_parserr at GLIBC_2.9 (as Debian's libkrb5.so.3 does);
#    linked statically with `-lresolv` (the empty libresolv.a, the functions in libc.a: D-16.6) it
#    runs and prints its line.
# Prints "check: PASS" or one "check: FAIL ..." line per problem (exit 1).
set -eu
export LC_ALL=C
[ $# -eq 1 ] || { echo "usage: tools/deb/check-system.sh DIR" >&2; exit 2; }
dir=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
ma=x86_64-linux-gnu
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fails=0
fail() {
    echo "check: FAIL $*"
    fails=$((fails + 1))
}

# field DEB NAME: one control field.
field() { dpkg-deb -f "$1" "$2"; }

debs=$(find "$dir" -maxdepth 1 -name '*.deb' | sort)
[ -n "$debs" ] || { echo "check: FAIL no .deb in $dir"; exit 1; }
# 1. The manifest's rows and control fields.
rc=0
python3 "$here/manifest.py" check system "$dir" > "$tmp/manifest" || rc=$?
n=$(grep -c '^check: FAIL' "$tmp/manifest" || true)
cat "$tmp/manifest"
fails=$((fails + n))
[ "$rc" -eq 0 ] || [ "$n" -ne 0 ] || fail "tools/deb/manifest.py check system exits $rc with no FAIL line"
python3 "$here/manifest.py" rows system > "$tmp/rows"
npkgs=$(python3 "$here/manifest.py" packages system | wc -w)
version=""
mkdir "$tmp/root"
for deb in $debs; do
    pkg=$(field "$deb" Package)
    v=$(field "$deb" Version)
    [ -n "$version" ] || version=$v
    [ "$v" = "$version" ] || fail "$pkg: version $v, the others $version"
    echo "$v" | grep -q -E '^2\.[0-9]+\+niji[0-9]{8}\.[0-9]+-[0-9]+$' || fail "$pkg: version $v is not <abi>+niji<YYYYMMDD>.<n>-<rev>"
    [ "$(field "$deb" Source)" = niji ] || fail "$pkg: Source is '$(field "$deb" Source)', not niji"
    [ "$(field "$deb" Maintainer)" = "${DEB_MAINTAINER:-Liella Maintainers <liella@koiren.ai>}" ] || fail "$pkg: Maintainer is '$(field "$deb" Maintainer)'"
    dpkg-deb -x "$deb" "$tmp/root"
done

r=$tmp/root
lib=$r/usr/lib/$ma
# base-files' merged-/usr links (not in these packages): the libc.so script names /lib and /lib64.
ln -s usr/lib "$r/lib"
ln -s usr/lib64 "$r/lib64"
# 2. The loader's prefix is / : a sysroot build names /usr/lib/niji (its own libraries and cache).
if [ -f "$lib/ld-linux-x86-64.so.2" ]; then
    ! grep -a -q '/usr/lib/niji' "$lib/ld-linux-x86-64.so.2" || fail "the loader was built with the sysroot prefix /usr/lib/niji, not /"
    grep -a -q "usr/lib/$ma" "$lib/ld-linux-x86-64.so.2" || fail "the loader does not name usr/lib/$ma"
fi
# Each shared library the manifest ships (a file row named lib*.so.<n>) has its file name as SONAME.
for so in $(awk -F '\t' '$3 == "file" { n = split($2, p, "/"); if (p[n] ~ /^lib.*\.so\.[0-9]+$/) print p[n] }' "$tmp/rows"); do
    sn=$(readelf -dW "$lib/$so" 2>/dev/null | sed -n 's/.*(SONAME).*\[\(.*\)\]$/\1/p')
    [ "$sn" = "$so" ] || fail "$so: SONAME '$sn'"
done
# SYS-DEV-LINKS: each of libc6-dev's links (manifest rows) resolves to a file in the unpacked root.
for dl in $(awk -F '\t' '$1 == "libc6-dev" && $3 == "link" { print $2 }' "$tmp/rows"); do
    [ -f "$r/$dl" ] || fail "libc6-dev: $dl does not resolve to a file"
done

# 3. libc6-dbg (SYS-DBG-PACKAGE): every strip debug row of the manifest (libc6's shared objects,
#     libc-bin's programs but iconv) is stripped (no symbol table) and keeps its GNU build-id
#     note; libc6-dbg holds exactly one
#     file per object, .build-id/<first byte>/<rest>.debug, whose own build-id is the object's and
#     which has the symbol table; libc.so.6's has line tables (.debug_line). gdb, given only the
#     debug directory, prints the source line of DBG_FUNC's entry address (`info line *getenv` by
#     default; the address form, because line tables carry no function entries to look a name up in).
build_id() { readelf -nW "$1" 2>/dev/null | sed -n 's/.*Build ID: \([0-9a-f]*\)$/\1/p' | head -1; }
dbgroot=$r/usr/lib/debug
: > "$tmp/ids"
for f in $(awk -F '\t' -v r="$r" '$6 == "debug" { print r "/" $2 }' "$tmp/rows"); do
    [ -f "$f" ] || continue
    name=${f#"$r"}
    id=$(build_id "$f")
    if [ "${#id}" -lt 4 ]; then
        fail "$name has no GNU build-id note"
        continue
    fi
    ! readelf -SW "$f" | grep -q -E ' \.(symtab|debug_[a-z_]+) ' || fail "$name is not stripped (it has a symbol table or debug sections)"
    d=.build-id/$(printf '%.2s' "$id")/${id#??}.debug
    echo "$d" >> "$tmp/ids"
    if [ ! -f "$dbgroot/$d" ]; then
        fail "libc6-dbg has no $d for $name"
        continue
    fi
    [ "$(build_id "$dbgroot/$d")" = "$id" ] || fail "libc6-dbg's $d has the build-id '$(build_id "$dbgroot/$d")', not $name's $id"
    readelf -SW "$dbgroot/$d" | grep -q ' \.symtab ' || fail "libc6-dbg's $d (for $name) has no symbol table"
    case $name in
        */libc.so.6) readelf -SW "$dbgroot/$d" | grep -q ' \.debug_line ' || fail "libc6-dbg's $d (for libc.so.6) has no line tables (.debug_line)" ;;
    esac
done
if [ -d "$dbgroot" ]; then
    (cd "$dbgroot" && find . -type f | sed 's|^\./||' | sort) > "$tmp/dbgfiles"
    extra=$(sort "$tmp/ids" | comm -13 - "$tmp/dbgfiles" | tr '\n' ' ')
    [ -z "$extra" ] || fail "libc6-dbg has debug files no packaged object names: $extra"
else
    fail "libc6-dbg ships no /usr/lib/debug"
fi
func=${DBG_FUNC:-getenv}
if command -v gdb > /dev/null; then
    out=$(timeout 120 gdb -batch -nx -iex "set debug-file-directory $dbgroot" -ex "info line *$func" "$lib/libc.so.6" 2>&1 || true)
    case $out in
        *"Line "[0-9]*" of \""*) ;;
        *) fail "gdb with libc6-dbg's directory shows no source line for $func in libc.so.6: $(echo "$out" | tail -2 | tr '\n' ' ')" ;;
    esac
else
    echo "check: INFO no gdb on this host: the source-line check of libc6-dbg was not run"
fi

# 4. Linking with plain gcc and the unpacked libc6-dev: its start files in the driver's order
#    (crt1, crti, crtbegin ... crtend, crtn), named by path so none comes from the host's libc6-dev.
cc=${CC:-gcc}
begin=$("$cc" -print-file-name=crtbeginS.o); end=$("$cc" -print-file-name=crtendS.o)
if "$cc" -O1 -nostartfiles -nodefaultlibs "-Wl,--sysroot=$r" -L"$lib" "$lib/Scrt1.o" "$lib/crti.o" "$begin" "$here/smoke/hello.c" -lc -lgcc "$end" "$lib/crtn.o" -o "$tmp/hello" > "$tmp/log" 2>&1; then
    interp=$(readelf -lW "$tmp/hello" | sed -n 's/.*Requesting program interpreter: \(.*\)\]$/\1/p')
    [ "$interp" = /lib64/ld-linux-x86-64.so.2 ] || fail "hello: program interpreter is '${interp:-none}', not /lib64/ld-linux-x86-64.so.2"
    needed=$(readelf -dW "$tmp/hello" | sed -n 's/.*(NEEDED).*\[\(.*\)\]$/\1/p' | tr '\n' ' ')
    [ "$needed" = "libc.so.6 " ] || fail "hello: needs '$needed', not libc.so.6 alone"
    ! readelf -dW "$tmp/hello" | grep -q -E '\((RPATH|RUNPATH)\)' || fail "hello has an RPATH or RUNPATH"
else
    fail "dynamic hello does not link: $(tail -3 "$tmp/log" | tr '\n' ' ')"
fi
begin=$("$cc" -print-file-name=crtbeginT.o); end=$("$cc" -print-file-name=crtend.o)
if "$cc" -O1 -static -nostartfiles -nodefaultlibs -Wl,--eh-frame-hdr -L"$lib" "$lib/crt1.o" "$lib/crti.o" "$begin" "$here/smoke/hello.c" -Wl,--start-group -lc -lgcc -lgcc_eh -Wl,--end-group "$end" "$lib/crtn.o" -o "$tmp/hello-static" > "$tmp/log" 2>&1; then
    out=$(timeout 60 "$tmp/hello-static" 2>&1) || fail "static hello exits $?: $out"
    case $out in *"hello from niji"*) ;; *) fail "static hello printed '$out'" ;; esac
else
    fail "static hello does not link: $(tail -3 "$tmp/log" | tr '\n' ' ')"
fi
# SYS-DEV-LINKS: -lanl -lBrokenLocale -lmcheck -lg (and -lc_malloc_debug, dynamic only) link. Dynamic
# (--no-as-needed, as a linker without that default records every -l): the links take the shared
# libraries, so each is needed. Static: the archives resolve, and libBrokenLocale.a's override is
# the one linked (MB_CUR_MAX 4 in C.UTF-8; without -lBrokenLocale the C library's 6, the control).
dl="-lanl -lBrokenLocale -lmcheck -lg"
begin=$("$cc" -print-file-name=crtbeginS.o); end=$("$cc" -print-file-name=crtendS.o)
if "$cc" -O1 -nostartfiles -nodefaultlibs "-Wl,--sysroot=$r" -L"$lib" "$lib/Scrt1.o" "$lib/crti.o" "$begin" "$here/smoke/devlinks.c" -Wl,--no-as-needed $dl -lc_malloc_debug -lc -lgcc "$end" "$lib/crtn.o" -o "$tmp/devlinks" > "$tmp/log" 2>&1; then
    needed=$(readelf -dW "$tmp/devlinks" | sed -n 's/.*(NEEDED).*\[\(.*\)\]$/\1/p' | tr '\n' ' ')
    [ "$needed" = "libanl.so.1 libBrokenLocale.so.1 libc_malloc_debug.so.0 libc.so.6 " ] ||
        fail "devlinks ($dl -lc_malloc_debug, dynamic) needs '$needed', not libanl.so.1 libBrokenLocale.so.1 libc_malloc_debug.so.0 libc.so.6"
else
    fail "devlinks ($dl -lc_malloc_debug) does not link dynamically: $(tail -3 "$tmp/log" | tr '\n' ' ')"
fi
begin=$("$cc" -print-file-name=crtbeginT.o); end=$("$cc" -print-file-name=crtend.o)
for v in "devlinks-static:$dl:4" "devlinks-control:-lanl -lmcheck -lg:6"; do
    p=${v%%:*}; libs=${v#*:}; libs=${libs%:*}; want=${v##*:}
    if "$cc" -O1 -static -nostartfiles -nodefaultlibs -Wl,--eh-frame-hdr -L"$lib" "$lib/crt1.o" "$lib/crti.o" "$begin" "$here/smoke/devlinks.c" $libs -Wl,--start-group -lc -lgcc -lgcc_eh -Wl,--end-group "$end" "$lib/crtn.o" -o "$tmp/$p" > "$tmp/log" 2>&1; then
        out=$(timeout 60 "$tmp/$p" 2>&1) || fail "$p ($libs, static) exits $?: $out"
        [ "$out" = "devlinks: MB_CUR_MAX $want" ] || fail "$p ($libs, static) printed '$out', not 'devlinks: MB_CUR_MAX $want'"
    else
        fail "$p ($libs) does not link statically: $(tail -3 "$tmp/log" | tr '\n' ' ')"
    fi
done
# libresolv (issue #5): the linker name, a dynamic -lresolv link, a static one that runs.
want_resolv="resolv: a.test A 192.0.2.1 ttl 3600; cut header -1 EMSGSIZE"
link=$(readlink "$lib/libresolv.so" || true)
[ "$link" = libresolv.so.2 ] || fail "libc6-dev's libresolv.so -> '$link', not libresolv.so.2"
begin=$("$cc" -print-file-name=crtbeginS.o); end=$("$cc" -print-file-name=crtendS.o)
if "$cc" -O1 -nostartfiles -nodefaultlibs "-Wl,--sysroot=$r" -L"$lib" "$lib/Scrt1.o" "$lib/crti.o" "$begin" "$here/smoke/resolv.c" -lresolv -lc -lgcc "$end" "$lib/crtn.o" -o "$tmp/resolv" > "$tmp/log" 2>&1; then
    needed=$(readelf -dW "$tmp/resolv" | sed -n 's/.*(NEEDED).*\[\(.*\)\]$/\1/p' | sort | tr '\n' ' ')
    [ "$needed" = "libc.so.6 libresolv.so.2 " ] || fail "resolv (-lresolv): needs '$needed', not libc.so.6 and libresolv.so.2"
    syms=$(readelf --dyn-syms -W "$tmp/resolv")
    for s in ns_initparse@GLIBC_2.9 ns_parserr@GLIBC_2.9; do
        echo "$syms" | grep -q " $s" || fail "resolv (-lresolv) does not bind $s"
    done
else
    fail "resolv (-lresolv) does not link: $(tail -3 "$tmp/log" | tr '\n' ' ')"
fi
begin=$("$cc" -print-file-name=crtbeginT.o); end=$("$cc" -print-file-name=crtend.o)
if "$cc" -O1 -static -nostartfiles -nodefaultlibs -Wl,--eh-frame-hdr -L"$lib" "$lib/crt1.o" "$lib/crti.o" "$begin" "$here/smoke/resolv.c" -lresolv -Wl,--start-group -lc -lgcc -lgcc_eh -Wl,--end-group "$end" "$lib/crtn.o" -o "$tmp/resolv-static" > "$tmp/log" 2>&1; then
    out=$(timeout 60 "$tmp/resolv-static" 2>&1) || fail "static resolv exits $?: $out"
    [ "$out" = "$want_resolv" ] || fail "static resolv (-lresolv) printed '$out', not '$want_resolv'"
else
    fail "static resolv (-lresolv) does not link: $(tail -3 "$tmp/log" | tr '\n' ' ')"
fi

if [ "$fails" -ne 0 ]; then
    echo "check: $fails problem(s)"
    exit 1
fi
echo "check: PASS ($npkgs packages, version $version)"
