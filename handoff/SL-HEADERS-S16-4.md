# SL-HEADERS-S16-4: slice 16 continuation (plant fix, merge of the new generator, link.h as table rows)

Branch `task/sl-headers-s16-4` (from `task/sl-headers-s16-3`). Say "slice 16" in commits. Role mei:~/w/s2.

**The red of the last run: s16c-vector-newtype SURVIVED.** The plant mutates `xtask/src/abi.rs` (the byte-array newtype of a
memberless 16+ byte typedef), whose output is `crates/abi/src/layout.rs`. The plant's command was `cargo xtask headers-gen --check`,
which reads headers only (abi-gen is deliberately outside `--check`: headers.rs says "`abi-gen --check` is its own gate step"). The check
exists: `cargo xtask abi-gen --check` (a `static` step) reports `generated files are stale`. Nothing was missing in the product; the plant
named the wrong command. The plant now runs `cargo xtask abi-gen --check` (msg `generated files are stale`).

**Done.**
* `git merge origin/next` (121 commits; conflicts in `xtask/src/headers.rs`, `headers/reserved.tsv`, `appendix_c_layouts.rs`): the generator
  now lives in `xtask/src/headers/*.rs` (HEADERS-GEN-COMPLETE). next's `headers.rs` was taken whole, so the shared generator changes of
  b98cbbf56 (`members_nested`, `NAMED_IN_CXX` removal) are not carried: next's generator covers those constructs. Kept from the slice:
  `abi.rs` (vector newtype, which the merge left intact), reserved.tsv `__x86_get_cpuid_feature_leaf emit`.
* Slice 16 as data on the new generator (no generator change): `s16.rs` `linked_elsewhere` lists the twelve `la_*` audit entry points
  (an audit library defines them); `typedefs.tsv` rows for `La_x86_64_xmm` (`float __vector(4)`), `_ymm`, `_zmm`, `_vector`, and their
  `visibility.tsv` rows (`!time64only`, as `La_x86_64_regs`); PROVENANCE line. `La_x86_64_regs`/`retval` get their members again
  (header_checks/s16.py member offsets pass); `Dl_info` in dlfcn.h gains `const` (next's generator reads the row's qualifiers).
* Regenerated: `libc-include/link.h`, `dlfcn.h`, `crates/abi/tests/appendix_c_layouts.rs`.
* Plants file `handoff/SL-HEADERS-S16-4-plants.txt`: 16 plants. The four on generator code that next replaced (layout-typedef,
  elf-typedef-members, elf-enum-width, and the audit-hook's old place) are gone or moved to the slice's data (vector-typedef-row,
  vector-align-row, elf-phdr-row, audit-hook on `s16.rs`).

**Proof.** `headers-gen` + `--check`, `header_check.py` (rc 0), `x86_header_check.py`, `cargo xtask stubs`: job sl-headers-s16-4-hg3 rc 0.
Lint (clippy, `xtask clippy`, `static`): sl-headers-s16-4-lint rc 0. Gate (`cargo xtask affected --run --base origin/next`, direct tier): sl-headers-s16-4-gate rc 0, 15 suites ok, 11268 tests passed,
0 failed, conformance 91 passed, peak 16G. Plants: sl-headers-s16-4-plants2 on the final plants file: 16 CAUGHT, 0 SURVIVED, 0 WARN
(first run sl-headers-s16-4-plants: 14 CAUGHT, 1 SURVIVED (an Elf64_Phdr offset row: headers-gen --check reads no offsets), 1 WARN-NO-MSG;
both fixed in the file). Results with the catchers: `handoff/SL-HEADERS-S16-4-plants.txt`. Time: the plants run took 1 h 54 min
alone on the host (a products build per plant), the gate 77 min.

**Open.**
1. RULING NEEDED (HEADERS-TABLES-COMPLETE): `La_x86_64_ymm` (32 bytes) and `_zmm` (64) are vectors aligned to 16 in the reference; a
   `vector_size` typedef cannot spell that alignment, so their typedefs rows are memberless (an aligned structure of the right size and
   alignment). Layout checks hold; the type is a structure, not a vector (it matters only to the argument-passing class of a by-value use,
   which the audit interface never does). Recommendation: an aligned-vector form in typedefs.tsv.
2. P2 `probe_compare visibility` four `sys/profil.h` FD_* rows close when `<sys/time.h>` (slice 7) is generated; the four La_* rows are
   now in visibility.tsv (rerun the probe to confirm, not done).
3. `thread_db.h`, `proc_service.h` (slices 8/9), `a.out.h`, `stab.h` (no tables); `abi_extract.py` spelling of `Elf*_Phdr` `p_type`;
   P6 (`conformance/run.sh --niji-headers s27_dl*`) needs `cargo xtask so` beside the gate: not run.

**Not to redo.** The merge, the slice files, the abi-gen step, D-X.130, the plants file.
