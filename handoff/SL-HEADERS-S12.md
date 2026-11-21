# SL-HEADERS-S12: slice 12, network core and name services (run 7)

**State.** Branch `task/sl-headers-s12-7` (continues run 6). Say "slice 12" in commits and handoffs. Report:
`handoff/reports/SL-HEADERS-S12-7.json`. Plants: `handoff/SL-HEADERS-S12-7-plants.txt` (results of the 4 plants of
`handoff/SL-HEADERS-S12-5-plants.txt`). Decision record: D-X.124.

**Done in run 7.** HEADERS-WINSIZE-TWICE had landed on next, so the run-6 blocker is gone. Merged origin/next (no
conflicts); `headers-gen` and `abi-gen` on the build host reproduce the committed `libc-include/` and
`appendix_c_layouts*.rs` byte for byte (no regeneration diff). Clippy (workspace, all targets, all features) and
`cargo xtask static` green. Direct-tier gate (`affected --base origin/next --run`): 18 suites ok, 11564 tests passed, 0
failed, 93 conformance passed, including `network_headers_pass_header_check`. Plants: 3 CAUGHT, 0 SURVIVED, 1
WARN-NO-MSG (`s12-needs-if-include`: caught by `network_headers_pass_header_check`, but the log does not contain the
`msg:` text `stale under libc-include`; the msg regex wants fixing to the test name). P5 for the 16 slice 12 headers,
each included twice, under `-std=c89|c99|c11|c17|c2x -pedantic-errors -Wall -Wextra -Werror`: all 80 compile clean
(one-off script, not committed).

**Open.** The `__has_include` guards for `sys/param.h` and `signal.h` go away with SL-HEADERS-S07-TAIL (ruling #6406).
Kernel-first order is not supported (ruling #6458). The strict-mode namespace check of P5 is not part of this slice.

**Things not to redo.** Probe classes in `handoff/SL-HEADERS-S12/accept.py`; merge resolution; regenerated headers; P5.

**Next step.** Land slice 12, then slice 13.
