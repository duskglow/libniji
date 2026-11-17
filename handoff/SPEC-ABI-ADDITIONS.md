# SPEC-ABI-ADDITIONS: exports beyond the reference survive a versions.tsv regeneration

State: branch `task/spec-abi-additions` (from origin/next ebb670553); HEAD and job results are in
handoff/reports/SPEC-ABI-ADDITIONS.json. Decisions in force: D-24.12 (owner ruling R3, the GLIBC_2.41 exports; its
body is corrected to name the additions file). No new decision record: the mechanism is described in Appendix C.0
"Additions" (glibc-functional-spec.md), the extractor's docstring and `xtask/src/abi_versions.rs`.

## What changed

- `spec/abi/versions-additions.tsv`: one row per export libniji adds beyond the reference, columns
  `arch library symbol version binding type scope size decision` (`arch` = all, x86_64 or aarch64; `decision` =
  the record id). Today: sched_getattr and sched_setattr at GLIBC_2.41, all, D-24.12.
- `spec/tools/abi_extract.py`: `write_versions` merges the additions into `versions{,-aarch64}.tsv` (each after the
  last row of its library, in file order; refuses a duplicate of a reference row, a row without a decision id, a
  missing file) and writes `versions{,-aarch64}.sha256` (sha256 and count of the reference rows alone, as rendered).
  New `--versions-only` (native) regenerates only those two files.
- `cargo xtask abi-versions`, also in `cargo xtask static`: addition shape, decision record exists under
  records/decisions/, no repeated addition, no reference duplicate; table minus additions matches the digest (a
  hand-added row fails, message names the additions file); table equals the merge byte for byte (a deleted
  additions file or row, a misplaced row fail). 8 unit tests. Registered in `affected::XTASK_TOOLS`.
- The two rows moved from beside sched_setscheduler to the end of the libc.so.6 block (both tables). No consumer
  depends on row order (placeholders, compat-gen, symbols.py read by key).

## Finding all additions (keke, the 2.39 reference; arm64 sysroot ~/ref-arm64/root)

A fresh extraction (job spec-abi-additions-fresh0, on origin/next) differed from the committed tables by exactly
the two sched_*attr rows in `versions.tsv` and two in `versions-aarch64.tsv`; exports-diff-aarch64.tsv, layouts,
initializers were identical. Final proof: job spec-abi-additions-regen-final (full native regeneration + aarch64
`--symbols-only` on the final head) - see the report json for the SAME lines.

## Informational (not this story)

- `prototypes.tsv` still carries the two sched_*attr rows by hand; a rerun of the extractor drops them (Appendix
  C.0 says so). Same fix shape would work (prototypes additions file), separate story.
- A full native rerun also rewrites `constants.tsv` (396 diff lines vs the committed file, which came from
  `--constants-only`, ERRATA E10) and `PROVENANCE` (hand notes appended to it). Regenerate versions with
  `--versions-only` until those are reconciled.

## Next step

Land. Plants: handoff/SPEC-ABI-ADDITIONS-plants.txt (7 plants: hand row, no decision, rows removed, file deleted,
digest ignored, merge at end, record unchecked). The extractor's merge has no plant of its own: the byte-for-byte
regeneration proof covers it.
