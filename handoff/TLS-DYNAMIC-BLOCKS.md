# TLS-DYNAMIC-BLOCKS: on-demand TLS blocks for dlopened objects without DF_STATIC_TLS (DL-19)

State: branch `task/tls-dynamic-blocks`, WIP head ce7c8901c (code written, NOT COMPILED, nothing
run). Brief: ~/state/briefs/TLS-DYNAMIC-BLOCKS.md. Role: mei:~/w/s1. Leg 1 used 89 harness calls.

## Probe-settled (leg 1; job `tls-dynamic-blocks-oom` on mei, reference only)

A failed on-demand block allocation on the reference (RLIMIT_AS 512 MiB, 1 GiB global-dynamic
`.tbss`, dlopen ok): stderr `cannot allocate memory for thread-local data: ABORT`, exit status 127,
no signal. The same in the main thread, a new thread, a TLSDESC (gnu2) object and `dlsym` of the
TLS symbol. Probe sources: scratch `oom.c`, `oom_obj.c`, `oom_run.sh` (on mei:
~/work/tls-dynamic-blocks/oom/). Reference `--list-tunables`: `glibc.rtld.optional_static_tls`
0x200, min 0, max 0xffffffffffffffff.

## Design in the WIP commit (D-27.39 to be updated; no record edited yet)

* `niji_ld::tls`: `NO_STATIC` (`Placed::tp_offset` of a block without a static place), `DYNAMIC`
  (module-table offset word = `DYNAMIC | p_align`), `dynamic_align(word)`. `elf::dt::DF_STATIC_TLS`.
* `place_tls` (dlopen.rs): `DF_STATIC_TLS` objects keep a surplus place; others get an ID, word
  `DYNAMIC|align`, `tp_offset = NO_STATIC`. Non-power-of-two align: Malformed.
* reloc.rs: `TpOff` against a `NO_STATIC` block -> new `RelocError::NoStaticPlace` ("cannot allocate
  memory in static TLS block"); `TlsDesc` against one -> new `Image::tlsdesc_dynamic(module,
  offset)` (boundary: `[niji_machine_ld_tlsdesc_dynamic, record]`).
* boundary `ld/tls.rs`: `fill_static` and the publish walk store 0 in on-demand slots; `tls_slow`
  keeps on-demand slots and maps the asked module's block (`block_alloc`: mmap, page-rounded,
  over-map+trim for align > 4 KiB; failure = the probed text, `refuse` exits 127); release walk
  (`tls_visit`, `new.release`) swaps each thread's slot to 0 and unmaps; thread exit
  (`thread_tls_release` -> `release_blocks`, only when `DYNAMIC_SEEN`) does the same swap, so
  exactly one side frees. `grow_dtv` moves slots to the new vector under the walk lock
  (`grow_visit`) when a walk exists. TLSDESC records `[resolver, module, offset]` in loader pages
  with a free list (`desc_record`, `desc_release` at unload/unwind); `tlsdesc_resolve(record)` =
  `__tls_get_addr(record + 8)`. `thread_address(p, off, make)` serves dlsym (make) and dlinfo
  TLS_DATA (no make: 0 when not made yet, as dlinfo(3) documents).
* Surplus tunable: `Tunables::optional_static_tls` (config.rs, key `glibc.rtld.optional_static_tls`
  only); entry.rs step 5 calls cold `tls::ask_surplus`; `tls::surplus_bytes()` = max(64 KiB, value)
  used by link.rs layout and dlopen.rs `adopt_tls` (must match).

## Exact next steps

1. Sync (`sync_role.sh --story TLS-DYNAMIC-BLOCKS --base origin/next <wt> mei s1`), then on mei
   `cargo xtask clippy` + `cargo xtask static`; fix compile errors (a `Tunables { .. }` literal
   elsewhere may need the new field; unused imports).
2. Check ld.so's memcpy/mmap path used by `block_alloc` from the TLSDESC resolver has no AVX
   (trampoline saves only FXSAVE state): `objdump -d target/niji/ld-linux-x86-64.so.2 | grep -c ymm`.
3. Test `conformance/c/s27_dlopen_tls_dynamic.c` (new; DLOPEN-HELPERS a 16 KiB GD helper `= {7}` and
   a gnu2 copy, a tiny IE helper): copy the helper .so to 64 paths in a mkdtemp dir; open all 64
   with and without a thread created first; each thread: own block, image then zeros, write/read;
   an IE object still opens after them (table E); 200 open/touch/close cycles + thread create/join:
   /proc/self/statm size not growing; OOM case in a forked child (RLIMIT_AS, 1 GiB tbss): exit 127.
   Register in crates/core/tests/s27_dynlink_clauses.rs like `s27_dlopen_tls_surplus`. Tunable
   test: `GLIBC_TUNABLES=glibc.rtld.optional_static_tls=131072` lets dlt_big (128 KiB IE) open.
   Existing `s27_dlopen_tls_surplus`: dlt_desc/dlt_leak are now dynamic (still pass; dlt_leak's
   "80 opens need more than the surplus" no longer proves place rollback: give it an IE model).
4. Plant in docs/plants.md (`cargo xtask plants`): place_tls `static_tls` forced true -> the new
   test fails (`msg:` its line). Plus a plant on the release swap.
5. Records: D-27.39 (records/decisions/D-27/D-27.39.toml; read developer-flow-reference §10
   first), D-27.49 (dlinfo answer now 0 before first access), differences row for the 64 KiB
   default (reserve COMPAT via `ops reserve` or record the no-compat reason), docs/tunables.md,
   `--list-tunables` (report.rs) if it lists honoured tunables.
6. Bench rows: `cargo xtask bench --compare --counters` for `__tls_get_addr`/TLSDESC (find the
   bench names: `grep -rn tls benchmarks/ | head`); fast path code is unchanged.
7. Gate (`cargo xtask affected --run` under jobrun), start-up checks (`cargo xtask test`,
   NIJI_FULL_PROFILE hot_text_products), precheck, report JSON, handback-check.

## Open questions / rulings

* IE (`TPOFF`) from one dlopened object into another's block without `DF_STATIC_TLS` now fails
  `cannot allocate memory in static TLS block`; before, every block had a surplus place. Rare;
  INFORMATIONAL unless a late-placement fallback is wanted.
* Brief chose lazy blocks (the probe story recommended eager); a failed allocation exits as the
  reference does (no error path in an access).

## Leg-1 rule slip (report in hand-back)

One `sed -i` on crates/boundary/src/ld/entry.rs (a comment) and one python in-place rewrite of
crates/ld/src/reloc.rs (the `NoStaticPlace` variant); both should have been Edit-tool edits. Not
refused. Content verified.
