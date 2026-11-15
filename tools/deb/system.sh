# tools/deb/system.sh: the system packages (sourced by tools/deb/build.sh --flavour system, after the
# products were built with prefix /). Owner ruling R1 (plan/system-libc.md section 3.1): Liella takes
# the binary package names of Debian's C library, Source: liella. Layout as plan section 3.3: the
# loader and libraries in /usr/lib/x86_64-linux-gnu, /usr/lib64/ld-linux-x86-64.so.2 a link to the
# multiarch copy (the program interpreter /lib64/ld-linux-x86-64.so.2 reaches it through base-files'
# merged-/usr link /lib64 -> usr/lib64, which these packages do not ship). Pending decision
# records/pending/SL-FLAVOUR-1.toml; checks tools/deb/check-system.sh, proof tools/deb/root-system.sh.
#
# Uses build.sh's variables (products, version, epoch, maint, ma, abi, commit, here, out, stage,
# spec) and helpers (put, put_link, put_text, need, installed_size).
#
#   libc6         run time: loader, libc.so.6, libm.so.6, the placeholder libraries; conffile
#                 /etc/ld.so.conf.d/x86_64-linux-gnu.conf; activates the ldconfig trigger;
#                 symbols and shlibs (tools/deb/symbols.py, plan section 3.2; checked by
#                 tools/deb/check-symbols.sh); Provides liella-native-abi (= <newest
#                 LIELLA_NATIVE node>), the virtual package the symbols file's native template names.
#   libc-bin      Essential: the loader configuration (/etc/ld.so.conf, ld.so.conf.d/libc.conf),
#                 /etc/gai.conf, the nsswitch.conf template its postinst installs when the root has
#                 none; owns the ldconfig trigger, which its postinst handles by running
#                 /usr/sbin/ldconfig (static, no interpreter: D-31.2; -r "$DPKG_ROOT" when set). Its
#                 other programs (getconf, getent, iconv, locale, ...) come with SL-BIN-CORE.
#   libc6-dev     link time: start files, the libc.so script, libc_nonshared.a, static archives,
#                 libm.so. No headers yet (SL-HEADERS, R4).
#   libc-dev-bin  placeholder (gencat, ldd: SL-DEVTOOLS, SL-BIN-CORE).
#   locales       placeholder, Architecture all (localedef, locale-gen: SL-LOCALEDEF; data: R5).
#   libc-l10n     placeholder, Architecture all (translations: R5).

sys=$here/system
pkgs="libc6 libc-bin libc6-dev libc-dev-bin locales libc-l10n"
s6=$stage/libc6
bin=$stage/libc-bin
sdev=$stage/libc6-dev
mlib=usr/lib/$ma

# --- libc6 ---
need ld-linux-x86-64.so.2
put 0755 "$products/ld-linux-x86-64.so.2" "$s6/$mlib/ld-linux-x86-64.so.2"
for so in libc.so.6 libm.so.6 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1 libanl.so.1 libnsl.so.1 libc_malloc_debug.so.0; do
    need "$so"
    put 0644 "$products/$so" "$s6/$mlib/$so"
done
put_link ../lib/$ma/ld-linux-x86-64.so.2 "$s6/usr/lib64/ld-linux-x86-64.so.2"
put 0644 "$sys/x86_64-linux-gnu.conf" "$s6/etc/ld.so.conf.d/$ma.conf"

# --- libc-bin ---
put 0644 "$sys/ld.so.conf" "$bin/etc/ld.so.conf"
put 0644 "$sys/libc.conf" "$bin/etc/ld.so.conf.d/libc.conf"
put 0644 "$sys/gai.conf" "$bin/etc/gai.conf"
put 0644 "$sys/nsswitch.conf" "$bin/usr/share/libc-bin/nsswitch.conf"
need ldconfig
put 0755 "$products/ldconfig" "$bin/usr/sbin/ldconfig"

