# LD-START-MAPS: the start's mappings at or below the reference's kernel cost

## Leg 2 (2026-11-23): design steps 1-5 implemented and measured; NOT yet gated, no tests or plants

Commits (on 5d1cebfea): a765f4a7b (read start dependencies from headers + mapped image; D-27.92 record),
ee10b271d (dynamic array copy by `pread`, removing +1 fault per library). Clippy of niji-boundary + niji-ld
clean on mei:~/w/s4. Code: `image.rs` `open_start` / `read_headers` / `image_object` / `Found` / `Mapped` /
`arena_give_back` / `image_spans` (split out of `image_view`; `file: Option<fd>`), `StartMapper::store_file`;
`map.rs` `plan_headers`, `Mapper::store_file` (default Err), cold `tail_from_file`; `dlopen.rs`
`loadable_dynamic` now pub; `link.rs` uses `open_start` in the search and for path names (falls back to the old
`open_file` path via cold `by_path` on any refusal), flat `maps` list (`keep_map`). ARENA 4 -> 16 KiB.

After (kit, release, Skylake-SP, median of 101; jobs `ld-start-maps-meas-new3` vs `ld-start-maps-meas-base`):

| program | user | kernel | faults | syscalls | maps |
|---|---|---|---|---|---|
| n0 | 159,047 -> 166,208 | 730,939 -> 628,650 | 58 -> 57 | 29 -> 28 | 14 -> 11 |
| n3 | 250,528 -> 274,189 | 1,122,981 -> 921,688 | 70 -> 70 | 59 -> 61 | 35 -> 29 |
| t | 216,900 -> 224,056 | 730,456 -> 629,441 | 58 -> 57 | 29 -> 28 | 14 -> 11 |
| glibc 2.39 g0 / g3 / true | 101,855 / 138,761 / 128,175 | 588,762 / 925,983 / 592,143 | 51 / 67 / 51 | 30 / 64 / 30 | |

