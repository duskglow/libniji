# CONFIG-TABLES-GROWTH: no limit on gai.conf rows, nsswitch sources per line or [...] groups (HL-16, HL-17)

Branch `task/config-tables-growth` (from origin/next 0495c4684). Role mei:~/w/s4.

## State
Done. Decision D-16.17 (record `records/decisions/D-16/D-16.17.toml`).
- **core** `names/nss.rs`: `Spec` keeps 16 sources inline and spills to a `TryVec` past them (cold `spill`). It is
  no longer `Copy`. `Config::spec`/`resolve`/`line` return `&Spec`, and the defaults are statics (`default_spec`
  returns `&'static Spec`). `Spec::try_parse` tells `EINVAL` from `ENOMEM`; `Spec::parse` stays `Option`.
  `Step::encode`/`decode` (5 bytes) replace the whole-line `Spec::encode`/`decode` and `SPEC_BYTES`/`MAX_STEPS`.
  `rules()` takes a `[...]` group one word at a time, so it has no 256-byte buffer. A word is two pieces joined
  when an `=` is next to the space between them, the same as the old normalisation. A single word longer than 34
  bytes, which cannot be a valid item, is still refused.
- **core** `names/sort.rs`: `Table` keeps 32 entries inline and spills past them. `Policy`/`Table` are no longer
  `Copy`. `Entry::encode`/`decode` (21 bytes) are public; `Policy::encode`/`decode`/`POLICY_BYTES` are gone. A
  table that cannot take an entry (`ENOMEM`) is the default table.
- **rt** `chunks.rs`: a new object-safe trait `ChunkPages` (map, unmap, words), with a blanket impl for every
  `Pages`. `Chunks` methods take `P: ChunkPages + ?Sized`. New `Chunks::reserve_first(pages, n, w)`.
- **legacy** `nss.rs`: `Switch` holds two `Lines` tables. Each has an inline `Buf` (a flag, a count and 16
  sources per database), a per-database count of extra sources, and a `Chunks` table (one word per extra source,
  database after database). Stores reserve first, so a failed store changes nothing. Loads read each slot's count
  first and decode only the sources the line has, where the old code read all 1148 bytes. `Ctx` gains
  `pages: &dyn ChunkPages`. It loses `Copy`, and `Debug` is now hand-written. `Ctx::system`/`with_paths`/`bare`
  take `pages`, and so do `Switch::config`/`config_as_read`/`configure`.
- **legacy** `gai.rs`: `GaiConf` keeps the table lengths, a 3 x 32-entry inline `Buf` and a `Chunks` table (3
  words per extra entry). The `reload` flag has its own `AtomicBool`, read before the lock. Each policy read is
  now one decode, where the old code decoded the full 2020-byte encoding twice.
- **Pages from the boundary**: `&crate::heap::KernelPages`. `nscache::Memory` gains `pages()`, which the cached
  paths and the cache's account stamp use. `netdb_compat::resolve`, `sys_compat::gethostid` and
  `accounting::login_name` gain a `pages` parameter.
- **ENOMEM rule**: an nsswitch line that cannot be held is taken as a line that does not parse (no source).
  `__nss_configure_lookup` returns `ENOMEM` and leaves the overrides unchanged. A gai.conf table that cannot be
  held is the default table, never a partial one.

## Tests, plants, gate
- `s16_nameservices_clauses`: `s16_1_c_2` calls `long_lines()`, which checks 40/41-source lines, 300- and
  600-byte groups, and two long file lines plus a long override, each line's own overflow checked. `s16_2_c_3`
  checks 40-row precedence and label tables, parsed and through `GaiConf`. They fail on next by construction: the
  old code refused a line of more than 16 sources or a group of more than 256 bytes, and dropped rows past 32.
  The plants `spec-spill-cut`, `group-256-cap` and `table-spill-cut` restore those failures, and the tests catch
  them. TESTS: no new test file; the existing s16 binary was extended. No allocation-failure test: the switch's
  spill allocations are a few hundred bytes from the heap's existing arenas, so an address-space limit does not
  reach them reliably.
- Plants: `handoff/CONFIG-TABLES-GROWTH-plants.txt`, 9 of 9 CAUGHT (job config-tables-growth-plants1).
- Direct gate (config-tables-growth-gate1, and gate2 on the final head): red only on locale-archive tests
  (locale_data, catalogs_products, locale_products, printf_corpus_products, s07 s7_2_f_0). mei has no
  `/usr/lib/locale/locale-archive`; this is the host change #7846, to be fixed by LOCALE-PER-DIR-LOAD (coordinator
  note on #7938).
- Counts: `validation/percall/nss_config.c` (getaddrinfo "localhost", getpwnam_r "root"), job
  config-tables-growth-percall1, base = the merge base 0495c4684 (instructions per call, glibc / base / new):
  getaddrinfo_localhost 12190 / 16865 / 15422 (-8.6%: one decode of the gai.conf tables, where the old code did
  two full decodes); getpwnam_r_root 3163 / 4809 / 4808. No row is slower.

## Not to redo
- Threading `Pages` as a generic would spread a type parameter through `Ctx`. `Pages` is not object-safe (it has
  an associated type), hence `ChunkPages`.
- No start-up code is touched (the switch is read on the first lookup only), so no start-up rows were counted.

## Next step
Land after LOCALE-PER-DIR-LOAD (#7846). Plan rows: plan/hardcoded-limits.md HL-16, HL-17; D-16.17.
