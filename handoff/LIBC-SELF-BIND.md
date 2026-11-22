# LIBC-SELF-BIND: handoff (end of leg 2)

State: branch task/libc-self-bind, code head bb67c81b1 (the handoff/report commit follows). Role keke:~/w/n1 synced
at bb67c81b1. Decision D-27.76 (records/decisions/D-27/D-27.76.toml). Ruling #7401 (the "keep" variant) in force.

## Done (both legs)
* xtask/src/so.rs: libc.so.6 (and its libc-owner.so link stub) links with `--dynamic-list` (the malloc family) and
  `--dynamic-list-data`. `LIBC_PREEMPTIBLE` is now the static archive's `INTERPOSABLE` list (D-X.14; adds only the
  compat cfree, which no relocation names), so the two links cannot drift. Build check `check_self_binding` + unit test.
* conformance/c/x_self_bind.c (+ helpers/sbind_lib.c, now `_POSIX_C_SOURCE`: the harness builds helpers as
  `-std=c17`, so `optind` was undeclared; leg 1's reference run had used plain gcc outside the harness).
* conformance/c/x_self_bind_stdio.c (new, PENDING-NIJI): fopen/fclose through the program's malloc family.
* crates/core/tests/hot_text_products.rs `DYN_START_INSTR` tightened to [373_000, 346_300] (both CPU rows).

## Measured (Zen 1 host, user instructions, median of 101, ASLR off, empty environment)
| program (kit form, LD_LIBRARY_PATH) | before | after | glibc 2.39 |
|---|---|---|---|
| empty main, fast (before = next 4f32aa946) | 754,018 | 356,120 | 98,874 |
| 3 libraries, fast | 1,007,472 | 533,674 | 134,110 |
| empty main, release (before = snapshot 1121E products) | 742,167 | 330,073 | |
| 3 libraries, release | 984,891 | 493,234 | |
hot_text_products empty main: fast 365,683, release 339,504. Symbolic relocations 288 -> 58 (fast: 55 GLOB_DAT, 3
R_X86_64_64, no JUMP_SLOT), 55 (release, not broken down); the malloc family left only as GLOB_DAT (the heap's entry-point table). Faults unchanged (65 fast,
59 release), kernel instructions -0.2%. Logs: keke ~/logs/libc-self-bind-measure.log (job superseded only because
the head moved after it; the changes after it touch no code that builds into libc.so.6).
Skylake-SP class: NOT MEASURED; it takes the Zen 1 bounds by the within-0.03% precedent in the bound's doc.

## Probe on libniji (first run, plants-2 baseline log)
Every family reads 0 reaches, as on the reference; COPY checks (stdout, optind, sbind_lib) pass. Only fopen/fclose
failed: libniji's stream objects do not come from the program's malloc family. Black-box check (keke
~/work/libc-self-bind/diag/fopen_alloc.c, own bump allocator): origin/next's products and this branch both report
`strdup mallocs 1; fopen mallocs 0; fclose frees 0`; the reference `1; 1; 1`. Not caused by this change: split into
x_self_bind_stdio (PENDING-NIJI until a ruling) and D-27.76 corrected.

## Jobs (keke), all on bb67c81b1
* libc-self-bind-gate-2 (direct tier; prints `FULL: the train's (xtask/src/so.rs ...)`: bounded set, not FULL),
  then libc-self-bind-xtest (`cargo xtask test`), then libc-self-bind-fullprof (NIJI_FULL_PROFILE pair); one
  after another by ~/work/libc-self-bind/final-chain.sh.
* libc-self-bind-plants-3 (5 plants, /dev/shm/claude/libc-self-bind/plants-3).
* SUPERSEDED: libc-self-bind-plants (refused: malloc-bound matched 2 sites), -plants-2 (baseline red: the fopen
  checks), -gate (killed: head moved), -list (dropped: the gate prints the plan), -measure (numbers above).

## RULING NEEDED
* Stream allocation: a program replacing malloc sees the reference's fopen/fclose allocate and free through it;
  libniji's do not (pre-existing). Recommend a follow-up story: allocate FILE objects through the caller
  allocator (`boundary::caller_alloc`), then drop x_self_bind_stdio's PENDING-NIJI.

## Next step (exact)
1. `~/bin/jobwait libc-self-bind-gate-2 libc-self-bind-plants-3 libc-self-bind-xtest libc-self-bind-fullprof`.
2. Red gate: fix, rerun the failing suite only. Plants: all CAUGHT expected (msg regexes in handoff/LIBC-SELF-BIND-plants.txt).
3. `cargo xtask precheck --jobs <those labels> --head <sha>` on the role; handback-check; hand back.
