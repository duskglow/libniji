# HEADERS-SIGSTKSZ-MATCH: MINSIGSTKSZ and SIGSTKSZ match the reference's form and value

State: branch `task/headers-sigstksz-match` (from origin/next 4540a7cc8). Decisions in force: D-20.10 (owner
ruling 2026-11-24, final), spec/ERRATA.md E14. Plants: `handoff/HEADERS-SIGSTKSZ-MATCH-plants.txt`.

## What changed

* `headers/signal.h.in` (and the generated `libc-include/signal.h`): under `__NIJI_DYNSTACK` (`_GNU_SOURCE`,
  `_DYNAMIC_STACK_SIZE_SOURCE`, C++) `MINSIGSTKSZ` is `__sysconf (_SC_SIGSTKSZ)` (was `_SC_MINSIGSTKSZ`);
  `SIGSTKSZ` unchanged. The constant branch (2048, 8192) is unchanged.
* `spec/tools/header_checks/s08.py`: a `value` unit per configuration where signal.h, sys/signal.h or sys/wait.h
  shows the macros: constant configurations assert the values as an integer constant expression; run-time ones
  assert `!__builtin_constant_p` (a static array or `constexpr` use fails, as with the reference), type `long`
  (coordinator note: the expansion's function is declared, so the call has sysconf's type) and, with `__sysconf`
  redefined as `(n)` after the include, that both name `_SC_SIGSTKSZ`.
* `conformance/c/s20_sigstksz_dynamic.c`: both macros must equal `sysconf (_SC_SIGSTKSZ)` at run time (was: MIN at
  least `_SC_MINSIGSTKSZ`); runs against the reference and with `--niji-headers`.
* D-20.10 record; ERRATA E14 (the spec and the 2.34 NEWS text say `_SC_MINSIGSTKSZ`; the reference gives
  `SIGSTKSZ`'s value; spec text unchanged, owner decides on an amendment). There is no `differences-from-docs` file
  in the tree, so the documentation disagreement is recorded in ERRATA E14 and D-20.10 (no differences row: libniji
  matches the reference).

## Probe matrix (black box, compile and run; libniji = its staged headers, run through its own loader)

"const" = a file-scope `static char a[M]` (C) or `constexpr long v = M` (C++) compiles. Values printed by a program
as `M sysconf(_SC_MINSIGSTKSZ) sysconf(_SC_SIGSTKSZ)`. Compile results from the Zen 1 host (headers and compiler,
host independent); run values on both hosts. Every cell is identical for reference and libniji.

| config | macro | defined | const | Zen 1 (keke, EPYC 7551) | Skylake-SP (mei, Xeon 8151) |
|---|---|---|---|---|---|
| gcc, no macro | MINSIGSTKSZ | yes | yes | 2048 | 2048 |
| gcc, no macro | SIGSTKSZ | yes | yes | 8192 | 8192 |
| `-std=c11` | both | no | - | - | - |
| `-std=c11 -D_POSIX_C_SOURCE=200809L` | both | no | - | - | - |
| `-D_XOPEN_SOURCE=700` | MINSIGSTKSZ / SIGSTKSZ | yes | yes | 2048 / 8192 | 2048 / 8192 |
| `-D_GNU_SOURCE` | both | yes | no | 8192 (sc_min 1776) | 14528 (sc_min 3632) |
| `-D_DYNAMIC_STACK_SIZE_SOURCE` | both | yes | no | 8192 | 14528 |
| g++ defaults | both | yes | no | 8192 | 14528 |
| g++ `-std=c++17` | both | yes | no | 8192 | 14528 |

Before the change libniji gave MINSIGSTKSZ = 1776 / 3632 in the four run-time rows. The brief expected the two
`-std=c11` rows to be constants; neither side defines the macros there (strict ISO C), as visibility.tsv has it.
The premise's "3632/14528 on the Zen 1 host" are the Skylake-SP host's values (AVX-512 frame); Zen 1 gives 1776/8192.

The landed form audit (`spec/tools/header_checks/form.py` on next, run standalone from a copy against this tree,
signal.h, sys/signal.h, sys/wait.h): `FORM-RUNTIME MINSIGSTKSZ/SIGSTKSZ ... libniji matches`, no FORM-DIFF row for
either (A2 clean for them; its `_SC_*` enum-type rows are outside this story).

## Results

* `python3 spec/tools/header_check.py`: 0 failed of 17150 (with the new units).
* conformance s20_sigstksz_dynamic: passed against the build host's headers and with `--niji-headers`.
* clippy clean; `cargo xtask static` clean (after naming the test file in D-20.10's `tests`).
* Gate (headers-sigstksz-match-gate): 776 passed / 0 failed in 9 suites, conformance 1 passed, peak 2.2G.
* Plants (headers-sigstksz-match-plants): 5 plants, 4 CAUGHT, 1 WARN-NO-MSG (hsm-min-sc-run: rc 1 with
  `conformance: FAIL s20_sigstksz_dynamic`; the expected text is in the program's stderr file, so the right regex is
  `conformance: FAIL s20_sigstksz_dynamic`).

## Notes

* `cargo xtask affected` treats `spec/tools/header_checks/*.py` as docs only, so header_check.py is not in the
  direct gate; it runs in the train's FULL steps. The plants call it directly.
* Next step: none beyond landing. The owner may amend s20_7_i_0's text (E14).
