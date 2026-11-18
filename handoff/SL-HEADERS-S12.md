# SL-HEADERS-S12: slice 12, network core and name services (run 3)

**State.** Branch `task/sl-headers-s12-3` (on `task/sl-headers-s12-2`'s work). Say "slice 12" in commits and handoffs.
Report: `handoff/reports/SL-HEADERS-S12-3.json`. Plants: `handoff/SL-HEADERS-S12/plants.txt` (7, stale output),
`handoff/SL-HEADERS-S12-2-plants.txt` (4, P10 through the script), `handoff/SL-HEADERS-S12-3-plants.txt` (4, alias macros and
the header_check cargo test). Decision record: D-X.124.

**Done.**
* P1, P5 (2687 units), P2, P3, P10: clean (runs 2).
* P8 libc-first: `headers/aliases.tsv` (new) lists macros headers-gen spells as another macro (`IP_RECVRETOPTS`,
  `IP_RECVORIGDSTADDR`, `IPV6_RECVORIGDSTADDR`), as `linux/in.h` does; the generator refuses a row whose two table values
  differ. `header_check.py --kernel DIR` (DIR holds `linux`, `asm-generic`, `asm` from linux-libc-dev): 0 failed of 2702. The
  kernel-first order is not supported (ruling #6458; slice 13 measures it).
* `struct res_sym` rows: `cargo xtask abi-gen` output (`crates/abi/src/layout.rs`, `crates/abi/tests/appendix_c_layouts.rs`)
  regenerated; this was the cause of the FULL gate's red at `stubs` / `cargo xtask test`.
* D-X.124 (decision record), differences row for `bindresvport6` (`s15_8_c_2`).
* xtask tests `alias_macros_are_emitted_by_name_and_must_agree` and `network_headers_pass_header_check` (header_check P1/P5,
  P8 where the host has `/usr/include/linux` and the multiarch `asm`, P10 where it has `/usr/include/netinet/in.h`; a skip prints why).

**Open.** P5 c89 / `-pedantic` for the slice's headers is not run. `header_check.py --p10` compares against the build host's own
headers (diagnostic only, never read as text). The `__has_include` guards for `sys/param.h` and `signal.h` go away with
SL-HEADERS-S07-TAIL (ruling #6406).

**Things not to redo.** Probe classes accepted in `handoff/SL-HEADERS-S12/accept.py`; template fixes of run 2.

**Next step.** Read the gate and plants rows (labels in the report); if red, fix the cause. Then slice 13 (kernel-first measurement is
in its brief).
