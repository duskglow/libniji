# HEADERS-STAGE2-GAPS-D: perl's gaps, IN_* class macros, sockaddr union member names, array parameters, __WALL, __timezone, IN6ADDR_*_INIT

**State:** branch `task/headers-stage2-gaps-d` (from origin/next ecc8c95e3), role keke:~/w/n1. Leg 3 (last). Code head
02478be5c (later commits: handoff and report only). Decisions: none new; every change follows black-box probes of the
reference (no differences row, no compat option). Scratch probes (not committed) on keke `~/work/headers-stage2-gaps-d/`:
`gp.py` (1310 stock-vs-libniji probes), `aps.py` (array parameters), `sj*.sh` (`__sigsetjmp`), `in6.sh` (IN6ADDR form
and value), `proof.sh`/`debs.sh`/`ystage2.sh` (proof), `pfail.awk` (perl failing-test names).

## Done (commits `git diff origin/next...HEAD`)
* Leg 1: perl's `__priority_which_t`, `h_addr`, `struct ip_mreq_source`; glib's IN_CLASSA..IN_BADCLASS; fakechroot's 13
  `__<tag>__` union members; gperftools' `__WALL`/`__WCLONE`/`__WNOTHREAD`; icu's `__timezone`/`__daylight`/`__tzname`;
  array parameters (`[N]` in the generator, arrayparams.tsv +29 rows). gdb pr*: no difference at this head.
* Leg 2: regenerated headers and layouts; header_check units (s12, s07); pthread.h no longer shows `__sigsetjmp` (the
  cleanup macros call `__niji_sigsetjmp`, asm label, `returns_twice`); plants file.
* Leg 3: dc8ff73f0 plants: s2d-mreq-order retargeted to niji-abi's layout test (it SURVIVED header_check in plants1: the
  header follows table order, not offsets), s2d-mreq-rows added (row swap, header_check), s2d-bound-dropped msg names
  the generator's rejection, s2d-bound-wrong added. d324c9b79 + 02478be5c (regen): **IN6ADDR_ANY_INIT / IN6ADDR_LOOPBACK_INIT** (perl's
  proof showed IO-Socket-IP t/04local-client-v6 and t/05local-server-v6 failing on libniji only: perl's Socket defines
  IN6ADDR_LOOPBACK only `#ifdef IN6ADDR_LOOPBACK_INIT`). Probe: visibility like IN_CLASS (20 rows, header_probe.py on
  the 10 headers showing in.h names), three brace levels, exactly 16 elements, no designators, bytes == in6addr_*;
  libniji's headers now give the same result on every probe line (PROBE-SAME). s12 unit `in6addr initializers`, 2 plants.

## Results (keke)
* gate3 (head 3d51d189e, products built first): `cargo xtask so`, `cargo test -p xtask` all green (transcripts 10/10),
  full header_check 0 failed of 20364, 6 cleanup-macro conformance programs green. rc 0.
* gate1 red (SUPERSEDED): `xtask --test transcripts` ENOENT: sync_role.sh removes `target/niji` (line 113) and the
  leftover `target/niji-test/gencat` names the removed loader; the gencat test, unlike the iconv one, does not skip that.
* proof1 (debs at 27f324fbe): perl nocheck package rc 0 (7 debs); checked: 6 failing tests = the 4 Net-Ping raw-socket
  ones stock also fails + the 2 IO-Socket-IP v6 ones (fixed above, NOT re-proven by a perl rebuild). faketime rc 0;
  fakechroot `bind.c`, glib `ginetaddress.c`, icu's putil users compile; the packages stop later on other stories' gaps:
  fakechroot FTS64 members (`<fts.h>`), glib `TCP_NODELAY` (HEADERS-NET-SYS-NAMES-3), icu `ELF64_ST_INFO` (`<elf.h>`,
  HEADERS-STAGE2-GAPS-B).
* hc6: header_check in6/exact/address-class units 0 failed of 117; in6.sh PROBE-SAME.
* Final head jobs: gate4 (clippy -D warnings, static, header_p2_fixed_rows, full header_check), plants2 (16 plants).

* gate4 rc 0 on code head 02478be5c. plants2 (16 plants, still running at hand-back, 15:50Z): 7 CAUGHT so far and
  **s2d-mreq-order SURVIVED again** with niji-abi's `abi_struct_ip_mreq_source`: an offset column that disagrees with
  the row order in layouts.tsv is caught by neither header_check (header follows row order) nor niji-abi.

## Exact next step (run ended at the wall-time limit: INCOMPLETE)
1. `~/bin/jobwait headers-stage2-gaps-d-plants2`; read its summary (`grep -E "SURVIVED|WARN-" ~/logs/...plants2.log`).
2. s2d-mreq-order: find what abi-gen does with the offset column (read `xtask/src/` abi-gen, not the generated file)
   and add a check that the generator (headers-gen or abi-gen) rejects a table whose offsets disagree with its member
   order (then the plant's msg names that error), or record `equivalent:` only if no product changes.
3. Sync (rc 4 = BUSY while plants run), `cargo xtask precheck --jobs headers-stage2-gaps-d-gate4,<plants label> --head
   <sha>`, handback-check. Full report: handoff/HEADERS-STAGE2-GAPS-D-report.md.
