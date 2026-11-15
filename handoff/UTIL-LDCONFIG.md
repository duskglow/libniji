# UTIL-LDCONFIG (UTIL-3, SL-LDCONFIG): ldconfig

**State.** Branch `task/util-ldconfig` (from origin/next `8261e29b2`), role keke `~/w/n2`. Run 1 stopped at the
context checkpoint (~150k) after the probes and the first draft of the engine: **WIP, INCOMPLETE, must not land as
it stands.** The engine is not yet wired into a front end; xtask still runs the UTIL-HARNESS stub, so the tree
stays coherent.

## What is on the branch (run 1)

* `crates/ld/src/ldconfig/mod.rs`: the `Fs` trait (stat/lstat/list/read_at/read_file/read_link/replace_link/
  replace_file/error_text/root), `Text` output buffers, S8 helpers `push`/`extend` (try_reserve, then a commented
  `#[allow(clippy::disallowed_methods)]`), argp-style `parse_args` (clusters, `-Cx`/`-C x`, `--long[=x]`, prefixes,
  `--`, operands anywhere), `run::<A: Arch, F: Fs>(argv0, args, fs, generator) -> Outcome`, `root_of`, `flags_text`,
  `print_cache` (moved logic of the stub's `-p`).
* `crates/ld/src/ldconfig/conf.rs`: `ld.so.conf` reader (comments, blanks, `include` with several patterns, globs
  through `expand` + `fnmatch`, relative includes, cycle/depth refusal by file identity, `hwcap` message).
* `crates/ld/src/ldconfig/scan.rs`: collection (`Can't stat`, duplicates by identity), scan (name filter, lstat/stat,
  `elf_info` by positioned reads, truncated, dynamic-linker message, dev-link rule, link rule, `(changed)`), the cache
  through `cache::write` and `Fs::replace_file`, `-c old|compat` refused, `-l` refused, `-p`.
* **Not compiled yet** beyond the check job named in the report (see below); expect clippy fixes.

## Probe-settled (reference ldconfig on keke, black box; scripts and full outputs: keke `~/work/UTIL-LDCONFIG/probe{1,2,3,4}.{sh,out}`)

* Relative `include` under `-r`: `<prog>: need absolute file name for configuration file when using -r`, exit 1,
  nothing written. Without `-r`, a relative include is relative to the including file's directory (not the cwd).
* `include` takes several patterns on one line; a bare `include` is a directory named `include`. `#` comments anywhere;
  blanks trimmed. `hwcap ...` line: `<prog>: <file>:<n>: hwcap directive ignored`. `/opt/a:elf` is a plain path.
* Missing `-f` file, missing include file, pattern without matches, include cycles (a->b->a, self): **silent, exit 0**.
* Directory collection first: `<prog>: Can't stat <dir>: <strerror>` (trusted dirs too); a directory reached twice by
  identity (merged-/usr `/lib -> usr/lib`): ``<prog>: Path `<later>' given more than once`` newline
  `(from <later-file>:<line> and <earlier-file>:<line>)`, first name kept. Order: operands `<cmdline>:0`, conf,
  `<builtin>:0` (`/lib/x86_64-linux-gnu`, `/usr/lib/x86_64-linux-gnu`, `/lib`, `/usr/lib`).
* `-v`: `<dir>: (from <file>:<line>)` per kept dir (also when empty); `\t<soname> -> <file>[ (changed)]`; the
  reference's per-directory library order is NOT sorted and changes between runs (hash order): transcripts must
  keep one library per directory for `-v`, or compare that block as a set.
* Under `-r`, a file reached by an include glob is shown with the root in front (`@ROOT@/etc/ld.so.conf.d/z.conf:1`);
  the main conf as given (`/etc/ld.so.conf`). `-p`, `Can't create temporary cache file` show the root too.
* Names read: `lib*.so*`, `ld-*.so*` (ldconfig(8)); `foo.so.1` ignored; non-ELF silent; an ELF cut at 100 bytes:
  `<prog>: file <path> is truncated` + an empty line. Dangling symlink: `<prog>: Cannot stat <path>: <strerror>`.
* `ld-linux-x86-64.so.2` in a dir: `<prog>: <path> is the dynamic linker, ignoring` + empty line, but it is still
  listed by `-v` and in the cache.
* Link rule: per soname the candidate with the highest NAME under D.3.3 compare wins, symlinks included
  (`libfoo.so.1 -> libother.so.5` where libother.so.5 -> libfoo.so.1.10); two regular files with one soname:
  `libaa.so.1 -> libmm.so.1.0`. A symlink whose name ends in `.so` and is a prefix of its target's soname
  (`libfoo.so`, `libnos.so -> libnos.so.7`, `libdd.so`) gets its own cache entry (key = link name), no link, no `-v`
  line; `libd.so -> libdd.so.5` (not a prefix), `libzz.so`, `libbar.so.1` (no `.so` end), `libcc.so.4.0.so` do not.
* No soname: key = file name, `-v` shows `libnos.so -> libnos.so` (never "changed").
* `-N -v`: links made, `(changed)` shown, no cache. `-X -v`: cache written, no links, no `(changed)`. `-N -X -v`: dirs
  listed but not scanned. `-n -v DIR`: only DIR, no cache; `-n` without operands: nothing, exit 0. Operands without
  `-n` are scanned first, then conf and trusted.
* `-p -N`: prints. `-c old`/`-c compat` write `ld.so-1.7.0` files; `-c bogus` exit 0. `-r` without arg:
  `option requires an argument -- 'r'`; `-Q`: `invalid option -- 'Q'`; then ``Try `ldconfig.real --help' or
  `ldconfig.real --usage' for more information.``, exit 64. `--usage` text recorded in probe2.out (Liella's matches it).
