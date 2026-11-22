# SL-HEADERS-S18 handoff: slice 18, fortify (`_FORTIFY_SOURCE` 1..3)

**State:** run 2 (story SL-HEADERS-S18-2), branch task/sl-headers-s18-2 (on task/sl-headers-s18, from origin/next
f15d8264f), role keke:~/w/n4. Code head 2c9f4b633. Proofs on that head: gate keke:sl-headers-s18-2-gate3, plants
keke:sl-headers-s18-2-plants3 (5 plants), P7 rerun keke:sl-headers-s18-2-p7c (zlib, xz-utils); the hand-back
gives their verdicts.

**Rulings in force**
* #7599 (coordinator, 11-22): slice 18 owns the fortify rows of P3 and every `*_chk` export reached. The 55
  other -O2 rows (27 names, listed below) are HEADERS-O2-INLINES. `*_chk` prototype rows by probe are table data.
* Slice contract (plan/headers.md 4.1): slice files only (`xtask/src/headers/slices/s18.rs`, `headers/fortify.tsv`,
  template `@@fortify` lines). Shared tooling edits are their own commits (below).

**Done (run 1)** tables (`spec/abi/chk-prototypes.tsv`, `headers/fortify.tsv`), the `@@fortify` marker, wrappers in
10 templates, the overflow test, 2 plants; Devuan excalibur dpkg-buildflags recorded: CPPFLAGS `-Wdate-time
-D_FORTIFY_SOURCE=2`, CFLAGS `-g -O2 -Werror=implicit-function-declaration -ffile-prefix-map=/=.
-fstack-protector-strong -fstack-clash-protection -Wformat -Werror=format-security -fcf-protection`.

**Done (run 2)**
* 5d620e988 `spec/tools/callsite_probe.py`: a value argument whose spelled type a header does not declare
  (`__off64_t`) is read as a run-time `long` before the constant `0` fallback. Reference run of the whole probe:
  callsites.tsv and noexcept.tsv byte-identical. P3 on libniji's 13 fortified headers: 56 lines -> 55, fortify
  lines 1 -> 0 (pread64 -> `__pread64_chk` row equal); the 55 are exactly the 27 HEADERS-O2-INLINES names.
* 56c69cdc3 `validation/smoke/programs.sh`: zlib's build takes the caller's CFLAGS (default `-O2 -g` kept) and
  CPPFLAGS; the configure builds already read them from the environment.
* 276de0ca8 open family: P7 showed xz referencing `__open_2` with the reference headers only. Spec 11.1 names
  `__open_2`/`__openat_2` (fortify); callsites.tsv has their rows. New fortify.tsv form `mode` for open, open64,
  openat, openat64; `@@fortify` in fcntl.h; abi_extract's fortify pass keeps `__*_2` names (5 new rows, incl.
  `__mq_open_2`). Behaviour by black-box probe (relocations of own objects, compiler exit status; levels 1-3 alike):
  no mode + run-time flags -> `_2` entry (64-bit one under FOB64); constant flags needing no mode, or a mode ->
  plain; constant O_CREAT / all O_TMPFILE bits without mode, or 2 variable args -> compile error (GCC `error`
  attribute). Test `an_open_without_its_mode_calls_the_checked_entry` (12 shapes); P3 for fcntl.h clean.
* 2c9f4b633: "constant flags" means the flags argument itself (`__builtin_constant_p (flags)`): xz's run-time flags
  built from constants let GCC fold the mode condition, and the first version made the plain call there (P7 rerun
  p7b on 276de0ca8 still showed `__open_2` on the reference side only). Reference probe of
  `open (p, d ? O_RDONLY | O_NONBLOCK : O_RDONLY)`: `__open_2`. Test shape and plant `flags-fold` added.
* SUPERSEDED jobs: sl-headers-s18-2-gate / -plants (56c69cdc3, before the open family), -gate2 / -plants2 / -p7b
  (276de0ca8, before the constness fix), -regen (its `| tail` hid the test lines; regen2/regen3 rerun them).
* P7 (job sl-headers-s18-2-p7, on 56c69cdc3's products, Devuan flags both sides, `--niji-headers`): zlib and
  xz-utils build and pass on both sides; undef sets: zlib same, xz `__open_2` (fixed by 276de0ca8) and `putchar`
  (HEADERS-O2-INLINES). sqlite3, grep, sed, diffutils, findutils, coreutils, gawk, bash do NOT build against
  libniji's headers, none for a fortify reason: `sys/wait.h` and `sys/ioctl.h` are not generated (slice 7/11
  lists them), `__fwriting`/`__fpurge` not declared in `stdio_ext.h` (slice 17), grep's gnulib localename uses
  `_NL_ITEM`/`struct __locale_struct` (reference-internal: an Aqours patch case); Devuan's
  `-Werror=implicit-function-declaration` makes the undeclared names errors. Their P7 waits for those slices.

**Open / not slice 18's**
* `mq_open` -> `__mq_open_2`: the row exists in chk-prototypes.tsv; mqueue.h is not generated. Whoever generates it
  adds a fortify.tsv `mode` row (`$0, $1`, flags `$1`) and `@@fortify` to its template.
* HEADERS-O2-INLINES: stdio.h feof_unlocked ferror_unlocked fgetc_unlocked fputc_unlocked getc_unlocked getchar
  getchar_unlocked getline putc_unlocked putchar putchar_unlocked; stdlib.h abs atof atoi atol atoll bsearch div;
  string.h basename memrchr rawmemchr strcasestr strchrnul; strings.h index rindex; wchar.h mbrlen.

**Next step:** if the gate and plants named in the hand-back are green, the slice is ready for the train. Rerun
P7 for the rest of the corpus when sys/wait.h, sys/ioctl.h and stdio_ext.h's GNU functions land (job script shape:
export the Devuan CPPFLAGS/CFLAGS above, `smoke.sh --niji-headers`, then `undef-compare.sh`).
