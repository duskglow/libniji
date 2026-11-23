# SHSTK-ENABLE handoff (leg 1 -> leg 2)

**State.** Branch `task/shstk-enable` (stacked on SHSTK-OPT-IN, merge base with next 0495c4684), WIP head
`f080f4c26`. Role mei:~/w/s2 (synced at f080f4c26; clippy --all-features and `cargo xtask static` clean there). Reserved ids: **D-21.6** (tunables + start decision + dlopen
policy), **D-21.7** (context rules: signal ucontext, makecontext shadow stacks, raw SSP), **D-22.29** (clone3
stack_size). No records written yet (`cargo xtask records new decision D-21.6`, then `records check`).

## Done in leg 1 (754e672f6; `cargo clippy --workspace --all-targets --all-features -D warnings` clean)

* K, tunables: `niji::config::{Tunables.shstk, Tunables.x86_shstk, ShstkMode, Tunables::shstk_wanted}`; unit test
  `config::tests::shstk_tunables`. Probe table (reference, N100 nozomi, SHSTK-SCOPE's `shb status`; log in this
  run's scratch `probe_tun.log`): `hwcaps=SHSTK` on+locked(shstk wrss); `shstk`, `SHSTK_Usable`, `-SHSTK`,
  `SHSTK,-SHSTK` off; `-SHSTK,SHSTK` and `IBT,SHSTK` on; `hwcaps=SHSTK:hwcaps=AVX2` off (later entry replaces);
  x86_shstk `OFF`/`On`/`bogus`/`PERMISSIVE` = default (on, locked); `=on` on+locked; `=permissive` on, NOT locked;
  `x86_shstk=off:hwcaps=SHSTK` off; `x86_shstk=off:x86_shstk=on` on+locked; static same; unmarked program
  (`shb_unm`) off even with `=on`. Not probed: `x86_shstk=off:x86_shstk=bogus` (libniji: bogus -> default).
* `niji_ld::shstk` (pure): `marked`, `start_allowed` (AND), `ShstkMode`, `on_dlopen` (Load/Refuse/Disable), `locks`;
  unit test `shstk::tests::dlopen_policy`. `Refusal::Shstk` = "rebuild shared object with SHSTK support enabled".
* B dynamic: `ld/entry.rs` keeps `SHSTK_WANTED` (code via `crate::shstk::code`) only when tunables ask;
  `ld/link.rs` `shstk_decide` (cold) before views unmap; enable via `crate::arch::niji_machine_ld_shstk_enter(lock)`
  (entry.S NIJI_LD; returns by pop+jmp) right before `run_init`; `link` is now `-> !` and ends in `arch::enter`.
* (f080f4c26) no `boundary/src/shstk.rs` (static refused its unsafe slices and feature cfgs): pure helpers
  `lock_mask`, `code`, `mode_of` are in `niji_ld::shstk`; the permissive broadcast is
  `runtime::shstk_off_everywhere`.
* B static: `runtime.rs` after `cpu::init_with`: `shstk_static_start` (cold; reads PT_GNU_PROPERTY via
  AT_PHDR) then `ffi::niji_machine_shstk_enter(lock)` (context.S copy) directly from `start` (never returns).
* C: `ld/dlopen.rs` after the dynamic check: `niji_machine_ssp() != 0 && !marked` -> `shstk_unmarked` (cold):
  refuse / load / call libc `__niji_shstk_off` (NIJI_PRIVATE, s01_startup.rs -> `shstk::off_everywhere`, setxid
  broadcast of ARCH_SHSTK_DISABLE) else `off_here`. Added to `LIBC_PRIVATE` (xtask/src/so.rs), `rt::start::private`.

## Next (leg 2), in order

1. `~/bin/jobwait shstk-enable-so2` (`cargo xtask so` on f080f4c26, mei; log ~/logs/shstk-enable-so2.log): fix what
   it names (asm assembles only here; a NIJI_PRIVATE list/ABI table may need `__niji_shstk_off`). (so1 was an
   earlier attempt: rc 1 from `cargo xtask static`, fixed in f080f4c26; SUPERSEDED.)
2. Test for B on any host: an ld test (pattern `crates/ld/tests/canonical_plt.rs` `cc` helper) building a marked
   (`-fcf-protection=full -Wl,-z,shstk`... any flags giving the note) and an unmarked (`-fcf-protection=none`) .so,
   asserting `niji_ld::shstk::marked` and `start_allowed` false with one unmarked in the list. Plants file
   `handoff/plants/SHSTK-ENABLE.plants`: B = `start_allowed` ignoring all but the first (`objs.all` -> `objs.take(1).all`),
   C = `on_dlopen` Default -> Load. Run `cargo xtask plants` (developer-flow step 6).
3. N100 functional (nozomi; build on mei, copy; scripts `~/work/shstk-opt-in/{build.sh,run.sh}` pattern): dynamic
   under libniji ld.so with each tunable row above, `/proc/self/status`, dlopen unmarked default/on/permissive,
   static. Turn it into a conformance test that skips without `user_shstk` (pattern
   `conformance/c/s21_shadow_stack.c`).
4. F: thread create through clone3 with `stack_size` only when `niji_machine_ssp() != 0` (thread.rs ~1215,
   `niji_machine_clone3_run` pattern; fall back to clone on ENOSYS). Count thread_create_join before/after (36,092).
5. D-21.7 context: signal ucontext rule (kernel sets uc_flags UC_SIGCONTEXT_SS=2; idea: getcontext writes uc_flags,
   setcontext with that bit pops to the newest sigframe token = word w at a with w == (a+8)|1<<63); raw SSP
   fix-forward; makecontext leak = RULING NEEDED (recommend reuse per `uc_stack.ss_sp`).
6. H posix_spawn test with the shadow stack on; 7. N100 cost table; records, docs/tunables.md row; start-up rows
   before/after (`cargo xtask test`, full-profile hot_text_products, layout-bins diff); gate; precheck; report.

Do not redo: SHSTK-SCOPE probes/asm survey, SHSTK-OPT-IN N100 controls, the tunable probe above.
