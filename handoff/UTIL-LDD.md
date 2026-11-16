# UTIL-LDD (UTIL-4): ldd

**State.** Branch `task/util-ldd-2` (UTIL-LDD's `task/util-ldd` plus UTIL-LDD-2), code head `4d6d440e5`; this
note and `handoff/reports/UTIL-LDD-2.json` are the commit after it. Role mei `~/w/s1`; the swap roots run on keke
(mmdebstrap, the mirror keyrings) from `~/work/UTIL-LDD-2/` (`swap.sh`, the debs, the branch's tools/deb copies).

## On the branch

* `csys/util/ldd.c`: ldd as a static PIE over `libc.a`, listing through Liella's loader in trace mode (D-31.7);
  `-v` from `DT_VERNEED`; `-d/-r/-u` refused until DL-10. Interpreter line: kept when a listed object names the
  interpreter in `DT_NEEDED` **or `libc.so.6` is listed** (UTIL-LDD-2: without that, every libc-linked program lost
  the line in a Liella root, and initramfs-tools' copy_exec takes the loader from it). Interim per ruling #6043.
* Decision record `records/pending/UTIL-LDD-2-1.toml` (D-NEW, series D-31): the front end, the identity string,
  the lazy `-v` header, the interim filter and the libc rule. CI assigns the id.
* `validation/transcripts/ldd/`: 28 transcripts (no-libc programs; recorded from the reference by UTIL-LDD).
  `docs/transcripts.md` documents `cc:` and `mask-addresses:`.
* `tools/deb/swap-cmds.sh` / `swap-system.sh`: the swap-root comparison now runs `ldd` (dash, perl, python3, a
  library, two operands, `-v` twice, a non-ELF file; only a line's final ` (0x<hex>)` is masked) and initramfs-tools'
  `copy_exec` on dash, perl and python3 (initramfs-tools-core added to both roots).
* `tools/deb/check-system.sh`: allows `/usr/bin/getent` and the `./usr/bin/` directory entry (the built libc-bin
  failed the check on the directory).
* Plants `handoff/UTIL-LDD-2-plants.txt` (12: 10 on ldd.c, 2 on the harness's address masking).

## Results (jobs)

* mei: `util-ldd-2-lint-2` rc 0 (clippy, static, records check on `80e52ac13`); `util-ldd-2-debs-2` rc 0 (system
  .debs + check-system); `util-ldd-2-gate-2` (direct tier), `util-ldd-2-transcripts-2` (ldd, getent, getconf,
  ldconfig transcripts), `util-ldd-2-plants-2`: see the hand-back (at writing: 9 of 12 CAUGHT, one WARN-NO-MSG:
  `ldd-several-header`'s mutant does not build under -Werror, `several` becomes unused).
* keke `util-ldd-2-swap-3` (final swap-cmds): `copyexec-1` PASS: copy_exec of dash, perl and python3 copies the same
  20 entries (loader, libc, libm, libcrypt, libexpat, libz, links) in both roots. `ldd-8` PASS. ldd-1..7 differ only
  by the loader's output (below); rc 1 for those, so it is evidence, not a JOBS entry. swap-1 (before the libc
  rule) showed the missing interpreter line; swap-2 showed copy_exec ran only for dash (fixed in `4d6d440e5`).
* The last commit `4d6d440e5` changes only tools/deb/swap-cmds.sh (proved by swap-3); gate/plants ran on `80e52ac13`.

## Open (the loader, not ldd: for a DL story)

1. Liella's loader resolves `libc.so.6`, `libm.so.6` ... to `/usr/lib/x86_64-linux-gnu/`, the reference to
   `/lib/x86_64-linux-gnu/` (same root, same ld.so.conf; Liella's cache has 54 entries, the stock one 64). ldd prints
   the loader's paths. copy_exec copies the same files either way (merged /usr), so no initramfs difference.
2. For a library operand (no `PT_INTERP`) the loader's trace prints `ld-linux-x86-64.so.2 (0x..)`, the reference
   `/lib64/ld-linux-x86-64.so.2 (0x..)` (swap ldd-4).
3. `-v`: Liella's libc.so.6 has no version needs on the loader, so its `\t/lib/.../libc.so.6:` block is absent.
4. When the loader lists the interpreter only when needed (and names libc's), remove ldd's filter (ruling #6043).

Next step: none for ldd; cut the DL story for 1, 2 and 4. Read: spec §31.3, D-31.7, the pending record, this note.

## Rule slips (no effect on the tree)

One `sed -i` on /dev/null (it failed, nothing edited); one oversized `ls` of keke's `~/w`.
