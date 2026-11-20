# LD-SPLIT-2 report: the loader's ld.rs split into modules by concern

Branch `task/ld-split-2`, cut from origin/next `c1413a3c4` (it contains DL-TRACE-INTERP-2). Code head `a9371bfd5`;
later commits change handoff/ only. Role: keke `~/w/n1`. Run 1's branch `task/ld-split` (old base) is superseded.

## What changed

`crates/boundary/src/ld.rs` (2,414 lines on c1413a3c4) is now `ld.rs` (the root: exports, `_r_debug`,
`_dl_debug_state`, the module map in its `//!`) plus eight new modules under `crates/boundary/src/ld/`: io, state,
tls, image, initfini, link, trace, entry. debug, dlopen, readers and table were already modules and do not change.
The module map, one line per module, is the `//!` of ld.rs.

Commits (every move commit was produced by `tools/ld_split.py`, kept in the tree so the split can be reproduced):
1. `8b637a241` audit paths first, so every move commit passes on its own. policy.rs's `export` category adds
   src/ld/entry.rs and src/ld/tls.rs. `ldso_run_features_only_from_arch` now scans ld.rs and every file under ld/,
   so dlopen.rs and its children are covered for the first time.
2. The generator commit, then eight move commits: io, state, tls, image, initfini, link, trace, entry.
3. Doc commit: the module map; spec/behaviour/dynlink-preload.md now points at ld/link.rs; the affected.rs
   direct-tier test also checks crates/boundary/src/ld/link.rs.
4. `a9371bfd5` plants: handoff/LD-SPLIT-2-plants.txt (five earlier loader plants, re-pointed at the new files).

Ruling #6193 has been applied: the split was regenerated on a next that contains DL-TRACE-INTERP. Its literal test
("differs from the checkpoint only by DL-TRACE-INTERP's lines") cannot hold. Between run 1's base and c1413a3c4,
ld.rs grew from 2,087 to 2,414 lines through four loader stories: DL-23 rendezvous, DL-STARTUP-OBJECTS tables,
LDSO-TLS-STATIC-INFO and DL-TRACE-INTERP. A stronger check was used instead: every item of next's ld.rs is in the
split exactly once, byte for byte (127 items before, 127 after, none in only one side). DL-TRACE-INTERP's change
is therefore in ld/trace.rs and ld/link.rs.

## Proof (head a9371bfd5)

