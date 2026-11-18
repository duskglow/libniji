# DL-DYNAMIC-RELOC handoff (leg 1 of the run, WIP)

Branch `task/dl-dynamic-reloc`, base origin/next de4cdfc8d. Role mei:~/w/s3 (products of the base built there by job
`dl-dynamic-reloc-so-base`, rc 0). Brief: ~/state/briefs/DL-DYNAMIC-RELOC.md. Before the final hand-back read
`~/w/.scratch/DL-DYNAMIC-RELOC/PRIVATE-notes.txt` (scratch, not in the tree) and follow it.

## Probe-settled behaviour (scratch probe ~/w/.scratch/DL-DYNAMIC-RELOC/probe, copied to mei ~/work/dl-dynamic-reloc/probe)

Probe: for each object of `dl_iterate_phdr` (in a constructor, in main, after `dlopen`), each in-memory dynamic entry
vs the same entry of its file: A = differs by exactly the load base, = unchanged. Reference results
(mei, `~/work/dl-dynamic-reloc/ref3.txt`):

| tags | reference (main PIE, start libs, libc, ld.so, dlopen, static-pie) | libniji origin/next |
| --- | --- | --- |
| PLTGOT HASH STRTAB SYMTAB RELA JMPREL RELR GNU_HASH VERSYM | A (base added) | = (link-time) |
| INIT FINI PREINIT_ARRAY INIT_ARRAY FINI_ARRAY VERDEF VERNEED TLSDESC_PLT TLSDESC_GOT, sizes, flags, NEEDED... | = | = |
| DEBUG (main, static-pie) | set by the loader (not this story) | = (INFORMATIONAL) |
| vDSO | untouched (kernel values) | untouched |

Already absolute in the main program's constructor (phase 0). `_DYNAMIC` STRTAB - l_addr = 0x5d0 on the reference;
on libniji `_DYNAMIC` STRTAB - l_addr is negative garbage (the issue). DT_REL is unobservable on x86-64 (GNU ld emits
RELA only): not in the set (INFORMATIONAL). Static-pie also adjusted by the reference: OUT of this story's scope
(brief: start-time, ld.so, dlopen) -> INFORMATIONAL + follow-up. The no-dependency program path (`run` ->
`relocate_self`) is not published either: INFORMATIONAL.
Also seen (INFORMATIONAL, not this story): libniji's `dlpi_name` of start-time libraries is the SONAME, the
reference's is the path; libniji does not set DT_DEBUG.

## The change (commit WIP, not compiled yet)

One rule, `niji_ld::elf`: `RUNTIME_ADDRESS_TAGS` (9 tags above) and `dynamic_published(vaddr, memsz, writable loads)`
(PT_DYNAMIC wholly inside one PF_W PT_LOAD; else the array is never written); `Elf::published_dynamic`;
`shift_address_entries(bytes, delta)`.
- writers: `boot::publish_dynamic` (ld.so's own, from `niji_ld_start` right after `relocate_image`, via
  `bootstrap::publish_dynamic`); `reloc::publish_dynamic(obj, img)` (from the reader's link-time view, value + bias:
  idempotent) in `link` before `protect_relro` (interp skipped) and in dlopen before `protect_relro`.
- readers: `image_view(.., published)` shifts its PT_DYNAMIC copy (and a whole-segment Copy span holding it) back by
  -bias: link's program view false, `own_object` true, `adopt_start` true. `readers::soname_at` drops the bias for a
  published array. ld.rs `run` reads the program's array before link (unchanged). dladdr uses `o.dynm` (link-time view).

## Exact next steps

1. Fix `soname_at`'s doc comment (still says "link-time values"). Sync, `cargo clippy --workspace --all-targets
   --all-features -- -D warnings` on mei, fix. Unit tests: elf (`dynamic_published`, `shift_address_entries`),
   boot (`publish_dynamic` with the `Flat` memory of boot.rs tests), reloc (`publish_dynamic`), clause `no spec clause`
   or the dynlink clause.
2. `cargo xtask so`, rerun the probe: `NIJI_ONLY=1 RO=1 sh ~/work/dl-dynamic-reloc/probe/run.sh /tmp/claude-dldr-X
   $PWD/target/niji` in ~/w/s3; expect A on the 9 tags for main/libs/ld.so/dlopen, RO object loads (phase 3) unchanged.
3. Conformance test `conformance/c/dl_dynamic_reloc.c` (LINK-HELPERS + DLOPEN-HELPERS, LINK-HELPERS lib with RUNPATH
   via HELPER-FLAGS; `_DYNAMIC` and `dl_iterate_phdr` views; DT_RUNPATH string via DT_STRTAB); verify it fails on base.
4. Commit the probe to tools/probe/dl/dynreloc WITHOUT the read-only case (see the private note). Record:
   `records new decision D-NEW --series D-27 --story DL-DYNAMIC-RELOC` (check `grep -rn "dynamic section\|DT_STRTAB"
   records/ docs/differences-from-glibc.md` first for an opposite rule). Plants in docs/plants.md (skip dlopen
   publish -> conformance test msg). Start-up counts before/after (hot_text_products, layout-bins diff), gate,
   `cargo xtask test`, NIJI_FULL_PROFILE hot_text_products, precheck, report json, handback-check.
