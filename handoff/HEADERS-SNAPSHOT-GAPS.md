# HEADERS-SNAPSHOT-GAPS: header gaps that broke tree, xxhash and static busybox (snapshot 24.2), sig_t (apt), stale "no headers" texts

**State:** branch `task/headers-snapshot-gaps` (from origin/next 460d23b8f), role keke:~/w/n1. Code head fc5f5b84a
(later commits: handoff/report only). Decisions: none new; every change follows the reference's probed visibility,
type and value (no differences row).

## What was done (commits on the branch)
* 251c06db3 docs: libc6-dev / niji6-dev descriptions, README.md:27, SL-HEADERS-S20 fragment (niji-gcc, niji-g++),
  install-packages.md (no a.out.h).
* eb3d41f4f, 0ec35ae45: `spec/tools/header_probe.py` `package_names()` (names from the failed builds plus their
  families) and the rule "each reserved `__X_t` candidate also probes `X_t`"; `--names FILE` subset probes. Rows
  spliced into visibility.tsv (1968), typedefs.tsv (29), macros.tsv (21); committed rows unchanged.
* 73c68a7a9: `<sys/stat.h>` S_IS* spelled with `@@value(S_IF*)` (xxhash: its build log shows S_ISDIR/S_ISREG
  expanding to S_IFMT/S_IFDIR, which the configuration hides; the reference keeps S_IS* usable there).
* c0a3db2c4: `RE_TRANSLATE_TYPE` in regex.h.in (`unsigned char *`, black-box probe in GNU mode; = layouts.tsv
  translate member), libc-include/ regenerated.
* 161769495: `header_check.py` `exact_units` (ctx.exact): each name exactly where visibility.tsv shows it, with
  size/alignment/C++ spelling, char[N] size and C++11 characters, type-valued macro type; absent elsewhere. Rows:
  s05 (27 sys/types.h names), s07 (utmp/wtmp paths in paths.h, utmp.h, utmpx.h, lastlog.h), s10 (RE_TRANSLATE_TYPE).
* ddbe69c38 (coordinator note, apt): `sig_t` probed (signal.h, sys/signal.h, sys/wait.h, wait.h, sys/param.h,
  resolv.h, arpa/nameser.h; default/bsd/svid/gnu/C++), typedef `void (*)(int)`; s08 rows for sig_t and sighandler_t.
* fc5f5b84a plants file handoff/HEADERS-SNAPSHOT-GAPS-plants.txt (8 template plants; table mutants move the check's
  expectation too, so plants mutate templates).

## Results
* header_check (keke, job headers-snapshot-gaps-hc2, form probe on): 0 failed of 17579.
* plants: job headers-snapshot-gaps-plants1 (see report).
* debs + ypk/hp on mei: see report.

## Exact next step
See handoff/HEADERS-SNAPSHOT-GAPS-report.md "Next step".
