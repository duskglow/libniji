# SHSTK-HW-ENABLE: the shadow stack on Zen 3 hardware (#154)

**State.** Branch `task/shstk-hw-enable` from origin/next fb12f4d3e. Commits: ea9e43598 (enforced row in
`s21_shstk_optin`), 9ceee540b (clause-test comment), then this note. No library code changed.

**Cause: no defect; the probe used a tunable that does not ask.** Yohane's run.sh sets only
`GLIBC_TUNABLES=glibc.cpu.x86_shstk=on|permissive|off`. Under D-21.6 (probe-settled against the reference on the
N100 class) only `glibc.cpu.hwcaps=SHSTK` asks for the shadow stack. `x86_shstk` picks the mode (`off` vetoes, `on`
keeps it on through an unmarked dlopen, `permissive` turns it off there). The host's glibc 2.39 on natsumi does
the same: off with `x86_shstk=*` alone, on with `hwcaps=SHSTK`. Coordinator agreed (#8388): no code change, land the
row and the plant.

**Evidence (natsumi, AMD Ryzen 5 5500 (Zen 3), kernel 6.8.0-147, `user_shstk`; kits built on keke n1).**

| kit | s21_shstk_optin (libniji dynamic) | s21_shstk_optin_static | s21_shstk_optin (host glibc 2.39) |
|---|---|---|---|
| kit1 = origin/next fb12f4d3e (before, 18/14/18 rows) | 0 failures | 0 failures | 0 failures |
| kit2 = this branch (19/15/19 rows, new `x` row) | 0 failures | 0 failures | 0 failures |
| kit-ctl = kit2 with both enable calls skipped | **11 failures** (every on-row; `x`: 66, want 64) | **8 failures** | 0 failures |

kit1 and kit2 ship byte-identical `ld.so` and `libc.so.6` (sha256 fe7adcf2..., b87b01b4...).

Yohane's `shst_m` / `shst_u` (host-built, unchanged; PT_INTERP pointed at kit2's `ld.so` through a short link) and
`shst.c` linked statically against kit2's `libc.a`: 102 runs. The host and libniji rows are identical once the
paths are taken out (0 differing lines):

| GLIBC_TUNABLES | x86_Thread_features / locked | return-address overwrite |
|---|---|---|
| (none), `glibc.cpu.x86_shstk=on`, `=permissive`, `=off` | (empty) / (empty) | goes through |
| `glibc.cpu.hwcaps=SHSTK` | `shstk` / `shstk wrss` | SIGSEGV si_code 10 (SEGV_CPERR) |
| `glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on` | `shstk` / `shstk wrss` | SEGV_CPERR |
| `glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=permissive` | `shstk` / (empty) | SEGV_CPERR |
| `glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=off` | (empty) / (empty) | goes through |

With `hwcaps=SHSTK` (each of the three asking rows), dynamic and static: longjmp (1000 x 50 frames), 4 threads (each
thread `shstk`), a signal handler and makecontext/swapcontext all pass. dlopen of the marked library loads with
the shadow stack still on. The unmarked one is refused under plain `hwcaps=SHSTK` ("rebuild shared object with
SHSTK support enabled", exit 2), loads with it kept under `:x86_shstk=on`, and loads after it is turned off under
`:x86_shstk=permissive`. The unmarked program `shst_u` stays off in every row. This matches D-21.6/D-21.7.

**Rows for Yohane (#154):** to turn it on, use `GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK`, optionally with
`:glibc.cpu.x86_shstk=on|permissive` appended for the dlopen policy. `x86_shstk` alone never asks, in libniji or in
stock 2.39. Expected results are the table above. C++ programs stay off while libstdc++/libgcc_s are unmarked.

**Test.** `s21_shstk_optin` (and its static copy) gains mode `x`: the child replaces its own return address and
returns. The row `{hwcaps=SHSTK, 'x', 64}` wants SIGSEGV with SEGV_CPERR (64), not the hijack target (66). It
SKIPs like the other rows without `user_shstk`. **Our CI hosts (Zen 1, Skylake-SP) cannot run any of these rows.**
Only natsumi (Zen 3) and the N100 class can.

**Plant.** The enable call skipped in both start paths (`crates/boundary/src/ld/link.rs` `niji_machine_ld_shstk_enter`,
`crates/boundary/src/runtime.rs` `niji_machine_shstk_enter`, each replaced by `let _ = lock;`). It was built on keke
(`shstk-hw-enable-kit2`, in place, restored with cat and rebuilt) and run on natsumi: CAUGHT (11 + 8 failures, above).
`cargo xtask plants` was not used for it. The tool runs the plant's command on the build host, where the row SKIPs,
so it would report SURVIVED. See the hand-back INFORMATIONAL.

**Jobs (keke):** final head: `shstk-hw-enable-gate1` (direct tier: 22 passed, conformance 1 passed, peak 1.5G),
`shstk-hw-enable-lint2` (clippy -D warnings + static). Evidence: `shstk-hw-enable-kit1` (before kit),
`shstk-hw-enable-kit2` (control + after kit; test source identical to the final head's). Superseded:
`shstk-hw-enable-lint` (ran before the clause-test comment). natsumi runs were
by hand (`nrun.sh`), under ~/work/SHSTK-HW-ENABLE there. Nothing is left running, and the /tmp links are removed.

**Do not redo.** The Zen 3 matrix above. **Next step:** land (test-only change). Reply to Yohane on #154 with the
rows above. Optional: a run-host mode for `cargo xtask plants` (build here, run the command on a host with the
capability).
