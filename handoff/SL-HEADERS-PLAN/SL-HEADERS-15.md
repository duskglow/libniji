# SL-HEADERS-15: administration, devices, legacy

Id: SL-HEADERS-15
Kind: code
Priority: 90
Size: S
Host: -
Stacks-on: SL-HEADERS-1, SL-HEADERS-6
Model: sonnet
Effort: medium
BUDGET: 60 tool calls / 120k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 15; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below. This slice also owns every R-H7 drop (plan/headers.md §5.1), whichever slice listed the header: one row in `docs/differences-from-glibc.md` per dropped header, and one registered compat record (`cargo xtask records new compat COMPAT-NEW`; restore = install the dropped headers, status registered, not implemented) naming the header decision. `headers-gen --check` fails if a dropped header is generated.

Headers (plan/headers-inventory.tsv, slice 15): `mntent.h`, `fstab.h`, `sys/mount.h`, `sys/swap.h`, `sys/reboot.h`, `sys/klog.h`, `sys/sysinfo.h`, `sys/quota.h`, `sys/acct.h`, `sys/io.h`, `sys/perm.h`, `sys/vlimit.h`, `ulimit.h`, `gnu/libc-version.h`, `sys/timeb.h`, `sys/timex.h`, `sys/kd.h`, `sys/vt.h`, `sys/soundcard.h`, `sys/mtio.h`, `sys/pci.h`, `sys/raw.h`, `scsi/scsi.h`, `scsi/scsi_ioctl.h`, `scsi/sg.h`, `nfs/nfs.h`, `sys/vm86.h`

Legacy headers of this slice (`sys/vlimit.h`, `sys/timeb.h`, `sys/vm86.h`, `nfs/nfs.h`, `sys/raw.h`): generate one only when plan/headers.md §5.1 lists it as shipped; leave the others out (their differences rows and the compat record belong to SL-HEADERS-15).

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P8 for `sys/mount.h` with `linux/mount.h`.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
