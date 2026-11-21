# SYS-DBG-DWARF: every libc6-dbg debug file carries DWARF (COMPLETE, leg 2)

Branch `task/sys-dbg-dwarf` (from origin/next df83365cd), role mei:~/w/s3 (= /mnt/zram/claude/w/s3).
Brief `~/state/briefs/SYS-DBG-DWARF.md`. Job scripts on mei `~/work/SYS-DBG-DWARF/` (pkg.sh, proof.sh,
recheck2.sh). Decision in force: D-X.146 (records/decisions/D-X/D-X.146.toml). Plants:
handoff/SYS-DBG-DWARF-plants.txt.

## Causes (per product) and fix (commit 75af35f67)

* libm.so.6, libresolv.so.2: Rust crates have line tables, but the one-object step
  `objcopy --strip-unneeded` (so.rs run_libm, libresolv_object) also drops debug sections. Fix:
  `KEEP_DEBUG_SECTIONS` = `--keep-section=.debug_*` on both (unneeded symbols still removed).
* libpthread, libdl, librt, libutil, libanl, libnsl, libBrokenLocale, libc_malloc_debug (placeholders.rs),
  libmvec.so.1 (libmvec.rs), getent/iconv/locale (so.rs run_dynamic_util): C compiled without `-g`.
  Fix: `so::release_debug_cflags(root, profile)`: release only, `-g1` + `-fdebug-prefix-map=<root>=/niji`
  (not -ffile-prefix-map: it rewrites `__FILE__`, loaded bytes). Also on getconf's and ldd's own C file
  and the placeholder/libmvec archives. Fast profile: no change.
* iconv, locale: manifest rows strip `unneeded` -> `debug` (libc6-dbg files through the manifest).
* check-system.sh: every debug file needs `.debug_info` and `.debug_line`; every ELF EXEC/DYN file the
  packages ship (found by ELF header, not the manifest) must have its libc6-dbg file.
* Lintian: libc6 override for libmvec.so.1 `(libc.so.6 libm.so.6)`; libc6 and libc-bin descriptions rewrapped.

## Results (mei, Skylake-SP host)

* `sys-dbg-dwarf-base` rc 0: next's system debs (copy /mnt/zram/claude/SYS-DBG-DWARF/base-debs).
* `sys-dbg-dwarf-proof` (SUPERSEDED: its gdb step used versioned names; its data holds): SHF_ALLOC sections (but the
  build-id note) SHA-256-identical base/after for all 19 ELF products; libc6-dbg 8.6 MB -> 11.0 MB; base
  check-system 29 FAIL.
* `sys-dbg-dwarf-recheck-2` rc 0 (packages of the after products with the final tools/deb, SOURCE_DATE_EPOCH set):
  check-system 0 FAIL; gdb with libc6-dbg alone gives a source line in each of the 19 objects; lintian trixie
  10E/46W (base 10E/61W): debug-file-with-no-debug-symbols 12 -> 0, undeclared-elf-prerequisites 1 -> 0,
  extended-description-line-too-long 2 -> 0. (`sys-dbg-dwarf-recheck` SUPERSEDED: no epoch, so build.sh's
  fallback epoch 1 gave 196 package-contains-ancient-file; and it tried only one function per object, which in
  libresolv was the C-layer niji_sys_syscall, built without -g.)
* `sys-dbg-dwarf-plants-2` rc 0: 5 of 5 CAUGHT, 0 WARN (`sys-dbg-dwarf-plants` SUPERSEDED: a separator typo in the
  check-every-elf line, fixed in ba2bb231d).
* `sys-dbg-dwarf-tests` rc 0 (`cargo xtask so`, `cargo test -p xtask`, niji deb_manifest, deb_symbols_products,
  cold_bss_products, heap_meta_products, hot_text_products): 604 passed, 0 failed. Gate: FULL: the train's
  (xtask changed).
* `sys-dbg-dwarf-lint-2`: clippy --all-features -D warnings, xtask clippy, static, records check on the final code.

## Shipped ELF files, each with `usr/lib/debug/.build-id/xx/yyyy.debug` in libc6-dbg

libc6: ld-linux-x86-64.so.2, libc.so.6, libm.so.6, libmvec.so.1, libpthread.so.0, libdl.so.2, librt.so.1,
libutil.so.1, libanl.so.1, libnsl.so.1, libresolv.so.2, libBrokenLocale.so.1, libc_malloc_debug.so.0;
libc-bin: getconf, getent, iconv, ldd, locale, ldconfig.

## Informational

* The C layer in libc.so.6 and libresolv.so.2 (csys) has no `-g`; those functions have no line in gdb.
* Coexist flavour: checked by the deb_manifest test (fast profile); the new flags are release-only.
* docs/install-packages.md:126's promise now holds.

## Next step

None for this story: the CI train runs the FULL gate.
