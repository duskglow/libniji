# RENAME-NIJI-CODE: Liella -> libniji (code). State at the first checkpoint, 2026-11-16

Branch `task/rename-niji-code`, base `origin/next` e1afc4e47. Role keke `~/w/r5` (synced at 36faffcf0 + the dylib test fix).
Brief: `~/state/briefs/RENAME-NIJI-CODE.md`; mapping and reasons: liella-ops `state/RENAME-LIBNIJI-PLAN.md`.

## Done (commits, oldest first)
- 5d693a101 `tools/rename_niji.py`: mapping table, exclusion list, PROTECT list (author identity, repo URL,
  liella-ops, `src/liella`, `liella-historic`, `LIELLA-UTILITIES-DESIGN`), EXPLICIT spellings (the 8-byte nscache
  magic `LIELLANC` -> `NIJI_NSC`; `libliella_native` -> `libniji`; `-lliella_native` -> `-lniji`), length-prefixed
  mangled names (`15liella_boundary` -> `13niji_boundary`), `git mv` for paths. A second run changes nothing.
- a0e0dd21a the script's output (1202 files, 11 path moves). 0a31513f3 adds `results/` (dated measurement records)
  to EXCLUDE and restores it: **the tree at 0a31513f3 is exactly the current script run on e1afc4e47** (reviewer:
  check out e1afc4e47, copy the script from the head, run it, diff against 0a31513f3).
- 8edbe7557, the dylib test commit: fixes a substitution cannot make: test strings with lengths (sethostname, setdomainname,
  prctl), ELF note owner "libniji" is 8 bytes with NUL (`.note.niji.nonshared` `.long 8`; Rust ABI note namesz 8).
- ad0ef9275 `--self-test` (8 examples, 7 paths; PASS). 36faffcf0 pending decision record
  `records/pending/RENAME-NIJI-CODE-1.toml` (D-NEW, series D-X; no differences row: no glibc-visible name changes) and
  test `xtask headers::tests::niji_macro_always_defined` (features.h and stdc-predef.h alone define `__NIJI__` 1;
  the always-defined ruling had no test).

## Open: fallout (next step, in this order)
1. **Clippy red** (job `rename-niji-code-clippy1`): a bare `Liella` used as a Rust identifier became `libniji`
   (`crates/boundary/src/heap.rs:292 pub struct libniji;`, `xtask/src/bench.rs:17` enum variant `libniji`, and their
   uses). Fix in the script, not by hand: a rule for `Liella` in identifier position in `.rs` files -> `Niji`
   (find the original sites with `git grep -nw Liella e1afc4e47 -- '*.rs'` outside comments and strings), add an
   EXAMPLES row, rerun on the base, recommit the output, then rerun clippy until clean.
2. `cargo xtask static`, `cargo xtask records check` (pending record), generators unchanged (`csys-gen`, `abi-gen`,
   `native-gen`, `headers-gen --check`, `compat-gen`, `stubs`: each must leave the tree unchanged).
3. After proof: `bash ~/work/RENAME-NIJI-CODE/proof2.sh niji ~/work/RENAME-NIJI-CODE/after` on the role (it exports
   the Debian keyring `~/keyrings/debian-archive-keyring.gpg`, which lint.sh and swap-system.sh need on keke), then
   `startup.sh` (below) with link dir `~/work/RENAME-NIJI-CODE/pa`; diff `before/` and `after/`: `symbols.txt` after
   `sed 's/NIJI_/LIELLA_/'` on the version column must equal before; `sizes.txt`, `layout-check.txt` numbers equal.
4. Rust dylib `libniji.so`: `cargo xtask rust-dylib` was not in the baseline; build it on both sides (the base tree
   `~/wl/src/e1afc4e47...` is primed on keke) and compare exports modulo crate names.
5. Gate (FULL tier per the brief), plants through `cargo xtask plants`: script rules (mangled length, PROTECT author,
   EXCLUDE results/, CAMEL) with cmd `python3 tools/rename_niji.py --self-test`; `libc-include/features.h` and
   `stdc-predef.h` define -> `cargo test -p xtask niji_macro_always_defined`; `.asciz "libniji"` in
   `csys/nonshared/notice.h` -> `cargo test -p niji-boundary --test s01_startup_symbols s1_2_c_5`;
   dylib namesz -> `cargo test -p niji-native --lib note_follows`.
6. `git grep -i liella` count and kept categories into this note; report json; hand back.

## Baseline (before, origin/next e1afc4e47, keke, jobs `rename-niji-code-before`, `rename-niji-code-before-chroot2`)
Results in keke `~/work/RENAME-NIJI-CODE/before/` (steps.txt, steps2.txt). Coexist: so, build, check PASS;
lintian rc 0 (after the keyring fix). System: so, build, check-system, check-symbols, empty root PASS; lintian rc 0;
**swapped trixie root rc 1 on the base** (look at `before/swap-system.txt` first: a base failure is not the rename's).
Default products: 4278 exported dynamic symbol rows in 13 shared objects (`symbols.txt`), `sizes.txt`, all 12 layout
contracts hold (`layout-check.txt`; STARTUP-DATA-WINDOWS 1 ro window, 7 rw pages). Start-up, neutral program
(`~/work/RENAME-NIJI-CODE/startup.sh`, before products still in role `target/liella`, keep them until measured):
static 6360-6361 instructions:u, 16-18 page faults. Dynamic: NOT MEASURED (the products' ld.so did not find libc.so.6
through `-rpath` to a symlinked dir; try the real path of equal length, or `LD_LIBRARY_PATH` inside the program's
environment only, never perf's).

## Notes
- The role's `target/liella` holds the BEFORE default products; `target/niji` will hold the after ones.
- INFORMATIONAL: compat records (records/compat/*.toml, history, excluded) still name `LIELLA_COMPAT_*` and the
  `liella.` tunable in `restore` text, and the records schema key `liella =` (read by xtask records) is unchanged;
  the generated docs/compat-flags.md follows the records. RENAME-NIJI-DOCS or a records correction should update them.
- `spec/behaviour/compat.glibc.txt` and `legacy-stubs.glibc.txt` keep `/nonexistent-liella` (glibc probe records;
  no test compares those lines).
