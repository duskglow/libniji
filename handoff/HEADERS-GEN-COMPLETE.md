# HEADERS-GEN-COMPLETE: handoff (after leg 3 of run 1, 2026-11-19)

State: branch `task/headers-gen-complete`, cut from origin/next fb63eeec8 (next had not moved at leg 3's start; check
`git fetch` + merge if it has). Code head: the WIP commit "Make the header emitter recursive ..." (leg 3), this note on
it. Role mei:~/w/s2 (cargo only there). Brief `~/state/briefs/HEADERS-GEN-COMPLETE.md`. Scratch
`~/w/.scratch/HEADERS-GEN-COMPLETE/` (`diffs/<branch>.diff` prior art; `rows6-failures.txt`, `rows8-summary.txt`).
Leg 1 (c52cc8ac8): one module per construct family. Leg 2 (451288a8b): rows test `xtask/src/headers/rows.rs`.

## Done in leg 3 (WIP commit; libc-include NOT yet regenerated in the worktree)

* aggregates.rs: ONE recursive member emitter (`aggregate` -> `Agg {kw, attrs, td_attrs, lines, nested}`, `level`,
  `member`): `<anon>` rows (`size N rows K`) at any depth, named `struct|union <anon>` (one line when plain),
  `struct <anon>[N]`, typedef-keyed aggregates, tag members (defined first at file scope), `const`/`volatile`
  pointees kept, `enum <anon>`/`enum X` -> unsigned int, bit-fields of scalar typedefs, over-alignment (`attrs` on the
  type, or `td_attrs` on the typedef when size % align != 0: `__pthread_unwind_buf_t`), packed, padding; tag()
  falls back to an opaque definition from the typedefs.tsv row when no rows describe the tag anywhere.
* headers.rs derive_layouts: also struct/union tags (typedefs.tsv size), all descendants rebased.
* typedefs.rs: char16_t/char32_t (`__CHAR16_TYPE__`, `#if !defined __cplusplus || __cplusplus < 201103L`), char8_t
  (C23 unsigned char unless `__cpp_char8_t`), `__gwchar_t` -> `__NIJI_WCHAR`, arrays of a tag (`jmp_buf`), va_list ->
  `__builtin_va_list`, typedef sharing its tag's name (`ucontext_t`), incomplete typedefs (empty size: `DIR`),
  linkage-named union/struct with the `{kind}_{cxx}` guard (`union sigval` once).
* types.rs: type_align, scalar_size_align, c_plain, c_words; type_words: typedefs.tsv tag rows, unknown names ->
  `struct w` declared first, `va_list *` -> `__builtin_va_list *` (was `__builtin_va_list`: printf.h fix).
* functions.rs from_type via c_words (was: every pointer a `struct T *`); `_Bool` -> `#ifdef __cplusplus bool`.
  objects.rs via type_words (`struct in6_addr const`, `struct _IO_FILE *`).
* rows.rs: cxx_type (elaborate a class only when hidden by a function/object or undeclared), va_list spellings,
  layouts of array typedefs use the element (`__typeof__ (((jmp_buf *) 0)[0][0])`).
* precheck.rs: check `slice-contract` + unit test `slice_contract_names_a_slice_change_to_the_shared_generator`.
* Dev jobs (kind build, superseded, not proof): headers-gen-complete-rows5..rows8 on mei.

## Rows test: 92 of 23879 rows fail (rows8), every one diagnosed

* Exceptions to write in `headers/rows-exceptions.tsv` (table, name, lang, reason; each a RULING NEEDED):
  enum tags without enumerators (typedefs enum __pid_type, __ptrace_request, __rlimit_resource, __vlimit_resource,
  mcheck_status; C++/C users: prototypes+typedefs function mcheck, mcheck_pedantic, mprobe, prlimit, prlimit64,
  ptrace, vlimit; recommend: a table of each enumeration's enumerators so `@@enum` is data); la_* (12 prototypes)
  and ps_* (4 typedefs function): provided by the program (rtld-audit(7), proc_service), S16's `linked_elsewhere`;
  bindresvport6, getumask: no default-version export (RULING: should libniji declare them); table depth gap:
  named `<anon>` with no rows below (siginfo_t `_sifields._sigfault._bounds`, `__atomic_wide_counter.__value32`
  -> layouts siginfo_t, __atomic_wide_counter, pthread_cond_t, struct __pthread_cond_s; check struct __res_state's
  cause the same way, it has `_u._ext` rows); layouts va_list lang c++ (G++'s `__va_list_tag` is no class);
  21 constants with no macros.tsv row and 12 visibility functions (signbit, fpclassify, isgreater... = macros;
  gets, bsd_signal) with no prototype row: check each, exception with reason.
* Still to fix in code: sockaddr functions (accept, accept4, bind, connect, getpeername, getsockname, recvfrom,
  sendto): the emitter declares `__SOCKADDR_ARG`, C++ wants `struct sockaddr *` (see how rows.c defines
  `__SOCKADDR_ARG`; per-language spelling, or exception); initializers PTHREAD_ONCE_INIT (scalar typedef) and `{0}`
  (mbstate_t -> follow the typedef to `__mbstate_t`) in values.rs `init_of`.

## Next steps (in order)

1. Fix sockaddr + init_of; write the exceptions; rerun: `ssh mei: setsid nohup ~/bin/jobrun headers-gen-complete-rowsN
   build -- sh ~/work/headers-gen-complete/rows.sh rowsN` (regenerates libc-include in the role, writes
   `/mnt/zram/claude/headers-gen-complete/rowsN/{include.diff,rows/failures.txt}`); green = 0 failed.
2. Review include.diff (rows8: 221 lines in 10 files: pthread_mutex/cond/rwlock_t opaque -> table layouts in
   stdlib.h and sys/types.h, `union epoll_data` guard, const members in argp.h/getopt.h, printf.h `va_list *`, ...);
   each change needs a reason in the hand-back (brief step 2). Copy the regenerated headers into the worktree (tar
   from the role) and commit them; `headers-gen --check` must pass.
3. Synthetic-row unit test for the vector construct (plant s16c-vector-newtype): a layouts-only typedef (size 16,
   align 16, member rows) injected into `Tables`, `rows::check(root, &t, dir, Some(&only))`.
4. Plants file `handoff/HEADERS-GEN-COMPLETE-plants.txt`: one per module (aggregates, typedefs, functions, objects,
   values, enums/item, types, cond), the vector one, the contract check; cmd `cargo test -p xtask --bin xtask
   headers::tests::every_table_row` / the precheck unit test; run `cargo xtask plants`.
5. plan/headers.md 2.4 "Generated tests" + 4.1 slice contract; D-row via `ops reserve` if needed.
6. Gate (`cargo xtask affected --run`), plants, clippy + static, precheck --jobs, report json, handback-check.

## Not to redo / notes

Inventory (leg 1) of next's tables: layouts 1913, typedefs 3889, macros 7387, constants 7084, prototypes 3781,
callsites 7890, visibility 72278, initializers 10, noexcept 3781, ftm-configs 40 (constructs per table in
`git show 5ad788f0d:handoff/HEADERS-GEN-COMPLETE.md`, with the branch prior art). INFORMATIONAL, self-reported (leg 2):
`git show origin/task/sl-headers-s16-3:spec/abi/typedefs.tsv | grep xmm` read a file the brief does not name.
