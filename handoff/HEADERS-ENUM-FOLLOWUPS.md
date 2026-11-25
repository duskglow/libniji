# HEADERS-ENUM-FOLLOWUPS: MS_* enumerators in both include orders, reg_errcode_t, nameser enumerations (handoff)

**State (leg 1, WIP):** branch `task/headers-enum-followups` from origin/next 88d22c9a2; role keke:~/w/n3. Scratch
`~/w/.scratch/HEADERS-ENUM-FOLLOWUPS/` (probe scripts probe1-3.sh, merge_vis.py, merge_td.py, probe outputs
hp1/hp3-visibility.tsv, hp2-typedefs.tsv, hp2-enum.tsv). Decision id reserved: **D-X.173** (supersedes D-X.126).
Not yet built or tested: no headers-gen, no clippy, no gate.

## Done (WIP commit)
* `spec/tools/header_probe.py` candidates: the names of enumerators.tsv rows, their `enum TAG` tags and `typedef
  NAME` typedef names (`_REG_NOMATCH`, `reg_errcode_t`, `ns_t_a`, `__ns_type` were in no candidate table).
* Probes on keke (reference headers, black box): jobs headers-enum-followups-hp1, -hp3 (header_probe.py --headers
  regex.h,re_comp.h,arpa/nameser.h,resolv.h,sys/mount.h: every committed row of those headers reproduced, 0 changed,
  0 gone); -hp2 (type_probe.py on regex.h,re_comp.h,arpa/nameser.h,resolv.h: 324 rows identical, 8 new; enum_probe.py
  over every header -> hp2-enum.tsv).
* visibility.tsv: +340 rows, only this story's: enumerators and `enum` tags of `enum __ns_*` (arpa/nameser.h,
  resolv.h), `_REG_*` enumerators and the `reg_errcode_t` typedef (regex.h, re_comp.h). Left out (INFORMATIONAL,
  not this story): 9 function-like macros MAX MIN clrbit howmany isclr isset powerof2 roundup setbit in nameser/resolv;
  sys/mount.h `enum fsconfig_command` + 13 enumerators (FSCONFIG_*, F_OWNER_*); resolv.h `enum __socket_type`.
* typedefs.tsv: +8 `enum __ns_*` rows (arpa/nameser.h, size 4 align 4). Table order: by name (stable).
* enum_probe.py: KEPT_MACROS (D-X.126) removed; `--derive hp2-enum.tsv`: enumerators.tsv +27 rows `anon MS_ACTIVE`,
  enum-macros.tsv +27 `MS_X MS_X` (MS_BORN, MS_NOREMOTELOCK, MS_NOSEC, MS_SUBMOUNT, MS_VERBOSE, MS_RMT_MASK,
  MS_MGC_VAL, MS_MGC_MSK are not enumerators in the reference: stay the kernel's macros).
* form.py: KEPT and the REG_* GAPS entry removed.
* Generator: typedefs.rs, a typedef whose cxx is an `enum TAG` of enumerators.tsv -> `tag(TAG, "enum")` +
  `typedef enum TAG name;` (was an opaque struct). New slice module `xtask/src/headers/slices/s15.rs`:
  `@@replace-macros` (#undef, before @@emit, each anon-enumeration enumerator that names itself as a macro, under its
  condition). `headers/sys/mount.h.in`: `@@replace-macros` before `@@emit`; the old MS_NOUSER undef/literal block removed.

## Exact next step
1. Sync; on keke run `type_probe.py --headers regex.h,re_comp.h --out ~/work/headers-enum-followups/hp4` (jobrun), fetch,
   `python3 -I ~/w/.scratch/HEADERS-ENUM-FOLLOWUPS/merge_td.py <wt> hp4-typedefs.tsv --apply` (adds `typedef
   reg_errcode_t`; without it typedef_enum() never fires and _REG_* are not declared).
2. `cargo xtask headers-gen` on the role; fix errors; `python3 spec/tools/header_check.py` (form: cxx17 `type differs`
   rows must be 0 now: were 45 = 27 MS_* + 18 REG_*).
3. Records: `cargo xtask records new decision D-X.173` with `supersedes = ["D-X.126"]`; retire COMPAT-34 (see
   `records check` for the status value); delete the MS_* row of docs/differences-from-glibc.md (line ~327);
   spec/abi/PROVENANCE paragraph (probes above); s15.py docstring/DEFERRED MS_NOUSER (now an int enumerator: can be
   un-deferred).
4. Tests (rows in header_checks modules): s15.py both include orders, C and C++, `-Wall -Wextra -Wsystem-headers
   -Werror`, `static_assert(std::is_enum<decltype(MS_RDONLY)>)`-style + `#define` self-naming check; s10.py
   (regex) `reg_errcode_t e = REG_NOMATCH;` C++; s12.py or a nameser unit: `ns_type t = ns_t_a; switch (c) { case
   ns_c_in: ... }` C and C++ with -Wswitch -Werror. Spec clause or `no spec clause` named on each.
5. Plants file handoff/HEADERS-ENUM-FOLLOWUPS-plants.txt (remove @@replace-macros; typedefs.rs enum branch -> opaque;
   enum_probe KEPT back), plants job, gate (affected escalates FULL: precheck + `cargo test -p xtask` + header_check),
   clippy, static, precheck, report json, handback-check.
