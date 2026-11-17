# LD-SPLIT: the loader's ld.rs split into modules by concern (run 1, checkpoint)

**State.** Branch `task/ld-split` from origin/next `30d07e74b` (contains SYS-COMPAT-VERSIONS-REST; does NOT
contain DL-TRACE-INTERP: it went red in CI precheck at 03:43, static `old-name`, after this run was admitted).
Code head `a0a5326e8`. Role keke `~/w/n1`. Run 1 stopped at the context checkpoint (~150k); split done and
linted, proofs not run yet.

## Module map (ld.rs keeps the product glue: panicked, NoHeap, ld_mem, the re-exports)

| module | owns |
|---|---|
| `ld/io.rs` | write_all, exit, stack_word, c_string, refuse, lo32, fail_with, fail_loading |
| `ld/state.rs` | shared start-time state: OWN_BASE, OBJECTS_MAX, START_OBJS/N/INTERP/PAGE/ARGS, AUXV_WORDS_MAX, PHDR_WORDS |
| `ld/tls.rs` | provisional thread block, static TLS + surplus, module table (THREAD_MODS...), dtv, generation, `__tls_get_addr` |
| `ld/image.rs` | open/identify files, image views, own image, SysMapper, ProcImage, cache file, search candidates |
| `ld/initfini.rs` | run_init (constructors), FINI_* hooks, fini_all |
| `ld/link.rs` | program start's link: DT_NEEDED search, preloads (Origin, Preloads), mapping, relocation, binding |
| `ld/trace.rs` | the dependency list (`--list`): list, Obj, hex16, LIST_OBJECTS_MAX, PATHS_MAX |
| `ld/entry.rs` | niji_ld_start, run, auxv/env, secure clear, rseq, nonshared report, List |

dlopen/dlclose/exit-set finalisers stay in `ld/dlopen.rs` (2,082 lines, untouched); dladdr/dlsym are already in
`ld/dlopen/`. Public paths unchanged: `pub use` of niji_ld_start, __rseq_*, __libc_stack_end, __tls_get_addr;
`pub(crate) use` of write_all, exit, stack_word, c_string (ldconfig.rs uses them). dlopen.rs and readers.rs
reach moved items through private `use` lines in ld.rs, so neither file changed.

## Commits (generated, not hand-edited)

`~/w/.scratch/LD-SPLIT/work/split.py` (stage k = first k modules moved; item lists, visibility fix-ups, import
drops in the script) and `commit_stages.sh`. Commits: io 9fd163595, state 1311961f8, tls bf859fbd6, image
e75ae58a6, initfini 9fe5e7da1, link 2e4743f92, trace c4f74399a, entry 539edadd3, fix a0a5326e8. The entry
commit alone left link.rs/trace.rs importing `super::List` (fixed by a0a5326e8, use lines only): 539edadd3
does not compile; stages 1-7 were NOT VERIFIED to compile one by one.

## Verified

- keke `ld-split-check-4` rc 0 on the a0a5326e8 tree: clippy -D warnings of the ld.so product (feature `ld`)
  and of niji-boundary with all features. The two warnings printed (heap.rs c_malloc, hot/hyperbolic.rs
  hyp_quick) are outside ld, in the ld-only configuration.

## Not done (next run, in order)

1. Audits: DONE in 0302a123d: policy.rs `export` category now names `src/ld/entry.rs`, `src/ld/tls.rs` (the 6
   moved export sites); `ops handback-check` on 0302a123d ok (static + clippy merged with next). Still to do:
   print the unsafe-audit counts on base and branch and show them equal. Paths in spec/behaviour/dynlink-preload.md:71, records D-27.18/.19
   (text only), tb.rs:298 doc, xtask/src/affected.rs:4432 test list (add an ld/ file).
2. Module-map `//!` doc in ld.rs (the old header still describes DL-5 only).
3. Proofs vs base copy keke `~/work/LD-SPLIT/base` (origin/next tree, no target): exported dyn-syms of every
   product (readelf --dyn-syms minus value), `cargo xtask bench --counters cbench_startup` both sides,
   `cargo xtask test` (full), `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p liella
   --test hot_text_products` (the brief says LIELLA_; the variable is NIJI_FULL_PROFILE), swap root via a copy of
   keke `~/work/SYS-LIBRESOLV-2/deb.sh` (build-system, check-symbols, swap-trixie; expect 54/7 both sides),
   loader plants, and the `git merge-tree` demo (tls.rs vs dlopen.rs).
4. Decide with the coordinator whether to wait for DL-TRACE-INTERP (it edits ld.rs: it will conflict with this
   branch, or this with it); the generator re-runs on a new base in seconds (item lists by name).

## Also look (brief step 5): not examined yet.
