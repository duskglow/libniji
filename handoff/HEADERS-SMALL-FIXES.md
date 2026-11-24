# HEADERS-SMALL-FIXES (handoff; last run HEADERS-SMALL-FIXES-2)

**State.** Branch task/headers-small-fixes-2 (continues task/headers-small-fixes), origin/next a0d93596e merged in.
Role mei:~/w/s1. Commits of run 1: ba0c33f14 (run.sh --niji-headers names), 5e5232e1d (signal.h `__sysconf`
declaration), 40d3809b5 (OPEN_TREE_CLOEXEC kept deferred). Run 2: 3a07eee3f (conformance report reads the new FAIL
lines), 3ae16aaec (plants), then this note, the plant results and the report.

**Run 2 done.**
1. CI red (`affected::tests::the_conformance_report_matches_the_runner_formats`): run.sh's FAIL lines print
   `${log#"$root/"}` since ba0c33f14. The test now pins those lines plus the `--niji-headers` log line. The report
   (`conformance_failures`) already took the stderr file from the FAIL line, but it read a build failure's log as
   `<name>.log`. It now reads the file named after "see conformance/build/" in the FAIL line (`<name>.lh.log` under
   --niji-headers) and falls back to `<name>.log` for lines that name no file. New test
   `a_failed_niji_headers_conformance_program_reports_its_own_files`. Plants: 2 of 2 CAUGHT
   (handoff/HEADERS-SMALL-FIXES-2-plants.txt).
2. Ruling #8393 triage, (c) = official docs:
   * glibc 2.34 NEWS (release notes, /usr/share/doc/libc6/NEWS.gz): "MINSIGSTKSZ is redefined to
     sysconf(_SC_MINSIGSTKSZ) and SIGSTKSZ is redefined to sysconf (_SC_SIGSTKSZ)" under `_DYNAMIC_STACK_SIZE_SOURCE`
     or `_GNU_SOURCE`.
   * glibc 2.39 manual, Signal Stack: MINSIGSTKSZ "is the amount of signal stack space the operating system needs
     just to implement signal delivery". Constants for Sysconf: `_SC_MINSIGSTKSZ` is "the minimum number of bytes of
     free stack space required in order to guarantee successful, non-nested handling of a single signal whose
     handler is an empty function". The manual gives no expansion; what it describes matches `_SC_MINSIGSTKSZ`.
   * sigaltstack(2): "MINSIGSTKSZ defines the minimum size required to execute a signal handler" (no expansion);
     sysconf(3): no entry for either name.
   * Black-box probe (Zen 1 host, `_GNU_SOURCE`): reference sysconf(_SC_MINSIGSTKSZ) 3632 = AT_MINSIGSTKSZ,
     sysconf(_SC_SIGSTKSZ) 14528, MINSIGSTKSZ 14528, SIGSTKSZ 14528. The docs agree with the spec (s20_7_i_0) and
     not with the probe, so the spec is kept and **MINSIGSTKSZ is unchanged** pending the coordinator's ruling.
   * Trixie check: on the trixie/sid build host, 16 ELF objects in 13 packages import sigaltstack: diffutils, gdb,
     grep, libgstreamer1.0-0, libhwasan0, libllvm18, liblsan0, libpython3.12t64, libsigsegv2, libubsan1,
     python3.12-minimal, vim, vim-tiny (~/work/headers-small-fixes-2/altstack-pkgs.txt). A glibc-built binary
     cannot show whether it wrote MINSIGSTKSZ or SIGSTKSZ: both compile to sysconf(_SC_SIGSTKSZ). Only a rebuild
     against libniji's headers is affected. Such a program gets 3632 bytes where the reference build gets 14528,
     so a program that sizes its whole alternate stack as MINSIGSTKSZ has less room for its handler.
3. Owed #85: the three templates already include <sys/ioctl.h> plainly (SL-HEADERS-S07-TAIL) and no longer own
   TCSETS2/TTYDEF_*. P2 rerun (header_probe --include on the staged headers, probe_compare with
   handoff/SL-HEADERS-S15/p2-accept.txt) for sys/mount.h, sys/mtio.h, sys/raw.h: `0 unaccepted of 27 differences
   in 1323 rows`, and all 18 TCSETS2/TTYDEF_* rows match the reference's (`!time64only`). P1: `header_check: 0
   failed of 17156`. **The 18 rows are closed.**

**RULING NEEDED (#8393).** The docs say `sysconf (_SC_MINSIGSTKSZ)`; the reference evaluates to SIGSTKSZ's value.
Recommendation: match the reference value (expand MINSIGSTKSZ to the `_SC_SIGSTKSZ` call) with a spec erratum on
s20_7_i_0 that cites the probe and records that NEWS says otherwise. A program rebuilt against libniji then never gets
less room than it gets from the reference. The cost is about 11 KB more for programs that size an alternate stack
from MINSIGSTKSZ. The alternative is to keep the documented value, add a differences row and a compat option.

**Next step.** Apply the ruling on #8393: change signal.h.in, s20_7_i_0 and s20_sigstksz_dynamic. The
`hsf-sc-swap` plant of handoff/HEADERS-SMALL-FIXES-plants.txt shows the pattern.
**Not to redo.** The doc triage and the probes (~/work/headers-small-fixes-2/ on mei: sigstk_probe.c,
altstack_scan.sh, p2.sh), the P2 rerun. Still deferred in s15.py (not this story's): MS_NOUSER, OPEN_TREE_CLOEXEC.
