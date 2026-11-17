# RENAME-NIJI-COMPAT-VIEW: docs/compat-flags.md names renamed interfaces by their new names. State 2026-11-16

Branch `task/rename-niji-compat-view`, base `origin/next` b5be25cd9. Role keke `~/w/n3`.
Brief: `~/state/briefs/RENAME-NIJI-COMPAT-VIEW.md`. Follows RENAME-NIJI-CODE / RENAME-NIJI-DOCS (rename decision D-X.114).

## Done
- **Generator text.** `records/views.toml` (compat-flags header: "libniji's behaviour") and
  `records/views/compat-flags.head.md` (intro "libniji deliberately differs", `niji::config::Tunables`,
  `niji.compat.rpc_reply_any_source`). These are generator inputs, not dated records.
- **Correction of interface names, without editing dated records.** The compat kind has no `amends`/`supersedes`
  (decision records only), and an in-place edit was ruled out by the brief, so the correction is declared next to the
  view: a view key `renames = ["old => new", ...]` (`View::renames`, xtask/src/records/mod.rs), applied by the renderer
  only inside the closed backtick code spans of record cells (render.rs `rename_code`). Prose, rationale, dates and
  rulings render as written; the records and their `liella` column key are untouched.
  `records check` reports a rename that no rendered code span uses ("matches no code span ... remove it"), so the list
  cannot go stale; a malformed entry (not `old => new`, equal names, backticks) is refused when views load.
- **Names mapped (compat-flags view only):** `liella.compat.` -> `niji.compat.` (COMPAT-12, COMPAT-20 tunables),
  `liella::config::` -> `niji::config::` (COMPAT-12, COMPAT-20), `LIELLA_OWNER_1` -> `NIJI_OWNER_1` and
  `liella-owner/` -> `niji-owner/` (COMPAT-30). Each new name checked against the code (crates/core `niji`,
  config.rs `Tunables`, classed.rs `niji.compat.no_classed_swap`, xtask/src/so.rs `niji-owner/`, NIJI_OWNER_1 in
  crates/boundary). `liella_ld::` (COMPAT-25/26) was dropped: it is only in record bodies, which the view does not
  render; the new check flagged it.

## Proof
- Out-of-tree `cargo xtask records-gen` before (origin/next) and after: of 39 views only `docs/compat-flags.md`
  differs (`diff -rq`). The diff touches exactly the intro, the Mechanism paragraph, the header and rows
  COMPAT-12, COMPAT-20, COMPAT-30, with the code spans listed above.
- `grep -n -i liella` on the regenerated view leaves 2 hits, both record prose (not interface names), kept as written:
  COMPAT-11 "leak checkers may report Liella's internal allocations" and COMPAT-16 "The reference's result
  differs from Liella's in the stream's error indicator only".
- Unit tests `renames_touch_code_spans_only`, `stale_and_malformed_renames` (xtask records::render); plants
  handoff/RENAME-NIJI-COMPAT-VIEW-plants.txt (5).
- Jobs on keke, head a7bd08ced: rename-niji-compat-view-lint (cargo xtask clippy + static, rc 0),
  rename-niji-compat-view-gate (affected --run --base origin/next: 3 suites, 462 passed, 0 failed),
  rename-niji-compat-view-plants (5 CAUGHT, 0 SURVIVED, no WARN). Earlier records-gen runs: -base (baseline view),
  -gen (red: views.toml takes no comments), -gen2 (red: stale rename `liella_ld::`, fixed), -gen3 (green, the diff).

## Open / next
- Prose in record cells still says "Liella" (2 hits above). The brief kept rationale as written; if the coordinator
  wants those as "libniji" too, that is a prose correction of the records (or a prose rename list), not this mechanism.
- The train regenerates docs/compat-flags.md; nothing else to do once landed.

## Not to redo
- Do not edit docs/compat-flags.md or the compat records for these names: the view's `renames` does it.
- views.toml takes no comment lines (the records TOML subset refuses them).
