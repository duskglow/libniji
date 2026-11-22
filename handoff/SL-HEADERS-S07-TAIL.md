# SL-HEADERS-S07-TAIL: slice 7's last five headers (handoff, run 1 leg 1)

**State.** Branch `task/sl-headers-s07-tail` from origin/next f15d8264f (no merge needed: slices 8, 14, 12, 15 and
HEADERS-GEN-COMPLETE / REGISTRY are all on that base). Commits: 239cee028 (five headers, guard removal, sys/ioctl.h
includes, s07.py, P10 W*), 62be0c87b (netdb.h). Role mei:~/w/s4. Data only: no file under `xtask/src/headers*`
outside `slices/` and not `spec/tools/header_check.py` changed (slice contract).

**Done.**
* Templates `headers/{sys/wait,wait,sys/param,spawn,sys/ioctl}.h.in`, rows in `headers/slices/s07.tsv` (p1 1, uapi -),
  `libc-include/` regenerated on the role and copied back (never hand-edited).
  * `sys/wait.h`: `@@needs-if SIGABRT signal.h` = `#if __NIJI_POSIX >= 200809 || __NIJI_XSI >= 450 / #include <signal.h>`:
    exactly the configurations where visibility.tsv shows all of signal.h in sys/wait.h (scratch analysis); outside them
    sys/wait.h shares only reserved `__x`/guard names with signal.h. W* macros: stdlib.h's text plus WIFSIGNALED,
    WIFSTOPPED (no visibility row: the spec never names them, so the probe has no candidate), WCOREDUMP.
  * `wait.h` = include of sys/wait.h. `sys/param.h` = plain includes of sys/types.h, limits.h, endian.h, signal.h (all
    of each in every configuration) + its own constants. `spawn.h` = sched.h, sys/types.h + emit. `sys/ioctl.h` =
    `<asm/ioctls.h>` (as termio.h) + emit, owns TCSETS2.
* Ruling #6406: arpa/nameser.h includes sys/param.h plainly. netdb.h includes NOTHING for signal.h: a plain include
  showed 1155 extra names (P2), and no configuration of netdb.h shows the whole of signal.h, so the generator declares
  its signal names itself (P2 netdb.h 0 of 814). The guard check: `spec/tools/header_checks/s07.py` `problems()` fails on
  any `__has_include(<H>)` in libc-include/ where `headers/H.in` exists. `@@needs` (s12.rs) now has no user.
* Slice 15's next step done: sys/raw.h, sys/mtio.h, sys/mount.h `#include <sys/ioctl.h>`, `@@owns TCSETS2 TTYDEF_*` dropped.
  NOT done: emptying `DEFERRED` in `spec/tools/header_checks/s15.py` (try it; keep if P1 fails).

**Proof so far (job sl-headers-s07-tail-proof1 on 239cee028, outputs mei:~/work/sl-headers-s07-tail/out1/*.out;
script handoff/SL-HEADERS-S07-TAIL/proof.sh).** P1/P5 header_check 0 failed of 14067 (also on 62be0c87b, job gen3).
P10 header_p10 0 failed (W* over every 16-bit status vs the reference, FD_*, major/minor). P2 (11 headers): 1197
differences, of which 1155 netdb.h (fixed in 62be0c87b), 27 sys/mount.h MS_* (COMPAT-34, accepted by S15), 6 reserved
`__pid_t` in sys/wait.h/wait.h (D-X.118 class), and 1 "differs" each in sys/param.h, resolv.h, arpa/nameser.h: NOT
YET READ (`grep -v "^extra netdb" out1/p2.out`). P3: `waitid` call errors in sys/wait.h (see Open).

**Open.**
1. `idtype_t` has no typedefs.tsv/enumerators.tsv row, so it is emitted as an opaque 4-byte struct and `waitid (P_PID,
   ...)` does not compile (P3). Read handoff/HEADERS-TABLES-COMPLETE.md for how an enumeration typedef is a table row;
   settle the reference's form by black-box probe (enum type? tag?) and add the rows; regenerate; rerun P3. If the
   tables cannot express it: RULING NEEDED naming HEADERS-TABLES-COMPLETE.
2. RULING NEEDED: sys/param.h's function-like macros (MIN MAX howmany roundup powerof2 setbit clrbit isset isclr) are in
   no table and the spec names none; recommend a spec + table story. INFORMATIONAL: stdlib.h lacks WIFSIGNALED/WIFSTOPPED.
3. Rulings #6377/#6378: check the getumask / bsd_signal rows (differences, COMPAT-32/33) still match (neither declared).
4. Not run yet: P6 (`conformance/run.sh --niji-headers`), P7 (`smoke.sh --niji-headers xz_utils sqlite3`, undef-compare),
   plants (`handoff/SL-HEADERS-S07-TAIL/plants.txt`, 7), clippy/static, gate, precheck.

**Next step.** Read the three P2 "differs" lines; fix Open 1; rerun proof.sh (new out dir); then clippy, static, plants
and gate as jobs (`cargo xtask affected` first: FULL escalation = precheck + changed tests), precheck, report json
`handoff/reports/SL-HEADERS-S07-TAIL.json`, `ops handback-check`.
