# STRNCMP-AVX2-WRITER: handoff (leg 1 → leg 2)

Branch `task/strncmp-avx2-writer`, role mei:~/w/s1 (synced at f34b3ff8f + this note). Base origin/next 9548c734e.

## Done (leg 1)
- Premise confirmed: both AVX2-head writer tails (`.Lsnc_hhit` and `.Lsnc_h32` in `niji_machine_strncmp`,
  `csys/src/machine/scan.S`) jumped to `.Lsnc_byte` with xmm3 unset. Fix: both now `jmp .Lsnc_swriter`
  (pxor xmm3; movq rax, rcx; jmp .Lsnc_byte), the SSE2 head's existing tail. Writer path only; hot paths
  unchanged (the tail is 2 bytes shorter; check the bench rows, step 4).
- Harness (selftest, asm in scan.S after strncmp): `niji_machine_strncmp_stepped` (xmm3 = fill, trap flag,
  call), `niji_machine_strncmp_step_trap` (SA_SIGINFO SIGTRAP handler: at the next instruction
  `movzbl (%rdi,%rax),%ecx` after cs padding prefixes, stores the byte once, clears TF),
  `niji_machine_strncmp_step_stores`. Declared in `csys/include/niji_csys.h`, `crates/boundary/src/ffi.rs`;
  wrapper `selftest::strncmp_with_writer_at_reread`.
- Test `strncmp_resumes_after_a_writers_change_with_the_zero_vector_on_both_heads`
  (`crates/boundary/tests/machine_scanners.rs`, end of file): both levels, hits at 5 and 37, texts at page
  offset 4000, NUL at 50, 0x01 bytes to an unmapped page; child per case. Green on mei (job
  strncmp-avx2-writer-t2, which also ran clippy + static: rc 0).
- Lesson: the layout pads instructions with `cs` (0x2e) prefixes; a byte-pattern match must skip them.

## Functions checked for the same register assumption (step 1)
- strcmp (SSE2 and AVX2 heads): every `*_head_rescan` / `*_v_head*` tail zeroes xmm0 before
  `.Lstrcmp_rescan` / `.Lstrcmp_v_rescan`; the settle paths come from loops entered after `pxor xmm0`. OK.
- NOT yet checked (leg 2, read-only, record the result): casecmp_avx2/sse2, casecmp_ord_avx2/sse2
  (writer re-read at scan.S ~5308/5600/5849), strchr/strchrnul (~6113), memrchr (~6499/6522), strstr
  (~7719 note), fold.S compare (~536), memcmp family. For each: does a writer-resume jump land on code that
  assumes a constant vector register its own head did not set?

## Jobs started at the end of leg 1 (on mei)
- `strncmp-avx2-writer-gate` (direct tier, `cargo xtask affected --base origin/next --run`)
- `strncmp-avx2-writer-plants` (`handoff/STRNCMP-AVX2-WRITER-plants.txt`, 4 plants, dir
  /mnt/zram/claude/strncmp-avx2-writer/plants-1); expected all CAUGHT.

## Exact next steps (leg 2)
1. `~/bin/jobwait strncmp-avx2-writer-gate strncmp-avx2-writer-plants` on mei; read summaries
   (`~/bin/test-summary ~/logs/<label>.log`, plants results.txt). Fix any red.
2. Finish the step-1 audit above (grep, read by range); fix any same-pattern bug found.
3. Counts: find the strncmp bench rows (`cargo xtask bench --help` / list), run
   `cargo xtask bench --compare --counters <name>` as job `strncmp-avx2-writer-counts`; rows must not move.
4. If this note's commit is the final head: `cargo xtask precheck --jobs <labels> --head <sha>` on the role,
   then write `handoff/reports/STRNCMP-AVX2-WRITER.json` (with `jobs`), push, `ops handback-check`.
