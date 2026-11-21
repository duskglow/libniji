# WL-NIGHTLY-1121: ls-l-enUS DIFF in the nightly kit table; two static page-fault rows over 1.10x

State: branch `task/wl-nightly-1121`, code commit 15baf51b7 (on origin/next 7e5d8b3fa), plus this note and the report json.
Decisions in force: D-4.42 (strcoll = strcmp of strxfrm keys, POSIX wins; COMPAT-2 restores the reference order),
D-X.141 (the kit sweep's INVIOLABLE verdict). No new decision row: a tooling fix with its own self-test.

## 1. ls-l-enUS output: an accepted difference that the kit table did not recognise

Probes on keke (Zen 1) at 0122a0d8a and mei (Skylake-SP) at b2485a7f1. Structure only, no file contents:

| check | result |
|---|---|
| structure, keke, one `ls -lR tree` in en_US.UTF-8 | 14,618 lines in all three columns; same multiset of lines; **one adjacent pair swapped** (`z3.h` / `z3++.h` in include/); every field (perms, size, date, name) is equal; C.UTF-8 output is identical in all three |
| time (date format by file age) | ruled out: no date field differs; order is the only difference |
| static vs dynamic | identical output (md5 51d1c5455ffb), so the link mode doesn't matter |
| host: locale data | `/usr/lib/locale/locale-archive` byte-identical on both hosts (md5 0ccb8c0d77af) |
| host: input tree | `~/wl/work/tree.tar` differs per host (keke 30afe5ba72d1, mei 006fe7b231e5); mei's tree has **no** `z3*` names |
| regression | none: on mei, b2485a7f1's static and dynamic sort/ls already order `z3++.h z3+.h z3+h z3-.h z3.h z3_.h`, glibc `z3+h z3.h z3++.h z3+.h z3-.h z3_.h`, the same as 0122a0d8a on keke. No bisect needed |
| fingerprint (wl_run.sh's count:sha1 of the differing lines, 10 iterations) | static and dynamic both `20:4c9756b03421` = exactly the `validation/wl_known_diffs.tsv` row (D-4.42) |

Cause: the accepted D-4.42 difference (strcoll at a punctuation/digit tie, `docs/differences-from-glibc.md` row
"strcoll on digit and punctuation runs") shows up on keke because keke's tree has names that hit it. mei's tree does not.
`wl_kits.py` compared md5s only and never read `wl_known_diffs.tsv` (only the gate's `wl_report.py` did). So the nightly
table showed an accepted difference as `DIFF(s!,d!)`.
Smallest probe: `printf 'z3.h\nz3++.h\n' | LC_ALL=en_US.UTF-8 <kit>/sort` (glibc: z3.h first; libniji: z3++.h first).

Fix (15baf51b7): `wl_kits.sh` records the fingerprint for a differing column (`dfp` raw row, the same computation
as wl_run.sh). `wl_kits.py` marks each libniji column `=`, `k` (fingerprint allowed for that workload) or `!`, prints
`KNOWN(sk,dk)` or `DIFF(...)`, and adds two summary lines (unexplained / known workloads). New `validation/wl_kits_test.sh`
(7 checks) is selected by affected.rs for wl_kits.py, wl_known_diffs.tsv and itself. wl_kits.sh stays in the
uncovered table (its shell part needs built kits and perf).

INFORMATIONAL: the two hosts' trees differ, so a kit table is only comparable across hosts for workloads whose input
is the same. If cross-host comparison matters, give both hosts the same tree.tar (a kit-input story, not done here).

## 2. Static page faults: heap first-touch in the work, not start-up and not the read-only layout

Kit command form under `perf record -e page-faults -d`, counting only the measured program, two rounds (equal ±5):

| region (faults) | ls-l-enUS glibc | ls-l-enUS static | delta | sed-s glibc | sed-s static | delta |
|---|---|---|---|---|---|---|
| total of the program | 3003 / 3007 | 3484 / 3488 | **+481** (the table's 3613−3132) | 83 | 105 / 118 | +22…35 |
| heap (anon + [heap] + brk) | 2320 | 2746 / 2751 | **+428** | 27 | 59 / 62 | +32…35 |
| text (r-x: libc+ld+exe vs one image) | ~270 | 350 | +80 | 23 | 27 | +4 |
| read-only data (r--) | ~100 | 70 | **−30** | 10 | 6 | −4 |
| writable data (rw-) | 150 | 100 | −50 | 14 | 8–13 | −3 |
| locale-archive + locale.alias | 80 | 110 | +30 | 0 | 0 | 0 |
| niji-nscache mapping | 0 | 30 | +30 | 0 | 0 | 0 |

- Per start, libniji is cheaper (`--version`: ls 73 vs 76, sed 49 vs 63 in the kit form). The extra faults are in the work.
- **Not the read-only PT_LOAD layout** (STATIC-LAYOUT-FLIP / ruling #7061): static r-- faults are below glibc's.
- ls-l-enUS: +43 heap pages per `ls -lR` (+18% over glibc's 232). The libniji code first touching them (per run): ls's
  gobble_file 148, memmove/move_avx2 52 (copies into fresh heap pages), Heap::refill 12, fill (memset) 11, sort_files 8,
  large_slot 4, resize_slow 4. libniji makes 6 mremap calls per run (glibc 3). The en_US-only part is +24 per run
  (C.UTF-8: +25 per run, ls-lR row 1.074): locale-archive +2, locale.alias +1, the rest heap.
- sed-s: libniji's sed heap is stable (59–62 pages) in both launch forms. glibc's sed varies with the launch context (27
  heap pages under `sh -c "env ..."`, 68–73 run directly; direct runs have libniji below glibc: 105 vs 136). So the 1.104 row
  is mainly glibc's context-dependent heap count, not a libniji change. libniji's heap pages: Heap::refill 13, memset 12,
  dfa build_state 4.
- Size of a fix: heap footprint (size-class rounding and per-class pages), which is the heap stories' area (owner
  thread on CLASSES 11-21). Not a few lines, so not done here. Next step for a heap story: count distinct heap pages per
  size class for `ls -lR tree` (the allocator's own statistics), against the +43 pages/run target.

## Results

Gate (direct tier, keke n4, `wl-nightly-1121-gate`): 5 suites ok, 565 passed, 0 failed, peak 1.5G. Plants
(`wl-nightly-1121-plants`, handoff/WL-NIGHTLY-1121-plants.txt, 4 plants): see the hand-back. clippy and static clean
(`wl-nightly-1121-lint`). No numbers or bounds changed.
Refused: one accidental heredoc (a `python3 - <<'X'` no-op) was blocked by the no-heredoc hook and not retried.
Exact next step: none for this story. The heap page-footprint attribution above goes to a heap story if wanted.
