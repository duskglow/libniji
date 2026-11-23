# LIBC-FREERES handoff (leg 1 -> leg 2)

Branch `task/libc-freeres`, HEAD 954bfb7ee (WIP). Role mei:~/w/s3. Scratch (local): `~/w/.scratch/LIBC-FREERES/`
(vg/*.c, vg/*.sh, vgfree.sh); the same scripts are installed on mei in `~/work/LIBC-FREERES/`.

## Done (leg 1)

* Code (commit 954bfb7ee, NOT yet compiled or tested):
  * `crates/legacy/src/stdio_compat.rs`: `release_buffers` / `release_buffer` (after `flush_all`): under the list lock
    and each stream's lock, a stream's library-allocated buffer goes back when it holds nothing (Writing: `flush`
    first, kept on error; Reading with `unread() == 0`: `purge` then release; read-ahead kept). Leaves the stream as
    before first use (`ensure_buffer` allocates again).
  * `crates/boundary/src/stdio.rs`: `release_buffers()` (cold) calling it with `Glue`.
  * `crates/boundary/src/posix/s03_memory.rs`: `__libc_freeres` = stdio release, then `users::thread_ending`,
    `resolv::thread_ending`, `dlerr::thread_ending` (calling thread's getpw*/netdb table, resolver state, dlerror
    record; each unhooks then frees, so re-use allocates again; idempotent).
  * Tests: `crates/core/tests/s29_legacy_clauses.rs` `s29_4_c_1` rewritten (tmpfile: release after write, re-alloc,
    read-ahead kept, released at EOF, ungetc after); `s03_memory_symbols.rs` comment.
  * Plants: `handoff/LIBC-FREERES-plants.txt` (4 plants, `cargo test -p niji --test s29_legacy_clauses s29_4_c_1`;
    check the package name `niji` is right for crates/core).
* Valgrind recipe: none installed on hosts; `~/work/LIBC-FREERES/vgfree.sh <role> <tag>` builds system debs from the
  role (tools/deb/build.sh, work in target/fw), then mmdebstrap a trixie root with valgrind: stock run (pre.sh),
  swap libniji's libc6/libc-bin/libc6-dev/libc-dev-bin/libc6-dbg, run with dbg, `dpkg -r libc6-dbg`, run without.
  Programs: /bin/true, puts, cxx (iostream + throw), mix (setlocale C.UTF-8, iconv, getaddrinfo, dlopen libm,
  localtime, getpwuid). Output lines `FR <tag> <prog> rc= errors= reachable=<bytes>B/<blocks> lost=` plus each
  reachable record's frames (owners). Log: `~/logs/<label>.log`, details `~/work/LIBC-FREERES/acc-<tag>/out/`.
* Job `libc-freeres-vg-before` (jobrun kind test, started 08:5x on the BASE tree 0495c4684): the BEFORE + stock
  columns. It holds role s3 (sync rc 4 BUSY) until it ends. `libc-freeres-lint` was started on the stale tree and
  killed (SUPERSEDED: ran on the base, not the head).

## Exact next steps (leg 2)

1. `~/bin/jobwait libc-freeres-vg-before` on mei; read its log (FR lines = before/stock table; reachable frames name
   what else libniji keeps: add releases for those owners, or list them with their owner).
2. Sync (rc must be 0), then clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`) and the two
   tests via jobrun; fix compile errors (unchecked: `load`/`store`/`flush`/`Role` names in stdio_compat are used as in
   `setvbuf`; `malloc_compat::inert` may now be used only by `tr_break` - fix its doc comment at line ~322).
3. Records: the difference closes. Retire `docs/differences-from-glibc.md` row `__libc_freeres` (line ~287; hand-
   written doc) and the COMPAT-11 record (edit `records/` TOML, never docs/compat-flags.md which is generated; see
   §10 of developer-flow-reference: a status like retired/withdrawn - check the schema with `cargo xtask records
   check`). New decision: `cargo xtask records new decision D-NEW --series D-29 --story LIBC-FREERES` (body: what is
   released, order, idempotence, read-ahead kept; supersedes ruling #1317's inert design per this brief). Mention
   ruling #1317 reversal as RULING NEEDED (recommend accept: the reference's behaviour; mid-run call stays safe
   for streams; getpw*/netdb results of the calling thread become invalid after a mid-run call).
4. `cargo xtask static`; commit; push; sync; start detached: gate (`libc-freeres-gate`, affected --run), plants
   (`libc-freeres-plants`, `cargo xtask plants handoff/LIBC-FREERES-plants.txt --dir /mnt/zram/claude/LIBC-FREERES/plants-1
   --timeout 600`), and `sh ~/work/LIBC-FREERES/vgfree.sh s3 after` (`libc-freeres-vg-after`) - not beside another
   build in s3: run vg-after AFTER gate/plants or accept the wait.
5. Start-up checks (brief §9: stdio is start-up code): `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask so &&
   NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`; new code is cold/inline(never).
6. Report `handoff/reports/LIBC-FREERES.json` (+ "jobs"), memcheck table (before/after/stock, with and without
   libc6-dbg) here, precheck, handback-check.

## Not to redo
The survey: libc-visible allocations are those through `caller_alloc` (the program's malloc); the plan caches use the
internal allocator (`TryBox`, not seen by valgrind). Per-thread release functions already exist (thread.rs `finish`).
