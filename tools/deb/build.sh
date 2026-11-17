#!/bin/sh
# tools/deb/build.sh: build libniji's reference Debian packages (docs/install-packages.md; the layout
# is the pending decision records/pending/RELEASE-PACKAGING-DEB-1.toml).
#
#   tools/deb/build.sh [--out DIR] [--work DIR]
#       From a clean tree: the committed HEAD is exported (git archive) into WORK/src, the products
#       are built there in the release profile with the packages' loader layout
#       (NIJI_PREFIX=/usr/lib/niji, NIJI_LIBDIR=usr/lib/x86_64-linux-gnu), and the packages
#       are written to OUT (default target/deb). Uncommitted changes are never packaged.
#   tools/deb/build.sh --products DIR --version V [--out DIR] [--work DIR]
#       Package an existing products directory (built with that layout) as version V; no build.
#   --flavour system (with either form; default --flavour coexist)
#       The system build (plan/system-libc.md section 3, records/pending/SL-FLAVOUR-1.toml): the
#       same tree built with prefix / (NIJI_PREFIX=/, NIJI_LIBDIR=usr/lib/x86_64-linux-gnu),
#       packaged as the system's C library under Debian's names (libc6, libc-bin, libc6-dev,
#       libc-dev-bin, locales, libc-l10n; tools/deb/system.sh), into OUT (default
#       target/deb-system; work directory target/deb-system-work). Version
#       <abi>+niji<YYYYMMDD>.<n>-<rev> (owner ruling R2): <abi> 2.39, <n> NIJI_DEB_SERIAL
#       (default 1), <rev> NIJI_DEB_REVISION (default 1). tools/deb/check-system.sh checks them.
#   --symbols-only (with --flavour system --products)
#       Stop after libc6's symbols, shlibs and control (WORK/stage/libc6/DEBIAN): no .deb is
#       written. The gate's check that every version node the libraries define has a dependency
#       rule (crates/core/tests/deb_symbols_products.rs), a few seconds instead of the packaging.
#
# Packages: niji6 (run time: the loader and the shared libraries) and niji6-dev (link time:
# start files, linker scripts, static archives, the niji-gcc/niji-g++ wrapper). Every file
# is under the sysroot /usr/lib/niji except the wrapper, its manual page and the packages'
# /usr/share/doc directories: nothing of the system's C library is touched (tools/deb/check.sh
# enforces it).
#
# Reproducible: SOURCE_DATE_EPOCH (default: the HEAD commit's time; 1 with --products) is every
# file's time and the time dpkg-deb records; owners are root:root (--root-owner-group); dpkg-deb
# writes members in sorted order; modes are fixed below. Building twice gives identical .debs.
#
# Version: 0.0~snapshotYYYYMMDD+<12-hex commit> (YYYYMMDD: the commit's UTC date), a native
# version that sorts below any 0.0 release.
set -eu
umask 022
export LC_ALL=C TZ=UTC

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
out=$root/target/deb
work=$root/target/deb-work
products=""
version=""
flavour=coexist
outset=""
workset=""
symbols_only=""
while [ $# -gt 0 ]; do
    case $1 in
        --symbols-only) symbols_only=1; shift ;;
        --out) out=$2; outset=1; shift 2 ;;
        --work) work=$2; workset=1; shift 2 ;;
        --flavour) flavour=$2; shift 2 ;;
        --products) products=$2; shift 2 ;;
        --version) version=$2; shift 2 ;;
        *) echo "usage: tools/deb/build.sh [--flavour coexist|system] [--out DIR] [--work DIR] [--products DIR --version V] [--symbols-only]" >&2; exit 2 ;;
    esac
done
if [ -n "$symbols_only" ] && { [ "$flavour" != system ] || [ -z "$products" ]; }; then
    echo "build.sh: --symbols-only needs --flavour system and --products" >&2
    exit 2
fi
case $flavour in
    coexist) ;;
    system)
        [ -n "$outset" ] || out=$root/target/deb-system
        [ -n "$workset" ] || work=$root/target/deb-system-work ;;
    *) echo "build.sh: --flavour $flavour: coexist or system" >&2; exit 2 ;;
esac

