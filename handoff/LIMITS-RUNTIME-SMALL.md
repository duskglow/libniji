# LIMITS-RUNTIME-SMALL: handoff (leg 1 → leg 2)

State: branch `task/limits-runtime-small`, base origin/next 5e1903cce. Role keke:~/w/r1.
Leg 1 pushed one WIP commit: code and Rust tests written, **NOT YET COMPILED** (no cargo on the
agent host; nothing built on the role from this code yet).

## Done in leg 1 (WIP commit)
- `crates/rt/src/chunks.rs` (new, `pub mod chunks` in rt/lib.rs): `Chunks`, a table of `w`-word
  entries in chunks that never move (chunk k = PAGE << k bytes, mapped on first need through
  `Pages::map`; directory of 48 addresses spans 2^60 bytes, so no limit but memory). API: `place`,
  `get` (lock-free, acquire), `reserve` (owner serialises; false without memory), `mapped`,
  `unmap_all` (tests).
- HL-12 `legacy/src/exit_handlers.rs`: `MAX_ENTRIES` and the 32 MiB extension removed; `ext: Chunks`
  (4 words per entry); `Registry::mapped()`, `Registry::unmap(pages)` added (tests).
- HL-8 `boundary/src/fmtmsg.rs`: the 256-entry static `ADDED` replaced by `Chunks` (3 words: level
  bits, string addr, len) + `USED` count; free entries (level 0) reused first; still under `LOCK`
  (the audit said fmtmsg reads it without a lock: in the code it reads under `LOCK`; chunks are
  safe either way). No inline area: the common case adds no severity (first addseverity maps 1 page).
- HL-9 `boundary/src/gai_async.rs`: queue = ring of `CAP` slots, 64 inline (`QUEUED`) + `QUEUE_MORE`
  chunks; a full ring grows by the next chunk and moves the wrapped part `0..HEAD` past the old end
  (`grow`, cold). HEAD now kept in `[0, CAP)`. Groups: 16 inline (`GROUP`, 6 words each) + `GROUP_MORE`
  chunks (read lock-free by finish/notify/notifier, so never moved); `group()` maps more under `LOCK`.
  Memory exhaustion is now `EAI_MEMORY` (group or queue growth failed; NOWAIT requests get
  `EAI_MEMORY` too); no worker stays `EAI_AGAIN`. Fork child: HEAD/LEN reset, memory kept, all
  mapped groups freed. `MAX_WORKERS` 32 unchanged (policy).
- Tests: s01 `s1_2_i_3` (100 handlers map 4096 bytes via `CountingPages`; 3 M handlers run once each
  most recent first, `Descending` CCall), `s1_2_f_1` (limit 3 pages: 416 ok, 417th fails cleanly,
  416 run in order). s02 `s2_6_c_3_table`: 2000 levels, printed by fmtmsg, replace, remove, reuse.

## Next steps (leg 2), in order
1. Sync (`sync_role.sh --story LIMITS-RUNTIME-SMALL --base origin/next <wt> keke r1`), then on r1:
   `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo xtask static`; fix.
   Watch: `thread_words()` type in gai `group()` (stored as u64 directly), unused imports.
   Baseline products of next were built by job `limits-runtime-small-so-before` (check it ended
   rc 0; the next `xtask so` overwrites target/niji, so run the reference/before probes first).
2. gai test (brief: 20000 pending at once, 5000 groups): only testable in products (no libniji
   workers in cargo tests). Plan: new scenario `gaimass` in `crates/core/tests/oracle/resolv_diff.c`
   (scenario table ~line 86 with gaia's conf `nameserver 127.0.0.1 / timeout:3 attempts:1`, add to
   `resolv_run.sh`'s default list, dispatch near line 1305, no `show_log`): NOWAIT `slow.test`
   first (the single-threaded server sleeps 1.5 s on it, so every worker blocks), usleep 100 ms,
   then one NOWAIT call of 20000 `a.test` requests and 5000 NOWAIT calls of 1 request with
   SIGEV_SIGNAL on SIGRTMIN (handler counts); print return codes summary, `pending>=19000: yes/no`
   right after submission, then wait each (gai_suspend loop), print ok/other counts and signals
   == 5000. Must match the reference line by line (dns_products `resolv_equals_the_reference`).
3. Probes before/after vs reference (scratch, not the tree): small C program for `atexit`/`on_exit`
   3 M in order and `addseverity` 2000 + fmtmsg; link static against `target/niji/libc.a`
   (`cc -static -nostdlib niji-crt1.o p.c libc.a -lgcc`) and against the host libc.
4. Records: amend D-2.6 and D-2.10 (F3) (no 256 cap), D-1.2 (exit registry chunks), D-16.8 (queue,
   groups, EAI_MEMORY); grep docs/spec for "256 added", "4096", "MAX_ENTRIES", "2^20". Mark HL-8/9/12
   done in plan/hardcoded-limits.md if that is the convention. Plants in docs/plants.md (each cap
   restored alone: e.g. `reserve` refusing past chunk 0 for each user; grow() returning false).
5. Gate (`cargo xtask affected --run` under ~/bin/gate), plants, startup counts (start-up row
   `cbench_startup`; exit registry is start-up adjacent: §9 of developer-flow-reference, layout-bins
   diff, `cargo xtask test`, NIJI_FULL_PROFILE hot_text_products), precheck, report json, handback-check.

## Informational found
- `gai_cancel(NULL)` cancels at most 64 queued requests per call (fixed array in `gai_cancel`); not
  in the audit; a list of more stays queued. Candidate follow-up.
- Leg 1 wasted one call on a stray `python3 -` (killed by PID).
