# SL-HEADERS-S05: generator and base layer (handoff after run 3, SL-HEADERS-S05-3)

**State.** Branch `task/sl-headers-s05-3` (stacked on `task/sl-headers-s05-2`, cut from origin/next 4a0a545ef). Head:
see `handoff/reports/SL-HEADERS-S05-3.json`. Not for landing yet: P2, P3, staging and run.sh are still open (below).
A continuation reads this page, plan/headers.md §2.3, §2.4 and §3, and `xtask/src/headers.rs` by range (module doc first).

**Decisions in force.** Pending record `records/pending/SL-HEADERS-S05-3-1.toml` (D-NEW, series D-X): marker format,
`__LIELLA_H_`/`__LIELLA_D_` guards, `__liella/` reserved, table-only integer types (#6047), PTHREAD_STACK_MIN,
gnu/stubs.h source, `__STDC_ISO_10646__`. Differences row for `__STDC_ISO_10646__` (no compat option, reason in the row).

**Done.**
* Runs 1-2: generator core, levels/reserved/iso10646 tables, 16 templates, include/ committed, `headers-gen --check` in static.
* Run 3:
  * Ruling #6047: `header_probe.py` candidates now include C 7.20.1/7.20.4/7.8.1 names (`c_integer_names`). Reran
    both probes on the reference system (mei job `sl-headers-s05-3-probe1`): ftm-configs identical, visibility +2197,
    typedefs +84, macros +154 rows, 0 removed. stdint.h/inttypes.h now come only from rows: the compiler-macro
    fallback, the signed-from-unsigned derivation and `@@formats` are gone (PRI/SCN are `macros.tsv` strings via `@@emit`).
  * `type_probe.py` second pass under `default` for constants that are not constant under `gnu`: PTHREAD_STACK_MIN
    16384, used by limits.h outside `__LIELLA_DYNSTACK`. Gap closed.
  * gnu/stubs.h from `spec/abi/stubs.tsv` (Liella's own ENOSYS-only bodies; 7 default-linked get `__stub_`).
    gnu-versions.h: plan §5.1 lists it as shipped, so it stays (guards only; no table names its contents yet).
  * `spec/tools/header_check.py` (replaces the S05-2 scratch smoke): levels vs levels.tsv in all 40 configs, P5
    alone/twice with -Werror, P1 C 7.20 type identity (C sizes/alignments, C++ `__is_same`): 700 units, 0 failed.
  * Plants file `handoff/SL-HEADERS-S05/plants.txt` (6 plants: levels cell, typedef emission, stubs row, template
    value, probe candidates, probe default pass).
* The S05-2 verify red (smoke2 tested a different include/): cause was a scratch script run on a tree that was
  regenerated afterwards; now the check is a repo tool and runs on the final head (`-check2`).

**Next steps, in order.**
1. P5 rest: `-std=c89 -pedantic-errors` units and the strict-mode namespace check in header_check.py.
2. P1 rest: generated `_Static_assert` TUs for every macros.tsv value and typedefs.tsv size of the slice's headers.
3. P2/P3: `header_probe.py`/`callsite_probe.py` on `include/` (an extra-flags option), compare with `omitted()` applied.
4. `so` staging (`target/liella/include` plus the UAPI copy); `conformance/run.sh --liella-headers`.
5. Make header_check a cargo test (xtask) so the direct tier selects it.

**Open gaps.**
* `PTHREAD_STACK_MIN` under `_DYNAMIC_STACK_SIZE_SOURCE` expands to `__sysconf (75)`, which no slice-5 header declares:
  a program using it there needs `<unistd.h>` (slice 6) to declare `__sysconf` or a template change to `sysconf`.
* `FD_SET` and friends under fortify: plain macros; P3 decides.

**Do not redo.** Level table and cascade (verified in all 40 configurations), the probe rerun (tables committed).
