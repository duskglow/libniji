# DL-OBJECT-PATHS: the path an object was loaded from, in every interface (GitHub #24)

State: branch `task/dl-object-paths`, cut from origin/next 0495c4684; head and job results in
handoff/reports/DL-OBJECT-PATHS.json and the hand-back.

## Premise check

The brief (written 2026-11-18) says search-found objects were reported by their bare DT_NEEDED name.
DL-NAME-PATH and DL-NAME-PATH-2 landed since then (handoff/DL-NAME-PATH-2.md): the start's link maps already
carry the path each search hit was opened at. This run's probe confirms that, so step 2's change for the
start-time names was not needed. The one difference the probe found is `dladdr` inside the vDSO (below).

## Probe-settled behaviour (glibc 2.39 black box, Zen 1 host; probes in the run's scratch)

Corpus in directory P; started `./bin/prog` from P with `LD_LIBRARY_PATH=P/libs:P/dl`,
`LD_PRELOAD="P/pre/libpa.so pre/libpb.so libpc.so"`, DT_RUNPATH `$ORIGIN/lib`, then `dlopen` of the names shown.
Every row's string is the same in `dlpi_name`, `dli_fname`, `_r_debug` and `dlinfo(RTLD_DI_LINKMAP)`, except where
noted. libniji before this change = origin/next 0495c4684.

| object | reference | libniji before | after |
|---|---|---|---|
| program | `""`; `dli_fname` = argv[0] (`./bin/prog`) | same | same |
| vDSO | `linux-vdso.so.1` in all four | `dladdr` returned 0 (the rest agreed) | same as reference |
| vDSO address, `dladdr1` | 1; base = vDSO page; symbol: highest start, lowest index (`clock_gettime` over `__vdso_clock_gettime`); `RTLD_DL_LINKMAP` = the vDSO's node; `RTLD_DL_SYMENT` set, NULL with no symbol; range = PT_LOAD lowest..highest, not page-rounded | 0, untouched | same as reference |
| LD_PRELOAD absolute / relative / bare | `P/pre/libpa.so` / `pre/libpb.so` (as given) / `P/libs/libpc.so` (searched dir + name) | same | same |
| DT_NEEDED bare via LD_LIBRARY_PATH | `P/libs/libsearch.so` | same | same |
| DT_NEEDED via `$ORIGIN` RUNPATH | `P/bin/lib/librun.so`; under `ld.so ./bin/prog`: `P/./bin/lib/librun.so` | same | same |
| DT_NEEDED with `/` (`slash/libslash.so`) | as given | same | same |
| libc.so.6 | the cache/dir path (`/lib/x86_64-linux-gnu/libc.so.6`); libniji: the LD_LIBRARY_PATH dir + name | same rule | same |
| system library via default dirs/cache (libz.so.1) | `/lib/x86_64-linux-gnu/libz.so.1` | same | same |
| ld.so | the program's PT_INTERP string | same | same |
| dlopen bare / `./dl/x` / absolute / `dl/../dl/x` | searched dir + name / as given / as given / as given | same | same |
| relative LD_LIBRARY_PATH (`libs`) | `libs/libsearch.so` (as written) | same | same |

## What changed

- `crates/boundary/src/ld/dlopen/dladdr.rs`: `vdso_dladdr`, a `#[cold] #[inline(never)]` fallback called only when
  no loaded object holds the address. It reads the vDSO image through `conv::kernel_image` (existing obligation:
  length from the image's own header) with `niji_ld::elf::Elf::parse`, and uses the objects' own `containing`.
  No new `unsafe`. `readers::vdso` became `pub(super)`; `debug::vdso_map` gives the vDSO's node.
- `crates/ld/tests/object_names.rs`: `every_object_is_named_by_the_path_it_was_opened_by`, every row of the table
  above (not the `$ORIGIN`/relative cases the DL-NAME-PATH-2 tests already cover) plus a vDSO function's symbol.
  Its C program passes unchanged against the reference.
- Decision D-27.93 (amends D-27.45): `records/decisions/D-27/D-27.93.toml`. No differences row: libniji now matches.
- Plants: handoff/DL-OBJECT-PATHS-plants.txt (the brief's bare-name plant, the fallback removed, the vDSO symbol
  bias, the vDSO link map).

## Start-up

No start-up code changed: `dladdr` runs only when called, and the new function is out of line in ld.so. libc.so.6
is untouched. No start-up counts were taken: the change adds no start-time code (brief's cost clause holds trivially).

## Run state (leg 2)

- Code head ed5783e23 (synced to keke:~/w/n4). Also changed: `crates/ld/tests/dl_find_object_index.rs`,
  whose dladdr oracle left out the vDSO (it encoded the old miss); now `span_owner` for both lookups.
- On ed5783e23 (keke): `dl-object-paths-gate3` direct-tier gate green, 742 passed / 0 failed (11 suites), peak 1.9G;
  `dl-object-paths-plants3` 4/4 CAUGHT, 0 WARN (results in handoff/DL-OBJECT-PATHS-plants.txt);
  `dl-object-paths-startup` (`cargo xtask test`, full-profile `so` and `hot_text_products`): see the hand-back.
- Earlier: clippy and `cargo xtask so`, object_names 5/5 (dl-object-paths-t1, head d59d55295, same code); `records
  check` and `cargo xtask static` (dl-object-paths-static, head 5e3e34cf2). Affected: Direct tier (no FULL).
- SUPERSEDED (stopped; older heads or an unsynced tree): dl-object-paths-gate, -plants, -gate2, -plants2.
- Commits after ed5783e23 touch handoff/ only.
- Next step: land (nothing left in this story).


## Open / next

- Not probed: `dlopen("linux-vdso.so.1", RTLD_NOLOAD)` and `dlsym` on such a handle (a loader story if they differ).
- Once landed the coordinator can comment on duskglow/libniji#24 that every case is covered.
