# HEADERS-GEN-S08-HOOKS: slice 8's generic generator edits, landed on their own

**State.** Branch `task/headers-gen-s08-hooks`, cut from origin/next (5fa9f9366). Head: see `handoff/reports/HEADERS-GEN-S08-HOOKS.json`.
Continued as HEADERS-GEN-S08-HOOKS-2 on `task/headers-gen-s08-hooks-2` (report `handoff/reports/HEADERS-GEN-S08-HOOKS-2.json`): CI went red only on
`headers-gen-check` after SL-HEADERS-S15-3 landed visibility rows. Merged origin/next (d4cbe1667) and regenerated: the only diff is the
`pthread_attr_t` block (same union typedef and `__NIJI_D_pthread_attr_t` guard) in `sys/acct.h`, `sys/mtio.h`, `sys/quota.h`, i.e. the merge of both changes.
Second finding of the -2 run: the direct gate (xtask tests, which the first run never ran) failed `header_p2_fixed_rows`: the union-tag
rule declared `pthread_attr_t` as an unnamed `typedef union {..}` in C++, which the visibility probe classifies as a typedef where the
reference table has none (the reference has a union tag of that name). Cause fixed in `typedefs.rs`: a union typedef with no C++
spelling is now spelled `union pthread_attr_t {..}; typedef union pthread_attr_t pthread_attr_t;` (same layout; `union pthread_attr_t`
now names the type in C and C++). All 15 headers carrying it regenerated (the 12 above plus acct.h, mtio.h, quota.h); plant
`s8-union-tag-spelling` covers it. The statement above that the output "is what slice 8 already carried" is therefore superseded:
SL-HEADERS-S08 should regenerate on top of this.
Ruling #7070 (obstack `chunkfun`/`freefun` spelling) is owed to its own story, not done here.
Premise (brief stated none): header slices are data-only (owner 11-19), so the generic edits of `task/sl-headers-s08-5` land first and
SL-HEADERS-S08 rebases on them as data-only. HEADERS-GEN-S10-HOOKS had not landed on this base (no s10 hook in `xtask/src/headers*`).

**Commits (one per item, why in each body).**
1. Probe tools: `type_probe.py` pointer constants (`pointer` macro type, NULL positional), `abi_types.rs` accepts/rejects `pointer`
   values (unit test line in `macros_shape`), `abi_extract.py` TYPES (signal-context, pthread-internal, sigevent_t), `header_probe.py`
   `standard_names()` (sig_atomic_t, sigevent_t, pthread mutex kinds, protocols, robustness, rwlock kinds). No table rows.
2. Function-pointer members: `headers/fnmembers.tsv` (declarator by layouts.tsv type and member, `@` = member name) read by
   `aggregates.rs`, which declares the tags and typedefs a declarator names first. Only row: `struct argp` `help_filter`, spelled from the
   GNU C Library manual (Argp Help Filtering). Slice 8's rows (sigaction, sigevent, _pthread_cleanup_buffer) are NOT here: they are data.
3. `cond.rs`: `LANGUAGE_MODE` (`max_align_t`), `union_tag` / `cxx_mask` (a union typedef with no C++ spelling is declared in C++).
4. `headers.rs`: an include right after an `#if` line is conditional; the rule is now `unconditional_includes()` with a unit test.
5. Plants file `handoff/HEADERS-GEN-S08-HOOKS-plants.txt`.

**Generated libc-include changes and why.**
* `argp.h`: `help_filter` is `char *(*help_filter)(int, const char *, void *);` (was `void (*help_filter) (void);`).
* `pthread_attr_t` (a union typedef with no C++ spelling) is now declared under `__NIJI_POSIX >= 199506` with no `__NIJI_CXX < 1`
  exclusion, in every header whose visibility rows show it: lastlog.h, stdlib.h, sys/epoll.h, sys/fsuid.h, sys/procfs.h, sys/random.h,
  sys/sem.h, sys/sendfile.h, sys/types.h, sys/uio.h, sys/xattr.h, utmp.h (stdlib.h and sys/types.h move their declaration to the new
  condition). The copies share the `__NIJI_D_pthread_attr_t` guard. Cause: the visibility probe does not classify the C++ union tag as a
  typedef, so the old condition left it out of C++. This is the output slice 8 already carried for these headers.
* Items 3 (`max_align_t`) and 4 change no header on this tree: no condition search fails and no template has an include after `#if`.

**Item 1 coverage.** Every function-pointer member next emitted as `void (*m) (void)`: argp `help_filter` (fixed), obstack `chunkfun` and
`freefun` (NOT fixed: one slot, two calling conventions in the manual, with and without an extra argument; a single declarator
would be wrong for one. Kept as before; a ruling or a union spelling is a separate decision).

**Verified (keke role).** `cargo xtask headers-gen --check` clean; `cargo test -p xtask --bin xtask -- headers:: abi_types::` 23 passed;
clippy `-D warnings` and `cargo xtask static` clean (job `headers-gen-s08-hooks-checks2`); plants job `headers-gen-s08-hooks-plants2`.

**Not to redo.** No second `LANGUAGE_MODE` or union-tag code. SL-HEADERS-S08 should drop its copies of these edits, add its fnmembers rows
(sigaction, sigevent, sigevent_t, _pthread_cleanup_buffer) and its layouts/macros rows, and regenerate (`cargo xtask headers-gen`).

**Equivalent plants (reason in the plants file).** `s8-fnmember-dependency` and `s8-language-mode`: no row or name on this tree reaches
them; slice 8's rows do. The type_probe.py edits are exercised only when slice 8 regenerates macros.tsv (no plant).

**Next step.** Land; then rebase SL-HEADERS-S08 and confirm those two plants are caught there.
