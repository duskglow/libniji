# STARTUP-REPROFILE: dynamic start-up re-profiled after self-binding; the chain re-ranked

**Ranking (release products, both classes, empty main user instructions):** 1 LD-PHDR-ONCE (handed back: land it;
-40k..-48k) - 2 LD-OBJECT-ONCE with LD-VERSIONS-ONCE merged in (largest left: -50k..-70k; name
`GnuHash::symbol_count`, ~31k) - 3 LD-RELR-LOOP (-50k / -66k three libraries; disjoint code, can run beside 2) - 4 a
new symbolic-lookup story (LD-LOOKUP-COST's structural step, never briefed: -38k / -58k / -64k `/bin/true`) - 5 a new
small story: libc's 8 weak references to the program's init/fini arrays (~-10k, estimated) - 6 LD-START-MAPS (user
~0; kernel +174k and +12 faults over glibc 2.39: attribute first).

**State (leg 2, done):** branch `task/startup-reprofile` from origin/next 9548c734e (LIBC-SELF-BIND-2 landed). No
product code changed. Release products (`NIJI_FULL_PROFILE=1`, the shipped profile; coordinator ruling #7560: rank
on release) built on the Zen 1 host, counted there and copied unchanged to the Skylake-SP host for counts and Intel PT
(no build there). Kit: `handoff/STARTUP-REPROFILE/kit/` (meas.sh: counts and PT; phase.py, relphase.py, fndiff.py,
incl.py, edges.py, gen.py, stc.c, progs/). Release phase table: `handoff/STARTUP-REPROFILE/phase-release-skylake-sp.md`.

## Counts (release, kit form, ASLR off, median of 101; user instr / kernel instr / faults / system calls)

| program | Skylake-SP libniji | Skylake-SP glibc 2.39 | Zen 1 libniji | Zen 1 glibc 2.39 |
|---|---|---|---|---|
| empty main (m0) | 331,895 / 761k / 63 / 29 | 101,855 / 587k / 51 / 30 | 331,733 / 767k / 63 / 28 | 96,040 / 594k / 50 / 30 |
| 3 libraries (m3) | 493,026 / 1,135k / 75 / 59 | 138,761 / 926k / 67 / 64 | 492,864 / 1,149k / 75 / 58 | 132,752 / 922k / 66 / 62 |
| `/bin/true` | 393,361 / 762k / 63 / 29 | 128,175 / 588k / 51 / 30 | 393,199 / 768k / 63 / 28 | 122,349 / 595k / 50 / 30 |
| per extra object ((m3-m0)/3) | 53,710 | 12,302 | 53,710 | 12,237 |
| per lookup ((p200-p0)/200) | 1,445 | 675 | 1,445 | 674 |

Release against the fast products leg 1 measured: empty main -28,181 (360,076), three libraries -43,279, `/bin/true`
-30,570, kernel about -45k and -6 faults per start. min/max spread: at most 10 instructions. Release libc.so.6 has 55
symbolic relocations (fast: 58; memalign, pvalloc, valloc references are gone in release), 1,609 RELATIVE, no RELR.
mmap/mprotect/munmap per empty start: 14 libniji, 12 glibc. The two classes differ by 162 user instructions (0.05%,
CPU-dispatched copies), so the Skylake-SP PT split below holds for Zen 1 within that bound (Zen 1 has no PT; a sampled
Zen 1 profile aliases on a deterministic start and was not used).

**LD-LOOKUP-COST on Zen 1** (brief step 4; its products and count.sh copied from the Skylake-SP host, run unchanged,
no build): empty main 763,224 -> 704,468 (-58,756), `/bin/true` 841,434 -> 768,332, per lookup 1,947 -> 1,507;
glibc 96,042 / 122,349 / 674. Same as its Skylake-SP rows within 0.03%.

## Phase table (release, Skylake-SP, Intel PT on libniji, every user instruction, inclusive of disjoint subtrees)

| phase | empty main | 3 libraries | `/bin/true` |
|---|---|---|---|
| ld.so self-relocation (relocate_self + publish) | 17,869 (5.4%) | 17,869 (3.6%) | 17,869 (4.5%) |
| symbolic relocations (`reloc::one`: decode, lookup, origin, store) | 70,842 (21.3%) | 107,895 (21.9%) | 120,625 (30.7%) |
| RELATIVE relocations and table walk (pass1 less one) | 47,605 (14.3%) | 63,348 (12.9%) | 52,476 (13.3%) |
| per-object tables (`Object::new`) | 83,822 (25.3%) | 107,505 (21.8%) | 85,956 (21.9%) |
| . of which `GnuHash::symbol_count` (inlined in release; see below) | ~31k (9%) | ~31k (6%) | ~31k (8%) |
| whole-file image view and span checks (`image_view`) | 40,213 (12.1%) | 43,237 (8.8%) | 42,727 (10.9%) |
| version requirements (`check_versions`, inlined into link; reconstructed) | ~20.3k (6.1%) | <= 29.7k | ~22.0k (5.6%) |
| search and open | 20,966 (6.3%) | 20,934 (4.2%) | 20,966 (5.3%) |
| mapping (`map_planned`) | 2,136 (0.6%) | 9,792 (2.0%) | 2,136 (0.5%) |
| RELRO | 3,284 (1.0%) | 8,168 (1.7%) | 3,284 (0.8%) |
| debugger rendezvous, publish dynamic | 8,777 (2.6%) | 16,207 (3.3%) | 9,001 (2.3%) |
| init hooks, initialisers | 3,571 (1.1%) | 6,076 (1.2%) | 3,210 (0.8%) |
| libc start, main, exit | 1,807 (0.5%) | 2,162 (0.4%) | (stripped) |
| rest of the loader (link's own code, entry, env, TLS, loader exports; less check_versions) | 10,678 (3.2%) | ~60k (12%) | 13,029 (3.3%) |
| **total** | 331,836 | 492,955 | 393,302 |

Release inlines two functions the fast table named. `Object::new`'s own code is 42,024 (fast: with_style 9,146 +
symbol_count 30,935 = 40,081), so symbol_count is still ~31k. check_versions = (release link own - fast link own) +
Verdefs::next called from link + fast check_versions' other callees (relphase.py): 20,266 / 22,023 match the fast
19,311 / 21,063 within 1k; on three libraries link also inlines more of the link loop, so 29,671 is an upper bound.
TLS set-up is under 1k (inside link).

Cross-cutting (inclusive, inside the rows): `Elf::read_phdr` 47,847 / 86,961 / 48,216 (callers: image_span_kind
16.6k, bytes_from 6.4k, bytes_at 5.0k, check_table 4.3k, published_dynamic 4.3k, image_view 3.2k, find 2.2k,
open_candidate 2.2k, ...); `Elf::image_span_kind` 29,487 / 32,475 / 31,971; `Dynamic::once` 22,004 / 42,223 /
24,673; `Verdefs::next` 21,253 (11.8k in Object::new, 9.4k in check_versions); `symbols::resolve` 32,707 / 61,666 /
58,776. Per extra object (fndiff, /3): 53,706, of it read_phdr 13,038, Dynamic::once 6,740, Object::lookup 6,233,
link own 5,584, symbols::lookup 2,928, check_table 1,755, reloc::target 1,692, RELRO 1,624.

## The 55 symbolic relocations left in release libc.so.6 (ruling #7560 asked which could be bound)

* malloc family (stay: interposable): malloc calloc realloc free aligned_alloc posix_memalign reallocarray (7 in
  release; memalign pvalloc valloc also in fast).
* exported data an executable may copy-relocate (stay symbolic, 40 entries): stdin stdout stderr,
  `_IO_2_1_std{in,out,err}_` (GLOB_DAT + one R_X86_64_64 each), environ, optarg optind opterr optopt, `__tzname
  __timezone __daylight`, program_invocation_name/_short_name, error_*, argp_program_*, argp_err_exit_status,
  obstack_*, re_syntax_options re_max_failures, getdate_err, loc1 loc2, `_res_hconf _IO_list_all _nl_domain_bindings
  _nl_msg_cat_cntr __curbrk __libc_single_threaded __fpu_control`.
* **8 weak undefined references** `_init _fini __preinit_array_{start,end} __init_array_{start,end}
  __fini_array_{start,end}`: the program's linker-defined symbols are hidden, so these never bind and each is a failed
  walk of the whole scope. Not an interposition question: libc can take the arrays from the program's dynamic section
  (DT_INIT_ARRAY...) instead. No story covers it. Only these 8 can go, by removing them, not by binding.

## Per-story estimates (release; same on both classes within 0.05%)

| story | covers (release, empty / 3 libs / true) | estimated saving: empty / 3 libs / true | risk | recommendation |
|---|---|---|---|---|
| LD-PHDR-ONCE (handed back 11-22, not landed) | read_phdr 47.8k / 87.0k / 48.2k | -40k..-48k / -75k..-87k / -40k..-48k (its own -71,423 was on fast products before self-binding) | low (done) | land first |
| LD-OBJECT-ONCE + LD-VERSIONS-ONCE | Object::new 83.8k (symbol_count ~31k, not in its brief), check_versions ~20k, Dynamic::once 22.0k, span checks ~13k beyond read_phdr | -50k..-70k / -65k..-95k / -50k..-70k (overlaps PHDR where Object::new reads headers: bytes_at/bytes_from under gnu_hash) | medium-high (folds checks on untrusted ELF data) | 2nd; merge VERSIONS in (shared verdef index; Verdefs::next counted in both); name symbol_count (bound it from the highest bucket's chain end) |
| LD-RELR-LOOP | RELATIVE + walk 47.6k + self-relocation 17.9k = 65.5k / 81.2k / 70.3k | -50k / -66k / -55k at its <= 15k target; .rela.dyn -38 KB (1,609 x 24 bytes) | medium (link change; an old loop measured +113k) | 3rd; touches reloc/bootstrap, not Object::new: can run beside 2 |
| (none) symbolic lookups | 70.8k / 107.9k / 120.6k; 1,445 vs 675 per lookup | -38k / -58k / -64k at the reference's per-lookup cost | medium | brief LD-LOOKUP-COST's structural step (decode once, (definer, index) from check_versions) |
| (none) 8 weak init/fini-array references | 8 failed scope walks | ~-10k empty, more per library (estimate, not measured) | low | small story |
| LD-START-MAPS | kernel +174k (Skylake-SP 761k vs 587k) / +173k (Zen 1); +12/+13 faults; 14 vs 12 maps; user ~0 | user ~0; kernel up to -174k and -12 faults | medium-high (fault bounds) | keep; first attribute the kernel gap (kernel symbols hidden from the claude user: kptr_restrict 1) |

Sum (empty main): 331.8k - 45k - 60k - 50k - 38k - 10k ~= 129k, less the PHDR/OBJECT overlap, against glibc 2.39's
101,855 (Skylake-SP) / 96,040 (Zen 1): the chain closes about 80-90% of the user-mode gap (~230k), not all of it; the
kernel gap (+174k) is then the larger one. The premise's "lookups ~30k of ~330k" is low: symbolic relocations are
70.8k (21%) of the release empty main, because the 55 libc and 4 program relocations cost 1,445 each.

## Jobs

Release (this leg, the evidence): keke `startup-reprofile-so-release` (build, rc 0), keke
`startup-reprofile-meas-zen1-rel2` (counts, rc 0), mei `startup-reprofile-pt-sky-rel2` (counts and PT, rc 0).
Superseded: keke `startup-reprofile-meas-zen1-rel` (rc 1: I passed a relative OUTDIR and meas.sh runs strace from
`/`; deterministic, rerun with an absolute path), mei `startup-reprofile-pt-sky-rel` (same command; stopped while
queued). Leg 1 fast evidence: keke startup-reprofile-so, -llc-zen1, -meas-zen1; mei -meas-sky, -pt-sky; sampled
Zen 1 profiles -zprof-zen1, -zprof2-zen1 (throttled, then aliased; not used).

## Next step

None for this story. For the coordinator: land LD-PHDR-ONCE; amend LD-OBJECT-ONCE (merge LD-VERSIONS-ONCE, add
symbol_count); brief the lookup story and the init/fini-array story; LD-START-MAPS starts with kernel attribution.
