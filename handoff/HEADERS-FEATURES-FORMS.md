# HEADERS-FEATURES-FORMS (handoff, end of leg 2 of run 1; WIP)

**State:** branch `task/headers-features-forms` (from origin/next db602e58c), role keke:~/w/r2. Head: the commit
after this note. Generated headers are regenerated and committed (headers-gen clean, 0 problems).

## Done (do not redo)
* Leg 1: probes and tables (`spec/abi/use-macros.tsv`, `spec/abi/macro-forms.tsv`, 47 visibility rows), features.h.in
  `__USE_*` set, generator rule `redefined()` (outright unless macro-forms.tsv keeps it), cond.rs misc/gnu as
  `defined __USE_MISC` / `defined __USE_GNU`, SUN_LEN and TIMESPEC_* in sys/un.h.in / sys/time.h.in.
* Leg 2: SOL_UDP/IPX/AX25/ATALK/NETROM/ROSE values (type_probe --retry-missing on the reference); TIMESPEC_* text in
  sys/procfs, sys/profil, sys/timex, utmp, utmpx templates; F_OWNER_* were already in enumerators.tsv; headers-gen
  (commit c7bd0d167); `@@intc` follows the same rule (INTn_C outright, s05.rs); `header_checks/features.py`
  (units: `__USE_*` per ftm config, version macros, package patterns; problems: EXTRA configs, flex SIZE_MAX C/C++,
  batched forms vs macro-forms.tsv) and `feature_probe.forms_batch`.
* libniji re-probe (job probe4): `__USE_*` 3216/3216 cells match. Forms (single-pair, 40326 pairs, before INTn_C):
  35360 replaced/replaced, 260 kept/kept, 952 replaced-form both; 3754 differ, of which ~1500 are `#if`-usability only
  (replaced-form vs replaced: form.py's area) and the guard mismatches are the names in features.py KNOWN/KNOWN_KEPT.
* features.py on keke (job check9): `--reference` 0 failed (all 37221 batched pairs agree with the one-pair table:
  validates the batching and the expectations); libniji 0 failed.

## Follow-up (report as INFORMATIONAL / next story; measured, not fixed here)
* Guard mismatches in hand-written template text (KNOWN in features.py): the 14 feature-test macros features.h sets
  under _GNU_SOURCE (reference replaces a program's own `_POSIX_C_SOURCE` etc. in all 202 headers),
  `__WORDSIZE_TIME64_COMPAT32` (203 headers), `_SIGSET_NWORDS` (77), NULL (28; ~18 templates `# ifndef NULL`), the
  `__is*_l` ctype macros, IP(V6)_RECVORIGDSTADDR / IP_RECVRETOPTS (@@alias), TEMP_FAILURE_RETRY, FD_SET, MM_NULL*,
  XATTR_CREATE, si_* (sys/pidfd.h); include-structure cases (KNOWN_KEPT: sys/mount.h O_*/F_*, net/if_ppp.h, ...).
* Pre-existing (on origin/next): `<sys/time.h>` then `<sys/types.h>` under `-Wsystem-headers -Werror` fails
  ("FD_SET redefined": sys/types.h.in defines FD_SET outright with __fdelt_chk, sys/time.h/utmp.h/sys/procfs.h.in
  `# ifndef FD_SET` with a plain text). Reference: no diagnostic.
* libniji emits no `#warning` for `_BSD_SOURCE` / `_SVID_SOURCE`; the reference warns (deprecated).
* `struct f_owner_ex.type` is `unsigned int` (layouts say `enum __pid_type`); pre-existing generator behaviour; C++
  code assigning an `enum __pid_type` member to a variable of that enum type would differ.
* `_REENTRANT` under strict C turns on POSIX..POSIX199506 `__USE_*` (matched), libniji's own declarations keep the
  strict level. Numeric `__USE_*` levels a program defines itself are not honoured (only MISC/GNU).
* openssl SOL_TCP: current netinet/tcp.h has SOL_TCP + TCP_NODELAY in gnu17 (unit passes); the 25.2 failure is not
  reproduced; check in the package proof.

## RULING NEEDED (hand-back)
* R-H2 limited `__USE_*` to names external code tests; the brief's coordinator decision defines the whole probed
  family (25 names). Recommend keep (npth, kernel UAPI and 7 corpus names test them; matching the whole family costs
  nothing).

## Exact next step
1. Package proofs as jobs on keke against the system flavour (`cargo xtask so`, include target/niji/include):
   gnupg2 `dirmngr/certcache.c` with npth.h, flex scanner defining SIZE_MAX first (or gcc-14 gengtype-lex.cc),
   python3.13 `Modules/fcntlmodule.c`, util-linux `lib/monotonic.c`, openssl ktls.h users (`ssl/ktls.c`?), less
   `pattern.c`. Sources: /srv/mirrors/debian/pool/main/...; extracted less/openssl in
   /dev/shm/claude/headers-features-forms/src on keke. Compile-only (`-fsyntax-only`) is enough.
2. clippy + static on the role; `cargo xtask affected --list --base origin/next` then the gate
   (`headers-features-forms-gate`).
3. Plants file (docs/plants.md form), e.g.: `redefined()` returns false (catches `FEATURES flex SIZE_MAX` / forms);
   cond.rs gnu back to `__NIJI_GNU` (`FEATURES pkg less regex`); a `__USE_XOPEN2K` formula in features.h.in
   (`FEATURES use`); SUN_LEN line removed (`FEATURES pkg fstrm`); @@intc `# ifndef` back (`FEATURES forms stdint.h`).
   cmd: `python3 spec/tools/header_checks/features.py --only '<title regex>'` after `cargo xtask headers-gen`
   (check how other header plants regenerate). Run via `cargo xtask plants`.
4. precheck, report json (`handoff/reports/HEADERS-FEATURES-FORMS.json`), handoff-check, hand-back.

Jobs so far (keke): headers-features-forms-probe4 (libniji probes), -probe5 (superseded: unsplit batch, wrong),
-check6..9 (features module; check9 = final module state).
