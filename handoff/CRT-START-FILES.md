# CRT-START-FILES (GitHub #27): start files hold only `_start`, the notes and the usual symbols

Branch `task/crt-start-files-2` (continues `task/crt-start-files`, merge base origin/next ed38a2b3d). Decision:
D-X.144 (records/decisions/D-X/D-X.144.toml, this chain's own record, not landed). Plants: handoff/CRT-START-FILES-plants.txt
(legs 1-2), handoff/CRT-START-FILES-2-plants.txt (leg 3). Role keke:~/w/n2 (Zen 1 host). Scratch on keke: ~/work/crt-start-files-2/.

## Legs 1-2 (CRT-START-FILES, done; details in D-X.144)
Start objects: `_start` (endbr64), `.note.gnu.property` (IBT, SHSTK, x86-64 baseline), `.note.ABI-tag`, `__data_start`,
weak `data_start`, `_IO_stdin_used`, nothing aligned above 64. The layout anchors are libc.a's first two members
(`niji_static_anchor.o`, `niji_static_pie_anchor.o`). crt1.o 67,136 -> 1,816 bytes; dynamic empty main one `R E` LOAD,
page-aligned LOADs, IBT/SHSTK kept. Ruling #7085 accepted the dynamic +4,932 user instructions (per-header cost in
ld.so: DYN-START-RING's item). Tests and leg-2 fixes: see D-X.144 and the plants file.

## Leg 3 (CRT-START-FILES-2): the CI instruction red
CI red: static `cbench_startup/startup_instr_c` 2191 vs limit 2161 (merged tree f56c02def).
Attribution (keke, release profile, the bench linked as `cargo xtask bench` links it, ~/work/crt-start-files-2/ab.sh,
min of 5 children x3, all identical): head cbf537515 2188; same with crt1's property note removed 2160 (ABI-tag note
removed: 2188, costs nothing). Same 5 LOADs in all variants (no read-only PT_LOAD flip, ruling #7061 not involved).
Cause: the static start's one walk over the program headers (`niji_rt::start::program`) cost 14 instructions per
unread non-load header and 8 per PT_LOAD; the property note adds PT_NOTE + PT_GNU_PROPERTY (+28).
Fix (commits 59b0a202f, d7c085162, 7e6a6d3de): a header's type is tested with one bit test
(`READS >> (type & 63) & 1`, mask of PT_PHDR/PT_TLS/PT_GNU_RELRO; a type sharing a bit is compared exactly, first of
each type still wins), and the walk runs over whole headers with one pointer bound. `as_chunks` gave that but is
newer than the workspace MSRV 1.85 (clippy incompatible_msrv), so the boundary now hands the start
`&[[u64; 7]]` (`conv::headers_at`, UNSAFE(slice), same contract as `words_at`, cfg feature "runtime");
`start::program(heads, phdr)`; `program_words(words, phnum, phdr)` keeps the word form for `relro()` and tests.
Measured with the as_chunks version (59b0a202f, same loop shape): 2188 -> 2133 with notes, 2160 -> 2121 without
(6 instructions per header). An intermediate `while get(at)` version measured 2194 (not inlined, bounds checks):
superseded. NOT YET MEASURED: the final head (headers_at form); expected the same as 59b0a202f.
Test: `crates/rt/src/start.rs` `program_reads_only_its_three_types_among_any_others`. Plants: 5/5 CAUGHT (job
crt-start-files-2-plants2 on 7e6a6d3de).

## Open: RED on the final head 7e6a6d3de (job crt-start-files-2-fullprof1)
`NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`: `init_zero_fault_calls_hold_at_every_text_phase`
"pre 0x800: rebind 1 faults, want 0" (every run; all other pads/modes 0). Inherited head passed (fullprof0).
Window bin 0 (release so log, WINDOW-BIN-BYTES): inherited head 63176 B / 103 members (headroom 1336); as_chunks
build 63112 / 103; final head 64395 / 109 members (headroom 117), bin 1 74 -> 68 members: the packer moved 6
members from bin 1 into bin 0 with this head. This touches start-up code, so ALWAYS FORWARD applies (not jitter).
Saved: ~/work/crt-start-files-2/layout-bins.release.59b0a202f.tsv (as_chunks build), layout-bins.fast.cbf537515.tsv.

## Exact next step
1. On keke: diff `~/w/n2/target/product/release/layout-bins.tsv` against layout-bins.release.59b0a202f.tsv: which
   6 members moved to bin 0, and why (start<false>/<true> size? `program_words` or `headers_at` emitted out of
   line?). Make the start's code match the as_chunks build's shape (e.g. `#[inline(always)]` on `headers_at`, or
   keep `program_words` out of the start path), rebuild release, rerun hot_text_products.
2. Then: `sh ~/work/crt-start-files-2/ab.sh ~/w/n2 ~/work/crt-start-files-2/ab3` (expect ~2133), clippy + static,
   direct-tier gate, `cargo xtask test`, fullprof, plants rerun on the final head, D-X.144's numbers checked
   (the body already states the 59b0a202f numbers), report json, precheck, handback-check.
3. Brief item still owed: Skylake-SP class numbers for ruling #7085's dynamic rows (#7090), same rows.

## Jobs (keke)
Green, but on older heads: crt-start-files-2-lint0, crt-start-files-2-fullprof0 (inherited cbf537515),
crt-start-files-2-lint2 (d7c085162), crt-start-files-2-plants2 (7e6a6d3de, 5/5). Red: crt-start-files-2-fullprof1
(7e6a6d3de). Superseded: crt-start-files-2-plants1 (killed, stale code), ab0-ab2, cg, cg2 (probes).
