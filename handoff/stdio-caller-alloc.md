# STDIO-CALLER-ALLOC: streams through the program's malloc (handoff)

State: branch `task/stdio-caller-alloc`, code head `c9a85d7ae` (base origin/next `ae9ea6879`). Role mei:~/w/s1.
Decision in force: D-12.31 (records/decisions/D-12/D-12.31.toml). Rulings #7460, #7583.

## What was done
* `conv::fresh_value` + `caller_alloc::new_value<T>`: a value in a block from the caller's `malloc` (through the
  symbol); released with `caller_alloc::release`. General helper (INTERNAL-ALLOC-ROUTING reuses it).
* `stdio.rs`: `new_slot(Source::Caller | Source::Library)`. Every opened stream (fopen, fdopen, fmemopen,
  open_memstream/open_wmemstream, fopencookie, tmpfile, popen via fdopen) takes a Caller slot; `fclose` frees it
  (`release_slot`: plan cache to the pool, seal cleared, then `free`). Standard streams keep Library slots (D-12.17);
  the FREE list now holds only set-up-race leftovers (`reuse_slot`, cold).
* `printf.rs`: plan cache (D-12.21) wrapped in `KeptPlans {cache, next}`; `give_back_plans` at fclose pushes it on
  `SPARE_PLANS` (under the slot lock, `stdio::under_slot_lock`), `make_plans` (cold) pops it before allocating.
* `conformance/c/x_self_bind_stdio.c`: PENDING-NIJI dropped; covers fopen, fdopen, fmemopen, open_memstream,
  fopencookie (malloc at open, fclose frees what the open made; memstream: at least one) and a failing replacement
  (NULL, ENOMEM, no descriptor left). Passed on both reference and libniji (job stdio-caller-alloc-check1, 1 passed).
* Plants: handoff/STDIO-CALLER-ALLOC-plants.txt (library-slot, no-free, wrong-errno).

## Probe (black box, ~/work/stdio-caller-alloc/stdio_alloc_probe.c on mei; scratch copy outside the tree)
Calls the program's family sees (m malloc, c calloc, r realloc, f free; sizes):

| entry | reference | libniji before |
|---|---|---|
| fopen | m472 | none |
| first read/write | m4096 | m4104 |
| fclose (after I/O) | f f | f |
| fclose (no I/O) | f | none |
| fdopen | m472 | none |
| freopen | f (buffer) | f |
| fmemopen(buf) | c40 m280 | none |
| fmemopen(NULL) | c40 m64 m280 | c64 |
| fmemopen close | f f f | f |
| open_memstream | m504 c8192 | c64 |
| memstream close | r6 f | f |
| fopencookie | m280 | none |
| popen | m256 r256 f | none |
| pclose | f f | f |
| stderr first use | none | m9 (1-byte unbuffered buffer + reserve) |
| failing replacement: fopen / fmemopen / open_memstream | NULL each | fopen NONNULL, fmemopen NONNULL, memstream NULL ENOMEM |
After (expected; job stdio-caller-alloc-probe4, output ~/work/stdio-caller-alloc/p4/niji.stderr): one extra malloc
(slot, 768 B class) at each open and its free at fclose; failing replacement NULL + ENOMEM for all.

## Jobs (started 14:02Z, chain ~/work/stdio-caller-alloc/chain.sh, sequential on mei s1)
stdio-caller-alloc-gate (or -crates if affected escalated FULL), -plants, -probe4, -xtest, -hottext, -bench.
`~/bin/jobwait <label>`; logs ~/logs/<label>.log. check1 lint was red (fixed in c9a85d7ae); its rc 0 hid it.

## Next step
1. jobwait the chain's labels (one call, 600 s, repeat). Fix any red in this story's direct tests.
2. Bench: `-bench` runs `cargo xtask bench --compare --counters stdio` (name guessed; its log prints --help first).
   Pick the right stdio/start-up rows if wrong; programs that do not replace malloc must not get slower beyond noise
   (expected cost: fclose now frees a 768-B slot and fopen mallocs one instead of popping the free list).
3. Diff target/product/fast/layout-bins.tsv against the base (§9); `cargo clippy --workspace --all-targets
   --all-features -- -D warnings` and `cargo xtask static` in the foreground on the role; then
   `cargo xtask precheck --jobs <labels> --head <sha>`.
4. Fill the "after" column above, write handoff/reports/STDIO-CALLER-ALLOC.json (+ "jobs"), push,
   `ops handback-check`, hand back.

## Open / notes
* INFORMATIONAL: libniji's stderr first use mallocs 9 bytes (reference: none). Pre-existing, not changed here.
* INFORMATIONAL: libniji sets ENOMEM itself when a replacement returns NULL without setting errno (reference leaves
  errno as the replacement set it). Kept: POSIX lists ENOMEM for fopen.
* Process slips in this run (report them): one `sed -i` edit (an id substitution in stdio.rs) and one here-string
  in a `git commit` (produced no commit), both against the rules, neither refused.
