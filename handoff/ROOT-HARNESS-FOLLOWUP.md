# ROOT-HARNESS-FOLLOWUP: contain the root and namespace launches outside the tests (report)

State: branch `task/root-harness-followup` (from origin/next 5f07429d3); head = the commit adding this note.
Role: mei `~/w/s1` (Skylake-SP class). Brief: `~/state/briefs/ROOT-HARNESS-FOLLOWUP.md`. Legs: 3.
Predecessor: `handoff/ROOT-HARNESS-CONTAIN.md`. Doc: `docs/testing/harness-containment.md` ("Outside the tests").

## Done (all brief steps)
- Bench root route (`cbench_netdb`, `cbench_nscache`): `bench::bench_output` runs it through `contain::output`
  (xtask/src/contain.rs: `user`, `root`, `output`; the tie is a socket pair). `past_sudo` passes `CBENCH_*` past
  sudo's environment reset (before this, `--rows` and `--counters` never reached the bench on this route).
  `NIJI_CONTAINED=none` = uncontained opt-out (as tools/contained.sh).
- `cbench_netdb` gained the counted mode (it never included cbench_counters.h, so `--compare --counters` printed
  `-` for it on any route). cbench_nscache: libniji-only header, no host build, no counted mode (not counted).
- validation/profile*.sh, tools/deb, tools/probe, oracle/specman_dns.sh, validation/tmpfs-run.sh: the
  `tools/contained.sh` prologue (kinds root/rootopt/user; `rootopt` without sudo is the one uncontained case,
  reason in the doc). xtask transcript and bench `namespace_command`: `contain::user`.
- Static check (xtask/src/harness_contain.rs) covers validation/, tools/, by-hand oracle scripts, xtask sources.
  Leg-3 fix: a script counted as run by a test only when a Rust test source names it (specman_probe.c's comment
  hid specman_dns.sh from the rule; plant static-oracle-prologue survived in plants-2, caught in plants-3).
- Kill tests: xtask contain::tests (user, root), crates/core/tests/harness_containment.rs (root script, user
  script), bench tests (route contained, variables carried).

## Results (final head)
- mei lint-4 (clippy -D warnings + static): rc 0. test-2 (`cargo test -p xtask` 614 passed incl. transcripts;
  `cargo test -p niji --test harness_containment`): rc 0. Gate: `cargo xtask affected` escalates (xtask's
  modules) -> FULL: the train's; precheck below.
- plants-3: 13 plants, 13 CAUGHT, 0 WARN (handoff/ROOT-HARNESS-FOLLOWUP-plants.txt).
- Count check: table in the doc. 5 runs each way, every contained range overlaps the uncontained one except glibc
  getservbyname_r_http (0.2 % lower); medians within 0.09 % (libniji) and 0.26 % (glibc). Syscall-free rows
  +0.00..0.05 % (libniji), +0.00..0.01 % (glibc), attributed to the
  kernel side (instructions event counts kernel; NOT VERIFIED: user-only event is multiplexed, unusable).
  Data: mei `~/work/root-harness-followup/count3/` (netdb-{uncontained,contained}-N.tsv, measure-*.tsv).
  count3 runs 1-3 (+uncontained-4 killed mid-run), count4 runs 4-5; same products (head differs only in xtask's
  static check). Some runs shared the host with plants/test builds.

## Open
- RULING NEEDED: accept the count check (kernel-inclusive instr within run-to-run range; recommend accept).
- INFORMATIONAL: the namespaced getent and locale transcripts were not run in this run (ldconfig's was, via the
  xtask tests); cbench_nscache budgets uncontained vs contained not run (wall time).
- Next step if wanted: a pinned user-only instructions event in cbench_counters.h would make such checks exact.
