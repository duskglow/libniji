#!/bin/sh
# validation/smoke/smoke.sh: build real programs from their Debian source packages against libniji, run their
# own test suites, and run the same suites on a build against the host's C library (the reference).
# See validation/smoke/README.md. Results: <work>/results.tsv and <work>/results/<program>.<side>.*.
#
#   validation/smoke/smoke.sh [--work DIR] [--products DIR] [--jobs N] [--timeout SECS]
#                             [--mirror URL --keyring FILE] [PROGRAM...]
#   validation/smoke/smoke.sh --list
#
# Default PROGRAMs: all of programs.sh, in the order of SMOKE_PROGRAMS below. libniji's products are read from
# --products (default target/niji; build them first with `NIJI_FULL_PROFILE=1 cargo xtask so`). Sources
# come from one Debian mirror (default https://deb.debian.org/debian, suite trixie), through a private apt
# configuration in <work>/apt; apt checks the suite's signature and every file's hash. The trust anchor is
# --keyring FILE (a pinned Debian archive keyring) or, without it, the debian-archive-keyring package
# fetched over https from the mirror; a mirror that is not https needs --keyring (the transport is then
# not trusted, only the signature). Nothing needs root, nothing is installed.
#
# Exit status: 0 when every requested program was processed (failures of a program's tests are results, not
# errors), 1 when a libniji build produced an object that check-elf.sh rejects (a fallback to the system's C
# library: the run cannot be trusted), 2 on a usage or setup error.
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
work=$root/target/smoke
products=$root/target/niji
J=$(nproc)
limit=5400
suite=trixie
mirror=https://deb.debian.org/debian
keyring=""
SMOKE_PROGRAMS="zlib xz_utils sqlite3 grep sed diffutils findutils coreutils gawk make bash perl python3 cppunit
 gawk_ext python3_shared lua_reload cxx_dlclose"

. "$here/programs.sh"

progs=""
while [ $# -gt 0 ]; do
    case $1 in
        --work) work=$2; shift 2 ;;
        --products) products=$2; shift 2 ;;
        --jobs) J=$2; shift 2 ;;
        --timeout) limit=$2; shift 2 ;;
        --mirror) mirror=${2%/}; shift 2 ;;
        --keyring) keyring=$(cd "$(dirname "$2")" 2>/dev/null && pwd)/${2##*/}; shift 2 ;;
        --list) echo "$SMOKE_PROGRAMS"; exit 0 ;;
        -h | --help) sed -n '2,20p' "$0"; exit 0 ;;
        -*) echo "smoke: unknown option $1" >&2; exit 2 ;;
        *) progs="$progs $(printf '%s' "$1" | tr '-' '_')"; shift ;;
    esac
done
[ -n "$progs" ] || progs=$SMOKE_PROGRAMS
mkdir -p "$work" || exit 2
work=$(cd "$work" && pwd)
products=$(cd "$products" 2>/dev/null && pwd) || { echo "smoke: no products directory" >&2; exit 2; }
ldso=$products/ld-linux-x86-64.so.2
sys=$work/sysroot
SMOKE_HERE=$here
export J SMOKE_HERE

# bypath IN OUT: a copy of the shared object IN whose DT_SONAME entry is retagged DT_CHECKSUM (loaders ignore
# it), so an object linked against OUT names it by OUT's path (the method of conformance/run.sh).
bypath() {
    off=$(readelf -Wd "$1" | sed -n 's/^Dynamic section at offset \(0x[0-9a-f]*\) .*/\1/p')
    idx=$(readelf -Wd "$1" | awk '$1 ~ /^0x/ { if ($2 == "(SONAME)") { print n + 0; exit } n++ }')
    [ -n "$off" ] && [ -n "$idx" ] && cp "$1" "$2" &&
        printf '\370\375\377\157\000\000\000\000' | dd of="$2" bs=1 seek=$((off + 16 * idx)) conv=notrunc 2>/dev/null &&
        ! readelf -Wd "$2" | grep -qF '(SONAME)'
}