sysroot=/usr/lib/niji
ma=x86_64-linux-gnu
# The loader's compile-time prefix (crates/ld/src/search.rs): the sysroot, or / for the system build.
prefix=$sysroot
[ "$flavour" = coexist ] || prefix=/
# The ABI level the system packages' versions start with (R2; stays 2.39 until GLIBC_2.41 is complete, R3).
abi=2.39
# The project's packaging address (owner 2026-11-13); DEB_MAINTAINER overrides it for a rebuild.
maint=${DEB_MAINTAINER:-Liella Maintainers <liella@koiren.ai>}

if [ -z "$products" ]; then
    [ -z "$version" ] || { echo "build.sh: --version goes with --products" >&2; exit 2; }
    rm -rf "$work/src"
    mkdir -p "$work/src"
    if git -C "$root" rev-parse -q --verify HEAD > /dev/null 2>&1; then
        commit=$(git -C "$root" rev-parse --short=12 HEAD)
        epoch=${SOURCE_DATE_EPOCH:-$(git -C "$root" log -1 --format=%ct HEAD)}
        if [ -n "$(git -C "$root" status --porcelain --untracked-files=no)" ]; then
            echo "build.sh: note: the tree has uncommitted changes; they are not packaged (HEAD $commit is)"
        fi
        # Through a file, not a pipe: a failed export stops the build (set -e).
        git -C "$root" archive --format=tar -o "$work/src.tar" HEAD
        tar -x -C "$work/src" -f "$work/src.tar"
        rm -f "$work/src.tar"
    else
        # A synced copy without .git (a build host's role directory): the caller names the commit
        # it holds and its time; the files are packaged as they are, without target/ and without
        # any other build directory (cargo tags each with CACHEDIR.TAG, e.g. fuzz/target): another
        # job may be writing one while it is copied, and the build never reads them. Through a file
        # again, so that a failed copy stops the build instead of packaging part of the tree.
        if [ -z "${NIJI_DEB_COMMIT:-}" ] || [ -z "${SOURCE_DATE_EPOCH:-}" ]; then
            echo "build.sh: $root is not a git tree: set NIJI_DEB_COMMIT and SOURCE_DATE_EPOCH" >&2
            exit 2
        fi
        commit=$(printf '%.12s' "$NIJI_DEB_COMMIT")
        epoch=$SOURCE_DATE_EPOCH
        echo "build.sh: note: $root is not a git tree; packaging its files as commit $commit"
        tar -C "$root" --exclude=./target --exclude=./.git --exclude-caches-all -cf "$work/src.tar" .
        tar -x -C "$work/src" -f "$work/src.tar"
        rm -f "$work/src.tar"
    fi
    if [ "$flavour" = coexist ]; then
        version=0.0~snapshot$(date -u -d "@$epoch" +%Y%m%d)+$commit
    else
        version=$abi+niji$(date -u -d "@$epoch" +%Y%m%d).${NIJI_DEB_SERIAL:-1}-${NIJI_DEB_REVISION:-1}
    fi
    echo "build.sh: building the products of $commit (release profile, prefix $prefix) in $work/src"
    (cd "$work/src" && NIJI_PREFIX=$prefix NIJI_LIBDIR=usr/lib/$ma NIJI_FULL_PROFILE=1 SOURCE_DATE_EPOCH=$epoch cargo xtask so)
    products=$work/src/target/niji
    # The specification tables of the tree the products were built from (system.sh's symbols file).
    spec=$work/src/spec
else
    [ -n "$version" ] || { echo "build.sh: --products needs --version" >&2; exit 2; }
    epoch=${SOURCE_DATE_EPOCH:-1}
    spec=$root/spec
fi
products=$(cd "$products" && pwd)
export SOURCE_DATE_EPOCH="$epoch"
# --products: the commit the products were built from, when the caller names it (the system
# packages' changelog names it, so a repack of the same products is byte-identical).
[ -n "${commit:-}" ] || commit=$(printf '%.12s' "${NIJI_DEB_COMMIT:-}")

stage=$work/stage
rm -rf "$stage"
rt=$stage/niji6
dev=$stage/niji6-dev

