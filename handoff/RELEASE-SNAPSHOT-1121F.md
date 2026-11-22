WIP (leg 1 of RELEASE-SNAPSHOT-1121F; refresh): validation of 2c64fde8a in progress. Done: smoke both, gate, start-up
counts both classes, gaimass, relocation attribution. Running/pending: debs-1, then the .deb validations (mei), plants,
pkgdiff, stage, install (keke).

# RELEASE-SNAPSHOT-1121F: snapshot validation of 2c64fde8a (#13 rest, getaddrinfo_a, ICONV-REST, DL-RUNPATH-4)

**State.** Head `2c64fde8a98df721ad7ee082d8ba8d7d073172d9` (origin/next at the start, 00:38 UTC; "Regenerate
train-owned views"; commit time 2026-11-21T23:47:05Z, epoch 1795304825). Branch `task/release-snapshot-1121f` = head +
handoff/ only. Role **keke `~/w/n3` (Zen 1)**; mei = Skylake-SP (no role there: mei runs test jobs only, on keke's
products and .debs, sha256 checked). 21.6 = 97bf3542f (RELEASE-SNAPSHOT-1121E). Serial: commit date 2026-11-21 =
the day's **seventh** snapshot: `NIJI_DEB_SERIAL=7`, `2.39+niji20261121.7-1` (dpkg --compare-versions: still to run).
ICONV-CJK-STARTUP is **not** in the head (no merge since 21.6).

**Landed since 21.6** (`git log --first-parent 97bf3542f..2c64fde8a`): release-snapshot-1121e (83f91cb05),
compat19-ldso-cost-2 (bbcc073c5), view regen (4f32aa946), wl-count-honest-cheap (228dcf3b6), view regen (0299fd6e1),
dl-runpath-4 (69e37399f), startup-structure-review (b1cd6f3e1), valgrind-string-redirect (e64dbbc30),
affected-coverage (ec977d861), view regen (e72318002), gai-mass-loss (6094e53e9), iconv-rest (ed6d48230), view regen
(2c64fde8a).

**Scripts** (mei and keke `~/work/RELEASE-SNAPSHOT-1121F/`, identical; local copies in this run's scratch): 1121E's
set by copy.sh + subst.sed (head, epoch, serial 7, labels, p216 = 21.6 / p217 = head, role `w/s1` -> `w/n3`), tree.tgz
= `git archive 2c64fde8a` (8924 files = ls-tree; sha checked on both hosts). Plants file
handoff/RELEASE-SNAPSHOT-1121F-plants.txt (1121E's six; each pattern present once in the head). New this run:
run-keke.sh (keke sequencing: smoke-keke-1 build -> products.tgz -> gate-1 -> debs-1 -> debs.sha -> plants-1 &
pkgdiff-1 -> chain-keke.sh (stage-1 && install-1); start-up counts beside), run-mei.sh (mei sequencing; waits for
products.sha, then debs.sha), run-gai.sh + gaimass.sh (keke), rootrun.sh (VALGRIND-STRING-REDIRECT's acc.sh root
recipe, probe dir as a parameter), **y13s.sh** (#13 rest; probe y13s/ = that story's vs/ unchanged), **drp.sh** (probe
drp/: DL-RUNPATH-4 caller DT_RUNPATH $ORIGIN, plain and setgid), **y6r.sh** (ICONV-REST six sets, made by mky6r.py from
y6c; every row judged; note: y6r/post.sh line 7 keeps y6c's stale "not judged" comment, the code judges every row).

## Results so far

| check | job | result |
|---|---|---|
| smoke | keke smoke-keke-1 rc 0 (build), mei smoke-mei-1 rc 0 | `SMOKE: RESULT bad=0` both |
| gate | keke gate-1 rc 0 | clippy all-features, xtask clippy, static, records check, affected: all RC 0, `GATE bad=0` |
| getaddrinfo_a | keke gaimass-1 rc 0 | head's dns_products, 3 passed, both sets; gaimass ok=25000 other=0, signals 5000/5000 |

**gaimass** (head's test; one build; probe linked against each set's libc.a; load average 21-33 on 60 vCPUs, the job
in a 4-CPU slot): p217 and p216 both `done: ok=25000 other=0 answer 192.0.2.10 only=25000; signals: 5000`, cache on and
off, as the reference. Submission CPU: head 0.022 / 0.023 s (on/off), 21.6 0.175 / 0.182 s; reference 0.011 s.
Note line for the notes: "21.6's 'loses up to ~2% under heavy load' was the test's fixed 65 s deadline running out on a
loaded host, not lost requests: run with the load-independent wait, 21.6 and this snapshot answer all 25,000 requests
and deliver all 5,000 signals." (gaimass.sh printed empty sections: it looked for resolv-*.stdout; the files are
resolv_diff-*.stdout, read from the saved copies keke ~/work/RELEASE-SNAPSHOT-1121F/gai-p21{6,7}/test-oracles/.)

**Start-up** (execcount, ASLR off, median of 101; user instructions / faults; startup-*-1, kstart-*-1, rstart-*-1):

| program | class | glibc 2.39 | 21.6 | 2c64fde8a | delta |
|---|---|---|---|---|---|
| empty main | Skylake-SP | 107,752 / 49 | 742,940 / 56 | 726,461 / 57 | -16,479 / +1 |
| 3 libraries | Skylake-SP | 143,094 / 65 | 985,927 / 71 | 966,077 / 72 | -19,850 / +1 |
| clock_gettime | Skylake-SP | 109,862 / 50 | 765,032 / 59 | 748,574 / 60 | -16,458 / +1 |
| empty main | Zen 1 | 101,533 / 48 | 742,708 / 56 | 726,222 / 57 | -16,486 / +1 |
| 3 libraries | Zen 1 | 136,734 / 64 | 985,680 / 71 | 965,824 / 72 | -19,856 / +1 |
| clock_gettime | Zen 1 | 103,672 / 50 | 764,828 / 60 | 748,365 / 61 | -16,463 / +1 |

Skylake-SP kernel instructions / syscalls (kstart, rstart): pie0 glibc 690,382 / 46, 21.6 742,353 / 29, head 749,611 /
29; pie3 981,782 / 72, 1,142,407 / 59, 1,150,405 / 59; res0 user 169,020 / 921,834 / 889,150, faults 55 / 61 / 62,
kernel 823,115 / 938,070 / 944,007, syscalls 58 / 42 / 42. Zen 1 kernel/res0 rows: in keke counts (kstart-keke-1,
rstart-keke-1 rc 0), still to tabulate (`grep -h '^KCOUNT' ~/logs/release-snapshot-1121f-{kstart,rstart}-keke-1.log`).

**Finding 1 (start-up, INFORMATIONAL):** COMPAT19-LDSO-COST-2's gain shows (-16.5k empty main, -19.9k three
libraries, both classes) but ~0.7k short of its stated -17.2k / -20.5k, and every program has **+1 page fault**.
libc.so.6: RELATIVE relocations 1,573 -> 1,606 (+33; relsites.py: 1,861 -> 1,894 sites, relocated pages 13 -> 14):
8 in `niji::iconv::cjk_tables::CP949` (ICONV-REST's new descriptor), 25 unnamed; RW segment file size +4,096 B
(0x23730 -> 0x24730); third read-only segment +123,264 B. ~20 instructions per relocation x 33 = ~660 (estimate) and
one more relocated page = the fault. Same mechanism as 1121E's finding 1; ICONV-CJK-STARTUP (queued) is the fix-forward.

## Exact next steps (leg 2)

1. `~/bin/jobwait release-snapshot-1121f-debs-1` on keke; record `check: PASS`, check-symbols, dbg lines.
2. Relay keke's .debs to mei: wait for keke `~/work/RELEASE-SNAPSHOT-1121F/debs.sha`; stream `tar cf - debs/system
   debs/coexist` (keke, in the work dir) into mei's work dir (`tar xf -`), then copy debs.sha LAST (mei's run-mei.sh
   starts the .deb jobs when debs.sha appears and checks it). Check debs/ for other files the validations read.
3. Wait (one jobwait call per host): mei roots-1 lint-1 yohane-1 many-1 dyn22-1 y10-1 y19-1 y26-1 y26t-1 y23-1 c19-1
   y25-1 y24-1 y28-1 y6-1 y25s-1 y21-1 yres-1 y23p-1 dlc-1 y27-1 y13-1 y14-1 y6c-1 y14r-1 eetwet-1 y13s-1
   y13s-ref216-1 drp-1 drp-ref216-1 y6r-1 y6r-ref216-1 sig32-1 y20-1 dpo600-1; keke plants-1 pkgdiff-1 stage-1
   install-1 (labels prefixed release-snapshot-1121f-).
4. Compare with 1121E (cmp4.sh in the work dir, masked verdict lines against 1121E's logs). Expected differences:
   y10 (DL-RUNPATH row now PASS), y6 (WINDOWS-1252/1251 rows now pass). y13s/drp/y6r: new; their -ref216 runs show
   21.6. Leak record stack: from y13-1 (cxx.cc).
5. dpkg --compare-versions 2.39+niji20261121.7-1 gt 2.39+niji20261121.6-1; niji6 version gt 21.6's.
6. Write the full report here (first line COMPLETE/blocking findings), handoff/reports/RELEASE-SNAPSHOT-1121F.json,
   push, sync, `cargo xtask precheck --jobs <labels> --head <sha>` on keke n3, then ops handback-check.
