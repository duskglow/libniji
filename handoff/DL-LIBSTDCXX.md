# DL-LIBSTDCXX handoff (GitHub #4: ld.so refused Debian's libstdc++.so.6), after DL-LIBSTDCXX-3

State: branch `task/dl-libstdcxx-3` (stacks on task/dl-libstdcxx-2, base db204dc19), code head `0a91427c7` (plus this
note and the report json). Role mei:~/w/s1. Report: handoff/reports/DL-LIBSTDCXX-3.json; plants
handoff/DL-LIBSTDCXX-2-plants.txt (10 plants, unchanged this run).

## Cause (settled, DL-LIBSTDCXX)

The start's fixed `UniqueTable::<64, 4096>` overflowed (libstdc++ binds 106 `STB_GNU_UNIQUE` names) and
`LookupError::NoMemory` printed as "malformed symbol tables". Fixed there: the start binds by global scope order
(`symbols::ScopeOrder`, no table).

## Done (DL-LIBSTDCXX-2, in force)

- dlopen's unique table is `reloc::GrowTable` over two `Stack`s in anonymous mappings grown by `mremap` (no cap but
  memory). Out of memory prints `symbols::UNIQUE_NO_MEMORY` ("cannot allocate memory for the unique symbol table").
- Tests crates/ld/tests/unique_table.rs, unique_start.rs; decision records/pending/DL-LIBSTDCXX-2-1.toml (D-NEW, series
  D-27; CI assigns the id).
- tools/deb: root-system.sh links iostream-so and throwmain (exception across libthrowlib.so) against the build
  system's shared libstdc++.so.6; swap-cmds.sh runs `apt-get check`, `apt-cache policy`, `apt-cache policy bash` in the
  trixie roots (Debian's own libstdc++.so.6 14.2).

## DL-LIBSTDCXX-3 (this run)

- Last run's red `dl-libstdcxx-2-deb` (keke, rc 4): `throwmain does not link` (undefined `throwlib_check`). Cause:
  `shared_cxx` put libthrowlib.so BEFORE throwmain.cc, and Ubuntu's g++ links with `--as-needed` by default, which
  drops a shared library no earlier object references. Not the loader: iostream-so (shared libstdc++) passed in that
  same run. Fix: `shared_cxx OUT SRC AFTER LINK...`, AFTER linked after the source (commit 0a91427c7). The swap/apt
  step had never run (the old deb.sh exited at rc 4); the new deb.sh runs both steps and reports both rcs.
- Counts (last run, keke, dl-libstdcxx-2-counts on 2e1854fe6, same Rust code as this head; perf -r 30, dynamic C hello,
  both trees built by plain `cargo xtask so`): instructions:u 8925714 -> 8924431, page-faults 94 -> 92.
  This run re-measures on mei: dl-libstdcxx-3-counts (results below when read).
- `dl-libstdcxx-3-deb` (mei, 0a91427c7): root-system PASS 15/0 (throwmain: "caught: throwlib: value 3 (throwlib)");
  trixie swap same=49 differ=1. The one difference was apt-1 `apt-get check`: stock rc 0, Liella rc 100 with
  "util-linux : PreDepends: libc6 (>= 2.41) but 2.39+liella20261115.1-1 is installed". So apt runs (Debian's
  libstdc++.so.6 loads, the database is read) and reports the version gap the swap's `--force-depends` creates.
  apt-2/apt-3 (`apt-cache policy`) are identical in both roots. Fix (commit 2263a2961): swap-cmds.sh drops, in both
  roots, only unmet-dependency lines on libc6 whose installed version is `+liella`, and counts 100 as 0 when those
  were apt's only complaint. The raw output stays in apt-1.raw. Checked with a fake apt-get on the captured outputs:
  stock and Liella match; another unmet dependency (100), loader text (127) and a crash status (139) still differ.

## Jobs on mei (final head 2263a2961; scripts in ~/work/DL-LIBSTDCXX-3 on mei)

chain.sh runs in order `dl-libstdcxx-3-gate2` (static, so, unique/dlsym tests, affected --base origin/next),
`dl-libstdcxx-3-startup` (`cargo xtask test`, FULL_PROFILE so + hot_text_products, plain so), `dl-libstdcxx-3-counts`;
in parallel `dl-libstdcxx-3-deb2` (build.sh system flavour, root-system.sh, swap-system.sh trixie with apt) and
`dl-libstdcxx-3-plants2` (handoff/DL-LIBSTDCXX-2-plants.txt). lint0 = clippy+static on the inherited head, green.
Superseded: dl-libstdcxx-3-deb (rc 5: apt-1 before the fix), dl-libstdcxx-3-gate and -plants (rc 124: stopped by me
to rerun on the new head).

Results, all rc 0 on 2263a2961:
- gate2: 8 suites, 591 passed / 0 failed, static ok, peak 2.1G.
- plants2: 10 plants, 10 CAUGHT, 0 SURVIVED, no WARN.
- deb2: root-system 15 pass / 0 fail (iostream-so, throwmain against the shared libstdc++.so.6); trixie swap same=50
  differ=0 (apt-1/2/3 identical; the Liella root's apt-1.raw differs only by the util-linux libc6 version line).
- startup: `cargo xtask test` (6656 + 5240 tests passed), FULL_PROFILE so + hot_text_products 26 passed, all 12 layout
  contracts hold; peak 16G.
- counts (perf -r 30, dynamic C hello, base db204dc19 vs branch, both built by plain `cargo xtask so`):
  instructions:u 8925508 -> 8924220, page-faults 94 -> 92 (keke last run: 8925714 -> 8924431, 94 -> 92).

## Open / next

The original brief's proofs are all done (unit tests, Debian libstdc++ in a Liella root, apt in the trixie swap,
plants, counts). Next: land. Optional follow-up: a test that drives dlsym's out-of-memory text end to end (needs a
mapping failure on demand). Do not redo: the cause hunt and the unique-table design (DL-LIBSTDCXX, -2).
