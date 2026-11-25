# HEADERS-FORM-BREAKS: the macro-form audit's build breakers (A1, B1, B2, B3)

**State:** branch `task/headers-form-breaks` (from origin/next 44304e709); head: see
`handoff/reports/HEADERS-FORM-BREAKS.json`. Plants: `handoff/HEADERS-FORM-BREAKS-plants.txt`.
Decisions in force: none new (every change matches the reference's observable form; no differences row).

## What changed (templates and tables only; `libc-include/` regenerated with `cargo xtask headers-gen`)

* **B1** `headers/reserved.tsv`: `__xpg_basename`, `__res_init`, `__res_ninit`, `__res_nclose` emitted (C.9 rows;
  their prototypes.tsv/visibility.tsv rows already existed, the reserved-name default omitted them).
  `headers/resolv.h.in`: `res_init`/`res_ninit`/`res_nclose` expand to the `__res_*` functions (were self-named).
  `headers/string.h.in`: the GNU `basename` (C and the C++ pair) is declared only `#ifndef basename`: after
  `<libgen.h>` its macro would rename the declaration into a second, conflicting `__xpg_basename`.
* **A1** `headers/pthread.h.in`, `headers/dirent.h.in`: PTHREAD_STACK_MIN in `<limits.h>`'s form (`__sysconf
  (_SC_THREAD_STACK_MIN)` under `__NIJI_DYNSTACK`, the constant otherwise).
* **B2** `headers/sys/vlimit.h.in`: `#undef INFINITY` then the int `@@value(INT_MAX)`; the header included last
  decides INFINITY (reference: math.h then vlimit gives int, vlimit then math.h gives float; libniji the same).
* **B3** cause: `<sys/mount.h>` includes the kernel UAPI `<linux/mount.h>`, whose MS_NOUSER is `(1<<31)` (int
  overflow: no integer constant expression in C, "variably modified at file scope"); the generator's own guarded
  definition never took effect. `headers/sys/mount.h.in` now `#undef`s it and defines `@@value(MS_NOUSER)`
  (`(-2147483647-1)`).
* **Test** `spec/tools/header_checks/form.py`: `ENFORCED` (macro, header) pairs (PTHREAD_STACK_MIN in pthread.h,
  dirent.h, limits.h; basename; res_init/res_ninit/res_nclose; vlimit INFINITY; MS_NOUSER in the C configurations)
  turn their FORM-DIFF rows into FAIL lines (module and stand-alone); every other group stays report mode.

## Probe-settled behaviour (black box, keke = Zen 1 host, gcc 13 with gcc 14's default errors:
`-Werror=implicit-function-declaration,implicit-int,int-conversion,incompatible-pointer-types`)

Symbols read with `nm -u` on the probe's own objects; types by `_Generic`/`__is_same`. 7 C configurations (none,
c11, posix2008, xopen700, gnu, default, dynstack) and g++ (default, -std=c++17):
* `<libgen.h>` basename: `__xpg_basename`, `char *(*)(char *)`, every configuration, both include orders with
  `<string.h>` (C and C++). `<string.h>` alone, `_GNU_SOURCE`: `basename` (GNU), `const char *` parameter.
* res_init/res_ninit/res_nclose: `__res_init`/`__res_ninit`/`__res_nclose`, every configuration.
* PTHREAD_STACK_MIN (pthread.h, dirent.h): constant int in none/default; `long` call of `__sysconf` in gnu,
  dynstack, C++; undefined in c11/posix2008/xopen700.
* After the change every probe cell (70 C, 18 C++) matches, except C++ `decltype (MS_NOUSER)` (enumeration type
  in the reference): the audit's group C1, unchanged, out of scope.
* Form audit (`form.py --headers libgen.h,resolv.h,pthread.h,dirent.h,sys/vlimit.h,sys/mount.h,limits.h,string.h`):
  before 53 rows of A1/B1/B2/B3 (A1 6, B1 32, B2 8, B3 7); after 0 (FORM-RUNTIME PTHREAD_STACK_MIN in dirent.h,
  limits.h, pthread.h: "libniji matches").

## Results (keke, head d888fb901)
* `python3 spec/tools/header_check.py`: 0 failed of 17150 (with the ENFORCED rows), job headers-form-breaks-hc2.
* Proof program (scratch: basename from both headers, res_init/res_ninit/res_nclose, PTHREAD_STACK_MIN, MS_NOUSER
  array, vlimit INFINITY in `#if`; gcc 14's errors): builds on both sides, libniji static link (libc.a) prints the
  reference's output (`xpg lib gnu []`, `res_init 0 res_ninit 0`, `stack_min 1 nouser 1`); objects reference
  `__xpg_basename`, `basename`, `__res_*`, `__sysconf` as the reference's. Job headers-form-breaks-hc2.
* Plants: 4 of 4 CAUGHT with their messages (job headers-form-breaks-plants).
* clippy and `cargo xtask static` clean on the role. `cargo xtask affected`: FULL (generated-table inputs), the
  train's. `sh conformance/run.sh --niji-headers` (job headers-form-breaks-hc, superseded: its proof step had a
  wrong crt path): 140 passed, 7 failed, none from this change (see Open).

## Open / next
* INFORMATIONAL, same cause as B1 outside the audit's list: `conformance/c/s00_result_buffers` with
  `--niji-headers` fails to link (`p_time`, `p_rcode`, `p_class`, `p_type` undefined: libniji exports
  `__p_*` (codemap); which symbol the reference's calls reach was not probed). Other `--niji-headers` build failures, untouched here: `PA_INT`
  (s12_registry, s12_symbols_registry), `ELFMAG`/`RTLD_DL_LINKMAP` (s27_dladdr), `RTLD_DI_*` (s27_dlinfo,
  s27_dlopen_tls_dynamic, s27_dynamic_address_entries). Next step: a follow-up story for the resolv.h `p_*` family
  (probe every reference resolv.h name whose call reaches a `__` symbol).
* C1 (C++ enumeration types, incl. MS_NOUSER) and C2, D1, D2, E1, F: the audit's other groups, still report mode.
* No gcc 14 on the build hosts: gcc 14's behaviour is emulated with the `-Werror=` flags above.
