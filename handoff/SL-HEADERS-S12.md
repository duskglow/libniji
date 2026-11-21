# SL-HEADERS-S12: slice 12, network core and name services (run 6)

**State.** Branch `task/sl-headers-s12-6`. Say "slice 12" in commits and handoffs. Report: `handoff/reports/SL-HEADERS-S12-6.json`.
Plants: `handoff/SL-HEADERS-S12-5-plants.txt` (4). Decision record: D-X.124.

**Done in run 6.** Merged origin/next (conflicts: `headers/reserved.tsv`, both sides kept; `crates/abi/tests/appendix_c_layouts.rs`
regenerated with `cargo xtask abi-gen`). Regenerated `libc-include/` (`headers-gen`, 186 headers). Clippy (workspace, all
targets) and `cargo xtask static` green on the merged head. `header_check.py`: 8 failed of 12831, all `sys/mount.h` P8.

**Blocker, not slice 12's.** The same 8 failures occur on origin/next itself (11659 checks, 8 failed, same lines). `sys/mount.h`
(slice 15's row, `headers/slices/s15.tsv`) prints `struct winsize` when `__has_include(<sys/ioctl.h>)` is true on the build host:
the host's own `sys/ioctl.h` is found and also defines it, so `struct winsize` is defined twice. Because of this
(a) the direct gate fails only in `headers::slices::s12::tests::network_headers_pass_header_check` (it runs the whole-tree
`header_check.py`); (b) `cargo xtask plants` aborts on its baselines (two of the four plant commands run the whole-tree
check), so no plant has run on this head. Everything else in the gate was green. Fixing it belongs to the owner of sys/mount.h
(a guard on the `winsize` emission or the slice-7 tail); that is outside slice 12 (RULING NEEDED).

**Open.** P5 c89 / `-pedantic` for the slice's headers is not run. The `__has_include` guards for `sys/param.h` and `signal.h`
go away with SL-HEADERS-S07-TAIL (ruling #6406). Kernel-first order is not supported (ruling #6458).

**Things not to redo.** Probe classes accepted in `handoff/SL-HEADERS-S12/accept.py`; the merge resolution; the regenerated headers.

**Next step.** When the sys/mount.h `winsize` double definition is fixed on next, merge next, rerun the direct gate and
`cargo xtask plants handoff/SL-HEADERS-S12-5-plants.txt` (RAM dir new), then land; then slice 13.