# put MODE SRC DEST: a copy of SRC at DEST (parents made).
put() {
    mkdir -p "$(dirname "$3")"
    cp "$2" "$3"
    chmod "$1" "$3"
}
# put_link TARGET LINK: a symbolic link LINK -> TARGET (parents made).
put_link() {
    mkdir -p "$(dirname "$2")"
    ln -s "$1" "$2"
}
# put_text MODE DEST TEXT: a file holding TEXT and a newline.
put_text() {
    mkdir -p "$(dirname "$2")"
    printf '%s\n' "$3" > "$2"
    chmod "$1" "$2"
}
need() {
    [ -f "$products/$1" ] || { echo "build.sh: $products/$1 is missing (cargo xtask so)" >&2; exit 1; }
}

# installed_size DIR: Debian's Installed-Size (KiB): each file's size rounded up, 1 per other entry.
installed_size() {
    find "$1" -path "$1/DEBIAN" -prune -o -mindepth 1 -printf '%y %s\n' |
        awk '$1 == "f" { t += int(($2 + 1023) / 1024); next } { t += 1 } END { print t + 0 }'
}

if [ "$flavour" = system ]; then
    . "$here/system.sh"
    exit 0
fi

# --- niji6: the run-time sysroot, laid out as Debian's libc6 lays out / (merged /usr). ---
rlib=$rt$sysroot/usr/lib/$ma
need ld-linux-x86-64.so.2
put 0755 "$products/ld-linux-x86-64.so.2" "$rlib/ld-linux-x86-64.so.2"
for so in libc.so.6 libm.so.6 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1 libanl.so.1 libnsl.so.1 libc_malloc_debug.so.0 libresolv.so.2 libBrokenLocale.so.1 libthread_db.so.1; do
    need "$so"
    put 0644 "$products/$so" "$rlib/$so"
done
# The program interpreter's path, /usr/lib/niji/lib64/ld-linux-x86-64.so.2, as Debian's
# /lib64/ld-linux-x86-64.so.2: lib64 -> usr/lib64, usr/lib64/<loader> -> the multiarch copy.
put_link ../lib/$ma/ld-linux-x86-64.so.2 "$rt$sysroot/usr/lib64/ld-linux-x86-64.so.2"
put_link usr/lib "$rt$sysroot/lib"
put_link usr/lib64 "$rt$sysroot/lib64"

# --- niji6-dev: link time, as Debian's libc6-dev (static archives included, as there). ---
dlib=$dev$sysroot/usr/lib/$ma
need niji-crt1.o
need niji-rcrt1.o
put 0644 "$products/niji-crt1.o" "$dlib/crt1.o"
put 0644 "$products/niji-crt1.o" "$dlib/Scrt1.o"
put 0644 "$products/niji-rcrt1.o" "$dlib/rcrt1.o"
# SL-CRT: the rest of the start files a plain compiler-driver link names (crti.o and crtn.o in
# every link, gcrt1.o for -pg, grcrt1.o for -pg -static-pie) and Mcrt1.o, which is empty.
for o in gcrt1 grcrt1 crti crtn Mcrt1; do
    need "niji-$o.o"
    put 0644 "$products/niji-$o.o" "$dlib/$o.o"
done
for a in libc.a libm.a libc_nonshared.a libpthread.a libdl.a librt.a libutil.a libanl.a libresolv.a libBrokenLocale.a libg.a libmcheck.a libpthread_nonshared.a; do
    need "$a"
    put 0644 "$products/$a" "$dlib/$a"
done
# The linker scripts name files inside the sysroot; the wrapper links with --sysroot, so the link
# editor reads them there (the script lies in the sysroot).
put_text 0644 "$dlib/libc.so" "/* GNU ld script: libniji's C library (niji6-dev). Paths are inside the sysroot
   /usr/lib/niji; link with --sysroot=/usr/lib/niji (niji-gcc does). */
