#!/bin/sh
# tools/deb/check-symbols.sh DIR: check libc6's symbols and shlibs control members in the system
# packages in DIR (tools/deb/build.sh --flavour system; generator tools/deb/symbols.py). No spec
# clause: release packaging (plan/system-libc.md section 3.2, owner ruling R2; deb-symbols(5),
# deb-shlibs(5), dpkg-gensymbols(1), dpkg-shlibdeps(1) are the specification).
#
# 1. The rules, line by line: one `<soname> libc6 #MINVER#` block per shared library libc6 ships;
#    no GLIBC_PRIVATE anywhere (R14-12); each name@GLIBC_x has the minimum version x when x is at or
#    below the packages' ABI level (the version's part before "+niji"), else the build's upstream
#    version; each name@NIJI_<KIND>_<n> names the template `| niji-<kind>-abi (>= <n>)`;
#    NIJI_PRIVATE only as the `(symver)` pattern; libc6 Provides each such package at the newest
#    level the file names (today niji-native-abi and niji-owner-abi); any other node fails.
#    shlibs: `<name> <n> libc6 (>= <abi>)` per library, nothing else.
# 2. dpkg-gensymbols -c4 against the packaged libraries: no symbol or library missing or extra.
# 3. dpkg-shlibdeps on six sample programs linked with plain gcc against the unpacked libc6 and
#    libc6-dev (smoke/sym-base.c, smoke/hello.c, smoke/sym-native.c, smoke/sym-owner.c,
#    smoke/sym-resolv.c, smoke/sym-relr.c) gives exactly:
#      sym-base   libc6 (>= 2.2.5)                           (GLIBC_2.2.5 symbols only)
#      hello      libc6 (>= 2.34)                            (the start files' __libc_start_main@GLIBC_2.34)
#      sym-native libc6 (>= 2.34), niji-native-abi (>= 1)  (niji_heap_alloc@NIJI_NATIVE_1)
#      sym-owner  libc6 (>= 2.34), niji-owner-abi (>= 1)   (atexit@NIJI_OWNER_1, D-27.29)
#      sym-resolv libc6 (>= 2.9)                             (-lresolv: ns_initparse@GLIBC_2.9 from
#                                                             libresolv.so.2, issue #5)
#      sym-relr   libc6 (>= 2.2.5)                           (sym-base with -z pack-relative-relocs: it
#                                                             has DT_RELR and requires GLIBC_ABI_DT_RELR,
#                                                             which dpkg-shlibdeps does not turn into a
#                                                             dependency: it reads imported symbols only)
# 4. Version ordering (dpkg --compare-versions): <abi> < the packages' version < the next ABI level.
# Prints "check-symbols: PASS" or one "check-symbols: FAIL ..." line per problem (exit 1).
set -eu
export LC_ALL=C
[ $# -eq 1 ] || { echo "usage: tools/deb/check-symbols.sh DIR" >&2; exit 2; }
dir=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
ma=x86_64-linux-gnu
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fails=0
fail() {
    echo "check-symbols: FAIL $*"
    fails=$((fails + 1))
}

deb6=$(find "$dir" -maxdepth 1 -name 'libc6_*_amd64.deb' | head -1)
debdev=$(find "$dir" -maxdepth 1 -name 'libc6-dev_*_amd64.deb' | head -1)
[ -n "$deb6" ] && [ -n "$debdev" ] || { echo "check-symbols: FAIL no libc6 and libc6-dev .deb in $dir"; exit 1; }
version=$(dpkg-deb -f "$deb6" Version)
upstream=${version%-*}
abi=${version%%+niji*}
pkg=$tmp/pkg/debian/libc6
mkdir -p "$tmp/pkg/debian" "$tmp/root"
dpkg-deb -R "$deb6" "$pkg"
dpkg-deb -x "$deb6" "$tmp/root"
dpkg-deb -x "$debdev" "$tmp/root"
sym=$pkg/DEBIAN/symbols
shl=$pkg/DEBIAN/shlibs
[ -s "$sym" ] || { echo "check-symbols: FAIL libc6 has no symbols file"; exit 1; }
[ -s "$shl" ] || fail "libc6 has no shlibs file"

# 1. The rules.
lib=$pkg/usr/lib/$ma
libs=$(for f in "$lib"/*.so*; do
    [ -L "$f" ] || readelf -dW "$f" | sed -n 's/.*(SONAME).*\[\(.*\)\]$/\1/p'
done | sort | tr '\n' ' ')
blocks=$(grep -v '^[ |*#]' "$sym" | awk '{ print $1 }' | sort | tr '\n' ' ')
[ "$blocks" = "$libs" ] || fail "symbols blocks '$blocks', not the libraries '$libs'"
! grep -n 'GLIBC_PRIVATE' "$sym" > "$tmp/priv" || fail "GLIBC_PRIVATE in symbols: $(head -1 "$tmp/priv")"
named=$(awk -F'\t' 'NR > 1 && $2 !~ /^GLIBC_[0-9.]+$/ { print $2 }' "$here/../../spec/abi/empty-versions.tsv" | sort -u | tr '\n' ' ')
awk -v abi="$abi" -v up="$upstream" -v named="$named" '
    function le(a, b,   x, y, n, m, i) {   # dotted numbers: a <= b
        n = split(a, x, "."); m = split(b, y, ".")
        for (i = 1; i <= (n > m ? n : m); i++) {
            if ((x[i] + 0) < (y[i] + 0)) return 1
            if ((x[i] + 0) > (y[i] + 0)) return 0
        }
        return 1
    }
    function bad(why) { printf "check-symbols: FAIL symbols line %d (%s): %s\n", NR, $0, why; nbad++ }
    /^[^ |*#]/ {
        if (NF != 3 || $2 != "libc6" || $3 != "#MINVER#") bad("not <soname> libc6 #MINVER#")
        nt = 0; delete tmpl; next
    }
    /^\| / {
        nt++
        if (match($0, /^\| niji-[a-z]+-abi \(>= [0-9]+\)$/)) tmpl[nt] = $2 " " substr($4, 1, length($4) - 1)
        else bad("an alternative template other than niji-<kind>-abi (>= <n>)")
        next
    }
    /^\* / { next }
    /^ \(symver\)/ { if ($1 != "(symver)NIJI_PRIVATE" || $2 != up || NF != 2) bad("a pattern other than (symver)NIJI_PRIVATE " up); next }
    /^ / {
        at = index($1, "@"); node = substr($1, at + 1)
        if (at == 0) { bad("no version node"); next }
        if (node ~ /^GLIBC_[0-9.]+$/) {
            lvl = substr(node, 7); want = le(lvl, abi) ? lvl : up
            if ($2 != want) bad("minimum version " $2 ", not " want)
            if (NF != 2) bad("a GLIBC node with a template")
        } else if (index(" " named " ", " " node " ") && $1 == node "@" node) {
            # a named node no symbol carries (spec/abi/empty-versions.tsv): the build upstream version
            if ($2 != up || NF != 2) bad("not " node "@" node " " up)
        } else if (node ~ /^NIJI_[A-Z]+_[0-9]+$/) {
            kind = node; sub(/^NIJI_/, "", kind); n = kind; sub(/_[0-9]+$/, "", kind); sub(/^[A-Z]+_/, "", n)
            pkg = "niji-" tolower(kind) "-abi"
            if (NF != 3 || tmpl[$3] != pkg " " n) bad("not on the template " pkg " (>= " n ")")
            if (n + 0 > top[pkg] + 0) top[pkg] = n
        } else bad("node " node)
        nsym++
        next
    }
    { bad("not a deb-symbols(5) line") }
    END {
        for (p in top) print "provides " p " " top[p] > "/dev/stderr"
        if (nsym < 1000) { print "check-symbols: FAIL only " nsym + 0 " symbol lines"; nbad++ }
        exit nbad > 0
    }
' "$sym" 2> "$tmp/provides" > "$tmp/rules" || true
if [ -s "$tmp/rules" ]; then
    head -20 "$tmp/rules"
    fails=$((fails + $(wc -l < "$tmp/rules")))
fi
# libc6 Provides each virtual package a template names, at the newest level the file names.
want_prov=$(sed -n 's/^provides //p' "$tmp/provides" | sort | awk '{ printf "%s%s (= %s)", s, $1, $2; s = ", " }')
provides=$(dpkg-deb -f "$deb6" Provides)
[ "$provides" = "$want_prov" ] || fail "libc6 Provides '$provides', not '$want_prov'"
case $want_prov in *niji-native-abi*niji-owner-abi*) ;; *) fail "the templates name '$want_prov', not niji-native-abi and niji-owner-abi" ;; esac
want_shl=$(for so in $libs; do echo "${so%.so.*} ${so##*.so.} libc6 (>= $abi)"; done)
[ "$(cat "$shl" 2>/dev/null)" = "$want_shl" ] || fail "shlibs is '$(tr '\n' ';' < "$shl" 2>/dev/null)', not '$(echo "$want_shl" | tr '\n' ';')'"

# 2. dpkg-gensymbols -c4: the file against the libraries.
printf 'Source: niji\n\nPackage: libc6\nArchitecture: amd64\n' > "$tmp/pkg/debian/control"
if ! (cd "$tmp/pkg" && dpkg-gensymbols -plibc6 -Pdebian/libc6 -v"$version" -c4 -I"$sym" -O"$tmp/gen.symbols") > "$tmp/gen.log" 2>&1; then
    fail "dpkg-gensymbols -c4: $(grep -m 8 -E '^[-+][^-+]|warning|error' "$tmp/gen.log" | tr '\n' ' ')"
fi

# 3. dpkg-shlibdeps on the three samples, linked against the unpacked packages.
r=$tmp/root
ln -s usr/lib "$r/lib"
ln -s usr/lib64 "$r/lib64"
dl=$r/usr/lib/$ma
# dpkg-shlibdeps takes a binary's package build directory to be the nearest parent with DEBIAN/.
mkdir -p "$tmp/pkg/debian/samples/usr/bin" "$tmp/pkg/debian/samples/DEBIAN"
cc=${CC:-gcc}
begin=$("$cc" -print-file-name=crtbeginS.o); end=$("$cc" -print-file-name=crtendS.o)
# sample NAME EXPECTED CC-ARGS...: link, then compare dpkg-shlibdeps's shlibs:Depends.
sample() {
    name=$1; expected=$2; shift 2
    if ! "$cc" -O1 -fno-stack-protector "-Wl,--sysroot=$r" -L"$dl" "$@" -o "$tmp/pkg/debian/samples/usr/bin/$name" > "$tmp/cc.log" 2>&1; then
        fail "$name does not link: $(tail -3 "$tmp/cc.log" | tr '\n' ' ')"
        return
    fi
    if ! got=$(cd "$tmp/pkg" && dpkg-shlibdeps -O -S"$pkg" "debian/samples/usr/bin/$name" 2> "$tmp/sd.log"); then
        fail "dpkg-shlibdeps $name: $(tail -3 "$tmp/sd.log" | tr '\n' ' ')"
        return
    fi
    [ ! -s "$tmp/sd.log" ] || fail "dpkg-shlibdeps $name warns: $(head -3 "$tmp/sd.log" | tr '\n' ' ')"
    [ "$got" = "shlibs:Depends=$expected" ] || fail "$name: '$got', not 'shlibs:Depends=$expected'"
    echo "check-symbols: dpkg-shlibdeps $name: $got"
}
sample sym-base "libc6 (>= 2.2.5)" -nostartfiles -nodefaultlibs "$here/smoke/sym-base.c" -lc
sample hello "libc6 (>= 2.34)" -nostartfiles -nodefaultlibs "$dl/Scrt1.o" "$begin" "$here/smoke/hello.c" -lc -lgcc "$end"
sample sym-native "libc6 (>= 2.34), niji-native-abi (>= 1)" -nostartfiles -nodefaultlibs "$dl/Scrt1.o" "$begin" "$here/smoke/sym-native.c" -lc -lgcc "$end"
sample sym-owner "libc6 (>= 2.34), niji-owner-abi (>= 1)" -nostartfiles -nodefaultlibs "$dl/Scrt1.o" "$begin" "$here/smoke/sym-owner.c" -lc -lgcc "$end"
sample sym-resolv "libc6 (>= 2.9)" -nostartfiles -nodefaultlibs "$here/smoke/sym-resolv.c" -lresolv -lc
sample sym-relr "libc6 (>= 2.2.5)" -nostartfiles -nodefaultlibs -fPIE -pie -Wl,-z,pack-relative-relocs "$here/smoke/sym-relr.c" -lc
relr=$tmp/pkg/debian/samples/usr/bin/sym-relr
if [ -e "$relr" ]; then
    readelf -d -W "$relr" | grep -q '(RELR)' || fail "sym-relr has no DT_RELR"
    readelf -V -W "$relr" | grep -q 'Name: GLIBC_ABI_DT_RELR ' \
        || fail "sym-relr records no GLIBC_ABI_DT_RELR requirement: the packaged libc.so.6 does not define the node"
fi

# 4. Version ordering (R2): the ABI level < this build < the next level.
next=${abi%.*}.$((${abi##*.} + 1))
dpkg --compare-versions "$abi" lt "$version" || fail "$version does not sort above $abi"
dpkg --compare-versions "$abi-9" lt "$version" || fail "$version does not sort above $abi-9"
dpkg --compare-versions "$version" lt "$next" || fail "$version does not sort below $next"
dpkg --compare-versions "$version" lt "$next~" || fail "$version does not sort below $next~"

if [ "$fails" -ne 0 ]; then
    echo "check-symbols: $fails problem(s)"
    exit 1
fi
echo "check-symbols: PASS ($(grep -c '^ ' "$sym") symbol lines, $(echo "$libs" | wc -w) libraries, version $version)"
