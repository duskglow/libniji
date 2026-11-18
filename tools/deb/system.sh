# tools/deb/system.sh: the system packages (sourced by tools/deb/build.sh --flavour system, after the
# products were built with prefix /). Owner ruling R1 (plan/system-libc.md section 3.1): libniji takes
# the binary package names of Debian's C library, Source: niji. Layout as plan section 3.3: the
# loader and libraries in /usr/lib/x86_64-linux-gnu, /usr/lib64/ld-linux-x86-64.so.2 a link to the
# multiarch copy (the program interpreter /lib64/ld-linux-x86-64.so.2 reaches it through base-files'
# merged-/usr link /lib64 -> usr/lib64, which these packages do not ship). Pending decision
# records/pending/SL-FLAVOUR-1.toml; checks tools/deb/check-system.sh, proof tools/deb/root-system.sh.
#
# Uses build.sh's variables (products, version, epoch, maint, ma, abi, commit, here, out, stage,
# spec, symbols_only, pkgs) and helpers (install_rows, rows_with, strip_rows, write_control,
# finish). What each package ships is tools/deb/manifest.tsv (flavour system and both rows), its
# control fields tools/deb/control.tsv and tools/deb/description/<package> (PKG-MANIFEST); the
# summary below describes those rows.
#
#   libc6         run time: loader, libc.so.6, libm.so.6, libmvec.so.1, the placeholder libraries (with
#                 libBrokenLocale.so.1: SYS-DEV-LINKS), libresolv.so.2 (D-16.6: its exports jump
#                 to libc.so.6's own definitions); conffile
#                 /etc/ld.so.conf.d/x86_64-linux-gnu.conf; activates the ldconfig trigger;
#                 symbols and shlibs (tools/deb/symbols.py, plan section 3.2; checked by
#                 tools/deb/check-symbols.sh); Provides, per libniji-only node kind, the virtual
#                 package its symbols' template names at the newest level (niji-native-abi
#                 (= <n>), niji-owner-abi (= <n>); symbols.py --provides).
#   libc-bin      Essential: the loader configuration (/etc/ld.so.conf, ld.so.conf.d/libc.conf),
#                 /etc/gai.conf, the nsswitch.conf template its postinst installs when the root has
#                 none; owns the ldconfig trigger, which its postinst handles by running
#                 /usr/sbin/ldconfig (static, no interpreter: D-31.2; -r "$DPKG_ROOT" when set),
#                 as it does on configure (an activation while libc-bin is unconfigured is not
#                 kept, deb-triggers(5)); its postrm removes /etc/ld.so.cache on purge.
#                 /usr/bin/getconf (UTIL-5: static, over libc.a), /usr/bin/getent (dynamic, over
#                 libc6's name services: UTIL-6), /usr/bin/iconv (UTIL-7: dynamic, over libc6's iconv(3)), /usr/bin/ldd (UTIL-4: static, runs the loader in trace mode). Its other programs (iconv, locale, ...) come
#                 with SL-BIN-CORE. No /etc/bindresvport.blacklist (spec 31.14, D-31.5: not
#                 installed) and no /etc/default/nss (libniji's name services read no such file).
#   libc6-dev     link time: start files, the libc.so script, libc_nonshared.a, static archives,
#                 libm.so, libmvec.so, libresolv.so, the development links libanl.so, libBrokenLocale.so,
#                 libc_malloc_debug.so. No headers yet (SL-HEADERS, R4). Depends on linux-libc-dev,
#                 libcrypt-dev and rpcsvc-proto (builds that include <linux/...> or <crypt.h>, or
#                 run rpcgen, get them through libc6-dev); Provides: libc-dev (= <version>), so
#                 versioned relations on libc-dev are met.
#   libc-dev-bin  placeholder (gencat: SL-DEVTOOLS, SL-BIN-CORE).
#   locales       placeholder, Architecture all (localedef, locale-gen: SL-LOCALEDEF; data: R5).
#   libc-l10n     placeholder, Architecture all (translations: R5).
#   libc6-dbg     the detached debugging information of libc6's shared objects and libc-bin's
#                 programs (SYS-DBG-PACKAGE): objcopy --only-keep-debug of each unstripped file
#                 (its symbol table and DWARF) as /usr/lib/debug/.build-id/xx/yyyy.debug, xx/yyyy its
#                 GNU build-id note, which the stripped file keeps, so debuggers find it by the id.
#                 Depends: libc6 (= version); Provides: libc-dbg, libc6-dbgsym; Multi-Arch: same.
#                 The packaged objects themselves are stripped exactly as before (no debuglink).

# The package holding the detached debugging information (libc6-dbg) and the one with the symbols
# and shlibs members (libc6): tools/deb/control.tsv's debug and symbols rows.
dbgpkg=$(python3 "$here/manifest.py" debug-package system)
sympkg=$(python3 "$here/manifest.py" symbols-package system)

install_rows

# --- libc6-dbg: split the debugging information off the strip debug rows before strip_rows. ---
# build_id FILE: the hex digits of FILE's GNU build-id note (empty when it has none).
build_id() { readelf -nW "$1" | sed -n 's/.*Build ID: \([0-9a-f]*\)$/\1/p' | head -1; }
if [ -z "$symbols_only" ] && [ -n "$dbgpkg" ]; then
    for f in $(rows_with debug); do
        id=$(build_id "$f")
        # At least one byte for the directory and one for the file name.
        [ "${#id}" -ge 4 ] || { echo "build.sh: ${f#"$stage"/} has no GNU build-id note ($dbgpkg's files are named by it)" >&2; exit 1; }
        d=$stage/$dbgpkg/usr/lib/debug/.build-id/$(printf '%.2s' "$id")/${id#??}.debug
        # Two objects with one id would make a debugger read the wrong file for one of them.
        [ ! -e "$d" ] || { echo "build.sh: ${f#"$stage"/} has the build-id $id of another packaged object" >&2; exit 1; }
        mkdir -p "$(dirname "$d")"
        objcopy --only-keep-debug --compress-debug-sections=zlib "$f" "$d"
        chmod 0644 "$d"
    done
fi

strip_rows

# libc6's symbols and shlibs (plan section 3.2), from the specification and the stripped libraries.
mkdir -p "$stage/$sympkg/DEBIAN"
python3 "$here/symbols.py" files --versions "$spec/abi/versions.tsv" --native "$spec/abi/native.tsv" \
    --gaps "$spec/../docs/spec-symbol-gaps.tsv" --abi "$abi" --version "$version" --libdir "$stage/$sympkg/usr/lib/$ma" \
    --symbols "$stage/$sympkg/DEBIAN/symbols" --shlibs "$stage/$sympkg/DEBIAN/shlibs" --provides "$stage/$sympkg.provides"
provides=$(cat "$stage/$sympkg.provides")
[ -n "$provides" ] || { echo "build.sh: symbols.py names no virtual package for $sympkg to provide" >&2; exit 1; }
write_control "$sympkg"
if [ -n "$symbols_only" ]; then
    echo "build.sh: symbols step only: $stage/$sympkg/DEBIAN (symbols, shlibs, control; Provides: $provides); no packages written"
    exit 0
fi
for p in $pkgs; do
    [ "$p" = "$sympkg" ] || write_control "$p"
done
finish
