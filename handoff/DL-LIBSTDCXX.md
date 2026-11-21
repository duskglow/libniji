# DL-LIBSTDCXX handoff (GitHub #4: ld.so refused Debian's libstdc++.so.6), after DL-LIBSTDCXX-4

State: branch `task/dl-libstdcxx-4` (= task/dl-libstdcxx-3 plus a merge of origin/next 921b3bc91), code head
`55fd3117f` (the merge) plus this note and the report json. Role keke:~/w/n1. Report:
handoff/reports/DL-LIBSTDCXX-4.json; plants handoff/DL-LIBSTDCXX-2-plants.txt (10 plants, unchanged).

## Cause and fix (settled; do not reopen)

- The start's fixed `UniqueTable::<64, 4096>` overflowed (libstdc++ binds 106 `STB_GNU_UNIQUE` names) and
  `LookupError::NoMemory` printed as "malformed symbol tables". The start now binds by global scope order
  (`symbols::ScopeOrder`, no table) (DL-LIBSTDCXX).
- dlopen's unique table is `reloc::GrowTable` over two `Stack`s in anonymous mappings grown by `mremap`; out of memory
  prints `symbols::UNIQUE_NO_MEMORY`. Tests crates/ld/tests/unique_table.rs, unique_start.rs; decision D-27.55
  (records/decisions) (DL-LIBSTDCXX-2).
- tools/deb: root-system.sh links iostream-so and throwmain (exception across libthrowlib.so) against the build
  system's shared libstdc++.so.6, AFTER-linked so `--as-needed` keeps libthrowlib.so (DL-LIBSTDCXX-3); swap-cmds.sh
  runs `apt-get check` (only the `+liella` libc6 version lines are dropped, in both roots; raw output in apt-1.raw),
  `apt-cache policy`, `apt-cache policy bash` in the trixie roots (DL-LIBSTDCXX-2/-3).

## DL-LIBSTDCXX-4 (this run): merge and rerun

- Run 3 was green but CI could not merge: next's 5ffe048a (DL-LD-PRELOAD-3) appended the fakeroot cases after
  find-5 in tools/deb/swap-cmds.sh, where this branch had appended the apt cases. Merge 55fd3117f keeps both blocks
  unchanged (apt first, then fakeroot exactly as next has it); `git diff origin/next -- tools/deb/swap-cmds.sh` shows
  only this branch's header line and apt block. No other conflict; the plant patterns still match once each.
- lint0 (clippy + static on the inherited head 92bf7dec0): green.
- keke has no Debian keyring in /usr/share/keyrings: the swap needs `KEYRING=~/keyrings/debian-archive-keyring.gpg`
  (the claude user's copy). dl-libstdcxx-4-deb (no KEYRING) ended rc 1 with swap-system rc 2 ("no keyring"), root-system
  15/0; superseded by dl-libstdcxx-4-deb2.

Results on 55fd3117f (keke):
- dl-libstdcxx-4-gate rc 0: static ok, so, unique_table/unique_start/dlsym, affected --base origin/next: 8 suites,
  598 passed / 0 failed, peak 2.2G.
- dl-libstdcxx-4-plants rc 0: 10 plants, 10 CAUGHT, 0 SURVIVED, no WARN.
- dl-libstdcxx-4-deb2 rc 0: root-system pass=15 fail=0 (iostream-so and throwmain against the shared
  libstdc++.so.6: "throwmain: caught: throwlib: value 3 (throwlib)"); trixie swap same=52 differ=0: apt-1/2/3 and
  fakeroot-1/2 PASS (Liella root: fakeroot inside 0 0 0:0, outside 65534; tar 0/0 under fakeroot, 65534 outside).
- dl-libstdcxx-4-startup and dl-libstdcxx-4-counts: still running at hand-back (`cargo xtask test` on a 4-core slot,
  153/6669 after 45 min); named in JOBS for the dispatcher. Last measured counts (run 3, mei, dynamic C hello, perf
  -r 30): instructions:u 8925508 -> 8924220, page-faults 94 -> 92; the merge changes no loader code of this branch.

## Jobs (keke, head 55fd3117f; scripts in ~/work/DL-LIBSTDCXX-4 on keke)

chain.sh runs in order `dl-libstdcxx-4-gate` (static, so, unique_table/unique_start/dlsym, affected --base
origin/next), `dl-libstdcxx-4-startup` (`cargo xtask test`, FULL_PROFILE so + hot_text_products, plain so),
`dl-libstdcxx-4-counts` (validation/wl_count.sh . seq, base = merge-base 921b3bc91). In parallel `dl-libstdcxx-4-deb2`
(build.sh --flavour system, root-system.sh, swap-system.sh trixie with KEYRING) and `dl-libstdcxx-4-plants`.

## Open / next

Next: land (RELEASE-SNAPSHOT-CXX waits on it). Optional follow-up: a test that drives dlsym's out-of-memory text end
to end (needs a mapping failure on demand). Do not redo: the cause hunt, the unique-table design, the merge.
