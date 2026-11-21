# SL-HEADERS-S08: slice 8, signals, threads, contexts (handoff after run 6, SL-HEADERS-S08-6, data-only rebase)

**State.** Branch `task/sl-headers-s08-6`. Head: see `handoff/reports/SL-HEADERS-S08-6.json`. Say "slice 8" in commits.

**Done in run 6.**
* Merged origin/next after HEADERS-GEN-S08-HOOKS landed. The branch has no diff under `xtask/`, `spec/tools/` or `tools/`: next's versions were
  taken whole (function-pointer members, `LANGUAGE_MODE`, `union_tag`/`cxx_mask`, the conditional include are next's).
* Data kept: slice 8's rows in `spec/abi/layouts.tsv`, `typedefs.tsv`, `visibility.tsv`; `headers/reserved.tsv` (both lists); the three
  slice 8 differences rows (both sides kept); `headers/fnmembers.tsv` = next's `struct argp` row plus slice 8's five rows
  (`struct sigaction`, `struct sigevent`, `sigevent_t`, `struct _pthread_cleanup_buffer`).
* `macros.tsv` conflicts: next's typed pointer constants taken (`void (*)(int)` SIG_*, `void*` RTLD_*, `char*` MM_NULL*, ...);
  `headers/fmtmsg.h.in` is next's (its `@@value` already carries the cast; slice 8's own cast produced a doubled cast and is dropped).
* Regenerated with `cargo xtask headers-gen` (145 headers; `abi-gen` rewrote `crates/abi/src/layout.rs` and `appendix_c_layouts.rs`),
  second run stable. clippy and `cargo xtask static` clean on the role.

**Not to redo.** Nothing generic in the generator; if another gap shows up it is a RULING NEEDED, not a slice edit.

**Open.** `header_check.py`: 8 failed of 9785, all P8 `sys/mount.h` vs `linux/mount.h` ("redefinition of struct winsize"). `libc-include/sys/mount.h`
and `sys/ioctl.h` are byte-identical to next's, so this is not slice 8's (slice 15's header; not chased). Plants of slice 8:
`handoff/SL-HEADERS-S08-4-plants.txt` (see the report json for the run's result).

**Next step.** The train's gate and landing.
