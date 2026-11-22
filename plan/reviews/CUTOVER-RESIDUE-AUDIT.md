# CUTOVER-RESIDUE-AUDIT: what the seven stories that went red at the 10-24 cutover never landed

Written 2026-11-21 (story CUTOVER-RESIDUE-AUDIT, an audit: no code change). Subjects: LEGACY-PROBE-COMPAT-2 (LG-0b),
LEGACY-OBJECTS-2 (LG-6), COMPAT-BEHAVIOUR-8 (LG-2 group 1), COMPAT-LIBM-3 (LG-4), COMPAT-BEHAVIOUR-G2 (LG-2
group 2), MUTATION-CAMPAIGN-2 and MUTATION-CAMPAIGN-2B. Plan rows: `plan/06-phases/18-phase-15-legacy-tier-and.md`
items 2, 7, 9, 11 and 24 (LG-0b, LG-2, LG-4, LG-6, LG-19).

## Verdict

**The premise holds for one story out of seven.** Six of the red heads reached the pre-cutover `next` on 10-24,
before the snapshot that became today's `origin/next`. So their code, tests, probes and symbol versions have been on
`origin/next` since its first commit. Today's products export every symbol version they added. Their ops state
`red` is stale. Only **COMPAT-BEHAVIOUR-G2** carries work that never landed: the old contracts of `regexec`,
`nftw` and `nftw64` @GLIBC_2.2.5 and `quick_exit`@GLIBC_2.10. That work has been on hold since owner ruling R14
(11-04): an old-behaviour version gets built only for a user that the LG-19 corpus finds. No LG-19 story exists yet,
and the LG-0c sweep found no host binary that binds any compat version.

Recommendation: close the six as landed (by content, evidence below). Close COMPAT-BEHAVIOUR-G2 too, and keep its
branch as a reference for a corpus-gated story. The open phase-15 rows that next still records as missing
(section "Still missing in these families") belong to the plan's LG rows. None of them is residue of these
branches.

## Method (content, not commits)

1. **How the red heads reached the old history.** Every story's last red head (the hashes in `ops log`) is an
   ancestor of the pre-cutover `next` (remote `historic`, branch `next`). To find where each one entered, a binary
   search on that branch's first-parent chain looked for the oldest commit that contains it:
   * LEGACY-PROBE-COMPAT-2 `f6a4fdd73`, LEGACY-OBJECTS-2 `d6e2deeda`, COMPAT-BEHAVIOUR-8 `d1512352d` and
     COMPAT-LIBM-3 `66c8d0ab2` all entered at first-parent commit `68cd7bafa` (2026-10-24 11:45Z).
   * MUTATION-CAMPAIGN-2 `3211b7585` and MUTATION-CAMPAIGN-2B `a84003867` entered with MUTATION-CAMPAIGN-2C's merge
     `5d8a12d0e` (13:20Z). CI recorded that merge as `landed` (CI GREEN, next=`fe2fb141b`, 13:47Z).
2. **The cutover snapshot.** The root of today's `origin/next`, `7374bf3f2` (10-24 19:41Z), differs from the old
   history's final `next` (`76041c218`, 19:38Z) only by 784 deleted files, all under `handoff/`. No code, spec, test
   or tool file differs.
3. **Change sets.** Each branch's own change set is the tree diff from the newest old `next` it had merged to its
   red head. For COMPAT-BEHAVIOUR-G2, a post-cutover branch, it is the diff from its merge base with `origin/next`.
   For every changed file, the check counted how many of the branch's added lines (6 characters or more) occur in
   the same file at the cutover root and on today's `origin/next` (`43058b7df`). Results are in the table below.
   `handoff/` files are left out (the cutover removed them all).
4. **Symbol versions on products.** The symbol versions each branch adds are the rows it removed from
   `spec/abi/compat-missing*.tsv`, which is the ratchet: a branch removes the names it now provides. `cargo xtask so`
   was run on `origin/next` `43058b7df` (story job `cutover-residue-audit-so`, rc 0). Then `readelf --dyn-syms -W`
   on its `libc.so.6` (sha256 `9327bbd0f7f208d5…`) and `libm.so.6` (`4ca8fe5c32534dbb…`) checked each name.
   The check needed three things: `sym@VER` is exported, it is not the default (`@@`) version, and for an `OBJECT`
   its `st_size` equals the size column of `spec/abi/versions.tsv`. COMPAT-BEHAVIOUR-G2's four rows are absent
   from the products, which shows that the check catches a missing version.

## Per story

