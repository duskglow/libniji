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
# Contents as data (PKG-MANIFEST): every shipped file is a row of tools/deb/manifest.tsv (path,
# kind, source, mode, strip rule) and every control field a row of tools/deb/control.tsv, read
# through tools/deb/manifest.py; this script and system.sh install exactly those rows. Shipping a
# file is a manifest row, not a shell line.
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
need() {
    [ -f "$products/$1" ] || { echo "build.sh: $products/$1 is missing (cargo xtask so)" >&2; exit 1; }
}

# installed_size DIR: Debian's Installed-Size (KiB): each file's size rounded up, 1 per other entry.
installed_size() {
    find "$1" -path "$1/DEBIAN" -prune -o -mindepth 1 -printf '%y %s\n' |
        awk '$1 == "f" { t += int(($2 + 1023) / 1024); next } { t += 1 } END { print t + 0 }'
}

# The flavour's rows of tools/deb/manifest.tsv (package, path, kind, source, mode, strip; manifest.py
# refuses malformed tables), read by the helpers below.
rows=$work/manifest-rows.tsv
mkdir -p "$work"
python3 "$here/manifest.py" rows "$flavour" > "$rows"
pkgs=$(python3 "$here/manifest.py" packages "$flavour")
tab=$(printf '\t')
date_r=$(date -u -d "@$epoch" -R)

# changelog: the text of every package's changelog row.
changelog() {
    if [ "$flavour" = coexist ]; then
        printf 'niji (%s) unstable; urgency=medium\n\n  * Snapshot of libniji (reference packages; docs/install-packages.md).\n\n -- %s  %s\n' \
            "$version" "$maint" "$date_r"
    else
        printf 'niji (%s) unstable; urgency=medium\n\n  * System build of libniji%s (plan/system-libc.md; docs/install-packages.md).\n\n -- %s  %s\n' \
            "$version" "${commit:+, commit $commit}" "$maint" "$date_r"
    fi
}

# install_rows: every row into STAGE/<package>/<path> (DEBIAN/<name>: a maintainer script).
install_rows() {
    while IFS=$tab read -r p path kind src mode strip; do
        dest=$stage/$p/$path
        case $kind in
            file) need "$src"; put "$mode" "$products/$src" "$dest" ;;
            text) put "$mode" "$here/$src" "$dest" ;;
            gzip) mkdir -p "$(dirname "$dest")"; gzip -9n < "$here/$src" > "$dest"; chmod "$mode" "$dest" ;;
            link) put_link "$src" "$dest" ;;
            changelog) mkdir -p "$(dirname "$dest")"; changelog | gzip -9n > "$dest"; chmod "$mode" "$dest" ;;
            dir) mkdir -p "$dest" ;;
            *) echo "build.sh: $p $path: kind $kind" >&2; exit 1 ;;
        esac
    done < "$rows"
}

# rows_with STRIP...: the staged path of every file row whose strip column is one of STRIP.
rows_with() {
    awk -F "$tab" -v want=" $* " -v stage="$stage" 'index(want, " " $6 " ") { print stage "/" $1 "/" $2 }' "$rows"
}

# strip_rows: strip the files as Debian does: the shared objects and programs (strip unneeded or
# debug) lose debug sections, .comment and .note; the archives and start files (archive) keep their
# symbols (the link needs them) but lose debug sections and .comment.
strip_rows() {
    rows_with unneeded debug | xargs -r strip --strip-unneeded --remove-section=.comment --remove-section=.note
    rows_with archive | xargs -r strip --strip-debug --remove-section=.comment --enable-deterministic-archives
}

# write_control PKG: DEBIAN/control (tools/deb/control.tsv and description/PKG), md5sums, and the
# conffiles and triggers members when the package has them.
write_control() {
    p=$1
    mkdir -p "$stage/$p/DEBIAN"
    python3 "$here/manifest.py" control "$flavour" "$p" --version "$version" --abi "$abi" --maint "$maint" \
        --installed-size "$(installed_size "$stage/$p")" --provides "${provides:-}" > "$stage/$p/DEBIAN/control"
    (cd "$stage/$p" && find . -path ./DEBIAN -prune -o -type f -printf '%P\n' | sort | xargs -r md5sum) > "$stage/$p/DEBIAN/md5sums"
    for m in conffiles triggers; do
        python3 "$here/manifest.py" member "$flavour" "$p" "$m" > "$stage/$p/DEBIAN/$m"
        [ -s "$stage/$p/DEBIAN/$m" ] || rm "$stage/$p/DEBIAN/$m"
    done
}

# finish: fixed modes (directories 0755, control members 0644 but the maintainer scripts' rows) and
# times, then one .deb per package.
finish() {
    find "$stage" -type d -exec chmod 0755 {} +
    find "$stage" -path '*/DEBIAN/*' -type f -exec chmod 0644 {} +
    while IFS=$tab read -r p path kind src mode strip; do
        case $path in DEBIAN/*) chmod "$mode" "$stage/$p/$path" ;; esac
    done < "$rows"
    find "$stage" -exec touch -h -d "@$epoch" {} +
    mkdir -p "$out"
    for p in $pkgs; do
        arch=$(sed -n 's/^Architecture: //p' "$stage/$p/DEBIAN/control")
        deb=$out/${p}_${version}_$arch.deb
        dpkg-deb --root-owner-group -Zxz -z9 --build "$stage/$p" "$deb" >/dev/null
        echo "build.sh: $(sha256sum "$deb" | cut -c1-64)  $deb"
    done
}

if [ "$flavour" = system ]; then
    . "$here/system.sh"
    exit 0
fi

# niji6 (run time: the sysroot laid out as Debian's libc6 lays out /) and niji6-dev (link time, as
# Debian's libc6-dev, static archives included): the manifest's both and coexist rows.
install_rows
strip_rows
for p in $pkgs; do
    write_control "$p"
done
finish
