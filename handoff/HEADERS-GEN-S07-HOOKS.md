# HEADERS-GEN-S07-HOOKS

State: branch task/headers-gen-s07-hooks, five commits on origin/next 189ab9cbc (four code commits plus this note and report).

Commits: (1) lfs redirections in headers-gen + regenerated stdio.h, stdlib.h, sys/pidfd.h, unistd.h; (2) no `__packed__` on a
structure of bytes; (3) smoke `--niji-headers` and `undef-compare.sh`; (4) abi_extract.py (utime.h, struct statx_timestamp,
`--prototypes-only`). Each rule has a unit test in xtask/src/headers.rs and a plant in handoff/HEADERS-GEN-S07-HOOKS/plants.txt.

Not done on purpose: brief item 2's enumeration tag (c3c4a479) needs no code on next (see the report). No slice 7 table rows.
`headers-gen --check` is NOT clean against the old libc-include: the four headers changed (listed in commit 1's body).

Next: SL-HEADERS-S07 rebases on this branch; its libc-include for those four headers will already carry the redirections.
Smoke programs were not run here (no smoke job in the brief); `--niji-headers` and `undef-compare.sh` are copied unchanged from slice 7.
