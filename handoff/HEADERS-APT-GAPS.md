# HEADERS-APT-GAPS: C++ sockaddr arguments, siginfo_t's si_syscall family, <arpa/nameser.h> HEADER (apt 3.0.4)

**State:** branch `task/headers-apt-gaps` (from origin/next 522900c6f), role keke:~/w/n2. Code head 1bbdd9ea6,
plants file 3d4e7ca4d (later commits: handoff/report only). Decisions: none new; every change follows the reference's
probed visibility, type, layout and value (no differences row, no compat option).

## What was done
* **C++ socket calls** (`xtask/src/headers/cond.rs` `union_tag`): the generator took the transparent unions
  `__SOCKADDR_ARG` / `__CONST_SOCKADDR_ARG` for C++ union tags (a union typedef with no C++ spelling) and added every
  C++ configuration to the typedef's condition; g++ defines `_GNU_SOURCE`, so C++ got the union and a
  `struct sockaddr *` or `0` did not convert. Transparent unions are now excluded: C++ gets the pointer macro
  (visibility.tsv's rows), C is unchanged. A decltype probe of the eight functions (accept, accept4, bind, connect,
  getpeername, getsockname, recvfrom, sendto) gives the reference's types in C++98 and C++17 (C++17's noexcept
  pattern included).
* **siginfo_t** (`headers/signal.h.in`, `headers/sys/pidfd.h.in`): `si_call_addr`, `si_syscall`, `si_arch`,
  `si_lower`, `si_upper`, `si_pkey`. header_probe.py `--names` rows (48) equal `si_status`'s in all eight headers, so
  they follow its pattern (`@@owns-macro`, `@@when <name>`); offsets/sizes (16/8, 24/4, 28/4, 32/8, 40/8, 32/4) are
  the reference's (offsetof probe) and layouts.tsv's.
* **HEADER**: visibility rows (arpa/nameser.h, arpa/nameser_compat.h, resolv.h; `!time64only`), 16 layouts.tsv rows
  from `abi_extract.py --layouts-only` (HEADER added to its type list; bit positions equal a black-box bit probe),
  typedefs.tsv row from `type_probe.py` (12, 4, C++ linkage name `HEADER`). Emitted in nameser_compat.h.
* Tests: header_check s08 (six names exact in signal.h, sys/signal.h, sys/wait.h, sys/pidfd.h, plus offsets/sizes)
  and s12 (HEADER exact; a C++ socket-call unit per C++ configuration, null pointers included).
* `spec/tools/header_probe.py` `package_names()` lists the new names; PROVENANCE line added.

## Results
* header_check (keke, job headers-apt-gaps-hc1, form probe on, before the sys/pidfd.h rows were added): 0 failed of
  17900. hp2's probe bodies against this head's tree (job headers-apt-gaps-hp2): 24 SAME, 0 DIFF.
* Gate (keke job headers-apt-gaps-gate1, direct tier, base origin/next): affected: ok, rc 0, peak 13G.
* Plants (job headers-apt-gaps-plants1): 5 CAUGHT, 0 SURVIVED, 0 WARN (handoff/HEADERS-APT-GAPS-plants.txt).
* clippy (workspace, all targets, all features, -D warnings) and cargo xtask static: clean (job headers-apt-gaps-lint1).

## HEADERS-APT-GAPS-2 (merge run, branch task/headers-apt-gaps-2)
Merged origin/next (31 commits). Conflicts: crates/abi/tests/appendix_c_layouts.rs (generated: took next's, re-ran
`cargo xtask abi-gen`, copied back; 6-line diff) and spec/abi/PROVENANCE (both sides kept). On the merged head: clippy and
static clean, headers-gen then header_check (form probe on) 0 failed of 18140 (job headers-apt-gaps-2-lint), direct-tier
gate ok rc 0 peak 12G (headers-apt-gaps-2-gate), plants 5 CAUGHT 0 SURVIVED 0 WARN (headers-apt-gaps-2-plants).
apt build still not run. No design changes.

## Not done / not verified
* apt 3.0.4 `dpkg-buildpackage -b` on mei (brief step 4) was **not run**: it needs debs built from this head and the
  ypk flow on mei, beyond this run's budget. The three gaps' probes are 0 DIFF; whether apt has a fourth gap is
  not known until the build runs.
* `abi_extract.py --layouts-only` at this tree also differs from layouts.tsv in rows this story did not touch
  (struct _fpstate / struct _libc_fpstate `_st.*`, `_xmm.element`, `__sigset_t`): only HEADER's rows were spliced.

## Exact next step
Land; then the next snapshot run builds apt 3.0.4 on the system flavour with debs from this head (ypk.sh) and reruns
hp2.sh on the deb (expected 0 DIFF). Plan rows: none new.
