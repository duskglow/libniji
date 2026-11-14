# SL-HEADERS-15: administration, devices, legacy

Id: SL-HEADERS-15
Kind: code
Priority: 90
Size: S
Host: -
Stacks-on: SL-HEADERS-6, ruling R-H7
Model: sonnet
Effort: medium
BUDGET: 60 tool calls / 120k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 15; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

The headers below. The legacy ones follow ruling R-H7.

Headers (plan/headers-inventory.tsv, slice 15): `mntent.h`, `fstab.h`, `sys/mount.h`, `sys/swap.h`, `sys/reboot.h`, `sys/klog.h`, `sys/sysinfo.h`, `sys/quota.h`, `sys/acct.h`, `sys/io.h`, `sys/perm.h`, `sys/vlimit.h`, `ulimit.h`, `gnu/libc-version.h`, `sys/timeb.h`, `sys/timex.h`, `sys/kd.h`, `sys/vt.h`, `sys/soundcard.h`, `sys/mtio.h`, `sys/pci.h`, `sys/raw.h`, `scsi/scsi.h`, `scsi/scsi_ioctl.h`, `scsi/sg.h`, `nfs/nfs.h`, `sys/vm86.h`

## Proof

P1 (generated `_Static_assert` TUs, C and C++), P2 (visibility equals `visibility.tsv` for every configuration), P3 (call sites equal `callsites.tsv`), P5 (hygiene) for every header of the slice; P9: plants through `cargo xtask plants` on the new templates and table rows. P8 for `sys/mount.h` with `linux/mount.h`.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/.

JOBS: gate, plants