# --- libc6-dev ---
need liella-crt1.o
need liella-rcrt1.o
put 0644 "$products/liella-crt1.o" "$sdev/$mlib/crt1.o"
put 0644 "$products/liella-crt1.o" "$sdev/$mlib/Scrt1.o"
put 0644 "$products/liella-rcrt1.o" "$sdev/$mlib/rcrt1.o"
# SL-CRT: crti.o and crtn.o (every driver link), gcrt1.o (-pg), grcrt1.o (-pg -static-pie) and
# the empty Mcrt1.o, as in the coexisting liella6-dev.
for o in gcrt1 grcrt1 crti crtn Mcrt1; do
    need "liella-$o.o"
    put 0644 "$products/liella-$o.o" "$sdev/$mlib/$o.o"
done
for a in libc.a libm.a libc_nonshared.a libpthread.a libdl.a librt.a libutil.a libanl.a libresolv.a; do
    need "$a"
    put 0644 "$products/$a" "$sdev/$mlib/$a"
done
put_text 0644 "$sdev/$mlib/libc.so" "/* GNU ld script: Liella's C library (libc6-dev). libc.so.6 is the library,
   libc_nonshared.a holds the functions linked into the calling module, the loader
   provides the symbols the ABI places in it. */
OUTPUT_FORMAT(elf64-x86-64)
GROUP ( /lib/$ma/libc.so.6 /usr/lib/$ma/libc_nonshared.a  AS_NEEDED ( /lib64/ld-linux-x86-64.so.2 ) )"
put_link libm.so.6 "$sdev/$mlib/libm.so"

# --- Documentation (every package; identical across architectures, as Multi-Arch: same needs). ---
date_r=$(date -u -d "@$epoch" -R)
for p in $pkgs; do
    d=$stage/$p/usr/share/doc/$p
    put 0644 "$here/copyright" "$d/copyright"
    printf 'liella (%s) unstable; urgency=medium\n\n  * System build of Liella%s (plan/system-libc.md; docs/install-packages.md).\n\n -- %s  %s\n' \
        "$version" "${commit:+, commit $commit}" "$maint" "$date_r" | gzip -9n > "$d/changelog.Debian.gz"
done

find "$s6" -type f -name '*.so*' -exec strip --strip-unneeded --remove-section=.comment --remove-section=.note {} +
find "$sdev" -type f \( -name '*.a' -o -name '*.o' \) -exec strip --strip-debug --remove-section=.comment --enable-deterministic-archives {} +

# --- Control members. ---
# sys_control PKG ARCH SECTION PRIORITY FIELD-LINES...: control (the extra field lines verbatim,
# then the description lines) and md5sums; conffiles from every file under etc/.
sys_control() {
    p=$1; arch=$2; sec=$3; prio=$4; shift 4
    mkdir -p "$stage/$p/DEBIAN"
    {
        echo "Package: $p"
        echo "Source: liella"
        echo "Version: $version"
        echo "Architecture: $arch"
        echo "Maintainer: $maint"
        echo "Installed-Size: $(installed_size "$stage/$p")"
        for line in "$@"; do echo "$line"; done
    } > "$stage/$p/DEBIAN/control.tmp"
    # Debian's field order: the fields given, then Section and Priority before the description.
    awk -v sec="$sec" -v prio="$prio" '/^Description:/ && !d { print "Section: " sec; print "Priority: " prio; d = 1 } { print }' \
        "$stage/$p/DEBIAN/control.tmp" > "$stage/$p/DEBIAN/control"
    rm "$stage/$p/DEBIAN/control.tmp"
    (cd "$stage/$p" && find . -path ./DEBIAN -prune -o -type f -printf '%P\n' | sort | xargs -r md5sum) > "$stage/$p/DEBIAN/md5sums"
    if [ -d "$stage/$p/etc" ]; then
        (cd "$stage/$p" && find etc -type f | sort | sed 's|^|/|') > "$stage/$p/DEBIAN/conffiles"
    fi
}
abi_note=" Liella is an implementation of the GNU C Library's ABI (glibc $abi, x86-64)."
# libc6's symbols and shlibs (plan section 3.2), from the specification and the stripped libraries.
mkdir -p "$s6/DEBIAN"
python3 "$here/symbols.py" files --versions "$spec/abi/versions.tsv" --native "$spec/abi/native.tsv" \
    --gaps "$spec/../docs/spec-symbol-gaps.tsv" --abi "$abi" --version "$version" --libdir "$s6/$mlib" --symbols "$s6/DEBIAN/symbols" --shlibs "$s6/DEBIAN/shlibs"
