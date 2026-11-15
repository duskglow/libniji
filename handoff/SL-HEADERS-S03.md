# SL-HEADERS-S03: type and value tables (plan/headers.md slice 3)

Branch `task/sl-headers-s03-2` (continuation of `task/sl-headers-s03`, from origin/next db204dc19). Role keke:~/w/n4
(keke: Ubuntu glibc 2.39-0ubuntu8.9, GCC 13.3, the reference toolchain). Briefs: ~/state/briefs/SL-HEADERS-S03.md (the
spec, "## The change") and ~/state/briefs/SL-HEADERS-S03-2.md. Plan rows: plan/headers.md §2.1, §2.2, §4 slice 3;
pending decision records/pending/SL-HEADERS-PLAN-1.toml. Report: handoff/reports/SL-HEADERS-S03-2.json.

## State (S03-2): head compiles; slice 3 complete
The first run stopped at a context checkpoint with a head that did not compile: `cargo xtask abi-gen` wrote `<anon>`
as a field name for the anonymous union of anonymous structs in `struct tcphdr` / `struct udphdr`
(gate `sl-headers-s03-gate2` red). Cause: layouts.tsv lists anonymous members flattened (as C names them), so the
table did not say where each anonymous struct ends; an extent alone is not enough (two sibling structs of a union
both start at its offset; the unit test `anon_structs_in_anon_union_nest_by_extent` caught that first design).
Fix (no special case for the two structs; the facts come from DWARF, not header text):
* `spec/tools/abi_extract.py`: each anonymous member's row gets `size N rows M` in the last column (DW_AT_byte_size
  of the member's type; M = rows that follow at its level and belong to it). 23 rows changed, no other row:
  14 `struct rusage`, `Dl_serinfo`, 2 `struct user`, 3 each in tcphdr/udphdr.
* `xtask/src/abi.rs`: `group()` takes exactly M rows under a synthesized field, recursively, and errors if one lies
  outside the member (or if M runs past the table); repeats are `__anon_<offset>_<k>`. Without the column
  (layouts-aarch64.tsv, not regenerated) the old rule for anonymous unions holds; an unsized anonymous struct is an
  error. Checked all shapes: in both tables the only `<anon>` members are rusage, Dl_serinfo, struct user (unions of
  scalars) and tcphdr/udphdr; named members of anonymous type are path-addressed and unaffected; no duplicate paths.
* Generated: only tcphdr/udphdr changed in crates/abi/src/layout.rs (now a union of two structs; `th_seq`/`th_ack`
  stay opaque `tcp_seq`); aarch64 output byte-identical. 221 Appendix C layout tests pass.

## Work done (slice 3, first run; do not redo)
* `spec/tools/type_probe.py` -> `spec/abi/typedefs.tsv` (3804 rows) and `spec/abi/macros.tsv` (7112 rows); two runs
  byte-identical (`sl-headers-s03-probe1`).
* `spec/tools/abi_extract.py`: 50 protocol, ELF, debug and large-file types; layouts.tsv 211 types.
* `xtask/src/abi_types.rs`: static check `abi-types` (6 unit tests).

## Proof (keke, final head)
* `sl-headers-s03-2-lint1`: abi-gen --check, xtask abi/abi_types unit tests (11), abi-types, workspace clippy
  -D warnings, xtask clippy, static: rc 0.
* `sl-headers-s03-2-plants`: handoff/SL-HEADERS-S03-2/plants.txt, 24 plants (17 checker plants from the first run,
  never completed there; 7 on the grouping and the probe column): 24 CAUGHT, 0 SURVIVED, no WARN lines
  (results: handoff/SL-HEADERS-S03-2-plants.txt).
* `sl-headers-s03-2-gate`: `cargo xtask affected --run --base origin/next`; affected escalated it to FULL (crates/abi,
  xtask and spec/abi changed), so it runs `cargo xtask test` (still running at hand-back; the dispatcher reads it).
* Superseded: `sl-headers-s03-2-regen1` (first grouping design, unit tests red), `-regen2` (table and generated
  files; tooling, not a gate).

## Open / next
* INFORMATIONAL: layouts-aarch64.tsv lacks the 50 new types and the `size N rows M` column (needs the arm64 sysroot run,
  `abi_extract.py --arch aarch64`); abi-gen handles it as before.
* INFORMATIONAL: macros.tsv's floating and string rows are few because the candidates are slice 2's; add the
  standards' macro names (C 7.8 PRI/SCN, POSIX `M_*`, paths.h) to header_probe.py's candidates and rerun slices 2-3.
  prototypes.tsv has 3778 names against 3397 function rows (C++ overload sets such as `strchr`).
* `tcp_seq` members are opaque in the generated Rust (no typedef mapping; typedefs.tsv could supply one later).
* Next step: slice 4 of plan/headers.md.
