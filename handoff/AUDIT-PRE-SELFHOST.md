# AUDIT-PRE-SELFHOST: tests, tooling and CI steps that still work around a capability libniji has

Branch `task/audit-pre-selfhost` from `origin/next` 90f71362a. Audit only: no code changed. Role keke n2 (Zen 1,
60 vCPU), one probe job. Costs come from the CI FULL gate of 90f71362a on mei (`ci-gate-90f71362a-1795132798`,
1583 s step total, 18G peak) unless a row says "probe".

## Where the gate's time goes (CI, 90f71362a, measured)

| Step | Wall | Note |
|---|---|---|
| `test/tests-nextest` (pass 1) | 767 s | 6863 tests in 320 binaries, **12366 test-s** |
| `test/tests-interposed-*` (pass 2) | 349 s | owned by PASS2-NATIVE-RUNNER, not counted below |
| `test/products` (`cargo xtask so`) | 283 s | `so/libc-archive` 244 s of it |
| `test/rust-dylib` | 66 s | |
| `test/native-examples` | 52 s | includes a release (fat LTO) build of niji-boundary's `secure_zero_codegen` |
| `test/conformance` | 27 s | |
| clippy, cheap steps, scans | ~35 s | the cheap steps run twice when escalated (item 9) |

Pass-1 test-seconds by binary (top): `complex_ld_cr_products` 3768 (21 tests), `complex_f128_products` 1473 (8),
`xtask` 1397 (509), `special_f128_products` 800 (8), `special_ld_products` 771 (9), `niji-math` 293,
`aio_products` 286, `strstr_find` 243. The five oracle sweeps are **6930 test-s, 56 % of pass 1**, and the longest
single tests (171-237 s each), so they also set pass 1's tail.

## Ranked list (saving per size; S < 1 day, M a story, L several)

