# INTERNAL-ALLOC-ROUTING report

Code head fbbb0d309 (commits 16db31e1d, fbbb0d309; later commits are handoff files only). Base origin/next 279a24ee4.
All jobs on mei (Zen 1 host class per earlier handoffs), role s3 and RAM copies.

## Jobs
| label | what | result |
|---|---|---|
| internal-alloc-routing-gate | `cargo xtask affected --run` (direct tier) on the role | rc 0: 18 suites ok, 11797 passed, 0 failed; conformance 41 passed, 0 failed; peak 15G |
| internal-alloc-routing-plants | `cargo xtask plants handoff/INTERNAL-ALLOC-ROUTING-plants.txt` | 3 CAUGHT (no-route, locale-query-alloc, reserve-fallback), 0 WARN |
| internal-alloc-routing-xtest | RAM copy: `cargo xtask test`; `NIJI_FULL_PROFILE=1 cargo xtask so` + hot_text_products; `cargo xtask so`; bench (head) | rc 0; hot_text_products 33 passed |
| internal-alloc-routing-basebench | RAM copy of the base tree: `cargo xtask so`, the same bench | rc 0 |
| internal-alloc-routing-conf2 | `cargo xtask static`; `affected --only conformance:x_internal_alloc --with-exports --run` | rc 0; 1 passed |
SUPERSEDED: so1 (WINDOW-BIN-BYTES red: ROUTE outside the start-up data pages; fixed by declaring it), conf1 (csys-audit:
logic in the C layer; moved to Rust).

## mint probe (calls the replacement sees; `so` variant; exe and pre agree)
| phase | reference (host glibc) | libniji base 279a24ee4 | libniji head |
|---|---|---|---|
| qsort 200,000 ints | 1 | 0 | 1 |
| opendir | 1 | 0 | 3 (closedir keeps spares: frees 0) |
| setlocale C.UTF-8, C | 225 | 0 | 3 |
| iconv_open | 17 | 0 | 0 (no heap allocation in libniji) |
| pthread_create | 1 | 0 | 0 (no heap allocation in libniji) |
| dlopen libm, libresolv | 13 | 0 | 0 (loader tables: exception, D-X.156) |
| glob | 425 | 212 | 476 |
| regcomp | 109 | 3 | 51 |
| getpwnam | 19 | 1 | 5 |
| getaddrinfo | 14 | 1 | 33 |
| outside phases | 2 | 2 | 4 |
Count class equal to the reference everywhere except iconv_open, pthread_create, dlopen (documented, D-X.156 row).
foreign=0 in every row and variant. x_internal_alloc fails on base (opendir, setlocale see no call) and passes on head.

## Cost (instructions per call, `cargo xtask bench --compare --counters`, base and head each built in a RAM copy)
| row | base | head | delta | glibc |
|---|---|---|---|---|
| startup_instr_c | 2117 | 2117 | 0 | 123452 |
| startup_instr_setloc | 61928 | 62065 | +137 (+0.22%) | 148258 |
| startup_instr_wctype | 62393 | 62557 | +164 (+0.26%) | 148463 |
| startup_instr_printf | 65453 | 65640 | +187 (+0.29%) | 150019 |
| startup_kinstr_c (kernel) | 149742 | 152077 | +1.56% (kernel noise per STDIO-CALLER-ALLOC's re-count; not re-counted) | 719799 |
| setlocale_all_c, setlocale_en_us_repeat, setlocale_query_en_us | 117.1, 133.1, 69.0 | same | 0 | |
| cbench_locale, cbench_malloc rows | | | 0 to +0.26% (fnmatch_en_us +1.0) | |
The start-up cost is the route decision in early_init (six calls to C wrappers and the comparisons, once) plus one
load and branch per allocation, release and resize of the library's own (rows that allocate more at start move
more). A process with a replacement pays a call through the symbol per internal allocation (not benched).
Layout: start-up bin 0 60737 -> 60678 bytes. The head copy (built after a NIJI_FULL_PROFILE build in the same copy) has
10 window bins (633,816 bytes) against the base copy's 9 (568,904); bin 9 holds 65 hot-list functions with no start-up
signature (fclose, getpwuid, ...), also present in base. Not investigated (budget); layout contracts and
hot_text_products are green.

## RULING NEEDED
1. Start-up cost: +137 to +187 user instructions (+0.22 to +0.29%) on the dynamic start-up rows, against the brief's
   "start-up must not move". A runtime choice of allocator needs a test per internal allocation; recommend accept, with
   a follow-up that has ld.so pass "family replaced" in StartInfo (removes the decision's calls, about a third).
2. dlopen: the loader's tables stay in ld.so's mappings (exception written as "ld.so's own data"). Recommend accept.
3. printf family (D-12.12): its format tables are internal allocations, now routed, so a refusing replacement makes
   snprintf fail with ENOMEM where the reference formats without allocating. Recommend a follow-up to format common
   conversions without heap tables rather than a private exception.

## INFORMATIONAL
* Under valgrind the GOT is unchanged, so the route stays private; libniji has `niji_machine_running_on_valgrind`.
* Static programs are not routed.
* `closedir` keeps directory buffers as spares (a replacement sees opendir allocate, closedir not free).
