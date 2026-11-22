# SL-HEADERS-S16-3: slice 16 continuation (merged onto the registry layout; gate, plants and proofs: see Jobs)

Branch `task/sl-headers-s16-3` (from `task/sl-headers-s16-2`). Say "slice 16" in commits.

**Done.**
* `git merge origin/next` (conflicts in `spec/tools/header_check.py`, `xtask/src/headers.rs`, `spec/abi/PROVENANCE`), moved onto the
  registry layout (SL-HEADERS-REGISTRY steps 1-4): next's two shared files taken, the slice re-expressed as `headers/slices/s16.tsv`
  (nine headers, `sys/profil.h` with `p1` 0, `sys/elf.h` with `p1` reject), `spec/tools/header_checks/s16.py` (member offsets,
  `sys/profil.h` P1 scope, `gnu/lib-names.h` sonames) and `xtask/src/headers/slices/s16.rs` (markers `@@sonames`, `@@x86features`).
  `header_check.py`: 0 failed of 5409 (before the merge 0 of 2165; M grew with next's slices and the new units), `headers-gen --check`
  clean, `x86_header_check.py` 172 features 0 failed, `cargo xtask stubs` rc 0 after abi-gen regenerated `crates/abi/src/layout.rs` and
  `crates/abi/tests/appendix_c_layouts.rs` (the missed step of the coordinator's note).
* Shared generator code changed (own commit b98cbbf56, named in the hand-back): `headers.rs` `members_nested` (anonymous unions and
  enumerations of `Elf*`/`Dl_*` typedefs, members only `layouts.tsv` describes (`La_x86_64_xmm`), pointer to a covered typedef,
  `empty` aggregate falls back to opaque), `link.h` `la_*` need no export, an object of structure type gets its tag; `abi.rs` (a
  memberless typedef of 16+ bytes becomes an aligned byte array); `header_check.py` (row `p1` reject: the unit must NOT compile).
  `NAMED_IN_CXX` is gone: ruling #6472 is now the two `visibility.tsv` rows (`!time64only`), with a PROVENANCE line, a differences row
  and record D-X.130 (no COMPAT: the row says why).
* `sys/profil.h`: FD_* are `@@owns` in the template until `<sys/time.h>` (slice 7) is generated; header_checks/s16.py runs P1 on `struct prof` only.
* `Elf32/64_Phdr` member `p_type` (was `__p_type`) also changes the generated `sys/auxv.h` (slice 9's header): same layouts rows, now real members for `Elf*_Dyn`/`Elf64_auxv_t`.

**Open (in order).**
1. Proofs (job sl-headers-s16-3-hg6 and sl-headers-s16-3-proofs on mei): P3 `probe_compare callsites` 0 unaccepted of 0 differences (the `__monstartup` row is gone); P2 `probe_compare visibility` 8 of 8 differences remain: four `sys/profil.h` FD_CLR/FD_ISSET/FD_SET/FD_ZERO rows (reference includes `<sys/time.h>`, slice 7: they close when that header exists, then drop `@@owns` in `headers/sys/profil.h.in` and `p1` 0 in `s16.tsv`) and four `link.h` typedefs `La_x86_64_xmm/ymm/zmm/vector` that libniji declares and `visibility.tsv` has no row for (candidate-list gap of the probe, as `RTLD_DL_LINKMAP`/`ELFMAG`: a probe of the reference's own header decides the rows and their C++ configurations; not done). P6 (`conformance/run.sh --niji-headers s27_dl*`) was NOT rerun: it needs `cargo xtask so` in the role beside the gate.
2. `thread_db.h`, `proc_service.h` (slices 8/9), `a.out.h`, `stab.h` (no tables): not generated.
3. P6: `RTLD_DL_LINKMAP`, `ELFMAG` are in no table (candidate-list gap of the probes); four programs "not covered" by other slices.
4. `abi_extract.py`: why its DWARF names the `Elf*_Phdr` member `__p_type` (rows respelled by hand, PROVENANCE).

**Not to redo.** The merge, the slice files, the abi-gen step, D-X.130, the plants file `handoff/SL-HEADERS-S16-3/plants.txt`.

**Jobs (final head, mei role s1).** `sl-headers-s16-3-hg6` (headers-gen, header_check 0 of 5409, x86 172/0, `--check`, stubs, P2: rc 0 steps), `sl-headers-s16-3-lint2` (clippy, xtask clippy, static: rc 0), `sl-headers-s16-3-gate2` (`cargo xtask affected --run --base origin/next`, FULL tier expected: about an hour) and `sl-headers-s16-3-plants2` (16 plants; each plant needs a products build, so hours) were started detached: re-attach with `~/bin/jobwait sl-headers-s16-3-gate2 sl-headers-s16-3-plants2` on mei. Superseded: `sl-headers-s16-3-lint` (killed in `cargo xtask test`, 6836 tests, on the head before the lib-names change), `-gate` and `-plants` (killed on that head to change `gnu/lib-names.h`); the first plants run named WARN `caught by no test the log names` for the x86 plants: the `msg:` regex was widened in the file, recheck in plants2's log.
Time went to host builds per generator round (4-8 min each) and to the plants' products builds (about 9 minutes each).
