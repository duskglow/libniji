# VALGRIND-LOADER-VIEW: full leak stacks through libc under valgrind, and helgrind (GitHub #13, the last of it)

Branch `task/valgrind-loader-view` (from origin/next d5764940d). Brief `~/state/briefs/VALGRIND-LOADER-VIEW.md`.
Role keke `~/w/n3` (Zen 1 host). Job scripts: keke `~/work/VALGRIND-LOADER-VIEW/` (local copies in
`~/w/.scratch/VALGRIND-LOADER-VIEW/job/`: `vgacc.sh DEBDIR LOGDIR [post.sh|diag.sh]`, `diag0.sh`, `vg/` with
`pre.sh`, `post.sh`, `diag.sh`, `hg.sh`, `vg.sh` and the probes `bad.c cxx.cc leak.c thr.c race.c`).
Valgrind 3.24.0 sources (allowed by the brief) extracted in `~/w/.scratch/VALGRIND-LOADER-VIEW/vgsrc/`.

## State (leg 1 ended by REFRESH at ~160k context)

* Commit 3f1ac8b16 (WIP, NOT built, NOT tested): item 1, the valgrind-only hidden view (option (b)).
* Job `valgrind-loader-view-diag0` (keke, started 18:01Z on origin/next = the BASE, before the commit): builds the
  system debs into `~/work/VALGRIND-LOADER-VIEW/debs-base/`, then runs the swapped-root diagnosis
  (`vg/diag.sh`: the leak record, helgrind on thr and race, helgrind's pthread_create/join redirections,
  stock beside). Output: `~/logs/valgrind-loader-view-diag0.log`, files in `~/work/VALGRIND-LOADER-VIEW/diag0/out/`.
  The role is BUSY until it ends (sync_role rc 4).

## Item 1: the whole-file view (cause from VALGRIND-DBG-RODATA; mechanism read in valgrind 3.24's source)

* `debuginfo.c` `di_notify_mmap`: a mapping that is not r/rw/rx (PROT_NONE) returns early, not recorded;
  `di_notify_mprotect` does nothing; `munmap` does not drop a pending mapping. `readelf.c` (~line 2141): each
  PT_LOAD is matched against every recorded mapping whose file range covers it; the view (offset 0, the whole
  file, `ro`) covers every r-- PT_LOAD, and it is recorded first.
* So "unmap the view early" does not help; PROT_NONE + `mprotect(PROT_READ)` does, at +1 system call per view.
  Option (a) (unconditional) adds one `mprotect` per loaded object to every start: the bounds in
  `crates/core/tests/hot_text_products.rs` `init_dynamic_start_system_calls_stay_within_their_bounds` (empty 23,
  three 50: only go down) would have to loosen, so (a) "costs measurably" by construction (empty +1, three +4,
  dlopen +1 per object). NOT yet measured in instructions/faults: the brief asks for the start-up table of (a);
  measure it by forcing `views_hidden()` true in a scratch copy (or a plant-style copy) and running the
  empty/three/dlopen rows, or state why the syscall count alone settles it.
* Implemented (b): `csys/src/machine/tool.S` `niji_machine_running_on_valgrind` (valgrind.h's documented amd64
  sequence, request 0x1001), declared in `ffi.rs`; `image.rs`: `map_view` asks `views_hidden()` (one cached
  AtomicU8, the request asked once in `#[cold] views_hidden_ask`), the hot path is `map_view_as(.., PROT_READ)`
  (`#[inline(always)]`), the hidden path `#[cold] map_view_hidden(fd, len, reveal)` maps PROT_NONE then calls
  `reveal` (mprotect). `map_view_alone` takes `prot`. Unit test
  `hidden_view_is_inaccessible_until_revealed_and_the_bare_machine_does_not_hide` (image.rs `mod tests`) reads
  `/proc/self/maps` inside `reveal`: `---p` at map time, `r--p` after, nothing after a failed reveal.
* To do: build + clippy + static; `cargo test -p niji-boundary --lib hidden_view`; the `.S` comment says `D-NEW`:
  reserve an id (`ops reserve --story VALGRIND-LOADER-VIEW D-X`) and write the record (plan/07 or records/pending);
  the layout-bins diff (§9); the acceptance (leak record names main).

## Item 2: helgrind (cause found, fix designed, NOT implemented)

* helgrind's `hg_intercepts.c` (3.24) on Linux wraps `pthread_create@*` only (`pthreadZucreateZAZa`) but
  `pthread_join` by its plain name (`pthreadZujoin`). valgrind matches names literally (m_redir.c
  `VG_(string_match)`) and never derives `@VERSION` from `.gnu.version` (no versym code anywhere in coregrind):
  a versioned name exists only if the symbol table spells it.
* libniji's libc6-dbg `.symtab` (from mei's VALGRIND-DBG-RODATA debs): `pthread_create@GLIBC_2.2.5` (the 5-byte
  compat stub, `.symver` in the generated compat_aliases.S) and plain `pthread_create` (the default, versioned
  by the version script only, so no `@@GLIBC_2.34` in `.symtab`). helgrind wraps the stub, programs call the
  default unwrapped; `pthread_join` IS wrapped, so JOIN_POST names a thread helgrind never registered (assert).
  420 `@` names in that `.symtab`; same for every default with a compat sibling (pthread_cond_* too).
* Fix design: give the default of every name that also has a compat version its `name@@VERSION` name in
  libc.so.6's `.symtab` (what `.symver name@@V` gives; size kept). `objcopy --add-symbol` is useless (size 0:
  valgrind readelf.c:483 ignores size-0 symbols), so RENAME: `objcopy --redefine-sym name=name@@VER` on the
  linked libc.so.6 (`.dynsym` untouched; check `readelf --dyn-syms` identical before/after and loaded bytes
  unchanged). Place it in xtask/src/so.rs after `link_libc` + checks (~line 1822-1845, before `publish`).
  RISK: the layout contracts (`crate::layout::check_products`, line ~1953) and tests may look up `.symtab`
  names like `memcpy` that would become `memcpy@@GLIBC_2.14`: check what reads names (grep the layout/hottext
  code for symtab lookups) and strip `@...` there, or rename only in the packaged debug file
  (tools/deb/system.sh, after `objcopy --only-keep-debug`) if the products' `.symtab` must keep plain names.
  Add a so.rs check (every default with a compat sibling is `name@@VER` in `.symtab`) + plant (rename skipped).
* Confirm with diag0's `hg-stock-redir` vs `hg-niji-redir` lines (which names helgrind redirected).

## Exact next steps

1. `~/bin/jobwait valgrind-loader-view-diag0` on keke; read `~/logs/valgrind-loader-view-diag0.log` (tail).
2. Sync (`sync_role.sh --story VALGRIND-LOADER-VIEW --base origin/next <wt> keke n3`, check rc), build, clippy,
   static, the unit test.
3. Implement item 2 as above; D-X id for both; plants file `handoff/VALGRIND-LOADER-VIEW-plants.txt` (view kept
   PROT_READ in `map_view_hidden` -> unit test; rename skipped -> so.rs check/test).
4. Debs from the head (diag0.sh's first half) + `vgacc.sh <debs> <dir> post.sh` (add hg.sh judging: race
   reported at race.c, no assert); gate (`cargo xtask affected --run`), plants, start-up checks (§9), precheck.
