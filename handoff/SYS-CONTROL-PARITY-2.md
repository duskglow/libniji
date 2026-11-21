# SYS-CONTROL-PARITY-2: merge of SYS-CONTROL-PARITY with next, all proofs rerun

State: branch `task/sys-control-parity-2`, from SYS-CONTROL-PARITY's head 435fbeed8; merge commit 36f66cd98
(origin/next ebb670553 merged in); then handoff/ only (plants file, this note, the report json). The story's
work and its record are in handoff/SYS-CONTROL-PARITY.md (unchanged; still the record of what the change is).
Brief: `~/state/briefs/SYS-CONTROL-PARITY-2.md`.

## What this run did

* Step 0: `sys-control-parity-2-lint0` (cargo xtask clippy + static on the inherited head 435fbeed8): rc 0.
  Nothing was red.
* Merge (36f66cd98): the only conflict was the header comment of `tools/deb/system.sh`. Next
  (SYS-LIBRESOLV-3) added `libresolv.so` to libc6-dev's file list, and this branch added the
  Depends/Provides note. The resolved comment keeps both. No code line conflicted. check-system.sh and
  root-system.sh auto-merged: next's libresolv/krb5 checks and lintian override sit alongside this branch's
  stand-ins (count 9) and field checks. Nothing beyond the merge was changed.

## Proof (mei, role s1; head 36f66cd98 for code, later commits handoff/ only)

* `sys-control-parity-2-gate` rc 0: `cargo xtask affected --run --base origin/next`; affected: ok, 0 SKIPPED.
* `sys-control-parity-2-pkg` rc 0 (`~/work/SYS-CONTROL-PARITY-2/pkg.sh`, RAM dir
  /mnt/zram/claude/SYS-CONTROL-PARITY-2/pkg-1, version 2.39+niji20261117.1-1). Both flavours built:
  `check.sh` PASS (2 packages), `check-system.sh` PASS (6 packages). `root-system.sh` gave 18 PASS / 0 FAIL:
  the six packages and the three stand-ins installed, no trigger pending; hello, threads, C++, resolv and
  krb5 (next's libresolv proof) all run; purge leaves the empty root. Parity rerun (inventory + compare against
  the committed reference; network steps not rerun): gaps.tsv 3028 -> 2998. The previous run's 16 removed
  rows are all still gone. The rest of the drop comes from next's libresolv shipping (the
  `symbols: libresolv.so.2` row), plus the libc6 symbols/shlibs rows changing with next's libresolv.
* `sys-control-parity-2-plants` rc 0: handoff/SYS-CONTROL-PARITY-2-plants.txt (the same 9 plants, repointed
  at this run's products). 9 CAUGHT, no WARN lines.
* `sys-control-parity-2-lint` rc 0: `cargo xtask clippy`, the full `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, and `cargo xtask static`, on 36f66cd98 + plants file.

## Open / next

* Nothing open. Out-of-scope rows of audit item 25 (libgcc-s1, Recommends/Suggests, debconf, Breaks), and the
  libc-bin/libc-dev-bin `Depends: libc6 (= <version>)` vs the reference's range, stay as recorded in
  handoff/SYS-CONTROL-PARITY.md.
