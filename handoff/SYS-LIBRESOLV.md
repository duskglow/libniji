# SYS-LIBRESOLV (issue #5): libresolv.so.2 in the system packages

State: branch `task/sys-libresolv`, WIP, stopped at the ~150k context checkpoint (first run, keke role n2).
Nothing below has been run yet. The products build `sys-libresolv-so1` was still running at hand-back.

## What was found (do not redo)

* The interface already exists. `spec/abi/versions.tsv` has the 69 `libresolv.so.2` rows (reference symbol table,
  GLIBC_2.2.5 / 2.3.2 / 2.9 / PRIVATE, 11 compat). `cargo xtask so` already builds `target/niji/libresolv.so.2`
  (D-16.6, SL-SONAMES: each export jumps to libc.so.6's `NIJI_PRIVATE` alias; `xtask/src/placeholders.rs`
  FORWARDED/DEV_LINKS), plus `libresolv.so` and an empty `libresolv.a`. The `ns_*` parsers are
  `crates/core/src/names/dns/{msg,name}.rs`, C layer `crates/boundary/src/resolv.rs` (initparse/skiprr/parserr ~l.660).
  `crates/core/tests/sonames_products.rs` already proves the forwarding (dynamic and static `-lresolv`).
* The coexist flavour (`tools/deb/build.sh`) already ships libresolv.so.2, libresolv.so and libBrokenLocale.
  The gap was only the system flavour (`tools/deb/system.sh`), which skipped them.
* `fuzz/fuzz_targets/dns.rs` already fuzzes `msg::parse`, `msg::rr`, `msg::skiprr`, `name::unpack/skip`
  (the Rust parsers under ns_initparse/ns_parserr). The fuzz crate does not depend on niji-boundary, so the C
  layer (`resolv::initparse/parserr`) is not fuzzed; the new differential probe covers its malformed paths.
* `check-symbols.sh` and `symbols.py` find libraries by listing libc6's library directory, so shipping
  libresolv.so.2 adds its block to `symbols`/`shlibs` with no generator change (to be confirmed by the run).

## Commits

* `WIP: Compare the ns_* message parsers ...`: `crates/core/tests/oracle/ns_parse_diff.c` (corpus of 40 good and
  malformed messages), `crates/core/tests/ns_parse_products.rs` (reference `-lresolv` vs libniji dynamic through
  libresolv.so.2 vs static libc.a; clause s16_4_c_3).
* `WIP: Ship libresolv.so.2 and libresolv.so in the system packages`: system.sh, check-system.sh (link, SONAME,
  dynamic and static `-lresolv` link of `smoke/resolv.c`), check-symbols.sh (sample `sym-resolv`, libc6 (>= 2.9)).

## Exact next steps

1. `jobwait sys-libresolv-so1` on keke; then run `cargo test -p niji --test ns_parse_products` (jobrun). Expected
   smoke line: `resolv: a.test A 192.0.2.1 ttl 3600; cut header -1 EMSGSIZE` (check it on the reference too).
   Any difference: fix in `crates/boundary/src/resolv.rs` / `names/dns/*.rs` (spec: RFC 1035, resolver(3)); a
   reference crash on some corpus entry is SECURITY-PRIVATE (drop the entry, hand-back only).
2. `tools/deb/build.sh --flavour system`, then check-system.sh and check-symbols.sh (expect the new block and
   sample to pass; symbols.py may need a look at libresolv's compat exports `_gethtbyname@GLIBC_2.2.5` etc.).
3. root-system.sh: add the krb5 proof (copy the build system's libgssapi_krb5.so.2, libkrb5.so.3, libk5crypto.so.3,
   libkrb5support.so.0, libcom_err.so.2, libkeyutils.so.1 into the root as the libstdc++ case does; link a program
   with them and run it in the root; run `/tmp/resolv` dynamically there). keke is Ubuntu 24.04, whose libkrb5.so.3
   NEEDs libresolv.so.2 and uses ns_initparse/ns_parserr@GLIBC_2.9 plus res_nsearch/ns_name_uncompress@GLIBC_2.34.
4. swap-system.sh trixie (expect 52/52) and lint.sh (no worse); plants file through `cargo xtask plants` (e.g.
   drop the bounds check in `name::unpack`, ENODEV->EMSGSIZE in parserr, `libresolv.so.2` removed from system.sh
   list with cmd check-system); clippy + static; handoff/report; handback-check.

## Open questions

* RULING NEEDED: the brief asks for "a real libresolv.a". D-16.6 keeps it empty because libc.a defines every
  libresolv function; a non-empty archive would duplicate definitions in a static link. Recommend keep D-16.6
  (the static `-lresolv` link in check-system.sh proves the functions resolve).
* INFORMATIONAL: the system flavour also lacks libBrokenLocale.so.1/.so/.a and libg.a, libmcheck.a,
  libpthread_nonshared.a, which the coexist flavour ships. Not in this story's scope.
