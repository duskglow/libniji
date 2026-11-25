# HEADERS-NONNULL-MATCH handoff (leg 1 of the run; WIP)

**State:** branch task/headers-nonnull-match, head: see `git log -1` (WIP commit). Role keke:~/w/n2. Scratch:
`$S/work` = the run's scratch dir; keke scratch `~/work/headers-nonnull-match/`.

## Cause (found, fixed in spec/tools/attr_probe.py)
The nonnull column of `spec/abi/diagattrs.tsv` comes from attr_probe's `n.k` tests. Four probe defects made it wrong:
1. **Function-pointer arguments were a literal `0` on every line**, so a function with a nonnull callback (pthread_create
   3, on_exit 1, qsort 4) drew -Wnonnull on every `n.k` line: pthread_create got 2,4, on_exit 2. Now `v[j]`.
2. **Numeric arguments were `1`**: a size argument of 1 makes the compiler report -Wnonnull from an `access` attribute
   (write 2, fwrite 1, strxfrm 1, regexec 4, regerror 3, getentropy 1, the `*_r` buffers, snprintf 3). `n.k` now
   passes 0 for every number (probe: write(1,NULL,0) quiet, write(1,NULL,1) warns on the reference).
3. **Compiler builtin knowledge** (printf/scanf formats, fputs/puts, nan*, dcgettext 2, vprintf...) was read as the
   header's: `n.k`/`rn` now compile in their own unit with `-fno-builtin`.
4. **Coverage**: only top-level headers were read (sys/statfs.h, sys/resource.h, sys/epoll.h ... never probed);
   `locale_t` parameters had no `n.k` test (strto*_l, strcoll_l); pointer typedef parameters (`__compar_fn_t`) were
   numbers. Now: recursive header walk (greedy clash filter), `loc` tested, `pointer_typedefs()` probe.
5. **Fortify**: Ubuntu's gcc defines `_FORTIFY_SOURCE=3` at -O2, so the table was the *fortified* set. The reference
   marks MORE parameters with fortify off (atoi 1, feof 1, bsearch 1,2, chdir 1, fgets 3 ...). New column
   `nonnull_nofortify` (marked only when fortify is off) emitted as `__NIJI_NONNULL_NF ((..))`, defined in
   headers/features.h.in from `__NIJI_FORTIFY` (empty when fortifying). Generator: xtask/src/headers.rs attr_text.

Measured (out2, old headers vs new probe): stricter 56 functions, looser 47 (matches Yohane's 56/43 premise).
Config checks with OLD headers (n/rn diffs): gnu 295, default 223, fortify2 170, c++ 279, c++-fortify3 162.
zstd 1.5.7 (`~/work/headers-nonnull-match/zstd-dl`, sha256 eb33e51f...) before: lib/common/pool.c fails
`-Werror=nonnull` (2 errors) on libniji's headers, 0 on the reference (job headers-nonnull-match-zstd-before).

## attr_probe.py additions
`--config {base,gnu,default,fortify2,c++,c++-fortify3}` (non-base runs only n/rn tests; one-sided compile errors are
listed as "not declared alike", not counted), `table --columns nonnull,nonnull_nofortify` (other columns keep their
values), `check --kinds n,rn`, `rn` test (returns_nonnull by marker; reference: only alloca and, under fortify, the
memcpy family, no column). Docstring NOT yet updated for these (do it).

## Exact next step
1. `~/bin/jobwait headers-nonnull-match-tbl3` (keke); copy `~/work/headers-nonnull-match/out3/diag.tsv` to
   spec/abi/diagattrs.tsv; read `out3/table.log` (any "only under _FORTIFY_SOURCE" lines need a design answer).
   Check error/error_at_line got no nonnull_nofortify (functions.rs nonzero_wrapper strips a trailing "))").
2. `cargo xtask headers-gen` (or the generator command named in libc-include headers), commit tsv + regenerated headers.
3. Sync; job: `check --config X --kinds n,rn` for all six configs (and fortify1 if cheap) -> 0 differ each;
   base full check as INFORMATIONAL. zstd.sh (scratch) -> 0 failed on libniji. Script: `$S/job2.sh` pattern.
4. Item 3 unit tests (no existing coverage found): regexec nmatch 0 + pmatch NULL, getentropy(NULL,0), on_exit(fn,NULL),
   snprintf(NULL,0,..), fwrite(NULL,1,0,f), write/pwrite(fd,NULL,0), re_search NULL regs, getgrnam_r buffer NULL len 0
   (s17 has some). Add to crates/boundary/tests/s<NN>_*_symbols.rs. Do NOT crash-probe the reference (fputs/printf/
   nan with NULL are UB; libniji's own behaviour only).
5. Plants (docs/plants.md format, handoff/HEADERS-NONNULL-MATCH-plants.txt): attr_probe zero/fn-arg/-fno-builtin,
   generator nonnull_nofortify emission. clippy, static, affected gate, precheck, report json, handback-check.

## INFORMATIONAL (for the report)
* A full `table` run also changes other columns (pure/const of ~180 math/complex functions, wur on readv/preadv*,
  deprecated on inet6_option_*/ns_*): not this story; hence `--columns`. Follow-up story recommended.
* The other attributes (wur) may also be fortify-dependent on the reference; the table was probed at fortify 3.
* c++: ~55 functions build on the reference only (chmod, chown, frexp, getgroups ...): libniji C++ declarations differ.
