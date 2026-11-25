# HEADERS-ENUM-FOLLOWUPS: MS_* enumerators in both include orders, reg_errcode_t, nameser enumerations (handoff)

**State:** branch `task/headers-enum-followups` from origin/next 88d22c9a2; code head ec9d14c42 (records fix on top
of ca435eef1); role keke:~/w/n3. Decisions in force: **D-X.173** (supersedes D-X.126); COMPAT-34 withdrawn, its
row removed from docs/differences-from-glibc.md. Generated views (plan/07-decisions, docs/compat-flags.md) are the
train's.

## Done
* `MS_*`: one unnamed enumeration plus self-naming macros; `<sys/mount.h>` includes `<linux/mount.h>` first, removes
  the kernel's macros (`@@replace-macros`, slice 15), declares the enumerators. MS_NOUSER is an int enumerator
  (-2147483648); s15.py's P1 no longer defers it.
* `reg_errcode_t`: typedefs.tsv row from type_probe hp4 (regex.h, re_comp.h, black box); `<regex.h>` declares the
  `_REG_*` enumeration, `REG_*` are macros naming them (REG_ENOSYS only where visibility.tsv shows it).
* `arpa/nameser.h`: every `enum __ns_*` (8) declared with the probed values; ns_type/ns_class/ns_sect are
  `typedef enum __ns_X`. Generator fix (xtask/src/headers.rs `emit`): an enumerator whose tag row is a reserved name
  the header omits now declares its enumeration (ns_r_servfail, ns_o_query, ns_f_qr, ns_uop_*, cert_t_* were dropped).
  Cosmetic: some guarded `enum __ns_*` bodies appear twice in nameser.h (the `__NIJI_D_enum_*` guard keeps one).
* Tests (header_check modules): s15 `enum_units` (both include orders, spelling, value, C++ enum type), s10
  `errcode_units`, s12 `nameser_enum_units`. `header_check.py --modules s10,s12,s15` runs only named modules (the
  full check is ~7 min serial; plants need < 600 s).
* Results: job headers-enum-followups-test2 (cargo xtask so, cargo test -p xtask all green, header_check 0 failed
  of 17897); plants 4/4 CAUGHT (headers-enum-followups-plants1); clippy and static clean (precheck). Superseded:
  test1 (gencat transcripts need the products: red without `cargo xtask so`, green in test2), lint1 (static red on a
  newline in the D-X.173 body, fixed in ec9d14c42), pre1 (precheck naming those two).

## Open / INFORMATIONAL
* `xtask --test transcripts` (gencat) fails with exec ENOENT on a role without `cargo xtask so` although its is_file
  skip check passes; it could skip instead.
* form.py NOTE `SERVFAIL`/`YXDOMAIN` of `<arpa/nameser_compat.h>` (expression on one side only): pre-existing, the
  NOTE set is identical before and after the generator fix (1708 lines both).
* Left out (not this story): nameser/resolv function-like macros MAX MIN clrbit howmany isclr isset powerof2 roundup
  setbit; sys/mount.h `enum fsconfig_command` + 13 enumerators; resolv.h `enum __socket_type`; typedefs ns_rcode,
  ns_opcode, ns_flag, ns_update_operation, ns_cert_types (no probe candidate names them yet).

## Next step
Precheck on the final head; then CI. Report: handoff/reports/HEADERS-ENUM-FOLLOWUPS.json.
