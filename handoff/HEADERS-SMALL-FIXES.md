# HEADERS-SMALL-FIXES (handoff)

**State.** Branch task/headers-small-fixes, cut from origin/next 3fca4673c. Commits: ba0c33f14 (run.sh), 5e5232e1d
(signal.h `__sysconf` declaration, s08.py, conformance s20_sigstksz_dynamic), 40d3809b5 (sys/mount.h change reverted,
test narrowed), then this note and the report. Role mei:~/w/s1. Jobs: `headers-small-fixes-gate` (clippy, static,
`affected --run`, then ~/work/headers-small-fixes/check.sh: probe + header_check + P6), `headers-small-fixes-plants`
(handoff/HEADERS-SMALL-FIXES-plants.txt, 3 plants).

**Done.**
1. signal.h: under `__NIJI_DYNSTACK` the template declares `extern long int __sysconf (int) __NIJI_NOTHROW;` (guard
   `__NIJI_D___sysconf`, as the generated declarations). Probe (mei, Zen 1 host): a use of MINSIGSTKSZ + SIGSTKSZ
   with signal.h alone, C and C++, `_GNU_SOURCE` and `_DYNAMIC_STACK_SIZE_SOURCE`, `-Wall -Werror`: reference rc 0,
   libniji rc 0 (before the fix C++ failed). header_check: `0 failed of 17156` with the new s08.py units (every
   configuration that shows the macros in signal.h, sys/signal.h, sys/wait.h).
2. sys/mount.h OPEN_TREE_CLOEXEC: **premise did not hold.** Probe: the reference's sys/mount.h rejects a use of
   OPEN_TREE_CLOEXEC in all 17 strict configurations (c89 ... isoc2x), as libniji's does. Nothing to fix; the
   template change was reverted and s15.py keeps the name deferred with the probe as the reason.
3. P6 harness: `--niji-headers` executables go in conformance/build/lh/<name> (the program's own name, so
   assert's `prog:` text matches); the FAIL lines print the log actually written (`${log#"$root/"}`). s02_assert and
   s20_sigstksz_dynamic pass in `--niji-headers` mode.
4. Coordinator addition (#85, ruling #7055): sys/mount.h, sys/mtio.h, sys/raw.h already include <sys/ioctl.h>
   plainly on next (SL-HEADERS-S07-TAIL). **Not done:** closing the 18 TCSETS2/TTYDEF_* P2 rows (budget).

**RULING NEEDED (MINSIGSTKSZ's run-time value).** Probe on the Zen 1 host: under `_GNU_SOURCE` the reference's
MINSIGSTKSZ evaluates to 14528, equal to SIGSTKSZ (sysconf (_SC_SIGSTKSZ)) and not sysconf (_SC_MINSIGSTKSZ);
libniji's evaluates to sysconf (_SC_MINSIGSTKSZ) = 3632 there, as clause s20_7_i_0 says. Recommend matching the
reference (expand MINSIGSTKSZ to the `_SC_SIGSTKSZ` call: larger, so no program gets less room, and ABI-identical)
with a spec erratum on s20_7_i_0; the test asserts only what both meet (SIGSTKSZ == sysconf's; sysconf
(_SC_MINSIGSTKSZ) <= MINSIGSTKSZ <= SIGSTKSZ; the kernel accepts a MINSIGSTKSZ-byte stack).
NOT VERIFIED: the probe on a second host (Skylake-SP).

**Next step.** The ruling above; then the 18 TCSETS2/TTYDEF_* P2 rows (coordinator note #85).
**Not to redo.** The probes (~/work/headers-small-fixes/probe.py on mei; output in ~/logs/headers-small-fixes-check.log
and the gate log).
