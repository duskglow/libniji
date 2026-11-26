# AFFECTED-LAYOUT-ROWS: a header story's direct-tier gate selects by the changed layout entries

State: branch `task/affected-layout-rows` (from origin/next d1a9ed061). Code: `xtask/src/affected.rs` only. Plants:
`handoff/AFFECTED-LAYOUT-ROWS-plants.txt`. Report record: `handoff/reports/AFFECTED-LAYOUT-ROWS.json`. Built and
tested on mei, role s2. Decision record: pending **D-NEW** (no id reserved; text below).

## Step 1: what drove the header gates (11-24/25, measured)

Parsed every `*headers*gate*.log` of 11-24/25 on the build hosts (46 gates: mei 2, keke 17 with a selection; shiki
gates selected little). Each crate test step's time was attributed to the changed files that selected its targets
(by target count). `crates/abi/src/layout.rs` drove 44,773 s of crate-step time; the next file 406 s
(`crates/boundary/tests/niji_cc_headers.rs`). No other entry-like file drives selection (`appendix_c_layouts.rs`
added only the dependents' unit tests through the image bounded set; `spec/abi/*.tsv`, headers/ and libc-include/
select only generator checks and their readers).

| gate (host) | targets / programs | step sum s | products s | xtask tests s | layout.rs-driven s |
|---|---|---|---|---|---|
| headers-stage2-gaps-b-4-gate6 (mei) | 225 / 103 | 1595 (summary) | 260 | 477 | ~1030 |
| headers-stage2-gaps-b-4-gate5 (mei) | 225 / 103 | 2580 | 382 | 570 | 1566 |
| headers-stage2-gaps-d-4-gate (mei) | 225 / 103 | 2541 | 375 | 564 | 1520 |
| headers-apt-gaps-gate1 (keke) | 225 / 102 | 5340 | 940 | 1112 | 3236 |
| headers-stage2-gaps-c-3-gate (keke) | 225 / 103 | 6260 | 1347 | 1648 | 3205 |
| sl-headers-unshipped-rest-4-gate (keke) | 227 / 103 | 5609 | 1020 | 1250 | 3285 |
| (14 more keke header gates) | 225 / 103 | 3200-5752 | 566-1145 | 751-1428 | 1037-3260 |

The layout-driven steps (`test -p niji`, `niji-boundary` x3 passes, `niji-math` x2, conformance) were 55-60% of each
gate's step time. Cause: a header story adds rows to `spec/abi/layouts.tsv`; `abi-gen` regenerates `layout.rs`;
the image bounded set (D-X.142) mapped it through the direct tier's module rule: module `niji-abi::layout` and its
57 one-hop users (every module naming `layout`), their sections (3,7-14,16-23,27,29) and the C entries reached.

## Step 2: what changed (rule in the module doc, TIERS paragraph)