native_level=$(python3 "$here/symbols.py" native-level --native "$spec/abi/native.tsv")
sys_control libc6 amd64 libs required \
    "Multi-Arch: same" \
    "Provides: liella-native-abi (= $native_level)" \
    "Breaks: base-files (<< 13.3~)" \
    "Description: Liella C library: shared libraries" \
    "$abi_note" \
    " This package is the system's C library: the dynamic loader" \
    " /usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 (program interpreter" \
    " /lib64/ld-linux-x86-64.so.2), libc.so.6, libm.so.6 and the libraries whose" \
    " functions live in libc.so.6 (libpthread.so.0, libdl.so.2, librt.so.1, ...)."
sys_control libc-bin amd64 libs required \
    "Essential: yes" \
    "Multi-Arch: foreign" \
    "Depends: libc6 (= $version)" \
    "Description: Liella C library: configuration and programs" \
    "$abi_note" \
    " This package holds ldconfig, the loader configuration (/etc/ld.so.conf and" \
    " /etc/ld.so.conf.d), /etc/gai.conf and the template of /etc/nsswitch.conf. The" \
    " C library's other programs (getconf, getent, iconv, locale) are not part" \
    " of this build yet."
sys_control libc6-dev amd64 libdevel optional \
    "Multi-Arch: same" \
    "Depends: libc6 (= $version), libc-dev-bin (= $version)" \
    "Provides: libc-dev" \
    "Description: Liella C library: development files" \
    "$abi_note" \
    " This package holds the start files, the libc.so linker script," \
    " libc_nonshared.a and the static archives. Liella's C headers are not part of" \
    " this build yet."
sys_control libc-dev-bin amd64 libdevel optional \
    "Multi-Arch: foreign" \
    "Depends: libc6 (= $version)" \
    "Description: Liella C library: development programs (placeholder)" \
    "$abi_note" \
    " The development programs (gencat, ldd) are not part of this build yet; the" \
    " package exists so that libc6-dev's dependency is met."
sys_control locales all localization optional \
    "Depends: libc-bin (>= $abi)" \
    "Description: Liella C library: locale support (placeholder)" \
    "$abi_note" \
    " The locale compiler and locale data are not part of this build yet; the" \
    " package exists so that dependencies on locales are met."
sys_control libc-l10n all localization optional \
    "Description: Liella C library: translations (placeholder)" \
    "$abi_note" \
    " The C library's message translations are not part of this build yet; the" \
    " package exists so that dependencies on libc-l10n are met."
# The ldconfig trigger (plan/system-libc.md section 2): libc6 activates it, libc-bin handles it.
printf 'activate-noawait ldconfig\n' > "$s6/DEBIAN/triggers"
printf 'interest-await ldconfig\n' > "$bin/DEBIAN/triggers"
put 0755 "$sys/libc-bin.postinst" "$bin/DEBIAN/postinst"

# Fixed modes and times.
find "$stage" -type d -exec chmod 0755 {} +
find "$stage" -path '*/DEBIAN/*' -type f ! -name postinst -exec chmod 0644 {} +
find "$stage" -exec touch -h -d "@$epoch" {} +

mkdir -p "$out"
for p in $pkgs; do
    arch=$(sed -n 's/^Architecture: //p' "$stage/$p/DEBIAN/control")
    deb=$out/${p}_${version}_$arch.deb
    dpkg-deb --root-owner-group -Zxz -z9 --build "$stage/$p" "$deb" >/dev/null
    echo "build.sh: $(sha256sum "$deb" | cut -c1-64)  $deb"
done
