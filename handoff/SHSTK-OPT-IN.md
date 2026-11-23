# SHSTK-OPT-IN handoff (part 1: marking + shadow-stack-correct asm)

**Scope (coordinator, accepted challenge #7946).** This story lands part 1 only: every shipped object marked SHSTK, the
build check, setjmp/longjmp + context functions + vfork shadow-stack-correct, in-process tests on the N100 class that
skip elsewhere. No behaviour change while nothing enables the shadow stack. Tunables, loader decision, dlopen policy and
the N100 scenario costs (SHSTK-SCOPE items B, C, K; plus F, clone3 thread stack size) are the follow-up **SHSTK-ENABLE**.

**State.** Branch `task/shstk-opt-in`, code head `c57e08264`, role mei:~/w/s2. Decisions: D-21.5 (asm + makecontext),
D-X.164 (marking + check), both written as records. No differences row (nothing differs from the reference while off).

## What was done

* `csys/src/machine/setjmp.S`: SSP at jmp_buf byte 80; longjmp pops with incssp (255/step).
* `csys/src/machine/context.S`: `__ssp[0]` (936) written by get/swapcontext; resume = token at T-8 ? (incssp 1;
  rstorssp; saveprevssp) : pop to T; always `jmp *%r11`. `niji_machine_start_context` `call`s the function (r12).
  New `niji_machine_ssp` (prototype in niji_csys.h).
* `crates/boundary/src/context.rs`: makecontext maps `map_shadow_stack(SET_TOKEN)` only when the thread has one (cold).
* `csys/src/machine/vfork.S`: with SSP != 0 both returns `incssp 1` + jump.
* `xtask/src/cetnote.rs`: note parse/append, `mark_archive` (from `so::publish` for *.a), `check_release` (end of
  `cargo xtask so`: "44 shipped ELF files and archives carry the SHSTK property"). `-z shstk` on every linked product.
  `import` installs earlier products verbatim (`so::publish_verbatim`): they were checked against their manifest.
* Tests: `conformance/c/s21_shadow_stack.c` (enables the shadow stack in-process; SKIPS without CPU/kernel support);
  `s21_1_g_0` also disassembles the libc.a members holding sigsetjmp, longjmp_nomask, get/set/swapcontext and vfork
  and requires their D-21.5 instructions (catches a lost incssp on any host). crt_products accepts Mcrt1.o's note.

## Results (mei = Skylake-SP role; nozomi = Intel N100, kernel 6.8, user_shstk)

* Plants (`cargo xtask plants handoff/plants/SHSTK-OPT-IN.plants`): plants2 (head 38b6aba58) 3/3 CAUGHT, 0 WARN;
  plants3 (c57e08264) 3/3 CAUGHT, 0 WARN. vfork.S has no `niji_machine_<fn>:` label (the tool's anchor rule), so the asm plants are
  sigsetjmp's rdssp and setcontext's saveprevssp; vfork's incssp is covered by the libc.a check and the N100 control.
* N100 functional (`nozomi:shstk-opt-in-n100`, static, built on mei): pos = exit 0 "shadow stack on: all steps
  passed"; neg_setjmp (pre-change setjmp.S object ahead of libc.a) = SIGSEGV; neg_vfork (pre-change vfork.S) = SIGSEGV.
  Build script needs `-DNIJI_EXPORT` for the old objects (without it they assemble empty).
* Per-call instructions, no shadow stack (`shstk-opt-in-percall2` on 46b80c66a, same as percall1 on 38b6aba58; base 0495c4684, glibc dynamic): thread_create_join
  36092.1 = 36092.1 (glibc 2266); setjmp_longjmp 103 -> 110 (glibc 135); getcontext 40 -> 45 (glibc 44);
  swapcontext 64 -> 72 per call (glibc 62). Cost of the rdssp test on the off path.
* `cargo xtask test` (test1, head 38b6aba58): 7178 passed, 11 failed. 4 were this branch's (so publish test, 2 import
  tests, crt_products Mcrt1 note): fixed in c57e08264, re-run in `shstk-opt-in-fix1`. 7 are not: locale_data x2,
  s7_2_f_0, locale_products, printf_corpus, catalogs fail the same way in 4 other stories' xtask-test logs on mei
  (host locale archive); hot_text init_data_clusters_hold_at_every_text_phase fails in two CI gate logs on other heads.
* FULL: the train's (csys header, setjmp.S exported C ABI, xtask). Full-profile hot_text_products (`shstk-opt-in-hottext1`, c57e08264): 33 passed, including
  init_data_clusters_hold_at_every_text_phase.

## Open for SHSTK-ENABLE (learned here)

* setcontext on a kernel signal ucontext (no `__ssp` there) reads garbage at 936; with the shadow stack on this pops
  wrongly. Needs a rule (e.g. uc_flags bit, or bound the same-stack pop).
* makecontext shadow stacks are never unmapped (no API hook); SHSTK-SCOPE item E options.
* Threads still created with `clone` (RLIMIT_STACK-sized shadow stacks): item F (clone3 stack_size).
* Off-path cost fix-forward: store the raw SSP in get/swapcontext and add 8 in setcontext's shadow-stack branch
  (-2 instructions per get/swapcontext, getcontext back under the reference's 44).
* Static libniji programs are marked end to end (crt1 IBT|SHSTK AND libc.a SHSTK = SHSTK).

**Do not redo.** SHSTK-SCOPE's probes and asm survey; the N100 run and its controls (scripts:
`~/work/shstk-opt-in/{build.sh,run.sh}` on mei).

**Next step.** Land part 1 (train FULL gate); then brief SHSTK-ENABLE (tunables, loader, dlopen, scenarios, costs).
Plan rows: D-21.5, D-X.164, D-X.144, owner ruling #7728.
