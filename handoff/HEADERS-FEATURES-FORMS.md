# HEADERS-FEATURES-FORMS (handoff, end of leg 3 of run 1)

**State:** branch `task/headers-features-forms` (from origin/next db602e58c), role keke:~/w/r2. Head: the commit
after this note. Generated headers regenerated and committed. Report json: `handoff/reports/HEADERS-FEATURES-FORMS.json`;
plants record: `handoff/HEADERS-FEATURES-FORMS-plants.txt`.

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

## Exact next step
Land. If the gate or plants named in the hand-back are red: fix what they name, run only that suite.

Jobs (keke, labels prefixed headers-features-forms-): final head: gate3, plants3, pkg6 (proofs), regen3 (table,
headers-gen, stubs, clippy). Superseded: pkg1..5 (script iterations / earlier heads), pkgctl (next), pkgctl252 (25.2),
fio and kspell (probes), lint1, gate (875 FAILs), gate2 (FIOCLEX), plants1 (--only bug), plants2 (stopped, stale head),
regen, regen2 (ran before a sync).
