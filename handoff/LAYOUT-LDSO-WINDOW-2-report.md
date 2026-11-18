# LAYOUT-LDSO-WINDOW-2 report

Branch `task/layout-ldso-window-2`, role mei:~/w/s1 (Skylake-SP host, Linux 6.8). Code head 3e2532328; later commits
add a products test (904906254), the record's result and format (4aaed116f) and notes.

## Commits on this branch (after origin/next de4cdfc8d merged in)
* 395a85303 Round ld.so's mapped span to a 512 KiB bucket (`ld-span.ld`, `check_interp_span`).
* 9ce45257f Build the interpreter's crates with one codegen unit in every profile (`LD_RUSTFLAGS`).
* b374a57f5 Refuse an interpreter archive split into codegen units (`check_ld_units`, unit test).
* 3e2532328 Move ld.so's large zero-filled tables after its state words (`LDSO_COLD_BSS` = 4096, `INSERT AFTER .lbss`).
* 904906254 Check that ld.so's large zero-filled objects sit in .lbss (`cold_bss_products`), plants file.
* 4aaed116f Record the measured start-up faults (record body format fixed: static's records check was red on it).

## Measurements (mei, page faults of the start-up probe at N=0/3/12 threads, ASLR off, 21 runs, every run equal)
Probe: a 46 KiB cold function added to ld.so (pad.patch, never committed).

| build | fast faults | fast instr N=0 | release faults | release instr N=0 |
|---|---|---|---|---|
| origin/next | 79/95/131 | 9038k | 76/92/128 | 8194967 |
| origin/next + probe | 83/98/134 | 9039k | (not built: release was already insensitive, earlier relproof) | |
| this branch | 78/93/129 | 9003818 | 74/90/126 | 8194965 |
| this branch + probe | 78/93/129 | 9003818 | 74/90/126 | 8194968 |

* Fast N=0 by mapping: next 17 anon + 3 ld.so rw-p; nextpad 19 + 4; fix and fixpad 16 + 3.
* ASLR-on medians (fast, N=0): next 80/80, nextpad 83, fix 79, fixpad 79.
* layout-bins.tsv (fast) identical between origin/next and the branch; libc.so.6 identical bytes in fast.
* ld.so (fast, branch): `.bss` 0x2028 bytes, `.lbss` 0xa000 (THREAD_MODS, readers::TABLE), `.niji_ldso_span` last.

## Jobs (mei)
* `layout-ldso-window-2-proof3`: fast pad pair, numbers above.
* `layout-ldso-window-2-relproof3`: release pad pair, numbers above.
* `layout-ldso-window-2-gate2`: `cargo xtask affected --run --base origin/next` (escalated itself to FULL: xtask
  changed, so it ran the workspace clippy and `cargo xtask test`): rc 0, 32 suites ok, 12161 passed, 0 failed,
  1397 ignored; conformance 139 passed; 1 SKIPPED line; peak 11G. Head 4aaed116f.
* `layout-ldso-window-2-plants2`: 7 plants (handoff/LAYOUT-LDSO-WINDOW-2-plants.txt): 7 CAUGHT, 0 WARN. Head 4aaed116f.
* `layout-ldso-window-2-fullprof2`: `NIJI_FULL_PROFILE=1 cargo xtask so` + `hot_text_products` (27 passed) +
  `cold_bss_products` (2 passed), rc 0, in a RAM copy of head 4aaed116f's tree (the role was busy with gate2;
  `cargo xtask test` ran inside gate2). Mapped span 0x80000.
* Clippy (`cargo xtask clippy`) and `cargo xtask static` clean on the role at 4aaed116f.
* fast/release DYN_TRUE_BOUND not tightened: its rule takes the highest of the Zen 1 and Skylake-SP host classes,
  and only the Skylake-SP class was measured here.

## Open
* INFORMATIONAL: DYN_TRUE_BOUND ([89, 86]) could be tightened after a measurement on a Zen 1 host.
* keke (Zen 1) half of the proof not run (an earlier refusal; not retried).
