# HEADERS-TABLES-COMPLETE: handoff (leg 2, 2026-11-19)

State: branch `task/headers-tables-complete` from origin/next 2b5604f9c. Role mei:~/w/s2. Code commits: 570b86928
(leg 1: groups 1, 3, 6, 7), 67c69b1a0 (plants file), 2ca7392b5 (leg-1 findings fixed), 7f0a5c5ea (group 2). Plants
file `handoff/HEADERS-TABLES-COMPLETE-plants.txt` (10 plants). Exceptions in `headers/rows-exceptions.tsv`:
99 -> 74 (leg 1) -> 42 (leg 2).

## Done

* Leg 1 (see 570b86928's message): enumerators.tsv + `@@enum` as data, layouts at every depth, bit positions
  `bits W@P` recorded and checked, va_list C++ row PERMANENT (#6721).
* 2ca7392b5: leg 1's test job found `header_p2_fixed_rows` red (union sigval defined in strict sys/pidfd.h, a
  regression from the complete siginfo_t). Fix: `typedef()`'s last branch spells a typedef named for linkage
  through a tag the header shows in fewer configurations through an untagged aggregate outside the tag's
  condition (`narrower_tag` now takes both kinds). Plants survivors fixed: htc-anon-level now runs the new unit test
  `an_anonymous_member_counts_only_its_own_level` (the tables' only `<anon>` with deeper rows is Dl_serinfo's last
  member, so the rows test cannot see a miscount); the per-enumerator guards in `enum_body` were dead (an enumerator
  of a declared enumeration is always routed to the whole enumeration) and are removed (mcheck.h, sys/ptrace.h
  lose those lines); plant htc-enum-route replaces htc-enum-guard. New plant htc-typedef-narrow-tag.
* 7f0a5c5ea, group 2: `headers/providers.tsv` (name, provider, source): la_* `program` (rtld-audit(7)), ps_*
  `debugger` (proc_service), bindresvport6 and getumask `none` (ruling #6715: declared, a use fails to link as on
  the reference). The export check in functions.rs skips these names; `load_providers` refuses an unknown
  provider, an undeclared name or a default-version export (unit test
  `a_provider_row_is_refused_unless_it_is_needed`). 32 exception rows removed. DEVIATION from the brief's wording
  (INFORMATIONAL for the hand-back): a reviewed generator input instead of a prototypes.tsv column, because
  abi_extract.py rewrites prototypes.tsv with three columns and the ps_* functions have no prototypes.tsv row.
  S16 has not landed (no headers/slices/s16*), so nothing of #6725 applies.
* Proof on 2ca7392b5/7f0a5c5ea (job headers-tables-complete-l2gen3, mei): headers-gen, `headers-gen --rows`
  ok (prototypes 0 excepted, typedefs 8 excepted, all 0 failed), both unit tests, header_p2_fixed_rows ok.
  libc-include changes this leg: mcheck.h, sys/ptrace.h (dead guard lines removed), sys/pidfd.h (union sigval
  under `__NIJI_POSIX >= 199309`, untagged `__sigval_t` union otherwise).

## Jobs running at the refresh (head 7f0a5c5ea): collect FIRST

`~/bin/jobwait headers-tables-complete-l2test1 headers-tables-complete-l2plants1` on mei.
* l2test1: clippy -D warnings, `cargo xtask static`, `cargo test -p xtask -p niji-abi`.
* l2plants1: the 10 plants, `--dir /mnt/zram/claude/headers-tables-complete/plants-2`; grep CAUGHT/SURVIVED/WARN.
Leg 1's jobs headers-tables-complete-test1 and -plants1 are SUPERSEDED (test1 red on p2, fixed in 2ca7392b5;
plants1 had 2 survivors, fixed). Job labels used so far: -gen2 (leg 1), -l2gen1..3, -l2unit1, -l2extract1: pick
new labels (jobrun silently refuses a used one).

## Next: group 4 (8 C++ exceptions), then group 5

Group 4 finding (leg 2, probe on mei): GCC's DWARF gives the `__SOCKADDR_ARG` / `__CONST_SOCKADDR_ARG`
transparent union NO member DIEs (the union DIE has no children), so `abi_extract.py --layouts-only` yields only
the head rows. The transparency itself is probe-able (GCC manual, transparent_union: a `void *` argument is
accepted only by a transparent union parameter); patch with `transparent()` and the two TYPES entries saved at
`~/w/.scratch/HEADERS-TABLES-COMPLETE/group4-extract.patch` (not committed; head rows only, members missing).
Next step: a member probe: for each `struct sockaddr*` tag visible under `_GNU_SOURCE` (typedefs.tsv/visibility),
compile `void f (__SOCKADDR_ARG); void g (struct sockaddr_X *p) { f (p); }` with `-Werror` (and `const` for the
CONST form); the accepted pointer types are the members (names are the implementation's: reserved). Record them
as layouts rows (head row last column `transparent_union`). Generator: in C a `typedef union {..} NAME
__attribute__ ((__transparent_union__))`; in C++ and where visibility.tsv has the name as a `macro` (`!gnu`), the
pointer form `struct sockaddr *__restrict` / `const struct sockaddr *` (POSIX accept/bind prototypes; the C++
form is checked against typedefs.tsv `function accept` = `int (int, sockaddr*, unsigned int*)`). tname spells a
restrict pointer `restrict struct sockaddr *`: the generator must place the qualifier after the `*`.
Group 5 as in the brief (18 pointer constants, FNM_NOSYS, DLFO_EH_SEGMENT_TYPE, TD_SIGNO_MASK, 10 type-generic
macros, gets and bsd_signal). type_probe.py's clean-room rule: never read `-E`/`-dM` output.

## Open / informational

* Duplicate guarded `union sigval` blocks in sys/pidfd.h: the nested-definition duplicate (#6723, its own story).
* Process slips: leg 1 one `sed -i` and two empty heredocs; leg 2 two heredocs (a scratch script, a no-op).
