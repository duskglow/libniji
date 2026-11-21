# COMPAT19-STARTUP-ZERO handoff

State: branch `task/compat19-startup-zero` (cut from origin/next df83365cd). Role mei:~/w/s4. Brief:
~/state/briefs/COMPAT19-STARTUP-ZERO.md. Leg 1 ended in a refresh (calls budget); leg 2 continues here.

## Design (leg 1, code committed; D-27.66 amended, COMPAT-19 `cost` and #6944 note in the record)
* A normal start reads nothing for COMPAT-19. ld.so's `run_init` reads `EXEC_STACK` only in the `else` of
  `if let Some(f) = early` (a start without libniji's C library, exotic). With libniji's libc, its
  `early_init` (runtime.rs) checks `t.exec_stack == Some(true)` only inside the `GLIBC_TUNABLES`-is-set closure,
  then (cold `exec_stacks_at_start`) walks the start objects with `dl_iterate_phdr` (`asks_exec_stack`, uses
  `niji_ld::map::executable_stack`); when one asks (ld.so accepted it: it refuses otherwise, same `AT_SECURE`
  rule both sides), makes the main stack executable (`thread::exec_main_stack(top, page)`, now shared by ld.so
  and libc; PROT_GROWSDOWN moved to thread.rs) and calls `thread::stacks_executable()`.
* Why not "do it in the loader's PT_GNU_STACK branch" as the brief proposed: at start the branch runs before
  libc is mapped/relocated (program accepted first), so libc's thread flag cannot be set there; any deferred
  action needs a check on the normal path. Also the auxv is not reachable from the start acceptance branch
  without a new store. The tunables closure is the existing branch nearest to "only when opted in".
* Residual cost: +2 instructions (field compare + branch) only in starts that set GLIBC_TUNABLES (any tunable).
* dlopen path unchanged (ld.so's `call_stack_exec` + `exec_main_stack` via the shared helper).

## Bisect of the 1120C +2.5k/start finding (#7046): NOT DONE
* Merges 3f2a84520..b732d9b46: efe95309f (COMPAT19-TUNABLE), 17eda37a8 (LD-SPLIT-3), c47f3d49c (LAYOUT-GOT-ORDER),
  plus HEAP-FOREIGN-PAIR-2, WL-DYNAMIC-KIT, HEADERS-GEN-S07-HOOKS, STDIO-WIDE-BYTE-READS-2, reviews. Sources primed on
  mei `~/wl/src/<sha>` (3f2a845, efe9530, 17eda37, c47f3d4, b732d9b, df83365). My first prep used wl_prep.sh, which
  builds STATIC products only (no libc.so.6): killed, useless. To bisect: per sha `cd ~/wl/src/<sha> &&
  NIJI_FULL_PROFILE=1 cargo xtask so`, copy target/niji to equal-length dirs, count as count.sh does.
  LD-SPLIT-3's own report measured base = head (1 instr), so it is unlikely; LAYOUT-GOT-ORDER and COMPAT19-TUNABLE are
  the candidates. If budget is short: report it as not attributed (INFORMATIONAL) with the plan above.

## Jobs (mei)
* SUPERSEDED: compat19-startup-zero-prep (static-only products, killed), compat19-startup-zero-check (killed when its
  ssh was ended), compat19-startup-zero-count1 (waited on the old check log; killed).
* compat19-startup-zero-check2 (started ~03:36Z): ~/work/compat19-startup-zero/check.sh = clippy -D warnings, xtask
  static, NIJI_FULL_PROFILE=1 xtask so, copies target/niji to ~/work/compat19-startup-zero/head/niji, then
  `cargo test -p niji-ld --test exec_stack`.
* compat19-startup-zero-count2: ~/work/compat19-startup-zero/count.sh 2 101: builds base df83365 full profile in
  ~/wl/src/df83..., waits for check2's tests to start, counts base vs head (pie0, pie3, time, static st0), lines
  `COUNT base|head <prog> ...` in ~/logs/compat19-startup-zero-count2.log.

## Next step (leg 2)
1. `~/bin/jobwait compat19-startup-zero-check2 compat19-startup-zero-count2`; `~/bin/test-summary ~/logs/...check2.log`.
   Red: fix, commit, sync, rerun only that part. Count: dynamic pie0 should drop by ~2 (the flag), no row worse.
2. Plants: handoff/COMPAT19-STARTUP-ZERO-plants.txt (5 plants) via `cargo xtask plants <file> --dir
   /mnt/zram/claude/compat19-startup-zero/plants-1 --timeout 600` under jobrun (label compat19-startup-zero-plants).
   The brief's count plant (flag read put back in run_init: `} else if EXEC_STACK.load(Relaxed) {` ->
   `} if EXEC_STACK.load(Relaxed) {` in crates/boundary/src/ld/initfini.rs) is caught by the count, not a test: run
   count.sh against a planted head build once, or report it with numbers.
3. Start-up checks: `cargo xtask test`, `NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji
   --test hot_text_products`; diff target/product/fast/layout-bins.tsv vs base (§9).
4. `cargo xtask affected --list` (crates/rt changed: may escalate to FULL -> gate on precheck + crate tests).
5. Report json handoff/reports/COMPAT19-STARTUP-ZERO.json, precheck, handback-check, hand back.
