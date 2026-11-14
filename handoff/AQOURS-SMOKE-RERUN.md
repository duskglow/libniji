# AQOURS-SMOKE-RERUN: the smoke run again on the complete loader (checkpoint 1, 2026-11-14)

**State:** branch `task/aqours-smoke-rerun` (cut from origin/next `f53c74611`), role keke `~/w/n1`. INCOMPLETE:
context checkpoint (~160k) reached while the three full runs were still going. No Liella code changed.

## Done (harness, `validation/smoke/`)
* `smoke.sh --mirror URL --keyring FILE` (default unchanged: deb.debian.org over https with the fetched keyring);
  a non-https mirror is refused without `--keyring`; `run-info.txt` records mirror and anchor.
* Kind `same` (exit 0 and the whole output equal to the reference build's) and `local:DIR` sources.
* New programs: `gawk_ext`, `python3_shared`, `lua_reload` (Debian lua5.4 + `dlclose/lua_reload.c` +
  `dlclose/reloadmod.c`, 50 load/dlclose/reload rounds), `cxx_dlclose` (`dlclose/cxx_host.cc`, `plug.cc`,
  `plug_tl.cc`; the plug-ins must define no STB_GNU_UNIQUE symbol, else the two builds' objects differ).
* `check-elf.sh` accepts the soname of Liella's own loader (objects using `__tls_get_addr` / `_dl_find_object`
  name it; `ld.so --list` shows it resolves to the running interpreter); selftest positive control
  `selftest/tls_plugin.c`.
* make-dfsg recipe builds `lib/` first (F11 fixed: make builds and its suite runs).
* NOT YET RUN on a host: `liella-cc` reads `liella-cc.env` (written by smoke.sh) when the environment lacks
  `LIELLA_SMOKE_SYSROOT`: make's test driver clears the environment, which failed 4 make tests on Liella only
  (`features/archives`, class (c)). Selftest check + plant `cc-env-file-ignored` added, not run.
* Plants `handoff/AQOURS-SMOKE-RERUN-plants.txt`: `aqours-smoke-rerun-plants1` (keke, head 390ebd7) 6 CAUGHT,
  0 SURVIVED, 0 WARN-RC-ONLY/NO-MSG. The 7th plant (cc-env-file-ignored) is newer: rerun the file.

## Runs (evidence; harness of 390ebd7 + products of next f53c74611, release profile, copied to all hosts)
`aqours-smoke-rerun-full-<host>` on keke, mei, shiki (SLOT_SIZE=L), work `~/work/AQOURS-SMOKE-RERUN/full`,
log `~/logs/aqours-smoke-rerun-full-<host>.log` (ends with results.tsv). Trials: `-prep1` (products; rc 1 from
the first cxx trial's check), `-trial2` (lua_reload passes: output identical to the reference).
Seen so far (keke, mei, shiki agree where finished): zlib, xz, sqlite3, sed, diffutils, findutils, gawk, perl
as the first run. grep: the same 3 (F5, F6). coreutils: F1 FIXED (`ls/removed-directory` passes); left:
`misc/stdbuf` FAIL and 7 LD_PRELOAD tests SKIP (F7, DL-21), 2 inotify-race SKIP (F8, DL-23); keke reference
965/0/220, Liella 955/1/229. make: builds now; ref 136/1, Liella 132/5 (4 = the env issue above, (c)).
bash: `run-glob-test` still differs, now the other way: the reference prints the setlocale warning WITHOUT
`strerror`, Liella WITH ": No such file or directory". `repro/setlocale_errno.c` gives errno 2 on both C
libraries on all three hosts (probe `aqours-smoke-rerun-probe1-<host>`), so bash reaches setlocale another way
(likely `setlocale(LC_ALL, "")` with the name in LC_ALL; compare `spec/behaviour/newlocale-mask.md`, D-7.17,
COMPAT-27). `run-read`: 5 different outputs in 5 runs on BOTH builds on every host (needs a tty): noise, (c).
gawk_ext: build failed on BOTH sides on mei (harness, cause not found yet: read the extension/ part of
`results/gawk_ext.glibc.build.log`). cxx_dlclose: rounds 1-3 (static destructors, function-local static,
atexit handlers at dlclose, fresh data on reload, exceptions after unload) identical to the reference; the
remaining difference is in the coordinator's private notes (do not record it in the tree).

## Next step (exact)
1. `~/bin/jobwait aqours-smoke-rerun-full-<host>` on each host; read the tail of each log (results.tsv).
2. Fix gawk_ext's build (both sides fail: harness) and, if cheap, python3_shared's if it fails likewise.
3. Sync, then on each host: re-stage `smoke-tool` from the branch, `smoke.sh --work .../full make gawk_ext ...`
   (the work dir keeps sources) with the mirror and keyring options.
4. Probe bash's setlocale path (`LC_ALL=zh_HK.big5hkscs` + `setlocale(LC_ALL, "")`, both C libraries); classify.
5. RESULTS.md: a second section for this run (date, next f53c74611, CPU classes, not host names), per-host table.
6. Final head: clippy + static, `cargo xtask affected --run --base origin/next`, plants file again.
