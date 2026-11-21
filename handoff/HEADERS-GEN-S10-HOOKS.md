# HEADERS-GEN-S10-HOOKS

State: branch task/headers-gen-s10-hooks, cut from origin/next. Slice 10's two shared generator edits, landed as a
generator change so SL-HEADERS-S10 can rebase on them as data-only (ruling #6748).

Premise: the brief's (slice modules are data-only; slice-contract enforces it). Neither edit was already on next.

## Commits
1. `spec/tools/header_check.py`: a slice check module may name `p1_skip` ((header, name) rows P1 leaves out) and
   `must_fail` (headers shipped to fail: no P5 unit; a unit titled "... must fail" passes only on libniji's
   "is not provided" #error); P1 skips a FILE typedef where the header shows no `struct _IO_FILE`.
2. `xtask/src/headers/typedefs.rs`: FILE tag-only rule. Written into the current if/else chain of `typedef()` (the
   slice 10 patch targeted an older shape and did not apply as a diff); same condition, no per-header code.
3. `libc-include/` regenerated with `cargo xtask headers-gen`: the rule is NOT dormant on this base. Six headers
   change (grp.h, gshadow.h, mntent.h, printf.h, pwd.h, shadow.h): each loses the `struct _IO_FILE { ... };` body
   (34 lines and its `__NIJI_D_struct__IO_FILE` guard) and gains `struct _IO_FILE;`, because visibility.tsv shows no
   struct _IO_FILE in them (stdio.h still defines it). Those five of six that exist on slice 10's branch are
   byte-identical to its output. `headers-gen --check` is ok after this.
4. Plants file `handoff/HEADERS-GEN-S10-HOOKS-plants.txt`.

## Results
- Direct gate (affected --base origin/next --run): 560 passed, 0 failed.
- Plant s10h-file-tag-only (headers-gen --check): see the report json / jobs line.
- header_check.py hooks are NOT planted here. `python3 spec/tools/header_check.py` is red on origin/next independent of
  this change: 8 FAILs, all P8 `sys/mount.h` vs `linux/mount.h` ("redefinition of struct winsize"); zero FAIL P1 with
  the FILE skip in. plants.rs aborts a plant whose command has a red baseline. No slice names p1_skip/must_fail on this
  base, so nothing exercises those two hooks; slice 10's plants (s10-p1-ftw-statx, s10-regexp-must-fail,
  s10-must-fail-reason, s10-p5-regexp-in-p5, s10-p1-file-incomplete) catch them once its rows rebase on this.

## INFORMATIONAL / RULING NEEDED
- INFORMATIONAL: header_check.py P8 sys/mount.h fails on origin/next (above); someone owns a fix (sys/mount.h and
  linux/mount.h both define struct winsize via its include guard).
- RULING NEEDED: none. The brief said `headers-gen --check` leaves libc-include unchanged "or list every changed line":
  listed above.

## Next step
Land this, then rebase SL-HEADERS-S10 on it (drop its header_check.py and typedefs.rs edits and its libc-include
changes for the six headers).
