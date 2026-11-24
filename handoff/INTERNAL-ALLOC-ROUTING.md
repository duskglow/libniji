# INTERNAL-ALLOC-ROUTING: libc's own allocations go through a replaced malloc family (handoff)

State: branch `task/internal-alloc-routing`, base origin/next `279a24ee4`. Role mei:~/w/s3.
Decisions in force: D-X.156 (new; records/decisions/D-X/D-X.156.toml), D-12.31 (streams, STDIO-CALLER-ALLOC),
D-X.14 (caller memory through the symbols). Ruling #7583.

## What was done
* One mechanism for every area instead of per-area edits: `caller_alloc::decide_route` runs first in `early_init`
  (libniji's ld.so calls it after relocation, before any constructor and before the library's first heap
  allocation). It compares the addresses libc.so.6's GOT gives `malloc`, `calloc`, `realloc` and `free` with the
  library's image (`__ehdr_start`..`_end`, C wrappers in runtime.c). If any lies outside, `ROUTE` is set and
  `heap::Niji` (Rust's global allocator in the product) sends every block of alignment <= 16 through the symbols
  (`caller_alloc::malloc/calloc/realloc/free`), cold and out of line (`routed_alloc`, `routed_block`). The heap is
  untouched at that point (`Heap::untouched`, new), so a release is the replacement's by alignment alone; if the heap
  ever held blocks then, `ROUTED_CHECKED` asks the heap whose block it is.
* A refused routed allocation is NOT served from the 64 KiB reserve (that stands in for libniji's heap only): the
  fallible collections report ENOMEM, so fault injection reaches the call.
* `global_locale::intern_name_locked`: the built-in name "C" is a static literal, so `setlocale(cat, NULL)` in the
  default locale never allocates (it returned NULL under a failing replacement: found by the new test).
* `ROUTE` declared in crates/libc-so/startup-state.list (the first build broke WINDOW-BIN-BYTES: the new flag,
  read by each start-up allocation, landed outside the start-up data pages).
* conformance/c/x_internal_alloc.c (dynamic mode, reference and libniji): opendir, setlocale, iconv_open with a
  replacement that fails every call and each Nth call; recovery after; no free of memory the replacement did not
  make; qsort sorts with every allocation failing. Passes on both.
* docs/differences-from-glibc.md row (D-X.156): kept private = loader tables (dlopen/dlclose), blocks aligned > 16,
  static programs; iconv_open/pthread_create make no allocation the replacement sees; closedir keeps spare buffers.

## Inventory (mint probe phases plus the code; host reference = the build host's glibc)
| area | reference calls | libniji after | how |
|---|---|---|---|
| qsort 200,000 ints | 1 | 1 | Rust heap -> routed |
| opendir/readdir/closedir | 1 (freed) | 3 (kept as spares by closedir) | routed |
| setlocale C.UTF-8 then C | 225 | 3-4 | routed |
| iconv_open/close | 17 | 0 | no heap allocation in libniji |
| pthread_create | 1 | 0 | no heap allocation (thread block is one mapping) |
| dlopen libm, libresolv | 13 | 0 | loader tables in ld.so's own mappings: exception |
| glob, wordexp, regcomp, tsearch, getpwnam, getaddrinfo | seen | seen | routed (were already through caller_alloc in part) |
| fopen/fclose, fmemopen | seen | seen | D-12.31 |
| time zones, catalogs, resolver, NSS, regex | - | through the same global allocator | routed (not probed one by one) |

## Open
* RULING NEEDED (start-up cost): dynamic start-up rows +137..+187 user instructions (+0.22..0.29%; startup_instr_c 0),
  against the brief's "start-up must not move": the decision's wrapper calls once, then a load and branch per internal
  allocation. Recommend accept; follow-up: ld.so passes "family replaced" in StartInfo. Numbers in the report.
* RULING NEEDED (dlopen): the loader's object tables stay in ld.so's mappings (no heap; lock-free readers). Recommend:
  accept as the ruling's "ld.so's own data" exception (row written).
* RULING NEEDED (printf, D-12.12): the printf family's format tables are internal allocations, now routed, so a
  replacement that refuses calls makes snprintf fail with ENOMEM where the reference formats without allocating.
  Recommend: a follow-up that formats common conversions without heap tables, not a private exception.
* INFORMATIONAL: under valgrind the GOT is unchanged (valgrind redirects the functions), so the route stays private;
  libniji already has `niji_machine_running_on_valgrind`: routing when it answers yes is a cheap follow-up.
* INFORMATIONAL: static programs are never routed (the image test cannot tell); follow-up if wanted.

## Next step
Land (coordinator). Jobs and numbers: handoff/INTERNAL-ALLOC-ROUTING-report.md.
