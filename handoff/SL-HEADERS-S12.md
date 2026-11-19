# SL-HEADERS-S12: slice 12, network core and name services (run 5)

**State.** Branch `task/sl-headers-s12-5`. Say "slice 12" in commits and handoffs. Report: `handoff/reports/SL-HEADERS-S12-5.json`.
Plants: `handoff/SL-HEADERS-S12-5-plants.txt` (4). Decision record: D-X.124.

**Done.** Run 5 merged origin/next after HEADERS-GEN-COMPLETE. The slice is data only: the sixteen templates under `headers/`,
`headers/slices/s12.tsv`, `spec/tools/header_checks/s12.py` and the marker module `xtask/src/headers/slices/s12.rs`
(`@@needs`, `@@needs-if`, and new `@@alias NAME OF`). `xtask/src/headers.rs` is the generator's own (no slice-12 edits).
The generator has no alias table, so `headers/aliases.tsv` is gone: `IP_RECVRETOPTS`, `IP_RECVORIGDSTADDR` and
`IPV6_RECVORIGDSTADDR` are spelled by `@@alias` in `headers/netinet/in.h.in` (refused when macros.tsv gives different
type or value). `header_check.py`: 0 failed of 6788 (P1, P5, P8 libc-first, whole tree). Headers regenerated.

**Open.** P5 c89 / `-pedantic` for the slice's headers is not run. `header_check.py --p10` compares against the build host's own
headers (diagnostic only). The `__has_include` guards for `sys/param.h` and `signal.h` go away with SL-HEADERS-S07-TAIL (ruling #6406).
The kernel-first order is not supported (ruling #6458; slice 13 measures it).

**Things not to redo.** Probe classes accepted in `handoff/SL-HEADERS-S12/accept.py`; the merge resolution.

**Next step.** Land; then slice 13.
