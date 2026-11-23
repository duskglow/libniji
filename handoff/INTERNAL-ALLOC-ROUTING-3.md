# INTERNAL-ALLOC-ROUTING-3 (handoff; end of leg 2)

State: branch `task/internal-alloc-routing-3` (base origin/next 279a24ee4, not merged). Code head 01b3bdc9a = option 1 +
the static probe GOT fix; 5c061eff8 adds handoff/INTERNAL-ALLOC-ROUTING-3-plants.txt. Option 2 (DO NOT LAND) is on
`task/internal-alloc-routing-3-opt2` ab2286af6. Role keke:~/w/n1. printf is PRINTF-NO-HEAP's: leave it alone.
Rulings in force: #7583, #7707, #7708 (two process-flag heap words), #7724 (land option 1 only if 0 added per
allocation and release and start-up no worse than run 2's (a)), #7725.

## Leg 2 findings (measured)
* PROFILE TRAP: base's dump (~/work/internal-alloc-routing-2/dis-base.txt) and (a)'s are RELEASE products (fat LTO;
  `target/niji/.profile` = release, written by `bench --compare`, which builds Release). A plain `cargo xtask so` is
  the fast profile (thin LTO, 16 units): GOT-relaxed `lea`+`cmpq`, `addr32 call`, and the alignment test come back
  there. dis-o1b.txt (fast) is NOT comparable; dis-o1-rel.txt (release, n1 after the o1bench job) is.
* Hit paths, release, small class (script ~/w/.scratch/INTERNAL-ALLOC-ROUTING-3/hitpath.py <dump> <fn> <taken addrs>;
  base and (a) add heap_cache_word's owned path, cmpq/je/mov/ret = 4):
  | | base | run 2 (a) | option 1 | option 2 |
  | per allocation (rust_alloc) | 54+4 = 58 | 56+4 = 60 | 42 (frameless, align test folded, gate `cmpq $0,X(%rip); je`) | PENDING |
  | per release (rust_dealloc) | 80+4 = 84 | 72+4 = 76 | 72+4 = 76 (byte-identical to (a)) | PENDING |
* cbench_startup user instructions (job internal-alloc-routing-3-o1bench, release, head 01b3bdc9a):
  | row | base | (a) | option 1 |
  | startup_instr_c | 2105 | 2105 | 2108 (+3) |
  | startup_instr_setloc | 62056 | 62089 | 62029 |
  | startup_instr_wctype | 62547 | 62567 | 62507 |
  | startup_instr_printf | 65627 | 65631 | 65596 |
  The +3 is open_own_fast in init_initial (inlined into runtime::start): `cmpq $-1,RESERVE_LAST; je; movq $0,OWN_FAST_OFF`.
* Static probe: so2 job green (`init_static_probe_has_no_got_at_any_relro_size`: `.got inputs []`).
* Zero at start-up is not reachable with write-once words: decide_route (early_init) runs before init_initial, so the
  open depends on data (>= 1 instruction). 0 would need a ruling (e.g. a routed bit in the malloc DEBUG word: the C
  entries test `flags != 0`, so libniji's own malloc would take its general path in routed processes, e.g. under
  forwarding wrappers). `OWNED` is the asm symbol niji_machine_owned: no pairing there.

## DECIDED, NOT YET EDITED: option 1b (one word, +1 at start-up)
Merge OWN_FAST_OFF into RESERVE_LAST (heap.rs lines 469-547 block, rust_alloc/rust_alloc_zeroed gates, routed()):
* `RESERVE_LAST` starts at `RESERVE` (closed; the release test then also names base+RESERVE, never a heap block);
  `route_internal` stores `usize::MAX`; `open_own_fast` (init_initial) = `RESERVE_LAST.fetch_sub(1, Relaxed)` (one
  instruction; safe code) -> `RESERVE - 1` (open) or `MAX - 1` (still routed). init_initial runs once per process.
* gate in rust_alloc / rust_alloc_zeroed: `reserve_last() == RESERVE - 1` (same 2 instructions: cmpq $imm,X; jne).
* `routed()` = `reserve_last() > RESERVE`. Remove OWN_FAST_OFF: startup-state.list line 50, xtask/src/policy.rs:105,
  docs/architecture.md:197 (process-flag words), the comments; thread.rs:239 keeps calling open_own_fast.
* Plants file: replace own-fast-open-when-routed with `open_own_fast` storing RESERVE - 1 unconditionally (msg as the
  others); the other five stay (anchors `if align <= ALIGN && routed() {`, `RESERVE_LAST.store(usize::MAX, Relaxed);`
  unchanged). Check every lit anchor still matches exactly once.

## Exact next steps
1. Edit as above; commit; push; sync (sync_role.sh ... keke n1; check rc).
2. Jobs (all detached, scripts in ~/work/internal-alloc-routing-3/, installed with `"${VSSH[@]}" 'cat > path' < local`;
   NEVER chain anything before `setsid nohup` over ssh): o1bench.sh (release bench on n1; after it, `sh
   ~/work/internal-alloc-routing-2/disasm.sh ~/w/n1` for the 1b hit paths and grep init_initial's
   RESERVE_LAST lines), then gate.sh (kind gate, under ~/bin/gate 16G; clippy, static, affected, refuses FULL),
   plants (new RAM dir /dev/shm/claude/internal-alloc-routing-3/plants-2, `--timeout 900`), then startup.sh
   (`cargo xtask test`, NIJI_FULL_PROFILE so + hot_text_products). One product build per role: bench, gate, startup in
   sequence on n1; plants in RAM can run beside.
3. Collect internal-alloc-routing-3-o2bench3 (option 2, release bench then dis-o2-rel.txt; started 00:42; the fast
   dump dis-o2.txt is not comparable). Option 2's fast `cargo xtask so` stopped on layout WINDOW-BIN-BYTES (.lbss).
4. Precheck, report json (counts table first), handback-check. Expected verdict: c row +1 vs (a) -> RULING NEEDED
   (accept +1 at start-up for -16 per allocation, -8 per release, -27..-60 on the other start rows), so not
   COMPLETE unless the ruling says so.

## Superseded jobs (keke)
internal-alloc-routing-3-so1 (first shape), -so2 (fast profile; its GOT test result stands for the C change),
-opt2bench (fast so failed layout), -o2bench2 (--no-build: no bench programs), -plants (head 01b3bdc9a, killed for 1b).
-o1bench is option 1's start-up row (kept for the table, not for the final head).

## Notes
* Leg 1 slip: one `sed -i` on the opt2 scratch worktree (not refused). Leg 2: one ssh call chained `test -s &&` before
  `setsid nohup` and hung until the local 10-minute limit (job o2bench3 started correctly under jobrun).
* Scratch: ~/w/.scratch/INTERNAL-ALLOC-ROUTING-3/ (opt2 worktree, hitpath.py, dumps); remove the opt2 worktree at the
  end (`git worktree remove`).
