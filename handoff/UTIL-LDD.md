# UTIL-LDD (UTIL-4): ldd

**State.** Branch `task/util-ldd-3` = UTIL-LDD-2's `task/util-ldd-2` (`6a3a047ea`) plus the merge of origin/next
`9bcf93146` (code head); this note and `handoff/reports/UTIL-LDD-3.json` are the commit after it. Role mei `~/w/s1`;
the swap roots run on keke (mmdebstrap, the mirror keyrings) from `~/work/UTIL-LDD-3/` (`swap.sh`, the debs, the
branch's tools/deb copies).

## UTIL-LDD-3 (merge only; ldd unchanged)

* CI could not merge `tools/deb/swap-system.sh`: next added apt (apt-1..3, DL-LIBSTDCXX-4) to the swap-root
  command set. The only conflict was that script's header comment naming the command set; it now names both (apt,
  and ldd plus copy_exec). `swap-cmds.sh` merged cleanly with the ldd, copy_exec, apt and fakeroot cases; the
  mmdebstrap include list keeps fakeroot and initramfs-tools-core. Nothing else changed.
* Every job reran on `9bcf93146` (see Results). The swap job now has a verdict that can pass: `swap.sh` (keke
  `~/work/UTIL-LDD-3/swap.sh`, not in the tree) requires every command but ldd-1..7 to PASS (the swap proof,
  apt, fakeroot, copy_exec, ldd-8 included) and ldd-1..7 to be identical once both sides are normalised for the
  three loader differences of ruling #6056: (a) `/usr/lib/x86_64-linux-gnu/` -> `/lib/x86_64-linux-gnu/`, (b) the
  bare interpreter name for a library operand -> `/lib64/...`, (c) `-v` blocks of the replaced libraries
  (libc.so.6, libm.so.6) dropped on both sides. (c) is wider than UTIL-LDD-2 wrote: ldd-7 (`-v` python3) also
  differs in libm.so.6's block, because Liella's libm has its own `DT_VERNEED` (it needs only `GLIBC_2.14` of libc,
  none of the loader); ldd prints what each object records, so this is the library, not ldd.

## On the branch (from UTIL-LDD and UTIL-LDD-2)

* `csys/util/ldd.c`: ldd as a static PIE over `libc.a`, listing through Liella's loader in trace mode (D-31.7);
  `-v` from `DT_VERNEED`; `-d/-r/-u` refused until DL-10. Interpreter line kept when a listed object names the
  interpreter in `DT_NEEDED` or `libc.so.6` is listed (interim filter, ruling #6043; D-31.15).
* `validation/transcripts/ldd/` (28), `docs/transcripts.md`; `tools/deb/swap-cmds.sh` ldd-1..8 and copyexec-1;
  `tools/deb/check-system.sh` allows `/usr/bin/getent` and `./usr/bin/`. Plants `handoff/UTIL-LDD-2-plants.txt` (12).

## Results (jobs on `9bcf93146`)

* mei: `util-ldd-3-lint-0` rc 0 (clippy + static on the inherited `6a3a047ea`, before the merge);
  `util-ldd-3-debs-1` rc 0 (system .debs, check-system PASS); `util-ldd-3-lint-1` rc 0 (xtask clippy, workspace
  clippy --all-features -D warnings, static, records check); `util-ldd-3-transcripts-1` rc 0 (ldd, getent,
  getconf, ldconfig: 120 ok, 0 mismatched); `util-ldd-3-gate-1` (direct tier, --base origin/next): rc 0, 32 suites ok, 12015 passed, 0 failed, conformance 137 passed, peak 16G (page cache) oom_kill 0;
  `util-ldd-3-plants-1`: rc 0, 12 plants: 11 CAUGHT, 0 SURVIVED, 1 WARN-NO-MSG (`ldd-several-header`: the mutant does not build under -Werror, `several` becomes unused; goes to DL-TRACE-INTERP per ruling #6056).
* keke `util-ldd-3-swap-1` rc 0: same=54 differ=7, the 7 being ldd-1..7, each KNOWN (a)-(c) only; apt-1..3,
  fakeroot-1/2, copyexec-1, ldd-8 PASS.

## Open (the loader, not ldd: DL-TRACE-INTERP, ruling #6056)

1. Loader trace paths `/usr/lib/x86_64-linux-gnu/` vs the reference's `/lib/x86_64-linux-gnu/`.
2. Bare interpreter name for a library operand (swap ldd-4).
3. `-v`: libc.so.6's block absent (no version needs on the loader); libm.so.6's block lists Liella's libm's needs.
4. Then remove ldd's interim filter, take over the `ldd-several-header` plant and the `-v` differences row.

Next step: none for ldd; land it. DL-TRACE-INTERP takes 1-4 (its swap check can reuse `swap.sh`'s normalisation
as the list of what must disappear). Read: spec section 31.3, D-31.7, D-31.15, this note.

## Rule slips (no effect on the tree)

UTIL-LDD-2: one `sed -i` on /dev/null (failed, nothing edited); one oversized `ls`. UTIL-LDD-3: one local
here-doc (`python3 - <<EOF`) to patch the scratch swap.sh (outside the tree).
