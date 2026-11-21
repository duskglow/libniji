# SL-HEADERS-S05: generator and base layer (handoff after run 4, SL-HEADERS-S05-4)

**State.** Branch `task/sl-headers-s05-4` (stacked on `task/sl-headers-s05-3`, cut from origin/next 4a0a545ef). Head:
see `handoff/reports/SL-HEADERS-S05-4.json`. A continuation reads this page, plan/headers.md §2.3, §2.4 and §3, and
`xtask/src/headers.rs` by range (module doc first).

**Run 4: the S05-3 gate red, cause and fix.** Job `sl-headers-s05-3-gate` on mei escalated to FULL (xtask and
spec/abi changed) and failed 595 liella-boundary tests (native_v1 `products_export_the_node_and_pass_the_c_program`,
`s00_conventions_clauses s0_6_*`, `s01_startup_symbols`, ...). Cause: `conformance/run.sh`, `bench.rs`,
`nscache_products.rs`, `cxx_throw_products.rs` and others compile against the build host's headers with
`-I include` to reach `<liella/native.h>`; run 3 committed the generated `stdint.h`, `errno.h`, `features.h`, ...
under `include/`, so they shadowed the host's headers in every such compile. No convention clause needed to change.
Fix (commit "Move generated libc headers to libc-include/ ..."): the generated tree has its own root
`libc-include/` (plan/headers.md §2.4 "Output" updated, pending record updated); `include/` holds only
`liella/native.h` again. `headers-gen --check` now also refuses a file under `libc-include/` that no template makes.

**Decisions in force.** Pending record `records/pending/SL-HEADERS-S05-3-1.toml` (D-NEW, series D-X): marker format,
guards, `__liella/` reserved, table-only integer types (#6047), PTHREAD_STACK_MIN, gnu/stubs.h source,
`__STDC_ISO_10646__`, and (run 4) the `libc-include/` root and the orphan check. Differences row for
`__STDC_ISO_10646__` (no compat option, reason in the row).

**Done.**
* Runs 1-3: generator, tables, 16 templates, probe rerun (#6047), `spec/tools/header_check.py` (levels in 40 configs,
  P5 alone/twice, P1 C 7.20 identity), plants file `handoff/SL-HEADERS-S05/plants.txt` (its stubs-row message now
  names `libc-include/`).
* Run 4:
  * the fix above;
  * staging: `cargo xtask so` runs `headers::stage` first: `target/liella/include` = `libc-include/` + `include/`
    (a path in both is an error; built in a sibling dir and renamed into place) and `target/liella/uapi` = the build
    host's kernel UAPI dirs (`linux asm asm-generic drm misc mtd rdma sound video xen`, multiarch `asm` first);
    refuses a stale `libc-include/`;
  * `conformance/run.sh --liella-headers [names]`: `-nostdinc -isystem<staged include> -isystem<compiler include>
    -isystem<staged uapi>`; builds only programs whose every `#include <...>` those dirs hold (C only), lists the
    rest NOT-COVERED (not counted; a named uncovered program is a FAIL); `.lh` executables and logs;
  * `header_check.py --include DIR` (default `libc-include/`);
  * affected: `libc-include/` and `headers/` classified like `include/` (FULL);
  * unit test `headers::tests::stage_tree_copies_and_refuses_a_path_from_two_trees`; plants
    `handoff/SL-HEADERS-S05/plants-4.txt` (4 plants: orphan check, stage collision, stage replace, stubs row).

**Results (run 4, keke n2).**
* Inherited head c88734abc: clippy + static green (`sl-headers-s05-4-lint0`).
* `sl-headers-s05-4-verify` (head 5603bb99a): clippy, static, xtask headers unit tests, `so` (staged 17 headers +
  977 UAPI files), `run.sh s00_errno` 1 passed (it failed in the S05-3 gate). Its `--liella-headers` step found
  4 covered programs, 3 failing to build: the coverage check missed quoted local includes (fixed in acd83c089).
* `sl-headers-s05-4-verify2` (acd83c089): headers-gen --check ok; `--liella-headers` 1 passed, 0 failed, 136 not
  covered; default `s00_errno s02_stack_smash s24_hostname_chk` 3 passed; header_check on the staged tree 0 failed of 700.
* `sl-headers-s05-4-plants` (plants-4.txt): 4 CAUGHT, 0 SURVIVED, no WARN.
* FULL gate `sl-headers-s05-4-gate` (acd83c089; escalated: xtask changed): 6730 passed, 1 failed, 788 ignored
  (S05-3: 595 failed). The one failure, `xtask affected::tests::the_conformance_report_matches_the_runner_formats`,
  pins run.sh's `log="$here/build/$name.log"` line, which run 4 had rewritten; e2120d49f restores it (the `.lh` log
  is set after it).
* `sl-headers-s05-4-refix` (e2120d49f, the final code head): that test, `--liella-headers` (1 passed, 0 failed),
  `s00_errno s02_stack_smash` 2 passed, `cargo clippy --workspace --all-targets --all-features -D warnings`, static: rc 0.
  The FULL gate did not run again on e2120d49f (the change since acd83c089 is the one run.sh line above).

**Next steps, in order.**
1. Make header_check a cargo test (e.g. an xtask test or a liella-boundary test that runs it on the staged tree), so
   the direct tier selects it for `headers/`, `libc-include/` and `spec/tools/header_check.py` changes (today those
   paths escalate to FULL, which runs no header_check).
2. P5 rest: `-std=c89 -pedantic-errors` units and the strict-mode namespace check in header_check.py.
3. P1 rest: generated `_Static_assert` TUs for every macros.tsv value and typedefs.tsv size of the slice's headers.
4. P2/P3: `header_probe.py`/`callsite_probe.py` on the staged tree (an extra-flags option), compare with `omitted()`.
5. A conformance program covered by slice-5 headers alone (stdint/inttypes/limits/errno/endian/byteswap), so P6
   builds at least one program before slice 6 brings `stdio.h`/`string.h`.

**Open gaps.**
* `PTHREAD_STACK_MIN` under `_DYNAMIC_STACK_SIZE_SOURCE` expands to `__sysconf (75)`, which no slice-5 header
  declares: a program using it there needs `<unistd.h>` (slice 6) to declare `__sysconf`, or the template must
  change. Not closed in run 4.
* `FD_SET` and friends under fortify: plain macros; P3 decides.
* The probe plants of plants.txt (`hc-probe-*`) need the reference system's headers and run long; run 4 ran only
  plants-4.txt.

**Do not redo.** Level table and cascade (verified in all 40 configurations), the probe rerun (tables committed), the
`libc-include/` move.
