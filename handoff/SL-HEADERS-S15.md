# SL-HEADERS-S15: slice 15, administration, devices, legacy (handoff)

**State.** Branch `task/sl-headers-s15` (from `origin/next`, which holds slices 1-6). Head: see `handoff/reports/SL-HEADERS-S15.json`.
Say "slice 15" in commits and handoffs.

**R-H7.** plan/headers.md §5.1: the whole-archive scan found a hit for each of the 14 undecided legacy headers, so none is
dropped. All five legacy headers of this slice (`sys/vlimit.h`, `sys/timeb.h`, `sys/vm86.h`, `nfs/nfs.h`, `sys/raw.h`) are shipped.
There is no R-H7 drop to land: no differences row and no compat record were written for a drop (none exists).

**Done.**
* 27 templates `headers/<path>.h.in` and their output in `libc-include/` (`headers-gen`: 54 headers, no problems).
* Generator (`xtask/src/headers.rs`): `float`/`double`/`long double` `inf` and `nan` literals (`INFINITY` in `sys/vlimit.h`); an
  enumerator yields to a macro of the same name that came first (kernel UAPI `linux/mount.h` before `sys/mount.h`); an enumeration
  the tables record only by name and size (`enum __vlimit_resource`) gets one placeholder enumerator; opt-in `@@partial-cover`
  (a name an included header makes visible in only some configurations is also emitted: `sys/timex.h`). The enumerator guard
  changed the committed `ctype.h` and `unistd.h` by guard lines only (regenerated).
* `sys/kd.h`, `sys/vt.h`, `nfs/nfs.h` include the kernel's header of the same name and nothing else; `scsi/scsi_ioctl.h` and
  `sys/pci.h` hold no name of the tables.
* `spec/tools/header_check.py`: P5 and P1 over the 27 headers, with the kernel UAPI directories staged; `DEFERRED` lists the
  four names that need slice 7 headers. `header_probe.py` and `callsite_probe.py` stage the UAPI directories too.
* Plants: `handoff/SL-HEADERS-S15/plants.txt` (12).

**Results.** `header_check.py`: 0 failed of 3101 (P5, P1; 40 configurations). P3 (`callsite_probe`, `probe_compare callsites`):
0 differences in 86 rows. P2: 352 differences of 2049 rows, NOT clean (see below).

**Open (P2, not green).** `probe_compare visibility` on the 27 headers (first tree, before the five kernel-only headers were
reduced to the kernel include, which removes their ~70 `extra` rows):
* Names that come from headers of other slices and that the reference's header includes: `sys/ioctl.h` (`TCSETS2`, `TTYDEF_*`),
  `fcntl.h` (`f_owner_ex`), `sys/select.h` (`select`, `pselect`, `fd_set`, `FD_*`, `sigset_t`) for `sys/mount.h`, `sys/mtio.h`,
  `sys/raw.h`, `sys/timex.h`. They are owned in the templates with an optional `__has_include` include; once slices 7 and 14
  land the include is taken and the owned names come from there. Replace `__has_include` by a plain include then.
* `MS_*` of `sys/mount.h` are enumerators in the tables and macros here (the kernel's header comes first): 31 rows of kind
  differing. Needs a differences row and a compat option (not written).
* `sys/types.h` names visible in ours in more configurations than in the reference for `sys/soundcard.h`, `sys/mtio.h`,
  `sys/quota.h`, `sys/acct.h`, `sys/timex.h` (`extra`, 272 rows): the templates include `sys/types.h` unconditionally.
* `be16toh`/`le64toh` kind `differs` (macro vs macro from `endian.h`).
* `sys/vm86.h`: the reference header is rejected (`include-error`) in some configurations; ours is not.

**Not done.** P8 (both include orders of `sys/mount.h` with `linux/mount.h`) was not run beyond the P5/P1 compile with the kernel
header first; the reverse order is not tested. P6 and P10 not run. The differences rows and compat records for the P2
residuals are not written.

**Next step.** Land slice 7 (and 14), swap the optional includes for plain ones, rerun P2; write the differences rows (the `MS_*`
kind, the config-set extras) with compat ids via `cargo xtask records new compat COMPAT-NEW`; P8 reverse order.