* Unwritable/missing cache dir: `<prog>: Can't create temporary cache file <root><cache>~: <strerror>`, exit 1.

## Decisions to record (pending, `records/pending/UTIL-LDCONFIG-1.toml`, series D-31 like UTIL-HARNESS; not written yet)

Deterministic order (sorted listings and groups), atomic link replacement, positioned bounds-checked ELF reads, native
class only (32-bit/x32 libs skipped: INFORMATIONAL, the reference records i386 libs as `libc6`; decide in run 2),
include cycle message (exit status unchanged; differences row: stderr line only, no compat flag: nothing parses it),
`-c old|compat` refused (spec UTIL-SPEC-6 row exists?, check `docs/differences-from-glibc.md`), `-l` refused for now.

## Next step (run 2)

1. Sync, `cargo clippy -p liella-ld --all-targets -- -D warnings`; fix.
2. Replace `xtask/src/transcript/stub.rs` by a host `Fs` (std; `-r` resolves every path inside the root component by
   component, absolute symlink targets re-rooted, `..` clamped, ELOOP after 40) calling `liella_ld::ldconfig::run::<Native, _>`;
   generator `ldconfig (Liella) 2.39` (Q3 open: `normalise:` it in transcripts); update docs/transcripts.md.
3. Harness: an `elf: <path> soname=<s>` setup item (a minimal ELF64 ET_DYN with PT_LOAD + PT_DYNAMIC + DT_STRTAB/
   DT_STRSZ/DT_SONAME) so transcripts need no compiler; record transcripts `-n`, `-v` (one lib per dir), `-N`, `-X`,
   dev links, include globs, duplicates, truncated, `-p` of a built cache, hwcaps (probe its `-v` lines by recording).
4. Unit tests (`crates/ld/tests/ldconfig.rs`, in-memory Fs): conf parser, fnmatch, cycles, link rule, determinism
   (two listing orders, identical bytes), a seeded random-conf loop (no panic, bounded); differential: reference
   loader resolves through a Liella-ldconfig-written cache (reuse `crates/ld/tests/cache_write.rs`'s namespace method).
5. Packaging: `tools/deb/system.sh` libc-bin postinst `triggered)` runs ldconfig (replace the documented no-op).
6. Plants on conf.rs and scan.rs through `cargo xtask plants`; direct-tier gate; then the freestanding static product
   (a separate story: `liella-ldconfig` product crate with a syscall `Fs`, like ld-so).

Read: this note, plan/utilities.md §2.3/§4/§5, spec §31.2 and D.3, handoff/UTIL-HARNESS.md, docs/transcripts.md.

## Rule slips (mine)

Two empty `python3 - <<'X'` heredocs typed by reflex before `python3 -c` calls (no effect); none since.
