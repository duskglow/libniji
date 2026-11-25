# HEADERS-FEATURES-FORMS (handoff, leg 1 of the run; WIP)

**State:** branch `task/headers-features-forms` (from origin/next db602e58c), role keke:~/w/r2. Head: the WIP commit
after this note. Generated headers are NOT regenerated yet (`cargo xtask headers-gen` not run).

## Probe facts (black box; keke = reference system, gcc 13; tool `spec/tools/feature_probe.py`, committed)
* **`__USE_*`** (`feature_probe.py use`, 67 configurations = ftm-configs' 40 + 27 extra): the reference defines 25
  names: ISOC95, ISOC99, ISOC11, ISOCXX11, POSIX, POSIX2, POSIX199309, POSIX199506, XOPEN, XOPEN_EXTENDED, UNIX98,
  XOPEN2K, XOPEN2KXSI, XOPEN2K8, XOPEN2K8XSI, LARGEFILE, LARGEFILE64, FILE_OFFSET64, MISC, ATFILE,
  DYNAMIC_STACK_SIZE, GNU, FORTIFY_LEVEL (always; = level), EXTERN_INLINES, POSIX_IMPLICITLY. All `1` except
  FORTIFY_LEVEL. None of ISOC2X, TIME_BITS64, REENTRANT on x86_64. Raw table: `spec/abi/use-macros.tsv`
  (config, name, value). features.h.in now defines them from the levels (formulas checked by hand against every
  configuration; the libniji-side re-probe is NOT run yet). `_REENTRANT`/`_THREAD_SAFE` under strict C turn on
  POSIX..POSIX199506 (reference fact; libniji's own declarations keep the strict level: INFORMATIONAL).
* **Program-defined `__USE_*`** (manual probes): the reference clears a program's `__USE_GNU` at the first libc
  include, but one defined after a libc header widens later headers (less's pattern.h defines `__USE_GNU` before
  `<regex.h>`; same for `__USE_MISC`, `__USE_XOPEN2K`). Done: features.h `#undef`s the 25 names on first inclusion;
  cond.rs renders the misc/gnu levels as `defined __USE_MISC` / `defined __USE_GNU` (generated conditions only).
  Numeric levels (XOPEN2K etc.) are not honoured when a program defines them itself: INFORMATIONAL.
* **Forms** (`feature_probe.py forms`, gnu config, sentinel defined first): reference 38568 pairs replaced, 1483
  replaced with a non-#if form, 275 kept (include guards, some limits names) of 40326 probed; libniji kept 35594.
  `SIZE_MAX` flex case: the reference replaces with no warning, also under `-Werror` (warning only with
  `-Wsystem-headers`). Table `spec/abi/macro-forms.tsv` = the non-`replaced` pairs (4424, incl. inferred kept);
  generator (`headers.rs` `redefined()`): a macro visible in `gnu` and not in that table is defined outright.
* **Rows**: `header_probe.py` candidates + 47 visibility rows: F_OWNER_TID/PID/PGRP (kind enumerator, 6 headers,
  gnu+C++), TIMESPEC_TO_TIMEVAL/TIMEVAL_TO_TIMESPEC (10 headers: lastlog.h proc_service.h protocols/timed.h
  sys/procfs.h sys/profil.h sys/time.h sys/timex.h thread_db.h utmp.h utmpx.h; gnu+C++), SUN_LEN (sys/un.h, misc),
  SOL_UDP (1 header). SOL_TCP/SOL_IP are NOT in the reference's `<sys/socket.h>`; SOL_TIPC etc. are, and libniji
  already matches. Templates: sys/time.h.in (TIMESPEC macros), sys/un.h.in (SUN_LEN). The other 9 headers of
  TIMESPEC_* need the same text (or headers-gen will say so).

## Open
* `type_probe.py --retry-missing --names F_OWNER_TID,...` wrote no macros.tsv rows: F_OWNER_* need values
  (enumerators; probe with enum_probe.py or type_probe) before headers-gen.
* openssl: SOL_TCP/TCP_NODELAY undeclared in 25.2's log, but current netinet/tcp.h has both in gnu17; not
  reproduced. Proof step must compile openssl's bio_sock2.c (sources: /srv/mirrors/debian/pool/main/o/openssl).
* Sources of all six packages are in /srv/mirrors/debian/pool/main/...; extracted less/openssl parts in
  /dev/shm/claude/headers-features-forms/src on keke.
* R-H2 (owner) limited `__USE_*` to names external code tests; the brief's coordinator decision defines the whole
  probed family. RULING NEEDED in the hand-back (recommend: keep; npth, the kernel UAPI and 7 corpus names test them).

## Exact next step
1. Get F_OWNER_* values into the tables; add TIMESPEC_* text to the other 9 templates as headers-gen requires.
2. `cargo xtask headers-gen`, commit generated separately; re-run `feature_probe.py use --include libc-include` and
   `forms --include libc-include --pairs <ref full output ~/work/headers-features-forms/forms-ref.tsv>` and diff.
3. header_check module `spec/tools/header_checks/features.py`: units comparing `__USE_*` per config with
   use-macros.tsv, a SIZE_MAX-first unit, less/npth/python/util-linux/fstrm pattern units.
4. Package proofs, plants (docs/plants.md style file), precheck, report.
