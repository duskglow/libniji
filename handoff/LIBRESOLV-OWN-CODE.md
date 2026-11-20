# LIBRESOLV-OWN-CODE: libresolv.so.2's own code in libresolv.so.2 (leg 1 handoff)

State: branch `task/libresolv-own-code`, base origin/next d4cbe1667 (has DYN-START-LINK-SHAPE and
LAYOUT-RO-STABLE). Role keke ~/w/n4; job scripts and scratch keke ~/work/libresolv-own-code/.
Leg 1 commits: WIP 841e284fb (code) + this note. NOT BUILT GREEN YET: `cargo xtask so` stops at the new
state check (below). No gate, plants or counts run yet.

## Findings (measured, keke, fast profile, head d4cbe1667)

* libc.so.6 never calls any of the 67 libresolv entry points: relinked without the 67 `--defsym`
  aliases, none of them is left (exp1, keke ~/work/libresolv-own-code/exp1). That relink is
  RELATIVE 2545 -> 2224 (-321), .text 3,184,905 -> 3,107,081 (-77,824 B), .data.rel.ro -3,880 B,
  .rela.dyn -7,704 B. (Fast profile; the brief's 1777/1459 are release numbers.)
* Linking libresolv.so.2 from libc's archive is WRONG (exp2): 363 KB text, binds nothing from libc.so.6,
  and carries its own malloc heap, stdio FILEs, environ, CPU-dispatch statics (44 libc export names
  duplicated). So libresolv.so.2 needs its own compilation that reaches libc state only via exports.

## Design (leg 1 code, commit 841e284fb)

* Classification (step 1): 54 names move (51 functions + `__p_class_syms`, `__p_type_syms`,
  `_res_opcodes`); 13 stay forwarded through `__niji_fwd_<name>@NIJI_PRIVATE`
  (`placeholders::STILL_FORWARDED`): `__fp_resstat` (compares the state address with libc's `_res`),
  `__hostalias`/`__res_hostalias` (thread `_res` + init, secure_getenv), `__res_close`,
  `__res_isourserver` (thread `_res`), `_gethtby*`, `_gethtent`, `_sethtent` (hosts cursor, NSS files,
  h_errno, owned result blocks), `res_gethostby*` (DNS client). Moving those needs more seams; they
  are follow-up material, not needed for the start-up gain if their closure is libc-shared code
  (check the RELATIVE delta once green).
* boundary feature `libresolv`; `export! { libc_libresolv "V"; }` = no_mangle with `export` (libc.a,
  static -lresolv, D-16.6) or `libresolv`; `libresolv_data!` for the 3 tables.
* Seams under `all(feature = "libresolv", not(feature = "runtime"))`: `users::block_addr` -> libc's
  private `__niji_resolv_block(kind, need)`; `resolv::thread_pfcode` -> `__niji_resolv_pfcode()`;
  `resolv::put` -> `fwrite`; `p_query`'s stdout -> libc's `stdout` object; Rust allocator
  `caller_alloc::Caller` -> malloc family. libc exports the two private entries at NIJI_PRIVATE
  (`LIBC_PRIVATE` in xtask/src/so.rs).
* crates/libresolv-so (product workspace member). xtask `so::libresolv_object`: cargo build,
  `ld -r --gc-sections -u <54 names>`, objcopy keep-global, checks (all 54 defined, nothing else global,
  every undefined symbol a libc.so.6 export, NO writable data except the 3 exported tables).
  `placeholders::build(.., Some(own))`: moved names exported by version script
  (`version_script_own`), forwards only for STILL_FORWARDED.
* Tests edited: crates/core/tests/sonames_products.rs (`sonames_resolv_own_code_and_private_aliases`
  replaces the forwards test; new `sonames_resolv_dlopen_own_code`; its expected first line
  `AAAA 4242 1h1m1s 1` is a GUESS: confirm against a glibc probe of the same program before trusting).

## Where leg 1 stopped (so1 log, keke ~/logs/libresolv-own-code-so1.log)

The state check refuses the object, correctly. Writable data found, and who references it
(objdump -dr of target/product/libresolv-all.o; listing in ~/work/libresolv-own-code/all.dis):
1. `cpu::STATE` read by ns_samename, ns_samedomain, ns_subdomain, ns_sprintrr(f), sym_ston (a
   case-insensitive compare dispatches on CPU features) -> `cpu::init_with` -> COPY_MIN, FILL_MIN,
   ERMS_*, AVX2_ERMS_*, FEATURES.*, DETECTIONS, dispatch::FMA, `.data1` (0xd0).
2. thread::foreign_words (SPARE, SHADOW_HEAD), thread::map_thread (NO_HUGE), cancel::push_old,
   cancel::unwound_through, sync::once_end (OWNED_HERE).
3. The object's undefined symbols include `_init`, `__init_array_start/_end`, `__fini_array_*`,
   `__ehdr_start`, `environ`, `niji_clone7`, `niji_execl4`, `niji_mq_open4`, `niji_fwd_slow_sqrtl`...:
   much more of libc than the 54 names need. Likely cause: `ld -r --gc-sections` keeps KEEP sections
   (.init_array/.fini_array, csys objects compiled without function sections) and their closure.
   CHECK FIRST: `readelf -SW libresolv-all.o | grep -E "init_array|fini_array|ctors"` and which
   archive member brings them (`ld -r ... -Map`); a libc constructor must never run from libresolv.so.2.

## Exact next step

1. Find what pulls (3): link map / `--trace-symbol` on `_init` and `niji_clone7`. Fix at the source
   (e.g. boundary build.rs compiling the csys pieces only what the libresolv product needs, or the
   ld -r step taking only the Rust object + the `niji_caller_*` shims), not by stripping sections.
2. (1): the compare's CPU dispatch: under the libresolv product use the portable path (or libc's
   exported `strncasecmp`); keep results bit-identical (owner rule: paths bit-identical, test both).
3. Rebuild (`cargo xtask so`, ~21 min), then: sonames_products + ns_parse_products + s16 symbols tests,
   affected gate, plants (docs/plants.md entries: state check, STILL_FORWARDED, seams), counts
   (RELATIVE/text/start-up rows on Zen 1 and Skylake-SP + a res_query/-lresolv program vs glibc 2.39),
   audit (step 3): readelf every secondary lib for `UND .*@NIJI_PRIVATE` (leg 1 grep: only
   libresolv forwards; libm/libmvec are own builds; the others are placeholders/stub tables).

## Rows to read

D-16.6 (`grep -rn "^| D-16.6 " plan/`), D-27.9, D-10.7 (libm product pattern), D-27.22 (NIJI_PRIVATE).
