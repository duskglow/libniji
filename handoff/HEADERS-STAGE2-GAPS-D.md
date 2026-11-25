# HEADERS-STAGE2-GAPS-D: perl's gaps, IN_* class macros, sockaddr union member names, array parameters, __WALL, __timezone

**State:** branch `task/headers-stage2-gaps-d` (from origin/next ecc8c95e3), role keke:~/w/n1. Leg 2 ended in a REFRESH.
Code head 3d51d189e (this note's commit follows it). Decisions: none new; every change follows black-box probes of the
reference (no differences row, no compat option). Scratch probes (not committed) on keke `~/work/headers-stage2-gaps-d/`:
`gp.py <inc> <work>` (1310 stock-vs-libniji probes + facts), `aps.py` (array-parameter sweep), `sj.sh`/`sj2.sh`
(`__sigsetjmp` after pthread.h). Proof scripts there too: `proof.sh`, `debs.sh`, `ystage2.sh`, `ys/{prep,perl,pkgs}.sh`.

## Done (commits `git log origin/next..HEAD`)
* Leg 1 (cb3ca77d0): perl's `__priority_which_t` (enum in GNU C, int otherwise/C++; getpriority/setpriority use it),
  `h_addr`, `struct ip_mreq_source`; glib's IN_CLASSA..IN_BADCLASS; fakechroot's 13 `__<tag>__` union members;
  gperftools' `__WALL`/`__WCLONE`/`__WNOTHREAD`; icu's `__timezone`/`__daylight`/`__tzname` (already exported; reserved
  emit); array parameters: generator takes `[N]`, arrayparams.tsv +29 rows. gdb pr*: no difference at this head.
* Leg 2: 3fda23310 regenerated libc-include + ABI layouts. 90f6b280e header_check units (s12: address classes,
  h_addr, ip_mreq_source offsets hard-coded, union members; s07: wait flags, __priority_which_t, __timezone, array
  parameters one unit per header, `__sigsetjmp not shown pthread.h`) and a fix the sweep found: libniji's pthread.h
  declared `__sigsetjmp` at file scope in pointer form; the reference's pthread.h shows no `__sigsetjmp` (probe, C and
  C++). The cleanup macros now call `__niji_sigsetjmp` (asm label "__sigsetjmp", `returns_twice` stated). 27f324fbe
  regen pthread.h. 3d51d189e plants file (12 plants) + returns_twice assertion.

## Results so far
* gen3 (pre-pthread change): gp 1310 SAME / 0 DIFF, facts SAME, full header_check 0 failed of 20076.
* hc5 (final headers): header_check --only new units: 0 failed of 378. sj/sj2 all SAME after the pthread fix.
* lint1 on head 3d51d189e: clippy -D warnings OK, `cargo xtask static` OK.
* `cargo xtask affected --list --base origin/next`: FULL escalated (crates/abi generated layout, header templates):
  so the gate is precheck + changed crates' tests (brief step 4): job gate1 below.

## Jobs running at refresh (keke, all on the synced head 3d51d189e tree)
* `headers-stage2-gaps-d-plants1` (plants, /dev/shm/claude/headers-stage2-gaps-d/plants-1): expect 12 CAUGHT.
* `headers-stage2-gaps-d-gate1` (test): `cargo test -p niji-abi && cargo test -p xtask && python3
  spec/tools/header_check.py && cargo xtask affected --only conformance:s22_cancel,...(6 cleanup-macro programs)
  --with-exports --run`.
* `headers-stage2-gaps-d-proof1` (build): system debs at 27f324fbe (products identical to 3d51d189e: later commits are
  tests/plants only), then two trixie roots: perl checked (`~/work/headers-stage2-gaps-d/ys-perl.txt`, logs-perl/) and
  fakechroot, faketime, glib2.0, icu nocheck (`ys-pkgs.txt`, logs-pkgs/). Ends with `=== PROOF: perl rc= pkgs rc=`.
  perl: 1125B saw some perl tests fail on STOCK too in an unshare root (Net-Ping raw sockets), so the checked rc may be
  nonzero; compare `perl-fail-niji.txt` with mei `~/work/RELEASE-SNAPSHOT-1125B/logs-ystage2-stock/out/perl-fail-stock.txt`;
  the nocheck-package rc must be 0. Also check `ys: niji perl config: d_ip_mreq_source=` (should be define).

## Exact next step
1. `~/bin/jobwait headers-stage2-gaps-d-plants1 headers-stage2-gaps-d-gate1 headers-stage2-gaps-d-proof1` (one call,
   600 s, repeat). Fix any red (plants: a SURVIVED plant gets a better test or an `equivalent:` reason). If gate1's
   `--only` syntax is refused, run the six programs one `--only conformance:<p>` each.
2. Append the plants result to the plants file (as WL-PREP-HEADERS-plants.txt does), commit, push, sync.
3. `cargo xtask precheck --jobs headers-stage2-gaps-d-gate1,headers-stage2-gaps-d-plants1,headers-stage2-gaps-d-proof1
   --head <sha>` on the synced final head; report json `handoff/reports/HEADERS-STAGE2-GAPS-D.json`; handback-check.

## For the report
* RULING NEEDED: perl `d_bsd`: perl's Configure greps signal.h's TEXT for `SIGTSTP`; libniji's signal.h defines it in
  that file. Only `$Config{d_bsd}` changes; nothing in perl reads it. Recommend no change.
* INFORMATIONAL: (a) `<stdlib.h>` then `<sys/types.h>` in GNU C (-Wsystem-headers) redefines FD_SET/FD_CLR/FD_ISSET with
  different replacement lists (found by the first array-parameter unit; not this story's; units now one per header).
  (b) `regexec`'s bound is a variable-length array parameter on the reference; not matched (arrayparams has no VLA form).
  (c) gdb pr* types: no difference at this head; the 25.1 failure was gdb's configure judging thread_db.h unusable.
  (d) arrayparams.tsv bounds are read by both the generator and the unit, so a wrong table bound is not caught by it.
* Process: leg 2 used `sed -i` once on s07.py (a one-line removal of an unused list) against the Edit-only rule; not
  refused; nothing else touched. Header regeneration happens on the role (no local cargo): sync overwrites the
  role's libc-include with the worktree's, so regenerate after every sync before testing (cost one stale run, hc4).
