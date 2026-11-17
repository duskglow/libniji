# DL-CANONICAL-PLT: the loader honours canonical PLT addresses (GitHub issue #10)

**State.** Branch `task/dl-canonical-plt` (cut from next e035b3034; not rebased). Code commit 6b913bd41. Role mei:~/w/s3.
Run 1 stopped at the context checkpoint (~146k) with the gate and plants running.

**What changed.**
- `crates/ld/src/symbols.rs`: `Object::program()` marks the program's lookup view; `canonical_plt(sym)` = undefined
  global/weak `STT_FUNC` with a non-zero value (gABI "Symbol Values": the executable's PLT entry is the function's
  address). `Object::lookup` accepts such a symbol as a definition unless the request is `Request::jump_slot()`.
  Versions need no change (`def_version_name` already falls back to `DT_VERNEED`). Cost on the hot path: one
  flag test for an undefined candidate in the program's hash chain (the canonical symbols are hashed: probe showed
  `f` at the GNU hash `symoffset`).
- `crates/ld/src/reloc.rs`: `target()` takes `jump_slot` (only `R_X86_64_JUMP_SLOT` passes true). A program
  `JUMP_SLOT` of a canonical entry bound to a protected definition elsewhere goes to `protected_canonical` (cold,
  never inlined): refused with `RelocError::IndirectExternAccess(name)` when the definer carries
  `GNU_PROPERTY_1_NEEDED` (0xb0008000) bit 0, else `Image::warn` (new trait method, default no-op).
- `crates/boundary/src/ld.rs`: start marks `objs[0]` as program; `ProcImage::warn` writes `ld.so: ...` to fd 2;
  the error prints like `undefined symbol` (status 127). `dlopen.rs`: `adopt_start` marks k == 0; `reloc_text`
  prints the new error. `dlsym(RTLD_DEFAULT)` gets the canonical address with no code change (program in scope).
- `spec/behaviour/dynlink.md` "Symbol binding": two probe rows. Test `crates/ld/tests/canonical_plt.rs` (3 tests).

**Probe-settled (mei, gcc 13.3, binutils 2.42, reference glibc 2.39; scripts ~/work/DL-CANONICAL-PLT/p/p2.sh, p3.sh).**
Issue program, pr19031, pr22842 (PIE `.long test - .` in data, `-Wa,-mx86-used-note=yes`), dlopen'd `GLOB_DAT`,
`dlsym(RTLD_DEFAULT)`: reference PASS/canonical; libniji FAIL (pr22842 abort) before, PASS after (all six agree).
Indirect-extern-access: on a default-visibility definition it changes nothing; a program built
`-mno-direct-extern-access` has no canonical entry (value 0). Protected definition swapped in after link (ld 2.42
refuses the link): reference warns and runs (addresses differ); protected + marked library: refused, exit 127.
libniji now does the same (own message texts). PIE `leaq test(%rip)` in text cannot link (ld refuses).

**Jobs (mei).** `dl-canonical-plt-build1` (clippy + so, rc 0, contracts hold). `dl-canonical-plt-gate` (static,
affected --run, xtask test, full-profile so + hot_text_products) and `dl-canonical-plt-plants` (10 plants,
`handoff/DL-CANONICAL-PLT-plants.txt`) were RUNNING at hand-back: read `~/logs/<label>.log`.

**NOT VERIFIED / open.**
- Start-up counters (brief proof): not measured. Next: build origin/next in a RAM copy and compare a dynamic niji
  program's instructions:u / page faults (pattern: ~/work/DL-STARTUP-OBJECTS/count2.sh), plus layout-bins.tsv diff
  against the base. Expected: unchanged within noise (one branch per undefined candidate in the program).
- The swapped trixie root (brief proof): not run.
- Decision record: none yet. Next: `cargo xtask records new decision D-NEW --series D-27 --story DL-CANONICAL-PLT`
  (canonical PLT rule, JUMP_SLOT exception, protected warn / indirect-extern-access refusal; cite gABI "Symbol
  Values" and the Linux gABI extensions "Program Properties").
- `dlsym(handle)` on a library handle (scope without the program) returns the library's own address; the reference
  was not probed for that case.
- Lazy binding: libniji binds `JUMP_SLOT` eagerly, so no runtime-resolver path needed the rule.

**Next step.** Read gate and plants verdicts; fix reds; measure counters; add the D-NEW record; hand back.