| # | Item | Where | Why it exists (record) | What replaced the reason | Est. saving (CI) | Risk | Size |
|---|---|---|---|---|---|---|---|
| 1 | ORACLE-VALUE-CACHE: the C-oracle sweeps recompute the multi-precision oracle for every line where libniji and the reference differ, on every run whose verdict key changed (any libniji byte) | `crates/math/tests/{complex_ld_cr,complex_f128,special_ld,special_f128,complex}_products.rs`, oracle `niji_mathtools::{mp,cplx}` | the verdict cache (D-10.57) skips a shard only when NOTHING changed, so every product change reruns all of it | n/a (not a capability item: the correctly rounded value of an input never changes; only the oracle's own code can change it) | up to ~6500 of the 6930 test-s and the pass-1 tail (171-237 s tests). Probe: per complex_ld shard the programs cost 4.5 s of ~190 s, so >97 % is in-process checking; 77396 of 146932 lines differ. Oracle share of that NOT VERIFIED (profile first) | med: a stale value is a wrong verdict: key by `niji-mathtools` bytes + (function, format, mode, operands); SLOW AND WORKING WINS applies, RULING NEEDED | M |
| 1b | ORACLE-SWEEP-RUN-ONCE: every shard compiles both oracle programs and runs both over the WHOLE grid, then checks its `shard::mine` share | same files (`build` + `run(&host), run(&ours)` before the share filter, complex_ld_cr 190-207) | sharding (D-X.36) split the check, not the runs | n/a (a run repeated per shard) | probe: 4.5 s x 19 redundant shards = ~85 test-s for complex_ld (0.8 s builds also run on cached shards); ~150-200 test-s over the five | low: outputs keyed like the verdict key, written once under a lock | S (or inside 1) |
| 1c | Plants rebuild everything cold | `xtask/src/plants.rs` 696 (copy without `/target`), 8-10 (`cargo xtask so` in the copy, again per products plant) | stale-mutant safety (XTASK-PLANTS, mtimes) | n/a | probe: a cold `cargo xtask so` is 1364 s wall / 4812 cpu-s on the Zen 1 host, before any test build; most is third-party and unchanged crates | med: seeding must never let a mutant be skipped (SLOW AND WORKING WINS): e.g. a read-only shared registry/deps build, never `target/` of the tree | M |
| 2 | Conformance dynamic mode re-makes products and links the host's `libc_nonshared.a` | `conformance/run.sh` 108-135 (`bypath`, `dyn_setup`) | "libniji's ld.so does not search for libraries yet": a SONAME-less copy plus a GROUP script naming the HOST `libc_nonshared.a` | the loader searches `DT_NEEDED` by name (RELEASE-NOTES "finds DT_NEEDED libraries by name", `LD_LIBRARY_PATH` for a build); `cargo xtask so` ships `target/niji/libc_nonshared.a`, the `libc.so` script and `libc-by-path.so` (`crt_products.rs` LIBS) | seconds; removes the host link-time archive from libniji's link | low: link against `target/niji/libc.so` (its own script) | S |
| 3 | native-examples 5b and nounwind.o | `xtask/src/native_examples.rs` doc 13-14, 44-48, 65-69 | by-path `libc.so.6` copy ("ld.so has no library search yet"); weak aborting `_Unwind_Resume` because libgcc_eh "needs dl_iterate_phdr and _dl_find_object, which arrive with phase 14"; no `std` example | loader search (as 2); probe: `getcwd`, `realpath`, `dl_iterate_phdr`, `_dl_find_object` are defined in both `libc.a` and `libc.so.6` now; `libc.a`'s strong `rust_eh_personality` remains (the static `std` collision) | ~0 s; drops a workaround, can add a `std` example (STD-SMOKE of TEST-PASS2-NATIVE) | low | S |
| 4 | rust-dylib's program is linked, never run | `xtask/src/rust_dylib.rs` 32, 755 | "running it needs libniji's dynamic loader (phase 14)" | the loader exists; TEST-PASS2-NATIVE ran 317 `std` test binaries under it | costs a few s; gains a run of the shipped Rust ABI | low | S |
| 5 | `bench --compare` never measures libniji's `libc.so.6` (static libniji vs static and dynamic glibc only) | `xtask/src/bench.rs` 643 | "needs libniji's dynamic loader (phase 14); the host loader refuses it" | the loader exists | none (adds a column users run: PLT, IFUNC through ld.so) | med: new rows need levels; ALWAYS FORWARD | M |
| 6 | Workload gate and per-call counts measure libniji STATIC against glibc DYNAMIC | `validation/wl_prep.sh` (kit relinked against libc.a), `wl_percall.sh` 4, 32; `scripts/niji-cc` 2-3 ("dynamic linking waits for phase 14"); `validation/suite.sh`, `smoke/` through niji-cc | phase 14 did not exist | the loader exists | none in time; the INVIOLABLE gate then measures the path users run (start-up through ld.so, PLT) | high: every recorded level moves: RULING NEEDED | L |
| 7 | Prefixed-interpreter trace test rebuilds the loader | `crates/core/tests/hot_text_products.rs` 337-358 (`cargo xtask ldso-trace --prefix /usr/lib/niji`) | the coexist package's prefix is compiled in (LDSO-TRACE-PREFIX) | n/a; PASS2-NATIVE-RUNNER will add a second prefixed loader build per gate | 97 test-s now; one build instead of two after PASS2-NATIVE-RUNNER | med | M |
| 8 | Validation stand-ins and modes `niji-w`/`niji-t1` | `validation/shim/{missing,libm}.c`, `suite.sh` 12-21, 77-85 | "Track M's libm is not a product yet (T3)" | `libm.a`/`libm.so.6` are products; `suite.sh` 72-73 already says as-is needs no shim | none (dead code, provisional-result modes) | low | S |
| 9 | Escalated FULL gate runs the cheap steps twice | `affected` escalation prechecks then `test_steps` (`main.rs` 339) | each entry point runs `CHEAP_STEPS` itself | n/a | ~8 s per FULL gate | low | S |
| 10 | xtask unit tests: `affected::tests` 25-90 s each | `xtask/src/affected.rs` tests (`csys_falls_back_to_full_when_unsure` 90 s, ...) | each test recomputes selections over the live tree | n/a (test-only path) | up to ~1000 of 1397 test-s if the tree index is built once per process: NOT VERIFIED (not profiled) | low | M |

Goals served (liella-ops GOALS.md, coordinator note 11-20): 1 E1, E3 (gate wall and test-s); 1b E1; 1c E3, E1 (a
cold product build per plants run; distinct from the queued PLANTS-COPIES, which is about two concurrent copies, but
the same code: merge if the coordinator prefers); 2, 3, 4 E2 (phase-14 workarounds, tests not run the way users run);
5, 6 E2 (measurement of the path users run), 6 also E1 (what the INVIOLABLE gate trusts); 7 E1; 8 E2; 9 E1; 10 E1, E3.

Not listed: pass 2 (`tests-interposed-*`, its unchanged-scan and doc run, `main.rs` 347-350, D-4.13), owned by
PASS2-NATIVE-RUNNER. Designed host-libc use that stays: the reference side of every oracle and bench (glibc is the
comparison), `wl_entries`/`wl_shapes` LD_PRELOAD observers (they profile the workloads on the glibc kit), conformance's
default build against the host's headers (`--niji-headers` exists; the gate never runs it: a candidate once headers
cover the suite, not costed here).

