# DL-ATEXIT-OWNER: registering exit/fork handlers without libc_nonshared.a (owner rulings #4860-#4862)

**State (run 2, DL-ATEXIT-OWNER-2, 2026-11-14).** Branch `task/dl-atexit-owner-2` (continues `task/dl-atexit-owner`,
from origin/next f53c74611), role shiki:~/w/sh2. Head and job verdicts: handoff/reports/DL-ATEXIT-OWNER-2.json.
Read first: handoff/DL-NONSHARED-OPTOUT.md (design, probes P1-P3), this note, D-27.29
(records/decisions/D-27/D-27.29.toml: the whole design in force). Prerequisites (DL-LIBC-NONSHARED D-27.28, DL-17
dlclose, DL-16c `_dl_find_object`, DL-ATFORK-FINALIZE) are on next.

## Built (runs 1 and 2)
* Exports `atexit`/`at_quick_exit`/`pthread_atfork@LIELLA_OWNER_1` (non-default; `csys/owner/owner.S` shims pass the
  return address), attribution through `_dl_find_object` (`crates/boundary/src/owner.rs`), the unload net after each
  object's finalisers on `dlclose` (`__liella_owner_unload`, `Finis::run` in `crates/boundary/src/ld/dlopen.rs`,
  `finalize_range` in `crates/legacy/src/exit_handlers.rs`, `fork::forget_opted`), the link-stub directory
  `target/liella/liella-owner/` (`libc-owner.so` + `libc.so` script). Details: D-27.29.
* Run 2 fixes: (1) the red `dl-atexit-owner-build-2`: `check_compat_exports` refused the three pairs (no versions.tsv
  compat row). They are now registered pairs (`owner_registered()` in xtask/src/so.rs: exactly those three at that
  node; any other non-default export still fails; `take_owner_node` checks each exported non-default). The check was
  not loosened. (2) `__liella_owner_unload` was missing from `LIBC_PRIVATE` (so.rs), so it was local and the loader's
  lookup could never find it: the net would never have run. Listed now (plant `unload-step-not-exported`).
* Tests: `crates/ld/tests/atexit_owner.rs` (under Liella: attribution also from a constructor under dlopen's lock,
  the handler-address net, stub entries owner-only (#4862), no quick-exit/atfork call after unmap, exit path once);
  `s1_2_c_5_owner_optin_link_and_reference_refusal` (products: the stub directory gives one `libc.so.6` need and
  `LIELLA_OWNER_1`, no archive on the link line; on the reference the program is refused naming the node, P3).
  The default link's two-way check stays `s1_2_c_5_nonshared_archive_two_way` (needs only GLIBC_*).
* Records: D-27.29 (reserved id), differences row (docs/differences-from-glibc.md, dynamic loader block),
  records/pending/DL-ATEXIT-OWNER-2-1.toml (COMPAT-NEW, restore = the default link; CI assigns the id).
* Plants: handoff/DL-ATEXIT-OWNER-2.plants (8: net on stub entries, owner-only net, quick/atfork kept after unload,
  misattribution, unload step not called / not exported, opt-in export made default).

## Open (next run, in order)
1. If the jobs named in the report json are not all green: fix from their logs (`~/logs/<label>.log` on shiki).
   Unverified until check1 ran: that Liella's loader matches the test stub's soname (the libc copy's path) to the
   loaded C library (one object), and that `ld` emits ONE `DT_NEEDED libc.so.6` for the shipped stub + libc.so.6.
2. Start-up checks on the final head (touches the loader): `cargo xtask test` and
   `LIELLA_FULL_PROFILE=1 cargo xtask so && LIELLA_FULL_PROFILE=1 cargo test -p liella --test hot_text_products`.
3. #4861: an opt-in build setting (env var for `cargo xtask so`) adding the allocated `.note.liella.nonshared` note and
   `.gnu.warning.<sym>` sections to the archive members (off by default), and an `ld.so` debug report of modules
   still using stubs. Not built.
4. User docs (the opt-in, the migration path of the OPTOUT note section 5, nothing deprecated by default) and a
   paragraph in spec/behaviour/dynlink.md. Not written.
5. Not planted: a registration under the loader lock that deadlocks (module_of is lock-free by construction; the
   constructor registration in atexit_owner.rs runs under dlopen's lock with an `alarm` hang guard). Find a one-site
   plant (e.g. a locking lookup in place of `LOADER_FIND`) or record it as not plantable.
6. `liella_legacy::owner_proto` (selftest prototype) is still there: drop it once the product tests are green.

## Not to redo
Probes P1-P3, the prerequisite check, the compat-export registration design above.
