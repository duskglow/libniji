# SPEC-ABI-ADDITIONS(-2): exports beyond the reference survive a versions.tsv regeneration

State: branch `task/spec-abi-additions-2` (SPEC-ABI-ADDITIONS's branch plus a merge of origin/next); code head and
job results are in handoff/reports/SPEC-ABI-ADDITIONS-2.json. Decisions in force: D-24.12 (owner ruling R3, the
GLIBC_2.41 exports; its body names the additions file). No new decision record: the mechanism is described in
Appendix C.0 "Additions" (glibc-functional-spec.md), the extractor's docstring and `xtask/src/abi_versions.rs`.

## What the mechanism is (SPEC-ABI-ADDITIONS)

- `spec/abi/versions-additions.tsv`: one row per export libniji adds beyond the reference, columns
  `arch library symbol version binding type scope size decision`. Today: sched_getattr and sched_setattr at
  GLIBC_2.41, all, D-24.12.
- `spec/tools/abi_extract.py`: `write_versions` merges the additions into `versions{,-aarch64}.tsv` (after the last
  row of their library) and writes `versions{,-aarch64}.sha256` (digest and count of the reference rows alone).
  `--versions-only` (native) regenerates only those two files.
- `cargo xtask abi-versions`, also in `cargo xtask static` (`static_list()`): addition shape, decision record exists,
  no repeat, no reference duplicate; table minus additions matches the digest; table equals the merge byte for byte.

## This run (SPEC-ABI-ADDITIONS-2): the CI conflict

- Step 0: clippy and static green on the inherited head 1f9a5c9de (keke r1).
- Merged origin/next (conflicts in `abi_extract.py` and `xtask/src/main.rs`). Kept both sides: next's
  `empty_versions()` / `--empty-versions-only` / `--prototypes-*` modes and SYS-LIBMVEC's reason for
  `--versions-only`; the full run and `--versions-only` write the merged table and digest, and the full run also
  writes `empty-versions.tsv`. `abi-versions` added to next's new `static_list()` (now 42 checks).
- next had added libmvec.so.1's 216 reference rows to versions.tsv, so the committed digest was stale (abi-versions
  failed as designed: "4816 rows ... 4600 rows"). `versions.sha256` was replaced by the extractor's own output
  (job spec-abi-additions-2-regen1), not edited by hand.
- Full regeneration on the final code head (native full run + aarch64 `--symbols-only`, job
  spec-abi-additions-2-regen-final, script `~/work/SPEC-ABI-ADDITIONS-2/regen.sh` on keke): see the report json.

## Can additions and empty-versions share one mechanism?

Not cleanly. empty-versions.tsv is pure reference data (version nodes the reference defines with no symbol),
regenerated wholesale and carrying no decisions; versions-additions.tsv is libniji's own data that the reference
cannot produce, each row tied to a decision record. What they could share: a libniji-added version node with no
symbol would belong in an additions-style file (with a decision), not in empty-versions.tsv. Not done here.

## Informational (not this story)

- `exports-diff-aarch64.tsv` is stale on next: a regeneration adds 216 rows (the libmvec.so.1 x86_64-only exports
  SYS-LIBMVEC added to versions.tsv without rerunning the aarch64 extraction).
- `prototypes.tsv` still carries the two sched_*attr rows by hand (a full rerun drops them; 4 diff lines); the
  same additions shape would fix it, separate story.
- A full native rerun also rewrites `constants.tsv` (454 lines, ERRATA E10), `layouts.tsv` (25 lines) and
  `PROVENANCE` (hand notes). Regenerate versions with `--versions-only` until those are reconciled.
- `cargo xtask affected` escalates to FULL for any spec/abi table change, including the additions file and digests
  read only by the extractor and one static check.

## Next step

Land. Plants: handoff/SPEC-ABI-ADDITIONS-2-plants.txt (7: hand row, no decision, rows removed, file deleted,
digest ignored, merge at end, record unchecked).
