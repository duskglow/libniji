# HEADERS-STAGE2-GAPS-K: handoff (run 1, end of leg 3: generator fixed after the gate; proof jobs running)

Branch task/headers-stage2-gaps-k, cut from origin/next 16e03637e, no merge. Code head b1b53ab1a. Role shiki:~/w/sh1
(Skylake-SP class, no clang: header_check's clang P5 is skipped there). clang-19 and the llvm-19 source are on keke.
Job scripts and outputs: shiki ~/work/headers-stage2-gaps-k/ (r1.sh TAG = headers-gen + j1 (p1 + form) + j2 (p2 pairs);
pk1.sh COMMIT EPOCH = system-flavour debs into ~/work/headers-stage2-gaps-k/debs/system). keke
~/work/headers-stage2-gaps-k/pk2.sh (installed, checked): extracts ~/work/headers-stage2-gaps-k/debs/libc6-dev_*.deb
and compiles llvm-19's sanitizer_platform_limits_posix.cpp with clang-19 and g++, stock and libniji (J's pf1.sh step 2).

## Done
* Leg 1-2 (see commits): `spec/abi/macro-spellings.tsv` (109 rows; generator `spellings`), `<netipx/ipx.h>` requests
  and sipx_special, header_checks/s13.d/60-stage2-gaps-k.py, form.py ENFORCED PPP/BLK requests.
* Leg 3: 3cd48a055 parenthesis check fixed (+0 could not see missing parentheses) and plants file. Gate run 1
  (label -gate, head 1756f2310+) RED: dup.py `TIOCSER_TEMT` defined outright in 5 headers, `SYNC_FILE_RANGE_WRITE_AND_WAIT`
  in 3; the `net/if_ppp.h`+`linux/ppp-ioctl.h` pair fails on struct npioctl (stock too). 6a74db8ca put kernel-sourced
  spelled names under #ifndef and dropped that pair: gate2 green, but probes k3 showed p1 `earlier-def-kept` DIFF for
  the PPP requests (stock replaces a program's earlier definition) and p2 SC_MASK. b1b53ab1a: spelled names outright
  again (redefined() override kept) AND under the emit-once guard (`item()`: `|| self.t.spellings.contains_key(name)`),
  so the first header included defines each; features.py forms check no longer skips spelled kernel-spelled names.

## Jobs running at the end of leg 3 (shiki; all on b1b53ab1a)
* `headers-stage2-gaps-k-gate3` (test): clippy, static, headers-gen, dup.py, header_check --only GAPS-K, features.py,
  cargo test -p xtask (full header_check inside), form.py, then r1.sh k4 (probes; outputs k4-*).
* `headers-stage2-gaps-k-plants3` (plants; 9 plants; dir /mnt/zram/claude/headers-stage2-gaps-k/plants-3).
* `headers-stage2-gaps-k-debs2` (test): waits for gate3, then pk1.sh b1b53ab1a (system debs).
SUPERSEDED: -gate (red, fixed), -plants (baseline red, same cause), -gate2/-probe3/-plants2 (6a74db8ca; plants2 refused:
k-spelling-off pattern stale), -debs (6a74db8ca, killed).

## Next step (leg 4)
1. `~/bin/jobwait headers-stage2-gaps-k-gate3 headers-stage2-gaps-k-plants3` on shiki. Probe check: `W=~/work/headers-stage2-gaps-k;
   diff $W/k4-p2-stock.out $W/k4-p2-niji.out` must show only `asm/termbits.h+termios.h` (pre-existing, INFORMATIONAL);
   `diff $W/k4-stock.out $W/k4-niji.out` only TTYDEF_IFLAG/OFLAG/LFLAG/CFLAG with <sys/ioctl.h> (pre-existing, 28 lines,
   INFORMATIONAL). If features.py forms fails on a spelled name with a kernel header in front: look at that name only.
2. Plants: every one CAUGHT (k-*). Copy `/mnt/zram/claude/headers-stage2-gaps-k/plants-3/results.txt` over
   handoff/HEADERS-STAGE2-GAPS-K-plants.txt and commit (docs/plants.md).
3. After debs2: copy `~/work/headers-stage2-gaps-k/debs/system/libc6-dev_*.deb` shiki -> local scratch -> keke
   `~/work/headers-stage2-gaps-k/debs/`, then on keke `setsid nohup ~/bin/jobrun headers-stage2-gaps-k-llvm19 test --
   sh ~/work/headers-stage2-gaps-k/pk2.sh` and jobwait it: RESULT lines (niji=0 for clang and gxx, or the first stop).
4. precheck `--jobs headers-stage2-gaps-k-gate3,headers-stage2-gaps-k-plants3,keke:headers-stage2-gaps-k-llvm19`
   on the synced final head; report json handoff/reports/HEADERS-STAGE2-GAPS-K.json (FULL: the train's: xtask and
   generator inputs); handback-check. INFORMATIONAL: asm/termbits.h+termios.h; TTYDEF_*FLAG with <sys/ioctl.h> alone;
   kernel-spelled.tsv not regenerated (the override makes it unnecessary).
