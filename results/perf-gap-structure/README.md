# Where libniji is slower than the reference, and why, by structure (PERF-GAP-STRUCTURE-AUDIT, 2026-11-23)

Audit, no product change. Products: libniji release products of `next` 0495c4684 (the base of this branch), the
dynamic kit (libniji's `ld.so` and `libc.so.6`) and the static kit (`libc.a`); the reference is the installed glibc
2.39, measured as a black box only (counters, faults by mapping, syscalls, `LD_DEBUG=statistics`, program headers;
no symbol of the reference was sampled or shown, no source read). Host: the Skylake-SP build host (Xeon Platinum
8151, 48 threads, a virtual machine). Every number is a `perf stat` counter (median of 7 to 21 runs unless marked),
an exact single-step count, or labelled **guess**/**estimate**. Cycles are counters on an unbooked VM: advisory,
used only where instructions mislead. Probes and job labels: section 8.

## 0. The answer

1. **libniji is not slower in general.** Per call it is at or ahead of the reference on most rows (malloc and free,
   printf, fprintf, strtol, mbrtowc, iswalpha); a static program starts with 2,974 user instructions and 9 faults
   against 155,703 and 27. The gaps users feel are narrow and structural:
   * **dynamic start-up**, paid by every process of a distribution: total cycles (user + kernel) 0.92 to 1.26 of
     the reference on nine distribution binaries, page faults 1.11 to 1.40 in every one (section 2.2);
   * **thread creation**: 36,015 against 2,265 user instructions per `pthread_create` + join;
   * **`strcoll` in a UTF-8 locale**: 1.9 times the instructions, 2.8 times the cycles; `towupper` 1.65 times;
   * **first use** of stdio and of threads: more faults than the reference's first use.
2. **The dynamic start-up gap is loader work and written pages, not code placement.** libniji's loader spends
   **1,510 user instructions per imported symbol** (the reference 643 when binding eagerly, 37 when lazily); a
   process's gap is about 55k fixed + about 870 per symbolic relocation of the program and its libraries (fits
   `/bin/true` to 0.1%, `ls` to 17%). Its read faults on file mappings (text, rodata, headers) are already
   *fewer* than the reference's (19 against 28; resident file pages 700 against 1,128 KiB): the layout machinery
   works. The extra faults are
   **written pages**: anonymous memory, stack and writable data (in `ls`: +28 anonymous, +10 stack, +6 writable
   data, +7 text), which placement cannot remove.
3. **The metric hid the kernel.** Kernel cycles are about 80% of a dynamic start-up's cycles. libniji's +41% user
   instructions cost +2.6% user cycles; its +8.8% total cycles is almost all kernel-side, and the larger part of
   that kernel excess (about +160k kernel instructions per process) is **not attributed** by this audit.
4. **Parity:** by estimate, the paths in section 5 bring user instructions to or below the reference and faults
   close to it. They do **not** reach demonstrated parity in total cycles, because the kernel residual has no
   attribution yet. That is the finding; section 6 names the measurement that settles it.

## 1. How it was measured

* Four builds of each probe: reference dynamic (gcc), reference static (`gcc -static`), libniji dynamic
  (`validation/wl_dyncc.sh`), libniji static (`scripts/niji-cc`). Distribution binaries (built for the
  reference) run under each loader invoked the same way: `<ld.so> --library-path <dirs> <program>`; outputs
  compared (all identical).
* Per call: (count(N) - count(0)) / N, median of 3. Start-up: median of 21. Counters: `instructions:u`,
  `instructions:k`, `cycles:u`, `cycles:k`, `page-faults`, `mem_inst_retired.lock_loads:u` (atomics),
  `branch-misses:u`.
* Attribution inside libniji: an exact gdb single-step of the whole dynamic start-up (`probes/trace_start.py`,
  exclusive and inclusive per function via a shadow call stack; 158,497 steps against perf's 159,940:
  within 1%). **Sampled `perf record` profiles of start-up are not usable on these VMs:** throttling keeps the
  first samples of each run, so the loader's environment scan showed 15.8% sampled, against about 4% expected
  from the measured slope with that environment (29 variables, 875 bytes) and under 0.3% in the exact trace with
  two variables.
  Per-call sampled profiles are used only where the run is long and single-process.
* Faults: `page-faults` samples with the data address, classified by the mapping (and permissions) that
  contains it; for libniji also the symbol at the address and, in probe 8, the DWARF call chain.

## 2. The gaps (measured)

### 2.1 Start-up, minimal programs

User instructions / kernel instructions / page faults (median of 21; the job's environment, 29 variables):

| Program | reference dynamic | reference static | libniji dynamic | libniji static |
|---|---:|---:|---:|---:|
| empty `main` | 119,980 / 623,882 / 53 | 155,703 / 309,201 / 27 | 169,209 / 742,857 / 58 | **2,974** / 164,454 / **9** |
| one `fopen` | 123,827 / 667,866 / 57 | 156,471 / 331,441 / 28 | 175,821 / 866,919 / 69 | 5,227 / 289,650 / 19 |
| one `printf` | 123,992 / 699,312 / 61 | 157,161 / 329,564 / 29 | 177,816 / 843,259 / 68 | 7,044 / 276,371 / 17 |
| `setlocale(LC_ALL, "")` + `printf`, `LANG=C.UTF-8` | 305,511 / 1,104,672 / 80 | 335,958 / 761,213 / 47 | **202,043** / 890,257 / 74 | 29,981 / 337,265 / 30 |

Cycles, user + kernel (mean of 21): empty dynamic 199,511 + 878,696 = 1,078,207 (reference) against 204,646 +
968,620 = 1,173,266 (libniji, **+8.8%**); one `printf` dynamic +9.9%; static empty 587,115 against 251,042 (0.43).

Environment size (two-variable environment plus N variables of about 100 bytes): libniji dynamic adds about 852
user instructions per variable, the reference about 610 (its static build about 634, libniji's static 31). With
two variables the gap is already 54,659 (105,281 against 159,940), so the environment scan is a minor part.

### 2.2 Distribution binaries (built for the reference, both loaders, `LANG=C.UTF-8`, median of 7)

| Program (libraries needed) | user instr. libniji / ref | kernel instr. | faults | cycles user + kernel |
|---|---:|---:|---:|---:|
| `true` (1) | 226,821 / 135,050 = **1.68** | 1.34 | 61 / 52 = 1.17 | **1.22** |
| `cat` (1) | 0.89 | 0.99 | 90 / 79 = 1.14 | 0.92 |
| `ls /` (2) | 823,759 / 561,371 = **1.47** | 1.27 | 141 / 101 = **1.40** | **1.20** |
| `sort` (2) | 1.37 | 1.17 | 230 / 197 = 1.17 | 1.13 |
| `date` (1) | 0.94 | 1.05 | 98 / 81 = 1.21 | 1.02 |
| `wc` (1) | 0.93 | 1.06 | 98 / 86 = 1.14 | 1.05 |
| `sed` (3) | 1.34 | 1.36 | 154 / 111 = **1.39** | **1.26** |
| `grep` (2) | 1.12 | 1.06 | 132 / 100 = 1.32 | 1.08 |
| `printf` (1) | 0.88 | 0.99 | 88 / 79 = 1.11 | 0.96 |

The programs that call `setlocale` and need only libc gain from libniji's cheaper locale start-up (no locale file
is mapped: 0 faults on locale files in `ls` against 14 for the reference); the gap grows with the number of
libraries and imported symbols. All these binaries and their libraries are linked `BIND_NOW`.

### 2.3 Per call (dynamic; user instructions reference / libniji; user cycles reference / libniji)

| Row | instructions | cycles | note |
|---|---:|---:|---|
| `malloc`+`free` 64 B | 141 / 116 | 83 / 64 | no atomics on either side |
| `malloc`+`free` 4 KiB | 378 / 124 | 223 / 77 | |
| `snprintf("%d")` | 537 / 449 | 311 / 226 | |
| `snprintf` mixed | 1,247 / 1,232 | 837 / 797 | |
| `fprintf` to /dev/null | 643 / 593 | 393 / 241 | |
| `fopen`+`fclose` | 885 / 751 | 930 / 883 | libniji 6 locked operations per pair, the reference 0; kernel +474 |
| `strlen` 64 | 32 / 34 | 15.4 / 20.3 | |
| `memcpy` 64 | 28 / 27 | 14.3 / 14.1 | |
| `memcmp` 64 | 26 / 29 | 17.7 / 15.5 | |
| `strcmp` 64 | 38 / **54** | 24.7 / 25.7 | |
| `strchr` miss 64 | 43 / **64** | 29.0 / 33.1 | |
| `strtol` | 216 / 151 | 165 / 119 | |
| `toupper` | 14 / 16 | 8.1 / 11.3 | |
| `iswalpha` (en_US.UTF-8) | 49 / 30 | 27.8 / 28.5 | |
| `towupper` (en_US.UTF-8) | 40 / **66** | 25.1 / **46.8** | |
| `strcoll` (en_US.UTF-8, 11 bytes) | 5,362 / **10,354** | 2,900 / **8,010** | |
| `mbrtowc` UTF-8 | 249 / 230 | 167 / 136 | |
| `pthread_create`+join | 2,265 / **36,015** | 4,782 / **32,201** | kernel 86.6k / 78.6k |
| first thread (N = 1) | 4,048 / 19,006 | | faults +10 / +15 dynamic, +2 / +18 static |

Dynamic against static costs libniji 2 to 6 instructions per call (the reference 0 to 34). The per-call cycle
column is noisy on this VM (the reference's static and dynamic `snprintf` differ by 140 cycles with the same
code); it is shown because it disagrees with instructions on `strcoll` and `towupper` in the same direction.

## 3. Attribution per gap

### 3.1 Dynamic start-up, user instructions: the loader's per-symbol cost

Exact split of libniji's dynamic empty `main` (158,497 instructions; two-variable environment): `ld.so`
155,917 (98.4%), `libc.so.6` 2,525 (its whole initialisation and exit), the program 55. Inclusive, from the
loader's entry:

| Phase (inclusive) | instructions | share |
|---|---:|---:|
| first relocation pass (`reloc::relocate_first` / `pass1`) | 89,958 | 56.8% |
| of it: symbolic relocations (`reloc::one` -> `target` -> `symbols::resolve` / `lookup`, version origin and names) | 63,536 | 40.1% |
| of it: RELR words and table walk (`pass1` exclusive) | 16,386 | 10.3% |
| per-object tables (`Object::new`: `Dynamic::once` 9,577, `versym_versions` 5,842, `memo_fill` 4,201) | 20,957 | 13.2% |
| ELF views and checks (`image_view` 7,515, `image_span_kind_with` 4,332, `check_table` 3,272, `Elf::parse` 2,454, `Elf::dynamic` 2,382, `bytes_at`/`bytes_from` 2,005) | about 22,000 | about 14% |
| self-relocation, publishing the dynamic array, debugger rendezvous, init hooks | about 9,000 | about 6% |

The symbolic relocations are few: `libc.so.6` has 47 (40 `GLOB_DAT`, 3 `R_X86_64_64`, 4 `JUMP_SLOT`, all to
symbols it defines itself) plus the program's handful; its relative relocations are 1,509 words on 12 pages and
`ld.so`'s 233 words on 2 pages. **Relocation volume is no longer the cost** (the LD-RELR-LOOP chain's premise);
the cost per symbol is. The reference's loader reports 95 relocations (7 from its cache) in 4,215 of 116,936
loader cycles (`LD_DEBUG=statistics`).

Per imported function never called (programs with 0, 50 and 200 PLT imports; per import = (200 - 0) / 200):

| Loader | linked lazily | linked `-z now` | user cycles per import (`-z now`) |
|---|---:|---:|---:|
| reference | 36.8 | 642.7 | 439 |
| libniji | 1,512.8 | 1,509.3 | 1,037 |

libniji binds every import at start-up whatever the link mode, and its lookup costs 2.35 times the reference's
eager lookup. The code path (`crates/ld/src/reloc.rs` `target`, `crates/ld/src/symbols.rs` `Object::lookup`):
each lookup re-reads symbol, string and version entries through validated, `Result`-returning accessors,
derives the wanted version per reference (`reference_want_origin`, a memo), compares version names as strings
when the origin short cut misses, and walks the global scope object by object.

Model: gap = about 55k (the fixed loader work above) + about 870 per symbolic relocation of the program and its
libraries (1,510 - 643). `/bin/true` (43 imports): predicted +92k, measured +91.8k. `ls` (118 + libselinux 146 +
libpcre2 26 = 290): predicted +306k, measured +262k (17% over). The distribution binaries are all `BIND_NOW`,
so lazy binding would not help them; a cheaper lookup would.

### 3.2 Dynamic start-up, faults and kernel: written pages, and an unattributed residual

Faults of the dynamic empty `main` by the mapping of the faulting address:

| Kind | reference | libniji |
|---|---:|---:|
| read faults on file mappings (text, rodata, headers, `ld.so.cache`, vdso) | 28 (libc text 15) | **19** (libc text 5, `ld.so` text 5) |
| writes to file-backed writable segments (copy-on-write) | 14 (libc 7, `ld.so` 4, program 3) | **19** (libc 12, `ld.so` 4, program 3) |
| zero-fill: anonymous + stack | 9 + 3 = 12 | 11 + **8** = 19 |

At the end of `main`: resident 1,216 KiB (file 1,128, anonymous 88) for the reference against 844 KiB (file 700,
anonymous 144) for libniji; stack 12 against 40 KiB; page tables (`VmPTE`) 40 against 52 KiB; address space
2,560 against 6,780 KiB. libniji maps *fewer* file pages; it *writes* more pages.

The same comparison on `ls /` (probe 7): the system libraries libselinux and libpcre2 are the same files under
both loaders and take the **same 13 faults**; the difference is libniji's own pages: anonymous 23 to 51, stack 2
to 12, libc writable data 7 to 13, libc text 17 to 24 (the reference instead takes 14 faults on 13 locale and
conversion-cache files that it maps). By call
chain (probe 8, libniji only): of 32 anonymous faults, 9 are written by the heap (`niji_rt::heap`), 4 by the
runtime under the program, 2 by stdio, 2 by `memcpy` into fresh pages, 3 by the loader; 9 of 11 stack faults have
no unwindable user frame. The loader's deepest functions (`niji_ld_start`, `ld::entry::run`, `ld::link::link`,
`ld::image::read_cache`, the `dlopen` paths) each have frames above 4 KiB (stack probes in their prologues).

Kernel unit costs on this host (probe 4, a libniji static program; the kernel's cost does not depend on the C
library): zero-fill fault 4,538 instructions / 7,205 cycles; file read fault with fault-around 13,031 / 16,564;
copy-on-write fault 5,104 / 9,207; `mmap` + `munmap` 12,527 / 12,006. Priced with these (an **estimate**: start-up
faults also allocate page-table pages), libniji's fault mix costs about 60k kernel instructions *less* than the
reference's, yet its measured kernel total is +119k (+90k cycles). Syscalls: 26 against 30 (`mmap` 11 against
8, about +19k at unit cost). **About +160k kernel instructions per dynamic process remain unattributed.**

### 3.3 Thread creation

36,015 against 2,265 user instructions per create + join (32.2k against 4.8k cycles); the sampled profile is
`thread::finish` 55% and `thread::create_inner` 44%. This confirms DECISION-EVIDENCE-AUDIT's attribution on a
second CPU class: the 4,096-entry `KeyTable::span` scan (`crates/rt/src/thread.rs`, `KEYS = 4096`, D-22.5) runs at
every cached-stack reuse and every exit, about 34,700 instructions per pair. Kernel work per pair is lower than the
reference's (78.6k against 86.6k: `clone` with fewer `rt_sigprocmask` calls, no `madvise`). The first thread costs
libniji +15 faults dynamic (+8 anonymous, +7 text) and +18 static (+9 text, +7 anonymous) against the reference's
+10 and +2: the thread-creation code is not in the start-up text windows, and a thread's own area touches more
pages (DECISION-EVIDENCE-AUDIT items 2 and 4: 64 KiB static-TLS surplus re-zeroed on each cached-stack reuse,
139,200 bytes between the thread pointer and the mapping's end).

### 3.4 Locale paths

`strcoll` (en_US.UTF-8): `locale::collate` `Key::raw_next` 42%, `Memo`/`Source::at` 29%, `Locale::compare_bytes`
10%, `Text::element` 7% (sampled, one long single-process run). The per-character element iteration through the
memo/source abstraction holds about 70% of the call. `towupper`: 66 against 40 instructions, 46.8 against 25.1
cycles; not profiled further (a per-call trace with `validation/wl_trace.sh` is the next step). `setlocale` and
`iswalpha` are faster than the reference.

### 3.5 Smaller per-call gaps

`fopen` + `fclose`: 6 atomic operations per pair in a single-threaded process (the reference: 0) and +474 kernel
instructions; libniji still uses fewer cycles. `strcmp`/`strchr` on 64 bytes: +16 and +21 instructions at
near-parity cycles (instruction choice inside the kernels; register 03 and 13 to 15 own these rows).

### 3.6 malloc and free

No gap on this CPU: fewer instructions and cycles at both sizes, no atomics. `docs/benchmarks.md` records time
ratios 1.11 to 1.18 for `malloc_free_*` on Ivy Bridge at 0.83 instructions: **unknown** here (section 6). The heap
writes 9 fresh anonymous pages in `ls` (section 3.2) and the process maps 4.2 MiB more address space.

## 4. The assumptions, with verdicts

| # | Assumption | Verdict | Evidence |
|---|---|---|---|
| A1 | Start-up cost is best attacked by placing code and data | **Fails** for what users run | Static start-up is already 52 times cheaper in instructions and 3 times in faults than the reference's: nothing left to place. Dynamic: read faults are already fewer (19 against 28); the excess is written pages (+7 zero-fill, +5 copy-on-write in the empty program; +28 anonymous, +10 stack in `ls`) and loader work (1,510 per symbol). |
| A2 | Rust abstractions and the boundary layer cost nothing on hot paths | **Holds** in libc's per-call paths; **fails** in the loader | Per call: dynamic adds 2 to 6 instructions; malloc, printf and strtol are below the reference. Loader: validated accessors and version-name matching per lookup (1,510 against 643 per symbol); frames above 4 KiB in its deepest functions (stack 40 against 12 KiB). |
| A3 | Lazy initialisation everywhere is free | **Fails** for faults, holds for instructions | First `printf` +8 faults static (reference +2); first thread +18 static, +15 dynamic (reference +2, +10). The loader is the opposite case: fully eager (1,513 per never-called import even under `-z lazy`), which costs nothing extra on the distribution's `BIND_NOW` binaries. |
| A4 | The allocator design | **Holds** on Skylake-SP; **unknown** on Ivy Bridge time | 116 against 141 instructions, 64 against 83 cycles, no atomics; the Ivy Bridge time ratios 1.11 to 1.18 are unexplained. Cost: 9 anonymous pages written at first use in `ls`. |
| A5 | Per-thread structures | **Fails** | 34,700 instructions per create + join in the key scan; 64 KiB surplus re-zeroed per reuse; first thread +7 to +8 anonymous pages. |
| A6 | User instructions are a sufficient working metric (added) | **Fails** for start-up, roughly holds per call | Kernel cycles are about 80% of a dynamic start-up; +41% user instructions = +2.6% user cycles. Per call, `strcoll` 1.9 times instructions but 2.8 times cycles. |
| A7 | Relocation volume drives the loader's cost (added; the LD-RELR-LOOP premise) | **Fails** now | 1,742 relative words cost about 16k; symbolic lookups for about 55 symbols cost 63.5k. |
| A8 | Static-link layout contracts protect what users run (added) | **Fails** | A distribution runs dynamic `BIND_NOW` binaries; the static path has no gap to the reference, the dynamic path has the gaps above. |
| A9 | The environment scan is a large start-up cost (from the sampled profile) (added) | **Fails** | About 4% with a 29-variable environment (slope 852 against the reference's 610 instructions per 100-byte variable); under 0.3% exact with two variables. |

## 5. Paths to parity, ranked by user impact over effort

| Rank | Gap | Structural change | Why it closes the gap | Estimate (basis) | Risk |
|---|---|---|---|---|---|
| 1 | Thread create | High-water mark of created keys so `KeyTable::span` is O(1) (DECISION-EVIDENCE-AUDIT's proposal); then zero only the used part of the static-TLS surplus on reuse | Removes the 4,096-word scan twice per pair | 36,015 to about 1,300 per pair (projection from the profile; reference 2,265) | Low: internal; keys beyond the reference's 1,024 stay allowed |
| 2 | Dynamic start-up, per symbol | Loader lookup rebuilt around validate-once tables: validate each object's symbol, string, hash and version tables once at map time, then read them without per-access checks; map every needed version to the defining object's version index once per object pair (an integer compare per symbol); keep the hash/bloom rejection for objects that do not define the name | The 1,510 instructions per symbol are accessor checks, version-name matching and scope walking (section 3.1) | Target 300 per symbol (**guess**: a hash probe, a few compares and a store). Empty `main` about -50k (`libc.so.6`'s 47 plus the program's symbols at about 850 to 1,200 each), `/bin/true` about -90k (43 + 47 symbols), `ls` about -390k (290 + 47): user instructions at or near the reference's on all nine binaries (`/bin/true` about 137k against 135k before path 3) | Medium: loader correctness (interposition order, versions, `STB_GNU_UNIQUE`, copy relocations); the "loaded data is untrusted" rule is kept by validating once at map time |
| 3 | Dynamic start-up, fixed work | Build per-object tables on first lookup in that object, not at load; one validation pass per table instead of views per access | `Object::new` 21k and views/checks about 22k for three objects | -15k to -25k per process, more with more libraries (**estimate** from the exact split) | Low to medium |
| 4 | Faults, written pages | (a) shrink the loader's >4 KiB frames (move buffers into memory the loader already touched); (b) the heap's first-use metadata on fewer pages; (c) start-up-written data of `libc.so.6` on fewer pages (12 copy-on-write faults against 7) | Written pages fault one by one (no fault-around); each costs 4.5k to 5.1k kernel instructions, 7k to 9k cycles | -5 stack, -3 to -5 writable data, -3 to -6 heap faults per process (**estimate**); `ls` from 141 to about 125 (reference 101) | Low; the start-up layout checks must follow (§9) |
| 5 | Kernel residual | First measure (section 6), then act | About +160k kernel instructions per process are unexplained | Unknown | - |
| 6 | `strcoll` UTF-8 | Collation without the per-character memo/source iteration: direct weight lookup per character with a fast path for equal runs, levels compared only past a first-level tie | 70% of the call is in the element iteration | Parity needs about half the instructions (**guess** that the iteration overhead is most of the 2x) | Medium: collation order must stay bit-identical (existing tests) |
| 7 | `towupper` | Profile first (`wl_trace.sh`), then a table shape with one lookup per call | 26 extra instructions per call | Parity (**guess**) | Low |
| 8 | `fopen`/`fclose` | Skip the 6 atomic operations while the process has one thread | Locked operations cost cycles, not instructions | Cycles only (small) | Low; the switch to threaded must be correct |

**Combined, plainly.** Paths 1 to 4 reach parity or better on user instructions (estimate) and come close on
faults (`ls` still about +24 faults after path 4: the rest of the anonymous pages are not attributed). Path 5 is
unknown. **The combined paths do not demonstrate glibc parity in total cycles for dynamic start-up**, because
roughly two thirds of the remaining kernel-side cycle excess has no cause yet. Thread creation reaches parity by
path 1 alone; `strcoll` and `towupper` need their own redesigns (6, 7).

## 6. Not settled here: cheapest data and decision rules

* **Kernel residual** (+160k kernel instructions per dynamic process). Cheapest: a kernel-symbol profile of the
  dynamic empty `main` under both loaders (`perf record -e instructions:k` with a readable kallsyms; needs
  read-only root on one build host). Rule: if VMA/mapping work dominates, map each object into one reservation
  with fewer `mmap` calls; if fault handling dominates, it is path 4; if exit-time teardown dominates, shrink the
  reservations (address space 6,780 against 2,560 KiB).
* **Ivy Bridge malloc time** (1.11 to 1.18 time at 0.83 instructions). Cheapest: `cycles:u` per call for
  `malloc_free_64` on a booked Ivy Bridge host. Rule: above 1.05, profile with cycle events there (stalls, not
  instructions).
* **The 9 unattributed stack faults** of `ls`. Cheapest: frame-size listing per function on the loader's start-up
  call path (`objdump` of libniji's own `ld.so`), compared with the exact trace's call stack. Rule: any frame above
  1 KiB on that path moves its buffer out.
* **The CI bound `init_dynamic_empty_main_stays_within_its_instruction_bound`** (red in 11 stories in 3 days, a few
  hundred instructions each). Its harness clears the environment, so the environment is not the cause. **Guess:**
  the count moves with table contents (each symbolic relocation added to `libc.so.6` costs about 1,150, each RELR
  word about 9, changed hash chains move the 55 lookups). Cheapest: for the 11 reds, the change in `libc.so.6`'s
  symbolic-relocation and RELR counts between base and head. Rule: if more than half are explained, bound the
  loader per phase (per symbol, per RELR word) instead of per process.

## 7. Cross-references

* **DECISION-EVIDENCE-AUDIT** (`results/decision-audit/README.md` on its branch): item 1 (`KEYS = 4096`)
  reproduced on Skylake-SP (section 3.3); item 8 ("dynamic start-up unexplained", +62k instructions, +19 faults)
  is explained here: loader per-symbol cost and fixed loader work for the instructions (3.1), written pages for
  the faults (3.2), with the kernel residual still open; item 6 (address space +8.5 MiB) is consistent (6,780
  against 2,560 KiB for an empty program).
* **TEST-BOUNDS-AUDIT**: no results file or branch existed when this audit was read (16:58Z); the bound question
  is in section 6.
* `handoff/STARTUP-REPROFILE/phase-release-skylake-sp.md` (an earlier phase table, before LD-OBJECT-ONCE and RELR)
  and `handoff/LD-RELR-LOOP-report.md`: the same loader phases; this audit's split is of `next` 0495c4684.

## 8. Reproduce

Probes in `probes/` (run from a directory holding them, on a build host with the base's products primed under
`~/wl/products/<sha>-release{,-dyn}` and a role tree for the link drivers): `run1.py` (start-up and per-call
counts), `run2.py` (relocation census, syscalls, `LD_DEBUG=statistics`, per-call libniji profiles), `run3.py`
(environment slope, resident pages, exact start-up trace through `trace_start.py`), `run4.py` (kernel unit costs
with `kunit.c`, faults by mapping, loader frame sizes), `run5.py` (per-import cost; distribution binaries; the
end-of-`main` status), `run7.py` (`ls` faults under each loader), `run8.py` (who writes libniji's anonymous and
stack pages). Job labels (jobrun kind `probe`): `perf-gap-structure-audit-probe1` to `-probe8` (probe 6 is
`run5.py` imports-only after a name-filter fix; the fault sections of probes 2 and 3 failed to parse and are
superseded by probe 4).
