# STARTUP-GOT-PAGE: `.got` starts on a page (D-X.122, ruling #6606 option C): CLOSED BY MEASUREMENT (run 7)

State: branch `task/startup-got-page-7` (continues `task/startup-got-page-6` ... `task/startup-got-page`), origin/next
50abc44ed merged (merge 1e4c20c98). Role keke:~/w/n1 (Zen 1 host). Decision `records/decisions/D-X/D-X.122.toml`
(amends D-X.101). Verdict run 7: DO NOT LAND. The change costs dynamic programs page faults; nothing in it is a gain.

## What the branch does (option C, unchanged since run 4)
`csys/src/machine/entry.S` (program start files only: `niji-crt1.o`, `niji-rcrt1.o`; not libc.so.6, ld.so or libc.a):
an empty `.got` input section `.balign 4096` after the start file's `.data.rel.ro.local`, so GNU ld starts the output
`.got` on a page. `crates/core/tests/hot_text_products.rs` `init_startup_data_pages_hold_at_every_data_phase` checks
it at relro pads 0x10-0x1000.

## Run 7: paired count (coordinator decision 11-20 06:17Z, step 1 before any code)
Method: release products (`NIJI_FULL_PROFILE=1 cargo xtask so`) of the merged head built once on keke; two start files
assembled with the build's final cluster defines (`NIJI_RODATA1_ALIGN=65536 NIJI_DATA1_ALIGN=16384
NIJI_RELRO_ALIGN=65536`): head `entry.S` (reproduces the product `niji-crt1.o` byte for byte) and origin/next's `entry.S`
(the head minus this branch's 8 lines). So the two builds of each program differ only in the start file. Measured on
the Skylake-SP host: execcount (user instructions, page faults, exec to exit), ASLR off, median of 101, order next,
head, head, next (all four runs of each pair identical; min = median). Programs from DYN-START-SPEED's probe kit
(empty main, main calling 3 shared libraries, clock_gettime; PIE, linked through `niji-crt1.o`) plus a static empty main.

| program | faults next | faults head | instr next | instr head | file bytes next / head | RELRO memsz next / head |
|---|---|---|---|---|---|---|
| static empty main | 8 | 8 | 2,098 | 2,106 | 16,353,856 / 16,370,240 | 0x7000 / 0x8000 |
| PIE empty main | 62 | 63 | 782,040 | 782,041 | 141,336 / 145,440 | 0x170 / 0x2150 |
| PIE 3 libs | 78 | 80 | 1,033,526 | 1,033,528 | 141,560 / 145,664 | 0x1f8 / 0x21c0 |
| PIE clock_gettime | 66 | 68 | 815,980 | 815,982 | 141,528 / 145,632 | 0x1b8 / 0x2190 |

Why dynamic loses: in a PIE the output `.got` grows from 0x20-0x38 bytes to 0x1008 at 0x23000, so it spans two pages
and the RELRO segment takes one or two more written pages. The static program takes no fewer faults either (its
`.got` already sat in one page on next). Release hot_text_products on the merged head: 30 passed (dynamic bound is
now [73, 67] after DYN-START-SPEED; CI's "87 > 86" red was against the older bound and does not reproduce).

## Exact next step
None: per the coordinator's 06:17Z decision the chain closes; HEAP-ADDRESS-SPACE-8 and LOCALE-SLOTS-GROWTH-6
measure on next as it is. If `.got` placement is ever revisited, measure PIEs first: a page-aligned `.got` input
in the start file splits a PIE's small `.got` across two pages. Option D (run 3 idea) was never measured.

## Not to redo
A/B/C/D measurements (run 3); the paired count above. Do not move a read into a `#[cold]` function (run 2).
Scripts: Skylake-SP host `~/work/startup-got-page-7/pair.sh DIR [RUNS]` (needs DIR/rel products and DIR/crt1-{next,head}.o).
Plan rows: D-X.101, D-X.122.