| story | plan row | set out to do | branch content on `origin/next` today (added lines found: cutover root → today) | product check | status |
|---|---|---|---|---|---|
| LEGACY-PROBE-COMPAT-2 | LG-0b | black-box probes of every `behaviour` row; `spec/behaviour/compat.md` + `.glibc.txt`; static check on the probe list | `tools/probe/compat/{compat_libc,compat_libm,compat_thread}.c`, `LIST.tsv`, `run.sh`: 100% → 98-100%; `spec/behaviour/compat.glibc.txt` 1488/1488 → 1486/1488; `xtask/src/compatprobe.rs` 70/74 → 70/74; check `compat-probes` is registered in `cargo xtask static` | n/a (no exports) | **done** (landed with the 10-24 merge; carried by the cutover) |
| LEGACY-OBJECTS-2 | LG-6 | 31 data objects (`sys_errlist`/`sys_nerr`/`sys_siglist` families, the inert malloc hook variables, `_null_auth`, `__libc_sa_len`, the `__*_version_placeholder` names) with manifest sizes; check `compat_objects` | `spec/behaviour/compat-objects.*` 100% → 100%; `xtask/src/compat_objects.rs` 270/271 → 264/271; `crates/boundary/src/data.rs` 21/21 → 21/21; generated `csys/src/machine/compat_aliases.S` 855/856 → 295/856 (generator output reshaped since; the products decide) | **31/31** exported, compat version, `st_size` = manifest | **done** |
| COMPAT-BEHAVIOUR-8 | LG-2 group 1 | old contracts of `memcpy`, `realpath`, `glob`, `glob64`, `fmemopen` @GLIBC_2.2.5 in `niji-legacy::versioned`, with probes and tests | `crates/legacy/src/versioned.rs` 53/53 → 51/53; `stdio_compat.rs` 14/14; `core/src/pattern/glob.rs` 13/13; symbol tests in `crates/boundary/tests/s04/s12/s13/s14_*` 80-100%; `tools/probe/compat/compat_lg2.c` 361/361. The branch's 5 table lines in `xtask/src/compat.rs` were not in the cutover root (0/8): they were superseded by `spec/abi/compat-plan.tsv`, which lists all 5 as `behaviour`/LG-2 | **5/5** exported, compat version | **done** (the generator table moved to `compat-plan.tsv`) |
| COMPAT-LIBM-3 | LG-4 | the 110 `__*_finite` (and related) compat aliases in `libm.so.6`; the `xtask so` libm check | `crates/libm-so/compat_aliases.S` 886/886 → 221/886 (generated; reshaped since); `xtask/src/compat.rs` 87/90 → 65/90; `xtask/src/so.rs` 27/27 → 23/27; all 110 rows are in `compat-plan.tsv` as `alias`/LG-4 | **110/110** exported from `libm.so.6`, compat version | **done** |
| COMPAT-BEHAVIOUR-G2 | LG-2 group 2 | old contracts of `regexec`, `nftw`, `nftw64` @GLIBC_2.2.5, `quick_exit`@GLIBC_2.10 (partial: `posix_spawn{,p}`@GLIBC_2.2.5 not started; plants not run; its gate red) | **not on next**: `crates/legacy/src/versioned.rs` 2/34, `crates/boundary/src/posix/s01/s13/s14` and `process.rs` 1-2 of 5-11, symbol tests 6-9 of 18-32; all 4 rows are still in next's `spec/abi/compat-missing.tsv`. The branch is 4 commits on a base 5978 commits behind `origin/next` | **0/4** (absent) | **missing; corpus-gated (R14)** |
| MUTATION-CAMPAIGN-2 | — | cargo-mutants over the string, stdio and heap kernels; killing tests for real gaps | killing test in `crates/mem/src/lib.rs` 21/21 → 21/21 | n/a | **done** (landed with MUTATION-CAMPAIGN-2C, CI GREEN 10-24) |
| MUTATION-CAMPAIGN-2B | — | continuation: strz triage, stream reruns | `crates/core/tests/stream_engine.rs` 101/101 → 99/101; `crates/mem/src/lib.rs` 21/21 | n/a | **done** (same landing); the heap module's survivors were never triaged (below) |

The drop in the generated `compat_aliases.S` files is the generator's output format changing (LG-1's generator was
reworked after these branches). It is not lost content: the product check finds every version these branches added.

## Still missing in these families (today's `origin/next`, from its own ratchet files)

None of these items comes from the seven branches. They are listed so that closing the seven drops nothing.