| check | result |
|---|---|
| step 0: clippy (workspace, all features) + static on the inherited head | keke `ld-split-2-step0` rc 0 |
| each move commit compiles | stages 2-9 built (`ld-split-2-stages-4`, `-stages-5`); workspace clippy -D warnings clean on the final tree |
| items moved, none changed | 127 items in base ld.rs, 127 in the split, 0 only on one side (ignoring `pub(super) ` and use/mod/`//!` lines) |
| moved-only, per commit (`git diff --color-moved=zebra`) | every +/- line that is not part of a moved block is a use line, mod line, module `//!`, blank line or `pub(super)` visibility pair. One other change, which is correct: the trace commit drops `pub(super)` from link.rs's `interp_name` because its last outside user moved |
| unsafe audit markers under ld.rs + ld/ (origin/next -> head) | allow(unsafe_code) 70/70, `unsafe {` 59/59, unsafe(no_mangle) 10/10, UNSAFE( 71/71, SAFETY: 60/60, unsafe fn 2/2, unsafe impl 1/1 |
| merge-tree demo | two throwaway branches from the head, one line before `close_begin` (dlopen.rs, dlclose) and one before `_dl_get_tls_static_info` (ld/tls.rs): `git merge-tree --write-tree` rc 0, no conflict (branches deleted) |
| direct-tier gate (static + `cargo xtask affected --run`) | keke `ld-split-2-gate` rc 0: 9 suites, 630 passed, 0 failed; peak 1.9G, oom_kill 0 |
| plants (`cargo xtask plants handoff/LD-SPLIT-2-plants.txt`) | keke `ld-split-2-plants` rc 0: 5 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY, 0 WARN-NO-MSG (one WARN-DEV-SHM: none of these plants' commands mount /dev) |
| start-up hot text (`NIJI_FULL_PROFILE=1 cargo xtask so && ... --test hot_text_products`) | keke `ld-split-2-hottext` rc 0: 30 passed |
| exported symbols, every shared product, vs c1413a3c4 (both NIJI_FULL_PROFILE=1) | keke `ld-split-2-compare-4`: 17 products, 17 same, 0 differ (name@version, bind, type, visibility, defined/undefined, object sizes). No exported function changes size |
| ld.so code and data | every ld.so symbol has the same size by type (t 314, T 3, b 54, B 5, d 23: 0 sizes differ); names differ only by module path. `.text` is the same size; `.bss` is 16 bytes smaller and `.niji_ldso_span` 64 bytes larger (statics come out in a different order, so the padding changes). libc.so.6: `.text` is byte-identical and every non-debug section is the same size |
| dynamic start-up instructions:u, ASLR off, perf stat -r 10, both builds' shared products copied to paths of equal length, each probe linked with its variant's ld.so as interpreter, `env LD_LIBRARY_PATH=<dir>` (env's cost is the same for both) | `ld-split-2-compare-4`: identical, sd 0.00%. ret 1,063,496 = 1,063,496; printf 1,076,966 = 1,076,966; dlopen(libm)+dlsym+dlclose 1,692,227 = 1,692,227 |
| static start-up | cannot move: the loader is not linked into libc.a (the affected-tier test asserts that) |
| swap root | SWAP |
| FULL `cargo xtask test` | not run: owner 11-19, the CI train is the FULL gate |

## start.rs and so.rs (brief step 5: report only, no split here)

**crates/rt/src/start.rs** (547 lines, 140 of them tests). The ld.rs pattern holds only weakly here: the file has
three concerns, and only one of them is likely to collide often.
- program headers (lines 23-208): `program`, `relro*`, `bias`, `read_only` and the PT_* constants, about 185 lines;
- secure mode and the standard descriptors (210-264): `UNSECURE`, `is_unsecure`, `entry_name`, `short_name`,
  `open_std_fds`, about 55 lines;
- `pub mod private` (265-405): the private interface between libc and ld.so (version node, `__niji_*` names, the
  build tag, `StartInfo`), about 140 lines.
I did not read other stories' history, so this is inferred from structure alone: the collisions most likely come
from `private`, because every story that adds a libc/ld.so hook adds a name and a `StartInfo` field there. Proposed:
`start/phdr.rs`, `start/secure.rs`, `start/private.rs`, with tests kept next to the code they cover. This will
not remove collisions between two stories that both add `StartInfo` fields. For that, the fix is a convention:
append new fields at the end, one per line, with a trailing comma.

**xtask/src/so.rs** (3,238 lines, 452 of them tests). The ld.rs pattern holds strongly. Concerns, with line ranges:
- process helpers: run, phases, product lock, publish (50-127, about 80 lines);
- archive and start-object checks: boot anchor, start object (128-249, about 120);
- version scripts and export checks: owner node, forwards, build tag, native exports, bindings, version sets,
  compat-missing (250-764, about 515);
- archive symbol weakening and localising (765-956, about 190);
- profile, archive build, layout trace, finish, link-edge and anchor checks, crt files (957-1417, about 460);
- `run_so_as`, the orchestrator (1418-1652, about 235);
- ld.so: link, span/phase scripts, layout, trace (1653-2051, about 400) and interp checks (2236-2404, about 170);
- utilities: getconf, ldconfig, getent, iconv, ldd (2052-2235, about 185);
- placement, nonshared, owner dir, builtins, libm (2405-2785, about 380).
Proposed: `xtask/src/so/` with `mod.rs` (run_so, run_so_as, Profile, ProductBuild), `proc.rs`, `archive.rs`,
`exports.rs`, `ldso.rs`, `utils.rs`, `nonshared.rs`, `libm.rs`, with each test moved beside its code. The same
generator approach works here (tools/ld_split.py takes item lists per module). `run_so_as` would still be shared by
every story that adds a product step. Making each step a function in its own module, and keeping `run_so_as` as a
list of calls, reduces such a story to one line in `mod.rs`.

## Notes

- Generator fix-ups needed for next: `NewTls.mods` is `pub(super)` because dlopen reads it; `child_needs()` reads
  the children's `super::` uses from the tree (code only, not doc links); per-module and per-stage DROP lists
  handle locals that share a name with an item.
- Superseded measurements, kept for the record: `ld-split-2-compare` counted an error exit (libniji's ld.so rejects
  `--library-path`, at base too), and `ld-split-2-compare-2` had RUNPATH probes that do not find libc.so.6. That is
  the recorded difference: libniji's own libraries resolve only from its directory or LD_LIBRARY_PATH.
  `ld-split-2-compare-3` showed head 166, 439 and 586 instructions below base. The cause was the different lengths
  of the two product paths, because the loader does per-byte path work. At equal path lengths (`-compare-4`) the
  counts are identical. Anyone comparing start-up counts across two trees should put both at paths of equal length.
- INFORMATIONAL: `ld.so --library-path DIR prog` gives "unrecognized option" (at base too, unchanged by this split).
  The spec has no clause on ld.so's command-line options. That is a spec gap for a loader story, not this one.
- `_r_debug` and `_dl_debug_state` stay in ld.rs because they are exports. A later story may move them to debug.rs.
