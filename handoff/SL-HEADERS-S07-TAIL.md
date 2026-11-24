# SL-HEADERS-S07-TAIL: slice 7's last five headers (handoff, run 1 leg 3)

**State.** Branch `task/sl-headers-s07-tail` from origin/next f15d8264f; final code head **cbb151bc0** (fc0904996: plants
file only; this note and the report json are committed on top). Role mei:~/w/s4. Data and slice check module only: no generator code changed.
Commits: 239cee028 (five headers `sys/wait.h` `wait.h` `sys/param.h` `spawn.h` `sys/ioctl.h`, slice 12 guards
removed, s07.py guard check, P10 W* checks), 62be0c87b (netdb.h signal names), 780086986 (differences rows name the
new headers, P2 accept file), 59764b091 + f89e764bf (s15.py: f_owner_ex checked), be67b5b63 (diagattrs.tsv: 29
spawn.h rows), 2602431a2 + cbb151bc0 (s07.py: P1 of the macros these headers show but another header owns).

**Proofs (plan/headers.md section 3).**
* Final-head jobs (ce50e469b, mei): `proof4`, `test3`, `hc5`, `plants4` (labels `sl-headers-s07-tail-<x>`); precheck.
* P1/P5 header_check: 0 failed of 14262 (`hc5`, `proof4`; 14067 before the new units).
* P10 header_p10: 0 failed; P2 header_probe: 0 unaccepted of 36 (accept file `handoff/SL-HEADERS-S07-TAIL/p2-accept.txt`
  + S15's; D-X.129 siginfo_t via <signal.h>, D-X.118 reserved names, COMPAT-34 MS_*); P3 callsite_probe: 2 unaccepted,
  both `waitid` (ruling 1) (`proof4`).
* `cargo test -p xtask` (rows test, diagnostic attributes): 609 passed, 0 failed (`test3`).
* Guard removal (ruling #6406): plain includes in arpa/nameser.h, resolv.h, netdb.h; s07.py `problems` fails on any
  generated header that `__has_include`-guards an include of a header libniji ships (plant s7t-guarded-include).
* Plants: `plants2` (be67b5b63) 6 CAUGHT, 2 SURVIVED: s7t-param-value (MAXSYMLINKS) and s7t-ioctl-value (TIOCGWINSZ).
  Cause: the table P1 asserts a macro value only in the header owning its macros.tsv row (arpa/nameser.h, `p1` 0;
  net/if_ppp.h, unshipped), so those values were checked nowhere. Fixed by s07.py's units: `plants3` (cbb151bc0) 7
  CAUGHT; s7t-ioctl-value still survived because a TIOC* value in sys/ioctl.h is dead text (<asm/ioctls.h>, included
  first, defines it before the `#ifndef` fallback), so fc0904996 plants CQUIT (sys/ioctl.h's own text): `plants4`.
* P6 (`conformance/run.sh --niji-headers`): 129 passed, 9 failed, 8 not covered. 5 fail to build on other slices'
  names (PA_INT printf.h; RTLD_DI_*, RTLD_DL_LINKMAP dlfcn.h; ELFMAG elf.h); 1 pending by ruling #7460
  (x_self_bind_stdio). The four run-time failures (`p6r`: all four pass with host headers) have causes outside these
  five headers: s02_assert: the harness names the executable `s02_assert.lh`, the expected text says `s02_assert:`;
  s02_chk_fail: no `_chk` wrappers in libniji's string.h yet (slice 18), so the stack protector reports the overflow;
  s08_strto_fenv_flags: all 61 failures are `atof`, declared `__pure__` by diagattrs.tsv (on next), so a caller may
  reuse errno's value from before the call and miss ERANGE; s02_report: a declaration in a header outside this slice
  (already on next), details to the coordinator in the hand-back.
* P7 (`smoke.sh --niji-headers`): zlib 1/1, xz-utils 19/19, sqlite3 1/1, as the reference build; undef-compare: 28 of
  175 objects differ (`_chk` pairs, atoi/strtol, bcmp/memcmp, sqlite3 math functions: not these headers).

**Rulings / findings.**
1. RULING NEEDED (HEADERS-TABLES-COMPLETE): typedefs of an unnamed enumeration cannot be tabled: `idtype_t`
   (sys/wait.h), `ACTION`, `VISIT` (search.h) are emitted as opaque structs, so `waitid (P_PID, ...)` does not compile
   (P3's 2 rows). Probe: all three are enums, no tag. Recommend an enumerators.tsv form `typedef NAME`.
2. RULING NEEDED: sys/param.h's MIN MAX howmany roundup powerof2 setbit clrbit isset isclr are in no table or clause;
   recommend a spec + table story.
3. INFORMATIONAL: header_check.py's table P1 checks a value only in the owning header; other slices' headers that
   show borrowed macros have the same gap (recommend the shared check assert every visible integer macro, skipping
   reserved names; s07.py does this for slice 7's five).
4. INFORMATIONAL (slice 8): with `_GNU_SOURCE` or `_DYNAMIC_STACK_SIZE_SOURCE`, libniji's <signal.h> defines
   MINSIGSTKSZ/SIGSTKSZ as `__sysconf (...)` with no declaration: C++ rejects a use, the reference compiles it
   (black-box probe, C and C++, signal.h and sys/wait.h).
5. INFORMATIONAL (slice 19): `atof`'s `pure` row (above). Rulings #6377/#6378 hold (getumask, bsd_signal undeclared,
   COMPAT-32/33); providers.tsv's getumask row is inert. sys/mount.h's OPEN_TREE_CLOEXEC stays deferred in s15.py.

**Not to redo.** Header generation, guard removal, the probes above, the P6 triage (`~/work/sl-headers-s07-tail/p6r`).
**Next step.** None for this story once plants3 is all CAUGHT; the landing needs FULL: the train's (generated headers).
