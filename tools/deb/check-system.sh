#!/bin/sh
# tools/deb/check-system.sh DIR: check the system packages in DIR (tools/deb/build.sh --flavour system)
# without installing them. No spec clause: release packaging (plan/system-libc.md sections 3.1-3.3,
# owner rulings R1 and R2; records/pending/SL-FLAVOUR-1.toml).
#
# 1. Identity (R1, R2): exactly libc6, libc-bin, libc6-dev, libc-dev-bin, locales, libc-l10n, libc6-dbg, each
#    Source: niji, the project's Maintainer, one version <abi>+niji<YYYYMMDD>.<n>-<rev> (no
#    epoch); libc-bin Essential: yes, Priority: required, Multi-Arch: foreign; libc6 and libc6-dev
#    Multi-Arch: same; libc6-dev Provides: libc-dev; locales and libc-l10n Architecture: all;
#    libc6-dbg Section: debug, Multi-Arch: same, Depends: libc6 (= version), Provides: libc-dbg,
#    libc6-dbgsym (SYS-DBG-PACKAGE).
# 2. Layout (section 3.3): every path is in the package's list below (nothing in a sysroot, no
#    /lib64: base-files owns it); owners root:root; the control members, conffiles and triggers
#    are exactly the ones named below.
# 3. The products: the loader was built with prefix / (no sysroot path in it); each shared
#    library's SONAME is its file name; libc-bin's ldconfig is stripped (no .symtab, no .debug_*).
# 4. Linking: plain gcc with the unpacked libc6-dev (its start files and libc.so script, --sysroot
#    the unpacked tree) makes a dynamic hello whose program interpreter is
#    /lib64/ld-linux-x86-64.so.2 (gcc's default) and which needs libc.so.6 alone with no
#    RPATH/RUNPATH; a static hello runs and prints its line.
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

