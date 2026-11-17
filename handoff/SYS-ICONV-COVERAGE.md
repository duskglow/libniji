# SYS-ICONV-COVERAGE: why 1122 of 1180 iconv names fail with libniji's packages alone (issue #6), run 1

**State.** Branch `task/sys-iconv-coverage`, code commit `240313ae9` (head: see report json). Verdict
PENDING JOBS: code written and pushed; gate and plants were running at hand-back (context checkpoint).
Decision: `records/pending/SYS-ICONV-COVERAGE-1.toml` (D-NEW, amends D-6.8). Plants:
`handoff/SYS-ICONV-COVERAGE-plants.txt`. Per-name table: `handoff/SYS-ICONV-COVERAGE-names.tsv`.

## Cause (evidence; reference system glibc 2.39, role keke n2)

* D-6.8: every converter other than the built-ins is read from `/usr/share/i18n/charmaps`. No
  libniji package shipped that directory (package-parity audit row 6 confirmed: our `locales` is a
  placeholder; row 24, gconv, is irrelevant: libniji has no modules). Per name, one process each:
  libniji opens **58** names alone, **683** with the host's charmaps visible (all 1180 on the reference).
* A second cause: `CHARMAPS = 32` in `crates/legacy/src/global_locale.rs`: a process loads at most
  32 charmaps; the 33rd distinct charset fails `EINVAL`. In one process libniji opened only 170
  names even with the host charmaps (the earlier inventory's "168" was this limit, not missing data).

## Classes of the 1122 names that failed alone (`names.tsv`, column `class`)

| class | names | fix |
| --- | ---: | --- |
| charmap-generated (this branch: 43 charsets, Unicode mapping tables reproduce the reference exactly) | 281 | done |
| charmap-not-shipped (host charmap converts as the reference; no public table vendored yet) | 382 | next: more sources (IBM EBCDIC pages, Mac, multi-byte from WHATWG) |
| charmap-not-shipped-differs (host charmap differs from the reference) | 59 | next: per-charset variant |
| single-byte-no-charmap-name (an alias or charset no charmap names) | 229 | next: aliases by probe grouping + data |
| multi-byte-no-charmap-name | 121 | next: multi-byte data (WHATWG indexes) |
| stateful-or-no-single-byte (ISO-2022, UTF-7, IBM93x, ...) | 50 | RULING (new converter kind) |

## Design

`cargo xtask so` writes one POSIX charmap per row of `spec/encoding/charmaps.tsv` into
`target/niji/charmaps` (`xtask/src/charmaps.rs`), from a Unicode mapping file vendored unchanged
in `spec/encoding/unicode/` (README: Unicode terms of use). A row exists only where the file's
decoding of every byte equals the reference's (fingerprint probe); aliases are the reference's
`iconv -l` names grouped by that fingerprint. `tools/deb/system.sh` installs them in libc6
(`/usr/share/i18n/charmaps`), `check-system.sh` checks their count and ISO-8859-15. Limit 32 -> 256.

## Tests and jobs (keke)

`crates/core/tests/iconv_charmaps_products.rs` (+ `tests/oracle/iconv_names.c`): every name of the
table in one process and each charmap's whole code space vs the reference, libniji seeing only
the generated set (`unshare -r -m` bind mount). Jobs: `sys-iconv-coverage-gate` (clippy, static,
so, the three iconv product tests, coexist + system package builds, check-system, affected),
`sys-iconv-coverage-plants` (4 plants). Probes: `-inv1`, `-inv2` (evidence only).
NOT VERIFIED at hand-back: the after column (expected 58 -> 339 alone) and the round trips.

## Next step

Read both jobs' logs; fix what is red. Then: measure the after column (`run2.sh` probe with the
generated dir mounted), add the next classes (IBM EBCDIC pages need a public source: RULING),
alias groups for host-only names. Plan rows: D-6.8, D-6.15, D-6.9.
