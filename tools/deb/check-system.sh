#!/bin/sh
# tools/deb/check-system.sh DIR: check the system packages in DIR (tools/deb/build.sh --flavour system)
# without installing them. No spec clause: release packaging (plan/system-libc.md sections 3.1-3.3,
# owner rulings R1 and R2; records/pending/SL-FLAVOUR-1.toml).
#
# 1. Identity (R1, R2): exactly libc6, libc-bin, libc6-dev, libc-dev-bin, locales, libc-l10n, each
#    Source: niji, the project's Maintainer, one version <abi>+niji<YYYYMMDD>.<n>-<rev> (no
#    epoch); libc-bin Essential: yes, Priority: required, Multi-Arch: foreign; libc6 and libc6-dev
#    Multi-Arch: same; locales and libc-l10n Architecture: all. Control fields as the
#    distribution's packages of these names (SYS-CONTROL-PARITY, plan/package-parity-audit.md
#    rows 7, 8, 25): libc6-dev Depends also on linux-libc-dev, libcrypt-dev, rpcsvc-proto and
#    Provides: libc-dev (= <version>); libc-l10n Multi-Arch: foreign; Priority libc6, libc6-dev,
#    libc-dev-bin optional, locales and libc-l10n standard.
# 2. Layout (section 3.3): every path is in the package's list below (nothing in a sysroot, no
#    /lib64: base-files owns it); owners root:root; the control members, conffiles and triggers
#    are exactly the ones named below.
# 3. The products: the loader was built with prefix / (no sysroot path in it); each shared
#    library's SONAME is its file name; libc-bin's ldconfig is stripped (no .symtab, no .debug_*).
# 4. Linking: plain gcc with the unpacked libc6-dev (its start files and libc.so script, --sysroot
#    the unpacked tree) makes a dynamic hello whose program interpreter is
#    /lib64/ld-linux-x86-64.so.2 (gcc's default) and which needs libc.so.6 alone with no
#    RPATH/RUNPATH; a static hello runs and prints its line. The development links and archives
#    (SYS-DEV-LINKS): libanl.so, libBrokenLocale.so, libc_malloc_debug.so name libc6's libraries;
#    -lanl -lBrokenLocale -lmcheck -lg link dynamically (each shared library needed) and statically
#    (the program runs; libBrokenLocale.a's MB_CUR_MAX override is the one linked). libresolv (issue #5):
#    libc6-dev's libresolv.so links to libresolv.so.2; smoke/resolv.c linked with `-lresolv` needs
#    libresolv.so.2 and libc.so.6 and binds ns_initparse and ns_parserr at GLIBC_2.9 (as Debian's
#    libkrb5.so.3 does); linked statically with `-lresolv` (the empty libresolv.a, the functions in
#    libc.a: D-16.6) it runs and prints its line.
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

