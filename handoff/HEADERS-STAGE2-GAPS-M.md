# HEADERS-STAGE2-GAPS-M: handoff (run 1 complete; leg 4 = proof and hand-back)

Branch task/headers-stage2-gaps-m, cut from origin/next af6f8acaf, no merge. Final head: see the report json
(`handoff/reports/HEADERS-STAGE2-GAPS-M.json`). Role shiki:~/w/sh1 (stock glibc 2.39 for black-box probes).
Plants: `handoff/HEADERS-STAGE2-GAPS-M-plants.txt`. Report: this page.

## What the branch does
* Rows L2-L9, L11-L13 and L15 of CONFIGURE-DIFF's libniji-class table, each matched to stock 2.39 per
  configuration (black-box probes only), and B1 (#8813 / owner on #8823): an `abort()` reached again in one process
  skips the SIGABRT handler and ends the process by SIGABRT (D-1.15, differences row; test `s1_2_c_4_1_again`).
* Leg 4: header_check fragment `spec/tools/header_checks/s13.d/80-stage2-gaps-m.py` (units per item; `problems()`
  for the L13 warning per combination, L2 warn_unused_result with and without fortification (full compiles: GCC
  reports unused results after parsing, `-fsyntax-only` shows none) and the L15 SIGTSTP text). Stand-alone
  `--reference` passes against stock 2.39's headers, so its expectations are stock's. header_check.py and
  features.py pass `-Wno-cpp` only for flags defining _BSD_SOURCE / _SVID_SOURCE without _DEFAULT_SOURCE.
  Provenance fragment `spec/abi/provenance.d/HEADERS-STAGE2-GAPS-M.txt` (order 230).
* Two defects the gate found, fixed: (1) libresolv.so.2 linked niji-rt's abort and so a second copy of the
  first-abort flag (`cargo xtask so` refuses writable state in libresolv): in the libresolv product `ret::abort` now
  calls libc.so.6's exported `abort`, and its panic handler writes the message then calls it; libc.so.6's code is
  unchanged (the other cfg arm is the old function). (2) tools/deb/manifest.tsv rows for include/bits/types.h and
  include/bits/niji-sigtstp.h (multiarch include dir), without which libc6-dev lacked them.

## Proof
* header_check whole (leg 4, before the two fixes, which touch no header): 0 failed of 26015. Fragment: 0 failed
  on libniji and on stock (`--reference`).
* Gate, plants: the jobs named in the hand-back (gate3, plants3 on the final head). gate2 (head e61a8c582) was red
  only on the manifest tests (fixed in the last commit); the first gate was red on the libresolv build check.
* CONFIGURE-DIFF rerun (mei, `cdiff.sh` over the 45 packages that differed, this branch's system debs repackaged
  from the role build with the manifest fix, check-system PASS): differing 220 -> 103 probes; 120 rows gone, every
  L2-L13, L15 and B1 row among them; the 100 kept are the recorded classes only: R1 fflush/fclose stdin and the
  derived `_fseeki64` / HAVE_FSEEKO rows, R4 rwlock order, perl Gconvert (D-8.10) and sig_name (D-20.1), fakechroot
  dlmopen, texinfo EUC-CN, libsigsegv's 2.41 host string, readline's i386 `build32` flavour (25), coreutils
  `gl_cv_clean_version_stddef` (libniji's answer is the one the probe wants), p11-kit L14. Stock-vs-stock control:
  2 differing. New rows (3): libunistring `gl_cv_func_pthread_rwlock_good_waitqueue` and its config.h define (R4,
  which the report lists as noisy on stock in libunistring) and diffutils `am_cv_filesystem_timestamp_resolution`
  (stock 0.2, libniji 0.1: automake's timing-based file-system probe; not chased, INFORMATIONAL).
* L7 list: over the 438 packages only `<bits/types.h>` differed (openldap's header check, psmisc's struct
  user_regs_struct test); in the rerun's config.logs the only direct `<bits/...>` include is `<bits/types.h>`
  (psmisc). Stock's bits/types.h also defines internal `__*_TYPE` / `_BITS_*_H` macros: not shipped (reserved
  names, no package use seen).

## Decisions in force
D-1.15 (B1). Coordinator rulings #8813, #8814 (L14 not a gap), #8815, #8776 (`__USE_TIME_BITS64`), owner on #8823.

## Open / not done
* B1 variants raise-then-abort and longjmp-then-abort were not probed on 2.41 (leg 1's mmdebstrap chroot on mei was
  refused, not retried); D-1.15 and the differences row state the rule "a second abort in one process ends it".
* L12: c89/gnu89 + `_XOPEN_SOURCE=500` per-name rows (snprintf etc.) remain: ftm-configs.tsv has no C90+X/Open
  configuration. Not chased.
* libm.so.6's internal-error path still runs niji-rt's own abort copy (its own first-abort flag); only reachable
  from a libm bug. Not changed.

## Not to redo
Probes per item (leg 2-3: `cmp.sh` on shiki, 0 DIFF), the debs (`/dev/shm/claude/headers-stage2-gaps-m/debs2` on
shiki), the cdiff rerun (`/mnt/zram/claude/headers-stage2-gaps-m/cdiff1` on mei).

## Next step
None for this story: land through the train.
