# SL-HEADERS-S18 handoff: slice 18, fortify (`_FORTIFY_SOURCE` 1..3)

**State:** branch task/sl-headers-s18 (from origin/next f15d8264f), role keke:~/w/n1. Run 1 ended after leg 3:
generate done (code 48d3c9b53, unchanged in leg 3); proofs on the final head: keke:sl-headers-s18-xtask4 (`cargo
xtask so && cargo test -p xtask`, 610 passed, 0 failed) and keke:sl-headers-s18-plants3 (2 of 2 CAUGHT, no WARN).
Hand-back: INCOMPLETE (generate done; P7 open) per the brief's two-run plan.

**Rulings in force**
* #7599 (coordinator, 11-22): slice 18 owns the 171 fortify rows of P3 (85 names) and every `*_chk` export reached.
  The 55 other -O2 rows (27 names, below) are HEADERS-O2-INLINES. `*_chk` prototype rows by probe are table data.
* Slice contract (plan/headers.md 4.1): slice files only (`xtask/src/headers/slices/s18.rs`, `headers/fortify.tsv`,
  template `@@fortify` lines). No edit of headers.rs, headers/*.rs outside slices/, header_check.py.

**Done**
* `spec/abi/chk-prototypes.tsv` (71 rows, abi_extract `--prototypes-fortify`) and `headers/fortify.tsv` (leg 1).
* `xtask/src/headers/slices/s18.rs`: marker `@@fortify` (after `@@emit`; in stdio.h at the end, after `stdout`).
  Per function `@@emit` declared here and fortify.tsv lists: `#if (<its condition>)[ && __NIJI_GNU] &&
  __NIJI_FORTIFY >= L` (L and the GNU-only case from the callsites.tsv `_chk` rows; their configs are the call
  probes' own ids fortifyL[+gnu|+gnu64], not ftm-configs ids); `_chk` declaration from chk-prototypes.tsv;
  gnu_inline always_inline wrapper; plain call through `__niji_plain_<name>` asm alias (a self-call in the inline
  body compiled to an endless loop at -O2: own probe); pread picks `__pread64_chk`/pread64 under `__NIJI_FOB64`;
  longjmp/_longjmp/siglongjmp redirect to `__longjmp_chk`. `__NIJI_BOS` = dynamic object size at level 3.
  FD_* (sys/select.h) already call `__fdelt_chk` at every level: no marker there.
* Templates with `@@fortify`: string, strings, stdio, wchar, unistd, stdlib, sys/socket, poll, syslog, setjmp.
* Checks (keke): all 13 headers compile alone at levels 1-3 x C/C++ x plain/GNU/GNU+FOB64 with -Werror (job
  sl-headers-s18-p3c, 0 failed). P3 (`~/work/sl-headers-s18/p3.sh p3c`): 226 -> 58 lines; fortify lines 171 -> 1.
  The one left: `pread64 call __pread64_chk [!fortify1,fortify2,fortify3]` missing. Cause: the call probe's first
  form spells `__off64_t`, which libniji's unistd.h does not declare, so it falls back to constant `0` arguments; a
  constant size 0 provably fits, and the wrapper takes the plain call. A non-constant size reaches `__pread64_chk`
  (own probe `~/work/sl-headers-s18/pr.sh`). Probe-input artefact: INFORMATIONAL, not a header defect.
* Test `headers::slices::s18::tests::a_deliberate_overflow_aborts_and_an_exact_fit_runs` (read, memcpy, level-3
  heap memset: 8 into 8 runs, 9 aborts with SIGABRT) green; plants file `handoff/SL-HEADERS-S18-plants.txt`.
* Devuan excalibur dpkg-buildflags (job sl-headers-s18-buildflags4; the fix: the keyring is staged in a 0755
  mktemp dir, as tools/deb/proof.sh does; the namespace root cannot enter the 0750 home): vendor Devuan, dpkg
  1.22.22, CPPFLAGS `-Wdate-time -D_FORTIFY_SOURCE=2`, CFLAGS `-g -O2 -Werror=implicit-function-declaration
  -ffile-prefix-map=/=. -fstack-protector-strong -fstack-clash-protection -Wformat -Werror=format-security
  -fcf-protection`; hardening fortify=yes. (Ubuntu 24.04 on the build host gives level 3: not Devuan's.)

* 48d3c9b53: the plain alias is spelled from the prototype (`__typeof__ (getwd)` warned: getwd is deprecated, and
  header_check P1/P5 failed under -Werror, seen as the s12 test red in job sl-headers-s18-xtask). After the fix
  (job sl-headers-s18-fix2, keke): header_check.py 0 failed of 13677, s18 test, clippy -D warnings, static green.

**SUPERSEDED jobs:** sl-headers-s18-xtask (36acf5afe: s12 test red, getwd, fixed), sl-headers-s18-plants (36acf5afe,
killed when the fix landed), sl-headers-s18-xtask2 / -plants2 (started after a sync that returned BUSY, so on an
unsynced tree: killed; leg 3 killed by PID the orphaned `cargo test -p xtask` they left in ~/w/n1),
sl-headers-s18-xtask3 (rc 101: only `xtask/tests/transcripts.rs` failed, 2 tests, `target/niji-test/iconv` from
11:03 is present but `target/niji/` (its ELF interpreter) was gone from the role, so iconv could not be run (ENOENT);
not this change; xtask4 rebuilt the products first and is green). `cargo xtask affected --list` says `FULL: the train's (headers/*.in, fortify.tsv: generated
table inputs)`: the direct gate is `cargo test -p xtask` (kind test) + precheck.

**INFORMATIONAL:** sync_role BUSY at 13:5x: a shell of DL-PROCESS-OBJECTS-3 (`cd ~/work/DL-PROCESS-OBJECTS-3; diff ...
~/w/n1/target/product/fast/layout-bins.tsv ...; jobwait dl-process-objects-3-startup`) reads this story's role dir.

**Not done / exact next step (run 2)**
1. P7 (plan/headers.md: the smoke package corpus against libniji's headers) with the Devuan flags above: not run.
   `validation/smoke` has no way to pass build flags today: `programs.sh` sets CFLAGS itself in some builds
   (zlib `CFLAGS="-O2 -g"`), the configure builds take CFLAGS/CPPFLAGS from the environment. Both sides (reference
   and `--niji-headers`) need the same dpkg-buildflags so `undef-compare.sh` can compare the `_chk` sets. That is a
   shared-tooling edit (own commit, named in the hand-back, plants on it); needs `NIJI_FULL_PROFILE=1 cargo xtask so`.
2. P1/P5 `header_check.py --include libc-include` on the final head: 0 failed of 13677 on 48d3c9b53 (fix2); the
   code has not changed since.
3. HEADERS-O2-INLINES (not slice 18's): stdio.h feof_unlocked ferror_unlocked fgetc_unlocked fputc_unlocked
   getc_unlocked getchar getchar_unlocked getline putc_unlocked putchar putchar_unlocked; stdlib.h abs atof atoi atol
   atoll bsearch div; string.h basename memrchr rawmemchr strcasestr strchrnul; strings.h index rindex; wchar.h
   mbrlen. Reference targets: their callsites.tsv call rows.

Full report: handoff/SL-HEADERS-S18-report.md is not written; this page and handoff/reports/SL-HEADERS-S18.json are it.
