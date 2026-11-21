# COMPAT19-TUNABLE report

Branch `task/compat19-tunable`, cut from origin/next 50abc44ed. Brief: implement registered flag COMPAT-19 (accept
executable-stack objects, opt-in, ignored in a secure process), Yohane's request; register the allowlist amendment;
refusal messages name the object, the program or `dlopen` caller and the two ways in.

## Commits (all mine)
* 20717549d WIP: Accept executable-stack objects under niji.compat.execstack (COMPAT-19) (leg 1: code)
* 55a2f15d0 WIP: COMPAT19-TUNABLE handoff after leg 1
* 8e8c7dc8f Test COMPAT-19 and record it; name the program and caller in refusals
* 936821898 Write D-27.66's protection flags without table bars
* 25f87fc14 Mark the two secure-mode plants equivalent; update the handoff
* (final) report and json

## What it does
`GLIBC_TUNABLES=niji.compat.execstack=1` (D-27.66): the loader accepts a program, a start-up dependency or preload,
or a `dlopen`ed library whose `PT_GNU_STACK` has `PF_X`; the main stack and every library-made thread stack (before
and after) become executable, a caller-supplied stack does not (the reference's behaviour, black-box probe
`tools/probe/execstack`, Zen 1 host). Default unchanged: refused, now with messages:
* program: `ld.so: <program>: the program asks for an executable stack (PT_GNU_STACK with PF_X); refused; <ways>`
* dependency: `<program>: error while loading shared libraries: <path>: the object asks ...; refused; <ways>`
* dlopen: `<path>: the object asks ...; refused (dlopen called from <caller>); <ways>`
* `<ways>`: `to accept it, start the process with GLIBC_TUNABLES=niji.compat.execstack=1 (COMPAT-19; not honoured
  in a secure process); a system allowlist (/etc/niji/execstack.d) is planned`
Secure (`AT_SECURE`): refused with the tunable set (ld.so removes `GLIBC_TUNABLES` there; the parse's secure flag is
a second guard).

## Tests (mei, role s4)
* `crates/ld/tests/exec_stack.rs`, 5 tests, no SKIP: program (direct and via `ld.so PROGRAM`, default / `=0` /
  `=1`), dependency, dlopen after two threads exist (one on a caller-supplied stack), set-group-ID secure (program
  and dlopen), GCC nested function taken by address (main thread and a created thread).
* compat19-tunable-crates (FULL escalated, see below): `cargo xtask so`; `cargo test --no-fail-fast -p niji-ld
  -p niji-rt -p niji-boundary -p xtask`; `cargo test -p niji --lib`; `-p niji --test s01_startup_clauses`:
  136 suites ok, 4481 passed, 0 failed, 563 ignored.
* `cargo xtask clippy` and `cargo xtask static` clean (job compat19-tunable-t3).
* compat19-tunable-xtest (`cargo xtask test`, the start-up check): rc 0; nextest passes 6900/6900, 4834/4834,
  3235/3235, 1203/1203 passed, 0 failed; conformance 140 passed, 0 failed.
* compat19-tunable-hot (`NIJI_FULL_PROFILE=1 cargo xtask so` + `hot_text_products`): result in the hand-back.
* Leg-2 fixes the new tests forced (each now a plant): dependency acceptance had to move into the start search's
  candidate check; program name empty when the kernel started the program; `ld.so PROGRAM` used the generic map text;
  dlopen caller empty for the main program.

## Plants (`cargo xtask plants`, handoff/COMPAT19-TUNABLE-plants.txt, 14)
Run 1 (compat19-tunable-plants): 12 CAUGHT, 2 SURVIVED (ld-secure-honoured, dlopen-secure-honoured): both are a
second guard (secure mode clears `GLIBC_TUNABLES` from the start environment before the loader and dlopen read it),
marked `equivalent:` and run again (compat19-tunable-plants2, from ~/work/compat19-tunable/plants.txt, the same file).
Run 2 (compat19-tunable-plants2, rc 0): `14 plants: 12 CAUGHT, 0 SURVIVED, 2 EQUIVALENT, 0 WARN-RC-ONLY,
0 WARN-NO-MSG, 0 BUILD-TIMEOUT`.

## FULL
`cargo xtask affected --list --base origin/next`: `FULL: the train's (xtask/src/so.rs: xtask (its commands are part
of the gate))` (so.rs: LIBC_PRIVATE gains `__niji_stack_exec`). No gate run; the crates' tests above plus precheck.

## Records
COMPAT-19.toml (implemented; the owner's allowlist amendment of 2026-11-20 recorded, registered, not implemented;
blocks the versioned release), D-27.66.toml, docs/differences-from-glibc.md row, docs/tunables.md row,
docs/RELEASE-NOTES.md "Unreleased" section (there was no such section; the coordinator may fold it into the next
snapshot's notes). Generated views are train-owned (records-gen validated, not written in place).

## RULING NEEDED
* Start-up cost: brief point 3 asks that dynamic start-up counts not move. The accept flag costs one relaxed load and
  branch in `run_init` per dynamic start, and one relaxed load per `pthread_create` (under the list lock it already
  holds); not measured with wl_percall in this run (budget). Recommend accept (about 2 instructions per start); a
  zero-cost alternative would make the start's check part of the C library's early-init argument, a larger change.

## INFORMATIONAL
* records/compat/COMPAT-20.toml still says `liella.compat.no_classed_swap` (the view renames it); COMPAT-19 follows
  the same convention.
* If the rest of a dlopen set fails after an executable-stack object was accepted, the stacks stay executable (as
  the reference's).

## Refusals
None.
