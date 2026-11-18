# DL-CANONICAL-PLT: the loader honours canonical PLT addresses (GitHub issue #10)

**State (run 6, leg 1 ended by REFRESH).** Branch `task/dl-canonical-plt-6` = run 5's branch + one WIP code commit
1c396a7ba (pushed, NOT yet built or tested). Role keke:~/w/n2 (Zen 1 host, 60 vCPU). Run-5 history, numbers and
the plants record: handoff/DL-CANONICAL-PLT-5-report.md (read only if needed). Record:
`records/pending/DL-CANONICAL-PLT-2-1.toml` (design text updated in 1c396a7ba; its **Cost** sentence still has
run-5 numbers and must be replaced with run-6 numbers).

**Ruling #6618 (in force):** the +1 ASLR-on start-up fault median for programs WITHOUT canonical entries was not
accepted. Run 6 must (a) move the flag out of `Object` (done in 1c396a7ba), (b) remeasure the ASLR-on distribution,
(c) add the dladdr-at-load-base test for the surviving plant `dladdr-any-undefined` (done in 1c396a7ba).

**Design now (1c396a7ba).** `Object` has no `canonical` field again (layout as on origin/next).
`Object::canonical_entries()` (the old `program()` scan, `#[inline(never)]`) is asked once per load: in `link`
(crates/boundary/src/ld.rs, `let canonical = syms[0].canonical_entries();` just before `Objects {..}`) and in
dlopen's `adopt_start` (`l.canonical`, new `Loader` field after `n_start_global`). `reloc::Objects` has `pub
canonical: bool`; `symbols::Scope` has a private `entries` bool set by `Scope::program_entries(on)`;
`symbols::lookup` goes to `#[cold] lookup_entries` only when it is set, which searches object 0 with `#[cold]
Object::lookup_program` (`find(req, true)`); plain `Object::lookup` = `find(req, false)` (inline(always) body).
reloc `one()` and `slot_target` pass `set.canonical`; dlsym and `Uniq::find` pass `l.canonical` (Uniq has a
`canonical` field). Protected check: `i == 0 && set.canonical`. dladdr: `containing(syms, program = i == 0, ..)`.
Test: the scoped test prints `dladdr_prog_base` and `dladdr_libdeep_base` (reference: `1 prog (null) null`,
`1 libdeep.so (null) null`; probe job keke:dl-canonical-plt-6-probe-ref3 — note `(void *)use_v` is itself a
canonical entry of the program, so libdeep.so (imports `f`) is the library used).

**Done in leg 1.** lint0 (inherited head: clippy + static green). Reference probe ref3 (above). Scripts on keke
~/work/DL-CANONICAL-PLT-6/: `lint.sh`, `prebuild6.sh <tag> <tree>` (cargo xtask so, products to
/dev/shm/claude/DL-CANONICAL-PLT-6/prod-<tag> + `.bins`, probe vs `probe/ref.expected` = reference output),
`step6.sh <tag>` (lint + prebuild6 on ~/w/n2 + tests canonical_plt/reloc/symbols), `count6.sh <tag> <A> <B> [runs=41]`
(run 5's count script without Intel PT: plain PIE and non-PIE canonical programs, N=0/3/12, instructions and faults,
ASLR off/on with the full fault distribution, N=0 fault-page diff), `probe/`, `base-next/` (origin/next merge base
1b7335a10 source copy). Jobs started at the end of leg 1 (were still in `cargo xtask so` trace-at-build, host load ~15):
`dl-canonical-plt-6-pb-base` (base products -> prod-base) and `dl-canonical-plt-6-pb-run5` (inherited run-5 head
53566166d, built IN THE ROLE ~/w/n2 -> prod-run5).

**Next steps (in order).**
1. `~/bin/jobwait dl-canonical-plt-6-pb-base dl-canonical-plt-6-pb-run5`; both must end PREBUILD-OK and print
   "probe base: DIFFERS" for base (no canonical support) and "probe run5: DIFFERS" only on the two new
   `_base` lines if any (run 5 had no dladdr bug at bases with the flag gate; expect match).
2. Only after pb-run5 ended: sync (`bash ~/src/liella-ops/tools/sync_role.sh --story DL-CANONICAL-PLT-6 --base
   origin/next <worktree> keke n2`, check rc; 4 = BUSY), then job `dl-canonical-plt-6-step1` running
   `sh ~/work/DL-CANONICAL-PLT-6/step6.sh new`. Fix compile/clippy/test failures (1c396a7ba was never compiled).
3. Counts: `bash ~/work/DL-CANONICAL-PLT-6/count6.sh m6 base run5` and `count6.sh m6b base new` (jobrun, kind
   count). The ruling's question: plain PIE ASLR-on fault median base vs new must be equal (run 5 on Skylake-SP:
   base 78, run-5 head 79). Put the distributions in the record's Cost sentence and the report.
4. Gate (direct tier, `--base origin/next`), plants: `cargo xtask plants handoff/DL-CANONICAL-PLT-6-plants.txt
   --dir /dev/shm/claude/dl-canonical-plt-6/plants-1 --timeout 900` (7 plants incl. dladdr-any-undefined).
5. Start-up checks (brief): `cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo
   test -p niji --test hot_text_products`; diff layout-bins (prod-base.bins vs prod-new.bins).
6. precheck on the final head, handoff/DL-CANONICAL-PLT-6-report.md, handoff/reports/DL-CANONICAL-PLT-6.json,
   handback-check.

**Process notes (leg 1, for the report):** one empty heredoc slipped in (a no-op `python3 - <<'X'`, nothing ran);
one probe job (`probe-ref2`) never started because the jobrun was chained after `tar` in one ssh command, and its
jobwait had to be killed by PID (own process). Nothing refused.
