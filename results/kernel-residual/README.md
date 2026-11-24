# Where libniji's extra kernel work in a dynamic start-up goes (KERNEL-RESIDUAL-PROFILE, 2026-11-23)

Investigation, no product change. Products: libniji release dynamic kit of `next` 0495c4684 (the base the
PERF-GAP-STRUCTURE-AUDIT measured, so its numbers and these are of the same build); the reference is the installed
glibc 2.39, measured as a black box (counters, kernel-side samples with kernel call chains only, syscalls as strace
decodes them, the process's own `/proc` files; nothing of the reference's code symbolised). Host: the Skylake-SP
build host (48 threads, a virtual machine, kernel 6.8, ext4, under a memory cgroup), kernel symbols from a
read-only copy of its kallsyms (#8142). Programs: an empty `main`, `true`, `ls /`, `sed s/a/b/ words.txt`;
environment `PATH HOME LANG=C.UTF-8`. Kernel instructions (`instructions:k`) are the measure; kernel cycles on this
loaded, unbooked VM moved by up to 30% between identical runs and are advisory only.

## 0. The answer

1. **About half of the measured kernel excess is the measurement setup, not libniji.** A file page's
   mapped-page statistic (memory-cgroup and node counters) is updated when the page becomes mapped for the first
   time anywhere and again when its last mapping goes. The reference's `ld.so` and `libc.so.6` pages are mapped by
   every process on the host, so a measured process skips that work; libniji's are mapped by nobody else, so every
   measured process pays it. Tested both ways (section 2): the reference with fresh copies of its `ld.so` and
   `libc.so.6` costs **+88k to +175k** kernel instructions; libniji with a process holding its two files' pages
   mapped costs **-63k to -187k**. On a system that uses libniji as its C library this effect is the reference's
   situation and disappears.
2. **The real excess** (both libraries' pages held mapped elsewhere) is **+100k** (empty), **+146k** (`true`),
   **+283k** (`ls`), **+393k** (`sed`) kernel instructions per process. It has three causes, all libniji
   decisions (section 3):
   * **whole-file views** (the loader maps each object's whole file read-only to read its headers and tables, in a
     64 MiB arena unmapped after linking): their mapping calls, the unmap, and the teardown of the page-table
     entries the views populated. Replay of the exact calls (section 4) prices the calls at 33k to 119k; with the
     teardown, 69k to 209k;
   * **written pages** (zero-fill and copy-on-write faults: the audit's path 4): +13.6 to +41.6 faults per process;
     nearly all of `ls`'s and `sed`'s fault excess (+207k, +225k) is written pages and extra `libc.so.6` text faults;
   * **more mapping calls** (per-object reservations, trimmed over-sized mappings, separate small anonymous
     mappings): the rest of the `mmap` excess.
   Offsets: libniji makes no `ld.so.cache`, locale-file or `stat` calls (-11k to -100k), and takes fewer read
   faults on file text and data.
3. **Not attributed by this run:** nothing above 25k per process. TLB shootdowns, page-table pages and page
   zeroing differ by under 6k on every program.
4. **Parity:** with the view removal, exact-size mappings, and the audit's path 4, the empty program and `true`
   reach the reference on kernel instructions (estimate, -24k to +25k). `ls` and `sed` keep **0 to +100k** and
   **+60k to +180k**, mostly anonymous written pages not covered by path 4 (+28 and +35 faults). Section 5 gives the
   arithmetic. Parity in total cycles on the nine binaries is **not demonstrated**.

## 1. Method

* `probes/krp.py`: `perf stat` medians of 21 (user and kernel instructions and cycles, faults); `strace -f -y` of
  every syscall with the mapping calls' sizes and flags; VMAs and `/proc/<pid>/smaps` at `exit_group` (strace
  delays the call, the probe reads the files meanwhile) and at `main` (`probes/maps.c`, built with each kit);
  kernel samples with kernel call chains (`perf record -e instructions:k|cycles:k -g`), 300 runs per event and
  program. The probe uses a **fixed period per run, varied over runs** (29 primes from 40,009 to 69,821). Frequency
  mode starts with very short periods and over-weights early code. One fixed period would hit the same
  instructions in every run of a deterministic start-up. Samples per run stay under this host's 26 per tick, so
  no run is throttled. Each sample weighs its period. Samples in perf's own mmap and exec bookkeeping (about 70k
  to 160k per run, absent under `perf stat`) are dropped, and the rest are scaled to the `perf stat` median: scale
  0.93 to 1.02.
* `--only shared` (section 2): four variants of each program, all started through an explicitly invoked loader
  (`<ld.so> --library-path ... <program>`, as in the audit): the reference; the reference with fresh copies of its
  `ld.so` and `libc.so.6`; libniji; and libniji while a holder process keeps every page of its `ld.so` and
  `libc.so.6` mapped.
* `probes/analyze.py`: two views of the samples. **Entry** is where the kernel was entered: syscall by name, page
  fault by kind, `exit_group`, with sub-buckets for `munmap` and `mmap`. **Mechanism** is the deepest frame that
  names one: VMA tree, PTE fill, PTE teardown, accounting, page allocation, zeroing, copying, page-table pages,
  TLB flush. The fault kind comes from marker functions, because several fault handlers are inlined in this kernel.
* `probes/faults.py`: every fault's data address (`perf record -e page-faults -c 1 -d`) placed in its mapping:
  view, text, rodata, data, anonymous, stack. Mean of 5.
* `probes/replay.py` + `probes/replay.c` (section 4): each loader's recorded `mmap`/`munmap`/`mprotect`/`brk`
  sequence replayed in a fresh static process. Addresses are re-expressed relative to earlier results. Nothing is
  touched, so the count has no faults, and the process's exit teardown is included. The count is the median of 21
  of the run minus parse-and-open only. Variants remove one libniji pattern each.

## 2. The measurement-setup effect: who else maps the pages

Kernel instructions per process, median of 21, all four variants through an explicitly invoked loader:

| Program | reference | reference, unshared copies | libniji | libniji, pages held | held - reference | libniji - unshared reference |
|---|---:|---:|---:|---:|---:|---:|
| empty `main` | 627,002 | 717,409 (+90,407) | 808,653 | 726,766 (-81,887) | **+99,764** | +91,244 |
| `true` | 624,562 | 712,363 (+87,801) | 833,552 | 770,271 (-63,281) | **+145,709** | +121,189 |
| `ls /` | 1,543,393 | 1,717,485 (+174,092) | 1,975,025 | 1,826,849 (-148,176) | **+283,456** | +257,540 |
| `sed` | 1,604,213 | 1,779,683 (+175,470) | 2,183,911 | 1,996,731 (-187,180) | **+392,518** | +404,228 |

Evidence for the mechanism (unheld libniji against the reference, empty `main`, 300 runs): the accounting
calls under `folio_add_file_rmap_ptes` (first mapping of a file page) cost about 43k per process for libniji
against about 1.2k for the reference, and those under `folio_remove_rmap_ptes` (last unmapping) about 7k against
0. With the pages held, libniji's accounting falls to 38.7k in total, against the reference's 20.6k; the
remainder is anonymous-page charging. Both files are on the same ext4 file system, which in this kernel keeps
page-cache folios at 4 KiB, so folio size is not the difference. Page faults are unchanged by holding (60 and 60
for the empty `main`).

Consequence for every libniji-versus-reference start-up number taken this way, including the audit's +119k and
the benchmark and workload rows that start dynamic programs: they include +63k to +187k of kernel work per
process that a system using libniji would not pay (section 6, ruling).

## 3. Attribution of the real excess (pages held; kernel instructions per process)

By where the kernel was entered (`analyze.py --shared --pair ref,nijiheld`; 150 runs per event and variant):

| Cause (entry) | empty | `true` | `ls` | `sed` | Evidence |
|---|---:|---:|---:|---:|---|
| `munmap`: the view arena and views mapped alone (PTE teardown, VMA work, TLB and page release) | +57.6k | +55.2k | +139.1k | +144.4k | the reference makes 0 or 1 `munmap`; libniji's are the arena (64 MiB), the program's view, the heap's two trims and two small blocks (strace) |
| `mmap`: new mappings and `MAP_FIXED` replacement of reserved ranges | +73.5k | +68.7k | +33.7k | +58.4k | 11 against 8 `mmap` (empty), 18 against 11 (`true`), 36 against 34 (`ls`), 42 against 37 (`sed`); per call about 19k against 15k |
| page faults, net | +5.8k | +17.9k | +206.7k | +224.8k | faults 60/52, 61/52, 141/102, 155/110 (held/reference); classes below |
| other syscalls (reference only: `ld.so.cache`, locale files, `stat`, `access`, `getcwd`; libniji's fewer `mprotect`) | -39.0k | -10.6k | -100.3k | -57.2k | strace counts |
| execve, exit, interrupts, other | +1.9k | +14.5k | +4.3k | +22.1k | remainder |
| **total (perf stat)** | **+99.8k** | **+145.7k** | **+283.5k** | **+392.5k** | |

By mechanism (deepest named frame): the empty `main` adds VMA tree and VMA objects +83.8k, fault entry +40.6k,
PTE teardown +31.5k, page-cgroup and stat accounting +18.1k, page allocation +14.9k, and saves file PTE fill
-42.7k, file open and lookup -26.9k, page copy -18.4k. `ls` adds fault entry +93.1k, PTE teardown +83.8k, file PTE
fill +68.5k, VMA work +62.8k, page allocation +31.3k, accounting +27.4k, and saves file open -62.9k. TLB flushes
(-0.1k empty, -2.8k `true`), page-table pages (+2.1k to +5.3k; `VmPTE` at exit 40 against 52 KiB for the empty
`main`, 44 against 52 for `ls`, 44 against 60 for `sed`) and page zeroing (under +4k) are small. No remote TLB
shootdown path differs: the processes are single-threaded.

Faults by the class of mapping, mean of 5 (reference / libniji):

| Program | anonymous | stack | file data (written) | text | rodata | views (libniji) / `ld.so.cache` and locale files (reference) |
|---|---:|---:|---:|---:|---:|---:|
| empty `main` | 10 / 12 | 1.8 / 8.4 | 14 / 19 | 19.8 / 10.8 | 5 / 4 | 2 / 3 |
| `true` | 10 / 12 | 1.8 / 10.8 | 14 / 19 | 19.8 / 11.4 | 4 / 4 | 2 / 4 |
| `ls /` | 24 / 51.8 | 3.4 / 10.8 | 21 / 25 | 26.6 / 36 | 12.4 / 7 | 16 / 8 |
| `sed` | 25 / 59.8 | 6.8 / 10.6 | 23 / 26 | 28 / 38.6 | 11 / 9 | 18 / 9 |

Written pages (anonymous + stack + data): +13.6, +16, +39, +41.6 faults; read faults on files: -9, -6.4, -4,
-0.4. Unit price from the same samples (empty `main`, pages held): (fault entry + kind) / faults is about 4.8k
per written-page fault and 7k to 9k per read fault with fault-around, consistent with the audit's probe 4 (zero-fill 4,538, copy-on-write
5,104). The coordinator's F-04 question: the loader's stack (40 KiB at `main`, `Table<..., INLINE>` in `link`'s
frame) accounts for +6.6 of the empty `main`'s +13.6 written-page faults, **about 31k kernel instructions, 48% of
its written-page excess**; for `ls` +7.4 of +39 (19%).

VMAs and address space (at `exit_group`, reference / libniji): empty 22 / 26 VMAs, 2,556 / 6,776 KiB; `ls` 47 / 40,
4,040 / 12,176 KiB; `sed` 52 / 44, 4,040 / 14,228 KiB. libniji keeps 3 `PROT_NONE` areas (76 KiB). The large
reservations cost nothing per page. They cost per VMA operation, which the replay prices (section 4). The 64 MiB
`MAP_NORESERVE` arena exists only during linking. At `main` the empty program has 23 / 27 VMAs, `VmPTE` 40 / 60 KiB.

## 4. Each mapping pattern tested: exact replay of the calls

Kernel instructions of the mapping calls alone (no faults; exit teardown included), median of 21:

| Program | reference | libniji | no views | no trimmed over-size | no per-object reservation | all three |
|---|---:|---:|---:|---:|---:|---:|
| empty `main` | 93,777 | 132,158 | -32,952 | -1,516 | -3,156 | -38,669 (93,489) |
| `true` | 153,675 | 226,969 | -58,912 | -26,740 | -4,260 | -64,492 (162,477) |
| `ls /` | 401,357 | 484,188 | -92,211 | -25,707 | -9,681 | -103,971 (380,217) |
| `sed` | 458,170 | 556,842 | -119,442 | -36,145 (one call failed in replay) | -9,809 | -145,383 (411,459) |

With the three patterns removed, libniji's mapping calls cost the same as or less than the reference's on every
program. The replay prices each call at about half its cost in the real process (section 3: 19k against about
9k), because the real process has a larger address space and warmer allocator state. The replay figures are
therefore **lower bounds** on the saving.

## 5. Causes, decisions, fixes, estimates

| # | Cause | libniji decision (where) | Change that removes it | Estimate per process (basis) |
|---|---|---|---|---|
| 1 | Whole-file views: arena reservation, view `mmap`s, arena `munmap`, teardown of the views' PTEs | DYN-START-RING view arena; D-X.152 (views under valgrind); `crates/boundary/src/ld/image.rs` `views_arm`, `views_take`, `views_close`, `open_whole`; the program's own view in `ld.so PROGRAM` mode | Read the ELF header and program headers with `pread` into loader memory. Read the dynamic tables through the object's own first read-only segment (the loader maps it anyway), validated once at map time. No view, no arena | -33k to -69k empty, -59k to -97k `true`, -92k to -177k `ls`, -119k to -209k `sed` (low: replay §4; high: plus the munmap PTE teardown and page release of §3, which the views alone populate) |
| 2 | Over-sized mappings trimmed afterwards: the heap's first region (32 MiB `MAP_NORESERVE`, then two `munmap`s), aligned single views | `crates/boundary/src/heap.rs` (first region, D-3.20 node placement), `image.rs` view alignment | Map at the final size, at an aligned hint with `MAP_FIXED_NOREPLACE`, falling back to the over-size only when the hint fails | -2k empty, -27k `true`, -26k `ls`, -36k `sed` (replay §4) |
| 3 | `PROT_NONE` reservation per object, segments `MAP_FIXED` into it | D-27.15, `crates/ld/src/map.rs` `reserve`, `map_planned` | Map the first segment over the whole span from the file, the others `MAP_FIXED` over it (no anonymous reservation) | -3k to -10k (replay §4): small, only worth doing with item 1 |
| 4 | Written pages: stack, `libc.so.6` data, anonymous | audit path 4; `Table<..., INLINE>` in `link` (F-04), heap first use, thread registry and descriptor (D-22.10, D-22.20) | audit path 4 (a) to (c) | path 4 removes 11 to 16 written faults on `ls`: -52k to -75k (4.8k per fault, §3); the empty `main`'s stack alone -31k |
| 5 | Pages mapped by no other process (measurement setup) | none: a property of how a non-system library is measured | Measure with libniji's pages held, or against unshared reference copies (§2) | -63k to -187k on the measured numbers; 0 on a system using libniji |
| 6 | libniji-only syscalls: `gettid` x14 and `getpid` (`thread::foreign_words`, which runs while libselinux's constructor runs before the runtime adopts the initial thread, D-22.20), `uname` (D-22.9), `get_mempolicy` + `mbind` (D-3.20), `ioctl(TCGETS)` per `fopen` | as named | Adopt the initial thread before any object's initialiser in a process libniji's `ld.so` starts; decide D-22.9 without `uname` on 6.7+ kernels (from the auxiliary vector or once per boot) | under 25k on `ls` (**guess**: about 1.5k per minimal syscall; not separable above sampling noise here) |

## 6. Parity, plainly

Kernel instructions, pages held (section 3), after items 1 to 4. The ranges combine each item's low and high
estimates:

* empty `main`: +100k, minus 33k to 69k (1), 2k (2), 3k (3), 38k (4: 5 stack and 3 data faults). Result **-12k to
  +24k**: parity within the estimates.
* `true`: +146k, minus 59k to 97k (1), 27k (2), 4k (3), 38k (4). Result **-20k to +18k**.
* `ls`: +283k, minus 92k to 177k (1), 26k (2), 10k (3), 52k to 75k (4). Result **-5k to +103k**. What remains is
  anonymous written pages (+28 faults, path 4 covers about 6 of them) and `libc.so.6` text (+9.4 faults).
* `sed`: +393k, minus 119k to 209k (1), 36k (2), 10k (3), 52k to 75k (4). Result **+63k to +176k**, for the same
  reasons as `ls` (+35 anonymous, +10.6 text faults).

Combined with the audit's paths 2 and 3 for user instructions: user cycles are about 20% of start-up cycles and
kernel cycles 80%. So the empty `main` and `true` would reach the reference in total cycles if both estimates hold.
`ls` and `sed`, and the nine binaries in general (`sort` and `grep` load two libraries, `sed` three), do not: they
keep the anonymous written pages and the extra text faults. The next measurement settles the anonymous pages: a
call-chain census of the anonymous faults of `ls` and `sed`, by writer. The audit's probe 8 began it with 9 of 32
from the heap, 4 from the runtime, 2 from stdio, 2 from `memcpy` and 3 from the loader. Kernel cycles were not
usable on this host (section 0).

**RULING NEEDED (measurement method).** Every start-up comparison against the reference (benchmarks, the
workload gate's dynamic rows) measures libniji's pages as mapped by no other process. Recommendation: run those
comparisons with a holder keeping libniji's `ld.so` and `libc.so.6` mapped (the `--only shared` probe's holder:
one process per host). This removes 63k to 187k kernel instructions per process that a libniji system does not
pay. Bounds set from today's numbers would move, so the switch needs a ruling.

## 7. Reproduce

On a build host with the base's products primed (`~/wl/products/<sha>-release-dyn`), a role tree for
`validation/wl_dyncc.sh`, and a kallsyms copy (`--kallsyms`), from the `probes/` directory:

```
python3 krp.py out1                        # stat, strace, VMAs, kernel samples (about 9 minutes)
python3 krp.py --runs 150 --only shared out3   # the four variants of section 2 (about 10 minutes)
python3 analyze.py out1                    # sections 3 views, libniji against the reference
python3 analyze.py out3 --shared --runs 150 --pair ref,nijiheld
python3 analyze.py out3 --shared --runs 150 --pair ref,nijiheld --detail instructions:k empty mechanism "memcg / stat accounting" 4
python3 replay.py out1 && python3 faults.py out1
```

Job labels (jobrun kind `probe`, Skylake-SP build host): `kernel-residual-profile-probe1` (out1),
`-probe3` (shared; `-probe2` was stopped for a parser defect fixed before `-probe3`), `-probe4` (replay; its fault
section is superseded), `-probe5` (faults). Probe 1's call-chain parser lost the frames of about 6% of the `ls`
and `sed` samples ("none"). Probe 3 onwards uses the fixed parser, and the `ls`/`sed` attribution above comes
from probe 3.
