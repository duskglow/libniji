# SL-HEADERS-S16-5: slice 16 continuation (merge of next after run 4, three fixes the merge exposed)

Branch `task/sl-headers-s16-5` (from `task/sl-headers-s16-4`). Say "slice 16" in commits. Role keke:~/w/n2.

**Done this run.**
* `git merge origin/next`. Conflicts: `spec/abi/layouts.tsv` (next's `Dl_serinfo` nested-member rows kept, then slice 16's rows),
  `spec/abi/PROVENANCE` and `docs/differences-from-glibc.md` (both sides kept), `spec/tools/header_check.py`,
  generated `crates/abi/tests/appendix_c_layouts.rs` (regenerated).
* `header_check.py`: next's `must_fail` form and slice 16's `p1 reject` P5 unit (sys/elf.h, ruling #6423) both kept; taking next's
  side alone had dropped the reject unit and orphaned the plant `s16c-reject-row`.
* `headers/dlfcn.h.in`: `RTLD_DEFAULT` / `RTLD_NEXT` use `@@value(...)` (next's generator now refuses a header that spells
  a value `macros.tsv` owns). Generated output unchanged in meaning (`((void *) (-1L))`).
* `header_checks/s16.py`: the offsets check skips `__pthread*` structures (their members now appear in layouts.tsv; the
  reference shows `struct __pthread_rwlock_arch_t` in link.h under POSIX 1995 where libniji's `<sys/types.h>` does not, a
  `<sys/types.h>` visibility matter, not slice 16's).
* Regenerated `libc-include/{dlfcn,link,sys/gmon,sys/profil}.h` and `appendix_c_layouts.rs` (headers-gen/abi-gen --check ok).

* `xtask/.../s16.rs`: `linked_elsewhere` emptied (the `la_*` names are now `headers/providers.tsv` rows); the audit-hook plant is
  gone; the two x86 table plants run `headers-gen --check` (the x86 checker reads its index from the table).

**FINAL PROOF (head e7439faf6):** gate `sl-headers-s16-5-gate3` rc 0 (18 suites, 11592 passed, 0 failed, conformance 93, peak 11G);
plants `sl-headers-s16-5-plants4` rc 0: 15 CAUGHT, 0 SURVIVED, 0 WARN; clippy, static, headers-gen/abi-gen --check, header_check 0 failed.
precheck: all green except slice-contract (header_check.py is shared tooling): RULING NEEDED, see the report json
(recommendation: land, then move sys/elf.h to the `must_fail` form so header_check.py equals next's).
Run 3 earlier plants3 run on the previous head found 3 survivors (audit-hook, x86-index, x86-register): the causes are the
two bullets above.

**Proof (earlier head).** `headers-gen --check`, `abi-gen --check`, `cargo xtask clippy`, `cargo xtask static` rc 0 on the merged head
(before the last two small commits; header_check.py 0 failed of 13677 after them). Gate `sl-headers-s16-5-gate2`, plants
`sl-headers-s16-5-plants3` (file `handoff/SL-HEADERS-S16-4-plants.txt`, 16 plants): results in the report json.
Superseded runs: `-gate` (killed by me: header_check fix needed), `-plants` (refused: directory I had created),
`-plants2` (refused: reject-row plant matched no site after taking next's header_check.py).

**Open (unchanged from run 4).**
1. RULING NEEDED (HEADERS-TABLES-COMPLETE): `La_x86_64_ymm`/`_zmm` are memberless aligned structures; recommendation: an
   aligned-vector form in typedefs.tsv.
2. `thread_db.h`, `proc_service.h` (slices 8/9), `a.out.h`, `stab.h`; P2 probe_compare visibility rerun and P6
   (`conformance/run.sh --niji-headers s27_dl*`, needs `cargo xtask so`) not run.
3. INFORMATIONAL: the reference's `struct __pthread_rwlock_arch_t` visibility in link.h/sys/gmon.h under POSIX 1995 is for the
   `<sys/types.h>` slice.

**Not to redo.** The merge, the slice files, the abi-gen step, D-X.130, the plants file.
