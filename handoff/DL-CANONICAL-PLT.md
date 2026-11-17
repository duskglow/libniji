# DL-CANONICAL-PLT: the loader honours canonical PLT addresses (GitHub issue #10)

**State.** Branch `task/dl-canonical-plt-2` (cut from next e035b3034; not rebased), HEAD after this note's commit.
Code: 6b913bd41 (rule), unchanged since. Run 2 (DL-CANONICAL-PLT-2) added the SysV-hash test (ec272402f), the
pending record `records/pending/DL-CANONICAL-PLT-2-1.toml` (D-NEW, series D-27), tried a restructure (e1f9e6f47)
and reverted it (512c51462: measured worse). Role mei:~/w/s3. Scripts: ~/work/DL-CANONICAL-PLT-2/ on mei.

**What the code does** (detail in the pending record): the program's view (`Object::program`, objs[0] at start,
k == 0 in dlopen) accepts an undefined global/weak `STT_FUNC` with a non-zero value as the definition for every
request but `JUMP_SLOT`; a program `JUMP_SLOT` of such an entry bound to a protected definition warns, or is
refused (127) when the definer has `GNU_PROPERTY_1_NEEDED_INDIRECT_EXTERN_ACCESS`. Probe-settled behaviour: run 1's
rows in spec/behaviour/dynlink.md "Symbol binding"; run 2 probe (mei, reference): a PIE or non-PIE linked
`--hash-style=sysv` that only calls `g` has `g` UND value 0 in its hashed dynsym, and the library's `return g;` and
`dlsym(RTLD_DEFAULT)` give the library's own `g`.

**Plants.** plants3 (run 1) survivor `value-zero-counts` is now caught: a GNU hash table leaves undefined value-0
symbols out of the chains, so only a SysV-hash program observes the value test; new test
`s27_1_g_0_sysv_hash_program_call_only_reference_binds_library_definition`. `dl-canonical-plt-2-plants1` (code as
now) reached no-canonical CAUGHT, value-zero-counts CAUGHT, then was stopped for the restructure. Full rerun of
`handoff/DL-CANONICAL-PLT-plants.txt` still to do (new dir, e.g. /mnt/zram/claude/DL-CANONICAL-PLT-3/plants-1).

**Start-up counters: NOT EQUAL (blocks landing).** count.sh (dynamic niji PIE with N trivial libraries, merge base
built in RAM from ~/wl/src/e035b3034 vs the role; perf stat -r 20, three repeats, deterministic):
instructions:u N=0 9037837 -> 9043598 (+5761), N=3 9318361 -> 9323756 (+5395), N=12 10121769 -> 10127448 (+5679);
page faults 83 -> 83, 98 -> 99 (2 of 3), 134 -> 135. layout-bins.tsv identical; ld.so .text 0x44664 -> 0x46826
(+8.6 KB: `reloc::target` +221 B, `reloc::one` +305 B, lookup closure +65 B, `protected_canonical` 1.5 KB, the
rest inlining churn: thread::SPARE/map_thread/once_* newly outlined, niji_ld_start -439 B).
Sampled profile (perf record instructions:u -c 1009, 300 starts, N=0; prof.sh): reloc functions only ~+550
instr/start; more work in leaf functions `niji_machine_equal` (~+1060), `memmove` (~+820), memset/fill_large
(~+900), `Relas::next` (~+450), `Verdefs::next` (~+320), and moves between read_phdr / image_span_kind / image_view.
Tried: drop the `program` field from `Object` and use index 0 (as COPY does): N=0 +5497, N=3 +8795, N=12 +19401,
faults +1/+2/+1 -> worse, reverted. So Object's size is not the cause.
Caveat for the next run: base and branch programs name loaders by paths of different lengths
(/mnt/zram/claude/DL-CANONICAL-PLT-2/base/target/niji vs ~/w/s3/target/niji): put both product sets at
equal-length paths before trusting a few hundred instructions.

**Other jobs.** dl-canonical-plt-2-lint0 rc 0 (clippy + static on the inherited head). dl-canonical-plt-2-t1 rc 0
(4 canonical_plt tests). dl-canonical-plt-2-deb rc 1: trixie swap same=63 differ=2, both `ldd -v` (ldd-6, ldd-7)
listing libc.so.6's own version needs (the reference's libc needs ld.so's GLIBC_* versions); not a binding
difference, not chased. Gate on the final head: not run (the gate on e1f9e6f47 was stopped when it was reverted).

**Next step.** (1) Find where the +5.7k goes, exactly: equal-length paths, then per-function counts (no valgrind
on mei; try `perf record -e instructions:u:pp` over 1000 starts, or a counting build). Leading suspects: the
extra `name` comparisons (`niji_machine_equal` +~1060: a candidate compared by name that was rejected before?),
codegen churn from `protected_canonical`'s iterator chain (move `needs_indirect_extern_access` behind one
`#[cold]` call that takes only indices), and `Request` growth. (2) Fix to parity, rerun counts. (3) Plants (full
file), gate (~/work/DL-CANONICAL-PLT-2/gate.sh: static, clippy, records check, affected --base origin/next,
xtask test, full-profile so + hot_text_products), deb swap optional. Plan rows: the pending record only.