# setup: the link-time directory niji-cc reads (fresh each run, so it always matches the products).
setup() {
    for f in libc.so.6 libm.a ld-linux-x86-64.so.2; do
        [ -f "$products/$f" ] || { echo "smoke: $products/$f is missing (cargo xtask so)" >&2; return 1; }
    done
    rm -rf "$sys" && mkdir -p "$sys" "$work/bin" "$work/results" || return 1
    bypath "$products/libc.so.6" "$sys/libc-by-path.so" || { echo "smoke: cannot make the by-path libc" >&2; return 1; }
    # libm: the archive. libniji's libm.so.6 names libc.so.6 by soname, which only a library search resolves
    # (at link time the system's libc.so.6 would be found; at run time the loader cannot search yet).
    cp "$products/libm.a" "$sys/libm.a" || return 1
    for a in libpthread libdl librt libutil libanl libresolv; do
        if [ -f "$products/$a.a" ]; then cp "$products/$a.a" "$sys/$a.a"; else ar rc "$sys/$a.a"; fi
    done
    nonshared=$(gcc -print-file-name=libc_nonshared.a)
    case $nonshared in /*) ;; *) echo "smoke: the host has no libc_nonshared.a" >&2; return 1 ;; esac
    printf 'GROUP ( "%s" "%s" AS_NEEDED ( "%s" ) )\n' "$sys/libc-by-path.so" "$nonshared" "$ldso" > "$sys/libc.so"
    printf 'NIJI_SMOKE_SYSROOT=%s\nNIJI_SMOKE_LDSO=%s\n' "$sys" "$ldso" > "$work/bin/niji-cc.env" &&
    cp "$here/niji-cc" "$work/bin/niji-cc" && chmod +x "$work/bin/niji-cc" &&
        ln -sf niji-cc "$work/bin/niji-c++" || return 1
    if [ -d "$root/.git" ] && rev=$(git -C "$root" rev-parse --short=12 HEAD 2>/dev/null); then :; else rev=unknown; fi
    profile=$(cat "$products/.profile" 2>/dev/null || echo unknown)
    printf 'tree %s, products %s (profile %s), host C library %s\n' "$rev" "$products" "$profile" \
        "$(ldd --version 2>&1 | sed -n 1p)" > "$work/results/run-info.txt"
}

# apt_trust DIR: the trust anchor and the source list of the private apt configuration in DIR (deb-src lines
# only, signed-by the one keyring, never trusted=yes). The keyring is --keyring, or the debian-archive-keyring
# package fetched from the mirror, which is accepted only over https. Sets $anchor (what the keyring is).
apt_trust() {
    a=$1
    if [ -n "$keyring" ]; then
        [ -f "$keyring" ] || { echo "smoke: the keyring $keyring does not exist" >&2; return 1; }
        cp "$keyring" "$a/debian-archive-keyring.gpg" || return 1
        anchor="pinned keyring $(sha256sum < "$keyring" | cut -c1-16)"
    else
        case $mirror in
            https://*) ;;
            *) echo "smoke: the mirror $mirror is not https: give the pinned archive keyring with --keyring" >&2; return 1 ;;
        esac
        mkdir -p "$a/key" || return 1
        deb=$(curl -fsSL "$mirror/pool/main/d/debian-archive-keyring/" | grep -o 'debian-archive-keyring_[^"<>]*_all\.deb' |
            sort -u -V | tail -1)
        [ -n "$deb" ] || { echo "smoke: cannot list debian-archive-keyring on $mirror" >&2; return 1; }
        curl -fsSL -o "$a/key/$deb" "$mirror/pool/main/d/debian-archive-keyring/$deb" &&
            dpkg-deb -x "$a/key/$deb" "$a/key/root" || return 1
        k=$(find "$a/key/root" -name 'debian-archive-keyring.gpg' -o -name 'debian-archive-keyring.pgp' | sed -n 1p)
        [ -n "$k" ] || { echo "smoke: no keyring in $deb" >&2; return 1; }
        cp "$k" "$a/debian-archive-keyring.gpg" || return 1
        anchor=$deb
    fi
    printf 'deb-src [signed-by=%s] %s %s main\n' "$a/debian-archive-keyring.gpg" "$mirror" "$suite" > "$a/sources.list"
}

# The private apt configuration, made once per work directory, mirror and trust anchor.
apt_setup() {
    a=$work/apt
    want="$mirror $suite ${keyring:-from-mirror}"
    [ -f "$a/ready" ] && [ "$(sed -n 1p "$a/ready")" = "$want" ] && return 0
    rm -rf "$a" && mkdir -p "$a/empty" "$a/state/lists/partial" "$a/cache/archives/partial" || return 1
    apt_trust "$a" || return 1
    : > "$a/status"
    apt_get update || return 1
    printf '%s\n%s\n' "$want" "$anchor" > "$a/ready"
    echo "sources: $mirror $suite, trust anchor $anchor" >> "$work/results/run-info.txt"
}
apt_get() {
    a=$work/apt
    apt-get -q -o Dir::Etc::SourceList="$a/sources.list" -o Dir::Etc::SourceParts="$a/empty" \
        -o Dir::Etc::Parts="$a/empty" -o Dir::Etc::PreferencesParts="$a/empty" -o Dir::Etc::Preferences="$a/empty/none" \
        -o Dir::Etc::Trusted="$a/empty/none" -o Dir::Etc::TrustedParts="$a/empty" \
        -o Dir::State="$a/state" -o Dir::State::status="$a/status" -o Dir::Cache="$a/cache" \
        -o Debug::NoLocking=1 -o APT::Sandbox::User="$(id -un)" -o Acquire::Languages=none "$@"
}

# fetch SRCPKG: unpack the source package once; prints its directory.
fetch() {
    d=$work/src/$1
    if [ ! -f "$d/.ready" ]; then
        rm -rf "$d" && mkdir -p "$d" || return 1
        (cd "$d" && apt_get source "$1") > "$d.log" 2>&1 || { echo "smoke: apt-get source $1 failed (see $d.log)" >&2; return 1; }
        touch "$d/.ready"
    fi
    find "$d" -mindepth 1 -maxdepth 1 -type d | sed -n 1p
}

# count KIND LOG DIR OUT: write "total pass fail skip xfail error" for one test run to OUT, and the names of the
# failing tests (one per line) to OUT.failed.
count() {
    kind=$1; log=$2; dir=$3; out=$4
    : > "$out.failed"
    case $kind in
        automake)
            find "$dir" -name test-suite.log -type f | sort | while IFS= read -r f; do sed -n 1,12p "$f"; done |
                awk '/^# TOTAL:/ { t += $3 } /^# PASS:/ { p += $3 } /^# FAIL:/ { f += $3 } /^# SKIP:/ { s += $3 }
                     /^# XFAIL:/ { x += $3 } /^# ERROR:/ { e += $3 } /^# XPASS:/ { f += $3 }
                     END { print t + 0, p + 0, f + 0, s + 0, x + 0, e + 0 }' > "$out"
            grep -E '^(FAIL|ERROR|XPASS): ' "$log" | sort -u > "$out.failed"
            # Each test's result from its .trs file, to compare the two builds test by test.
            (cd "$dir" && find . -name '*.trs' | sort | while IFS= read -r f; do
                printf '%s %s\n' "$f" "$(sed -n 's/^:test-result: //p' "$f" | sed -n 1p)"; done) > "$out.results" ;;
        gawk)
            n=$(sed -n 's/^\([0-9][0-9]*\) TESTS* FAILED.*/\1/p' "$log" | tail -1)
            t=$(grep -c '^[a-zA-Z0-9_]*$' "$log")
            if grep -q 'ALL TESTS PASSED' "$log"; then n=0; fi
            [ -n "$n" ] || n=1
            echo "$t $((t - n)) $n 0 0 0" > "$out"
            sed -n '/TESTS* FAILED/,$p' "$log" | grep -v 'FAILED' | sort -u > "$out.failed" ;;
        make)
            if grep -q 'No Failures' "$log"; then f=0; else f=$(sed -n 's/^\([0-9][0-9]*\) Tests* in [0-9]* Categor.* Failed.*/\1/p' "$log" | tail -1); fi
            t=$(grep -c '\.\.\.\.\.\.' "$log")
            [ -n "$f" ] || f=1
            echo "$t $((t - f)) $f 0 0 0" > "$out"
            # The test's name only: an "Error running" line names the build directory, which differs by side.
            grep -E '\.\.\.\.\.\. *(FAILED|Error)' "$log" | awk '{print $1}' | sort -u > "$out.failed" ;;
        bash)
            # A run-* test passes when its section of output is the reference build's; the reference build
            # itself counts its sections as passed (its own diffs are the suite's expected noise on this host).
            # Process ids and the build directory differ between any two runs; they are not compared.
            awk '/^run-/ { name = $1; next } name != "" { print name "\t" $0 }' "$log" |
                sed -e 's#/build/[^/]*/\(glibc\|niji\)/#/build/P/S/#g' -e 's/([0-9][0-9]*)/(N)/g' | sort > "$out.sections"
            t=$(grep -c '^run-' "$log")
            if [ -f "$5" ]; then
                diff "$5" "$out.sections" | sed -n 's/^[<>] \([^\t]*\)\t.*/\1/p' | sort -u > "$out.failed"
            fi
            f=$(wc -l < "$out.failed")
            echo "$t $((t - f)) $f 0 0 0" > "$out" ;;
        perl)
            t=$(sed -n 's/.* scripts=\([0-9]*\) .*/\1/p' "$log" | tail -1); t=${t:-0}
            f=$(sed -n 's/^Failed \([0-9]*\) tests* out of.*/\1/p' "$log" | tail -1); f=${f:-0}
            [ "$5" = 0 ] || [ "$f" -gt 0 ] || f=1
            echo "$t $((t - f)) $f 0 0 0" > "$out"
            grep -E '\.\.\. *FAILED|\.\.\.\.* *FAILED' "$log" | sed 's/ *\.\.\..*//' | sort -u > "$out.failed" ;;
        python)
            # regrtest prints the names of failed and skipped tests as indented lines after a header line.
            awk '/^[0-9]+ tests? (failed|failed again)/ { m = "f"; next } /^[0-9]+ tests? (skipped|altered|omitted|run no tests)/ { m = "s"; next }
                 /^[^ ]/ { m = "" } m != "" && /^    / { for (i = 1; i <= NF; i++) print m, $i }' "$log" > "$out.names"
            t=$(eval "echo \${${name}_tests}" | wc -w)
            f=$(grep -c '^f ' "$out.names"); s=$(grep -c '^s ' "$out.names")
            [ "$5" = 0 ] || [ "$f" -gt 0 ] || f=1
            echo "$t $((t - f - s)) $f $s 0 0" > "$out"
            sed -n 's/^f //p' "$out.names" | sort -u > "$out.failed" ;;
        same)
            # $5: the suite's exit status; $6: the reference build's test log (empty on the reference side).
            # Passes when the suite exits 0 and its whole output is the reference build's.
            if [ "$5" != 0 ]; then
                echo "1 0 1 0 0 0" > "$out"; echo "suite (exit $5)" > "$out.failed"
            elif [ -n "${6:-}" ] && ! cmp -s "$6" "$log"; then
                echo "1 0 1 0 0 0" > "$out"; echo "output differs from the reference build's" > "$out.failed"
                diff "$6" "$log" > "$out.diff"
            else
                echo "1 1 0 0 0 0" > "$out"
            fi ;;
        rc)
            if [ "$5" = 0 ]; then echo "1 1 0 0 0 0" > "$out"; else echo "1 0 1 0 0 0" > "$out"; echo "suite (exit $5)" > "$out.failed"; fi ;;
    esac
}