- `ENTRY_TABLES` (`crates/abi/src/layout.rs`, `layout_aarch64.rs`); `Repo::entry_diff` = `git diff -U0` of
  `crates/abi/` against the merge base (`git_base_diff`, shared with D-X.41's xtask diff), else the file
  `.changed.abi` in a tree without history; `Repo::entry_texts` = the tables' current text.
- `entry_change(diff, file, text)`: Some(changed entries) only when the file has one plain modification section, the
  `+` lines equal the current text where the hunks say (a stale diff is refused), and every added/removed line is
  one `abi-gen` writes for an entry (`#[repr(C)]`/`(C, packed)`/`(C, align(<digits>))`/`(transparent)`,
  `#[derive(Clone, Copy)]`, `pub struct|union X {`, `pub struct X(pub T);`, `    pub f: T,` with T a name or
  `[T; <digits>]`, `}`) lying inside an entry (current text for `+`, base text rebuilt from the diff for `-`), or a
  blank/comment line. Entries whose members name a changed entry are added (fixpoint). Anything else: None.
- Direct tier, `classify_path`: Some -> `narrow_entries` (= `narrow_direct_by` with the entries): users are modules
  whose text names a changed entry as a whole word; targets naming a changed entry are selected; the table's module
  itself selects no namers. None -> today's `narrow_direct`. Wide and full tiers unchanged.
- Image bounded set, direct tier: no dependents' unit tests for an entries-only change or for a file of the crate's
  `tests/` (compiled only into its own targets). Products, k5 and the start-up layout tests stay.

## Step 4: proof (selection: `affected --list`, origin/next's xtask vs this branch's, same trees, mei)

| case | before | after |
|---|---|---|
| headers-stage2-gaps-b-4 (13 entries added, Elf32_Chdr ...) | 225 targets, 103 programs, 4 libs | 5 targets, 0 programs, 1 lib |
| headers-stage2-gaps-d-4 (3 entries: __SOCKADDR_ARG, __CONST_SOCKADDR_ARG, struct_ip_mreq_source) | 225 / 103 / 4 | 5 / 0 / 1 |
| headers-stage2-gaps-d-3 (same 3 entries) | 225 / 103 / 4 | 5 / 0 / 1 |
| layout.rs code (`impl div_t { const ZERO }` appended, synthetic) | 225 / 103 / 4 | identical (every line) |
| libm (code line in crates/math/src/hyper.rs, synthetic; no unlanded libm branch in the last 60) | 12 / 0 / 1 | identical |
| xtask (this branch: xtask/src/affected.rs) | xtask tests only | identical |

Kept for the header branches: niji-abi lib and `appendix_c_layouts` (names the entry), `appendix_c_layouts_aarch64`
(reads layouts.tsv), the three start-up layout tests (libc.a), the xtask tests, generator checks, static, clippy.
Dropped: 220 targets, 103 programs, the niji/niji-boundary/niji-legacy unit tests. Why none can be affected: the
changed entries are new `#[repr(C)]` types that no module of any crate names (0 modules naming them, by whole-word
search, which also counts comments, aliases of the module and glob imports); a type no code names generates no code
and changes no other type (entries containing one are added by the fixpoint); C code sees headers, which keep their
own rules.

Wall time, same host (mei, Skylake-SP class, zram role), headers-stage2-gaps-b-4 tree:
- before: headers-stage2-gaps-b-4-gate6 (origin/next's rule): summary 1595 s (job 27.5 min); products 260 beside
  xtask tests 477, then test -p niji 246 + native 74, niji-boundary 118 + 114 + 47, niji-math 158 + native 266,
  conformance 20, static 16.
- after: affected-layout-rows-hdrb4-after (the b-4 tree with this branch's affected.rs, `.changed` = b-4's files
  plus affected.rs, `.changed.abi` = b-4's crates/abi diff written by hand, as the proposed sync line would):
  summary 821 s (job 881 s), green, peak 5.5G; products 345 beside xtask tests 542, xtask on fresh products 17,
  test -p niji 94 + native 42 (the three start-up layout tests), niji-abi 2, static 16.
- d-4 and d-3: selection only (same 3 entries, same 5 targets); not gated (budget). Non-header cases: selection
  identical line by line, so their gate time is unchanged by step 2.

## Plants time (coordinator note 01:11)

The plants tool takes no selection from `affected`: it builds the products in each tree copy (`cargo xtask so`,
346-469 s per copy on mei, two copies by default) whenever a plant's test names products (`names_products`), then
runs each plant's own command. Only a plant whose command is `cargo xtask affected ... --run` uses this rule (and
needs `.changed.abi` in the copy to narrow). So this fix does not shorten plants jobs in general; what drives them
is the per-copy products build plus each plant's command. Even this story's plants (xtask unit tests only) built the
products twice (~350 s each) because the xtask test text names products: a candidate for the plants tool.

## Results (mei, role s2)

- affected-layout-rows-gate3 (head 36fc32010, this branch's own direct gate): ok, summary 575 s, peak 4.1G
  (products beside the xtask tests, transcripts on the fresh products, static).
- affected-layout-rows-plants2 (same head): 7 plants, 7 CAUGHT, 0 SURVIVED, 0 WARN.
- affected-layout-rows-unit: 112 passed (first code head). affected-layout-rows-lint2: clippy + static ok.
- SUPERSEDED: affected-layout-rows-gate, -gate2 (red: `the_committed_gencat_transcripts_hold`, ENOENT running
  target/niji-test/gencat: the role's products were cleared and an xtask-only selection built none; fixed by
  `builds_products`, commit 36fc32010); affected-layout-rows-plants (6 plants, 6 CAUGHT, before the 7th plant);
  affected-layout-rows-lint (clippy question_mark, fixed).

## Step 3 (overlap): not done; measured reason

After step 2 a header gate's crate steps are ~1 min (3 layout tests + abi); its critical path is products (260 s)
beside "xtask tests that run no products" (477 s), selected by the stories' own `xtask/src/headers*.rs` changes.
Overlapping the crate steps would save little for header gates. It matters for gates whose crate steps are long
(libm: `niji-math` 158 s then native 266 s; boundary x3 passes). Design for a next story: build each step's test
binaries with `cargo test --no-run` (holds the target/debug lock only while building), then run the binaries
directly beside each other, with a bound of `min(cores, cores - loadavg)` / per-step thread share, output buffered
per step and printed in today's order; concurrency defects found that way are fixed, not rerun.

## Open

- **OPS CHANGE NEEDED (no effect on role gates without it):** role trees have no git, so `affected` reads the entry
  diff from `.changed.abi`, which `sync_role.sh` does not write yet. One line after the `.changed.xtask` line:
  `git -C "$wt" diff -U0 "$mb" -- crates/abi/ | "${VSSH[@]}" "cat > ~/w/$role/.changed.abi"` (and add
  `--exclude=/.changed.abi` to the rsync and the `! -name .changed.abi` to the cleanup, as for `.changed.xtask`).
  Until then a role gate selects as before (safe).
- D-NEW (pending id): "Direct tier: an entries-only diff of a generated entry table selects by its changed entries
  (AFFECTED-LAYOUT-ROWS)".
- Reading of the brief: the rule narrows existing per-fix coverage for layout rows; the brief asked for exactly
  this, the train (FULL) still runs everything.

Next step: land; then the ops line above; then step 3 as its own story.
