# SL-HEADERS-S16: slice 16, ELF, dynamic linking, debugging (INCOMPLETE, continuation needed)

**State.** Branch `task/sl-headers-s16`, cut from `origin/next` (slice 6 is on next). `headers-gen` is clean (36 headers),
`headers-gen --check`, `cargo xtask clippy` and `cargo xtask static` are green (job `sl-headers-s16-gate`), plants run as
`sl-headers-s16-plants` (6 plants, `handoff/SL-HEADERS-S16/plants.txt`). Say "slice 16" in commits and handoffs.
`header_check.py` (P1/P5) is NOT green: 71 failures of 2087 units, all in three places listed under Open. P2, P3 and P6 were not run.

**Done.**
* Templates (all generated into `libc-include/`): `dlfcn.h`, `link.h`, `elf.h` (3,000+ values from `macros.tsv`, all ELF types from the
  existing `layouts.tsv` rows), `ar.h`, `sys/gmon.h`, `sys/gmon_out.h`, `sys/profil.h`, `gnu/lib-names.h`, `sys/elf.h`.
* `gnu/lib-names.h`: new generator marker `@@sonames`: `<STEM>_SO` for each library of `versions.tsv`/`empty-versions.tsv`
  (`LIBC_SO`, `LIBM_SO`, `LIBMVEC_SO`, `LD_LINUX_X86_64_SO`). The macro names are libniji's rule (stem upper-cased); nothing was probed
  for the reference's names (no `visibility.tsv` rows exist for this header): check them with a `#ifdef` probe in the continuation.
* `sys/elf.h`: `visibility.tsv` says every configuration rejects it, so libniji's is an `#error` pointing at `<elf.h>`;
  `header_check.py` checks the rejection (`P5 reject sys/elf.h`).
* Generator (`xtask/src/headers.rs`): anonymous-enum members (`r_state`), anonymous union/struct members (`d_un.d_val`, unnamed unions),
  member types that are typedef structures, structure objects (`extern struct r_debug _r_debug`), `la_*` audit hooks of link.h are not
  libniji exports. The member parsing moved into `member_decl`.
* Tables: `struct prof` (sprofil, sys/profil.h) and `La_x86_64_regs/retval/xmm/ymm/zmm/vector` added to `layouts.tsv` through
  `abi_extract.py --layouts-only` (earlier rows unchanged; `spec/abi/PROVENANCE` has the lines).
* `header_check.py`: slice 16 headers in P5 and P1, member `offsetof` checks for the slice's own typedefs and structs, soname macros,
  `sys/elf.h` rejection.

**Open (next step, in this order).**
1. `link.h`: `La_x86_64_regs` and `La_x86_64_retval` (typedefs.tsv rows with empty `cxx`) still emit as opaque or undeclared: their
   members use `La_x86_64_xmm/ymm/zmm/vector`, which have `layouts.tsv` rows (new) but no `typedefs.tsv` row, so `member_decl` returns None.
   Give those types typedef rows (type_probe.py) or let `member_decl` emit a guarded typedef from a layouts key.
2. `dlfcn.h`: `Dl_serinfo` is still an opaque structure although its rows (unnamed union with a `[0]` array) should now parse: find why
   `members_nested("Dl_serinfo")` returns None (not debugged: out of budget).
3. `sys/profil.h` P1 in c89: the check tests `sizeof(fd_set)` for configurations where libniji's `sys/types.h` does not make `fd_set` visible
   but `visibility.tsv` says it is; compare the `visibility.tsv` rows of `sys/profil.h` with `sys/types.h`.
4. Not generated yet: `thread_db.h` and `proc_service.h` (need slices 8/9: `signal.h`, `pthread.h`, `sys/procfs.h`, `sys/user.h`; `td_*` and `ps_*`
   are not libc exports, so the generator needs a "provided by libthread_db / the debugger" rule like the `la_*` one; the C++ call-site
   rows of `td_*` carry mangled names (the reference header has no `extern "C"`): libniji's header has it, which needs a differences row
   with no compat option, since no library can provide the mangled names). `sys/platform/x86.h`: spec gap, see RULING NEEDED.
   `a.out.h` and `stab.h` (R-H7: ship): no tables at all (not in `visibility.tsv`); they need a probe slice for `struct exec`, `N_*`, `OMAGIC`.
5. P2, P3 (`header_probe.py`/`callsite_probe.py --headers ...`), P6 (`conformance/run.sh --niji-headers` for the dlfcn, link and elf
   programs), and the pending decision record (`records/pending/SL-HEADERS-S16-1.toml`, series D-X: `@@sonames`, anonymous members,
   audit hooks, `sys/elf.h` as an error header).

**Do not redo.** The `rows` shadowing bug in `members_nested` (lost struct padding of `struct timex`) is fixed; `libc-include/{stdio,stdlib,time,getopt}.h`
are byte-identical to next's again except where the regenerated output differs on purpose (check `git diff origin/next -- libc-include`).

**RULING NEEDED.**
* `sys/elf.h`: every configuration rejects it, so shipping it only adds an `#error`. Recommend drop (SL-HEADERS-15's drop row and compat record).
* `sys/platform/x86.h`: `CPU_FEATURE_ACTIVE/PRESENT(name)` need a feature-name table (CPUID leaf, register, bit, from the Intel SDM) and the
  member layout of `struct cpuid_feature` (no rows). Recommend a new `x86-features.tsv` made from the SDM plus a layout probe of the type.

**Jobs run.** `sl-headers-s16-gate` (headers-gen --check, clippy, static), `sl-headers-s16-plants`. Diagnostics, not on the final head:
`sl-headers-s16-gen0..4`, `-lay`, `-lay2`, `-hc1` (headers-gen and header_check runs; the last header_check run reports 71 failed of 2087).
Where the time went: each generator edit needs a build on the host (about 100 s), so six generator rounds cost most of the calls.
