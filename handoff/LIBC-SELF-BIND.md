# LIBC-SELF-BIND: handoff (leg 1 -> leg 2)

State: branch task/libc-self-bind, head 8a686485b (pushed; the handoff commit follows). Role keke:~/w/n1 synced
at 8a686485b (tree 295e0775a). Decision D-27.76 (reserved, records/decisions/D-27/D-27.76.toml). Ruling #7401
(the "keep" variant) is in force.

## Done
* xtask/src/so.rs: libc.so.6 (and the libc-owner.so link-time stub, same closure) links with
  `--dynamic-list=<product>/libc-preemptible.list --dynamic-list-data`. `LIBC_PREEMPTIBLE` = malloc, free, calloc,
  realloc, aligned_alloc, posix_memalign, memalign, valloc, pvalloc, malloc_usable_size, reallocarray. The build check
  `check_self_binding` (pure `self_binding_errors`, unit test
  `libc_self_binding_keeps_only_the_malloc_family_and_data_symbolic`) fails the build if any other own FUNC/IFUNC
  still has a symbolic relocation, or if no data reference is left symbolic.
* ld 2.42 semantics checked on a toy object (keke ~/work/libc-self-bind/ldsem): unlisted functions bind (GOT ones
  disappear), listed malloc keeps JUMP_SLOT, data keeps GLOB_DAT. An IFUNC reached through the PLT keeps a
  JUMP_SLOT (it becomes IRELATIVE only with -fno-plt). libc.so.6 has no IFUNC self-references today, so the check
  passes, and it would flag one if it appeared.
* Baseline analysis (next 4f32aa946, fast products; keke ~/work/libc-self-bind/base.txt, selfrefs.py): 288 symbolic
  relocations, 285 names, 277 own: 206 JUMP_SLOT + 34 GLOB_DAT to functions, 37 GLOB_DAT + 3 R_X86_64_64 to data,
  8 undefined. Function callers: mostly compat or alias stubs (`x@GLIBC_old` -> x, `_IO_*`, llseek, pthread_yield,
  __secure_getenv, __dn_*); the heap's entry-point table (`Caller::new` -> `entry_points()`, mtrace frame skipping:
  the whole malloc family plus `__libc_*` and `niji_heap_*` by address); memcmp/bcmp from Rust code (setlocale,
  gettext catalogs, getaddrinfo, nss netgroup, select); fwrite/ftell/fflush from the XDR stdio stream. Baseline
  products saved at keke ~/work/libc-self-bind/base-niji/.
* conformance/c/x_self_bind.c + helpers/sbind_lib.c (dynamic mode). **Reference (host glibc 2.39) run: passes;
  every family reads 0** (pthread, aio, sem, mq, clock, timer, C11, dl, ns, dn, llseek, pthread_yield,
  __secure_getenv, _IO_fputs, openpty, memcmp/bcmp users, XDR fwrite/ftell/fflush); the malloc family serves
  strdup/fopen/fclose/asprintf/getline. The program has COPY relocs of stdout, optind and opterr. Not yet run on
  libniji (no products of the change built yet).
* Plants file handoff/LIBC-SELF-BIND-plants.txt (5 plants).

## Jobs started (keke)
* `libc-self-bind-plants` (cargo xtask plants, --dir /dev/shm/claude/libc-self-bind/plants-1), on 8a686485b.
* `libc-self-bind-list` (cargo xtask affected --list --base origin/next): read ~/logs/libc-self-bind-list.log. If it
  says `FULL (escalated`, do NOT run a gate: run precheck plus the changed tests as kind `test` and write
  `FULL: the train's (xtask change)` in the hand-back.
* SUPERSEDED: libc-self-bind-so0 (baseline build of next, rc 0, for the analysis), libc-self-bind-so1 (killed while
  queued, 35 min without a slot).
* The slots were full all of leg 1 (wl-levels-rerecord k1-k5, CI instr): expect queue waits.

## Next step (exact)
1. `jobwait libc-self-bind-list`; choose the gate (direct tier) or the FULL route above. Run it on the role
   (it builds the products). Then run the probe on libniji:
   `cargo xtask affected --only conformance:x_self_bind --with-exports --run` (must pass), and check
   `readelf -rW target/niji/libc.so.6`: about 48 symbolic relocations left.
2. Measure: the empty main and 3 libraries (execcount, median of 101) on the products against baseline
   base-niji/ and the reference. Must prove: empty main <= 400k, 3 libraries <= 560k. Also
   `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products`: it prints
   the measured empty main. Then tighten `DYN_START_INSTR` in crates/core/tests/hot_text_products.rs to the measured
   level + 2% (ALWAYS FORWARD) with a doc line (LIBC-SELF-BIND, D-27.76). The Skylake-SP class is not measured
   (only the keke role): either take the same bound by the earlier within-0.02% precedent (say so), or ask.
3. Start-up checks (§9): `cargo xtask test` and the FULL_PROFILE pair, in JOBS. Diff
   target/product/fast/layout-bins.tsv against the base (the libc text should not change, only relocations).
4. clippy (foreground) + `cargo xtask static` + `cargo xtask records check`, then precheck on the final head,
   report json handoff/reports/LIBC-SELF-BIND.json, handback-check, hand-back (first line with per-start numbers).
