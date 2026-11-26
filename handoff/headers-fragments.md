# HEADERS-FRAGMENTS: handoff (final, leg 3 of run 1)

Branch task/headers-fragments, base origin/next cd31515dc (unchanged on 2026-11-26 10:12Z). Role shiki:~/w/sh1.
Commits (all mine): 924b20dd9 probe/check fragments; f962c734a template fragments (subject says WIP, code final and
proven, no history rewrite); 358f64541 table keys; 4e37752d7 duplicate-definition check; 1cfd99d28 plant msg fix;
then the handoff/report commit. Plants file: handoff/HEADERS-FRAGMENTS-plants.txt (with the run's results).

## What landed (brief step: proof)
1. Probe and check fragments: spec/tools/header_probe.d/<family>.py (`candidates()`), and
   spec/tools/header_checks/<slice>.d/<NN>-<family>.py (`units(ctx)`), loaded in sorted file-name order; shared
   machinery stays in header_probe.py / header_check.py. PROOF (shiki job headers-fragments-checklist): the
   `header_check --list` output for origin/next's spec/tools and for the branch is identical (23264 units), and so
   are the FAIL lines (0 failed of 23694 both). probe candidates() identical (20048 lines, local diff).
2. Template fragments: a template has one `@@fragments` line; xtask headers-gen splices
   headers/<h>.h.d/*.frag in sorted name order. Seven templates have fragments: complex, libintl, math, resolv,
   syslog, utmp, wchar (the brief's six plus resolv.h from the 11-24/25 conflict list). PROOF: `cargo xtask headers-gen --check` reports "ok, 242 headers"
   (libc-include byte-identical). There is one output tree, and configurations are `#if` text inside it, so every
   configuration is identical.
3. Layout tests: appendix_c_layouts.rs is already @generated from spec/abi/layouts.tsv rows, so nothing to split
   (brief premise challenge, cheap).
4. Declared table keys: spec/tools/table_keys.tsv (62 tables, every *.tsv under headers/ and spec/abi/; format at
   its top: path, key columns, sort key or `-`, unit `row`|`block`). The static check is `table-keys` (also
   `cargo xtask table-keys`), and every table on next passes.
5. Duplicate-definition check: spec/tools/header_checks/dup.py, run by header_check.py (problems only, unit list
   unchanged). On next it finds 3808 macro names defined identically in 2+ headers (the generator emits them per
   header: 2953 AT_* elf.h + sys/auxv.h; 392 signal.h + unistd.h; 154 inttypes.h + netinet/icmp6.h + ip6.h;
   utmp.h + utmpx.h; __FD_SETSIZE in 41) and 0 typedef/tag/enumerator duplicates. The struct utmp compat names are
   defined once on next already (fixed before this story). The list is headers/dup-listed.tsv, a ratchet: a new or
   changed duplicate fails, and so does a stale row. EXCEPTIONS (stock does the same) is empty: no stock probe was
   needed because nothing was excepted.
6. Merge proof (local throwaway worktree and branches, removed, never pushed): hf-a and hf-b each add a math.h.d
   fragment, a header_probe.d file, an s12.d check file and a headers/reserved.tsv row 50 lines apart; `git merge
   hf-b` into hf-a: rc 0, no conflict. A control pair, hf-c and hf-d, each add a reserved.tsv row after the SAME
   line: CONFLICT in headers/reserved.tsv only (rc 1, aborted). Adjacent table rows are what table_keys.tsv is for:
   the coordinator's row-by-row merge.

## Jobs (shiki)
* headers-fragments-test1 (rc 0): headers-gen --check, clippy -D warnings, xtask static, `cargo test -p xtask
  --bin xtask` 692 passed (includes network_headers_pass_header_check = header_check.py with dup.py). `cargo
  xtask affected --list` says FULL (fragments are generator inputs), so this job plus precheck is the gate: FULL is
  the train's.
* headers-fragments-plants1 (rc 0, superseded): 13 CAUGHT, 1 WARN-NO-MSG (frag-dropped: caught by
  headers-gen's "has no macros.tsv value" for the fragment-owned names, not the stale message). Its msg was fixed
  in 1cfd99d28.
* headers-fragments-plants2 (rc 0, on 1cfd99d28): 14 plants, 14 CAUGHT, 0 SURVIVED, 0 WARN. It covers: a
  duplicate macro, typedef, tag and enumerator planted in a fragment; the dup.py `#undef`, listed-row and ratchet
  logic; a duplicate reserved.tsv key; the sorted, block and undeclared-table checks; a dropped or misordered
  template fragment; and unloaded check fragments.

## How a header story adds a family now (for the briefs)
* Probe names: a new file spec/tools/header_probe.d/<family>.py with `candidates()`.
* Checks: a new file spec/tools/header_checks/<slice>.d/<NN>-<family>.py with `units(ctx)` (an NN not yet used in
  that slice).
* Template text: a new file headers/<h>.h.d/<NNN>-<family>.frag (with its own `@@owns`/`@@if` lines), then
  `cargo xtask headers-gen`. A new template gets one `@@fragments` line where the families go.
* Table rows: rows in the tables listed in spec/tools/table_keys.tsv; a repeated key fails `cargo xtask static`.
  Put a row next to its own family, not at the end of the file, so two stories rarely add after the same line.
* Layouts: a row block per type in spec/abi/layouts.tsv, then the generators.
* Never add a row to headers/dup-listed.tsv: if your header defines a name another header already defines, dup.py
  fails (in header_check.py). Include the provider header or guard the definition instead.

## Open
* RULING NEEDED: the 3808 listed cross-header identical macro definitions. Recommendation: a separate story makes
  the generator emit each name once, in its provider header (others include it or `#ifndef`-guard), and the list
  shrinks as it lands.
* Next step: none for this story beyond landing; the coordinator can read table_keys.tsv for row-wise merges.