want="libc-bin libc-dev-bin libc-l10n libc6 libc6-dev locales"
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
            expect libc6 Architecture amd64; expect libc6 Multi-Arch same; expect libc6 Priority optional
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
            expect libc6-dev Multi-Arch same; expect libc6-dev Provides "libc-dev (= $v)"
            expect libc6-dev Priority optional
            expect libc6-dev Depends "libc6 (= $v), libc-dev-bin (= $v), linux-libc-dev, libcrypt-dev, rpcsvc-proto" ;;
        libc-dev-bin) expect libc-dev-bin Multi-Arch foreign; expect libc-dev-bin Priority optional ;;
        locales) expect locales Architecture all; expect locales Priority standard ;;
        libc-l10n)
            expect libc-l10n Architecture all; expect libc-l10n Priority standard
            expect libc-l10n Multi-Arch foreign ;;
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
            ./ | ./usr/ | ./usr/lib/ | ./usr/lib/$ma/ | ./usr/lib64/ | ./usr/bin/ | ./usr/sbin/ | ./usr/share/ | ./usr/share/doc/ | ./etc/ | ./etc/ld.so.conf.d/ | ./usr/share/libc-bin/)
                [ "$type" = d ] || echo "check: FAIL $pkg: $path is not a directory"
                continue ;;
            ./usr/share/doc/"$pkg"/ | ./usr/share/doc/"$pkg"/copyright | ./usr/share/doc/"$pkg"/changelog.Debian.gz) continue ;;
        esac
        ok=no
        case $pkg:$path in
            libc6:./usr/lib/$ma/ld-linux-x86-64.so.2 | libc6:./usr/lib/$ma/lib*.so.[0-9] | libc6:./usr/lib/$ma/libc_malloc_debug.so.0) ok=yes ;;
            libc6:./usr/lib64/ld-linux-x86-64.so.2 | libc6:./etc/ld.so.conf.d/$ma.conf) ok=yes ;;
            libc6:./usr/share/lintian/ | libc6:./usr/share/lintian/overrides/) [ "$type" = d ] && ok=yes ;;
            libc6:./usr/share/lintian/overrides/libc6) ok=yes ;;
            libc6:./usr/share/i18n/ | libc6:./usr/share/i18n/charmaps/) [ "$type" = d ] && ok=yes ;;
            libc6:./usr/share/i18n/charmaps/[A-Z]*) [ "$type" = - ] && ok=yes ;;
            libc-bin:./etc/ld.so.conf | libc-bin:./etc/ld.so.conf.d/libc.conf | libc-bin:./etc/gai.conf) ok=yes ;;
            libc-bin:./usr/share/libc-bin/nsswitch.conf | libc-bin:./usr/sbin/ldconfig | libc-bin:./usr/bin/getconf | libc-bin:./usr/bin/getent | libc-bin:./usr/bin/ldd) ok=yes ;;
            libc6-dev:./usr/lib/$ma/*crt1.o | libc6-dev:./usr/lib/$ma/crt[in].o | libc6-dev:./usr/lib/$ma/lib*.a | libc6-dev:./usr/lib/$ma/libc.so | libc6-dev:./usr/lib/$ma/libm.so | libc6-dev:./usr/lib/$ma/libresolv.so) ok=yes ;;
            libc6-dev:./usr/lib/$ma/libanl.so | libc6-dev:./usr/lib/$ma/libBrokenLocale.so | libc6-dev:./usr/lib/$ma/libc_malloc_debug.so) ok=yes ;;
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
for so in libc.so.6 libm.so.6 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1 libanl.so.1 libnsl.so.1 libc_malloc_debug.so.0 libresolv.so.2 libBrokenLocale.so.1; do
    sn=$(readelf -dW "$lib/$so" 2>/dev/null | sed -n 's/.*(SONAME).*\[\(.*\)\]$/\1/p')
    [ "$sn" = "$so" ] || fail "$so: SONAME '$sn'"
done
# SYS-ICONV-COVERAGE: libc6 carries one charmap per row of spec/encoding/charmaps.tsv, each naming
# itself in its header (iconv_open finds a charset by that name and its aliases).
want=$(awk -F'\t' '!/^#/ && NR > 1 && $1 != "name" { n++ } END { print n + 0 }' "$here/../../spec/encoding/charmaps.tsv")
got=$(find "$r/usr/share/i18n/charmaps" -type f 2>/dev/null | wc -l)
[ "$got" = "$want" ] || fail "libc6: $got charmaps in /usr/share/i18n/charmaps, not $want (spec/encoding/charmaps.tsv)"
grep -q '^<code_set_name> ISO-8859-15$' "$r/usr/share/i18n/charmaps/ISO-8859-15" 2>/dev/null ||
    fail "libc6: /usr/share/i18n/charmaps/ISO-8859-15 is missing or does not name ISO-8859-15"
# SYS-DEV-LINKS: libc6-dev's development links name libc6's libraries (the link resolves in the
# unpacked root), and the archives -l<x> needs in a static link are there.
for so in libanl.so.1 libBrokenLocale.so.1 libc_malloc_debug.so.0; do
    dl=${so%%.so.*}.so
    t=$(readlink "$lib/$dl" || true)
    [ "$t" = "$so" ] || fail "libc6-dev: $dl -> '$t', not $so"
    [ -f "$lib/$dl" ] || fail "libc6-dev: $dl does not resolve to a file"
done
for a in libanl.a libBrokenLocale.a libg.a libmcheck.a libpthread_nonshared.a; do
    [ -f "$lib/$a" ] || fail "libc6-dev has no $a"
done

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
echo "check: PASS (6 packages, version $version)"