OUTPUT_FORMAT(elf64-x86-64)
GROUP ( /lib/$ma/libc.so.6 /usr/lib/$ma/libc_nonshared.a  AS_NEEDED ( /lib64/ld-linux-x86-64.so.2 ) )"
# libm.so stays a link: libniji has no libmvec, so no linker script is needed (SL-SONAMES).
put_link libm.so.6 "$dlib/libm.so"
put_link libresolv.so.2 "$dlib/libresolv.so"
put_link libBrokenLocale.so.1 "$dlib/libBrokenLocale.so"
put_link libthread_db.so.1 "$dlib/libthread_db.so"
put 0755 "$here/niji-gcc" "$dev/usr/bin/niji-gcc"
put_link niji-gcc "$dev/usr/bin/niji-g++"
mkdir -p "$dev/usr/share/man/man1"
gzip -9n < "$here/niji-gcc.1" > "$dev/usr/share/man/man1/niji-gcc.1.gz"
put_link niji-gcc.1.gz "$dev/usr/share/man/man1/niji-g++.1.gz"

# --- Documentation of both packages. ---
date_r=$(date -u -d "@$epoch" -R)
for p in niji6 niji6-dev; do
    d=$stage/$p/usr/share/doc/$p
    put 0644 "$here/copyright" "$d/copyright"
    printf 'niji (%s) unstable; urgency=medium\n\n  * Snapshot of libniji (reference packages; docs/install-packages.md).\n\n -- %s  %s\n' \
        "$version" "$maint" "$date_r" | gzip -9n > "$d/changelog.gz"
done

# Strip the shared objects as Debian does (debug sections, .comment, .note); the archives and
# start files keep their symbols (the link needs them) but lose debug sections and .comment.
find "$rt$sysroot" -type f -name '*.so*' -exec strip --strip-unneeded --remove-section=.comment --remove-section=.note {} +
find "$dev$sysroot" -type f \( -name '*.a' -o -name '*.o' \) -exec strip --strip-debug --remove-section=.comment --enable-deterministic-archives {} +

# control PKG SECTION DEPENDS DESCRIPTION...
control() {
    p=$1; sec=$2; deps=$3; shift 3
    mkdir -p "$stage/$p/DEBIAN"
    {
        echo "Package: $p"
        echo "Source: niji"
        echo "Version: $version"
        echo "Architecture: amd64"
        echo "Maintainer: $maint"
        echo "Installed-Size: $(installed_size "$stage/$p")"
        [ -z "$deps" ] || echo "Depends: $deps"
        [ "$p" != niji6-dev ] || echo "Recommends: g++"
        echo "Section: $sec"
        echo "Priority: optional"
        for line in "$@"; do echo "$line"; done
    } > "$stage/$p/DEBIAN/control"
    (cd "$stage/$p" && find . -path ./DEBIAN -prune -o -type f -printf '%P\n' | sort | xargs -r md5sum) > "$stage/$p/DEBIAN/md5sums"
}
control niji6 libs "" \
    "Description: libniji C library (run time, beside the system's C library)" \
    " libniji is an implementation of the GNU C Library's ABI (glibc 2.39, x86-64)." \
    " This package installs its loader and shared libraries in the sysroot" \
    " /usr/lib/niji. It does not replace or change the system's C library: only" \
    " programs whose interpreter is /usr/lib/niji/lib64/ld-linux-x86-64.so.2" \
    " (built with niji-gcc from niji6-dev) run on libniji."
control niji6-dev libdevel "niji6 (= $version), gcc, libc6-dev, binutils" \
    "Description: libniji C library (development files and compiler wrapper)" \
    " libniji is an implementation of the GNU C Library's ABI (glibc 2.39, x86-64)." \
    " This package installs the start files, linker scripts and static archives in" \
    " the sysroot /usr/lib/niji, and niji-gcc and niji-g++, which build" \
    " programs that run on libniji's loader. Programs compile with the system's C" \
    " headers (from libc6-dev)."

# Fixed modes and times.
find "$stage" -type d -exec chmod 0755 {} +
find "$stage" -path '*/DEBIAN/*' -type f -exec chmod 0644 {} +
find "$stage" -exec touch -h -d "@$epoch" {} +

mkdir -p "$out"
for p in niji6 niji6-dev; do
    deb=$out/${p}_${version}_amd64.deb
    dpkg-deb --root-owner-group -Zxz -z9 --build "$stage/$p" "$deb" >/dev/null
    echo "build.sh: $(sha256sum "$deb" | cut -c1-64)  $deb"
done
