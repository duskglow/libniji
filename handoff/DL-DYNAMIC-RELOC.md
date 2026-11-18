# DL-DYNAMIC-RELOC handoff (end of leg 2 of run 1, WIP)

Branch `task/dl-dynamic-reloc`, base origin/next de4cdfc8d. Role mei:~/w/s3. Brief: ~/state/briefs/DL-DYNAMIC-RELOC.md.
Before the final hand-back read `~/w/.scratch/DL-DYNAMIC-RELOC/PRIVATE-notes.txt` (scratch, not in the tree) and
follow it. Decision record in force: **D-27.60** (reserved via `ops reserve`, written:
records/decisions/D-27/D-27.60.toml). No existing record or differences row says the opposite (searched
records/ and docs/differences-from-glibc.md: D-27.18 reads the program from its image; consistent).

## Probe-settled behaviour (now in spec/behaviour/dynlink.md "The loaded dynamic array")

Reference: PLTGOT HASH STRTAB SYMTAB RELA JMPREL RELR GNU_HASH VERSYM get the load base (program, start libs, libc,
ld.so, dlopen, static-pie), already in the first constructor; INIT/FINI/*_ARRAY/VERDEF/VERNEED/TLSDESC_*/sizes/
flags unchanged; vDSO untouched. libniji origin/next: all link-time. Reference raw output: mei
`~/work/dl-dynamic-reloc/ref3.txt`; libniji before: `~/work/dl-dynamic-reloc/niji-before.txt`.
INFORMATIONAL (not this story): static-pie own array (reference adjusts, libniji not); DT_DEBUG not set by
libniji; libniji's `dlpi_name` of start libs is the SONAME (reference: path); DT_REL unobservable on x86-64.

## Done (all in the worktree; leg 2 commits it as WIP)

- Code (leg 1 + leg 2): `niji_ld::elf::{RUNTIME_ADDRESS_TAGS, dynamic_published, shift_address_entries,
  Elf::published_dynamic}`, `boot::publish_dynamic`, `reloc::publish_dynamic`; writers in `niji_ld_start` (ld.so),
  `run`'s no-dependency path (leg 2: now published too, after `relocate_self`), `link` (all but interp, before
  protect_relro), `load_set` in dlopen.rs; readers `image_view(.., published)`, `soname_at`. Doc of soname_at fixed.
- Unit tests: `boot::tests::publish_dynamic_adds_bias_to_address_tags` (crates/ld/src/boot.rs),
  `reloc_publish_dynamic_in_the_model` (crates/ld/tests/reloc.rs: the rule's cases, model corpus, idempotent,
  shift-back round trip).
- Conformance: `conformance/c/s27_dynamic_address_entries.c` + helpers `dyr_start.c`, `dyr_open.c`; clause test
  `s27_1_c_8` specified in crates/core/tests/s27_dynlink_clauses.rs (was a stub).
- Probe committed in tools/probe/dl/dynreloc (read-only case removed, per the private note).
- Plants file: handoff/DL-DYNAMIC-RELOC-plants.txt (6 plants; all cmd `cargo test -p niji --test
  s27_dynlink_clauses s27_1_c_8`).
- NOTHING COMPILED YET: mei slots were full for 40 min (clippy never started).

## Jobs

- `dl-dynamic-reloc-step1b` (mei, queued 13:21Z, script `~/work/dl-dynamic-reloc/step1.sh`): conformance on the
  BASE products first (expect FAIL = the "fails on origin/next" proof; target/niji there still holds the base
  build), then clippy, the unit tests, `cargo xtask so` (log ~/work/dl-dynamic-reloc/so-1.log), the probe
  (NIJI_ONLY, `/mnt/zram/claude/dl-dynamic-reloc/probe-after-1`), conformance again, `cargo test -p niji-ld --test
  reloc`. Read with `grep -n "^###\|error\|FAIL\|PASS\|passed\|test result" ~/logs/dl-dynamic-reloc-step1b.log`.
- SUPERSEDED: dl-dynamic-reloc-clippy1, dl-dynamic-reloc-step1 (both killed while queued, tree changed).

## Exact next steps

1. `jobwait dl-dynamic-reloc-step1b`; fix compile/clippy/test failures (re-sync; rerun only what failed). Check the
   probe output: A on the 9 tags for main/libs/libc/ld.so/dlopen.
2. Commit (non-WIP subject, why-body), push. Start-up counts: before/after for hot_text_products and the start-up
   count tests (brief "Start-up cost"; §9 of developer-flow-reference.md), and diff
   `target/product/fast/layout-bins.tsv` against the base. Bounds only tighten; a rise = RULING NEEDED with numbers.
3. `cargo xtask static`, then gate (`cargo xtask affected --run --base origin/next` under ~/bin/gate), plants
   (`cargo xtask plants handoff/DL-DYNAMIC-RELOC-plants.txt --dir /dev/shm/claude/dl-dynamic-reloc/plants-1
   --timeout 600`; mei is Zen 1), `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1
   cargo test -p niji --test hot_text_products`.
4. precheck on the final head, report json handoff/reports/DL-DYNAMIC-RELOC.json (+ "jobs"), handback-check, hand back
   (SECURITY-PRIVATE section from the private note).
