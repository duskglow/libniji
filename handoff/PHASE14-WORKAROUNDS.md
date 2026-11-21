# PHASE14-WORKAROUNDS: the pre-loader workarounds in conformance, native, rust-dylib and validation

State: branch `task/phase14-workarounds` (from origin/next b732d9b46), code head 9c0bac1a3; jobs on
mei role s1: `phase14-workarounds-gate1` (direct tier), `phase14-workarounds-plants1`
(handoff/PHASE14-WORKAROUNDS-plants.txt, 5 plants). Premise: AUDIT-PRE-SELFHOST items 2, 3, 4, 8
(the loader finds `DT_NEEDED` by name on `LD_LIBRARY_PATH`); nothing landed since moved it.

## What each step now runs that it did not before

1. `conformance/run.sh` dynamic mode: libniji's builds link against `target/niji/libc.so` (the
   products' own script: `libc.so.6`, libniji's `libc_nonshared.a`, the loader) with
   `-L target/niji`; no SONAME-less copy, no host `libc_nonshared.a`. The programs need
   `libc.so.6` by name (checked: `NEEDED libc.so.6`) and the loader finds it on `LD_LIBRARY_PATH`
   (helper dir, then target/niji). New coverage: libniji's `libc_nonshared.a` in every dynamic
   link, the loader's `LD_LIBRARY_PATH` search for libc.so.6 in all 7 dynamic tests (7 of 7 pass).
   `s27_dlinfo`'s SERINFO check now expects the run's `LD_LIBRARY_PATH` entries (helper dir still
   checked first), since libniji's run names one more directory.
2. `cargo xtask native`: the examples link libgcc_eh (static: in a group with `libc.a`; dynamic:
   after `libc.so.6`) instead of `nounwind.o`'s aborting `_Unwind_Resume`; the dynamic links use
   `crates/native/link/personality.c` (weak `rust_eh_personality` only: `libc.so.6` exports
   none). New checks: both runs assert `_Unwind_Resume` is defined and not WEAK (libgcc_eh's).
   5b links `libc.so.6` by soname and runs with `LD_LIBRARY_PATH=target/niji` (env step expects 3
   entries then). STD-SMOKE has not landed: no `std` example added.
   `libc_by_path` stays: `cargo xtask so` still makes `libc-by-path.so` with it (out of scope).
   `nounwind.c` stays: Rust ABI lr1's recorded `product.link_args` name `nounwind.o` and the
   definition's hash covers them (changing it would need a new ABI version).
3. `cargo xtask rust-dylib`: the program is now run, for lr1 and the coexistence variant
   lr999-test, under `target/niji/ld-linux-x86-64.so.2` run as a program (the recorded link is
   unchanged) with `LD_LIBRARY_PATH=<version dir>:target/niji`; its output must equal the dynamic
   example's with `ok rust abi: built against <abi>, running with <abi>` first (14 steps).
4. `validation/`: `shim/missing.c` was linked only by suite.sh's `niji-w`/`niji-t1`/`niji-t1r`,
   `shim/libm.c` by nothing (git grep). Modes and both files removed; `profile.sh` (the only
   reader of the niji-w build) reads the as-is `niji` build; README keeps the recorded results.

Checked on mei s1 (job phase14-workarounds-dev2, head 37f08f3fc + the env-count fix in 9c0bac1a3):
conformance 7/7 dynamic tests pass; native and rust-dylib differed only by the env count (fixed).

## Not to redo
- libgcc_eh links against both products (no unresolved symbols); libniji's ld.so as a program
  honours `LD_LIBRARY_PATH`.

## Next step
If gate1/plants1 are green: nothing. Open: items 5 and 6 of the audit (bench --compare dynamic
column; workload gate) are separate stories.