want="libc-bin libc-dev-bin libc-l10n libc6 libc6-dbg libc6-dev locales"
debs=$(find "$dir" -maxdepth 1 -name '*.deb' | sort)
[ -n "$debs" ] || { echo "check: FAIL no .deb in $dir"; exit 1; }
got=$(for d in $debs; do field "$d" Package; done | sort | tr '\n' ' ')
[ "$got" = "$want " ] || fail "packages are '$got', not '$want'"
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
    # expect PKG FIELD VALUE
    expect() { [ "$(field "$deb" "$2")" = "$3" ] || fail "$1: $2 is '$(field "$deb" "$2")', not '$3'"; }
    ctrl_want="./ ./control ./md5sums"
    case $pkg in
        libc6)
            expect libc6 Architecture amd64; expect libc6 Multi-Arch same; expect libc6 Priority required
            expect libc6 Essential no
            # The symbols and shlibs members' contents: tools/deb/check-symbols.sh.
            ctrl_want="$ctrl_want ./conffiles ./shlibs ./symbols ./triggers"
            conffiles="/etc/ld.so.conf.d/$ma.conf"
            triggers="activate-noawait ldconfig" ;;
        libc-bin)
            expect libc-bin Essential yes; expect libc-bin Priority required; expect libc-bin Multi-Arch foreign
            expect libc-bin Depends "libc6 (= $v)"
            ctrl_want="$ctrl_want ./conffiles ./postinst ./postrm ./triggers"
            conffiles="/etc/gai.conf /etc/ld.so.conf /etc/ld.so.conf.d/libc.conf"
            triggers="interest-await ldconfig" ;;
        libc6-dev)
            expect libc6-dev Multi-Arch same; expect libc6-dev Provides libc-dev
            expect libc6-dev Depends "libc6 (= $v), libc-dev-bin (= $v)" ;;
        libc-dev-bin) expect libc-dev-bin Multi-Arch foreign ;;
        locales | libc-l10n) expect "$pkg" Architecture all ;;
        libc6-dbg)
            expect libc6-dbg Architecture amd64; expect libc6-dbg Multi-Arch same; expect libc6-dbg Section debug
            expect libc6-dbg Priority optional; expect libc6-dbg Depends "libc6 (= $v)"
            expect libc6-dbg Provides "libc-dbg, libc6-dbgsym" ;;
    esac
    dpkg-deb --ctrl-tarfile "$deb" | tar -t | sort | tr '\n' ' ' > "$tmp/ctrl"
    [ "$(cat "$tmp/ctrl")" = "$(echo "$ctrl_want" | tr ' ' '\n' | sort | tr '\n' ' ')" ] ||
        fail "$pkg: control members '$(cat "$tmp/ctrl")', not '$ctrl_want'"
    dpkg-deb -e "$deb" "$tmp/ctrl-$pkg"
    case $pkg in
        libc6 | libc-bin)
            [ "$(sort "$tmp/ctrl-$pkg/conffiles" | tr '\n' ' ')" = "$conffiles " ] ||
                fail "$pkg: conffiles '$(tr '\n' ' ' < "$tmp/ctrl-$pkg/conffiles")', not '$conffiles'"
            [ "$(cat "$tmp/ctrl-$pkg/triggers")" = "$triggers" ] || fail "$pkg: triggers '$(cat "$tmp/ctrl-$pkg/triggers")', not '$triggers'" ;;
    esac

    dpkg-deb --fsys-tarfile "$deb" | tar -tv > "$tmp/list"
    awk '$2 != "root/root" { print $0 }' "$tmp/list" > "$tmp/owners"
    [ ! -s "$tmp/owners" ] || fail "$pkg: entries not owned by root/root: $(head -1 "$tmp/owners")"
    awk '{ print substr($1, 1, 1), $6 }' "$tmp/list" | while read -r type path; do
        case $path in
            ./ | ./usr/ | ./usr/lib/ | ./usr/lib/debug/ | ./usr/lib/debug/.build-id/ | ./usr/lib/$ma/ | ./usr/lib64/ | ./usr/bin/ | ./usr/sbin/ | ./usr/share/ | ./usr/share/doc/ | ./etc/ | ./etc/ld.so.conf.d/ | ./usr/share/libc-bin/)
                [ "$type" = d ] || echo "check: FAIL $pkg: $path is not a directory"
                continue ;;
            ./usr/share/doc/"$pkg"/ | ./usr/share/doc/"$pkg"/copyright | ./usr/share/doc/"$pkg"/changelog.Debian.gz) continue ;;
        esac
        ok=no
        case $pkg:$path in
            libc6:./usr/lib/$ma/ld-linux-x86-64.so.2 | libc6:./usr/lib/$ma/lib*.so.[0-9] | libc6:./usr/lib/$ma/libc_malloc_debug.so.0) ok=yes ;;
            libc6:./usr/lib64/ld-linux-x86-64.so.2 | libc6:./etc/ld.so.conf.d/$ma.conf) ok=yes ;;
            libc-bin:./etc/ld.so.conf | libc-bin:./etc/ld.so.conf.d/libc.conf | libc-bin:./etc/gai.conf) ok=yes ;;
            libc-bin:./usr/share/libc-bin/nsswitch.conf | libc-bin:./usr/sbin/ldconfig | libc-bin:./usr/bin/getconf | libc-bin:./usr/bin/getent | libc-bin:./usr/bin/ldd) ok=yes ;;
            libc6-dbg:./usr/lib/debug/.build-id/??/ | libc6-dbg:./usr/lib/debug/.build-id/??/*.debug) ok=yes ;;
            libc6-dev:./usr/lib/$ma/*crt1.o | libc6-dev:./usr/lib/$ma/crt[in].o | libc6-dev:./usr/lib/$ma/lib*.a | libc6-dev:./usr/lib/$ma/libc.so | libc6-dev:./usr/lib/$ma/libm.so) ok=yes ;;
        esac
        [ $ok = yes ] || echo "check: FAIL $pkg: path not in the system layout: $path"
    done > "$tmp/paths"
    if [ -s "$tmp/paths" ]; then
        cat "$tmp/paths"
        fails=$((fails + $(wc -l < "$tmp/paths")))
    fi
    dpkg-deb -x "$deb" "$tmp/root"
done

r=$tmp/root
lib=$r/usr/lib/$ma
# base-files' merged-/usr links (not in these packages): the libc.so script names /lib and /lib64.
ln -s usr/lib "$r/lib"
ln -s usr/lib64 "$r/lib64"
link=$(readlink "$r/usr/lib64/ld-linux-x86-64.so.2" || true)
[ "$link" = "../lib/$ma/ld-linux-x86-64.so.2" ] || fail "/usr/lib64/ld-linux-x86-64.so.2 -> '$link', not ../lib/$ma/ld-linux-x86-64.so.2"
# 3. The loader's prefix is / : a sysroot build names /usr/lib/niji (its own libraries and cache).
if [ -f "$lib/ld-linux-x86-64.so.2" ]; then
    ! grep -a -q '/usr/lib/niji' "$lib/ld-linux-x86-64.so.2" || fail "the loader was built with the sysroot prefix /usr/lib/niji, not /"
    grep -a -q "usr/lib/$ma" "$lib/ld-linux-x86-64.so.2" || fail "the loader does not name usr/lib/$ma"
fi
# libc-bin's ldconfig is stripped: no symbol table and no debug sections.
if [ -f "$r/usr/sbin/ldconfig" ]; then
    ! readelf -SW "$r/usr/sbin/ldconfig" | grep -q -E ' \.(symtab|debug_[a-z_]+) ' ||
        fail "usr/sbin/ldconfig is not stripped (it has a symbol table or debug sections)"
else
    fail "libc-bin has no usr/sbin/ldconfig"
fi
for so in libc.so.6 libm.so.6 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1 libanl.so.1 libnsl.so.1 libc_malloc_debug.so.0; do
    sn=$(readelf -dW "$lib/$so" 2>/dev/null | sed -n 's/.*(SONAME).*\[\(.*\)\]$/\1/p')
    [ "$sn" = "$so" ] || fail "$so: SONAME '$sn'"
done

# 3b. libc6-dbg (SYS-DBG-PACKAGE): every shared object of libc6 and every program of libc-bin is
#     stripped (no symbol table) and keeps its GNU build-id note; libc6-dbg holds exactly one
#     file per object, .build-id/<first byte>/<rest>.debug, whose own build-id is the object's and
#     which has the symbol table; libc.so.6's has line tables (.debug_line). gdb, given only the
#     debug directory, prints the source line of DBG_FUNC (default getenv) in libc.so.6.
build_id() { readelf -nW "$1" 2>/dev/null | sed -n 's/.*Build ID: \([0-9a-f]*\)$/\1/p' | head -1; }
dbgroot=$r/usr/lib/debug
: > "$tmp/ids"
for f in $(find "$lib" -type f -name '*.so.*' | sort) "$r/usr/sbin/ldconfig" "$r/usr/bin/getconf" "$r/usr/bin/getent" "$r/usr/bin/ldd"; do
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
    out=$(timeout 120 gdb -batch -nx -ex "set debug-file-directory $dbgroot" -ex "info line $func" "$lib/libc.so.6" 2>&1 || true)
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

if [ "$fails" -ne 0 ]; then
    echo "check: $fails problem(s)"
    exit 1
fi
echo "check: PASS (7 packages, version $version)"
