# STRNCMP-AVX2-WRITER: handoff (final, leg 2)

Branch `task/strncmp-avx2-writer`, role mei:~/w/s1. Base origin/next 9548c734e. Code head f34b3ff8f; later
commits are handoff files only.

## State: COMPLETE (ready to land)
- Fix (fde1e891d): both AVX2-head writer tails of `niji_machine_strncmp` (`.Lsnc_hhit`, `.Lsnc_h32` in
  `csys/src/machine/scan.S`) jumped to `.Lsnc_byte` with xmm3 unset (the head uses only ymm1/ymm2, so xmm3
  could hold anything). Both now `jmp .Lsnc_swriter` (pxor xmm3; movq rax, rcx; jmp .Lsnc_byte), the SSE2
  head's existing tail. Writer path only.
- Harness (selftest, asm after strncmp in scan.S): `niji_machine_strncmp_stepped` (xmm3 = fill, trap flag,
  call), `niji_machine_strncmp_step_trap` (SIGTRAP handler: at the re-read `movzbl (%rdi,%rax),%ecx`, cs
  padding prefixes skipped, stores the byte once, clears TF), `niji_machine_strncmp_step_stores`. Declared in
  `csys/include/niji_csys.h` and `crates/boundary/src/ffi.rs`; wrapper `selftest::strncmp_with_writer_at_reread`.
- Test `strncmp_resumes_after_a_writers_change_with_the_zero_vector_on_both_heads`
  (`crates/boundary/tests/machine_scanners.rs`): both levels, hits at 5 and 37, texts at page offset 4000,
  NUL at 50, 0x01 bytes up to an unmapped page; a child per case.

## Proof
- Gate (mei, `strncmp-avx2-writer-gate`, direct tier, on f34b3ff8f + note): rc 0; 12 suites ok, 10789
  passed, 0 failed, 1501 ignored; conformance 145 passed; peak 14G, oom_kill 0.
- Plants (`strncmp-avx2-writer-plants`, `handoff/STRNCMP-AVX2-WRITER-plants.txt`): 4 of 4 CAUGHT, no WARN.
- Counts (`strncmp-avx2-writer-counts`, `cargo xtask bench --compare --counters --runs 3 cbench_hot --rows
  strncmp_eq16,strncmp_diff8,strncmp_names64`, Skylake-SP), instr/op libniji vs glibc: eq16 36 vs 30, diff8
  35 vs 30, names64 43 vs 39. Before = after: scan.S assembled before and after the change (scratch,
  objdump of `niji_machine_strncmp`): the only instruction differences are the two writer tails (2
  instructions -> 1 jmp each) and the branch targets into them; function size 918 bytes in both, the
  alignment padding before the SSE2 part absorbs the 6 bytes, so every later offset is unchanged. No
  measured base count (`bench --base` needs a git checkout, not a role directory).

## Functions checked for the same register assumption (brief step 1)
- strcmp, SSE2 and AVX2 heads: every head-rescan tail zeroes xmm0 before the rescan; OK.
- casecmp_ord_avx2 / casecmp_ord_sse2: equal non-NUL re-read goes to `niji_hot_casecmp_rest` (a new call,
  r8 = 2), no resume in the kernel; OK.
- casecmp_avx2 / casecmp_sse2, scan.S memcmp, wcspair: index kernels with no re-read; nothing to check.
- strchr / strchrnul: the writer resume (`.Lschr_check` -> `.Lschr_body`) rebuilds xmm0 (splat c) and
  xmm3 (zero) at `.Lschr_body`; OK.
- strstr: a NUL a writer changed goes to `niji_hot_strstr_next` (Rust); OK.
- memcmp family (fold.S): an equal re-read pair goes to `niji_hot_compare_rescan` (Rust); OK.
- No wide-character kernel re-reads (wcspair is the only one in scan.S).
No other function has the bug.

## Open questions / rulings
None.

## Next step
None for this story: the coordinator lands it.
