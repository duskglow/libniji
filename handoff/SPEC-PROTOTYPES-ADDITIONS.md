# SPEC-PROTOTYPES-ADDITIONS: rows beyond the reference in prototypes.tsv survive a regeneration

State: branch `task/spec-prototypes-additions` (cut from origin/next 237dbda92); the head and job results are in
handoff/reports/SPEC-PROTOTYPES-ADDITIONS.json. Decisions in force: D-24.12 (owner ruling R3, the GLIBC_2.41
sched_getattr/sched_setattr exports). No new decision record. The mechanism is the one SPEC-ABI-ADDITIONS built for
the versions tables (handoff/SPEC-ABI-ADDITIONS.md), applied to prototypes.tsv. It is described in Appendix C.0
"Additions" (glibc-functional-spec.md), in the extractor's docstring and in `xtask/src/abi_prototypes.rs`.

## What changed

- `spec/abi/prototypes-additions.tsv` (sibling of versions-additions.tsv): columns `name header declaration
  decision`, one row per function libniji declares beyond the reference. Today it holds sched_getattr and
  sched_setattr (D-24.12).
- `spec/tools/abi_extract.py`: `write_prototypes` merges the additions into the reference rows. Each addition goes
  before the first row whose name sorts after its own. It also writes `prototypes.sha256` (sha256 and count of the
  reference rows alone). `reference_prototypes` = the `_GNU_SOURCE` pass plus `prototypes_missing` (gets,
  bsd_signal), so the full run and `--prototypes-only` reproduce the whole table in one run. Before, the table needed
  a second `--prototypes-missing` step that edited the committed file in place. That flag is now an alias of
  `--prototypes-only`.
- `cargo xtask abi-prototypes`, also in `cargo xtask static` (now 44 checks), checks:
  - addition shape: the name is an identifier, the header is not empty, and the declaration declares that name;
  - the decision record exists;
  - no addition is repeated, and none duplicates a reference row;
  - the table minus its additions matches the digest. A hand row fails, naming prototypes-additions.tsv;
  - the table equals the merge byte for byte.
- `prototypes.tsv` + `prototypes.sha256`: the extractor's output on the reference system (job
  spec-prototypes-additions-fresh2). `--prototypes-only`, `--prototypes-missing` and the full run gave identical files.
  The table holds the same rows as before; only the two sched_*attr rows moved into sorted position. Readers key by
  name (noexcept.tsv already had them in sorted position).

## Survey: which extractor tables have hand rows (fresh full native run vs committed, job spec-prototypes-additions-fresh1)

These are identical byte for byte: versions.tsv, initializers.tsv, enumerators.tsv, empty-versions.tsv,
chk-prototypes.tsv (`--prototypes-fortify`), and every other spec/abi file (the extractor writes none of them).
These differ:
- prototypes.tsv: the 2 sched rows. Fixed by this story.
- constants.tsv: 445 diff lines, ERRATA E10 (#6187). Out of scope, not touched.
- layouts.tsv: 25 diff lines. Not a simple additions case, so it is listed here and left alone:
  - two reference rows renamed by hand: Elf32_Phdr/Elf64_Phdr `__p_type` -> `p_type`, the SL-HEADERS-S16-2 note in
    PROVENANCE. These are corrections, not additions; they would need a "respell" mechanism, not an additions file;
  - 13 reference rows the committed table lacks: members of _fpstate, _libc_fpstate, __sigset_t and La_x86_64_regs.
    These are omissions;
  - struct res_sym sits in a different place.
  These are the same differences HEADERS-TABLES-NET recorded in PROVENANCE.
- PROVENANCE: hand notes by design.
- aarch64 tables (`--arch aarch64`): not rerun here (that needs the arm64 sysroot). versions-aarch64.tsv already
  has the additions mechanism. exports-diff-aarch64.tsv is known stale (handoff/SPEC-ABI-ADDITIONS.md).

## Plants

handoff/SPEC-PROTOTYPES-ADDITIONS-plants.txt has 11 plants:
- a hand row (through abi-prototypes, and through `cargo xtask static`);
- an addition without a decision;
- the additions removed;
- the additions file missing;
- the extractor skipping the merge (it runs on the reference system, then the check);
- 5 code plants on the check, each caught by its unit test.

## Next step

Land. Possible follow-up stories (not briefed): a respell/omission mechanism for layouts.tsv; and an aarch64 rerun
to refresh exports-diff-aarch64.tsv.
