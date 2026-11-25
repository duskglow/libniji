# HEADERS-FEATURES-FORMS (handoff, end of run 3, HEADERS-FEATURES-FORMS-3)

**State:** branch `task/headers-features-forms-3` (run 2's branch plus run 3's merge of origin/next 8828f0ce8), role
keke:~/w/r3. Report json: `handoff/reports/HEADERS-FEATURES-FORMS-3.json` (runs 1, 2: `...-FORMS.json`, `...-FORMS-2.json`);
plants records: `handoff/HEADERS-FEATURES-FORMS-plants.txt` (run 1, 7/7 CAUGHT), `...-2-plants.txt` (1/1),
`...-3-plants.txt` (run 3: those 8 re-run on the merged head, plus the utmp.h.in macro: 9/9 CAUGHT).

## Run 3 (this run): the CI conflict
* Merged origin/next by hand (the helper script works on the coordinator's checkout). Conflicts:
  `headers/utmp.h.in` (this branch added TIMESPEC_TO_TIMEVAL/TIMEVAL_TO_TIMESPEC to <utmp.h>; next added the struct
  utmp member-name macros ut_name/ut_time/ut_xtime/ut_addr): both kept, one definition each, `@@owns` is the union;
  `spec/tools/header_probe.py` (both candidate lists kept). Generated `complex.h math.h utmp.h wchar.h` taken from
  next then regenerated with `cargo xtask headers-gen`; `net/if.h` merged cleanly but regenerates differently
  (both sides changed generator inputs), regenerated copy committed.
* Plant on the merged utmp.h.in macro: the util-linux package pattern does not include <utmp.h> (SURVIVED); the
  per-header comparison `FEATURES forms utmp.h` catches it.
* Gate red twice on `xtask::transcripts the_committed_gencat_transcripts_hold`: not a header failure. The role's
  `target/niji-test/gencat` was a stale test link whose interpreter (`target/niji/ld-linux-x86-64.so.2`) no longer
  existed (the test only checks the link is a file). `cargo xtask so` on the role then `cargo test -p xtask --test
  transcripts` passed (job -transcripts); the gate was re-run (-gate3).
* Plants job 1 (-plants) failed its baseline-7 by the 600 s timeout because the gate ran beside it on the same role
  (my mistake: one build/load per role at a time); -plants2 (timeout 1500, alone) is the run.

## Run 2 (this run)
* Gate3's red (p2_visibility_of_the_fixed_rows: "extra sys/pidfd.h F_OWNER_GID enumerator") was a gap in the
  reference table, not in libniji's headers: a black-box probe (`header_probe.py --names` over every public header on
  the reference) shows F_OWNER_GID as an enumerator in the same six headers and configurations as F_OWNER_PGRP
  (fcntl.h, sys/fcntl.h, sys/file.h, mqueue.h, sys/mount.h, sys/pidfd.h; gnu and every C++ mode). Run 1 spliced rows
  only for F_OWNER_TID/PID/PGRP. 6 rows spliced into visibility.tsv; headers unchanged.
* Provenance fragment `spec/abi/provenance.d/HEADERS-FEATURES-FORMS-2.txt` added for the branch's table changes (run 1
  added none).
* Plant hff2-gid-row (row deleted) CAUGHT by p2_visibility_of_the_fixed_rows.

## Done
* Leg 1: probes and tables (`spec/abi/use-macros.tsv`, `spec/abi/macro-forms.tsv`, 47 visibility rows), features.h.in
  `__USE_*` set (the whole probed family: owner amended R-H2 on 11-25, now final), generator rule `redefined()`
  (outright unless macro-forms.tsv keeps it), cond.rs misc/gnu as `defined __USE_MISC` / `defined __USE_GNU`, SUN_LEN
  and TIMESPEC_* in sys/un.h.in / sys/time.h.in.
* Leg 2: SOL_* values, TIMESPEC_* in the other templates that show them, `@@intc` (INTn_C outright),
  `header_checks/features.py` and `feature_probe.forms_batch`. Re-probe: `__USE_*` 3216/3216 cells match.
