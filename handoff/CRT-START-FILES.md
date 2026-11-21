# CRT-START-FILES (GitHub #27): start files hold only `_start`, the notes and the usual symbols

Branch `task/crt-start-files-3` (continues `task/crt-start-files-2` and `task/crt-start-files`; origin/next e4777a6ba merged in leg 5). Decision:
D-X.144 (records/decisions/D-X/D-X.144.toml, this chain's own record, not landed). Plants: handoff/CRT-START-FILES-plants.txt
(legs 1-2), handoff/CRT-START-FILES-2-plants.txt (legs 3-4). Role keke:~/w/n2 (Zen 1 host). Scratch on keke:
~/work/crt-start-files-2/ (ab.sh static A/B, pf.sh text-phase fault probe, tail.py head-bin tail classifier).

## Legs 1-2 (CRT-START-FILES, done; details in D-X.144)
Start objects: `_start` (endbr64), `.note.gnu.property` (IBT, SHSTK, x86-64 baseline), `.note.ABI-tag`, `__data_start`,
weak `data_start`, `_IO_stdin_used`, nothing aligned above 64. The layout anchors are libc.a's first two members.
crt1.o 67,136 -> 1,816 bytes; dynamic empty main one `R E` LOAD, page-aligned LOADs, IBT/SHSTK kept. Ruling #7085
accepted the dynamic +4,932 user instructions (per-header cost in ld.so: DYN-START-RING's item).

## Leg 3 (done): the CI instruction red
Static `cbench_startup/startup_instr_c` 2191 vs limit 2161: the property note adds PT_NOTE + PT_GNU_PROPERTY and the
static start's header walk cost 14 instructions per unread header. Fix: one bit test per header type
(`READS >> (type & 63) & 1`), whole headers with one bound (`conv::headers_at`, `start::program(heads, phdr)`).
Test `program_reads_only_its_three_types_among_any_others`; plants lines 3-7.

## Leg 4 (this leg): the full-profile hot_text_products red, fixed in the window-bin planner
Red: `init_zero_fault_calls_hold_at_every_text_phase` "pre 0x800: rebind 1 faults" (full profile only).
Cause (probe pf.sh, maps and perf page-fault addresses): a `pre` pad shifts the whole hot group; the head bin is the
one bin not aligned at its start (HEAD_ROOM 1 KiB), so a pad > ~0x3c0 pushes a suffix of bin 0 into a window of its
own and bin 1 realigns past it. Leg 3's start is 108 bytes smaller (5150 -> 5042), so the planner fitted the six
rebind-phase-only sections (same_texts, Mo::hinted, Mo::form, unlink, unlinkat, rmdir; 1,154 B) at the end of bin 0
(64,395 of 64,512 B). At pad 0x800 that suffix held only them: rebind's first touch of the window = 1 fault. At 0x2000
the suffix also held niji_machine_equal (run in the warm-up): 0.
Fix (xtask/src/bins.rs `head_takes`, `put`): a group counting a phase mode (mode without its `.pre` in the signature)
takes a later bin rather than the head bin when both charge it the same modes: a tie-break only, so no mode's window
count can move. Two stricter forms were tried and REJECTED (jobs fullprof2, fullprof3): refusing such groups in the
head bin outright pushed the start-up group out (fullprof2), then strings/locale over their window budgets
(fullprof3: strings 1/0, locale 3/2); the fast plan's float sections have no other zero-cost bin either.
Test `a_measured_phase_only_section_never_ends_the_head_bin`; plants lines 9-12.
Residual (INFORMATIONAL, not fixed): the exact condition is "no phase-only section of bin 0 without a warm-up section
of that phase after it"; tail.py shows the fast plan's two float-phase sections (printf Env::text, conv::printf_text)
violate it, reachable only by a pre pad of most of a window and not covered by the text-phase test (float untested).

## Leg 5 (CRT-START-FILES-3, branch `task/crt-start-files-3`, role mei:~/w/s2): merge-only
CI conflict in xtask/src/bins.rs with next's HEAP-ADDRESS-SPACE-6, which fixed the same rebind fault with a hard
rule (`head_may_take`: a group only measured phases count never joins the head bin). Merged: next's structure
(`phases()`, `put(.., head, ..)` filter) plus this chain's tie-break (`head_takes`, now `put(.., tie, ..)`), which
still matters for groups a measured phase AND other modes count. The leg-4 test's phase-only case is now next's
rule, so the test became `a_section_a_measured_phase_counts_prefers_a_later_bin_on_a_tie` (mixed group); plants
lines 9-12 and D-X.144 follow. `affected` escalates to FULL (csys start sources): gated on precheck plus the
changed crates' tests (test2), `cargo xtask test` (xtest2), full-profile hot_text_products (fullprof2); plants2.
plants1 (merge commit 340b65681): 8/9 CAUGHT, tie-break-over-cost SURVIVED (next's rule took the phase-only case that
caught it); fixed by section `q` in the test (5aba79593). test1/xtest1/fullprof1 stopped for the new head (superseded).

## Results on the leg-4 head (904e4002e)
Static `startup_instr_c` (ab.sh, release/full profile, min of 5 x3, all identical; job fullprof4): with notes 2108
(CI red 2191, limit 2161), notes removed 2096; same 5 LOADs (no read-only PT_LOAD flip, ruling #7061 not involved).
Full profile `hot_text_products` green, bin 0 62,984 B / 103 members (headroom 1,528), windows per mode all within
budget (fullprof4). Plants (plants5): 9/9 CAUGHT, no WARN lines; lint4 green (clippy -D warnings, static).
Ruling #7085's dynamic rows on the Skylake-SP class (#7090; mei, Xeon Platinum 8151; sky.sh = leg 1's startcount.sh
with the final head's libc.so.6/ld.so, old vs new crt1; execcount 101 runs, user / kernel median / faults):
empty main 787,674 -> 792,166 user (+4,492), kernel 860,465 -> 852,395, faults 57 -> 56; 3 libraries 1,042,894 ->
1,035,107 (-7,787), kernel 1,289,172 -> 1,276,266, faults 73 -> 71; clock_gettime 821,675 -> 814,266 (-7,409), kernel
894,339 -> 881,656, faults 61 -> 59. Same shape as the Zen 1 host.
Jobs: keke crt-start-files-2-fullprof4, -plants5, -lint4, -gate4, -xtest4; mei crt-start-files-2-sky0.
Superseded: fullprof1 (red, 7e6a6d3de), fullprof2/fullprof3 (rejected stricter rules), plants3 (refused pattern),
plants4 (killed, older rule), bins0, pf0, aff0/aff1 (probes).

## Exact next step
None for the story once leg 5's jobs (mei crt-start-files-3-test2, -xtest2, -fullprof2, -plants2) are green: land. Follow-ups (not this story): the head-bin tail condition in
general (Residual above; a planner that orders the head bin's phase-only sections ahead of a warm-up section, or a
HEAD_ROOM covering the text-phase pads); the per-header dynamic start cost is DYN-START-RING's (#7062).
