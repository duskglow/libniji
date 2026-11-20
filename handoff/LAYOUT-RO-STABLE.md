# LAYOUT-RO-STABLE: one read-only segment whatever the text size

**State:** branch `task/layout-ro-stable` from next b732d9b46. Commits: 585fdd41d (code: libc.so.6 link fragment, build
check, two unit tests), b99775ef7 (record D-X.143, plants), then this note and the report. Decision in force: **D-X.143**
(amends D-X.133). Build role keke `~/w/n4` (Zen 1 host); attribution and counts also on mei (Skylake-SP). Scripts:
keke and mei `~/work/LAYOUT-RO-STABLE/` (`exp1.sh`..`exp4.sh` static layouts, `count.sh` = DYN-START-BISECT-1120's,
`pt.sh`/`ptjob.sh`/`dec.sh` intel_pt attribution, `kcount.sh` kernel counts, `runenv.c`, `chain.sh` final jobs).

## Result

* **libc.so.6: done.** Both libc links add `RODATA1_FIRST` (`INSERT BEFORE .rodata`): `.rodata1` (start-up group,
  64 KiB aligned) starts the read-only segment, `.rodata` follows without a gap: one read-only PT_LOAD at every text
  size. `cargo xtask so` refuses anything else (`check_read_only_segment`). Unit tests:
  `libc_so_read_only_data_is_one_segment_led_by_the_startup_group` (the check) and
  `startup_group_first_keeps_one_read_only_segment_at_every_text_size` (a libc-shaped library at 16 text sizes, every
  4 KiB phase of a 64 KiB window: one segment with the fragment, two at some sizes without).
* **Static programs: not changed; RULING NEEDED** (below). libc.a, niji-crt1.o and ld.so are byte-identical to the
  base build, so no static row moves.

## Segment tables (readelf -lW, release)

| libc.so.6 | read-only PT_LOADs after the text |
|---|---|
| snapshot 50abc44ed | 1 (0x30d000, `.rodata` ended 2 KiB below the boundary) |
| next b732d9b46 | 2 (0x30e000 `.rodata`, 0x3b0000 `.rodata1`) |
| b732d9b46 + this change | 1 (0x310000, led by `.rodata1`) |
| unit test, 16 text pads | with the fragment 16 of 16 one segment; without, some split (asserted) |

Static `cbench_startup` (exp1, text pads 0..60 KiB): today 7 of 8 pads give two read-only PT_LOADs.

## Dynamic start-up (execcount: user instructions, median, ASLR off; faults)

| row | host | snapshot 20.1 | next b732d9b46 | this change | change |
|---|---|---|---|---|---|
| empty main | Zen 1 | 781,710 / 59 | 787,723 / 57 | 783,943 / 57 | -3,780 |
| 3 libraries | Zen 1 | 1,033,070 / 75 | 1,042,973 / 73 | 1,039,193 / 73 | -3,780 |
| clock_gettime | Zen 1 | 815,680 / 64 | 821,755 / 62 | 817,974 / 62 | -3,781 |
| empty main | Skylake-SP | 781,785 / 59 | 787,802 / 57 | 784,022 / 57 | -3,780 |
| 3 libraries | Skylake-SP | 1,033,114 / 75 | 1,043,022 / 73 | 1,039,243 / 73 | -3,779 |
| clock_gettime | Skylake-SP | 815,725 / 63 | 821,805 / 61 | 818,025 / 61 | -3,780 |

Kernel instructions (kcount.sh, through runenv, 101 interleaved rounds, min / median, base -> change): Zen 1
-1.3k / -9.0k, -14.2k / -25.1k, -17.5k / -43.6k; Skylake-SP -6.9k / -6.6k, -12.9k / -11.3k, -12.0k / -12.7k (busy
hosts: noisy, every one lower). No row is worse than next. Against snapshot 20.1 the rows are still +2.2k / +6.1k /
+2.3k user: that is other code since 20.1 (ld.so differs), not this layout.

