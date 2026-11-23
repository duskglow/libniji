# SHSTK-ENABLE report

Opt-in user shadow stack as the reference (owner ruling #7728, PROVISIONAL): tunables (K), start-up
decision (B), dlopen policy (C), clone3 thread shadow-stack size (F), context rules (SHSTK-OPT-IN's
open items), posix_spawn (H), N100 cost table. Records: D-21.6, D-21.7, D-22.29. Stacked on
SHSTK-OPT-IN (a2cee937a; merge base with next 0495c4684).

## What changed

* **K** `niji::config`: `glibc.cpu.hwcaps=SHSTK` asks; `glibc.cpu.x86_shstk` = `on` / `off` / `permissive`
  (exact spellings, last entry counts; probe table below). Default off.
* **B** dynamic: `ld.so` link step ANDs the marking of program, DT_NEEDED, preloads, ld.so, libc
  (`niji_ld::shstk::start_allowed`); static: the Rust start reads its own `PT_GNU_PROPERTY`. Enabled
  from a never-returning frame, locked (`shstk|wrss`) unless `permissive`. All cold.
* **C** dlopen of an unmarked object with the shadow stack on: refused (dlerror names SHSTK), loaded
  (`on`), or loaded after turning every thread's shadow stack off (`permissive`).
* **F** `clone3` with `stack`/`stack_size` while the creator has a shadow stack; ENOSYS -> `clone`.
* **D-21.7** context.S: `__ssp[0]` holds the raw SSP; with a shadow stack get/swap/makecontext set
  `uc_flags` to 0; setcontext on a context with `UC_SIGCONTEXT_SS` (a kernel signal frame) pops to the
  innermost signal frame's interrupted SSP (token = SSP | bit 63). Without a shadow stack `uc_flags`
  is untouched (reference probe: all three leave 0xff..ff; a signal frame carries 0x7).
* Docs: two rows in differences-from-glibc.md, tunables.md rows.

## Evidence

* **N100 functional** (nozomi, `shstk-enable-n100e`, kit built on mei from 6e647e491's products):
  optin-ref 0, optin-niji 0, optin-static 0 (all 18 rows incl. dlopen refuse/on/permissive, thread
  shadow stack 262,144 B); shadow-static (s21_shadow_stack incl. the new handler-setcontext step):
  "all steps passed"; **control** shadow-static-ctl (context.S minus the signal-frame branch, linked
  ahead of libc.a): SIGSEGV (exit 139). clone3 negative control (seccomp ENOSYS): fallback works, 't'
  row fails as it must (67,108,864 B). optin-static-ref: the known reference-static difference
  (`x86_shstk=off` ignored there; differences row).
* **posix_spawn with the shadow stack on** (item H): a step of s21_shadow_stack.c, passes on N100.
* **Tunable probe** (leg 1, reference on N100): `hwcaps=SHSTK` on+locked(shstk wrss); `shstk`,
  `SHSTK_Usable`, `-SHSTK`, `SHSTK,-SHSTK`, `hwcaps=SHSTK:hwcaps=AVX2` off; `-SHSTK,SHSTK`,
  `IBT,SHSTK` on; `x86_shstk=OFF|On|PERMISSIVE|bogus` = default; `x86_shstk=off` before or after
  `hwcaps=SHSTK` vetoes (dynamic); `off:...:on` on; unmarked program with `on`: off. libniji matches
  every row (unit test `shstk_tunables`, conformance s21_shstk_optin).

### Per call, default mode (no shadow stack; Skylake-SP, `mei:shstk-enable-percall3`, base 0495c4684)

| case | glibc | base | new |
|---|---:|---:|---:|
| thread_create_join | 2,266 | 36,092.0 | 36,105.2 (+13.2: item F's test) |
| getcontext | 45 | 41 | 46 (SHSTK-OPT-IN's +5; this branch 0) |
| swapcontext | 62 | 64.1 | 72.0 (SHSTK-OPT-IN's; this branch 0) |

### Start-up, default mode and opt-in on a CPU without the feature (Skylake-SP, `mei:shstk-enable-su7`)

Final code head 73d2ad2f6 (fast products of `shstk-enable-xtest4`'s build, copied before
`hottext4` rebuilt the tree). Before = SHSTK-OPT-IN head a2cee937a (this branch's base), after =
this branch; empty program, mean of 50 runs, two passes in one job (base rows agree to 1).
Dynamic rows run through each side's ld.so (program mode).

| row | before instr:u | after instr:u | faults before -> after |
|---|---:|---:|---|
| static | 2,927 | 2,934 (+7) | 8 -> 8 |
| dynamic | 189,266 / 189,267 | 189,309 / 189,297 (+30..+43) | 67 -> 67 |
| static, hwcaps=SHSTK | 6,257 | 7,566 (+1,309) | 12 -> 17 |
| dynamic, hwcaps=SHSTK | 192,727 | 193,512 / 193,495 (+768..+785) | 69 -> 69 |

The static +7 is the not-asked tunable test and the larger `Tunables` default. The dynamic +30..+43
(0.02%) is stable within the job (separate jobs on identical binaries differ by up to ~600, so the
earlier su6 read it as noise; within one job it is real). Cause not isolated: in default mode the
loader adds one atomic load and branch in `link`; the one new export (`__niji_shstk_off@@NIJI_PRIVATE`,
libc.so.6 2,822 -> 2,823 dynamic symbols) changes the hash table, a candidate, not verified. The
static opt-in row on a CPU without the feature still reads its own image and maps the cold
decision code (+5 faults); a CPUID test before the image read would skip it (fix-forward, asked
path only). Against the merge base 0495c4684 (`shstk-enable-su5`) dynamic is +385..+438:
SHSTK-OPT-IN's share.

### Layout (§9; `target/product/fast/layout-bins.tsv`, a2cee937a vs 73d2ad2f6)

551 rows each; the bin plan is unchanged except: bin 0 `runtime::start` 5,658 -> 5,808 bytes
(+150), `cpu::init` 50 -> 60 (+10); bin 0's cold filler swapped (`timefmt` 448 B -> `pairscan_ci`
328 B); bin 1 `time_compat::select` 3,953 -> 3,079; the rest are `.llvm.<hash>` renames and
page-fault mode lists. The cold move (73d2ad2f6) did not shrink `start` (f4e60abca: 5,799): a
disassembly diff of `start` (+28 instructions, frame 0xf8 -> 0x108) shows the `shstk == Some(true)`
test and branch, the wider `Tunables` default store (4+1 bytes instead of 2), and register
churn from passing `&aux` (it pins the auxiliary-vector view to the stack). Fix-forward: pass only
`&tunables` and read AT_PHDR/AT_PHNUM in the cold function through `process::auxval` (not done:
it needs a fresh gate cycle that does not fit this run's wall time). Start-up faults are unchanged.

### N100 cost table, opt-in off vs on (nozomi `shstk-enable-cost1`, 3 passes, medians; dynamic rows minus env's own cost)

g = the reference (glibc 2.39 dynamic/static), n = libniji. Per thread = (2000 threads - 0) / 2000.

| row | impl | metric | off | on | on - off |
|---|---|---|---:|---:|---:|
| start empty | g | instr:u | 117,808 | 117,679 | -129 |
| start empty | g | instr:k | 590,697 | 606,845 | 16,148 |
| start empty | g | faults | 51 | 52 | 1 |
| start empty | n | instr:u | 186,700 | 186,860 | 160 |
| start empty | n | instr:k | 827,790 | 843,277 | 15,487 |
| start empty | n | faults | 66 | 69 | 3 |
| start main3 | g | instr:u | 154,729 | 154,656 | -73 |
| start main3 | g | instr:k | 935,445 | 946,792 | 11,347 |
| start main3 | g | faults | 67 | 68 | 1 |
| start main3 | n | instr:u | 291,409 | 292,514 | 1,105 |
| start main3 | n | instr:k | 1,303,753 | 1,324,042 | 20,289 |
| start main3 | n | faults | 79 | 81 | 2 |
| start empty_static | g | instr:u | 154,478 | 154,617 | 139 |
| start empty_static | g | instr:k | 296,865 | 317,366 | 20,501 |
| start empty_static | g | faults | 26 | 27 | 1 |
| start empty_static | n | instr:u | 6,275 | 7,455 | 1,180 |
| start empty_static | n | instr:k | 228,962 | 280,112 | 51,150 |
| start empty_static | n | faults | 12 | 17 | 5 |
| per thread | g | instr:u | 2,275.9 | 2,279.0 | 3.0 |
| per thread | g | instr:k | 53,733.4 | 66,852.9 | 13,119.5 |
| per thread | g | faults | 0.0 | 1.0 | 1.0 |
| per thread | n | instr:u | 40,231.1 | 40,262.5 | 31.4 |
| per thread | n | instr:k | 44,990.2 | 57,722.3 | 12,732.1 |
| per thread | n | faults | 0.0 | 1.0 | 1.0 |
| per static_thread | g | instr:u | 2,179.1 | 2,180.2 | 1.0 |
| per static_thread | g | instr:k | 51,422.3 | 67,039.1 | 15,616.8 |
| per static_thread | g | faults | 0.0 | 1.0 | 1.0 |
| per static_thread | n | instr:u | 40,150.5 | 40,182.5 | 32.0 |
| per static_thread | n | instr:k | 45,009.4 | 57,478.2 | 12,468.7 |
| per static_thread | n | faults | 0.0 | 1.0 | 1.0 |
| prog_sort | n | instr:u | 513,676,889 | 513,678,043 | 1,154 (+0.00%) |
| prog_sort | n | instr:k | 24,871,002 | 24,721,798 | -149,204 (-0.60%) |
| prog_sort | n | faults | 3,770 | 3,771 | 1 (+0.03%) |
| prog_python | n | instr:u | 75,884,110 | 75,839,522 | -44,588 (-0.06%) |
| prog_python | n | instr:k | 13,672,471 | 13,625,631 | -46,840 (-0.34%) |
| prog_python | n | faults | 1,601 | 1,601 | 0 (+0.00%) |
| prog_perl | n | instr:u | 225,087,830 | 224,880,394 | -207,436 (-0.09%) |
| prog_perl | n | instr:k | 24,939,723 | 24,885,488 | -54,235 (-0.22%) |
| prog_perl | n | faults | 4,810 | 4,811 | 1 (+0.02%) |
| prog_find | n | instr:u | 12,838,421 | 12,840,076 | 1,655 (+0.01%) |
| prog_find | n | instr:k | 32,561,384 | 32,645,914 | 84,530 (+0.26%) |
| prog_find | n | faults | 319 | 318 | -1 (-0.31%) |

## Plants

`handoff/plants/SHSTK-ENABLE.plants` (3): start-first-only (B), dlopen-default-loads (C),
shstk-off-ignored (K). D-21.7's branch is not plantable on the Skylake-SP gate host (the test SKIPs
without user shadow stacks); its control is the N100 SIGSEGV run above.

## Open

* RULING NEEDED: makecontext shadow stacks are never unmapped (D-21.7 (5)); recommend a registry
  keyed to `uc_stack.ss_sp` that unmaps the earlier one when makecontext reuses the same stack memory.
* RULING NEEDED: reference-static `x86_shstk=off` difference; recommend keeping the veto (row, no
  compat option).
* RULING NEEDED: the -2 instructions per get/swapcontext fix-forward needs `uc_flags` written in
  every mode (a default-mode difference); recommend keeping the default mode exact (taken: 0 change).
* INFORMATIONAL: with `GLIBC_TUNABLES` set, a libniji dynamic program run directly (libc.so.6 pinned
  to its install dir, D-27.16) fails "libc.so.6: cannot open shared object file" (rc 127) on mei, the
  same at 0495c4684 and here; without the variable it runs. Not this branch's; not chased.
* INFORMATIONAL: setcontext on a kernel signal ucontext restores the x87 environment and MXCSR from
  the user-context layout (fldenv from `fpregs`, ldmxcsr at byte 448), which in a kernel frame are
  the fxsave layout and not the library's field; pre-existing, not changed here; the N100 step passes.
* Cost accepted: thread create +13.2 instructions default mode (fix-forward: inline `rdssp` instead
  of the `niji_machine_ssp` call, about -5); static start-up +7, dynamic +30..+43 (0.02%), `start`
  +150 bytes (fix-forwards in the start-up and layout sections).

## Final-head jobs (code head 73d2ad2f6, mei)

* `shstk-enable-lint4`: clippy --workspace --all-targets --all-features -D warnings, then
  `cargo xtask static`: rc 0.
* `shstk-enable-plants4`: 3 plants, 3 CAUGHT, 0 WARN (results identical to
  `handoff/SHSTK-ENABLE-plants.txt`).
* `shstk-enable-xtest4` (`cargo xtask test`): 7,192 run, 7,186 passed, 6 failed, 790 skipped. The
  six (catalogs_products, locale_data x2, locale_products, printf_corpus_products, s7_2_f_0) fail
  the same way in this host's CI gates `ci-gate-691c188e3` and `ci-gate-a176ecda8` (host
  locale data); none is in code this branch touches.
* `shstk-enable-hottext4` (`NIJI_FULL_PROFILE=1 cargo xtask so` + `hot_text_products`): rc 0, 33 passed, 0 failed.
* `shstk-enable-su7`: start-up rows above.
* Earlier heads (code paths they measure unchanged since): `shstk-enable-percall3` (6e647e491),
  `nozomi:shstk-enable-n100e`, `nozomi:shstk-enable-cost1`, `shstk-enable-su6`.
* FULL tier: the train's (csys machine `.S` files: context.S, entry.S, thread_start.S).
