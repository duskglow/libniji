# STARTUP-REPROFILE: dynamic start-up re-profiled after self-binding; the chain re-ranked

**State (leg 1, WIP):** branch `task/startup-reprofile` from origin/next 9548c734e (LIBC-SELF-BIND-2 landed). No
product code changed. Role keke `~/w/r1` (Zen 1 host); Skylake-SP counts and Intel PT on mei (products copied, no
build there). Kit: `~/work/startup-reprofile/` on both hosts (meas.sh, stc.c, incl.py, phase.py, fndiff.py,
edges.py, gen.py, progs/). **Release products: build running (`startup-reprofile-so-release`); every table below is
FAST products until leg 2 replaces it (coordinator ruling #7560: rank on the release kit, both classes).**

**Ranking (fast products, empty main, user instructions; release check pending):** 1 LD-PHDR-ONCE (-60k..-71k,
handed back) - 2 LD-RELR-LOOP (-68k) - 3 LD-OBJECT-ONCE with LD-VERSIONS-ONCE merged in (-60k..-80k, naming
`GnuHash::symbol_count`) - 4 a new symbolic-lookup story (LD-LOOKUP-COST's structural step, never briefed: -40k
empty, -70k `/bin/true`) - 5 LD-START-MAPS (user ~0, kernel up to -220k and -18 faults; attribution first) - 6 a new
small story: libc's 8 weak references to the program's init/fini arrays (-10k).

## Counts (kit form, ASLR off, median of 101; user instr / kernel instr / faults / system calls)

| program | Skylake-SP libniji | Skylake-SP glibc 2.39 | Zen 1 libniji | Zen 1 glibc 2.39 |
|---|---|---|---|---|
| empty main (m0) | 360,076 / 806k / 69 / 29 | 101,855 / 587k / 51 / 30 | 359,911 / 823k / 69 / 28 | 96,040 / 595k / 50 / 30 |
| 3 libraries (m3) | 536,305 / 1,192k / 81 / 59 | 138,761 / 922k / 67 / 64 | 536,140 / 1,215k / 81 / 58 | 132,752 / 924k / 66 / 62 |
| `/bin/true` | 423,931 / 810k / 69 / 29 | 128,175 / 587k / 51 / 30 | 423,766 / 823k / 69 / 28 | 122,349 / 596k / 50 / 30 |
| per extra object ((m3-m0)/3) | 58,743 | 12,302 | 58,743 | 12,237 |
| per lookup ((p200-p0)/200) | 1,507 | 674 | 1,507 | 674 |

mmap/mprotect/munmap per empty start: 14 libniji, 12 glibc (strace). User counts differ between the classes by 165
instructions (0.05%, CPU-dispatched copies), so the Skylake-SP PT split below holds for Zen 1 within that bound. A
sampled Zen 1 profile (`perf record -e instructions:u`, fixed period over 40,000 starts) aliased: a deterministic
start sampled at a fixed period hits the same few instructions every run, so it is not used (period 997 was
throttled). **Zen 1 per-function split: not separately measured; bounded by the 0.05% total difference.**

**LD-LOOKUP-COST on Zen 1** (its products and count.sh copied from the Skylake-SP host, run unchanged, no build):
empty main 763,224 -> 704,468 (-58,756), `/bin/true` 841,434 -> 768,332, per lookup 1,947 -> 1,507; glibc 96,042 /
122,349 / 674. Same as its Skylake-SP rows within 0.03%.

## Phase table (Skylake-SP, Intel PT on libniji only, every user instruction, inclusive of disjoint subtrees)

| phase | empty main | 3 libraries | `/bin/true` |
|---|---|---|---|
| ld.so self-relocation (relocate_self + publish) | 22,195 (6.2%) | 22,195 (4.1%) | 22,195 (5.2%) |
| symbolic relocations (`reloc::one`: decode, lookup, origin, store) | 77,269 (21.5%) | 116,394 (21.7%) | 129,133 (30.5%) |
| RELATIVE relocations and table walk (pass1 less one; libc 2,395 RELATIVE) | 60,764 (16.9%) | 76,774 (14.3%) | 65,983 (15.6%) |
| per-object tables (`Object::with_style` = Object::new) | 77,474 (21.5%) | 97,380 (18.2%) | 79,538 (18.8%) |
| . of which `GnuHash::symbol_count` | 30,935 (8.6%) | 31,190 (5.8%) | 30,935 (7.3%) |
| whole-file image view and span checks (`image_view`) | 41,928 (11.6%) | 44,952 (8.4%) | 44,442 (10.5%) |
| version requirements (`check_versions`) | 19,311 (5.4%) | 22,347 (4.2%) | 21,063 (5.0%) |
| search and open | 21,026 (5.8%) | 20,994 (3.9%) | 21,026 (5.0%) |
| mapping (`map_planned`) | 2,270 (0.6%) | 10,346 (1.9%) | 2,270 (0.5%) |
| RELRO | 6,259 (1.7%) | 15,601 (2.9%) | 6,259 (1.5%) |
| debugger rendezvous, publish dynamic | 9,124 (2.5%) | 17,043 (3.2%) | 9,288 (2.2%) |
| init hooks, initialisers | 3,532 (1.0%) | 6,043 (1.1%) | 3,172 (0.7%) |
| libc start, main, exit | 1,865 (0.5%) | 2,277 (0.4%) | (stripped) |
| rest of the loader (link/entry code, env, TLS, loader exports, the 3-lib link loop) | 16,994 (4.7%) | 83,882 (15.6%) | 19,497 (4.6%) |
| **total** | 360,011 | 536,228 | 423,866 |

Cross-cutting (inclusive, already inside the rows): `Elf::read_phdr` 62,496 / 124,248 / 62,868 (callers:
image_span_kind 16.7k, image_view 7.9k, bytes_from 6.4k, protect_relro 5.5k, bytes_at 5.0k, check_table 4.3k,
published_dynamic 4.3k, link 3.1k, ...); `Dynamic::once` 20,256 / 40,530 / 22,926; `Verdefs::next` 21,191 (11.8k in
Object::new, 9.4k in check_versions); `symbols::resolve` 34,739 / 65,180 / 61,152. TLS set-up is under 1k (inside
link). Per extra object (fndiff, /3): read_phdr 20,584, symbolic relocations 13,042, Dynamic::once 6,758,
Object::new 6,635, parse 6,579, check_table 5,832, RELRO 3,114. Per lookup (n200-p0): reloc::target 1,179 incl,
Object::lookup 572, closure 349, symbols::lookup 690 incl.

## The 58 symbolic relocations left in libc.so.6 (55 GLOB_DAT + 3 R_X86_64_64)

* malloc family (stay: interposable): malloc calloc realloc free aligned_alloc memalign posix_memalign pvalloc valloc
  reallocarray (10).
* exported data an executable may copy-relocate (stay symbolic): stdin stdout stderr, `_IO_2_1_std{in,out,err}_`
  (GLOB_DAT + one R_X86_64_64 each), environ, optarg optind opterr optopt, `__tzname __timezone __daylight`,
  program_invocation_name/_short_name, error_print_progname error_one_per_line error_message_count, argp_program_version
  argp_program_version_hook argp_program_bug_address argp_err_exit_status, obstack_exit_failure
  obstack_alloc_failed_handler, re_syntax_options re_max_failures, getdate_err, loc1 loc2, `_res_hconf _IO_list_all
  _nl_domain_bindings _nl_msg_cat_cntr __curbrk __libc_single_threaded __fpu_control` (37 entries).
* **8 weak undefined references** `_init _fini __preinit_array_{start,end} __init_array_{start,end}
  __fini_array_{start,end}`: the program's linker-defined symbols are hidden, so these never bind and each is a failed
  walk of the whole scope. Not an interposition question: libc can take the arrays from the program's dynamic section
  (DT_INIT_ARRAY...) instead. ~10k on the empty main, more per library. No story covers it.

## Per-story estimates (fast products; same on both classes within 0.05%)

| story | covers (measured now) | estimated saving: empty / 3 libs / true | risk | recommendation |
|---|---|---|---|---|
| LD-PHDR-ONCE (handed back 11-22, not landed) | read_phdr 62.5k / 124k / 63k | -60k..-71k / ~-125k / -60k..-71k (its own: -71,423 and -21.5k per object on a pre-self-bind base, Zen 1) | low (done) | land first |
| LD-RELR-LOOP | RELATIVE + walk 60.8k, self-relocation 22.2k (83k; brief said 73k) | -68k / -84k / -73k at its <= 15k target; .rela.dyn -55 KB | medium (link change; old loop measured +113k) | move to 2nd; touches reloc/bootstrap, not Object::new: can run beside LD-OBJECT-ONCE |
| LD-OBJECT-ONCE | Object::new 77.5k (symbol_count 30.9k not in its brief), Dynamic::once 20.3k, span checks ~13k, search | -45k..-65k / -75k..-100k / similar | medium-high (folds checks on untrusted ELF data) | 3rd; name symbol_count (bound it from the highest bucket's chain end) |
| LD-VERSIONS-ONCE | check_versions 19.3k / 22.3k / 21.1k (brief's 43k on true is stale: LD-LOOKUP-COST halved it) | -14k / -17k / -16k | low | merge into LD-OBJECT-ONCE (shared verdef index; Verdefs::next counted in both) |
| LD-START-MAPS | kernel side: +219k (Skylake-SP) / +228k (Zen 1) kernel instructions, +18/+19 faults, 14 vs 12 maps; user ~0 | user ~0; kernel up to -220k and -18 faults | medium-high (fault bounds) | keep; first attribute the kernel gap (kernel symbols are hidden from the claude user: kptr_restrict 1) |
| (none) symbolic lookups | 77.3k / 116.4k / 129.1k; 1,507 vs 674 per lookup | -40k / -60k / -70k at the reference's per-lookup cost | medium | brief LD-LOOKUP-COST's structural step (decode once, (definer,index) from check_versions) |
| (none) 8 weak init/fini-array references | 8 failed scope walks | ~-10k empty, more per library | low | small story |

Sum of the estimates (empty main): 360k - 68k (PHDR) - 68k (RELR) - 70k (OBJECT+VERSIONS) - 40k (lookups) - 10k
~= 104k against glibc 2.39's 101,855 (Skylake-SP) / 96,040 (Zen 1): user-mode parity is in reach; the kernel gap
(+220k) then dominates.

## Jobs

keke: startup-reprofile-so (fast products, rc 0), startup-reprofile-llc-zen1 (rc 0), startup-reprofile-meas-zen1
(rc 0), startup-reprofile-so-release (running). mei: startup-reprofile-meas-sky (rc 0, first PT attempt: perf report
inclusive did not synthesise chains), startup-reprofile-pt-sky (rc 0, the tables above). Superseded:
startup-reprofile-zprof-zen1 and -zprof2-zen1 (sampled Zen 1 profile: throttled, then aliased; not used).

## Next step (leg 2)

When `startup-reprofile-so-release` ends: copy `~/work/startup-reprofile/release` to mei; run
`meas.sh <release> out-zen1-rel` (keke) and `meas.sh <release> pt-sky-rel pt` (mei) as jobrun count jobs;
`phase.py` on pt-sky-rel; replace every table with release figures (keep a fast column only where it shows the
split is the same); re-check the ranking; then report json, precheck, handback-check.
Local copies of the kit and the fast PT tables: the run's scratch dir (`kit/`, `pt-sky/`). meas.sh sets
`KEEP=1` to leave its program dir; incl.py needs `perf script --itrace=i1ig256 -F comm,ip,sym | c++filt`. Ruling
#7560 also asked for the 55 libc names and which could be bound: answered above (only the 8 weak array references,
and those by removing them, not binding). JOBS so far for the hand-back: keke:startup-reprofile-so-release plus the
two release meas jobs of leg 2 (the fast jobs are evidence, not proof of a head: no code changed).