* **COMPAT-BEHAVIOUR-G2's four contracts** (`nftw`/`nftw64`/`regexec`@GLIBC_2.2.5, `quick_exit`@GLIBC_2.10) and the
  rest of LG-2 group 2 and later groups. In total, 23 `behaviour`/LG-2 rows are still recorded missing: the 4
  above, `posix_spawn{,p}`@GLIBC_2.2.5, the 6 `sched_`/`pthread_*affinity`@GLIBC_2.3.3 names,
  `pthread_getattr_np`, `pthread_atfork` and the 2 `_pthread_cleanup_*` names @GLIBC_2.2.5,
  `lio_listio{,64}`@GLIBC_2.2.5 and the 5 `timer_*`@GLIBC_2.2.5 names. The plan's LG-2 row has been corpus-gated since R14
  (11-04). Other plan rows hold the other still-missing `behaviour` rows: LG-5 has 21 (the `__str*_c[123]` and
  `__*_small` helpers, `__sig{add,del}set`, `__sigismember`, `sigvec`, `ustat`, `vtimes`), LG-7 has `__ivaliduser`
  and LG-8 has `_IO_vfscanf`.
  **Which old binaries need them:** only binaries linked against a C library whose default for the name was the
  older node. By the manifest (`spec/abi/versions.tsv`), that means linked before `GLIBC_2.3.3` (`nftw`, `nftw64`),
  before `GLIBC_2.3.4` (`regexec`), before `GLIBC_2.24` (`quick_exit`@GLIBC_2.10) or before `GLIBC_2.15`
  (`posix_spawn{,p}`). The LG-0c sweep (`validation/results/bincompat/README.md`, two hosts, 1930-1957 dynamic
  ELFs) found 0 real binaries binding any compat version. So no user is known today.
* **LG-6:** 7 `object` rows: `__ctype_b`, `__ctype_tolower`, `__ctype_toupper`, `__ctype32_b`,
  `__ctype32_tolower`, `__ctype32_toupper` and `_obstack`, all @GLIBC_2.2.5. They are compat-only in the manifest
  (no default version), so only binaries from a very old C library bind them. LEGACY-OBJECTS-2 did not claim
  them. R14 does not gate LG-6, so a small story can take them when wanted (S).
* **LG-4:** 26 `behaviour` rows. They are the SVID-error-handling variants (`expf`, `exp2`, `exp2f`, `exp10f`,
  `logf`, `log2f`, `powf`, `lgamma{,f,l}`@GLIBC_2.2.5) and the by-value `totalorder*`/`totalordermag*`
  @GLIBC_2.25-2.27. There is also the `_LIB_VERSION` object and one stub. COMPAT-LIBM-3 delivered only the `alias`
  rows.
* **Mutation campaign:** MUTATION-CAMPAIGN-2C's hand-back left the heap fast path's 100 survivors untriaged. The
  list was in `handoff/mutation-campaign-2.md`, which the cutover removed with the rest of `handoff/`. It still
  exists on the old history's `next`. The heap code has changed since (HEAP-SPILL-COLD and other landings), so
  the 10-24 list is stale: any follow-up reruns cargo-mutants on today's heap module.

## Ranked table

| rank | story | still-missing items | why it matters | size | recommendation |
|---|---|---|---|---|---|
| 1 | COMPAT-BEHAVIOUR-G2 | old contracts `regexec`, `nftw`, `nftw64` @GLIBC_2.2.5, `quick_exit`@GLIBC_2.10 (branch, unplanted, gate red); `posix_spawn{,p}`@GLIBC_2.2.5 never started | P1 only for binaries linked before GLIBC_2.3.3 / 2.3.4 / 2.24 / 2.15. No known user (LG-0c: 0 compat binds). Corpus-gated by R14 | M (the branch would have to be brought forward 5978 commits, plus plants and posix_spawn) | **close**; keep `origin/task/compat-behaviour-g2` as a reference; a new corpus-gated story when LG-19 finds a user |
| 2 | MUTATION-CAMPAIGN-2B | heap fast-path survivors never triaged (stale list) | tests (mutation coverage of the heap fast path), not P1 | M | **close** (content landed via 2C); new story only if heap mutation coverage is wanted, rerun on today's heap |
| 3 | LEGACY-OBJECTS-2 | nothing from the branch (31/31 exported). Family: 7 LG-6 objects (`__ctype*_b/tolower/toupper`, `_obstack`) | P1 for very old binaries only; not corpus-gated | S (family item) | **close** as landed; family item to the LG-6 row (new story when phase 15 resumes) |
| 4 | COMPAT-LIBM-3 | nothing from the branch (110/110 exported). Family: 26 LG-4 `behaviour` rows + `_LIB_VERSION` | P1 for old binaries only | M (family item) | **close** as landed |
| 5 | COMPAT-BEHAVIOUR-8 | nothing (5/5 exported) | — | — | **close** as landed |
| 6 | LEGACY-PROBE-COMPAT-2 | nothing (probes, behaviour lines and `compat-probes` check on next) | — | — | **close** as landed |
| 7 | MUTATION-CAMPAIGN-2 | nothing (killing test on next; continued by 2B/2C) | — | — | **close** as landed |

## Process notes

* The 11-20 red triage read these stories as never landed because their ops state stayed `red` after their heads
  reached the old `next` through other merges. A check of the form `git merge-base --is-ancestor <red head>
  historic/next` (and, after the cutover, a content check like this one) would have caught that in seconds.
* The cutover dropped every `handoff/` file. Untriaged lists that lived only there (the heap survivors) can now
  be found only in the old history.
