# SHSTK-OPT-IN handoff (part 1: marking + shadow-stack-correct asm)

## Run 3 (SHSTK-OPT-IN-3, branch `task/shstk-opt-in-3`, role keke:~/w/n3, Zen 1 host)

**CI red (train on 90fdd8142): hot_text_products dynamic empty main 173794 > 173600 (Skylake-SP).** This branch
is a measured cause, not only one of the train's "several": on the Zen 1 host (fast profile, the test's own count,
3 runs each, identical) the marked head counts 170825, the same tree with `-z shstk` and the check turned off
170464: **+361**. The marking adds a second `PT_NOTE` (8-aligned, beside the 4-aligned build-id note) and
`PT_GNU_PROPERTY` to ld.so and libc.so.6 (9 -> 11 headers each). A gdb single-step split by function (scratch probe,
libniji only) put +123 in `Elf::check_table` and the rest in eight header walks (relocate_self +48, open_candidate
+36, publish_dynamic +36, find +28, protect_relro +26, map_planned +24, boot::tables +16, Loads::of +12). The
property readers in reloc.rs are cold (not reached).

**Fix (D-27.97, `25297497e`).** `image_view` checked its private copy of the table as a probe; each caller then
checked the same bytes again with `from_image`. It now returns the probe with the spans attached
(`Elf::with_spans`, `None` for a file reader). Count: **170580** (-245 against the marked head, +116 over
unmarked). Test: `reloc_image_reader_slices_only_unchanging_segments` also builds the reader that way and refuses a
file reader. Plants `handoff/plants/SHSTK-OPT-IN-3.plants` (2).

**Remaining cost (accepted, fix-forward):** +116 instructions on a dynamic start against an unmarked build. The
next saving: `find` and the `PT_LOAD` filters could use what `check_table` sees in its one pass (first index per
type, last `PT_LOAD`). Not done here (budget).

**Results (keke, head 25297497e):** clippy + static green on the inherited head (`shstk-opt-in-3-lint0`) and on
the fix (`shstk-opt-in-3-lc1`, with the count above). Plants `shstk-opt-in-3-plants1`: 2/2 CAUGHT, 0 WARN
(`handoff/SHSTK-OPT-IN-3-plants.txt`). `cargo xtask test` (`shstk-opt-in-3-test1`): rc 0, 7190 + 4878 + 129 + 46
passed, "43 shipped ELF files and archives carry the SHSTK property". Full profile (`shstk-opt-in-3-hottext1`):
33 passed, release dynamic empty main 159407 (bound 162300). FULL: the train's (xtask, csys header). Diagnostics
(not proofs): dyn-a / dyn-b (marked / unmarked counts), so-c, trace1 (gdb picked up libniji through
LD_LIBRARY_PATH: probe fixed), trace2 (the per-function split). Scratch scripts: `~/work/shstk-opt-in-3/` on keke.

**Next step.** Land with the train (the Skylake-SP count should drop by about 245 from 173794). Then the
fix-forward above, and SHSTK-ENABLE as in run 1's notes below.

## Run 2 (SHSTK-OPT-IN-2, branch `task/shstk-opt-in-2`, role shiki:~/w/sh1)

**Cause of train 950's red.** `cetnote::check_release` walked all of `target/niji`. `cargo xtask test` puts
unshipped products there (`cargo xtask rust-dylib`'s `rust/lr1/` and `rust-test/` rlibs, dylibs and programs;
`native/libc-by-path.so`), so the train's `so` after those steps found 17 unmarked objects. None of them is in a
package: the shipped set is the `file` rows of `tools/deb/manifest.tsv` (PKG-MANIFEST; build.sh/check.sh install and
verify exactly those). **Fix:** the check reads that manifest (`cetnote::shipped_products`), checks each listed
product that is ELF or an archive, and fails on a listed product the build did not make. D-X.164 point (4) updated.
The shipped set did not change, so the N100 run was not repeated (coordinator's note). Plants:
`handoff/plants/SHSTK-OPT-IN-2.plants` (3, on the new code); part 1's libm-unmarked plant still covers the
`so` integration (libm.so.6 is a manifest row).

**Results (shiki role, Haswell host).** clippy + static green on the inherited head (`shstk-opt-in-2-lint0`) and on
the fix (`shstk-opt-in-2-lint1`, with the cetnote unit tests: 3 passed). Plants `shstk-opt-in-2-plants1`: 3/3 CAUGHT,
0 WARN (`handoff/SHSTK-OPT-IN-2-plants.txt`). `cargo xtask test` (`shstk-opt-in-2-test1`): rc 0, 7190 tests passed,
native examples and rust-dylib green; "so: 43 shipped ELF files and archives carry the SHSTK property". Then
`cargo xtask so` with `rust/` and `rust-test/` present in `target/niji` (the train's failing situation):
`shstk-opt-in-2-so2`. 43, not part 1's 44: the one ELF file under `target/niji` outside the manifest (the
unshipped Rust dirs aside) is the link stub `niji-owner/libc-owner.so` (docs/install-packages.md, opt-in, linked from
the build tree, in no package); it is still linked with `-z shstk`, only no longer checked.

**Not covered by D-X.164 (INFORMATIONAL):** `libniji_rust.so.lr1` and its rlibs (docs/toolchain.md tells third
parties how to build them) carry no SHSTK; they ship in no package today. If they ever do, they need a manifest
row and marking (rustc emits no note: the rlib members would need `mark_archive`, the dylib `-z shstk`).

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

**Next step.** Land part 1 + run 2's check fix (train FULL gate); then brief SHSTK-ENABLE (tunables, loader, dlopen, scenarios, costs).
Plan rows: D-21.5, D-X.164, D-X.144, owner ruling #7728.