# run_one NAME SIDE SRC: build NAME on SIDE (glibc | niji) and run its tests; appends to results.tsv.
run_one() {
    name=$1; side=$2; src=$3
    b=$work/build/$name/$side
    r=$work/results/$name.$side
    rm -rf "$b" && mkdir -p "$work/build/$name" && cp -a "$src" "$b" || return 1
    if [ "$side" = niji ]; then cc_=$work/bin/niji-cc; cxx_=$work/bin/niji-c++; else cc_=gcc; cxx_=g++; fi
    eval "kind=\${${name}_kind}"
    noregen="ACLOCAL=true AUTOCONF=true AUTOMAKE=true AUTOHEADER=true MAKEINFO=true"
    status=ok; check=-
    if (cd "$b" && CC=$cc_ CXX=$cxx_ MAKEFLAGS=$noregen NIJI_SMOKE_SYSROOT=$sys NIJI_SMOKE_LDSO=$ldso \
            timeout "$limit" sh -c '. "$1"; "$2"' sh "$here/programs.sh" "${name}_build") > "$r.build.log" 2>&1; then
        built=yes
    else
        brc=$?; built=no; status="build failed (exit $brc)"
    fi
    if [ "$side" = niji ]; then
        if "$here/check-elf.sh" "$ldso" "$sys" "$b" > "$r.check.log" 2>&1; then check=ok; else check=FAIL; fi
    fi
    counts="- - - - - -"
    if [ $built = yes ] && [ "$check" != FAIL ]; then
        eval "skip=\${${name}_skip:-}"
        if [ -n "$skip" ]; then
            status="tests skipped: $skip"
        else
            (cd "$b" && MAKEFLAGS=$noregen NIJI_SMOKE_SYSROOT=$sys NIJI_SMOKE_LDSO=$ldso timeout "$limit" sh -c '. "$1"; "$2"' sh "$here/programs.sh" "${name}_test") \
                > "$r.test.log" 2>&1
            trc=$?
            [ $trc -eq 124 ] && status="test suite hit the ${limit}s hang guard"
            ref=""; [ "$side" = niji ] && ref=$work/results/$name.glibc.test.log
            count "$kind" "$r.test.log" "$b" "$r.count" "$( [ "$kind" = bash ] && echo "$work/results/$name.glibc.count.sections" || echo $trc)" "$ref"
            counts=$(cat "$r.count")
        fi
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$side" "$built" "$check" "$(echo "$counts" | tr ' ' '\t')" "$status" \
        >> "$work/results.tsv"
    echo "smoke: $name $side: built=$built check=$check total/pass/fail/skip/xfail/error=$counts $status"
    [ "$check" != FAIL ]
}

