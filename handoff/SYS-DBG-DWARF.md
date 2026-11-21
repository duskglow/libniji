# SYS-DBG-DWARF: every libc6-dbg debug file carries DWARF (leg 1 done; leg 2 = jobs and hand-back)

Branch `task/sys-dbg-dwarf` (from origin/next df83365cd), role mei:~/w/s3 (= /mnt/zram/claude/w/s3).
Brief `~/state/briefs/SYS-DBG-DWARF.md`. Scratch `~/w/.scratch/SYS-DBG-DWARF/` (local), job scripts mei
`~/work/SYS-DBG-DWARF/` (pkg.sh, proof.sh, recheck.sh, plants.txt).

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
* Lintian: libc6 override for libmvec.so.1 `(libc.so.6 libm.so.6)`; libc6 description rewrapped;
  95a1fb95a: libc-bin description rewrapped (it had the same tag) and names locale.

## Results so far (mei)

* `sys-dbg-dwarf-base` (next's system debs, ~/w/s3/target/sdd/base; copy /mnt/zram/claude/SYS-DBG-DWARF/base-debs) rc 0.
* `sys-dbg-dwarf-proof` (after build of 75af35f67, ~/w/s3/target/sdd/after): allocated sections (every SHF_ALLOC
  but the build-id note, SHA-256) IDENTICAL base/after for all 19 ELF products; 19 libc6-dbg files, each with
  .debug_info + .debug_line (CUs: libc 613, libm 610, libresolv 611, ld 9, ldconfig 10, getconf/ldd 29, C ones 1);
  check-system after PASS (0 FAIL), base 29 FAIL (12 x no .debug_info, 12 x no .debug_line, 17 vs 19 files, text);
  lintian trixie base 10E/61W -> after 10E/47W: debug-file-with-no-debug-symbols 12 -> 0,
  undeclared-elf-prerequisites 1 -> 0, extended-description 2 -> 1 (libc-bin, fixed in 95a1fb95a, not yet re-linted).
  rc 1 ONLY because the proof's gdb step used versioned names (`x@GLIBC_2.2.5`) for 10 objects (proof-script bug;
  the 9 others printed a line, e.g. libm acos `boundary/src/export.rs:38`). recheck.sh fixes it (address form).
  SUPERSEDED in the hand-back (reason: proof-script gdb naming bug; data above still holds).
* libc6-dbg .deb 8.6 MB -> 11.0 MB.
* `sys-dbg-dwarf-lint` (clippy --all-features -D warnings, xtask clippy, static) rc 0 on 75af35f67.
* `sys-dbg-dwarf-plants` (handoff/SYS-DBG-DWARF-plants.txt, 5 plants, env NIJI_FULL_PROFILE=1 NIJI_PREFIX=/
  NIJI_LIBDIR=lib/x86_64-linux-gnu, dir /mnt/zram/claude/SYS-DBG-DWARF/plants-1) RUNNING at refresh (baselines ok).

## Exact next steps (leg 2)

1. `~/bin/jobwait sys-dbg-dwarf-plants` on mei; read verdicts (`grep -E 'CAUGHT|SURVIVED|WARN' ~/logs/sys-dbg-dwarf-plants.log`).
   Copy the verdict lines into handoff/SYS-DBG-DWARF-plants.txt's header if wanted.
2. Sync (rc 4 = BUSY while my jobs run): `bash ~/src/liella-ops/tools/sync_role.sh --story SYS-DBG-DWARF --base origin/next <wt> mei s3`.
3. Start `sys-dbg-dwarf-recheck` (`~/bin/jobrun sys-dbg-dwarf-recheck build -- sh ~/work/SYS-DBG-DWARF/recheck.sh`):
   expect check-system PASS, a gdb line per object, lintian 0 of the three tags.
4. Gate: `cargo xtask affected --base origin/next` decides; xtask changed, so likely `FULL (escalated`: then
   kind `test` job: `cargo xtask so && cargo test -p xtask && cargo test -p niji --test deb_manifest` (check the
   package name of crates/core) plus `cargo xtask precheck --jobs <labels> --head <sha>`; write
   `FULL: the train's (xtask changed)`.
5. D-row: none written yet (`ops reserve --story SYS-DBG-DWARF D-X` for "C products get -g1 in release" or say pending).
6. Report json handoff/reports/SYS-DBG-DWARF.json (+ "jobs"), handback-check, hand-back with the ELF list:
   libc6: ld-linux-x86-64.so.2, libc.so.6, libm.so.6, libmvec.so.1, libpthread.so.0, libdl.so.2, librt.so.1,
   libutil.so.1, libanl.so.1, libnsl.so.1, libresolv.so.2, libBrokenLocale.so.1, libc_malloc_debug.so.0;
   libc-bin: getconf, getent, iconv, ldd, locale, ldconfig: each `.build-id/xx/yyyy.debug` in libc6-dbg.

## Informational

* The C layer compiled into libc.so.6 (csys via the cargo build) still has no `-g`; libc's DWARF is the Rust part.
* docs/install-packages.md:126's promise now holds.
