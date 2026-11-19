# HEADERS-TABLES-COMPLETE: handoff (leg 3, 2026-11-19)

State: branch `task/headers-tables-complete` from origin/next 2b5604f9c. Role mei:~/w/s2. Code commits: 570b86928
(leg 1: groups 1, 3, 6, 7), 67c69b1a0 (plants file), 2ca7392b5 (leg-1 findings), 7f0a5c5ea (group 2), 2894a9fb4
(group 4), 955a77192 (group 5: pointer and X/Open macros). Plants file `handoff/HEADERS-TABLES-COMPLETE-plants.txt`
(15 plants). Exceptions in `headers/rows-exceptions.tsv`: 99 -> 74 (leg 1) -> 42 (leg 2) -> 15 (leg 3).

## Done (do not redo)

* Legs 1-2: see commit messages 570b86928, 2ca7392b5, 7f0a5c5ea (enumerators, depth, bit positions, va_list
  PERMANENT #6721, providers.tsv with ruling #6715; DEVIATION: providers.tsv instead of a prototypes.tsv column,
  INFORMATIONAL for the hand-back).
* Leg 2's jobs collected: l2test1 green (738 passed), l2plants1 10/10 CAUGHT.
* 2894a9fb4, group 4: abi_extract.py probes transparent unions (void * accepted), their members over candidate
  sockaddr tags (tables + man pages ddp(7), netlink(7), packet(7), vsock(7), x25(7)), names `__tu_<tag>`
  (unobservable: INFORMATIONAL), restrict of the strict-C macro form by `_Generic`. Members found: sockaddr, _in,
  _in6, _un, _x25 (a member tag outside the candidates would be missed: INFORMATIONAL). Generator: union with
  `__transparent_union__`; where visibility has the name as a macro, `#else #define NAME <first member ptr>
  [__restrict]`; rows unit (no header rows) uses the union in C, pointer in C++. Rows test: members + void* call
  (C), exact pointer type with `__is_same` (C++). abi-gen regenerated crates/abi (layout.rs, appendix_c_layouts.rs).
  No libc-include change (sys/socket.h is not generated yet). Proof on 2894a9fb4: l3test1 green (clippy, static,
  740 tests), l3plants1 13/13 CAUGHT.
* 955a77192, group 5 part: type_probe.py `--retry-missing --names`, config fallback (first C config showing a
  name), pointer pass (C++ decltype spelling + constants.tsv value). 19 macros.tsv rows; value emitter casts
  pointer rows (`c_words`); fmtmsg.h.in uses @@value for MM_NULL{LBL,TXT,ACT,TAG} (libc-include/fmtmsg.h: `0` ->
  `0L` in four casts); rows test checks pointer type C/C++ and address at run time. l3gen6: rows ok.
  Probe: TD_SIGNO_MASK is object-like, `void *` in C, value 0 (not function-like).

## Jobs running at the refresh (head 955a77192): collect FIRST

`~/bin/jobwait headers-tables-complete-l3test2 headers-tables-complete-l3plants2` on mei (clippy -D warnings,
static, `cargo test -p xtask -p niji-abi`; 15 plants, dir plants-4). SUPERSEDED: l3test1/l3plants1 (green, older
head 2894a9fb4). Labels used: -l3extract1, -l3gen1..6, -l3tp1, -l3test1/2, -l3plants1/2: pick new ones.

## Next (15 exception rows left)

1. Type-generic macros (10: iscanonical iseqsig issignaling iszero fpclassify isfinite isgreater isnormal
   isunordered signbit): function rows (or a visibility kind) for their function forms (ruling #6720).
2. gets (c89/c99/isoc99/cxx98/cxx11) and bsd_signal (xopen500/600/ext): prototypes rows from a configuration that
   declares them (abi_extract.py prototypes() only probes its own configuration).
3. NULL (C++ type is not a pointer; macros.tsv has one C type column) and TD_SIGNO_MASK (C++ decltype line dropped
   in type_probe's C++ unit; find why): reasons updated in the exceptions file; RULING NEEDED candidate for NULL
   (recommend: keep NULL the templates', mark PERMANENT).
4. Then final: sync, clippy foreground, static, test + plants jobs, precheck, report JSON, handback-check.
   gen script: `~/work/headers-tables-complete/gen.sh <n>` (abi-gen, headers-gen, --rows; tars outputs).

## Open / informational

* Duplicate guarded `union sigval` blocks in sys/pidfd.h: #6723, its own story.
* Process slips: leg 1 one `sed -i`, two empty heredocs; leg 2 two heredocs; leg 3 one no-op `sed -i` (a
  substitution of a match by itself on typedefs.rs; file unchanged), reported.