## Observations for the coordinator (liella-ops, not in the tree)

- `~/bin/jobwait --help` does not print usage: it waited like a label (my call blocked until moved to the background).
- CI's FULL gate log interleaves the prechecks and `test`'s own copy of the same cheap steps (item 9): one list
  would also make the step summary shorter.

## Probe (keke n2, job `audit-pre-selfhost-probe`)

One job on 90f71362a: `cargo xtask so` cold (1364 s wall, 4812 cpu-s, 1.7 GB peak), then:
- symbols defined (`nm`): `getcwd`, `realpath`, `dl_iterate_phdr`, `_dl_find_object` in `libc.a` and `libc.so.6`;
  `rust_eh_personality` in `libc.a` only; `_Unwind_Resume` in neither.
- complex_ld_cr oracle pair, once: build host 0.20 s, build libniji 0.58 s, run host 0.32 s, run libniji 3.31 s
  (wall); 146932 lines each, 77396 differ (go to the oracle).
- a program linked against `target/niji/libc.so` (the products' own script, no by-path copy) with libniji's `ld.so`
  as interpreter: `NEEDED libc.so.6`, runs with `LD_LIBRARY_PATH=target/niji` (rc 0); without it the loader
  refuses (`libc.so.6: cannot open`, rc 127), as the recorded rule says (libniji's libraries only from its own
  directory or `LD_LIBRARY_PATH`). Items 2 and 3 can drop the by-path copies and set `LD_LIBRARY_PATH`.

## RULING NEEDED

- Item 1 (a cache of oracle VALUES that survives product changes): recommend yes, keyed by the oracle crate's bytes
  and the exact input, after a profile confirms the oracle dominates the 97 % in-process share.
- Item 6 (static vs dynamic measurement in the workload gate): recommend adding a libniji-dynamic kit beside the static
  one first, recording its levels for one train, then deciding which one the INVIOLABLE rule binds.

## Next step

The coordinator picks items. Order by saving per size: 1 (profile the sweeps first; M, up to ~6500 test-s), 1b (S,
~150-200 test-s, or inside 1), 1c (M, a cold product build per plants run), 2-4 together (S, one story: "drop the
phase-14 workarounds"; coverage, ~0 s), 9 (S, ~8 s), 7 (M, 97 test-s), 10 (M, unverified), 5 and 6 (fidelity, no
saving; 6 needs the ruling), 8 (S, cleanup).

Not to redo: the per-shard program cost is measured (4.5 s for complex_ld); the static/dynamic symbol and loader-search
facts above are settled on 90f71362a.
