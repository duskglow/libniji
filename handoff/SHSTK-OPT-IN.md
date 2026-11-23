# SHSTK-OPT-IN handoff (part 1: marking + shadow-stack-correct asm)

**Scope (coordinator, accepted challenge #7946).** This story lands part 1 only: every shipped object marked SHSTK, the
build check, setjmp/longjmp + context functions + vfork shadow-stack-correct, in-process tests on the N100 class that
skip elsewhere. No behaviour change while nothing enables the shadow stack. Tunables, loader decision, dlopen policy and
the N100 scenario costs (SHSTK-SCOPE items B, C, K; plus F, clone3 thread stack size) are the follow-up **SHSTK-ENABLE**.

**State.** Branch `task/shstk-opt-in`, role mei:~/w/s2. Decisions: D-21.5 (asm + makecontext), D-X.164 (marking + check),
both reserved and written as records.

**What was done.**
* `csys/src/machine/setjmp.S`: SSP at jmp_buf byte 80; longjmp pops with incssp (255/step).
* `csys/src/machine/context.S`: `__ssp[0]` (936) written by get/swapcontext; resume = token at T-8 ? (incssp 1;
  rstorssp; saveprevssp) : pop to T; always `jmp *%r11`. `niji_machine_start_context` now `call *%r12` (the function),
  `call` setcontext / exit. New `niji_machine_ssp`.
* `crates/boundary/src/context.rs`: RIP = start_context, r12 = func, RSP = sp+8; `shadow_stack()` (cold) maps
  `map_shadow_stack(ss_size rounded, SET_TOKEN)` only when `niji_machine_ssp() != 0`.
* `csys/src/machine/vfork.S`: with SSP != 0 both returns `incssp 1` + jump.
* `xtask/src/cetnote.rs`: note parse/append, `mark_archive` (called from `so::publish` for *.a), `check_release`
  (end of `cargo xtask so`). `-Wl,-z,shstk` (`cetnote::LINK_FLAG`) on libc.so.6, ld.so, libm/other .so (so.rs 3995),
  static-PIE tools, dynamic utilities, ldd, libmvec, placeholders. `csys/start/Mcrt1.S` got NIJI_NOTE_PROPERTY.
* `conformance/c/s21_shadow_stack.c` (clause s21_1_g_0 in crates/core/tests/s21_contexts_clauses.rs).

**Jobs.** `mei:shstk-opt-in-so1` (first build: check found only niji-Mcrt1.o unmarked, fixed). `mei:shstk-opt-in-so2`
(xtask cetnote tests, so, conformance s21_*, s21 clause test, clippy).

**Exact next steps.**
1. Done: `mei:shstk-opt-in-so2` = cetnote unit tests 2 passed, `cargo xtask so` green ("44 shipped ELF files and
   archives carry the SHSTK property"), conformance s21_shadow_stack/s21_context/s21_setjmp/s22_more 4 passed (the
   shadow-stack program SKIPS on the Zen 1 host), s21 clause test 11 passed; clippy failed only on an unused `mut` in
   cetnote.rs tests (fixed in the next commit). Next: re-sync, foreground `cargo xtask clippy` + `cargo xtask static`.
2. Product test (catchable on hosts without CET): objdump libc.a; the bodies of niji_machine_longjmp_nomask,
   niji_machine_sigsetjmp, niji_machine_setcontext/swapcontext/getcontext and vfork contain rdssp, and incssp /
   rstorssp+saveprevssp where D-21.5 says. Plants (docs/plants.md lines, `cargo xtask plants`): remove vfork's
   `incsspq %rcx` (perl anchored on `vfork:`), and drop `.arg(crate::cetnote::LINK_FLAG)` from the libm link
   (cmd `cargo xtask so`, msg `lack the SHSTK property`).
3. N100 (nozomi, through `~/bin/timing <label> ...`, no compiler there): stage the static s21_shadow_stack binary built
   on mei, plus two negative controls linked with origin/next's setjmp.S / vfork.S objects ahead of libc.a (get the old
   files by reverse-applying `git diff origin/next...HEAD -- <file>` in scratch). Expect pass / SIGSEGV / SIGSEGV.
4. Gate (direct tier), `cargo xtask test`, `NIJI_FULL_PROFILE=1` hot_text_products (products change), precheck,
   report json, handback-check.

**Open for SHSTK-ENABLE (learned here).**
* setcontext on a kernel signal ucontext (no `__ssp` there) reads garbage at 936; with the shadow stack on this pops
  wrongly. Needs a rule (e.g. uc_flags bit, or bound the same-stack pop).
* makecontext shadow stacks are never unmapped (no API hook); SHSTK-SCOPE item E options.
* Threads still created with `clone` (RLIMIT_STACK-sized shadow stacks): item F (clone3 stack_size).
* Static libniji programs are now marked end to end (crt1 IBT|SHSTK AND libc.a SHSTK = SHSTK).

**Do not redo.** SHSTK-SCOPE's probes and asm survey.