# selftest: the harness's own check (no download). A program and a shared object built by niji-cc must pass
# check-elf.sh, run on libniji's loader and list libniji's libc; the same program built by the system compiler
# must be rejected. A wrapper or check that lets a build fall back to the system's C library fails here.
selftest() {
    t=$work/selftest
    rm -rf "$t" && mkdir -p "$t/ok" || return 1
    e=0
    env NIJI_SMOKE_SYSROOT="$sys" NIJI_SMOKE_LDSO="$ldso" "$work/bin/niji-cc" -O1 -shared -fPIC \
        "$here/selftest/plugin.c" -o "$t/ok/libplugin.so" > "$t/build.log" 2>&1 &&
    env NIJI_SMOKE_SYSROOT="$sys" NIJI_SMOKE_LDSO="$ldso" "$work/bin/niji-cc" -O1 -shared -fPIC \
        "$here/selftest/tls_plugin.c" -o "$t/ok/libtls_plugin.so" >> "$t/build.log" 2>&1 &&
    env NIJI_SMOKE_SYSROOT="$sys" NIJI_SMOKE_LDSO="$ldso" "$work/bin/niji-cc" -O1 "$here/selftest/hello.c" \
        -lm -lpthread -o "$t/ok/hello" >> "$t/build.log" 2>&1 || { echo "smoke-selftest: FAIL niji-cc does not build (see $t/build.log)"; return 1; }
    # Negative controls, one per rule of check-elf.sh: each must be rejected on its own.
    #   system: the system compiler's build (system loader, libc.so.6 by soname)
    #   interp: libniji's libc by path, the system's loader as PT_INTERP
    #   soname: libniji's loader and libc, the system's libc.so.6 named as well (--no-as-needed keeps both)
    #   static: a static program (no PT_INTERP)
    #   nolibc: libniji's loader, no C library named at all
    for k in system interp soname static nolibc; do mkdir -p "$t/bad-$k"; done
    lcc() { env NIJI_SMOKE_SYSROOT="$sys" NIJI_SMOKE_LDSO="$ldso" "$work/bin/niji-cc" "$@"; }
    { gcc -O1 "$here/selftest/hello.c" -lm -o "$t/bad-system/hello" &&
        lcc -O1 "$here/selftest/hello.c" -lm -Wl,--dynamic-linker=/lib64/ld-linux-x86-64.so.2 -o "$t/bad-interp/hello" &&
        lcc -O1 "$here/selftest/hello.c" -lm -Wl,--no-as-needed /lib/x86_64-linux-gnu/libc.so.6 -Wl,--allow-shlib-undefined -o "$t/bad-soname/hello" &&
        gcc -O1 -static "$here/selftest/hello.c" -lm -o "$t/bad-static/hello" &&
        lcc -O1 -nostartfiles -Wl,--as-needed "$here/selftest/nolibc.c" -o "$t/bad-nolibc/nolibc"; } >> "$t/build.log" 2>&1 ||
        { echo "smoke-selftest: FAIL a negative control does not build (see $t/build.log)"; return 1; }
    # A test suite that clears the environment (make's) still links against libniji (check-elf checks ok/).
    env -i PATH="$PATH" "$work/bin/niji-cc" -O1 "$here/selftest/hello.c" -lm -o "$t/ok/hello-noenv" >> "$t/build.log" 2>&1 ||
        { echo "smoke-selftest: FAIL niji-cc does not build with a cleared environment"; e=1; }
    "$here/check-elf.sh" "$ldso" "$sys" "$t/ok" > "$t/check-ok.log" 2>&1 || { echo "smoke-selftest: FAIL check-elf rejects niji-cc's build:"; cat "$t/check-ok.log"; e=1; }
    # The thread-local plug-in is the positive control for the loader's own soname: it must name it.
    self=$(readelf -Wd "$ldso" | sed -n 's/.*(SONAME) *Library soname: \[\(.*\)\]$/\1/p')
    readelf -Wd "$t/ok/libtls_plugin.so" | grep -qF "Shared library: [$self]" ||
        { echo "smoke-selftest: FAIL the thread-local plug-in does not name the loader's soname ($self)"; e=1; }
    for k in system interp soname static nolibc; do
        if "$here/check-elf.sh" "$ldso" "$sys" "$t/bad-$k" > "$t/check-bad-$k.log" 2>&1; then
            echo "smoke-selftest: FAIL check-elf accepts the $k negative control (not linked against libniji)"; e=1
        fi
    done
    out=$("$t/ok/hello" 2 2>&1); rc=$?
    [ $rc -eq 0 ] && [ "$out" = "hello 1.414214" ] || { echo "smoke-selftest: FAIL hello on libniji: exit $rc, output: $out"; e=1; }
    "$ldso" --list "$t/ok/hello" > "$t/list.log" 2>&1
    grep -qF "$sys/libc-by-path.so" "$t/list.log" || { echo "smoke-selftest: FAIL libniji's loader does not list libniji's libc:"; cat "$t/list.log"; e=1; }
    # The trust anchor (no network): a plain-http mirror without --keyring is refused before any download;
    # with --keyring the source list is signed-by a copy of exactly that keyring and never trusted=yes.
    mkdir -p "$t/apt-http" "$t/apt-pinned" && printf 'selftest keyring\n' > "$t/pinned.gpg"
    if (mirror=http://smoke-selftest.invalid/debian; keyring=""; apt_trust "$t/apt-http") > "$t/apt-http.log" 2>&1 ||
        ! grep -q 'is not https: give the pinned archive keyring' "$t/apt-http.log"; then
        echo "smoke-selftest: FAIL a plain-http mirror without --keyring is not refused:"; cat "$t/apt-http.log"; e=1
    fi
    if ! (mirror=http://smoke-selftest.invalid/debian; keyring=$t/pinned.gpg; apt_trust "$t/apt-pinned") > "$t/apt-pinned.log" 2>&1 ||
        ! cmp -s "$t/pinned.gpg" "$t/apt-pinned/debian-archive-keyring.gpg" ||
        [ "$(cat "$t/apt-pinned/sources.list")" != "deb-src [signed-by=$t/apt-pinned/debian-archive-keyring.gpg] http://smoke-selftest.invalid/debian $suite main" ]; then
        echo "smoke-selftest: FAIL --keyring does not make the pinned keyring the only trust anchor:"
        cat "$t/apt-pinned.log" "$t/apt-pinned/sources.list" 2>&1; e=1
    fi
    # Kind `same`: a suite passes only with exit 0 and the reference build's output, byte for byte.
    printf 'line 1\nline 2\n' > "$t/same.ref"; printf 'line 1\nline 2\n' > "$t/same.eq"; printf 'line 1\nline 3\n' > "$t/same.ne"
    for c in "eq 0 1 1 0" "ne 0 1 0 1" "eq 1 1 0 1"; do
        set -- $c
        count same "$t/same.$1" "$t" "$t/same.count" "$2" "$t/same.ref"
        [ "$(cat "$t/same.count")" = "$3 $4 $5 0 0 0" ] ||
            { echo "smoke-selftest: FAIL kind same, output $1, exit $2: counts $(cat "$t/same.count"), want $3 $4 $5 0 0 0"; e=1; }
    done
    [ $e -eq 0 ] && echo "smoke-selftest: PASS"
    return $e
}

setup || exit 2
if [ "$progs" = " selftest" ]; then selftest; exit $?; fi
apt_setup || exit 2
[ -f "$work/results.tsv" ] ||
    printf 'program\tside\tbuilt\tcheck\ttotal\tpass\tfail\tskip\txfail\terror\tstatus\n' > "$work/results.tsv"
bad=0
for p in $progs; do
    eval "srcpkg=\${${p}_src:-}"
    [ -n "$srcpkg" ] || { echo "smoke: unknown program $p (--list)" >&2; exit 2; }
    case $srcpkg in
        local:*) src=$here/${srcpkg#local:} ;;
        *) src=$(fetch "$srcpkg") && [ -n "$src" ] || { echo "smoke: $p: no source" >&2; continue; } ;;
    esac
    run_one "$p" glibc "$src"
    run_one "$p" niji "$src" || bad=1
    if [ -f "$work/results/$p.glibc.count.failed" ] && [ -f "$work/results/$p.niji.count.failed" ]; then
        sort -u "$work/results/$p.glibc.count.failed" > "$work/results/$p.g.tmp"
        sort -u "$work/results/$p.niji.count.failed" | comm -13 "$work/results/$p.g.tmp" - > "$work/results/$p.niji-only.failed"
        rm -f "$work/results/$p.g.tmp"
        echo "smoke: $p: $(wc -l < "$work/results/$p.niji-only.failed") failure(s) on libniji only ($work/results/$p.niji-only.failed)"
    fi
    if [ -f "$work/results/$p.glibc.count.results" ] && [ -f "$work/results/$p.niji.count.results" ]; then
        join "$work/results/$p.glibc.count.results" "$work/results/$p.niji.count.results" | awk '$2 != $3' \
            > "$work/results/$p.differ"
        echo "smoke: $p: $(wc -l < "$work/results/$p.differ") test(s) with another result on libniji (test, reference, libniji: $work/results/$p.differ)"
    fi
done
[ $bad -eq 0 ] || echo "smoke: FAIL: a libniji build produced an object not linked against libniji (see results/*.check.log)"
exit $bad
