# DOCS-CURRENT: public docs brought in line with the tree (README first)

State: branch `task/docs-current`, cut from `origin/next` at f494ad65d; docs only, no code. Commands run on this
head in job `docs-current-cmds` (keke, role n1): `cargo xtask stubs`, `so`, `affected --list --base origin/next`,
`status`, `symgap`, `native`, `help`, `static`, `clippy`.

## Audit table

Scope: README.md, CONTRIBUTING.md, SECURITY.md, CLAUDE.md, docs/*.md (not compat-flags, saturation register,
RELEASE-NOTES, codemap), validation/*.md, tools/probe/README.md (crates/ has no README). Every in-scope file had its
relative links, `cargo xtask <sub>` names, backticked repo paths and code-block script paths checked mechanically
(scratch checker; 0 broken links, 0 unknown subcommands). Prose was checked against the code and the 11-15 release
notes for the loader, preload, utilities, headers and packages; see "Not audited" for the limit.

| File:line (before) | Statement | State | Fix / command run |
|---|---|---|---|
| README.md:17 | `LD_PRELOAD` not provided yet | stale (DL-LD-PRELOAD landed: crates/ld/tests/preload.rs, docs/install-packages.md) | "honours `LD_PRELOAD`"; `LD_AUDIT` named instead |
| README.md:21 | no C headers of its own yet | current | pointer to plan/headers.md added |
| README.md:39 | phase 14 row: `LD_PRELOAD` not provided | stale | moved to provided |
| README.md:57-66 | repository list has no loader or utilities | stale (crates/ld, csys/util/getconf.c, crates/ldconfig-bin) | one bullet added |
| README.md:74 | `cargo xtask so` builds libc.a, libc.so.6, liella-crt1.o | stale (also ld.so, libm, rcrt1, getconf, ldconfig: xtask/src/so.rs) | comment updated; ran `so` |
| README.md:73-77 | stubs / affected --list | current | ran both |
| README.md:75, CONTRIBUTING.md:72 | `cargo xtask test` (full gate) | current (subcommand exists) | not run: FULL gate is the release manager's |
| README.md:46-47 | `cargo xtask status`, `symgap` | current | ran both |
| README.md:80 | `scripts/dev.sh xtask test` | current (file exists) | not run (same FULL gate) |
| README.md:40 | Phase 15 row "has not been demonstrated" | inaccurate, positioning | left; RULING NEEDED 1 |
| CLAUDE.md:53, CONTRIBUTING.md:71 | `so` builds three products | stale | same comment fix |
| CONTRIBUTING.md:11 | phases 14 and 15 are the largest open areas | stale (loader runs real programs) | open areas restated: phase 15, rest of 14, headers, utilities |
| CONTRIBUTING.md:80 | `cargo xtask help` lists the rest | current (unknown/no arg prints usage) | ran `help` |
| CONTRIBUTING.md:116 | direct tier: `.S` mapping and interposed re-run "not implemented yet" | stale (affected.rs rule 6; interposed pass skipped outside mem/rt/kernel/abi/boundary) | "partly implemented"; rest still planned |
| CONTRIBUTING.md:163 | `tests/support/timeout.rs` | path incomplete | `crates/core/tests/support/timeout.rs` |
| CONTRIBUTING.md:176-188 | timing on Ivy Bridge and N100 only | stale (Zen 3 sweeps in release notes) | Zen 3 row and mention added |
| SECURITY.md | reporting process | policy, not changed | none |
| docs/architecture.md:62-66 | status: before phase 14, dynamic programs cannot run | stale | phase 14 runs real programs; dynamic programs run under Liella's ld.so |
| docs/architecture.md:128 | `liella-ld`: "not yet written" | stale | describes loader, dlopen, ldconfig engine |
| docs/architecture.md:561 | `LD_PRELOAD` not provided | stale | moved to provided |
| docs/architecture.md:562 | compat versions "only relevant once the loader exists" | stale | clause removed |
| docs/architecture.md:565 | `std` waits for getcwd/realpath/dl_iterate_phdr/_dl_find_object | stale (toolchain.md §5: defined now) | "defined now, no `std` program linked yet" |
| docs/interfaces.md:241-246 | dynamic ways cannot run until phase 14 | stale for ways 1 and 3 (crates/ld/tests; `cargo xtask native` step 5b) | per-row: run under ld.so / not run in the gate yet; ran `native` |
| docs/interfaces.md:292 | dynamic programs cannot run until phase 14 | stale | dylib programs linked but not run |
| docs/toolchain.md:67-69 | `so` builds "the three products" | stale | names ld.so, libm, getconf, ldconfig too |
| docs/toolchain.md:181 | native C dynamic program "cannot run yet" | stale reason | runs only under Liella's interpreter |
| docs/toolchain.md:536-540 | §8: no dynamically linked program runs | stale | narrowed to dylib programs and §4 links; `dynamic` example runs |
| docs/install-packages.md:119 | libc-bin's ldconfig trigger "a no-op until Liella ships ldconfig" | stale (tools/deb/system.sh ships /usr/sbin/ldconfig, /usr/bin/getconf) | contents and trigger restated |
| docs/install-packages.md:108,140 | pending decisions RELEASE-PACKAGING-DEB-1, SL-FLAVOUR-1 | stale (assigned) | D-X.94, D-X.106 |
| docs/install-packages.md:33,120 | no C headers yet | current | none |
| docs/differences-from-glibc.md:304,325,326,523-526 | nine `records/pending/*.toml` refs | stale (files gone; ids assigned) | D-27.33, COMPAT-22, D-27.40, COMPAT-28, D-31.4..D-31.7 (matched by story and body) |
| docs/plants.md:128-129 | example plant paths `crates/core/src/stdio/stream.rs`, `machine/memchr.S` | stale (files do not exist) | `crates/boundary/src/stdio.rs`, `csys/src/machine/scan.S` (patterns stay illustrative) |
| validation/README.md:6-8 | dynamic linking "waits for phase 14" | stale | loader landed; dynamic runs in release notes; items below wait for a dynamic re-run |
| validation/README.md:131-511 | T4/T17/T27/T33 "phase 14" | dated record of the static runs | left as the record (header now explains) |
| docs/spec-symbol-gaps.md:86,94,97; docs/tunables.md:15,36 | fpu_control dynamic run a reported skip; libresolv.so.2 not shipped "(phase 14)" | NOT VERIFIED | left; next step |

Not audited beyond the mechanical checks (budget): the prose of docs/benchmarks.md, docs/PRINCIPLES.md (policy),
docs/clean-room.md (policy), docs/fuzzing.md, docs/testing*.md, docs/perf-ledger.md, docs/nextest.md,
docs/transcripts.md, docs/arch-*.md, docs/wordexp-profile.md, docs/libm-fastpath-plants.md, the rest of
validation/README.md, validation/profiles.md, validation/headers-corpus-README.md, tools/probe/README.md.

## RULING NEEDED (one, positioning sentences)

1. README.md table, Phase 15 row: "Running unmodified glibc-linked binaries is the project's aim and has not been
   demonstrated." Inaccurate: the 11-15 release ran 47 commands of unmodified Debian trixie programs (ls, bash,
   python3, ...) with Liella swapped in as the system C library. Proposed: "Running unmodified glibc-linked binaries
   that need compatibility-only symbol versions has not been demonstrated; unmodified Debian trixie programs run with
   Liella as the system C library (docs/RELEASE-NOTES.md)."
   README.md:20 "Liella as the system C library is experimental" and "not for production" stay accurate.

## INFORMATIONAL

* Stale code comments (out of a docs story): xtask/src/rust_dylib.rs:31 "needs Liella's dynamic loader (phase 14)";
  xtask/src/native_examples.rs:14 and :46 ("arrive with phase 14", "`ld.so` has no library search yet").
* records/compat/COMPAT-22.toml body still names `records/pending/DL-16-PLAN-4.toml` (now D-27.33).

## Keep it current (proposal, not built)

`cargo xtask doc-links` in `cargo xtask static`, ~150 lines of xtask, under a second: over the hand-written docs
(the generated views skipped by their `@generated` line) check (1) every relative Markdown link resolves to a tracked
file; (2) every `cargo xtask <word>` names an arm of xtask/src/main.rs's dispatch; (3) every backticked path that
starts with a top-level directory (`crates/`, `csys/`, `docs/`, `tools/`, `validation/`, `records/`, ...) exists;
(4) no `records/pending/<x>.toml` reference whose file is gone (catches every id assignment). The scratch prototype
found 0, 0, 3 and 9 items on this head: all real, all fixed here. Prose staleness ("not yet", "phase 14") needs a
person; a release-notes step "grep the docs for each newly landed feature's name next to 'not'" is the cheap habit.

## Next step

Verify and fix the NOT VERIFIED rows (spec-symbol-gaps fpu_control skip, libresolv.so.2, tunables) and audit the
prose of benchmarks.md and the rest of validation/README.md; apply ruling 1.
