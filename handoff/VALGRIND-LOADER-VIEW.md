# VALGRIND-LOADER-VIEW: full leak stacks through libc under valgrind, and helgrind (GitHub #13, the last of it)

Branch `task/valgrind-loader-view` (from origin/next d5764940d). Brief `~/state/briefs/VALGRIND-LOADER-VIEW.md`.
Role keke `~/w/n3` (Zen 1 host). Job scripts on keke `~/work/VALGRIND-LOADER-VIEW/` (local copies in
`~/w/.scratch/VALGRIND-LOADER-VIEW/job/`): `leg2.sh`, `vgacc.sh DEBDIR LOGDIR [post.sh|diag.sh]`, `meas/measure_a.sh`,
`vg/` (`pre.sh post.sh hg.sh vg.sh diag.sh`, probes `bad.c cxx.cc leak.c thr.c thr_old.c race.c`). Valgrind 3.24.0
source files (allowed by the brief) in `~/w/.scratch/VALGRIND-LOADER-VIEW/vgsrc/` and `vgsrc2/`.
Reserved ids: **D-X.152** (item 1), **D-X.153** (item 2).

## State (leg 2 ended by REFRESH at ~155k context, 18:35Z)

Commits: 3f1ac8b16 (item 1: the valgrind-only hidden view, option (b)), 354d33f30 (item 2: `.symtab` names
`name@@VERSION`), then the D-X.153 record + this note. Records: D-X.153 written; **D-X.152 NOT yet written** (needs
the (a) table below).

### Evidence so far (diag0, origin/next debs, swapped trixie root, `~/logs/valgrind-loader-view-diag0.log`)
* Stock leak record: malloc, `__vasprintf_internal`, asprintf, main (leak.c:9). libniji (base): malloc, malloc
  (caller_alloc.rs:58), vasprintf (printf.rs:957), then `0x7: ???` frames: item 1's symptom.
* helgrind stock: thr/thr_old 0 errors, race 2 races at race.c; stock redirects `libc.so.6:pthread_create@@GLIBC_2.34`
  (so stock's `.symtab` has the versioned default: our D-X.153 naming matches). libniji base: thr and race assert
  (hg_main.c:5421); thr_old (binds pthread_create@GLIBC_2.2.5, the stub) passes on the base.
* RISK to check in acc1: after D-X.153, thr_old's call is wrapped twice (the stub `pthread_create@GLIBC_2.2.5` is
  wrapped and it `jmp`s to the default, now also wrapped). If thr_old fails in acc1 (assert / wrong counts), fix:
  either make the stub's `.symtab` name not match (not good), or better: note it as INFORMATIONAL with evidence and
  decide; post.sh judges thr_old against stock.

### Item 1 design (unchanged from leg 1)
`csys/src/machine/tool.S` `niji_machine_running_on_valgrind` (client request 0x1001); `image.rs` `map_view` asks
`views_hidden()` once (cached AtomicU8), hidden path `#[cold] map_view_hidden` maps PROT_NONE then mprotect. Option
(a) (unconditional) = +1 mprotect per view (empty +1, three +4, dlopen +1 per object), so it costs measurably by
construction; the brief still wants its start-up table: job `valgrind-loader-view-measure-a` builds (a) in
`/dev/shm/claude/valgrind-loader-view/measure-a` (image_a.rs: views_hidden() always true, +2 instructions per view vs
a pure (a)) and prints `ROW <empty|three|dlopen> <next|head|a> user= kernel= faults= syscalls= mprotect=`
(perf stat -r 50, strace -f).

## Running jobs (keke), started 18:33Z
* `valgrind-loader-view-leg2` (log `~/logs/valgrind-loader-view-leg2.log`): `cargo xtask so` (log
  `~/work/VALGRIND-LOADER-VIEW/so1.log`, must print `so: N defaults with a compat ...`), the xtask so::tests, the
  boundary `hidden_view` test, `tool_symtab_products`, clippy, static (lines `LEG2: ...`), then head debs
  (`debs1.log`) and the acceptance `vgacc.sh debs-head acc1 post.sh` (`=== VGACC pass= fail=`, PASS/FAIL lines incl.
  helgrind thr/thr_old/race and memcheck leak `leak_at_main=1 unnamed_frames=0`).
* `valgrind-loader-view-measure-a` (log `~/logs/valgrind-loader-view-measure-a.log`): the (a) table.

## Exact next steps
1. `~/bin/jobwait valgrind-loader-view-leg2 valgrind-loader-view-measure-a` (keke); read the logs with grep
   (`LEG2:|test result|VGACC|vgacc:|ROW`). Fix anything red (so/clippy/static/tests), re-sync, rerun only that.
2. Write `records/decisions/D-X/D-X.152.toml` (template: D-X.150/D-X.153): the cause (ruling #7301, valgrind
   `di_notify_mmap` records readable mappings before the segments, ignores PROT_NONE, no mprotect hook), the (a) table
   (next/head/a rows), why (b), the bare-machine cost (one client request per process, no syscall), tests
   `crates/boundary/src/ld/image.rs::hidden_view_is_inaccessible_until_revealed_and_the_bare_machine_does_not_hide`.
   Check the D-X.153 test references format with `cargo xtask records check` on the role.
3. Gate: `cargo xtask affected --run` (jobrun gate, `~/bin/gate 16G`), plants:
   `cargo xtask plants handoff/VALGRIND-LOADER-VIEW-plants.txt --dir /dev/shm/claude/valgrind-loader-view/plants-1 --timeout 600`
   (7 plants), start-up checks: `cargo xtask test` and `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo
   test -p niji --test hot_text_products` (one product build per role at a time: run them in sequence in one job,
   plants in RAM beside). Layout-bins diff vs base (§9): image.rs hot path should be unchanged except the cached test.
4. Report: handoff/VALGRIND-LOADER-VIEW-report.md, handoff/reports/VALGRIND-LOADER-VIEW.json (with "jobs"), precheck
   `cargo xtask precheck --jobs <labels> --head <sha>`, `ops handback-check`, hand back.
