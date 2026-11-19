# LAYOUT-DYN-PHASE: the dynamic start-up fault count must not depend on unrelated image sizes

State: branch `task/layout-dyn-phase` (cut from origin/next 36df714e7, no code commits yet), role mei:~/w/s1
(Skylake-SP, Linux 6.8). Brief: ~/state/briefs/LAYOUT-DYN-PHASE.md. Records to read: D-X.123, D-X.125
(`records/decisions/D-X/`). Leg 1 (this note) measured and found the causes; leg 2 implements.

## Method (cheap, validated)
`cargo xtask so` run once per profile with `CC=~/work/LAYOUT-DYN-PHASE/cc-log.sh` (logs every `-shared` link);
`padprobe.py relink` reruns the last libc.so.6 / ld.so link with a retained `.text.niji_zz_pad` section of k pages
(int3, after every input, so after the hot group) and `padprobe.py probe` reproduces hot_text_products' dynamic
probe (same staging lengths, argv[0], env, cwd, ADDR_NO_RANDOMIZE, warm-up + 5, min). A relink with k = 0 gives
byte-identical libc.so.6 and ld.so (checked with cmp). `attr.py` splits one run's faults by mapping with
`perf record -e page-faults -d` (perf totals run ~6 over minflt, the deltas are what matter). Scripts: mei
~/work/LAYOUT-DYN-PHASE/{cc-log.sh,padprobe.py,series.sh,attr.py,attr-series.sh}; saved products and padded
libraries in prod-<profile>/, splits in attr-<profile>/ (`<lib>-<k>.faults`, `.maps`).

## Sensitivity on origin/next 36df714e7 (fast, Skylake-SP; job layout-dyn-phase-sens-fast3, every run equal)
| k (pages of cold text) | 0 | 1-5 | 6-9 | 10 | 11-16 |
|---|---|---|---|---|---|
| libc.so.6 padded | 86 | 87 | 86 | 85 | 86 |

ld.so padded: 86 for every k except 87 at k = 6, 7. (DYN_TRUE_BOUND fast is 89; CI saw 91 on other bases.)
Release: job layout-dyn-phase-sens-rel (started at the end of leg 1: release build + same series + splits; read
`grep -E "^release" ~/logs/layout-dyn-phase-sens-rel.log`). Zen 1 host class: not measured yet.

## Where the moving faults are (attr-fast, per-mapping split)
Only `libc.so.6 r--p` moves (9 / 8 / 7 / 8 / 9 over k); libc text 7, libc rw 19, anon 23, stack 18, ld.so 2+3+4 flat.
1. **The loader's whole-file view of libc** (ld maps the whole file read-only to parse it; 2 MiB aligned, so its
   window phase follows file offsets): reads at offset 0 (Elf::parse), 0x1d000 and 0x20000 (versions), `.dynamic`
   (Elf::dynamic, 0x435000 + k) and **the last page of each read-only PT_LOAD** (niji_machine_move_sse2: 0x30000,
   0x343000 + k, 0x42a000 + k). Cause of the last three: `crates/ld/src/map.rs` `map_segment` treats a segment as
   `partial` whenever its file end is not page-aligned (`zero_end` is page-rounded), even with p_memsz == p_filesz,
   and for a read-only segment replaces the last page by an anonymous copy (read from the view). Whether the
   `.dynamic` read and the RO-end read share a 64 KiB window depends on k.
2. **libc's start-up read-only data**: ld (`niji_boundary::ld::link`) reads `__niji_build_tag` (`.rodata`
   0x3d60b8, outside the start-up cluster) and libc's start-up code reads `.rodata1` (0x3d7c00-0x3e1120, 37 KiB,
   NOT aligned in libc.so.6: the cluster anchors are only in crt1 / entry.S, which the shared link does not have).
   Both move with text growth; they take 1 or 2 windows by phase.
3. **ld.so pad k = 6, 7**: ld.so's read-only segment (0x49000.., 30 KiB) crosses one more window (ld.so's base
   is fixed by D-X.125's 512 KiB span, its RO segment moves with its text). Base phase with ASLR off is 0xf000.
libc's text is flat: its text segment is 64 KiB aligned (p_align 0x10000) and the loader honours p_align, so
libc's base is 64 KiB aligned. The hole above libc (reservation trimmed) moves with span but no class moved.

## Decisions in force / plan (leg 2)
* (C) loader: a segment is partial only when p_memsz > p_filesz (real zero fill); a read-only segment without
  zero fill maps its last page from the file (no view read, no anonymous copy: saves up to 3 faults and 3 pages
  per library). Writable segments: no zeroing store without zero fill. Needs Segment to carry the exact memsz end
  (check `grep -rn "Segment {" crates/`); gABI: only bytes past p_filesz up to p_memsz are defined zero.
* (A) libc.so.6: an anchor object first in the shared link (entry.S variant or a small .S) aligning `.rodata1` to
  NIJI_RODATA1_ALIGN (pow2 >= cluster, <= 64 KiB, from startup_ro::cluster_aligns_of), and `__niji_build_tag`
  placed in the cluster (startup_ro::place or the tag's section name; avoid `#[link_section]` in Rust).
* (B) ld.so: its read-only data segment starts 64 KiB aligned relative to its base (entry.S under NIJI_LD is
  first in the ld.so link, or an INSERT fragment beside ld-span.ld); check what ld.so's start-up reads there.
* Then: series flat for k = 0..16 both libraries, both profiles; Zen 1 host class (find a way: ops / coordinator);
  new DYN_TRUE_BOUND at the measured level (lower ok; higher = RULING NEEDED); decision record via
  `python3 ~/src/liella-ops/bin/ops reserve --story LAYOUT-DYN-PHASE D-X`; plant(s) in docs/plants.md
  (drop the alignment / restore the old `partial` test) caught by a k-pad product test if cheap.

## Do not redo
Fast sensitivity series and its splits (above). k = 0 relink identity. The release job if it finished green.

## Exact next step
`~/bin/jobwait layout-dyn-phase-sens-rel` on mei, read the release table, then implement (C), (A), (B).
