# DL-LIBSTDCXX handoff (GitHub #4: ld.so refused Debian's libstdc++.so.6), after DL-LIBSTDCXX-2

State: branch `task/dl-libstdcxx-2`, code head `2e1854fe6` (plus this note and the report json). Role keke:~/w/n2.
Report: handoff/reports/DL-LIBSTDCXX-2.json; plants handoff/DL-LIBSTDCXX-2-plants.txt (supersedes DL-LIBSTDCXX-plants.txt).

## Cause (settled, DL-LIBSTDCXX)

The start's fixed `UniqueTable::<64, 4096>` overflowed (libstdc++ binds 106 `STB_GNU_UNIQUE` names) and
`LookupError::NoMemory` printed as "malformed symbol tables". Fixed there: the start binds by global scope order
(`symbols::ScopeOrder`, no table).

## DL-LIBSTDCXX-2 (this run)

- Last run's reds: plant `dlopen-start-fallback-off` SURVIVED because check 10 (`dlsym(RTLD_DEFAULT, "uw")`) recorded the
  start's definition before check 11 asked the late handle; the test now asks the handle first and checks the value.
  `dl-libstdcxx-counts` (products path `base/src/target/liella`, not `base/target`) and `dl-libstdcxx-deb` (build.sh
  outside a git tree needs LIELLA_DEB_COMMIT and SOURCE_DATE_EPOCH) were script faults; both are fixed in
  ~/work/DL-LIBSTDCXX-2/{counts,deb}.sh on keke.
- #5983: dlopen's table is `reloc::GrowTable` over two `Stack`s; the loader's `MapStack` is an anonymous mapping grown by
  `mremap` (no cap but memory; all-zero = empty, so the zeroed `Loader` needs no set-up). A failed record and a rollback
  hand back key bytes.
- Error path: `RelocError::Lookup(NoMemory)` and dlsym's NoMemory print `symbols::UNIQUE_NO_MEMORY` ("cannot allocate
  memory for the unique symbol table"); other lookup errors still say "malformed symbol tables".
- Tests: crates/ld/tests/unique_table.rs (new: growth, rollback, out-of-memory, the text); unique_start.rs adds libud/libue
  (1500 new unique names, ~100 KiB of keys, the second binds the first's: table contents survive `mremap` moves).
- Decision: records/pending/DL-LIBSTDCXX-2-1.toml (D-NEW, series D-27); CI assigns the id.

## Jobs on keke (final head 2e1854fe6; ~/work/DL-LIBSTDCXX-2/chain.sh runs the first four in order)

`dl-libstdcxx-2-gate` (static, so, unique/dlsym tests, affected --base origin/next), `dl-libstdcxx-2-startup` (`cargo xtask
test`, FULL_PROFILE so + hot_text_products, plain so), `dl-libstdcxx-2-counts` (base db204dc19 built in
/dev/shm/claude/DL-LIBSTDCXX-2/base; dynamic C hello, perf instructions:u and page-faults, 30 runs, before/after),
`dl-libstdcxx-2-deb` (build.sh system flavour, root-system.sh with the dynamic C++ programs, swap-system.sh trixie with
apt-get check / apt-cache policy; prints both roots' apt outputs), `dl-libstdcxx-2-plants` (10 plants).
Superseded: dl-libstdcxx-2-lint0 (inherited head, green), check1/check2 (clippy, trace), check3 (green dev check).

## Next step

Read the jobs above (`~/bin/jobwait <label>` on keke). If `deb` shows apt-1 differing between stock and liella roots only
because the swap uses `--force-depends` (unmet newer-libc6 dependencies), compare only that apt ran (rc 0 or 100 with
apt's own text, never 127 or loader text) in tools/deb/swap-cmds.sh and say why there. Record the counts in the json.
Not done: a test that drives dlsym's out-of-memory text end to end (needs a mapping failure on demand).
