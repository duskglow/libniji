# PROVENANCE-FRAGMENTS: handoff (leg 1 done; leg 2 continues)

State: branch task/provenance-fragments, HEAD 76ee82f6c (pushed), base origin/next 855efa170 (no newer next when cut).
Role keke:~/w/n4 (synced at 76ee82f6c). Decision: D-X.172 (reserved, record written).

## Done
* spec/abi/PROVENANCE split into 20 fragments, spec/abi/provenance.d/<ID>.txt, front matter `order: 10..200`, text
  unchanged (checked: the History part of the new view equals the old file's non-blank lines byte for byte). Five
  paragraphs name no story; they are SPEC-ABI-EXTRACT, LIBM-ULPS, CALLSITE-PROBE, DIAGATTRS (descriptive) and
  SL-HEADERS-2 (its handoff names the header_probe PROVENANCE line).
* xtask/src/provenance.rs: parses and validates fragments and counts 17 tables (rows plus distinct columns). The view
  is added to records-gen's rendered views (records/mod.rs `stale()`), so `records-gen` renders it, `--check` and the
  static check `records-gen` see it (advisory off the train, fatal on it: D-X.55/59). A malformed fragment is an
  error everywhere. 4 unit tests (pass on keke, job provenance-fragments-xt, rc 0; records:: tests 21 pass).
* New `cargo xtask records-gen --check <view path>...` (records::check_paths): checks only the named views. Needed
  because the branch's own new record leaves plan/07-decisions view pending (not committed, per D-X.55).
* spec/abi/PROVENANCE committed regenerated (records-gen --in-place on keke; only that view was copied back).
* abi_extract.py writes a fragment (`--story ID`, needed for the full run and --empty-versions-only). appendix_c.py reads
  the ldd line from the fragments (its output Reference system line unchanged; running it also rewrites other parts of
  glibc-functional-spec.md that had drifted before this story: that was discarded, INFORMATIONAL).
* CONTRIBUTING.md lists PROVENANCE with the generated views and points to provenance.d/README.md. No other repo doc
  told slices to edit PROVENANCE.
* Step 3, NOT done, by design: validation/headers_unshipped.tsv is not derivable. Its `waits_for` column is
  hand-written reasons, and the list is a ratchet: P4 fails when a header stops shipping, and a generated list would
  absorb that. 5 rows left.

## Jobs started (keke), at 76ee82f6c
* provenance-fragments-gate: clippy + static + affected --run --base origin/next (kind gate)
* provenance-fragments-plants: handoff/PROVENANCE-FRAGMENTS-plants.txt (3 plants), dir /dev/shm/claude/provenance-fragments/plants-1

## Exact next steps (leg 2)
1. `~/bin/jobwait provenance-fragments-gate provenance-fragments-plants` on keke (vm_ssh). Fix any red, read
   ~/logs/<label>.log with grep/tail only.
2. Merge proof (brief step 5): in a scratch clone (`git clone -q --shared` of the worktree into
   /home/claude/w/.scratch/PROVENANCE-FRAGMENTS/mp), make two branches from HEAD. Each adds a fragment
   (A.txt / B.txt, order 210) and a row to a different table (or a different place in visibility.tsv). Merge both:
   expect no conflict. The summary regenerating is covered by unit test counts_follow_the_tables and plant
   prov-fragment-edit; optionally run `cargo xtask records-gen --check spec/abi/PROVENANCE` on a merged tree.
3. Foreground on role: `cargo clippy --workspace --all-targets --all-features -- -D warnings` (gate's xtask clippy
   may cover it) and `cargo xtask precheck --jobs provenance-fragments-gate,provenance-fragments-plants --head <sha>`.
4. Write handoff/reports/PROVENANCE-FRAGMENTS.json (+ "jobs"), push, `ops handback-check`, hand back.
   Possible RULING NEEDED: none expected. INFORMATIONAL: glibc-functional-spec.md appendix C is stale against the tables
   (appendix_c.py rewrites ~180 lines); headers_unshipped.tsv rationale above.