**Attribution (exact, `perf record -e intel_pt//u`, Skylake-SP, pt2 + dec.sh):** the whole -3,780 is ld.so's
per-segment work for the one program header less: `Elf::read_phdr` -2,583, `link` -266, `Elf::check_table` -235,
`protect_relro` -161, `Loads::of` -92, the rest under 85 each.

**Step 2, the bisect's +2,638:** the same trace on its product sets (pt1: 20.2 = 3f2a84520 vs its fix 5dc6d6b5d) puts
all of it in ld.so, and the two ld.so binaries differ (other sha256; 320 vs 339 text symbols: `check_versions`,
`map_segment` out of line in one only). The fix set's counts equal this change's exactly (784,022 / 1,039,242 / 818,025
on Skylake-SP), so its loader matched today's next: the +2,638 is loader code between 20.2 and that base, not the
placement. Paired on one base the placement gains on every row.

## RULING NEEDED: static programs

GNU ld's default script (what a user's static link uses) places `.rodata1` after `.rodata`; the only object a link
always puts first is the start file, and none is reliably last except crtn.o. Measured levers (cbench_startup,
`true` child, 201 interleaved rounds; kernel instructions median; faults equal in all):

| layout | Zen 1 | Skylake-SP | read-only segments | cost / risk |
|---|---|---|---|---|
| today, split (15 of 16 data phases, 7 of 8 text pads) | 173.1-173.4k | 165.9k | 2 | jitter |
| (a) crtn.o holds an empty `.rodata` aligned like the group (crtn is last in every driver link) | 168.1k | 160.3-160.4k | 1 always | every driver link (dynamic programs and shared libraries too) gets `.rodata` 64 KiB aligned at both ends: up to ~128 KiB more file per binary; -nostdlib links (benches, tests) need crti/crtn |
| (b) a static-only libc.a member with an empty 64 KiB `.rodata` + `.rodata1` page-aligned | - | - | 1 always | the group spans 2 windows in 8 of 16 data phases: +1 fault (setlocale, printf; exp4) |
| (c) the group in `.lrodata` (own segment after .bss) | 168.7-169.2k | 160.9-161.3k | 2 always | rejected: after `.lbss`, a medium-model program with > 2 GiB `.lbss` overflows libc's PC32 references |
| (d) leave as is | | | 1 or 2 | the jitter stays |

Recommend (a) only if the file-size cost is acceptable for a distribution; otherwise (d) for static and the layout
gate keeps treating a static split as known jitter (#4347). (b) trades the segment jitter for a fault jitter.

## INFORMATIONAL

* `Elf::read_phdr` costs ld.so about 2.6k user instructions per program header (60,639 of 784,032 on empty main,
  117,342 of 1,039,236 with 3 libraries, Skylake-SP trace): a loader story could look at its per-header cost.
* libc.so.6's text is also two R E PT_LOADs (`.init`/`.plt` at 0x2c000, the 64 KiB-aligned text at 0x30000); the
  headers would have to grow ~12 KiB before that count changes.

## Jobs

Final head (keke, tree b99775ef7; later commits are this note and the report): layout-ro-stable-lint (records check,
clippy, static, the 3 so.rs unit tests: ok), -gate (direct tier, 6 suites, 592 passed, 0 failed, peak 1.8G), -xtest
(`cargo xtask test`: 6922 passed, 0 failed, peak 12G), -fullhot (release `cargo xtask so` + hot_text_products: 31
passed), -plants (4 CAUGHT, 0 SURVIVED; one WARN-DEV-SHM notice: the RAM dir is under /dev/shm as the brief asks).
Counts ran on products of 7691b4d99 (same link code); the final head's release libc.so.6 has the same sha256
(fe8d7622ed0f5a67...), and ld.so, libc.a and niji-crt1.o equal the base's, so the counts are the final head's:
keke layout-ro-stable-count2, layout-ro-stable-kcount-k; mei layout-ro-stable-pt2, layout-ro-stable-kcount-m.
Evidence only: pt1 (bisect sets), exp1-exp4, base-build, lb-build; count1 failed on a path (`~` after `TAG:`).

**Next step:** the owner's ruling on static programs; if (a), a story adds the crtn anchor and links benches and
the products tests with crti/crtn, as the driver does.
