# LIBC-SELF-BIND: handoff (end of LIBC-SELF-BIND-2, the merge continuation)

State: branch task/libc-self-bind-2 (continues task/libc-self-bind), code head 06e90fa37 (handoff/report commit
follows). Role mei:~/w/s3. Decision D-27.76 (records/decisions/D-27/D-27.76.toml). Rulings in force: #7401 (the
"keep" variant), #7460 (stream allocation: follow-up STDIO-CALLER-ALLOC, held until this lands).

## Done
* Legs 1-2 (task/libc-self-bind): xtask/src/so.rs links libc.so.6 with `--dynamic-list` (the malloc family,
  `LIBC_PREEMPTIBLE` = the static archive's `INTERPOSABLE`) and `--dynamic-list-data`; build check
  `check_self_binding` + unit test; conformance x_self_bind (+ helpers/sbind_lib.c), x_self_bind_stdio (PENDING-NIJI).
* This run: merged origin/next c9f54d512 (LD-LOOKUP-COST; 7d8a77edb; so.rs merged cleanly, the only conflict was
  `DYN_START_INSTR`). x_self_bind_stdio's PENDING-NIJI and D-27.76 now name STDIO-CALLER-ALLOC (#7460; a2326b06e).
  `DYN_START_INSTR` set from the merged products measured on BOTH CPU classes (06e90fa37): Skylake-SP
  [367_400, 338_700], Zen 1 [367_300, 338_500] (each host's level + 2%; next had [719_100, 679_100]).

## Method (the same products on both classes)
Release products of next c9f54d512, the pre-merge branch c82357afe and the merged head, plus merged fast, all built on
mei (Skylake-SP) and copied to keke (Zen 1). Scripts: mei and keke ~/work/libc-self-bind-2/{build.sh,count.sh,kit/};
logs ~/logs/libc-self-bind-2-{measure,count-skx} (mei), ~/logs/libc-self-bind-2-count-zen (keke).
"kit" = the release snapshots' start-up kit (execcount user / kernel instructions and faults, median of 101, ASLR
off, LD_LIBRARY_PATH only; syscalls = lines of one strace -f run). "test" = hot_text_products' own method
(staging, link, oracle/exec_instr.c, median of 21); on mei it equals the test's printed value exactly (332,012 /
360,169). Check of the cross-host products: the pre-merge release products read 339,504 on keke in the test method,
the first run's figure exactly, and 330,068 in the kit (first run 330,073).

## Start-up rows, release products (user instructions / user page faults / kernel instructions / syscalls)
Kernel counts move by up to ~50k run to run; syscalls are exact.

| row | class | glibc 2.39 | next (LD-LOOKUP-COST) | pre-merge branch | merged |
|---|---|---|---|---|---|
| empty main (pie0) | Skylake-SP | 107,175 / 49 / 686,656 / 46 | 657,748 / 57 / 741,369 / 29 | 330,277 / 56 / 736,132 / 29 | 323,388 / 56 / 735,332 / 29 |
| 3 libraries (pie3) | Skylake-SP | 142,426 / 65 / 983,612 / 72 | 894,158 / 72 / 1,148,295 / 59 | 493,438 / 71 / 1,161,038 / 59 | 485,704 / 71 / 1,142,472 / 59 |
| clock_gettime | Skylake-SP | 109,285 / 50 / 689,417 / 46 | 672,081 / 60 / 771,264 / 29 | 352,390 / 59 / 764,947 / 29 | 337,721 / 59 / 765,310 / 29 |
| res0 (libresolv) | Skylake-SP | 168,614 / 55 / 817,973 / 58 | 810,823 / 62 / 935,214 / 42 | 466,578 / 61 / 931,568 / 42 | 451,115 / 61 / 930,719 / 42 |
| empty main (pie0) | Zen 1 | 100,956 / 48 / 700,630 / 42 | 657,540 / 57 / 762,392 / 28 | 330,068 / 56 / 756,494 / 28 | 323,179 / 56 / 755,993 / 28 |
| 3 libraries (pie3) | Zen 1 | 136,192 / 64 / 997,238 / 68 | 893,958 / 72 / 1,174,688 / 58 | 493,237 / 71 / 1,166,828 / 58 | 485,503 / 71 / 1,169,503 / 58 |
| clock_gettime | Zen 1 | 103,095 / 50 / 707,995 / 42 | 671,901 / 61 / 796,068 / 28 | 352,210 / 60 / 790,929 / 28 | 337,541 / 60 / 790,576 / 28 |
| res0 (libresolv) | Zen 1 | 162,210 / 54 / 842,024 / 54 | 810,432 / 62 / 958,612 / 40 | 466,186 / 61 / 951,921 / 40 | 450,723 / 61 / 952,120 / 40 |

res0 - pie0 (user): next +153,075 / +152,892, merged +127,727 / +127,544 (Skylake-SP / Zen 1).
First run (Zen 1, kit, base = snapshot 1121E): empty main 742,167 -> 330,073, 3 libraries 984,891 -> 493,234.
Fast merged (kit, user): pie0 351,446 / 351,233, pie3 528,891 / 528,679 (Skylake-SP / Zen 1).
Test method, empty main: next 666,372 / 666,210; pre-merge 339,666 / 339,504; merged release 332,012 / 331,850,
merged fast 360,169 / 360,004 (Skylake-SP / Zen 1). Symbolic relocations: next 288, merged 58 fast / 55 release.
The whole hot_text_products file on mei, merged, both profiles: 32 passed, only `DYN_START_INSTR` red (10% under the
old bound), before the bound commit. Not run on keke (no role there): the Zen 1 bound is this test's method by
script, not the test binary.

## Jobs (mei, all on 06e90fa37)
libc-self-bind-2-gate (direct tier: the xtask bounded set, 3 targets + 144 conformance programs, not FULL), then
libc-self-bind-2-xtest, then libc-self-bind-2-fullprof (sequenced by ~/work/libc-self-bind-2/final-chain.sh);
Gate: green (affected ok, conformance 144 passed, 1 of them pending on libniji: x_self_bind_stdio; peak 2.4G).
libc-self-bind-2-plants-2: 5 plants, 5 CAUGHT, no WARN (handoff/LIBC-SELF-BIND-2-plants.txt; input
handoff/LIBC-SELF-BIND-plants.txt). SUPERSEDED libc-self-bind-2-plants: 3 CAUGHT + 2 WARN-NO-MSG: malloc-bound and
no-dynamic-list were caught by x_self_bind as intended, but their `msg:` regexes were wrong (the runner prints
`conformance FAILED: x_self_bind`; and `--dynamic-list-data` alone binds the malloc family too, so the build check
never fires there); regexes corrected, nothing in the code changed.
Measurements: libc-self-bind-2-count-skx (mei), keke:libc-self-bind-2-count-zen (rc 0). SUPERSEDED
libc-self-bind-2-measure: rc 1 by design (built the measured products on 7d8a77edb and ran hot_text_products before
the bound commit; its only red was DYN_START_INSTR 10% under next's bound). xtask test and the full-profile pair:
green; the release empty main on the final head reads 332,012 (bound 338,700), as measured.
Step 0: libc-self-bind-2-lint0 (clippy + static on the inherited head c82357afe: green).

## Open / next step
* Nothing open on this story beyond its jobs. STDIO-CALLER-ALLOC (#7460) follows once it lands.
* Not done: a diff of layout-bins.tsv against the merge base (no fast products of next were built); start-up faults
  read 1 under next in every kit row and the start-up layout tests pass.
