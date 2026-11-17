# Hard-coded limits in libniji's products (AUDIT-HARDCODED-LIMITS)

An audit, for the owner to rule on case by case. It changes no product code. Owner, 2026-11-17: fixed
limits "are clearly causing more problems than they're solving. Memory is an *almost* inexhaustible
resource for the purposes of this project", and "None of that is to mean that memory is to be used
*irresponsibly* ... I just don't want artificial limits". Ruling #5594 (11-14, regcomp) gives the test:
"we are protecting the user from malicious code and security issues, not from themselves"; process limits
(ulimit, `RLIMIT_*`, cgroups) are the user's bound, and running out is a clean error from a fallible
allocation.

Scope: code that ships (`crates/*/src` of the shipped crates, `csys/`, the loader, ldconfig, the C
utilities `ldd`, `getconf`, `getent`). Test code, xtask, tools, `spectest`, `mathtools` and the
`#[cfg(test)]` regex oracle are left out. Read on `next` at 575214f4b (this branch's base).

## Method

**Search routes** (column "Route" below; all five rerun with `bash results/hardcoded-limits/search.sh
[OUTDIR]` from the repository root, which writes one file per route):

| Route | What it finds | Hits (lines) |
|---|---|---|
| R1 | `const`/`static` named `*_MAX`, `*_LIMIT`, `*_CAP`, `*_SIZE`, `*_DEPTH`, `*_SLOTS`, `*_ENTRIES`, `*_COUNT`, `*_LEN`, `MAX_*`; C `#define`s alike | 219 |
| R2 | fixed arrays used as tables: statics, atomic pools, arrays sized by a named constant; C arrays with a size of 10 or more | 268 |
| R3 | narrowing of counts and indices (`u8::try_from`, `u16::try_from`, ...) | 273 |
| R4 | "too many / no room" texts, and errnos a cap could invent (`EMFILE`, `ENOSPC`, `E2BIG`, `MM_NOTOK`, ...) | 52 + 29 |
| R5 | depth, retry, iteration and time limits (`*_DEPTH`, `*RETRY*`, `*BUDGET*`, `depth > MAX`, ...) | 66 |

Every hit was read; most are layouts, numeric constants or caches. This document keeps every hit that
bounds a variable number of things a user can reach. Section 6 lists the caches and inline areas that
looked like limits but fall back without failing, so a reader can see they were checked.

**Probes** (`results/hardcoded-limits/`): `probes.c` and `run.sh` run each probe past the limit against
the reference (a build host running Ubuntu 24.04, GLIBC 2.39-0ubuntu8.9) and against libniji's products
built on that host on 2026-11-17 07:31 (a role directory's `target/niji`, a `next`-era build: the code read here is the same, but the
products were not built from this branch's head). Probe programs are linked against libniji the same
way `xtask so` links its utilities. Output: `probes.log` (job `ahl-probes-5`). `objects.sh`
measures how many shared objects real programs load (`objects.log`, job `ahl-objects`). Summary:

| Probe | Reference | libniji |
|---|---|---|
| `dlopen` 600 objects (each with a TLS variable), all held open | ok, 600 | stops at 509: "cannot load more objects" |
| `dlopen` one library that needs 100 new objects | ok | "cannot load more objects in one call" |
| program with 100 / 300 `DT_NEEDED` | ok / ok | "ld.so: libp63.so: too many objects", exit 127 |
| 300 named semaphores open at once | ok | `sem_open` fails at 256: `EMFILE` |
| 300 `addseverity` levels | ok | fails at 256 (`MM_NOTOK`) |
| `getaddrinfo_a(GAI_WAIT)` with 5000 requests | ok, 5000 | ok, 5000 (the limit was not reached: workers empty the queue while it fills) |
| 1100 distinct directory-form locales (`LOCPATH`, links to `C.utf8`) | ok, 1100 | `newlocale` fails at 78: `ENOENT` |
| `register_printf_modifier` ×20, `register_printf_type` ×300 | 16 and 248, then `ENOSPC` | 16 and 248, then `ENOSPC` (same) |
| 1,100,000 `atexit` handlers | ok | fails at 1,048,575 |
| `fnmatch` with 65 / 200 nested `@(...)` groups (`FNM_EXTMATCH`) | match / match | error / error |
| `wordexp` with 129 / 300 nested `${x:-...}` | ok / ok | `WRDE_SYNTAX` / `WRDE_SYNTAX` |
| `ldd` of the 300-object program | 303 lines | 304 lines (this build's `ldd` traces with the reference loader, so libniji's `--list` limit was not exercised) |

Real programs on that host (a build server with no desktop; `objects.log`): 1504 programs and shared
objects examined, **7 need more than 64 objects at start** (`libvte-2.91` 80, `fwupdtool` 77, the
`gtk-*` tools 68, `libgtk-3` 67, `fwupdmgr` 67, `dbxtool` 64 + more), none more than 256. A
`python3` that imports its 47 stdlib extension modules maps 65 shared objects.

## 1. Remove candidates: resource caps only

Each row: where, the value, what it bounds, how a user meets it, what the reference does, the route,
and what replacing it with fallible growth takes. Memory notes describe growth with use, never a
larger preallocation.

| ID | Where | Value | Bounds | User meets it as | Reference | Route |
|---|---|---|---|---|---|---|
| HL-1 | `boundary/src/ld.rs:878` `OBJECTS_MAX`; `ld/src/init.rs:24` (u8 indices, `init.rs:74` `OrderError::TooMany`) | 64 | objects loaded at start, the program included | `ld.so: <obj>: too many objects`, exit 127, before `main` | no limit seen (300 ok) | R1 R3 R4 |
| HL-2 | `boundary/src/ld.rs:1407` `EDGES_MAX` | 1024 | `DT_NEEDED` links over all start-time objects | `ld.so: <obj>: too many DT_NEEDED entries` | not probed (300 objects × 1 link ok) | R1 R4 |
| HL-3 | `boundary/src/ld/dlopen.rs:53` `CAP`; `ld/readers.rs:49` `TABLE`; `ld/src/tls.rs:87` `MODULES_MAX`; `ld/src/init.rs:180` `EXIT_MAX`; u16 indices in `dlopen.rs` | 512 | objects in the process at once (start-time ones included), TLS modules, exit order | `dlopen` NULL, `dlerror`: "cannot load more objects" (at 509 + 3) | 600 ok | R1 R2 R3 R4 |
| HL-4 | `boundary/src/ld/dlopen.rs:55` `NEW_MAX` (= HL-1's 64), `:185` `Binds`, `:837` `Ctors` | 64 | new objects one `dlopen` call adds | `dlopen` NULL: "cannot load more objects in one call" | 100 ok | R1 R2 R4 |
| HL-5 | `boundary/src/ld/dlopen.rs:57` `NAMES`, `:59` `DEPS` | 128 KiB; 16 Ki entries | bytes of all object paths; dependency-list entries | "out of room for object names"; "cannot load more objects in one call" | not probed | R1 R4 |
| HL-6 | `boundary/src/ld.rs:1693` `LIST_OBJECTS_MAX`, `:1695` `PATHS_MAX`; `csys/util/ldd.c:295` `l[256]` | 256; 64 KiB | objects and path bytes `ld.so --list` / `LD_TRACE_LOADED_OBJECTS` prints; lines `ldd` parses | "too many shared objects", "path too long" (D-27.16) | 303 lines ok | R1 R2 R4 |
| HL-7 | `boundary/src/sync.rs:835` `NAMED_MAX` | 256 | named semaphores open at once in a process | `sem_open` `EMFILE` (no descriptor limit was reached) | 300 ok | R1 R2 R4 |
| HL-8 | `boundary/src/fmtmsg.rs:75` `CAPACITY` (D-2.6) | 256 | `addseverity` levels | `addseverity` `MM_NOTOK` | 300 ok | R2 R4 |
| HL-9 | `boundary/src/gai_async.rs:82` `QUEUE`, `:84` `GROUPS` | 4096; 256 | `getaddrinfo_a` requests pending at once; lists awaiting a notification | `EAI_AGAIN` for the call and the requests not queued (code read) | 5000 ok | R1 R2 |
| HL-10 | `legacy/src/global_locale.rs:40` `FILES`, `:45` `MAP_LIMIT`, `:38` `CAPACITY` | 1024 files; 2048 mappings; 1024 locales and 1024 names | locale files mapped (each distinct path once; a file replaced and re-mapped after an invalidation, D-X.13, counts again); interned locales and names | the locale is "not found": `newlocale` NULL with `ENOENT`, `setlocale` NULL. A directory-form locale maps about 13 files, so **78 distinct locales** | 1100 ok | R1 R2 |
| HL-11 | `legacy/src/global_locale.rs:86` `CHARMAPS` | 32 | charmaps loaded per process (iconv) | `iconv_open` fails | no limit | R1 R2 (owner already ruled: SYS-ICONV-COVERAGE-2 removes it) |
| HL-12 | `legacy/src/exit_handlers.rs:39` `MAX_ENTRIES` | 2^20 | `atexit`/`on_exit`/`__cxa_atexit` entries | registration fails at 1,048,575 | 1.1 M ok | R1 R5 |
| HL-13 | `core/src/pattern/fnmatch.rs:59` `MAX_DEPTH` | 64 | nesting of extended groups (`FNM_EXTMATCH`) | `fnmatch` returns an error (not `FNM_NOMATCH`) | 200 matches | R1 R5 |
| HL-14 | `core/src/pattern/wordexp.rs:221` `MAX_DEPTH` | 128 | nesting of `${...}`, `$((...))` | `WRDE_SYNTAX` | 300 ok | R1 R5 |
| HL-15 | `ld/src/ldconfig/conf.rs:27` `FILE_MAX`, `:29` `DIRS_MAX`; `ld/src/cache/writer.rs:35` `HWCAPS_MAX`, `:37` `GENERATOR_MAX`, `:39` `ENTRIES_MAX`; `boundary/src/ldconfig.rs:21` `ARGS_MAX` | 1 MiB; 65536; 64; 4096; 2^20; 4096 | ldconfig: configuration bytes read (**the rest of a longer file is ignored silently**); directories named; hwcaps names; generator text; cache entries; arguments | silent truncation; "too many directories, ignored"; "too many glibc-hwcaps subdirectories"; "too many libraries"; `E2BIG` text | not probed | R1 R4 |
| HL-16 | `core/src/names/sort.rs:30` `MAX_ENTRIES` | 32 | entries of one `gai.conf` table (`label`, `precedence`, `scopev4`) | further lines ignored silently | not probed (needs root to edit `/etc/gai.conf`); `gai.conf(5)` names no limit | R1 R5 |
| HL-17 | `core/src/names/nss.rs:287` `MAX_STEPS` | 16 | sources on one `nsswitch.conf` line | the line does not parse | not probed (root) | R1 R5 |
| HL-18 | `boundary/src/report.rs:229` `LAST_MAX` | 4096 | file name `error_at_line` remembers for `error_one_per_line` | a longer name is never seen as a repeat: the message prints again | not probed | R1 R2 |
| HL-19 | `boundary/src/users.rs:719` `PASS_DEFAULT_LIMIT`; `native/src/getpass.rs:13` (D-17.8) | 1 MiB | `getpass` input before the newline | per D-17.8 | not probed | R1 |
| HL-20 | `rt/src/unwind.rs:59` `REMEMBER_DEPTH`, `:61` `EXPR_STACK` | 4; 32 | `DW_CFA_remember_state` nesting; DWARF expression stack | unwinding through such a frame stops (code read) | not probed (the reference's unwinder is outside this audit's probes) | R1 R5 |

**What removing each takes** (sizes are estimates from reading the code, not measured):

- **HL-1, HL-2** (in progress as DL-STARTUP-OBJECTS, GitHub #9): the start path keeps about a dozen
  stack arrays sized `OBJECTS_MAX` (`ld.rs:1112-1382`, `1455-1537`, `init.rs:56-72`) and u8 indices
  (`ld.rs:1139`, `init.rs:74`). Keep inline arrays of 64 for the common case and move to a loader
  arena (anonymous `mmap`, doubled as needed, unmapped after start) past it; indices to u16/u32.
  Cost to watch: none below 64 objects (the inline path stays); start-up instructions and faults of
  the 30 workloads must not move. Tests: start with 65, 300 and 1000 objects; init and fini order
  past 64; edges past 1024.
- **HL-3, HL-4, HL-5**: the process table (`CAP`), the readers' lock-free table, `THREAD_MODS`, the
  dtv module slots and the exit order all assume 512, and indices are u16. Growth needs tables that
  are never moved (generations, as the heap's registry and the loader's `GrowTable` for unique
  symbols already do), the dtv grown per thread on demand, and u32 indices. The arenas (HL-5) become
  growable `MapStack`s, as the unique-symbol table already is. `NEW_MAX` (HL-4) goes with HL-1's
  `init_order` change. Largest change in this list (several stories); hot paths to watch:
  `dladdr`, `dl_iterate_phdr`, `__tls_get_addr`. Tests: 2000 objects held, 200 new objects in one
  call, TLS in all of them, `dlclose` and reuse.
- **HL-6**: the `--list` path's arrays become the same arena as HL-1; `ldd.c` grows its `listed`
  array with `realloc`. Small.
- **HL-7**: the 256-row static table becomes a growable table under `NAMED_LOCK` (lookups already
  take the lock), with fallible allocation (`ENOMEM`). Small. Memory: 32 bytes per open name instead
  of a fixed 8 KiB.
- **HL-8**: `fmtmsg` reads the table without a lock; an append-only list of chunks (never moved)
  keeps that. Small.
- **HL-9**: the queue becomes a growable ring under the existing lock; `GROUPS` likewise. Small.
  `MAX_WORKERS` (32) is a concurrency policy, not a failure; it stays (section 6).
- **HL-10**: the slot table (1024 × 112 bytes, page-aligned) becomes chunked growth that is never
  moved; re-mapping after an invalidation should release the replaced mapping rather than count it
  (whether the old mapping is unmapped today was not checked). Medium. Start-up page faults must not
  move (STARTUP-DATA-PAGE-ALIGN-3 placed the first page on purpose). The failure is reported today as
  `ENOENT` (not found), which hides the cause; with growth only `ENOMEM` remains.
- **HL-12**: past the inline entries the registry maps 32 MiB of address space in one go
  (`exit_handlers.rs:98`); doubling mappings would map only what is used and remove the cap. Small.
- **HL-13, HL-14**: the limits exist because matching and parsing recurse. Replacing recursion with
  an explicit stack on the heap bounds depth by the pattern's length (the caller's memory) and keeps
  the stack safe. The work bound for hostile patterns must stay polynomial (tests: deep nesting,
  backtracking-heavy extglob). Medium for `fnmatch`, small for `wordexp`.
- **HL-15**: ldconfig reads root's configuration and runs once per package install; growth costs
  nothing that matters. Small each; the silent truncation at 1 MiB is the one that loses data.
- **HL-16, HL-17**: inline arrays with a spill to the heap. Small.
- **HL-18**: keep the remembered name in an allocation of its length. Tiny.
- **HL-19**: D-17.8 chose 1 MiB; under ruling #5594 the input is the user's own terminal, so the
  owner may want growth instead. Tiny.
- **HL-20**: inline 4 / 32 with a spill to the heap; the unwinder may run in a signal handler or on
  a small stack, so the spill must be optional (stop unwinding when allocation is not possible), and
  `EXPR_BUDGET` (section 3) stays. Small.

## 2. Ranking of the remove candidates (how likely a real program meets them)

| Rank | ID | Likelihood | Evidence |
|---|---|---|---|
| 1 | HL-1 (HL-2) | met today | measured: 7 of 1504 programs/objects on a server host need more than 64 objects at start (GTK tools, fwupd); GitHub issue #9; any GTK/Qt desktop program |
| 2 | HL-4 | very likely | measured: `libgtk-3` alone needs 67 objects, so a host with only libc loaded that `dlopen`s it (python3-gi, plugin hosts, `gtk_init` in a scripting language) passes 64 new objects in one call |
| 3 | HL-10 | likely for some programs | measured: 78 directory-form locales per process. Programs that open every installed locale (test suites, PostgreSQL's `pg_import_system_collations`, which by its documentation adds a collation for each locale the OS has), and long-running daemons that see repeated locale updates (each re-map counts against `MAP_LIMIT`). How locale-archive-form locales count against `FILES` was not checked |
| 4 | HL-7 | likely for Python | Python's `multiprocessing` builds `Lock`, `Semaphore` and `Queue` on POSIX named semaphores (several per queue, held open; CPython's documented requirement of a working `sem_open`, the count per queue not measured here); a program with 100 queues passes 256 |
| 5 | HL-3, HL-5 | possible | large desktop sessions and scientific Python (each extension module is an object; a stdlib-only python3 already maps 65, measured) |
| 6 | HL-9 | possible | mass resolvers (crawlers, mail servers) with more than 4096 lookups pending at once; not reached by the probe |
| 7 | HL-13, HL-14 | rare | only generated or hostile patterns nest past 64 / 128 |
| 8 | HL-6, HL-15 | rare | no program on the build host needs more than 80 objects; ldconfig inputs are far below the caps |
| 9 | HL-8, HL-12, HL-16, HL-17, HL-18, HL-19, HL-20 | very rare | no known program near them |

## 3. Keep candidates

### Security bounds on untrusted input

| ID | Where | Value | Threat it bounds | Route |
|---|---|---|---|---|
| HL-21 | `core/src/time.rs:43` `FILE_LIMIT` | 4 MiB | `TZ` can name any file (also in a privileged program): bounds memory and parse time; installed zone files are under 100 KiB | R1 |
| HL-22 | `legacy/src/global_locale.rs:103` `CHARMAP_LIMIT` | 32 MiB | a compressed charmap that inflates without end (decompression bomb); installed ones are a few MiB | R1 |
| HL-23 | `core/src/names/cache.rs:55` `CAP_BYTES`, `:67` `MAX_ENTRY_WORDS` | 8 MiB; 64 KiB | the name-service cache's shared region is written by every process of the user and read as untrusted; a longer answer is just not cached | R1 |
| HL-24 | `ld/src/cache.rs:36` `EXT_MAX`; `ld/src/map.rs:47` `SPAN_MAX`; `kernel/src/vdso.rs:13` `MAX_IMAGE`; `ld/src/ldconfig/scan.rs:12,14,16` | 64; 2^46; 1 MiB; 256 MiB, 4096, 1 MiB | bounded work on a hostile `ld.so.cache`, ELF file or image (`e_phnum` is 16-bit anyway) | R1 |
| HL-25 | `core/src/locale/collate.rs:36` `MAX_LEVELS`; `core/src/time/format.rs:335` `COMPOSITE_MAX`; `core/src/time/fast.rs:216` composite depth | 16; 2^20 units; small | locale data from `LOCPATH` is user-controlled: a malformed table or a `%c` that names itself must end | R1 R5 |
| HL-26 | `rt/src/unwind.rs:63` `EXPR_BUDGET` | 1024 operations | DWARF expressions can branch backwards: a loop guard on object-file data | R5 |

### Overflow and correctness guards

| ID | Where | Value | What it guards | Route |
|---|---|---|---|---|
| HL-27 | `boundary/src/ld.rs:2034` `AUXV_WORDS_MAX` | 256 words | copy of the kernel's auxiliary vector (today under 30 pairs); start refuses with a text past it. Could become dynamic cheaply if the kernel ever grows it | R1 R4 |
| HL-28 | `ld/src/ldconfig/conf.rs:25` `DEPTH_MAX` | 32 | `include` cycles in ld.so.conf | R1 |
| HL-29 | `boundary/src/argp.rs:88-89` | 4096; 2^20 | a cyclic (malformed) argp tree ends | R1 |
| HL-30 | `core/src/search.rs:522` `MAX_DEPTH` | 96 | AVL height: a tree that high would hold more than 2^64 nodes, so never reached | R1 |
| HL-31 | `core/src/num/parse.rs:274` `MAX_KEPT`, `core/src/num/format.rs:60` `MAX_DIGITS` | 12,000; 12,288 | the digits correct rounding can depend on (exact expansions); never a user-visible cap | R1 |
| HL-32 | `rt/src/heap.rs:143,145,147` `MAX_SEGS`, `LARGE_CAP`, `MAX_CACHES` | 2^20 each | address-space-scale registries (16 TiB of small blocks; a million live threads); representation bounds, not reached before the kernel's own limits | R1 |
| HL-33 | `ld/src/tls.rs:20` `SURPLUS` | 64 KiB | static TLS for objects loaded later (static TLS cannot move once threads exist; the reference has a fixed surplus too, by its manual's tunables section). Its size against the reference's was not probed: follow-up probe recommended | R1 |

## 4. ABI- or spec-fixed (not ours to change)

`PTHREAD_KEYS_MAX` (ABI 1024; libniji allows 4096, `rt/src/thread.rs:28`, D-22.5);
`PTHREAD_DESTRUCTOR_ITERATIONS` 4 (`rt/src/thread.rs:30`); `SEM_VALUE_MAX` (`rt/src/pthread_sync.rs:1196`);
`DELAYTIMER_MAX` (`boundary/src/timer.rs:120`); `AIO_PRIO_DELTA_MAX` (`boundary/src/aio.rs:174`);
`TMP_MAX` and the mkstemp attempts 238,328 (`boundary/src/stdio.rs:2029`, `core/src/fs/temp.rs:19`);
`tempnam`'s 5-byte prefix (`boundary/src/stdio.rs:2035`); `NL_ARGMAX` 4096 for positional arguments
(`boundary/src/printf.rs:209`, `legacy/src/scanf_va.rs:10`; whether the reference enforces it was not
probed); `RE_DUP_MAX` 0x7fff (`core/src/pattern/regex/parse.rs:22`); `re_max_failures`
(`core/src/pattern/regex.rs:402`); `register_printf_modifier` 16 (a bit in `printf_info.user`, an
unsigned short) and `register_printf_type` 248 (`legacy/src/printf_registry.rs:58,62`; the probe shows
the reference stops at the same counts), and its 64-character modifier text (`:60`, `EINVAL`; reference
not probed); thread names 15 bytes (`legacy/src/pthread_compat.rs:285`, the kernel's comm length); pty
names (`core/src/term.rs:387`); kernel `PATH_MAX` / `NAME_MAX` (`kernel/src/path.rs:10,12`,
`ld/src/search.rs:32`, `legacy/src/intl.rs:72`, `core/src/catalog/xopen.rs:24`, `legacy/src/nss.rs:308`,
`ld/src/ldconfig/sysfs.rs:26`, `core/src/locale/name.rs:7` = one path component); symlink hops 40 in
`realpath` (`core/src/fs/canon.rs:31`, the kernel's `ELOOP` count in path_resolution(7)); `resolv.conf`
caps ndots 15, timeout 30, attempts 5 (`core/src/names/dns/conf.rs:71-75`, resolv.conf(5)); DNS
messages 65,535 bytes (`core/src/names/dns/client.rs:35`, the 16-bit TCP length of RFC 1035); host.conf
trim domains 4 / 127 bytes (`core/src/names/hostconf.rs:22,24`, matching the reference's warning, per
the code comment); `nsap_ntoa` 255 bytes (`legacy/src/inet_compat.rs:337`); `lckpwdf` 15 s and utmp
lock 10 s (`legacy/src/accounting.rs:50,52`, §17.4, §17.6); `sysconf` `ARG_MAX` figures
(`core/src/system.rs:80,82`); `NI_MAXHOST` buffer in getent (`csys/util/getent.c:571`); struct layouts
and sizes (`SIGSET_SIZE`, `TTYENT_SIZE`, `SEM_SIZE`, `OPTION_SIZE`, `STATE_SIZE` 568, `FILE_WORDS`,
`ResObject`, `sys_siglist[65]`, `h_errlist[5]`, the ELF record sizes in `ld/src/elf.rs:19-39`,
`ASCTIME_LEN`, `STAT_SIZE`); `mallopt` parameter bounds (`legacy/src/malloc_compat.rs:185,189`);
`MAX_RW` 0x7ffff000 (the kernel's largest single transfer, `boundary/src/aio.rs:184`).

## 5. Not verified

- `boundary/src/aio.rs:1773-1774` io_uring ring sizes (256 / 4096): whether more requests in flight
  than the ring holds queue in software or fail was not read.
- `boundary/src/posix/s29_rpc_clnt.rs:40` `TEXT_SIZE` 1024 (the `clnt_sperror` family's text).
- The `--list` limit (HL-6) under libniji's own loader (this build's `ldd` used the reference loader).
- `getaddrinfo_a` past 4096 pending requests (HL-9): code read only.

## 6. Looked at, not limits (fallback without failure)

Caches and inline areas that spill or fall back to a slower path: the stack cache 40 MiB
(`boundary/src/thread.rs:86`, D-22.9, retention bound), `MAX_SPARES` (`boundary/src/dirtree.rs:83`),
`FOLD_MAX` (`boundary/src/dispatch.rs:206`), iconv `MEMO_SLOTS` (`core/src/iconv/mod.rs:489`), message
catalogue `R_SLOTS`, `MEMO_MAX`, `SHORT_PATH` (`legacy/src/intl.rs:375,389,815`), `IX_PROBE_MAX`
(`core/src/catalog/mo.rs:323`), plural `INLINE_DEPTH` (`core/src/catalog/plural.rs:311`), locale
`FILE_SLOTS`, `POOL_LEAVES`, `LATIN_SLOTS`, `ASCII_SLOTS`, `LEAD_SLOTS` (`core/src/locale/classes.rs:45,48`,
`view.rs:299`, `collate.rs:44,60`), `STACK_GROUPS` (`legacy/src/cred_compat.rs:23`), `XFRM_STACK`
(`legacy/src/cstring.rs:117`), `STACK_WORDS` (`core/src/search.rs:19`), `SIMPLE_MAX`
(`core/src/printf.rs:1411`), the regex engine's `NODE_CAP`, `MAX_STATES`, `ROOM_CAP`, `EPS_DEPTH`,
`RECURSE_DEPTH`, `required.rs` bounds (each falls back to another engine path; regcomp's 2^21-node cap
is now `HARNESS_MAX_NODES`, used by fuzz targets and tests only, ruling #5594), name-service cache TTL
caps (`legacy/src/nscache.rs:107,110`), `gai_async` `MAX_WORKERS` 32 and `aio` worker counts
(concurrency policy), `malloc_debug` quarantine 2 MiB (`legacy/src/malloc_debug.rs:100`, a debugging
feature), `NIJI_DIAG` value 256 bytes (`boundary/src/diag.rs:43`, libniji's own variable), timer and
mqueue retry back-off (`boundary/src/timer.rs:131-208`, `boundary/src/mqueue.rs:84`), spin counts
(`rt/src/sync.rs:18`, `rt/src/pthread_sync.rs:113`).

## 7. Memory stewardship (seen on the way; sizes from the code, not measured)

- `exit_handlers.rs:98`: one 32 MiB address-space mapping as soon as the inline entries are used up,
  whatever the number of handlers (counts against `RLIMIT_AS`). Doubling would map what is used (HL-12).
- Fixed tables that removing the caps would also shrink or make lazy: the readers' table
  (`ld/readers.rs:49`, 20 KiB), `THREAD_MODS` (`ld.rs:526`, 20 KiB), the locale slots (112 KiB,
  page-aligned), the `gai_async` queue (32 KiB), the named-semaphore table (8 KiB). All are `.bss`:
  untouched pages cost address space only. None of these is waste by itself.
- The heap's registries "never moved, never unmapped" (`rt/src/heap.rs:198-200`): the tables never
  shrink. Address space only; listed for the owner's view.
- The stack cache keeps up to 40 MiB of freed thread stacks (D-22.9): a stated, bounded retention.
- Whether a locale file replaced on disk has its old mapping released when it is re-mapped (HL-10)
  was not checked.

## 8. Ruling table

| ID | What | Class | Recommendation | Decision |
|---|---|---|---|---|
| HL-1 | start-time objects 64 | resource cap | remove (DL-STARTUP-OBJECTS, in progress) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-2 | start-time `DT_NEEDED` links 1024 | resource cap | remove with HL-1 | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-3 | process objects / TLS modules / exit order 512 | resource cap | remove (several stories; generations, dtv growth) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-4 | new objects per `dlopen` 64 | resource cap | remove, first of the dlopen set (libgtk-3 alone is 67) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-5 | dlopen name arena 128 KiB, deps arena 16 Ki | resource cap | remove with HL-3 | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-6 | `--list`/`ldd` 256 objects, 64 KiB paths | resource cap | remove with HL-1 | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-7 | named semaphores 256 (`EMFILE`) | resource cap | remove (small; Python multiprocessing) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-8 | `addseverity` 256 | resource cap | remove (small) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-9 | `getaddrinfo_a` queue 4096, groups 256 | resource cap | remove (small) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-10 | locale files 1024, mappings 2048, locales/names 1024 (`ENOENT` at 78 directory locales) | resource cap | remove; release replaced mappings | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-11 | iconv charmaps 32 | resource cap | already ruled: remove (SYS-ICONV-COVERAGE-2) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-12 | exit handlers 2^20 (+ 32 MiB mapping) | resource cap | remove (doubling mappings) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-13 | `fnmatch` extglob nesting 64 | resource cap (stack) | remove: explicit heap stack, keep work bounded | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-14 | `wordexp` nesting 128 | resource cap (stack) | remove: explicit heap stack | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-15 | ldconfig: 1 MiB config (silent truncation), 65536 dirs, 64 hwcaps, 2^20 entries, 4096 args | resource cap | remove; the truncation first | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-16 | `gai.conf` 32 entries per table (silent) | resource cap | remove (small) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-17 | `nsswitch.conf` 16 sources per line | resource cap | remove (inline 16 + spill) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-18 | `error_one_per_line` name 4096 | resource cap | remove (tiny) | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-19 | `getpass` 1 MiB (D-17.8) | resource cap | owner's call: remove under #5594, or keep D-17.8 | remove (owner 2026-11-17: "Yes, as recommended."; ruling #5594 over D-17.8) |
| HL-20 | unwinder remember-state 4, expression stack 32 | resource cap | remove with an optional spill | remove (owner 2026-11-17: "Yes, as recommended.") |
| HL-21 | zone file 4 MiB | security bound | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-22 | charmap inflation 32 MiB | security bound | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-23 | name-service cache region 8 MiB, entry 64 KiB | security bound | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-24 | hostile cache/ELF/vDSO bounds | security bound | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-25 | locale data levels 16, composite text 2^20, composite depth | security bound | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-26 | DWARF expression budget 1024 | security bound (loop guard) | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-27 | auxv 256 words | correctness guard | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-28 | ld.so.conf include depth 32 | correctness guard (cycles) | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-29 | argp tree 4096 / 2^20 | correctness guard (cycles) | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-30 | AVL depth 96 | correctness guard (unreachable) | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-31 | strtod / printf exact digits | correctness guard | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-32 | heap registries 2^20 | representation bound | keep | keep (owner 2026-11-17: "Yes, as recommended.") |
| HL-33 | static TLS surplus 64 KiB | design bound | keep; probe its size against the reference | keep, and probe its size (owner 2026-11-17: "Yes, as recommended.") |
