# WL-DYNAMIC-KIT report

Premise (from the brief, AUDIT-PRE-SELFHOST item 6): the workload gate and per-call counts measure libniji STATIC
against glibc DYNAMIC, while users run libniji dynamically. Checked against what landed since the brief:
DYN-START-SPEED-2 moved dynamic start-up, but nothing switched the kit, so the premise holds.

## Commits

- 3d0374ff6 Add a libniji-dynamic workload kit beside the static one (WL_LINK, wl_dyncc.sh, wl_dynlink.py, wl_kits.sh/.py,
  per-call and count plumbing, the count-row cache directory fix, self-test sections 6-7, affected.rs rows).
- 513040f09 Link the workload kits from the pre-rename validation trees (ltree fallback, the measured tree's own driver).
- a5898fd57 Resolve libm.so.6's libc.so.6 to libniji's in the dynamic link (`-rpath-link`).
- db49df193 Find pre-rename validation trees by suffix, not by the former name (`xtask static` old-name check failed
  on 513040f09's fallback; the new one resolves to the same trees on the Skylake-SP build host, checked for `-m` and
  `-noload`).

## How the dynamic kit is linked, and why so

The same validation objects and link recipes as the static kit. `wl_dyncc.sh` runs the system `cc` with `-B`/`-L` at
a `lib/` directory laid out as an installed libniji (crt1.o/Scrt1.o = niji-crt1.o, crti.o, crtn.o, libc_nonshared.a,
libc.so and libm.so scripts) and `--dynamic-linker` = the products' ld.so. libniji's loader opens its pinned sonames
(libc.so.6, libm.so.6) only from `LD_LIBRARY_PATH` or `<prefix>/<libdir>` (crates/ld/src/search.rs), never RUNPATH.
`LD_LIBRARY_PATH` would also reach the host `sh`, `env` and `cat` in the workload commands. So the link reads
SONAME-less copies (absolute `DT_NEEDED`), and `wl_dynlink.py retarget` rewrites that string in place to the shipped
file, which keeps its SONAME. That lets libm.so.6's bare `NEEDED libc.so.6` match the loaded libc. The rewrite refuses
when another `.dynstr` reference points inside the string. The shipped `.so` files and ld.so run unchanged.
Probe-settled: gawk dynamic with libm runs (`sin`), and every workload's output matches glibc's (md5, column `out`).

## Where the dynamic column differs from the static one, and why

- **Start-up (dominant).** One dynamic start costs about 0.94M more instructions than a static one (`printf --version`:
  1.046M vs 0.105M; glibc 0.338M) and about +48 page faults (114 vs 62; glibc 74). That is the loader mapping libc.so.6,
  processing relocations, resolving symbols and IFUNCs, and the copy-on-write faults of relocated pages. Start-up
  share of the dynamic-minus-static instructions: printf-loop 100 %, du 94 %, grep-i-utf8 97 %, gzip-c 96 %,
  pipe-mix/tar-gz 84 %, ls-lR 58 %.
- **PLT.** +1 instruction per libc call (per-call builtin spec: fstatat 33 -> 34, malloc_free 115.2 -> 117.2 for two
  calls). Visible only as d/s 1.000-1.005 in long single-process workloads.
- **IFUNC and relocation.** Once per start, inside the start-up number. No per-call cost after binding (`-z now`
  per-call binaries; the kits use the default lazy binding of the toolchain, as the glibc kit does).

## Table (b2485a7f1 products, release profile; Skylake-SP build host, 48 threads, load ~30; 5 rounds, medians)

iu = instructions:u, pf = page faults, tc = task-clock (ADVISORY: unbooked, loaded build host; the timing verdict
belongs to the booked timing sweep). Ratios to glibc. "dyn@g-start/g" = dynamic with each start priced at glibc's
per-start instructions.

```
wl_kits: step prep-static 0 s
wl_kits: step prep-dynamic 1 s
wl_kits: step workloads 1133 s (5 rounds)
Per start (`<prog> --version`, median of 3): instructions:u / page faults
prog            glibc       static      dynamic     pf g     pf s     pf d
bash           470050       106180      1503185      102       87      149
cut            347090       109431      1036484       77       69      119
diff           497507       156486      1226561       81       79      126
du             385044       112918      1134039       77       70      119
gawk           642040       282972      1822262      112      115      170
grep           568489       208978      1280146       82       83      132
gzip           268080        64986      1062271       81       57      111
ls             378892       111909      1167365       80       70      122
make           441498       156166      1258169      106      116      161
md5sum         349501       118914      1046463       76       67      116
printf         338300       105434      1045734       74       62      114
sed            557448       198068      1225867       78       77      124
seq            343692       106501      1071202       77       68      118
sort           413096       129279      1265675       77       82      128
tar            871502       574130      2000617       86       78      129
tr             340549       106617      1048346       77       68      117
uniq           343399       108670      1052423       75       69      118
wc             359403       117869      1095992       76       71      119

workload          iu glibc     iu static        iu dyn    s/g    d/g    d/s    pf g    pf s    pf d pf s/g pf d/g tc s/g tc d/g starts start share dyn@g-start/g out
sort-C          1540493475    1463854493    1485923474  0.950  0.965  1.015   37517   37535   37588  1.000  1.002  0.874  0.883      1     5%    0.964 SAME
sort-utf8       5641114274    5279414283    5360725551  0.936  0.950  1.015   37546   37553   37605  1.000  1.002  0.924  0.949      1     1%    0.950 SAME
sort-enUS      16285846172    6737956130    6819281025  0.414  0.419  1.012   37538   37569   37619  1.001  1.002  0.639  0.646      1     1%    0.419 SAME
ls-l-enUS       1387632252     899896941     918077909  0.649  0.662  1.020    2674    3057    3568  1.143  1.334  0.929  0.941     10    58%    0.656 SAME
sort-n          5812479490    5717955866    5740458690  0.984  0.988  1.004   37529   37548   37600  1.001  1.002  1.024  1.025      1     5%    0.987 SAME
wc              1094700383    1094384300    1095362886  1.000  1.001  1.001     283     257     293  0.908  1.035  1.001  0.985      1   100%    1.000 SAME
md5sum           365782676     365716683     366646096  1.000  1.002  1.003     149     137     184  0.919  1.235  0.968  0.996      1   100%    1.000 SAME
cut-tr-uniq      926284635     825012277     832863685  0.891  0.899  1.010     492     464     617  0.943  1.254  0.884  0.949      3    36%    0.897 SAME
seq             2427055725    2257224825    2309233098  0.930  0.951  1.023     139     119     171  0.856  1.230  0.863  0.964      1     2%    0.951 SAME
printf-loop     5618244469    1668254158   20485457660  0.297  3.646 12.280 1498866  992970 2074171  0.662  1.384  0.633  1.672  20000   100%    1.128 SAME
ls-lR            905115614     826577780     844761942  0.913  0.933  1.022    2770    2836    3348  1.024  1.209  0.948  0.967     10    58%    0.925 SAME
du               101606879      78729681      89641606  0.775  0.882  1.139    3120    2856    3345  0.915  1.072  0.965  0.980     10    94%    0.809 SAME
sed-s          17836028509    9904166089    9919196562  0.555  0.556  1.002     221     240     286  1.086  1.294  0.519  0.528      1     7%    0.556 SAME
sed-utf8        5683350824    2453979207    2463447221  0.432  0.433  1.004     233     239     286  1.026  1.227  0.494  0.556      1    11%    0.433 SAME
bash-loop      20006307189   18834687609   19006489033  0.941  0.950  1.009     195     204     258  1.046  1.323  0.963  0.994      1     1%    0.950 SAME
grep-F            55232786      55865970      57171476  1.011  1.035  1.023     434     454     500  1.046  1.152  1.004  1.038      1    82%    1.022 SAME
grep-E           466014731     466187267     468466906  1.000  1.005  1.005     440     465     515  1.057  1.170  0.940  0.958      1    47%    1.004 SAME
grep-i-utf8      475852071     358046856     359156612  0.752  0.755  1.003     462     473     523  1.024  1.132  0.826  0.836      1    97%    0.753 SAME
gawk-sum        2153044209    1822886457    1832745897  0.847  0.851  1.005     251     254     303  1.012  1.207  0.875  0.874      1    16%    0.851 SAME
gawk-regex      3420062442    3312124125    3334560700  0.968  0.975  1.007     255     262     312  1.027  1.224  0.930  0.967      1     7%    0.975 SAME
gawk-utf8       3588557464    3456833659    3473694199  0.963  0.968  1.005     309     308     358  0.997  1.159  1.022  0.999      1     9%    0.968 SAME
gawk-printf     8660913422    5860801621    5895342522  0.677  0.681  1.006     251     249     301  0.992  1.199  0.591  0.602      1     4%    0.681 SAME
diff             784557989     784690344     786778546  1.000  1.003  1.003   52516   52564   52614  1.001  1.002  0.957  1.015      1    51%    1.002 SAME
tar-c             44746960      43156191      44983381  0.964  1.005  1.042     409     443     488  1.083  1.193  0.997  1.004      1    78%    0.980 SAME
tar-tv           124366097     119675585     121752528  0.962  0.979  1.017     237     240     289  1.013  1.219  0.653  0.682      1    69%    0.970 SAME
gzip-c          9411190034    9411040043    9412080844  1.000  1.000  1.000     275     256     307  0.931  1.116  1.060  1.052      1    96%    1.000 SAME
gzip-d          1043391867    1036392428    1042630523  0.993  0.999  1.006     218     206     258  0.945  1.183  0.959  1.000      1    16%    0.999 SAME
make-n           880958764     747843898     756310553  0.849  0.859  1.011    9404    7471    7518  0.794  0.799  0.798  0.898      1    13%    0.858 SAME
pipe-mix         532924050     439045519     443987347  0.824  0.833  1.011    1140    1140    1329  1.000  1.166  0.881  0.900      4    84%    0.827 SAME
tar-gz          5336630230    5334905868    5337775700  1.000  1.000  1.001     528     543     642  1.028  1.216  0.995  1.039      2    84%    1.000 SAME

geomean over 30 workloads (ratio to glibc): instructions:u static 0.8181 dynamic 0.9017 (dynamic at glibc's per-start cost 0.8621); page faults static 0.9780 dynamic 1.1512; task-clock (advisory) static 0.8540 dynamic 0.9086
static  instructions:u         > 1.10:  0  
static  instructions:u         > 1.00:  3  grep-F 1.011, grep-E 1.000, diff 1.000
static  page faults            > 1.10:  1  ls-l-enUS 1.143
static  page faults            > 1.00: 18  sort-C 1.000, sort-utf8 1.000, sort-enUS 1.001, ls-l-enUS 1.143, sort-n 1.001, ls-lR 1.024, sed-s 1.086, sed-utf8 1.026, bash-loop 1.046, grep-F 1.046, grep-E 1.057, grep-i-utf8 1.024, gawk-sum 1.012, gawk-regex 1.027, diff 1.001, tar-c 1.083, tar-tv 1.013, tar-gz 1.028
static  task-clock (advisory)  > 1.10:  0  
static  task-clock (advisory)  > 1.00:  5  sort-n 1.024, wc 1.001, grep-F 1.004, gawk-utf8 1.022, gzip-c 1.060
dynamic instructions:u         > 1.10:  1  printf-loop 3.646
dynamic instructions:u         > 1.00:  9  wc 1.001, md5sum 1.002, printf-loop 3.646, grep-F 1.035, grep-E 1.005, diff 1.003, tar-c 1.005, gzip-c 1.000, tar-gz 1.000
dynamic page faults            > 1.10: 22  ls-l-enUS 1.334, md5sum 1.235, cut-tr-uniq 1.254, seq 1.230, printf-loop 1.384, ls-lR 1.209, sed-s 1.294, sed-utf8 1.227, bash-loop 1.323, grep-F 1.152, grep-E 1.170, grep-i-utf8 1.132, gawk-sum 1.207, gawk-regex 1.224, gawk-utf8 1.159, gawk-printf 1.199, tar-c 1.193, tar-tv 1.219, gzip-c 1.116, gzip-d 1.183, pipe-mix 1.166, tar-gz 1.216
dynamic page faults            > 1.00: 29  sort-C 1.002, sort-utf8 1.002, sort-enUS 1.002, ls-l-enUS 1.334, sort-n 1.002, wc 1.035, md5sum 1.235, cut-tr-uniq 1.254, seq 1.230, printf-loop 1.384, ls-lR 1.209, du 1.072, sed-s 1.294, sed-utf8 1.227, bash-loop 1.323, grep-F 1.152, grep-E 1.170, grep-i-utf8 1.132, gawk-sum 1.207, gawk-regex 1.224, gawk-utf8 1.159, gawk-printf 1.199, diff 1.002, tar-c 1.193, tar-tv 1.219, gzip-c 1.116, gzip-d 1.183, pipe-mix 1.166, tar-gz 1.216
dynamic task-clock (advisory)  > 1.10:  1  printf-loop 1.672
dynamic task-clock (advisory)  > 1.00:  7  sort-n 1.025, printf-loop 1.672, grep-F 1.038, diff 1.015, tar-c 1.004, gzip-c 1.052, tar-gz 1.039
```

## Per-gate cost (the brief's "per-gate wall time")

No xtask or CI step runs the workload measurement. The per-gate step is the story's `wl_count` / `wl_percall` job
(and CI's instruction-limit check reads their rows). Measured on this head:
- products: one release `cargo xtask so`, shared by both kits (cold 14 min in the static count job here). The dynamic
  set as built today runs `cargo xtask so` a second time in the warm tree: 171.8 s, only to copy other files. A
  switch that replaces static removes that duplicate. Keeping both builds would add it to every gate.
- kit links: static all 18 programs 8 s; dynamic about the same (gawk relink 1 s; retarget is milliseconds).
- one pass of all 30 workloads (sum of medians, wall): static 40.9 s, dynamic 56.9 s, glibc 50.3 s. A `wl_count` run
  executes each column about 10 times (5 perf, md5, clone trace, 3 strace), so the libniji column moves from about
  410 s to about 570 s (+160 s, +39 %; printf-loop's 20000 dynamic starts are about half of the +16 s per pass). The
  glibc column is cached (now that release rows cache, see below).
- per-call: unchanged in practice (binaries link in under a second each).
- Replacing static with dynamic therefore costs about +2.7 min per full-workload count and saves the 171.8 s duplicate
  build, so the net is about +0 to +1 min. Adding dynamic beside static would cost +2.7 min +2.9 min = about +5.6 min.

## RULING NEEDED: which kit the INVIOLABLE rule binds (switch plan)

Recommendation: bind INVIOLABLE to the DYNAMIC kit on every gate, replacing the static step, never adding a second.
Move static to a periodic sweep.
1. Phase 1 (now, with this branch landed): run `wl_kits.sh <train head>` once (19 min job on the Skylake-SP host) to
   record the dynamic levels beside the static ones for one train. The default stays static.
2. Phase 2 (one small story): `WL_LINK` defaults to dynamic in wl_lock.sh (one line), and `wl_build_products` copies
   both product sets from a single build. Recorded workload and per-call levels are re-recorded dynamically, and
   INVIOLABLE binds dynamic instructions per workload. Rows that would fail at once only because of dynamic start-up
   bind to ALWAYS FORWARD at their recorded dynamic level (no worse than recorded) until DYN-START-SPEED reaches
   glibc's start-up, then to <= 1.00:
   - instructions: printf-loop (3.646; 1.128 at glibc's per-start cost);
   - small margins that move with start-up: grep-F 1.035, and wc, md5sum, grep-E, diff, tar-c, gzip-c, tar-gz within
     0.5 %;
   - page faults: 22 workloads above 1.10, about +45 faults per start.
3. Phase 3: a periodic static sweep, nightly on the build host and in every RC's checklist: `wl_kits.sh` (all three
   columns) and the static per-call spec. It covers what only static exercises: static start-up (crt1, static IFUNC
   and TLS set-up), `-static` users, libc.a's own layout. A static regression found there opens a story; it does not
   block a gate.
Phasing reason: phase 1 costs one job and no gate time. Phase 2's cost is neutral (above) and measures the path users
run. Without phase 2's carve-out, every gate would go red on start-up that DYN-START-SPEED already owns.

## INFORMATIONAL

- The workload kits did not link on the Zen 1 or the Skylake-SP build host before 513040f09: the object trees there
  are still named `liella-m` / `liella-noload` and the cached recipes call `~/liella-test/scripts/liella-cc`, which
  reads `LIELLA_PRODUCTS`. On the Skylake-SP host the 90f71362a kit holds only `seq`. Any wl_count of other workloads
  there failed at the link (the first static count of this run, `wl-dynamic-kit-static-count`, did, at sort).
- Release count rows were never cached: `wl_count_run.sh` made `~/wl/count/<sha>` and wrote `<sha>-release/...`, so
  every run re-measured the base column. Fixed in 3d0374ff6.
- Dynamic start-up per program is 1.04-2.00M instructions here. The brief's "~785k" was measured on a smaller program.
  The numbers above include program initialisation (`--version` path), the same in all three columns.

## Jobs

`wl-dynamic-kit-kits3` (the table), `wl-dynamic-kit-percall` (per-call static and dynamic), `wl-dynamic-kit-locktest`
(self-test), `wl-dynamic-kit-gate2` (direct tier on db49df193: 5 suites, 548 passed, 0 failed, including both
selected validation self-tests; peak 1.4G), `wl-dynamic-kit-lint2` (clippy + static on db49df193, clean). The
measurement jobs ran with the a5898fd57 tools; db49df193 changes only how the validation tree is found and resolves to
the same trees on that host. Superseded: `wl-dynamic-kit-static-count` (failed at the sort link, fixed in 513040f09),
`wl-dynamic-kit-kits` (same failure), `wl-dynamic-kit-kits2` (gawk dynamic link, fixed in a5898fd57),
`wl-dynamic-kit-gate` (green, but on a5898fd57), `wl-dynamic-kit-lint` (static old-name red on a5898fd57, fixed in
db49df193).
