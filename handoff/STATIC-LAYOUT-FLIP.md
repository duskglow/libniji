# STATIC-LAYOUT-FLIP: judge static start-up rows at a matching read-only layout (ruling #7061)

**State:** branch `task/static-layout-flip` from next 7ac731794. Code head 8cb409fd9 (startpair.rs, main.rs, affected.rs
tool registry), then this note and the report. Decision in force: **D-X.148** (reserved; cites #7061, D-X.143). Role
mei `~/w/s3` (Skylake-SP). Scripts: mei `~/work/static-layout-flip/` (`proof.sh`, `proof2.sh`, `sweep.sh`); trees
`/mnt/zram/claude/static-layout-flip/proof-{1,2}/{base,head}` (next b732d9b46, HEAP-ADDRESS-SPACE-9 head f9eb8bd63;
proof-1 fast products, proof-2 release `NIJI_FULL_PROFILE=1`).

## What landed

`cargo xtask startup-pair --base <tree> [--head <tree>] [--runs N] [--rows a,b] [--head-pad BYTES]`
(`xtask/src/startpair.rs`): links `cbench_startup` of both trees with `bench`'s libniji command line, counts read-only
PT_LOADs after the text; on a mismatch relinks both with the smallest never-run `.text` pad (k pages, k = 1..16, bench
programs only) that makes the counts agree, prints `LAYOUT-FLIP base N head M: judged at text pad P`, reports the
unpadded pair, judges the padded one (exit 1 on WORSE). Verdict: user rows WORSE when the head median is above the base
max; `startup_kinstr_c` when above the base max by more than 1% of the base median. `--head-pad` is the self-check
(head = base plus a fixed text pad: a change that only grows the text). No product change.

## Proof (mei, Skylake-SP; counts = instructions per start, min / median / max over rounds)

| run | RO PT_LOADs base / head | pair | startup_instr_c | startup_kinstr_c | verdict |
|---|---|---|---|---|---|
| self-check, base tree vs itself, `--head-pad 32768` (fast) | 2 / 1: LAYOUT-FLIP, pad 0x1000 (both 2) | unpadded (reported) | 2193 vs 2186 (-7) | 143,276 vs 137,740 (-5,536) | - |
| same | | padded 0x1000 (judged) | 2193 vs 2193 (+0) | 143,409 vs 142,982 (-427) | ok |
| self-check, head-pad 4..28 KiB | 2 / 2 | unpadded | +0 | -611..+1,703 | ok |
| HEAP-ADDRESS-SPACE-9 case, release, 7 rounds | 2 / 2 (no flip) | unpadded | 2169 vs 2169 | 145,102 vs 143,184 | ok |
| HEAP-ADDRESS-SPACE-9 case, fast, 7 rounds | 2 / 2 (no flip) | unpadded | 2193 vs 2203 (+10) | 143,256 vs 158,683 (+15,427) | WORSE |

The HAS-9 pair does not flip on this host and link (both have two read-only segments). Its +7.5k was measured with
its own scripts on a Zen 1 host (not read); the flip path is shown by the self-check. In the fast profile the HAS-9
head costs +10 user / +15.4k kernel instructions per static start at an equal layout (all four user rows WORSE): a
real cost, not layout (INFORMATIONAL for HEAP-ADDRESS-SPACE-10, which carries that work; release: +0..+6 user, kernel
within noise).

Plants (`handoff/STATIC-LAYOUT-FLIP-plants.txt`, `cargo xtask plants`): 6 of 6 CAUGHT, no WARN. Gate (direct tier,
bounded xtask set): 5 suites, 565 passed, 0 failed, peak 1.8G. Lint: clippy `--workspace --all-targets
--all-features -D warnings`, `cargo xtask static`, `records check`: ok.

## Open / for the coordinator

* The brief names the per-train instruction-limit check: it compares one tree against `bench/instr_limits.tsv`, with
  no base, so there is nothing to pair; `startup-pair` is the base-vs-head compare (recorded in D-X.148 "Not
  covered"). A layout flip moved `startup_instr_c` by 7 user instructions here; the limit has 4 of headroom on the
  Zen 1 row (2155 vs 2151), so a flip alone can redden the train's check. RULING NEEDED (recommend: the train's check
  on a red `cbench_startup` row runs `startup-pair --base <merge base>` and judges on its verdict).
* Not done: wiring `startup-pair` into the train or `wl_kits.sh` (ops side, not in this tree).

**Next step:** land; then the coordinator decides on the train wiring above. Plan rows: D-X.148, D-X.143.
