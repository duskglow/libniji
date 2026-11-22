# STDIO-CALLER-ALLOC: streams through the program's malloc (handoff)

State: branch `task/stdio-caller-alloc`, code head `46c53df32` (base origin/next `ae9ea6879`). Role mei:~/w/s1.
Decisions in force: D-12.31 (records/decisions/D-12/D-12.31.toml), D-X.140 (no `.got` in a static program).
Rulings #7460, #7583.

## What was done
* `conv::fresh_value` + `caller_alloc::new_value<T>`: a value in a block from the caller's `malloc` (through the
  symbol); released with `caller_alloc::release`. General helper (INTERNAL-ALLOC-ROUTING reuses it).
* `stdio.rs`: `new_slot(Source::Caller | Source::Library)`. Every opened stream (fopen, fdopen, fmemopen,
  open_memstream/open_wmemstream, fopencookie, tmpfile, popen via fdopen) takes a Caller slot; `fclose` frees it
  (`release_slot`: plan cache to the pool, seal cleared, then `free`). Standard streams keep Library slots (D-12.17);
  the FREE list now holds only set-up-race leftovers (`reuse_slot`, cold).
* `printf.rs`: plan cache (D-12.21) wrapped in `KeptPlans {cache, next}`; `give_back_plans` at fclose pushes it on
  `SPARE_PLANS` (under the slot lock), `make_plans` (cold) pops it before allocating.
* `conformance/c/x_self_bind_stdio.c`: PENDING-NIJI dropped; covers fopen, fdopen, fmemopen, open_memstream,
  fopencookie and a failing replacement (NULL, ENOMEM, no descriptor left).
* `strz.rs` (leg 2): the first gate was red in `init_static_probe_has_no_got_at_any_relro_size`: libc.a kept one plain
  `R_X86_64_GOTPCREL`, a `cmova` of the address of `strz::cursor`'s static `END`, emitted in another crate's
  code-generation unit once this branch moved unit boundaries. cmov is not a relaxable form (D-X.140), so the cursor
  now ends on a flag (no static address). A unit test `strz::tests::cursor_yields_each_byte_then_zero_for_good`
  covers the cursor (the collation differential drives `niji::locale::collate` with its own stream, not this one).
* Plants: handoff/STDIO-CALLER-ALLOC-plants.txt (library-slot, no-free, wrong-errno, cursor-stop): 4 CAUGHT.

## Probe (black box; scratch copy outside the tree; calls the program's family sees: m malloc, c calloc, r realloc, f free)

| entry | reference | libniji before | libniji after |
|---|---|---|---|
| fopen | m472 | none | m712 |
| first read/write | m4096 | m4104 | m4104 |
| fclose (after I/O) | f f | f | f f |
| fclose (no I/O) | f | none | f |
| fdopen | m472 | none | m712 |
| freopen | f (buffer) | f | f |
| fmemopen(buf) | c40 m280 | none | m712 |
| fmemopen(NULL) | c40 m64 m280 | c64 | c64 m712 |
| fmemopen close | f f f | f | f f (buf) / f f f (NULL) |
| open_memstream | m504 c8192 | c64 | c64 m712 |
| memstream close | r6 f | f | f f |
| fopencookie | m280 | none | m712 |
| popen | m256 r256 f | none | m712 |
| pclose | f f | f | f f |
| stderr first use | none | m9 | m9 (unchanged) |
| failing replacement: fopen / fmemopen / open_memstream | NULL each | fopen NONNULL, fmemopen NONNULL, memstream NULL ENOMEM | NULL + ENOMEM each |

Sizes differ (libniji's slot is one 712-byte object; the reference's layout is its own); the calls a replacing
program sees at open and close now match in kind: one allocation per stream at open, released at close.

## Jobs on 46c53df32 (mei)
gate3 (direct tier, role), plants3 (4 CAUGHT), xtest3 (`cargo xtask test`, then the NIJI_FULL_PROFILE hot-text
check, in a RAM copy of the role), bench3 (probe, then `cargo xtask bench --compare --counters cbench_stdio
cbench_startup`, in a RAM copy), basebench (the same bench on the base, in its own copy), baseso (base products for
the layout-bins diff). Superseded: gate (red, the GOT above, head c9a85d7ae), gate2/plants2 (head 22ce171bd:
plants2 cursor-stop SURVIVED, aimed at the wrong test; gate2 stopped for the final head).

## Layout (§9): target/product/fast/layout-bins.tsv, base vs head
Start-up bin 0: 60751 -> 60737 bytes (-14). In it: new_slot 720 -> 796, reuse_slot +364 (outlined, cold),
`TryBox<Slot>::new` +223 (outlined), vfprintf's closure 885 -> 681. Other bins move by -91..+125.

## Bench (instructions per call, Zen 1 host; base = origin/next ae9ea6879 built in its own copy; jobs basebench, bench3)

| row | base | head | delta | glibc |
|---|---|---|---|---|
| startup_instr_c | 2117 | 2117 | 0 | 123452 |
| startup_instr_printf | 65525 | 65484 | -0.06% | 150019 |
| startup_instr_setloc | 61874 | 61895 | +0.03% | 148259 |
| startup_instr_wctype | 62365 | 62373 | +0.01% | 148464 |
| startup_kinstr_c (kernel) | 150702 | 152070 | +0.91% | 720316 |
| fprintf_d | 633.7 | 634.1 | +0.06% | 685.2 |
| fputs_32 | 125.0 | 125.0 | 0 | 146.0 |
| fwrite_4k | 1088.0 | 1088.6 | +0.06% | 1046.0 |
| putc | 23.2 | 23.2 | 0 | 32.2 |
| snprintf_d | 489.5 | 489.8 | +0.06% | 576.7 |
| snprintf_mixed | 2014.2 | 2015.8 | +0.08% | 3336.8 |

startup_kinstr_c is kernel noise: re-counted alternately on the two prebuilt trees (job kinstr, `--runs 5`): base
167910 / 164093, head 167295 / 167927; user-space startup_instr_c 2117 on both. No bench row opens or closes a stream
(cbench_stdio opens its stream once, outside the timed loop), so the fopen/fclose cost itself (a caller malloc and free
of the 712-byte slot in place of a free-list pop and push) is NOT MEASURED here; a fopen_fclose row is a follow-up.

## Open / notes
* INFORMATIONAL: libniji's stderr first use mallocs 9 bytes (reference: none). Pre-existing, not changed here.
* INFORMATIONAL: libniji sets ENOMEM itself when a replacement returns NULL without setting errno (the reference
  leaves errno as the replacement set it). Kept: POSIX lists ENOMEM for fopen.
* Process slips in leg 1 (reported): one `sed -i` edit and one here-string in a `git commit` (no commit made).

## Next step
Land (coordinator). INTERNAL-ALLOC-ROUTING reuses `caller_alloc::new_value` / `release`.
