# STARTUP-STRUCTURE-REVIEW: where a dynamic start's instructions go, and what could structurally go

Review only: nothing here changes libniji. Measured on snapshot-2026-11-21.6 (RELEASE-SNAPSHOT-1121E, 97bf3542f;
+1,408 user instructions per start over the brief's 21.5). Zen 1 build host (AMD EPYC 7551), median of 101, ASLR
off. Every number names its job (`jobrun` labels; logs and outputs on the Zen 1 build host, named in
handoff/STARTUP-STRUCTURE-REVIEW.md).

**The three biggest structural costs (empty dynamic main, 752,610 user instructions native, glibc 2.39 95,775):**

1. **libc.so.6 looks up its own names at every start.** It carries 288 symbolic relocations (210 JUMP_SLOT,
   75 GLOB_DAT, 3 R_X86_64_64; BIND_NOW); 277 of their 285 names are defined in libc.so.6 itself. Resolving them
   costs **~460k instructions** (298 symbolic relocations in all: 493,648 instructions, 1,656 each, 59% of the
   start). Prototype: bound at link time (malloc family left preemptible) the empty main drops **742,449 ->
   328,846 (-413,603, -56%)** and the 3-library program **985,293 -> 482,090 (-503,203)**.
2. **One symbol lookup costs 1,373 instructions against the reference's ~600** (black box: 200 imported libc
   functions), and every lookup walks each object ahead of the definer at ~120 instructions per object.
3. **Each loaded object costs ~94k against the reference's ~12k.** Program headers are decoded about 550 times
   (`Elf::read_phdr` 82,779 on the empty main, 149,568 with 3 libraries). Dynamic tags are found by linear
   rescans (28,449). Building each object's symbol tables costs ~30k (91,584 for 3 objects). The whole-file check,
   path search and image views come on top.

The reference spends ~98% of its start in its loader too: libc's own initialisation, `main` and exit cost about
2-3k instructions in both libraries. **The whole gap is in ld.so.**

## 1. Method and jobs

| job (Zen 1 build host) | what |
|---|---|
| `startup-structure-review-cg3` (rc 0) | trixie root (mmdebstrap, valgrind 3.24.0, gcc 14.2.0); before the swap: glibc 2.41 (the root's own) and the host's glibc 2.39 copied with its own loader (PT_INTERP + RUNPATH); after swapping libniji's libc6, libc6-dev, libc-bin, libc-dev-bin and libc6-dbg (names) in: empty main, 3 libraries, `/bin/true`, a glibc-built empty main. Per program: native counts, the same with valgrind's client environment, callgrind, strace. Outputs `~/work/STARTUP-STRUCTURE-REVIEW/cg3/out/*.cg` |
| `startup-structure-review-proto1` (rc 0) | prototypes below; kit form (snapshot programs, `LD_LIBRARY_PATH`) on the host |
| `startup-structure-review-ips2` (rc 0) | CPU time per start: 5,000 starts x 3 interleaved rounds under `perf stat` |

Programs: the snapshot start-up kit's `m0.c` (empty main) and `m3.c` (main calling one function in each of three
one-function libraries), `/bin/true` (coreutils, no argument: it returns before any locale or stdio work).
Scripts: `~/work/STARTUP-STRUCTURE-REVIEW/` (cg.sh, pre.sh, post.sh, meas.sh, stcount.c, proto.sh, selfbind.py,
iprobe.c, ips.sh, execloop.c).

**Native counts in the root (cg3; user instructions / faults / system calls):**

| program | glibc 2.39 | glibc 2.41 | libniji 21.6 |
|---|---|---|---|
| empty main | 95,775 / 49 / 29 | 96,541 / 51 / 30 | 752,610 / 63 / 26 |
| 3 libraries | 131,649 / 65 / 64 | 132,776 / 66 / 62 | 1,034,067 / 75 / 62 |
| `/bin/true` | - | 128,063 / 51 / 30 | 884,931 / 63 / 26 |
| glibc-2.41-built empty main | - | - | 753,523 / 63 / 26 |

The snapshot's own `execcount` in the root on the kit's empty main (`LD_LIBRARY_PATH` form) reads 742,003 (host
table: 742,708); the ordinary build in the swapped root costs +10.6k more (system paths, the loader cache, crt
files).