n3's kernel count is now below glibc's; n0 is +40k over it (was +142k). Open cost: user +7.2k per object
(n3 +24k), not yet attributed (candidates: `image_spans` + `from_image` per object, image-mode `span_bytes`
lookups in symbol/reloc reading, the `pread` copy; use the kit's callgrind/PT tables, `meas.sh ... pt`).
Net instructions per start still fall (n0 -95k, n3 -177k), but a cost row must go in the report.

Next steps, in order:
1. Attribute the +7.2k user instructions per object and cut what is cheap (ALWAYS FORWARD).
2. Tests (name the clause, §27.1): (a) `crates/ld/tests/map.rs`: `plan_headers` on a header prefix gives
   `plan`'s result and errors on the whole file (segment past the end -> OutOfRange(Segment(i))); (b) a start
   whose dependency has a read-only segment with zero fill (`store_file` path) runs and sees its bytes; (c) a
   search candidate refused at the dynamic step (PT_DYNAMIC outside every PT_LOAD) is skipped and the next
   directory's copy is loaded; (d) headers past 1 KiB (many PT_NOTEs) still load (view fallback).
3. Plants (`docs/plants.md` via `cargo xtask plants`): `read_headers` keep computation, `plan_headers` range
   check, `tail_from_file`/`store_file`, the `image_object` refusal mapping, `arena_give_back` zeroing,
   `image_spans` pread offset.
4. Gate (`cargo xtask affected --run`, base origin/next), `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask so
   && ... hot_text_products` (DYN_TRUE_BOUND may tighten: re-measure, tighten, never loosen), layout-bins diff,
   static, precheck, report JSON, handback-check. Mention in hand-back first line: per-start numbers (owner 11-21).

## Leg 1

**State (leg 1 of the run, attribution done, no product code changed yet):** branch `task/ld-start-maps` from
origin/next e3b0caf60. Role mei:~/w/s4 (Skylake-SP). Release products (`NIJI_FULL_PROFILE=1`) of that base are in
`~/work/ld-start-maps/base` on the role's host; the kit is installed in `~/work/ld-start-maps/kit` there
(STARTUP-REPROFILE's kit plus this story's files in `handoff/LD-START-MAPS/kit/`: `attr.sh`, `ki.py`, `pfclass.py`,
`build.sh`; `meas.sh` reads `K` from the environment).

## Before counts (release, kit form, ASLR off, median of 101; Skylake-SP; job `ld-start-maps-meas-base`)

| program | libniji user / kernel / faults / syscalls / maps | glibc 2.39 |
|---|---|---|
| empty main (n0 / g0) | 159,047 / 730,939 / 58 / 29 / 14 | 101,855 / 588,762 / 51 / 30 / 12 |
| 3 libraries (n3 / g3) | 250,528 / 1,122,981 / 70 / 59 / 35 | 138,761 / 925,983 / 67 / 64 / 27 |
| `/bin/true` (t / true) | 216,900 / 730,456 / 58 / 29 / 14 | 128,175 / 592,143 / 51 / 30 / 12 |

The kernel gap is +142k per empty start (+197k with three libraries), and the fault gap +7 (+3).

## Attribution (job `ld-start-maps-attr-base4`; `attr.sh base OUT 600` with `PERIOD=50000`)

Kernel instructions are sampled every 50,000 with the user registers at kernel entry, 600 starts, and each sample is
charged to the user instruction that entered the kernel (`ki.py`; system calls split by length argument). Faults
are every fault with its data address, classified by mapping (`pfclass.py`). Kernel symbols are hidden from the
`claude` user (kptr_restrict 1), so the kernel side is attributed by entry site. The finer periods are throttled
(the host's perf_event_max_sample_rate is 26000), which is why the period is 50,000.

Per empty start, libniji (kernel instructions, ±~1.5k each):

| what | kernel instr | faults |
|---|---|---|
| `munmap` of the 64 MiB view arena (with libc's view in it) | 37,900 | - |
| `mmap` of libc.so.6's whole-file view (18 MB release file) over the arena | 22,200 | - |
| `mmap` of the 64 MiB PROT_NONE arena | 8,700 | - |
| faults on the view: header page (`Elf::parse`), `_DYNAMIC` (`Elf::dynamic`), a string in `link` | ~6k + ~21k + part of `link` | 3 |
| **whole-file view, total** | **about 70k syscalls + about 30k faults** | **3** |
| main stack: `link`'s frame (5 pages, stack probes over its 16-entry inline tables), `search::search` (2), `entry::run` (1) | about 33k in `link` + search | 8 (glibc 2.39: 1) |
| libc.so.6 RELRO pages written by `reloc::pass1` | 37k | 7 (glibc 2.39: 5) |
| libc.so.6 writable data | | 5 (glibc 2.39: 2) |
| path store (16 KiB anonymous, `PathStore`, first search hit) | 6,500 mmap + 1 fault | 1 |
| thread block and TLS (70,736 B, `map_area`) | 5,800 mmap; `rseq` registration faults 13,600 | 2 |
| main thread's descriptor area (69,632 B, MAP_STACK, `start_under_niji_ld`) | 4,200 + 1 fault | 1 |
| libc text faults | | 5 (glibc 2.39: 15) |

Conclusion: the whole-file view is about 100k of the 142k kernel gap and 3 faults. The stack is the other
large fault item (+7); it comes from `link`'s inline per-object tables (`Table<_, INLINE>`, INLINE 16, entries of
`Obj`, `symbols::Object`, ...), touched page by page by the stack probes whether or not entries are used.

## Design for the view (the next step)

The candidate check (`dlopen::check_into`: `Elf::parse`, `map::plan`, `loadable_dynamic`) reads the whole file:
`map::plan` asks `segment_bytes` for each PT_LOAD (a range check against the file), and `Elf::dynamic` reads
`.dynamic` and `.dynstr` (via `bytes_at(DT_STRTAB)`). So a pread of the headers alone cannot feed it. The order that
works, with no check dropped (owner 11-17: move or fold, never drop):

1. `open_candidate` / `open_file` (crates/boundary/src/ld/image.rs): `pread` ehdr plus the program header table
   into the start arena (one `pread` of the first 1 KiB, a second only when `e_phoff + table` lies beyond it).
2. A header-mode reader: `Elf::parse_headers::<A>(prefix, file_len)` doing exactly `Elf::parse`'s checks, with
   `segment_bytes`' file-range check replaced by `offset + filesz <= file_len` for `map::plan` (same
   `ElfError::OutOfRange(Part::Segment(i))`).
3. Plan and map the segments (`map::map_planned`). `map_segment` reads file bytes only for a read-only segment's
   partial tail page: `pread` there (the `StartMapper` has the fd).
4. Read the object from its image as the program and ld.so already are (`image_view` + `Elf::from_image`,
   D-27.18): the header table at `loaded.phdr`, `.dynamic` copied, the read-only segments sliced in place. Then
   `loadable_dynamic` (PIE refusal) and the SONAME test for ld.so (`link.rs` line ~207) run on that; a refusal now
   unmaps the mapping and skips as before (the search's verdict order is unchanged: header errors, EXEC, plan,
   dynamic).
5. The start arena (`ARENA`, 4 KiB in ld.so's .bss) must hold each object's header copy, spans and `.dynamic` copy
   (~2 KiB per object): enlarge it (untouched .bss pages cost nothing), or a start with 3+ libraries falls back to
   anonymous scratch mappings and gets worse.
6. Keep `ld.so --list`, `dlopen` and the exec-stack paths (`exec_stack_candidate`, `exec_stack_dependency`) on
   the view path unless they move with the same reader; DYN-START-PARITY-2's file-system dependence goes only when
   the start path maps no view.

Expected: -3 syscalls (arena mmap, view mmap, arena munmap) and about -70k kernel instructions per object loaded;
the view's 3 faults move to the segments (2 fault-around windows of the first segment, 1 read of `.dynamic` in the
RELRO page that `pass1` writes anyway), so faults stay about level. That alone leaves about +70k and +7 faults:
the stack (inline tables), the path store mapping and the descriptor area are the rest.

## Do not redo

- The attribution above (kit and logs on the role's host: `~/work/ld-start-maps/attr-base/`).
- Fault-around tricks on the view (`MADV_RANDOM`): each touched page then faults alone (rela.dyn alone is ~10
  pages), more faults, rejected by reasoning, not measured.
- Mapping the view without the arena (`map_view_alone`): brings back DYN-START-SPEED-2's alignment dependence.

## Next step

Implement the design steps 1-5 on the start path only, measure with `meas.sh` (n0, n3, t) against
`~/work/ld-start-maps/base`, then the gate, plants, `cargo xtask test` and the release hot_text_products check.
