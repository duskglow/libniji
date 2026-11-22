# SL-HEADERS-S07-TAIL: slice 7's last five headers (handoff, run 1 leg 2)

**State.** Branch `task/sl-headers-s07-tail` from origin/next f15d8264f, final code head **be67b5b63** (this note is
committed on top). Role mei:~/w/s4 (synced at be67b5b63). Data only: no generator code changed.
Commits: 239cee028 (five headers, guards removed, sys/ioctl.h includes, s07.py, P10 W*), 62be0c87b (netdb.h signal
names), 780086986 (differences rows name the new headers, P2 accept file, p67.sh), 59764b091 + f89e764bf (s15.py:
f_owner_ex checked; OPEN_TREE_CLOEXEC stays deferred, see Findings), be67b5b63 (diagattrs.tsv: 29 spawn.h rows by
`attr_probe.py table`; spawn.h regenerated; plant s7t-spawn-nonnull).

**Jobs on be67b5b63, started 13:5xZ on mei, NOT YET READ** (next leg: `~/bin/jobwait` them first):
* `sl-headers-s07-tail-proof3` (proof.sh: P1/P5, P10, P2 with accept file, P3; outputs ~/work/sl-headers-s07-tail/out3)
* `sl-headers-s07-tail-test2` (`cargo test -p xtask`: the rows test and slice 19's attribute test; test1 on f89e764bf
  failed only diagnostic_attributes_match_the_reference on 29 spawn.h functions, fixed by be67b5b63)
* `sl-headers-s07-tail-plants2` (8 plants, dir /mnt/zram/claude/sl-headers-s07-tail/plants-2)
Tier: `cargo xtask affected --list` = FULL (escalated: generated headers): "FULL: the train's". Gate = precheck +
test2. SUPERSEDED: proof2 (f89e764bf's parent tree: P1 17 failed from the first s15 change), lint1 (clippy rc 0,
static rc 0, on 59764b091; rerun not needed for data-only changes but precheck re-checks), hc3 (P1 0 of 14067 on
f89e764bf), test1 (red, fixed), plants1 (stopped: older head, missing the new plant), attr1 (the probe run), p67
(P6/P7 on f89e764bf; spawn.h attributes changed since, results below still hold for P6/P7's purpose).

**Results so far.** P1/P5 0 failed of 14067 (hc3). P10 0 failed. P2 0 unaccepted of 36 (accept file:
`handoff/SL-HEADERS-S07-TAIL/p2-accept.txt` + S15's; siginfo_t via <signal.h> = D-X.129, reserved __pid_t etc =
D-X.118, both rows now name these headers; MS_* = COMPAT-34). P3: 2 unaccepted = waitid (RULING 1). P6
(`conformance/run.sh --niji-headers`): 129 passed, 9 failed, 8 not covered; 5 fail to build on other slices' names
(PA_INT printf.h; RTLD_DI_*/RTLD_DL_LINKMAP dlfcn.h; ELFMAG elf.h), 1 pending by ruling (x_self_bind_stdio), and
s02_assert, s02_chk_fail, s02_report, s08_strto_fenv_flags fail at run time: newly covered (they include
<sys/wait.h>), cause NOT diagnosed; s02_report's child differs in exit status AND output text (error()/err() path),
W* decoding is proved by P10 for every status. P7 (`smoke.sh --niji-headers`): zlib 1/1, xz-utils 19/19, sqlite3
1/1, same as the reference build. undef-compare 28 of 175 objects differ: `_chk` pairs (slice 18), atoi/strtol,
bcmp/memcmp, and sqlite3/fuzzcheck reference no math functions in the libniji build although configure is identical
(-lm found, SQLITE_ENABLE_MATH_FUNCTIONS both sides): not these headers (math.h, niji-cc link), INFORMATIONAL.

**Rulings / findings.**
1. RULING NEEDED (HEADERS-TABLES-COMPLETE): typedefs of an unnamed enumeration cannot be tabled: `idtype_t`
   (sys/wait.h), `ACTION`, `VISIT` (search.h) are emitted as opaque 4-byte structs, so `waitid (P_PID, ...)` and
   `hsearch (e, FIND)` do not compile. Probe (reference, g++ -std=c++17): all three are enums, `decltype (P_PID)` is
   idtype_t (FIND: ACTION, leaf: VISIT), no tag of that name (`enum idtype_t x;` = incomplete). Recommend an
   enumerators.tsv tag form `typedef NAME` emitted as `typedef enum { ... } NAME;`, checked by the rows test.
2. RULING NEEDED: sys/param.h's MIN MAX howmany roundup powerof2 setbit clrbit isset isclr are in no table and no
   spec clause; recommend a spec + table story.
3. Rulings #6377/#6378 hold: reserved.tsv omits getumask and bsd_signal, no libniji header declares them (these five
   included), matching differences rows (COMPAT-32/33). INFORMATIONAL: providers.tsv's getumask row (#6715, "declared
   as the reference does") is inert because reserved.tsv omits the name; one of the two is stale.
4. INFORMATIONAL: sys/mount.h's OPEN_TREE_CLOEXEC does not compile in 17 strict configurations (linux/mount.h defines
   it as O_CLOEXEC, not visible there); kept in s15.py DEFERRED with the reason. stdlib.h lacks WIFSIGNALED/WIFSTOPPED.

**Next step.** jobwait proof3 test2 plants2; read P1 (0 failed expected), P3 (only waitid), test2 (green expected),
plants (8 CAUGHT expected; list WARN lines). Decide P6's four run-time failures: one look each at
conformance/build/<name>.lh.log.stderr on the role; if not this story's headers, INFORMATIONAL with evidence.
Then on the role `cargo xtask precheck --jobs sl-headers-s07-tail-proof3,sl-headers-s07-tail-test2,sl-headers-s07-tail-plants2
--head <final sha>`, write handoff/reports/SL-HEADERS-S07-TAIL.json (+ "jobs"), push, `ops handback-check`. Verdict:
INCOMPLETE if P3/P6 gaps must block, else COMPLETE with the two RULINGs (waitid is unusable until ruling 1).
