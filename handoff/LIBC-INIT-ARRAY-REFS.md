# LIBC-INIT-ARRAY-REFS: no whole-scope lookups at start for libc's init/fini-array references

**State:** branch `task/libc-init-array-refs` from origin/next f1a02c7a0. Code commit 9e4edde25; this note and the
report follow it. Decisions in force: ruling #7568 (take a dynamic program's arrays from its dynamic section, drop the
references from libc.so.6). No new D-row: the change follows the existing `__rela_iplt_start` form in the same file.

## What changed

* `csys/src/adapter/runtime.c`: the eight references (`_init`, `_fini`, `__preinit_array_{start,end}`,
  `__init_array_{start,end}`, `__fini_array_{start,end}`) are `weak, visibility("hidden")` instead of `weak`. They are
  resolved when the object is linked: in libc.so.6 to 0 (the linker turns the GOT load into `mov $0x0,%rax`,
  checked by disassembly), in a static or static-PIE link to the program's own markers. libc.so.6 now has no
  dynamic symbol, relocation or start-up lookup for them. Only a static program's start reads them
  (`crates/boundary/src/runtime.rs`, `static_program` branches). A dynamic program's arrays were already run by the
  loader from DT_PREINIT_ARRAY / DT_INIT / DT_INIT_ARRAY / DT_FINI_ARRAY / DT_FINI (`crates/ld/src/init.rs`), so
  libc.so.6 needed no new code to take them from the dynamic section (the ruling's intent was already how it worked).
* New test `crt_products::plain_cc_runs_the_program_arrays_in_the_reference_order` (s1_1_c_5, s1_1_c_6), oracle
  `crates/core/tests/oracle/crt_arrays.c`. A plain `cc` link (PIE, `-static`, `-static-pie`) of a program with a
  `.preinit_array` entry, `.init`/`.fini` contributions, constructors and destructors with a priority, and `atexit`
  from a constructor and from main. Each form's output must equal the fixed order, which the same links against the
  reference also produce. Before this, no test ran a static program's preinit array, `.init`/`.fini` or fini array
  against the reference.

## Counts (STARTUP-REPROFILE kit meas.sh, unchanged; release products, Zen 1 host, ASLR off, median of 101)

| program | before (f1a02c7a0) user / kernel / faults | after user / kernel / faults | user delta |
|---|---|---|---|
| empty main (n0) | 268,752 / 772,298 / 58 | 261,771 / 773,515 / 58 | -6,981 |
| 3 libraries (n3) | 377,987 / 1,116,435 / 70 | 368,477 / 1,120,906 / 70 | -9,510 |
| `/bin/true` (t) | 329,137 / 774,055 / 58 | 322,147 / 776,093 / 58 | -6,990 |
| 200 imports (n200) | 554,657 | 543,549 | -11,108 |
| glibc 2.39 empty main (g0) | 96,040 / 590,796 / 50 | 96,040 / 596,899 / 50 | 0 |

min/max spread: 2 instructions. The kernel column moves by noise (glibc's own g0 moved +6.1k between the two runs).
libc.so.6 symbolic relocations: 55 -> 47 (kit's GLOB_DAT/JUMP_SLOT/64/TPOFF/DTPMOD count); undefined weak dynamic
symbols 0; ld.so has none of the eight before or after. **Skylake-SP not measured** (budget). STARTUP-REPROFILE found
the two classes within 0.05% on the same products, so the delta is expected to hold there, but it was not measured.

## Order probe (scratch, not in the tree; dynamic, program + liba + libb linked, libc dlopen'd from main, atexit in each constructor and in main)

libniji (before and after) and glibc 2.39 print the same 16 lines (`cmp` equal): preinit main; ctor libb; ctor liba;
ctor main; main main; ctor libc; dlopen ok; atexit from ctor libc; atexit from main main; atexit from ctor main; dtor
main; dtor liba; atexit from ctor liba; dtor libb; atexit from ctor libb; dtor libc.

## Jobs (keke, role r1)

`libc-init-array-refs-base` (origin/next release build + counts), `-after` (release build + counts + order probe +
`NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`: 33 passed), `-lint` (clippy all targets + static),
`-gate` (direct tier), `-plants` (4 plants), `-xtest` (`cargo xtask test`).

* lint: rc 0 (clippy `--workspace --all-targets --all-features -D warnings`, `cargo xtask static`).
* gate: rc 0, 9 suites ok, 1218 passed / 0 failed / 114 ignored, peak 2.1G; it ran `niji::crt_products`. `affected`
  printed `FULL: the train's` for csys/ and ran its bounded set (products with the start-up layout tests, k5,
  niji-boundary).
* plants (`handoff/LIBC-INIT-ARRAY-REFS-plants.txt`, `cargo xtask plants`): 4 CAUGHT, 0 SURVIVED, each by the new
  test's panic message. They cover the init array, the preinit array and the fini array (each start marker moved to its
  end marker) and `_init` resolved to 0. Only warning: WARN-DEV-SHM (the brief's /dev/shm directory for this host class).
* xtest: see the hand-back.

## Next step

None for this story. Skylake-SP counts if wanted (copy `~/work/libc-init-array-refs/{base,after}` and the kit).
