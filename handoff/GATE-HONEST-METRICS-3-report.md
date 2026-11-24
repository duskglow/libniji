# GATE-HONEST-METRICS-3 report (third leg of GATE-HONEST-METRICS)

Branch `task/gate-honest-metrics-3` (continues `task/gate-honest-metrics-2`, no merge), role mei:~/w/s3
(Skylake-SP host). One-page state: `handoff/GATE-HONEST-METRICS.md`. Earlier legs:
`handoff/GATE-HONEST-METRICS-2-report.md`, `handoff/reports/GATE-HONEST-METRICS.json`.

## Train 970 red: the cause

Train 970 (head b3f3fd771) failed `init_data_clusters_hold_at_every_text_phase` with one line:
`locale (per-locale directory): <Chain<Filter<FlatMap<.., Split<u8, find_named<Files<Glue>>::{closure}..>>>, Once<..>>
as Iterator>::try_fold<.., find_map::check<..>>` at 0x5772c0, outside the hot group 0x410000..0x4a0005.

The cause is none of the three the brief named (a closure the trace missed, stale names, the interaction
as such). It is in the build's list step:
- regen.sh read libc.a's function-section names with `readelf -SW`. That tool cuts a name at 256 bytes
  (probe on the Skylake-SP host, binutils 2.42: a 406-byte section name prints as 256 bytes; `objdump -h` and
  `objdump -t` print all 406). The failing function's mangled name is 538 bytes, so its section is longer
  still.
- `lib.keys` (the names that may be listed) therefore held a cut name. The traced key matched nothing, and
  the function was dropped from `hot-text.list` without a message, whatever the per-directory trace saw.
- On this branch's own base the instance is either not a separate function or never runs, so the branch's
  gates passed. In the merged train head it was a separate 538-byte function. The per-directory trace
  this story added was the first check that ran it.
- xtask's own grouping already reads whole names from the ELF headers (`archive_headers`; LAYOUT-STARTUP-
  RODATA-3, #4354 hit the same cut). regen.sh's list step was the one reader left behind.

Not done: building the merged head (the brief allows no merge). The mechanism holds for any traced function
with a name over 250 bytes. On this branch's build it moved 4 functions into the group (below).

## Fix (commits 0610a173c, 9e34353e7, b7b37e33f)

- `keys.py --sections <archive>` reads the section names itself with `objdump -h` (whole names). If objdump
  fails (e.g. a wrong path) it exits non-zero; it never lists nothing. regen.sh passes it `libc.a`.
- xtask test `a_cold_renamed_function_reenters_the_list_by_trace` compiles a real object and archive. It has
  every section shape `function_of` strips plus a 300-byte name. It checks keys.py against `function_of`
  (sorted), checks that the long name comes back whole, and checks that a C source given as the archive fails.
  The tool choice is now under test, not a hand-written readelf listing.

The end-to-end plant (names cut again, judged by the start-up text test) SURVIVED on 9e34353e7 (plants1). On
this tree, none of the functions the cut drops runs in the modes that test traces. The build still lost them
without a word, and the same silent drop let train 970's function out of the group. Commit b7b37e33f:
`keys.py --unlisted` reads the archive's symbol table (`objdump -t`, whole names) and prints each traced key
that names a symbol in a function section missing from the section listing. regen.sh fails on any such key.
An xtask test covers the mode (a missed section is named, also when traced at a label inside it; a listed
section, its label and the program's own function are not). The real build passes the guard (gate4).

Effect on this branch's fast build (mei, gate3 products), measured with `objdump -h` of the grouped
`libc.a`:
- 191 function sections in the archive have names over 256 bytes.
- 5 grouped sections have such names. 4 of them were over 256 bytes before grouping, so the old list
  step dropped them: `nss::passwd_cached<..>`, `nscache::line_system<..>` (two instances) and
  `nscache::line<..>` (tiers tool / extra).
- `hot-text.list` holds 5 keys over 250 bytes.
- Fault counts were not compared against a base build; there are no primed products of the merge base. The
  gate's fault bounds hold.

## INFORMATIONAL

- v0 key walker (keys.py `decimal` and xtask/src/hottext.rs `V0::decimal`): `<decimal-number>` reads every
  digit, but v0 allows only `0` or a number without a leading zero. Nested closures' empty identifiers (`000`)
  therefore parse as one number, the name fails to parse, and the key falls back to the plain strip, whose
  back-references are byte offsets. Both sides share the parser, and no list crosses a build (LT-5), so
  grouping is not affected today. Recommend a small follow-up: read `0` as one digit in both, with a test.
- Other `readelf -SW` section-name readers: `xtask/src/so.rs` `exclude_link_edges` (matches names
  containing the link-edge marker; a name over 256 bytes would be missed), `tools/hottext/faultmap.py` and
  `startup_data.py` (output sections of a program; short names). None was changed here.
