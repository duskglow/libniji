# HEADERS-TABLES-COMPLETE: handoff and report (final, leg 4, 2026-11-19)

State: branch `task/headers-tables-complete` from origin/next 2b5604f9c, role mei:~/w/s2. Exceptions in
`headers/rows-exceptions.tsv`: 99 -> 74 (leg 1) -> 42 (leg 2) -> 15 (leg 3) -> **3** (leg 4): va_list in C++
(PERMANENT, ruling #6721), NULL and TD_SIGNO_MASK (RULING NEEDED below). Plants file
`handoff/HEADERS-TABLES-COMPLETE-plants.txt` (21 plants: 20 caught + 1 declared equivalent).

## Commits (one per group, as the brief asks)

* 570b86928 groups 1, 3, 6, 7: enumerators.tsv (`@@enum` is data), layouts at every depth, recorded bit positions,
  va_list PERMANENT. 67c69b1a0 plants. 2ca7392b5 leg-1 findings (union sigval out of strict sys/pidfd.h).
* 7f0a5c5ea group 2: `headers/providers.tsv` (program / debugger / none; ruling #6715: bindresvport6, getumask
  declared as the reference does).
* 2894a9fb4 group 4: transparent-union layouts rows and the C++ pointer form (`__SOCKADDR_ARG`).
* 955a77192 group 5a: macros.tsv pointer types (value emitter spells the cast), FNM_NOSYS, DLFO_EH_SEGMENT_TYPE.
* 182656969 the abi-types static check accepts the pointer rows (leg 3's test job was red on it: 17 unknown types).
* 272428ddb group 5b: `abi_extract.py --prototypes-missing` (gets from c89, bsd_signal from xopen500, non-fortify
  configurations only, existing row order kept); providers.tsv provider `generic` for the 10 type-generic macros'
  C++ function forms (iscanonical iseqsig issignaling iszero fpclassify isfinite isgreater isnormal isunordered
  signbit): the generator declares nothing for them and refuses a generic row a prototype declares or no header shows
  as both macro and function.
* d03e77f0b plants for the new code. 0825a267f callsites.tsv / noexcept.tsv rows for gets and bsd_signal
  (`callsite_probe.py --names gets,bsd_signal`, merged).

## Probe-settled (reference, black-box)

* C++ type-generic forms (g++ -std=c++17, <math.h>): `&N` does not compile for all 10 (overload sets), calls with
  float/double/long double do, none is a macro; the six C99 classifiers are `std::N`; iscanonical, iseqsig,
  issignaling, iszero have no `std::` form (the header's own overloads; calls reference `__iseqsig*`,
  `__issignaling*`, `__iscanonicall`).
* NULL and TD_SIGNO_MASK: both object-like, value 0, `void *` in C, `long` in C++ (`__null`).

## Generated output

No change in `libc-include/` or `crates/abi` from leg 4 (gen-6 vs gen-8 identical). Over the whole branch:
`libc-include/fmtmsg.h` (MM_NULL{LBL,TXT,ACT,TAG}: `0` -> `0L` in the casts, macros.tsv pointer rows);
mcheck.h, sys/ptrace.h, sys/pidfd.h (enumerators.tsv: `enum mcheck_status`, `__ptrace_request`, `__pid_type`
as data; sys/pidfd.h also keeps union sigval out of the strict configurations, 2ca7392b5); stdlib.h, sys/types.h
(layouts at every depth: `__atomic_wide_counter` is its union with `__value64` / `__value32` instead of an opaque
8-byte struct); crates/abi layout.rs and tests/appendix_c_layouts.rs (abi-gen: deep
layouts, bit positions, transparent unions).

## Jobs on the final code head 68ec5bdce (mei)

l4test3 (clippy -D warnings, `cargo xtask static`, `cargo test -p xtask -p niji-abi`, the rows test included),
l4plants3 (21 plants, dir plants-7). SUPERSEDED: l4gen2 (rows ok, 0 failed, 3 excepted; head 0825a267f), l3test2 (red: abi-types,
fixed in 182656969), l3plants2 (15/15 caught, older head), l4test1 (red: noexcept rows, fixed in 0825a267f),
l4plants1 (stopped, older head), l4test2 (green, 741 passed, head 0825a267f), l4plants2 (19 caught, htc-generic-shown
survived: the test named M_PI, which has no visibility row; fixed in 68ec5bdce). Results: l4test3 green (741
passed), l4plants3 20 CAUGHT, 1 EQUIVALENT (htc-generic-emit), 0 survived. Gate: the changed crates' tests (xtask,
niji-abi), as the earlier legs ran; whether `cargo xtask affected` escalates was not checked this leg.
Commit 68ec5bdce: the provider test uses EOF for the macro-and-function rule.

## RULING NEEDED

* NULL and TD_SIGNO_MASK: a null pointer constant's type differs by language (C `void *`, C++ `long`), and
  macros.tsv has one type column. Recommend a macros.tsv type `null` (value 0) whose definition the generator emits per
  language (C `((void *) 0)`, C++ `__null`, GCC's documented C++ null), checked by the rows test in both languages;
  it decides the C++ spelling of NULL in public headers, hence the ruling.

## INFORMATIONAL

* Deviation: provider data lives in `headers/providers.tsv`, not a prototypes.tsv column; the type-generic function
  forms use its provider `generic` (ruling #6720 allowed function rows or a visibility kind; a visibility kind would
  have reclassified every C++ overload set, sin included).
* gets/bsd_signal call-site and noexcept rows come from a `--names` run of callsite_probe.py merged into the tables,
  not a full run.
* A transparent-union member tag outside the candidate list would be missed (2894a9fb4).
* Process slips: leg 1 one `sed -i`, two empty heredocs; leg 2 two heredocs; leg 3 one no-op `sed -i`.

## Next step

Land through the train. Follow-up: the `null` macros.tsv type once ruled (removes the last two non-permanent
exceptions); the C++ overloads of iscanonical/iseqsig/issignaling/iszero belong to a math.h template when math.h is
generated.
