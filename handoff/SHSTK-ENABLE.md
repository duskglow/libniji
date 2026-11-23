# SHSTK-ENABLE handoff (leg 2 -> leg 3)

**State.** Branch `task/shstk-enable` (stacked on SHSTK-OPT-IN, merge base with next 0495c4684), head `64e90cdf8`
(pushed). Role mei:~/w/s2 synced at 64e90cdf8 (clippy --all-features clean there at 3470398c2; `cargo xtask so` ok).
Reserved ids: **D-21.6** (tunables + start decision + dlopen policy), **D-21.7** (context rules), **D-22.29**
(clone3 stack_size). No records written yet (`cargo xtask records new decision D-21.6`, then `records check`; §10).

## Done (legs 1-2; do not redo)

* Leg 1: K tunables (`niji::config`), `niji_ld::shstk` (marked/start_allowed/on_dlopen/locks), B dynamic
  (ld link.rs `shstk_decide`) and static (runtime.rs), C dlopen refuse/load/permissive broadcast. Tunable probe
  table in scratch `probe_tun.log` (summary in the leg-1 notes, now in the commits).
* Leg 2: so2 green. `crates/ld/tests/shstk_marking.rs` (cc -nostdlib marked `-Wl,-z,shstk` / unmarked objects;
  unmarked in every position vetoes) + `handoff/plants/SHSTK-ENABLE.plants` (B start-first-only, C
  dlopen-default-loads); both tests pass on mei (job shstk-enable-t1). Plants NOT run yet.
* F (D-22.29): `thread.rs` `clone3_thread` (cold): clone3 with stack=lo, stack_size=sp-lo when
  `niji_machine_ssp() != 0`, ENOSYS -> clone; asm `niji_machine_clone3_thread` (thread_start.S, shares
  `.Lthread_child`), header + ffi decl. Thread-create count before/after NOT measured yet (36,092 base).
* Conformance `s21_shstk_optin.c` (dynamic: DLOPEN-HELPERS s21sk_unmarked, HELPER-FLAGS -fcf-protection=none;
  re-execs /proc/self/exe per row; exit bits 1 on, 2 locked, 4 loaded, 8 on after dlopen, 16 refusal names SHSTK,
  32 thread shadow stack < 1 MiB with RLIMIT_STACK 64 MiB) + `s21_shstk_optin_static.c` (includes it, start+thread
  rows), listed in `s21_1_g_0`. mei: SKIPPED (no user_shstk), conformance 3 passed.
* **N100 functional (nozomi, job nozomi:shstk-enable-n100d; kit scripts mei+nozomi ~/work/SHSTK-ENABLE/{build.sh,
  run.sh,nosys_clone3.c}, copies in scratch):** reference dynamic 0 failures, libniji dynamic 0 failures (all 18
  rows incl. dlopen refuse/on/permissive, thread 262144 B), libniji static 0 failures, s21_shadow_stack static
  passes. Negative control `nosys-clone3` (seccomp: clone3 -> ENOSYS): fallback creates threads, shadow stack
  67108864 B, 't' row FAILs as it must (static and dynamic). **Reference static differs:** it ignores
  `glibc.cpu.x86_shstk=off` in any order (`off:hwcaps=SHSTK`, `hwcaps=SHSTK:off` -> on+locked), honours
  `permissive`; its dynamic build vetoes. libniji vetoes in both (manual's meaning) -> needs a
  docs/differences-from-glibc.md row (no compat option: reason = a veto that is ignored only weakens the opt-out;
  drop `x86_shstk=off` for the reference's static result) + RULING NEEDED (recommend keep the veto).
* Item H: posix_spawn with the shadow stack on is already a step of `s21_shadow_stack.c` (passes on N100).
* libniji pins libc.so.6 to its install dir (D-27.16): N100 runs need LD_LIBRARY_PATH (RUNPATH ignored for it).

## Running

* **nozomi:shstk-enable-cost1** (`~/bin/timing`, started 14:36Z): `~/work/SHSTK-ENABLE/cost/cost.sh` (scratch
  `cost.sh`, kit from `cbuild.sh`): 3 passes, n and g, off/on (GLIBC_TUNABLES hwcaps=SHSTK / -SHSTK): envbase,
  empty_static, empty, main3 (50 reps), thread 0/2000 dyn+static (5 reps); real programs via libniji ld.so program
  mode (sort, python, perl, find) with a CHECK line per arm; SYSCALLS + SHSTK-VMA lines; ends with DONE. Read with
  `grep -E "^(ROW|CHECK|SYSCALLS|RC|DONE)" ~/logs/shstk-enable-cost1.log` and build the section-1 table
  (per op = (n=2000 - n=0)/2000; dynamic rows minus envbase).

## Next (leg 3), in order

1. D-21.7 in `csys/src/machine/context.S` (designed, not edited): getcontext and swapcontext write
   `movq $0, (%rdi)` (uc_flags: a user context) and store the RAW SSP (`xorl %eax,%eax; rdsspq %rax; movq
   %rax, 936(%rdi)`, replacing NIJI_SSP_AFTER_RETURN: net -2 instr each); setcontext label 3: `testb $2, (%rdx)`
   (UC_SIGCONTEXT_SS: kernel signal frame) `jnz 7f`; else load 936, `jz 2b`, then `addq $8, %rcx` and continue
   as now. 7: rcx = SSP (%rax); loop: rsi = (rcx); r8 = rcx+8 with bit 63 set (`btsq $63`); rcx += 8; until
   rsi == r8; then `jmp 4b` (pops to the interrupted SSP). makecontext (`context.rs` `shadow_stack`): store
   `b + len - 8` in W_SSP (raw convention). Update the header comment (lines 10-18). Test: add a step to
   `conformance/c/s21_shadow_stack.c`: SIGUSR1 handler calling `setcontext(ucp)` (its 3rd arg) from nested
   frames, then main continues and returns through frames. Run on N100 (kit build.sh + run.sh); negative control
   = current context.S with the 7: branch removed, assembled with `-DNIJI_EXPORT`, symbols renamed
   `__niji_libc_<sym>` (objcopy --redefine-sym; libc.a's internal names carry that prefix) and linked ahead of
   libc.a with `-Wl,--allow-multiple-definition -Wl,-z,shstk`: expect SIGSEGV.
2. Counts on mei: thread_create_join before/after (`cargo xtask bench --compare --counters` row; base 36,092),
   getcontext/swapcontext per call; start-up rows before/after (`cargo xtask test`,
   `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`,
   layout-bins.tsv diff vs merge base; §9). Fix the `niji_machine_shstk_enter is never used` warning in
   feature builds of boundary (cfg its ffi decl like its caller).
3. Records: D-21.6, D-21.7, D-22.29 (`cargo xtask records new decision ...`, §10); differences row (above);
   docs/tunables.md rows for glibc.cpu.x86_shstk / hwcaps=SHSTK; cost table into the report.
4. Gate (`cargo xtask affected --run`, jobrun gate), plants (`cargo xtask plants handoff/plants/SHSTK-ENABLE.plants
   --dir /mnt/zram/claude/shstk-enable/plants-1 --timeout 600`), clippy + static, precheck, report json,
   handback-check.

RULING NEEDED (for the report): makecontext shadow stacks are never unmapped (recommend: key them to
`uc_stack.ss_sp`, unmap the earlier one when makecontext reuses the same stack memory, since that context is dead;
not implemented); reference-static x86_shstk=off difference (recommend keep the veto).
Note for the report: one Bash call in leg 2 used a here-string (`git commit -F - <<< ""`); it ran (empty message,
commit refused) before I noticed. Not repeated.
