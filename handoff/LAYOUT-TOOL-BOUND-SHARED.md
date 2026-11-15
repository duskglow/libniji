# LAYOUT-TOOL-BOUND-SHARED: why three branches read 51 on the coreutils-shaped bound (50)

**State:** investigation, no product code. Branch `task/layout-tool-bound-shared` from origin/next 955c18b87. Handoff
and tools only (`handoff/LAYOUT-TOOL-BOUND-SHARED-tools/`: `setup.sh` + `build.sh` build a RAM clone of next with a
branch merged as CI merges it; `pf.py` counts the tool probe as the test does and attributes every fault with
`perf record -e page-faults -d`; `cmp.py` aligns two fault lists; `gaps.py` lists alignment gaps in a link map).
Measured on mei (Skylake-SP, CI's host class), fast products, RAM clones `/mnt/zram/claude/LAYOUT-TOOL-BOUND-SHARED/<tag>`,
results in `out/` there. No decision record: nothing was changed; the fix is a RULING below.

## Result (next 955c18b87; every run identical; each build repeated where noted, byte-identical `libc.a`)

| tree | tool | start-up tier end | margin to 0x478000 | own start-up change |
|---|---|---|---|---|
| next | **50** | 0x477a2a | 1494 B | - |
| next + regcomp-fallible-alloc-2 8c297b520 (twice) | **50** | 0x4779ea | 1558 B | none (start-up and probe keys = next's; tier -64 B) |
| next + tb-foreign-words-low-10 3c90ec0bf (twice; contains -8) | **50** | 0x477e2a | 470 B | yes: start-up keys differ, tier +1024 B |
| next + setlocale-ctype-env-4 874d17d92 | **51** | 0x4782aa | -682 B | yes: `setlocale` -> `setlocale_c` + `setlocale_env`, `query_index`, `CText::read_at` join the tier, +2176 B |

The hypothesis "next sits at a boundary and growth moves one start-up page across it" holds, with this boundary:
the text group is the start-up tier (`.text.sorted.h00xxx`, ~450 KiB from 0x408000) followed by the window-safe
clusters, each aligned 32 KiB (`so` log: strings 20634 B, time 16810 B, rebind 19182 B, rebind-tail 16588 B, all
align 32768; float 14410 B, 16384), in the order strings (`h900`), time, rebind, rebind-tail, float, then the
probe's own `.text`. When the tier end crosses a 32 KiB multiple, the whole block and `main` move by 32 KiB, which
flips which 64 KiB window each cluster shares. Next's tier ends 1494 B below 0x478000.

**The extra fault (setlocale-ctype-env-4):** text page 0x485000, `__printf_chk` (`.text.sorted.h90000115`, strings
cluster), its first instruction. Tool probe text windows, next: 0x400000 (`_start`), 0x410000 (printf `Env::text`),
0x420000 (`setlocale`), 0x430000 (`finish_grow`), 0x460000 (`textdomain`), **0x470000 (`getenv` 0x476000 and
`__printf_chk` 0x47d000 share it)**, 0x480000 (`bindtextdomain` 0x488000), 0x490000 (`main` 0x49b897): 8. Branch:
the same six, then 0x470000 (`getenv`), **0x480000 (`__printf_chk` 0x485000, alone)**, 0x490000 (`bindtextdomain`),
0x4a0000 (`main`): 9. Every other fault (data, relro, bss, rodata, stack, locale archive) is the same page.

## Per branch (step 2/4)

* **setlocale-ctype-env-4: (b) its own start-up cost**, amplified by the boundary. Its setlocale rework is on the
  tool probe's start-up path and grows the tier by 2176 B, past the 1494 B margin. Fix forward in its story: net
  tier growth under the margin, or wait for the structural fix below. It is not layout-only.
* **tb-foreign-words-low-10 (and -8): (b) its own start-up growth (+1024 B), harmless at today's next** (50, 470 B
  left). CI's 51 (trains 705, 709) came on older nexts, not rebuilt here: consistent with a margin under 1024 B
  then (NOT MEASURED). Re-running CI on today's next should read 50; any later start-up growth on next above 470 B
  turns it red again.
* **regcomp-fallible-alloc-2: (a) no start-up change** (keys = next's, tier 64 B smaller, 50 twice). CI read 51 at
  14:27 (ci-gate-2f0ad6dd4) on a composition with the same changed files plus regenerated views (no build input
  reads a view), not reproduced here (see "CI environment"). Re-running CI on today's next should read 50.

## CI environment

The same regcomp merge built inside `~/bin/gate 16G` (as CI's jobs run) gives a byte-identical `libc.a` and the
same four list hashes as the plain build: the gate environment does not change the self-trace or the layout. CI's
train commits (2f0ad6dd4, d8be23454) are not on origin or on mei, so CI's base next for those runs could not be
rebuilt; the most likely cause is an older base next with a smaller margin (NOT VERIFIED). INFORMATIONAL for the
coordinator: a CI red on this bound should record the base next sha, so it can be rebuilt.

## RULING NEEDED: the structural fix (step 3; not small and clearly correct, so not implemented)

Options (each must be measured on all ten probe modes, both profiles, with a `pre` pad sweep):
* **(A) recommended:** start the cluster block on a 64 KiB boundary and order it so the clusters the coreutils-shaped
  path opens share one window: [strings | rebind] [time | rebind-tail] [float]. The block's windows then no longer
  depend on the tier's size (tier growth only moves the block by whole windows). Expected tool count 50 at next and
  on any branch whose growth stays inside the tier's own windows (estimate, not measured). Cost: up to 64 KiB of
  alignment fill (virtual; file bytes in static links), and the rebind / rebind-tail pair then always spans two
  windows (today it depends on parity). Size S-M in `xtask/src/hottext.rs` (cluster order and `cluster_align`).
* **(B)** LT-2 CLUSTERS-BY-PHASE for the tier itself: the tool path (setlocale, textdomain, getenv, printf) is
  spread over six windows of the 450 KiB tier, so growth inside the tier still moves it. (A) removes this
  incident's boundary; (B) removes the class. Size M.
* **(C)** keep: next holds 1494 B; every landed start-up growth spends it and the next unrelated branch trips it.

## Not to redo / next step

Do not rebuild these trees: builds are deterministic on mei (repeat builds byte-identical). Next step: the RULING;
with (A) chosen, a story that changes the cluster order and alignment and proves it with `pf.py` on next and on
next + setlocale-ctype-env-4.
Jobs (mei): `layout-tool-bound-shared-setup`, `-b-next`, `-b-t10`, `-b-sce4` (rc 101: the bound test fails by
design, 51), `-b-rfa2`, `-b-rfa2b`, `-b-t10b`, `-b-nextg`, `-b-rfa2g`.
