# UTIL-LDCONFIG (UTIL-3, SL-LDCONFIG): ldconfig

**State.** Branch `task/util-ldconfig-2` (run 2, stacked on run 1's `task/util-ldconfig`), role keke `~/w/n2`.
Run 2 stopped at the context checkpoint (~155k): **WIP, INCOMPLETE, must not land as it stands** (one transcript
mismatched; unit tests written but not yet run; no plants; packaging not done).

## Done (runs 1 and 2)

* Engine `crates/ld/src/ldconfig/{mod,conf,scan}.rs` (run 1), corrected in run 2 by recorded probes (below).
* Front end `xtask/src/transcript/ldconfig.rs` replaces the UTIL-HARNESS stub: host `Fs` over std, `-r` resolved
  inside ROOT component by component (`..` clamped, absolute link targets re-rooted, ELOOP after 40), atomic link
  replacement (temp link + rename), cache through `<cache>~` + fsync + rename. Generator `ldconfig (Liella) 2.39`.
* Harness: `elf: <path> [soname=<s>] [cut=<n>]` setup item (minimal ELF64 ET_DYN, no compiler) and
  `as-cache: <path>` (a written cache compared as its entry list, generator excluded). docs/transcripts.md NOT yet
  updated for these two (do it).
* 9 new transcripts in validation/transcripts/ldconfig/ (recorded with the reference): n-links, r-conf-cache,
  N-no-cache, X-no-links, NX-list-only, bad-files, r-relative-include, cache-dir-missing, usage-error. Each has
  `normalise:` lines for `/sbin/ldconfig.real` and `ldconfig.real` (the reference runs through Debian's wrapper).
  Check on keke (job util-ldconfig-2-rec3): **11 ok, 1 mismatched (r-conf-cache)**.
* `crates/ld/tests/ldconfig.rs`: in-memory Fs; conf order/includes, cycles, link rule, dangling links, hwcaps,
  determinism (reversed listing), relative include under -r, seeded 3000-case conf fuzz. **NOT RUN yet.**
* Cache writer (`crates/ld/src/cache/writer.rs`): a hwcaps entry's path may end in any valid file name.

## Probe-settled in run 2 (transcript record = black box; probe files keke `~/work/UTIL-LDCONFIG-2/probe/`)

* Directories are stat'ed and de-duplicated as they are named: `Path ... given more than once` and `Can't stat` come
  between the reader's own messages, in configuration order (engine now does this in `conf::Reader::add`).
* `-N -X -v` still scans and prints `\t<soname> -> <file>` lines (run 1's note was wrong).
* `glibc-hwcaps/<name>` subdirs are scanned AFTER their parent; `-v` header `<dir>: (hwcap: "<name>") (from f:l)`;
  no links there; the cache entry key is the soname, path the real file (`.../libdd.so.4.0`).
* Dangling `lib*.so*` links: `Cannot stat` only with `-v`; removed unless `-X` or the name ends in `.so` (also with
  `-N`, `-n`). Not probed: dangling links inside a hwcaps dir (Liella does not remove there).
* A fake `ld-linux-x86-64.so.2` (minimal ELF, e_entry 0) draws NO "is the dynamic linker" message, while run 1 saw
  it for the real ld.so. Rule unknown; dropped from bad-files. Open question (probe with a nonzero e_entry / PT_INTERP).

## Next steps (run 3), in order

1. **Fix the writer for hwcaps entries whose file differs from the key**: today it stores the key as the tail of
   the path string, giving key `bdd.so.4.0` (r-conf-cache's only mismatch). Store such a key on its own; add a
   cache_write.rs case (hwcaps entry, file != key, read back by the loader's reader).
2. Run the check script (keke `~/work/UTIL-LDCONFIG-2/rec.sh`, operands = transcripts to re-record): transcript check, clippy, then
   `cargo test -p liella-ld --test cache_write --test ldconfig`; fix what fails.
3. docs/transcripts.md: document `elf:` and `as-cache:`; update the stub paragraph (line ~19).
4. Records: `records/pending/UTIL-LDCONFIG-2-1.toml` (`D-NEW`, series D-31): deterministic order, atomic links,
   positioned ELF reads, native class only, include-cycle message (differences row: stderr line only, no compat flag),
   `-l` refused. `-c old|compat` already has its row (UTIL-SPEC-6).
5. Packaging: `tools/deb/system.sh` libc-bin postinst `triggered)` runs ldconfig; libc6 `activate-noawait ldconfig`,
   libc-bin `interest-await ldconfig`.
6. Differential: reference loader resolves through a Liella-ldconfig-written cache (reuse cache_write.rs's namespace
   method). Plants (`cargo xtask plants`) on conf.rs (cycle check, sorted globs, relative base) and scan.rs (version
   compare direction, dev-link rule, dangling removal, hwcaps no-link); direct-tier gate.

Read: this note, plan/utilities.md §2.3/§4/§5, spec §31.2 and D.3, docs/transcripts.md.
