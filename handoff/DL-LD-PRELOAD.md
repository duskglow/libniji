# DL-LD-PRELOAD: LD_PRELOAD and /etc/ld.so.preload in Liella's ld.so (handoff, run 3: DL-LD-PRELOAD-3)

State: branch `task/dl-ld-preload-3` (stacks on `task/dl-ld-preload-2`: run 1's two WIP commits, run 2,
run 3), role keke:~/w/n3. Code, tests, docs and the fakeroot case are committed and pushed; run 3 fixed
the secure-mode test's dependence on where the tree lives and reran every proof job on the code head.
Full report: this file ("Run 3 jobs") and handoff/reports/DL-LD-PRELOAD-3.json.

## Decisions in force
- plan/ph14/threat-model-draft.md rule 1 (LD_PRELOAD secure exception), rule 5 (trusted directories);
  spec/behaviour/dynlink-secure.md (secure-mode facts); spec/behaviour/dynlink-preload.md (run 1's
  probe record; probe scripts in tools/probe/dl/preload/); D-27.16 / D-X.95 (pinned sonames resolve
  to `<prefix>/<libdir>` only). No ruling was needed.

## What run 3 found and fixed
- Run 2's plants baseline failed `secure_preload_only_trusted_set_user_id` (exit 127, "libc.so.6:
  cannot open shared object file") in the RAM copy under /dev/shm. Cause: /dev/shm is mounted
  `nosuid`, so the kernel ignored the program's set-group-ID bit, the program ran WITHOUT AT_SECURE,
  `libz.so.1` from LD_PRELOAD loaded as an ordinary preload, and its `libc.so.6` dependency (a pinned
  soname) went to the build's `<prefix>/lib` = `/lib`, which holds no Liella libc: fatal before the
  test's old AT_SECURE check (after the preload run) could skip. Not a loader defect: secure mode
  was never entered.
- Fix (test only, crates/ld/tests/preload.rs): before the preload case the test proves AT_SECURE
  with a plain run of the set-group-ID program; it tries the corpus directory, then a copy under the
  temporary directory (the program names its libraries and interpreter by absolute path), and
  SKIPs with the reason only when neither honours the bit. docs/testing-filesystems.md has the row.
- Run 2's gate went red only on `cargo xtask static` (a test name that was not a clause ID and a
  behaviour record without one); run 2's last commit had already fixed both; run 3's lint0 job
  confirmed the inherited head clean.

## Run 3 jobs (keke, role n3; code head 12fce6d28, later commits docs/handoff only)
- dl-ld-preload-3-lint0: inherited head 0f65826ae, `cargo xtask clippy` + `cargo xtask static`: rc 0.
- dl-ld-preload-3-plants: handoff/DL-LD-PRELOAD-2-plants.txt in /dev/shm/claude/DL-LD-PRELOAD-3/plants-1
  (the nosuid RAM copy that failed before): baseline ok; 10 plants, 9 CAUGHT, 0 SURVIVED, 1 WARN-NO-MSG,
  rc 0; results in handoff/DL-LD-PRELOAD-3-plants.txt. `mode-check-dropped` was caught by the secure
  test alone, so the test ran in secure mode from the copy (through the temporary-directory copy).
  WARN-NO-MSG `preload-after-needed`: caught by `ldso_run_reads_ld_settings`; cargo stops at the first
  failing test binary (ldso_run runs before preload), so the regex's preload test never ran. A later
  plants file should pass `--no-fail-fast` or name that test too.
- dl-ld-preload-3-fakeroot: system .debs from 12fce6d28, swap root trixie: `RESULT same=49 differ=0`,
  fakeroot-1 and fakeroot-2 PASS; rc 0.
- dl-ld-preload-3-gate (`~/bin/gate`, ~/work/DL-LD-PRELOAD-3/gate.sh): direct tier `affected --base
  origin/next --run` (5 suites, 164 tests, static ok), then `cargo xtask test`, then
  `LIELLA_FULL_PROFILE=1 cargo xtask so` + `hot_text_products`: all green, rc 0 (direct 164 tests; xtask test 6657 + 5240 interposed passed; hot_text_products 26 passed; peak 13G).
- dl-ld-preload-3-lint1: final head, clippy (all targets, all features) + static: see the hand-back (run after the last commit).

## Things not to redo
- Do not probe the reference again (run 1's record is complete).
- A segfault seen in run 2 was the HOST's programs inheriting LD_PRELOAD of a Liella-linked library;
  the file test sets LD_PRELOAD only for the final `exec`.
- Names from the preload file point into its mapping, which `link` unmaps; nothing keeps a name
  past that point.

## Next step (exact)
- None for this story once the named jobs are green (see the report). Coordinator: remove
  `LD_PRELOAD` from the "Not available yet" list in liella-ops release notes when this lands.

## Open
- INFORMATIONAL: `ld.so --list` / LD_TRACE_LOADED_OBJECTS does not show preloads; `ld.so --preload`
  is not done (follow-up story if wanted).
- Plant not run: an edge from the program to a preload is equivalent for the order (the root is
  placed last by init_order).
