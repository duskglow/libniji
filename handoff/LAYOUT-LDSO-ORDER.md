# LAYOUT-LDSO-ORDER: ld.so gets a traced, ordered start-up layout like libc

State: branch `task/layout-ldso-order-2` (continues `task/layout-ldso-order`), run 2 stopped at the context
checkpoint (extension refused). Base origin/next 627ef49ca (next has not moved). Role keke:~/w/n1.
Brief: ~/state/briefs/LAYOUT-LDSO-ORDER.md ("## The change", "## Proof"). Decision: records/pending/LAYOUT-LDSO-ORDER-2-1.toml (D-NEW, series D-X).

## Done
* Run 1: tracer mode `HOT_TEXT_LDSO`, `tools/hottext/ldso_trace.sh` (`true`, `libs3`; slow and numa1 traces must
  equal), `group_ldso`/`ldso_lists` in xtask/src/hottext.rs, traced link in xtask/src/so.rs (details in the D-NEW body).
* Run 2: lint (clippy + static) green on the inherited head (`layout-ldso-order-2-lint0`).
* Proof (1), counts: script ~/work/LAYOUT-LDSO-ORDER-2/count.sh on keke (RAM products under
  /dev/shm/claude/layout-ldso-order-2/r1/prod/{base,new,basepad,newpad}; base built from ~/wl/src/627ef49ca).
  Dynamic program with N libraries, 21 runs, paths of one length (count4 rows; count1 had unequal path lengths,
  which moved instructions by up to ~300):

  | N | faults ASLR off base -> new | ASLR on median base -> new (range) | instructions:u median base -> new |
  |---|---|---|---|
  | 0 | 81 -> 78 | 82 (82-83) -> 79 (78-81) | 9037737 -> 9038084 (+347) |
  | 3 | 97 -> 95 | 97 (96-99) -> 95 (94-97) | 9317884 -> 9317882 |
  | 12 | 133 -> 131 | 134 (132-136) -> 131 (129-133) | 10120229 -> 10120228 |

  The N=0 +347 instructions is deterministic and not explained (libc.so.6 identical, ld.so's dynsym identical).
* Page split (`pagediff.sh`, perf page-fault addresses, N=0, ASLR off): ld.so text faults 5 -> 2; ld.so data
  (5 faults: ELF header page, RO 0x4a000, RW 0x50000, 0x51000 twice) and 17 anonymous unchanged.
* Proof (2), insensitivity: NOT achieved. A 46 KiB `#[cold]` function in ld.rs (pad.patch, not committed; it
  changes libc.so.6 only by the build tag, which hashes all sources, so an ld.so-only swap is refused by ld.so)
  adds +4 faults on base and +4 on the branch (N=0; +4/+3 at N=3, +3/+2 at N=12). Page split of branch+pad:
  +1 ld.so text (the kernel maps the interpreter at a page-aligned base below the previous mapping, so the
  64 KiB window phase moves with the image size; the ~80 KiB group then crosses one more window), +1 ld.so RW
  page, +2 anonymous pages. So ld.so's data (and the window phase) still decide faults.
* Proof (3), determinism: keke (Zen 1) and mei (Skylake-SP) builds of the branch give byte-identical
  ldso-text.list, ldso-text.signatures, hot-text.signatures, layout-bins.tsv, ld.so and libc.so.6; the build's
  slowed and numa1 traces equalled the plain one.
* New test `init_dynamic_startup_only_program_stays_within_its_page_fault_bound` (hot_text_products): page_faults.c
  linked dynamically (fixed-length staging dir, LD_LIBRARY_PATH), `true` mode, bound fast / release 89 / 85
  (Zen 1; mei fast also 89, test passes there). Plants file handoff/LAYOUT-LDSO-ORDER-2-plants.txt (2 plants), not run.

## Not done / next step (in order)
1. Sync, `cargo xtask clippy` + `cargo xtask static` (the pending record included; `cargo xtask records check`).
2. Plants: `cargo xtask plants handoff/LAYOUT-LDSO-ORDER-2-plants.txt --dir /dev/shm/claude/<id>/plants-1 --timeout 3600`
   (llo-link-ungrouped needs a full product build per run, ~20-40 min on keke under load).
3. Gate (direct tier, `--base origin/next`) + the two start-up checks (`cargo xtask test`;
   `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`).
4. Ruling on the N=0 +347 instructions (0.004%) against -3 faults; follow-up story for the window phase
   (e.g. keep the interpreter's mapped size a multiple of 64 KiB or align its base) and ld.so data/bss pages.

Do not redo: base/new/pad builds and counts (logs ~/logs/layout-ldso-order-2-*.log on keke).
Process: one refused `rm -rf $P/${s}ldpad` (variable path) and one refused `ops extend-budget`; see report.
