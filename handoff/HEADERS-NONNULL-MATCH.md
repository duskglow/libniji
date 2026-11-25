# HEADERS-NONNULL-MATCH handoff (end of leg 2; WIP)

**State:** branch task/headers-nonnull-match, head: `git log -1` (WIP commit "probe nonnull through (F)(..)").
Role keke:~/w/n2. keke scratch `~/work/headers-nonnull-match/` (job scripts job6.sh, zstd.sh, nnonly.py, summ.py).

## Cause (fixed in spec/tools/attr_probe.py)
The nonnull column comes from attr_probe's `n.k` tests; five probe defects made it wrong:
1. function-pointer arguments were a literal `0` on every line (pthread_create got 2,4, on_exit 2): now `v[j]`;
2. numeric arguments were `1`, waking `access` attributes (write 2, fwrite 1, strxfrm 1, regexec 4, regerror 3, the
   `*_r` buffers): now 0;
3. builtin knowledge (printf/scanf formats, fputs, nan*, dcgettext 2): the n/rn unit compiles with `-fno-builtin`;
4. coverage: recursive header walk, `locale_t` and pointer-typedef parameters (qsort 4, bsearch 5, statfs, rlimit, statx,
   strto*_l ...);
5. (leg 2) **a function-like macro hid the call**: the reference's C headers at -O2 turn `fread_unlocked`/
   `fwrite_unlocked` calls into code -Wnonnull does not see; a parenthesised `(fwrite_unlocked)(..)` and every -O0
   call warn on argument 4 (probe ul.c, fortify on and off). `n.k` now calls `(F)(..)`.

## Done in leg 2
* Table from job tbl3 committed + headers regenerated (56 now nullable, 47 now nonnull, list: nndiff.py in scratch).
  The fortify split leg 1 reported (atoi, feof ...) was an old-probe artifact: those warn with fortify on and off.
* `check --kinds n,rn` compares only -Wnonnull / the rn marker (MEASURES); one-sided ERR = "not declared alike".
* New column `nonnull_fortify` (`__NIJI_NONNULL_F`, features.h.in) next to `nonnull_nofortify` (`__NIJI_NONNULL_NF`);
  `nonnull` = marked in both. Generator: xtask/src/headers.rs attr_text.
* `table --columns nonnull*` runs only the nonnull tests. New xtask test `nonnull_columns_are_the_probes`
  (xtask/src/headers/slices/s19.rs): regenerates the nonnull columns into a copy and requires the committed table.
* Unit tests (item 3) added to existing tests: getentropy(NULL,0), write/pwrite NULL 0, fwrite NULL with 0, re_match_2
  NULL empty first string. Already covered: strxfrm NULL 0, regerror NULL 0, regexec nmatch 0, re_search_2 NULL,
  getgrnam_r NULL 0, pthread_create NULL attr/arg, snprintf(NULL,0). libniji's code needed no fix (conv::check:
  length 0 is an empty slice before any NULL test). printf/fputs/nan with NULL are UB: not tested, not probed.
* Job chk6 (table with the fortify split, before the (F) fix): base/gnu/default/fortify2 0 differ; c++ 2, c++-fortify3
  1 (fread/fwrite_unlocked 4: the macro finding above). **zstd: libniji 41 files 0 failed, -U and -D_FORTIFY_SOURCE=2
  (reference too).**
* Plants file: handoff/HEADERS-NONNULL-MATCH-plants.txt (7 plants; patterns checked unique).

## Exact next step
1. `~/bin/jobwait headers-nonnull-match-chk7` (keke; table with the (F) probe -> role's spec/abi/diagattrs.tsv,
   headers-gen, six `check --kinds n,rn`, full base check, zstd; logs `~/work/headers-nonnull-match/out7/`). Copy back:
   `"${VSSH[@]}" 'cd ~/w/n2 && tar cf - libc-include spec/abi/diagattrs.tsv' </dev/null | tar xf -` in the worktree;
   commit. Expect fread/fwrite_unlocked nonnull 4 and every config 0 differ.
2. If `nonnull_fortify` and `nonnull_nofortify` both come out empty: decide whether to keep the machinery (then the
   three fortify plants SURVIVE: drop them) or remove the columns/macros (simpler; the six-config check stays the guard).
   Recommendation: remove, note in the report that fortify does not change the reference's nonnull sets.
3. **Existing test `diagnostic_attributes_match_the_reference` (xtask s19) will likely be RED**: the full base check
   reports 205 differences, none -Wnonnull: deprecated (inet6_option_*, ns_*, ftime, inet_neta), wur (getrandom,
   preadv*, pwritev*, strdup/strndup on n lines), pure (gnu_dev_*, nan* -Wunused-value), execl sentinel and
   v*printf -Wformat-extra-args on n lines, is*_l macros (ERR). Mostly newly visible through the recursive header
   walk. First check whether it is green on origin/next (run the test there or the old probe). Then either fix the
   cause per attribute (stale table columns: regenerate those columns with `--columns deprecated,wur` and check) or,
   if it needs more than this story, RULING NEEDED. Never loosen the test.
4. clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`), `cargo xtask static`, gate
   (`cargo xtask affected --run`, kind gate), plants (`cargo xtask plants handoff/HEADERS-NONNULL-MATCH-plants.txt
   --dir /dev/shm/claude/headers-nonnull-match/plants-1`), precheck, report json, handback-check.

## For the report
* INFORMATIONAL: the reference marks warn_unused_result only under _FORTIFY_SOURCE (gnu config: atoi, a64l, bsearch,
  chdir ... warn on libniji only); libniji marks it always: a -Werror=unused-result build without fortify fails on
  libniji only. Follow-up story recommended (same probe, wur per config).
* INFORMATIONAL: libniji has no fread_unlocked/fwrite_unlocked macro: a direct C call with a literal NULL stream at -O2
  warns on libniji, not on the reference (declarations equal).
* INFORMATIONAL: c++: 55 functions build their test lines on the reference only (chmod, chown, frexp, getgroups ...).
* COORDINATOR NOTE 15:51 (package sources only from the local mirror): zstd 1.5.7 in `zstd-dl` was fetched in leg 1,
  before the note; its origin is not recorded beyond the sha256 (eb33e51f...). Say so in the hand-back; redo from the
  mirror recipe (RELEASE-SNAPSHOT-1125B ystage2.sh/yapt.sh on mei) if the coordinator asks.
* Leg 2 started chk6 with `test -s .. && setsid ..` chained: the local ssh held the channel and was killed at 10 min
  (the job itself ran to the end). Start jobs bare.
