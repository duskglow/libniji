# SETLOCALE-CTYPE-ENV: handoff (after SETLOCALE-CTYPE-ENV-5)

State: branch `task/setlocale-ctype-env-5` (runs 1-4, then this run's three commits on 874d17d92). Build role
keke:~/w/n2. Decision: records/decisions/D-X/D-X.93.toml. Percall spec: validation/percall/setlocale.c.
Ruling #5298 (coordinator): no EnvMemo for setlocale(cat, ""); the environment is read on every call.
Not merged with next since 8f3e2993e (next was f6cab48d7 at this run; the brief allowed no merge).

## SETLOCALE-CTYPE-ENV-5 (this run): the coreutils-shaped start-up fault, found and removed

- Step 0: clippy + static on the inherited head 874d17d92: green (job setlocale-ctype-env-5-lint0).
- Measured (keke, Zen 1, fast products, `page_faults` linked as hot_text_products links it, `ADDR_NO_RANDOMIZE`,
  `perf record -e page-faults -d`; script ~/work/SETLOCALE-CTYPE-ENV-5/faultset.sh on keke): `relay tool`
  next 8f3e2993e 50, inherited head 51. Diff of the fault lists: one extra `.text` fault, `__printf_chk`'s
  page. Cause: the traced hot text group ends right before a 32 KiB-aligned boundary (next: ends 0x477e5b,
  421 bytes spare before 0x478000). The branch made the group 682 bytes too long, so everything after it
  (`__liella_libc_strlen` and on) moved by 32 KiB, and `bindtextdomain`, `__printf_chk` and `main` landed
  in one more 64 KiB fault-around window. Growth: `setlocale_env` 3093 bytes (its 14-value loop unrolled into
  ten inlined `strlen` sequences, and the lookup closure inlined at four sites), against next's
  `setlocale` + closure 1857.
- Fix 1 (696e56fcf): keep the values' addresses and measure a value only when the selection reads it.
  Measured: still 51, because this rebuild had LLVM inline the 12 KiB `time::format::format<u8>` (strftime's
  general fallback, cold) into the traced `strftime` (codegen-unit churn, no source change there).
- Fix 2 (e91db4e85): the lookup is one `#[inline(never)] fn env_lookup`; `time::format::format` is
  `#[inline(never)]`. Measured: tool 50 (every run identical), group ends at 0x477e5b, the same as next.
- Percall (release, keke, instructions/call, base 874d17d92 -> e91db4e85; glibc): ctype "" 517 -> 568 (2656);
  ctype "" LANG only 839 -> 880 (6421); all "" 1047 -> 1083 (11865); all "" LC_ALL empty 4833 -> 4386 (22275);
  all "" LANG only 4298 -> 3824 (21739); C/POSIX/query rows unchanged. The single-category rows pay a call per
  lookup (+36..+51), the cost of the out-of-line lookup; every row stays far below next (run 2: 8257/7070/8801)
  and glibc. Job setlocale-ctype-env-5-percall2.
- Gate (CI's ci_gate_cmd as of 11-13, without LIELLA_TRAIN: static, `affected --tier full --run`, release
  products + hot_text_products): job setlocale-ctype-env-5-gate on keke: running at hand-back (named in JOBS for the dispatcher).
- Plants: handoff/SETLOCALE-CTYPE-ENV-5-plants.txt, job setlocale-ctype-env-5-plants: rc 0: lc-all-dropped and known-off-by-one CAUGHT by setlocale_env_is_read_on_every_call, formatter-inlined EQUIVALENT (layout only); WARN-DEV-SHM only.

## Open / next step

- The hot group still has only 421 bytes before its 32 KiB boundary on next itself; any traced growth of that
  size re-adds this fault in any story. Worth a layout story (headroom or a boundary-independent window
  count), not more trimming here.
- If CI's merge with a later next goes red on a start-up bound, measure with faultset.sh first (one-line diff of
  the fault lists, then `nm -nS | grep -B1 __liella_libc_strlen` for the group end).
- A cost bound in setlocale_env_repeat (counts printed only); a cheaper LANG-only walk (ruling #5298 rules out
  a memo).

## Not to redo

The kept path is right for LC_ALL empty; #5188 is settled. The 51st fault is understood (group end vs the
32 KiB boundary); don't re-profile it.

## Notes

- Full report: handoff/reports/SETLOCALE-CTYPE-ENV-5.json; earlier runs: handoff/reports/SETLOCALE-CTYPE-ENV-*.json.
- Superseded jobs: setlocale-ctype-env-5-fs-head, -fs-head2, -fs-next (diagnostics), -lint0 (step 0,
  on the inherited head), setlocale-ctype-env-5-percall (stopped by me with kill-label: it was counting the intermediate commit and blocked the sync).
