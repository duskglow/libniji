# HEADERS-STAGE2-GAPS-K: handoff (run 1, leg 4: done; full report below)

Branch task/headers-stage2-gaps-k, cut from origin/next 16e03637e, no merge. Code head eb9361a0e plus the regenerated
libc-include (next commit; later commits: handoff only). Role shiki:~/w/sh1 (Skylake-SP class, no clang: header_check's clang P5 is skipped there); clang-19 and
the llvm-19 source are on keke. Job scripts: shiki ~/work/headers-stage2-gaps-k/ (g6.sh gate chain, r1.sh/j1/j2
probes, pk1.sh system debs); keke ~/work/headers-stage2-gaps-k/ (pk2.sh llvm-19 unit, pk3.sh addstock + pk2, tp.sh).

## What the branch does
1. `<netipx/ipx.h>`: SIOCAIPXITFCRT, SIOCAIPXPRISLT, SIOCIPXCFGDATA, SIOCIPXNCPCONN (SIOCPROTOPRIVATE+0..3,
   parenthesised) and sipx_special, per configuration (headers/netipx/ipx.h.in, visibility.tsv).
2. `spec/abi/macro-spellings.tsv` (generator input, 109 rows: name, the kernel's spelling, its source header):
   the generator emits each name in that form (PPPIOC*, BLK*, MTIO*, TIOCSER_TEMT, ...), outright and once (the
   emit-once guard: the first header included defines it). form.py ENFORCED for the PPP/BLK/MTIO requests: FORM-DIFF 0
   for ioctl requests (k5-form.diff).
3. `<termios.h>` + `<sys/ioctl.h>` (and the other PAIRS in 60-stage2-gaps-k.py): no redefinition, C and C++.
   TTYDEF_SPEED dropped from headers/dup-listed.tsv (now defined once).
4. (coordinator note, #8803) `<protocols/timed.h>` no longer includes `<sys/socket.h>` (the reference's shows no
   socket name; struct osockaddr came with it since GAPS-I). No other header source includes `<sys/socket.h>` where the
   reference's header lacks `_SYS_SOCKET_H`.
Tests: header_checks/s13.d/60-stage2-gaps-k.py (142 units: pairs, kernel spelling, netipx requests, TTYDEF_SPEED,
timed.h own socket names); features.py forms no longer skips spelled kernel-spelled names.

## Proof (final head with libc-include regenerated, unless noted)
* gate7 (shiki; gate6 on eb9361a0e the same, green): clippy, static, headers-gen, dup.py 0, header_check GAPS-K 0/142, features.py forms 0, header_check
  protocols/timed.h 0/135, cargo test -p xtask, probes k6 + form.py. FULL: the train's (xtask and generator inputs).
* plants7 (shiki): 10 plants, 10 CAUGHT, no WARN (handoff/HEADERS-STAGE2-GAPS-K-plants.txt).
* keke llvm19-2 (pk3.sh on debs3: eb9361a0e's system libc6-dev, sha256 2f8c2bd8...): RELEASE-SNAPSHOT-1127B's
  addstock.sh against RELEASE-SNAPSHOT-1126C's libc6-dev: user-namespace names not in stock only in sched.h (28) and
  spawn.h (27), the recorded sched_attr family; protocols/timed.h no longer listed (1127B's run: osockaddr). Then
  llvm-19 compiler-rt 19.1.7 sanitizer_platform_limits_posix.cpp: clang-19 stock=0 niji=0, g++ stock=0 niji=0.
* Probes (k5/k6): p2 differs from stock only in asm/termbits.h+termios.h (stock warns of 111 redefinitions, libniji
  none); p1 only TTYDEF_{C,I,L,O}FLAG with `<sys/ioctl.h>` alone (stock cannot evaluate them, libniji can): both on
  base, not this story's names.

## INFORMATIONAL
* features.py run standalone compiles its units with `-nostdinc` and no kernel UAPI dir; base `<sched.h>` (GNU) now
  includes `<linux/sched/types.h>` (sched_attr, ruling #8766), so standalone `FEATURES pkg npth rwlock gnu` fails
  ("linux/sched/types.h: No such file"). Inside header_check (line ~497, kernel-uapi dir) it passes. Fix: give
  `_compile` the same kernel dir. Not this story's code; the gate runs features.py `--only "FEATURES forms"`.
* The GAPS-K kernel-spelling unit respells with the table's own text: it proves the generator emits the table's
  spelling; the value is P1's (plant k-ppp-nr names P1 net/if_ppp.h).
* spec/abi/kernel-spelled.tsv not regenerated (the spellings override makes it unnecessary).

## Superseded jobs
-gate (red: dup), -plants (baseline red), -gate2/-probe3/-plants2 (6a74db8ca), -debs (killed), -gate3 (red: stale
TTYDEF_SPEED row in dup-listed.tsv), -plants3 (baseline red, same), -debs2/keke -llvm19 (b1b53ab1a, headers before item
4), -gate4 (standalone features.py, see INFORMATIONAL), -plants4 (k-ppp-nr survived: command too narrow), -gate5 and
-plants5 (87f6d3e5a, before item 4), -gate6 and -plants6 (eb9361a0e: the generated libc-include was
not committed, so they tested regenerated headers the head lacked; -gate6 also had no tested-content record),
-regen (the regeneration itself).

## Next step
None for this story: land through the train. Plan rows: none new.