**Separating valgrind's start-up shims.** callgrind counts from the loader's first instruction. valgrind adds an
object (`vgpreload_core-amd64-linux.so`, `LD_PRELOAD`) and `VALGRIND_LAUNCHER` to the client. Each program was
also counted natively with exactly that environment:

| program | native | native + valgrind's environment | callgrind | callgrind minus like-for-like |
|---|---|---|---|---|
| libniji empty main | 752,610 | 840,715 (+88,105) | 843,436 | +2,721 |
| libniji 3 libraries | 1,034,067 | 1,126,612 (+92,545) | 1,134,201 | +7,589 |
| libniji `/bin/true` | 884,931 | 977,777 (+92,846) | 980,484 | +2,707 |
| glibc 2.39 empty main | 95,775 | 108,983 (+13,208) | 113,881 | +4,898 |
| glibc 2.39 3 libraries | 131,649 | 146,781 (+15,132) | 151,828 | +5,047 |

The preload object's own code runs 30 instructions. Its cost is the loader's per-object work (+88k on libniji,
+13k on the reference, the same as one more library). The residual (+2.7k to +7.6k, under 1%) is valgrind's
emulation (synthesised stack and auxiliary vector, CPUID). The phase tables below are callgrind's counts.
**Subtract the preload row to read native numbers.**

## 2. Phase table

callgrind, instructions inclusive. libniji: functions grouped by phase from its call tree. Reference: a black-box
split by object and by marginal cost only (its internals are not read).

| phase | libniji empty main | libniji 3 libraries | glibc 2.39 (black box) |
|---|---|---|---|
| kernel entry to loader start (entry, auxv, environment, LD_* and tunables, CPU features, thread pointer) | 5,540 | 5,540 | (in its loader total) |
| loader self-relocation (232 words, ~70 instructions each) | 16,250 | 16,250 | (in its loader total) |
| finding, checking and mapping objects (search with whole-file check, image views, parse, program headers, dynamic arrays, segments, published dynamic) | 108,012 | 204,718 | per object, see below |
| per-object symbol tables (`Object::new`: dynamic tags, GNU hash, version definitions) | 91,584 | 115,556 | |
| symbolic relocations (298 / 313; `reloc::one`) | **493,648** (1,656 each) | **611,122** (1,952 each) | ~600 each (marginal, see below) |
| RELATIVE relocations and table walking (`pass1` less `one`) | 73,094 | 90,960 | |
| version requirements (`check_versions`) | 20,041 | 23,113 | |
| RELRO protection | 4,779 | 8,865 | |
| TLS set-up | < 1,000 (inside `link`) | < 1,000 | |
| loader bookkeeping (`link` itself, debugger rendezvous, init hooks, loader-export lookups) | 20,842 | 41,059 | |
| other (each under 1,000) | 6,906 | 14,115 | |
| **loader total** (rows' sum; ld.so's own code 840,647 / 1,131,123) | **840,696** | **1,131,298** | **111,705** (98.1%) |
| libc's own initialisation, start, `main`, exit and destructors (all code in libc.so.6 and the program) | 2,740 | 2,903 | 2,106 |
| locale, environment and stdio set-up; constructors | not reached by these programs | | |
| **total** | 843,436 | 1,134,201 | 113,881 |
| less valgrind's preload object (native, measured) | -88,105 | -92,545 | -13,208 |

**Black-box marginals (native).** These compare per object and per lookup without reading the reference's
internals:

| marginal | libniji | glibc 2.39 | ratio |
|---|---|---|---|
| one more loaded library (3-library minus empty main, / 3; cg3) | 93,819 | 11,958 | 7.8x |
| one more symbolic relocation against libc (200 imported functions minus none, / 200; proto1) | 1,373 | ~600 (122,066 / 205 GLOB_DAT) | 2.3x |
| symbolic relocations at an empty start | 298 (283 without valgrind's object) | (not counted: black box) | |

`/bin/true` against the empty main (49 more imports, bigger program): libniji +132,321, glibc 2.41 +31,522.

**Why one libniji lookup costs 1,373-1,656.** Per relocation, from the exclusive table: ~390 in `reloc::target`
(relocation decode, symbol fetch through checked readers, name memo); ~640 walking each object's hash chains
(`Object::lookup` and its compare closure, checked byte reads on every step); ~250 rebuilding names and versions
per lookup (`memo_name`, `version_name`, `Verdefs::next`, `def_name_from`); ~150 in `symbols::lookup`; plus ~120
for each object ahead of the definer in the scope (no cheap reject before the chain walk).

### Exclusive, every function over 1% of its program's total (libniji, callgrind; cg3)

| function (object) | empty (% of 843,436) | 3libs (% of 1,134,201) | true (% of 980,484) |
|---|---|---|---|
| `ld::reloc::target` (ld.so) | 115,441 (13.7) | 120,460 (10.6) | 131,239 (13.4) |
| `<ld::symbols::Object>::lookup::{closure#0}` (ld.so) | 97,576 (11.6) | 98,812 (8.7) | 108,315 (11.0) |
| `<ld::symbols::Object>::lookup` (ld.so) | 92,778 (11.0) | 167,838 (14.8) | 107,387 (11.0) |
| `<ld::elf::Elf>::read_phdr` (ld.so) | 82,779 (9.8) | 149,568 (13.2) | 85,116 (8.7) |
| `ld::reloc::pass1::` (ld.so) | 57,482 (6.8) | 60,571 (5.3) | 61,914 (6.3) |
| `ld::symbols::lookup` (ld.so) | 44,546 (5.3) | 77,486 (6.8) | 51,339 (5.2) |
| `<ld::symbols::Object>::new` (ld.so) | 42,339 (5.0) | 43,291 (3.8) | 42,445 (4.3) |
| `<ld::symbols::Object>::memo_name` (ld.so) | 37,490 (4.4) | 37,490 (3.3) | 46,252 (4.7) |
| `<ld::elf::Dynamic>::once` (ld.so) | 28,449 (3.4) | 49,147 (4.3) | 30,667 (3.1) |
| `ld::reloc::one::` (ld.so) | 25,926 (3.1) | 27,231 (2.4) | 30,297 (3.1) |
| `<ld::symbols::Object>::version_name` (ld.so) | 21,490 (2.5) | 21,490 (1.9) | 26,154 (2.7) |
| `<ld::elf::Verdefs as core::iter::traits::iterator::Iterator>::next` (ld.so) | 14,177 (1.7) | 14,177 (1.2) | 24,152 (2.5) |
| `<ld::elf::Elf>::image_span_kind` (ld.so) | 13,461 (1.6) | 16,953 (1.5) | 15,465 (1.6) |
| `<ld::reloc::Loads>::inside` (ld.so) | 13,328 (1.6) | 14,204 (1.3) | 15,484 (1.6) |
| `niji_machine_equal` (ld.so) | 12,703 (1.5) | 12,892 (1.1) | 14,660 (1.5) |
| `ld::reloc::one::::{closure#1}` (ld.so) | 11,622 (1.4) | 12,207 (1.1) | 13,377 (1.4) |
| `b::ld::link::link` (ld.so) | 11,526 (1.4) | 28,471 (2.5) | 11,893 (1.2) |
| `ld::symbols::resolve` (ld.so) | 10,048 (1.2) | 10,504 (0.9) | 11,708 (1.2) |
| `b::bootstrap::relocate_self` (ld.so) | 9,224 (1.1) | 9,224 (0.8) | 9,224 (0.9) |
| `niji_machine_memmove` (ld.so) | 8,654 (1.0) | 9,992 (0.9) | 13,954 (1.4) |
| `<ld::elf::Versions>::def_name_from` (ld.so) | 7,336 (0.9) | 7,336 (0.6) | 14,742 (1.5) |
| `<ld::elf::Elf>::check_table` (ld.so) | 5,698 (0.7) | 12,868 (1.1) | 5,698 (0.6) |
| `ld::reloc::check_versions` (ld.so) | 5,291 (0.6) | 6,229 (0.5) | 11,947 (1.2) |
| `<ld::symbols::Object>::needed_version_name` (ld.so) | 888 (0.1) | 888 (0.1) | 19,800 (2.0) |

### Inclusive, every function over 1% (libniji, callgrind; cg3)

| function (object) | empty (% of 843,436) | 3libs (% of 1,134,201) | true (% of 980,484) |
|---|---|---|---|
| `niji_machine_ld_entry` (ld.so) | 843,436 (100.0) | 1,134,201 (100.0) | 980,484 (100.0) |
| `niji_ld_start` (ld.so) | 841,377 (99.8) | 1,131,799 (99.8) | 978,439 (99.8) |
| `b::ld::entry::run` (ld.so) | 819,587 (97.2) | 1,110,009 (97.9) | 956,649 (97.6) |
| `b::ld::link::link` (ld.so) | 818,389 (97.0) | 1,108,811 (97.8) | 955,503 (97.5) |
| `ld::reloc::relocate_first::` (ld.so) | 566,742 (67.2) | 702,082 (61.9) | 675,757 (68.9) |
| `ld::reloc::pass1::` (ld.so) | 564,500 (66.9) | 697,677 (61.5) | 673,487 (68.7) |
| `ld::reloc::one::` (ld.so) | 493,648 (58.5) | 611,122 (53.9) | 596,710 (60.9) |
| `ld::reloc::target` (ld.so) | 437,025 (51.8) | 551,649 (48.6) | 528,033 (53.9) |
| `ld::symbols::resolve` (ld.so) | 293,864 (34.8) | 403,463 (35.6) | 346,354 (35.3) |
| `ld::symbols::lookup` (ld.so) | 283,816 (33.6) | 392,959 (34.6) | 334,646 (34.1) |
| `<ld::symbols::Object>::lookup` (ld.so) | 242,185 (28.7) | 318,616 (28.1) | 286,222 (29.2) |
| `<ld::symbols::Object>::lookup::{closure#0}` (ld.so) | 149,407 (17.7) | 150,778 (13.3) | 178,835 (18.2) |
| `<ld::symbols::Object>::new` (ld.so) | 91,584 (10.9) | 115,556 (10.2) | 93,637 (9.6) |
| `<ld::elf::Elf>::read_phdr` (ld.so) | 82,779 (9.8) | 149,568 (13.2) | 85,116 (8.7) |
| `<ld::symbols::Object>::memo_name` (ld.so) | 65,374 (7.8) | 65,374 (5.8) | 105,118 (10.7) |
| `b::ld::image::image_view` (ld.so) | 42,435 (5.0) | 45,940 (4.1) | 44,465 (4.5) |
| `<ld::elf::Elf>::image_span_kind` (ld.so) | 31,173 (3.7) | 34,665 (3.1) | 33,177 (3.4) |
| `<ld::symbols::Object>::version_name` (ld.so) | 28,826 (3.4) | 28,826 (2.5) | 40,896 (4.2) |
| `<ld::elf::Dynamic>::once` (ld.so) | 28,449 (3.4) | 49,147 (4.3) | 30,667 (3.1) |
| `ld::reloc::one::::{closure#1}` (ld.so) | 24,734 (2.9) | 25,979 (2.3) | 28,469 (2.9) |
| `<ld::elf::Verdefs as core::iter::traits::iterator::Iterator>::next` (ld.so) | 21,791 (2.6) | 21,791 (1.9) | 37,124 (3.8) |
| `ld::search::search::` (ld.so) | 21,586 (2.6) | 58,657 (5.2) | 21,586 (2.2) |
| `<ld::elf::Dynamic>::gnu_hash` (ld.so) | 20,808 (2.5) | 36,632 (3.2) | 21,064 (2.1) |
| `ld::reloc::check_versions` (ld.so) | 20,041 (2.4) | 23,113 (2.0) | 43,311 (4.4) |
| `<ld::elf::Elf>::check_table` (ld.so) | 19,105 (2.3) | 42,880 (3.8) | 19,105 (1.9) |
| `b::ld::link::link::{closure#13}` (ld.so) | 18,748 (2.2) | 47,347 (4.2) | 18,748 (1.9) |
| `b::ld::image::open_candidate` (ld.so) | 18,524 (2.2) | 46,475 (4.1) | 18,524 (1.9) |
| `ld::dlopen::check::` (ld.so) | 18,120 (2.1) | 45,153 (4.0) | 18,120 (1.8) |
| `<ld::elf::Elf>::parse::` (ld.so) | 16,497 (2.0) | 43,992 (3.9) | 16,497 (1.7) |
| `b::bootstrap::relocate_image` (ld.so) | 16,250 (1.9) | 16,250 (1.4) | 16,250 (1.7) |
| `b::bootstrap::relocate_self` (ld.so) | 16,240 (1.9) | 16,240 (1.4) | 16,240 (1.7) |
| `<ld::reloc::Loads>::inside` (ld.so) | 13,328 (1.6) | 14,204 (1.3) | 15,484 (1.6) |
| `niji_machine_equal` (ld.so) | 12,785 (1.5) | 12,974 (1.1) | 14,742 (1.5) |
| `memcpy` (ld.so) | 11,325 (1.3) | 14,878 (1.3) | 16,769 (1.7) |
| `niji_machine_memmove` (ld.so) | 11,112 (1.3) | 14,581 (1.3) | 16,438 (1.7) |
| `<ld::elf::Elf>::bytes_from` (ld.so) | 10,788 (1.3) | 16,242 (1.4) | 10,788 (1.1) |
| `<ld::elf::Elf>::bytes_at` (ld.so) | 10,174 (1.2) | 18,281 (1.6) | 10,685 (1.1) |
| `<ld::elf::Elf>::dynamic` (ld.so) | 8,348 (1.0) | 19,743 (1.7) | 8,406 (0.9) |
| `<ld::elf::Versions>::def_name_from` (ld.so) | 7,336 (0.9) | 7,336 (0.6) | 14,742 (1.5) |
| `ld::reloc::publish_dynamic::<b::ld::image::ProcImage>` (ld.so) | 7,039 (0.8) | 13,341 (1.2) | 7,194 (0.7) |
| `<ld::symbols::Object>::needed_version_name` (ld.so) | 888 (0.1) | 888 (0.1) | 19,800 (2.0) |

## 3. The multiplier

| where | libniji starts per run | source |
|---|---|---|
| `cargo xtask test` pass 2 | 4,860 test processes (one per test, nextest) + nextest's list runs (2 per binary: 394-662) = **5,254-5,522**, plus processes the tests start (not counted) | a 2026-11-21 `cargo xtask test` log on the Zen 1 build host: 4,860 tests in 197 binaries, 714 s in a 4-core slot (~2,856 CPU-s) |
| one CI gate | pass 2 + 142 conformance programs + the start-up tests' repeated runs + product tracing: order 10^4 (not counted exactly) | the same log |
| dynamic workload kit, one variant | printf-loop 20,000 per run + ~56 for the rest; each workload runs ~8 times per variant (warm-up, 5 counted, output check, strace) = **~160,000** | validation/wl_workloads.sh, wl_run.sh |

**Seconds (ips2, Zen 1, loaded build host; an estimate, not a timing verdict).** CPU time per start is about 2 ms
for both libraries (libniji 1.85-2.11 ms, glibc 2.25-2.40 ms over 3 interleaved rounds). The kernel dominates it:
about 1.8M kernel cycles per start (fork, exec, mapping, faults, exit). libniji's extra 643k user instructions
cost +296k user cycles (0.46 cycles per instruction: the loader's loops run at high IPC), about 0.1 ms per start.
So **each 100k instructions per start is about 46k cycles, 15-23 us at 2.0-3.0 GHz.**

| per run | each 100k instructions per start | the whole current surplus (643k per start) |
|---|---|---|
| pass 2 | 0.08-0.12 CPU-s (of ~2,856) | 0.5-0.8 CPU-s (~0.03%); test binaries' own starts are larger, not measured (x3: still ~0.1%) |
| dynamic kit, one variant | 2.4-3.7 CPU-s | 15-24 CPU-s; printf-loop's instruction verdict (3.63x, held row) is 100% start-up |
| a system (Yohane), per 10,000 process starts | 0.15-0.23 s | 1.0-1.5 s, about +5-7% of each tiny process's CPU |

Conclusion: start-up is **not** a significant cost of our own gate (challenge #7390, agreed by the coordinator).
It dominates process-heavy work: the workload kit's printf-loop row, shell scripts, `configure`, package builds.
Yohane's usage pass (per-program processes and CPU) had not landed when this ran. It will put real numbers on the
last row.

## 4. Kernel side per start

proto1, host, kit form; cg3 in the root:

| | static empty main (floor: exec and exit) | glibc 2.39 dynamic | libniji dynamic |
|---|---|---|---|
| kernel instructions | 298,579 | 649,293 | 758,398 (+109k, +17%) |
| page faults | 26 | 50 | 59 (+9; root: 63 vs 49) |
| system calls (root, strace) | - | 29 (mmap 7, openat 3, mprotect 3, pread64 2, newfstatat 2, read, prlimit64, brk, access, ...) | 26 (mmap 11, openat 2, mprotect 2, arch_prctl 2, uname, munmap, fstat, close, ...) |

libniji makes fewer system calls but 4 more mappings, and that is where its extra kernel work comes from. The
start reserves 64 MiB of PROT_NONE address space (the path store and views). It maps the whole of libc.so.6
read-only (4.7 MB, the view the loader parses) before mapping libc's segments. It also maps three anonymous
areas: 16 KiB scratch, 70 KiB thread block and TLS, 68 KiB.

The extra faults come from three places: pages of that view touched by header and table reads, relocated data
pages (1,573 RELATIVE plus 288 symbolic entries spread over libc's writable pages), and the separate thread area.
The dynamic machinery as a whole costs 2.2x the kernel floor in the reference and 2.5x in libniji.

## 5. Structural candidates, phase by phase

| # | phase | structural reason | saving (empty main, native) | evidence | cost, risk | touches | security rule |
|---|---|---|---|---|---|---|---|
| C1 | symbolic relocations | libc.so.6 references 277 of its own exported names through its GOT/PLT, so every start resolves them through the whole scope, though nothing but an interposer can change the answer | **-413,603** (prototype "keep"); shippable variant ~-370k (data objects stay symbolic: ~25 GLOB_DAT) | proto1 | M; **ABI**: which libc-internal calls a program's own definition reaches | libc.so.6 link only (`-Bsymbolic-functions` semantics plus a dynamic list keeping the malloc family preemptible), no loader change | none dropped |
| C1b | same | alternative without an ABI change: the build records libc's self-bindings; at start the loader checks only whether an object ahead of libc defines one of those names, then applies the recorded values like RELATIVE entries | ~-400k before C2, ~-35k after C2 | estimate | M-L; correctness of the interposer check (versions, weak definitions, COPY, preloads) | loader + build | the recorded table is validated once at load, inside libc.so.6 (same trust as its relocations) |
| C2 | symbol lookup | each lookup re-reads through checked readers, rebuilds names and versions, and walks every earlier object's chain with no bloom reject | (1,373 - 600) x lookups: ~-230k before C1; after C1 ~-25k on the empty main, ~-50k on `/bin/true`, ~-200-300k on a program with 300 imports (Rust test binaries) | cg3, proto1 (per-lookup marginal) | M; hot loader code | crates/ld symbols.rs, reloc.rs | bounds checked once when the object's tables are validated (`Object::new`), then indexing within validated ranges: a check moves, none is dropped |
| C3 | finding, checking, mapping; per-object tables | program headers decoded on every walk (~550 times); dynamic tags by linear rescans; the whole-file check, parse and table check are separate passes over the same bytes; per-object set-up ~30k for libc | ~-100k; per-object marginal 94k toward ~20k | cg3 | M | crates/ld elf.rs, symbols.rs, dlopen.rs check, boundary image.rs | one validating pass per object (fold, never drop) |
| C4 | RELATIVE relocations, self-relocation | 1,573 RELA entries in libc.so.6 at ~30 instructions each; ld.so's 232 words at ~70 | ~-55k | cg3; DYN-START-LINK-SHAPE option (a) | S-M; RELR for libc.so.6 is a product decision (ruling pending since DYN-START-PARITY) | libc link + reloc.rs RELR loop + boot.rs | none |
| C5 | version requirements | names compared by walking version definitions; the hash stored in the structures is not used first | ~-15k (/bin/true ~-35k) | cg3 | S | reloc.rs check_versions, elf.rs | none |
| C6 | kernel side | whole-file view and the 64 MiB reservation on the start path; extra anonymous areas | +109k kernel instructions and +9 faults to recover | proto1, cg3 strace | M; layout coupling (fault bounds) | boundary image.rs, link.rs, thread.rs | headers read by `pread` into the start arena are checked as now |
| - | libc init, start, main, exit; TLS; constructors | already at parity (2.7k vs 2.1k; TLS < 1k) | - | cg3 | - | - | - |

**Interposition semantics (the C1 ruling).** Measured with a black-box probe (iprobe.c): a program defining the
malloc family and `open`/`openat`, then calling `strdup` and `fopen`.

* Today: libc's `strdup` reaches the program's `malloc`; `fopen` never reaches the program's `open`/`openat`.
  The reference behaves the same (its `fopen` also allocates through the program's `malloc`).
* "all" (every self-reference bound) **loses malloc replacement** (strdup -> malloc: 0). Not acceptable.
* "keep" (the malloc family left preemptible: malloc, free, calloc, realloc, aligned_alloc, posix_memalign are
  the six libc references) keeps both behaviours.

What programs legitimately interpose:

* The malloc family, documented in the GNU C Library manual ("Replacing malloc"): it must stay preemptible.
* Exported data that programs reach by COPY relocation: `stdin`, `stdout`, `stderr`, `optarg`, `optind`, `opterr`,
  `optopt`, `program_invocation_name`, `program_invocation_short_name`, the `_IO_2_1_*` objects, `re_syntax_options`,
  `obstack_*` handlers. These must stay symbolic. The prototype bound them, so a shippable link binds functions
  only.
* Everything else (functions): preload tools (sanitizers, fakeroot, time fakers) intercept the calls programs make,
  and those still go through the program's own relocations under C1. What changes is only whether libc's *own*
  internal calls reach them.

The bound function families are pthread (92), aio (15), sem (11), mq (10), timer and clock (10), C11 threads
(12), dl* (8), ns* (7) and ~40 others. For each family, a probe of the reference settles whether its internal
callers reach a program's definition. Any family where the reference does is kept preemptible (a dynamic list
entry), so the behaviour stays the reference's.

## 6. Plan

Ranked, one structural change each. Briefs: handoff/STARTUP-STRUCTURE-REVIEW/<ID>.md. Savings are on the native
empty main of the root form (752,610) unless stated; "must prove" is the floor each story's count must show.

| rank | story | size | must prove | depends |
|---|---|---|---|---|
| 1 | LIBC-SELF-BIND (C1; ruling needed) | M | empty main <= 400k (from 752k), 3 libraries <= 560k, malloc and COPY-data interposition probes unchanged, per-family reference probes recorded | ruling |
| 2 | LD-LOOKUP-COST (C2) | M | per-lookup marginal <= 600 (200-import probe), stretch 300; `/bin/true` -45k after rank 1 | - |
| 3 | LD-PHDR-ONCE (C3, headers) | S-M | `read_phdr`-class work <= 10k on the empty main (from 82,779); 3-library marginal -20k per object | - |
| 4 | LD-OBJECT-ONCE (C3, tables, check folding) | M | per-object marginal <= 25k (from 93,819); empty main -60k | 3 |
| 5 | LD-RELR-LOOP (C4) | S-M | RELATIVE work plus self-relocation <= 15k (from ~89k); .rela.dyn -36 KB | RELR ruling |
| 6 | LD-VERSIONS-ONCE (C5) | S | `check_versions` <= 5k empty main, <= 10k `/bin/true` | - |
| 7 | LD-START-MAPS (C6) | M | kernel instructions and faults per start <= the reference's (649k / 50 on the host kit) | layout owners |

**Realistic target after ranks 1-6: about 180k user instructions** for an empty dynamic main, ±30k (estimate;
only rank 1 is prototyped). That is ~4x better than today and ~1.8x the reference's 96-100k. What remains:

| remaining | about |
|---|---|
| per-object set-up for program and libc | 2 x ~20k |
| mapping, search and check | ~40k |
| loader bookkeeping | ~20k |
| RELATIVE | ~10k |
| the remaining symbolic lookups | ~30 x 600 |
| entry | 5.5k |
| relro and published dynamic | ~10k |
| libc's own code | 2.7k |

**Parity needs more than the plan:**

* a per-object cost at the reference's ~12k: a different per-object design, not a tune-up of this one;
* loader bookkeeping halved;
* C1b's recorded bindings extended into a validated build- or install-time summary per object (decoded headers,
  dynamic tags, hash parameters, self-bindings), checked against the file's identity and build-id at load.

That last item is a loader cache with "never serve a stale result" as its first test (SLOW AND WORKING WINS). It
would go below the reference, and it needs its own RETHINK and a ruling.

## 7. Findings outside the plan

* libniji's system `libc6-dev` (21.5 and 21.6) ships no `.h` files. Swapping it into a root removes
  `/usr/include/stdio.h`, and the root's gcc can no longer compile `#include <stdio.h>`. For the packaging owners.
* libc.so.6 is pinned (R14-3): a program with `DT_RUNPATH` to a products directory does not find libc.so.6 there;
  the loader tried only `/lib/libc.so.6`. By design. Measurement kits use `LD_LIBRARY_PATH` or a swapped root.
* callgrind aborts on 21.5 (`find_rodata` assertion), as VALGRIND-DBG-RODATA found; 21.6 works.
* A static glibc empty main runs 133,200 user instructions, more than its dynamic one (99,262).