* Leg 3:
  - Package proofs (compile-only, `-fsyntax-only`, gcc-14's default errors turned on; config headers from configuring
    each package with the host compiler; sources from the local Debian mirror only): gnupg2's npth `<npth.h>`
    rwlock use (npth 1.8's configured header; full certcache.c needs libgpg-error/ksba/assuan headers, not on the
    host), flex 2.6.4 `scan.c` and gcc-14's `gengtype-lex.cc` generated text up to the end of the skeleton's limits
    block + `<stdint.h>` + hwint.h's `#if SIZE_MAX <= UINT_MAX` (C++11 and gnu89), python3.13 `Modules/fcntlmodule.c`,
    util-linux `lib/monotonic.c`, openssl 3.5.9 `bss_sock.c`, `bss_conn.c`, `bio_sock2.c` (also with Yohane's own
    flags), `ktls_meth.c`, less 668 `pattern.c` (`--with-regex=gnu`). 14 compiles each side:
    branch head 14/14 pass, reference 14/14; control on snapshot 25.2's headers 0/14 (the errors of Yohane's logs, at
    the same line:column); control on origin/next db602e58c 5/14 (openssl already passes there: fixed since 25.2).
  - Gate (job -gate) found 875 header_check.py FAILs (P5 and kernel-first orders, `-Wsystem-headers -Werror`):
    FIOASYNC, TIOCSER_TEMT, IPPROTO_DSTOPTS made outright while a kernel UAPI header defines them first in another
    spelling. A three-name exception (e7463ca98) was not the set (-gate2: FIOCLEX too; FAIL lines show only a unit's
    first errors). Fix: `spec/tools/kernel_spelled.py` writes `spec/abi/kernel-spelled.tsv` (1414 names: every libniji
    macro a staged UAPI header defines with other replacement text); the generator keeps those under `#ifndef` (as on
    next, where header_check passes; with the kernel header in front its definition is the one in force, as in the
    reference); features.py leaves them out of the forms comparison. 57 headers regenerated.
  - features.py `--only` with one header's forms title selected nothing (fixed).

## Decisions
* D-NEW (pending, no id reserved): libniji defines a macro outright exactly where the reference's definition replaces
  a program's earlier one (macro-forms.tsv probe), except names in kernel-spelled.tsv, which stay guarded.
* R-H2: owner amended 11-25, final: the whole `__USE_*` family with the reference's meaning (done).

## Open
* kernel-spelled.tsv is conservative (name level): for a name a libniji header defines without including the kernel
  header, a program's own earlier definition is kept where the reference replaces it (next's behaviour, unchanged).
  Narrowing it to the names that really meet a kernel definition in one translation unit is a follow-up.

## Follow-up (INFORMATIONAL; measured, not fixed here)
* Guard mismatches in hand-written template text (features.py KNOWN): the 14 feature-test macros features.h sets under
  _GNU_SOURCE, `__WORDSIZE_TIME64_COMPAT32`, `_SIGSET_NWORDS`, NULL (~18 templates `# ifndef NULL`), `__is*_l`,
  IP(V6)_RECVORIGDSTADDR / IP_RECVRETOPTS, TEMP_FAILURE_RETRY, FD_SET, MM_NULL*, XATTR_CREATE, si_*; KNOWN_KEPT
  include-structure cases (sys/mount.h, net/if_ppp.h, ...).
* Pre-existing on origin/next: `<sys/time.h>` then `<sys/types.h>` under `-Wsystem-headers -Werror`: "FD_SET redefined".
* No `#warning` for `_BSD_SOURCE` / `_SVID_SOURCE` (the reference warns: deprecated).
* `struct f_owner_ex.type` is `unsigned int` (layouts: `enum __pid_type`), pre-existing generator behaviour.
* Numeric `__USE_*` levels a program defines itself are not honoured (only MISC/GNU).

* Other enumerators.tsv names may lack visibility.tsv rows the same way (only the six P2 headers are compared in the
  direct tier); a full `header_probe.py` re-run would settle it (INFORMATIONAL).

## Exact next step
Land. If the gate or plants named in the hand-back are red: fix what they name, run only that suite.

Run 2 jobs (keke, labels prefixed headers-features-forms-2-): gate (direct/bounded xtask set), plants, lint0 (clippy +
static on the inherited head), gidprobe (reference probe of F_OWNER_GID).
Run 1 jobs (keke, labels prefixed headers-features-forms-): gate3 (red: F_OWNER_GID row, fixed in run 2), plants3,
pkg6 (proofs), regen3 (table, headers-gen, stubs, clippy). Superseded: pkg1..5 (script iterations / earlier heads), pkgctl (next), pkgctl252 (25.2),
fio and kspell (probes), lint1, gate (875 FAILs), gate2 (FIOCLEX), plants1 (--only bug), plants2 (stopped, stale head),
regen, regen2 (ran before a sync).
