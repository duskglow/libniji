# HEADERS-STAGE2-GAPS-D: perl's gaps, IN_* class macros, sockaddr union member names, array parameters, __WALL, __timezone, IN6ADDR_*_INIT

**State:** branch `task/headers-stage2-gaps-d-2` (continues `task/headers-stage2-gaps-d`, from origin/next ecc8c95e3), role
keke:~/w/n1. Leg 4 (HEADERS-STAGE2-GAPS-D-2) is COMPLETE: code head 79e42f02e (later commits: handoff and report only).
Decisions: none new; every change follows black-box probes of the reference (no differences row, no compat option).
Full report: handoff/HEADERS-STAGE2-GAPS-D-report.md.

## Done (commits `git diff origin/next...HEAD`)
* Legs 1-3: perl's `__priority_which_t`, `h_addr`, `struct ip_mreq_source`; glib's IN_CLASSA..IN_BADCLASS; fakechroot's 13
  `__<tag>__` union members; gperftools' `__WALL`/`__WCLONE`/`__WNOTHREAD`; icu's `__timezone`/`__daylight`/`__tzname`;
  array parameters (29 rows in arrayparams.tsv); pthread.h shows no `__sigsetjmp`; IN6ADDR_ANY_INIT/IN6ADDR_LOOPBACK_INIT;
  header_check units and the plants file. gdb pr*: no difference at that head.
* Leg 4 (79e42f02e): the plant survivor s2d-mreq-order was a real gap in the checks, not in the row. The header generator
  declares members in row order and never compared that with the offset column; niji-abi's types and layout test follow
  the same table, so they agree with a mutated table. `layout_order_problems` (xtask/src/headers.rs, called by
  `generate`) now rejects any layouts.tsv aggregate whose member offsets fall below the previous member's of the same
  level, or whose part (`m.x`) falls below `m`; bit-fields (storage-unit offset) and `<anon>` groups (restart at their
  own offset, `struct udphdr`) are exempt. It runs over ALL rows (the check, not only the one row); the real tables
  pass unchanged. Unit test `layout_offsets_follow_the_row_order`. Plants: s2d-mreq-order now names the generator's
  message; s2d-order-compare/-anon/-bits/-parent break the check and the unit test catches each.

## Results (keke, final code head 79e42f02e)
* headers-stage2-gaps-d-2-gate (`cargo xtask affected --base origin/next --run`, Direct tier): rc 0 (conformance 103 passed;
  clippy -D warnings and `cargo xtask static` also run in the foreground before it, both rc 0).
* headers-stage2-gaps-d-2-plants (20 plants, whole file): 20 CAUGHT, 0 SURVIVED, 0 WARN (only WARN-DEV-SHM, an environment note).
* Earlier legs' proof (perl nocheck builds, faketime, fakechroot bind.c, glib ginetaddress.c, icu putil): see the report.
  NOT re-run this leg: this leg changed only the generator's checks, no header text (headers-gen output unchanged).

## Known Configure difference (ruling #8606, accepted: no change)
perl `$Config{d_bsd}` differs from stock: Configure greps signal.h text for SIGTSTP, which libniji's signal.h defines in that
file; nothing in perl reads d_bsd.

## Open / things not to redo
* Wiring of `layout_order_problems` into `generate` (one line) is covered by s2d-mreq-order only while the line is present;
  no plant removes the line (a plant cannot also mutate layouts.tsv in the same copy).
* The perl rebuild after the IN6ADDR fix is still NOT VERIFIED end to end (leg 3 note).
* Next stories: fakechroot `<fts.h>` FTS64 members, glib TCP_NODELAY (HEADERS-NET-SYS-NAMES-3), icu ELF64_ST_INFO
  (HEADERS-STAGE2-GAPS-B) stop the packages after this story's gaps.
