# SL-HEADERS-S07-TAIL-3: merge next again, resolve diagattrs.tsv, keep headers-gen clean

State: branch `task/sl-headers-s07-tail-3` (continues SL-HEADERS-S07-TAIL-2, whose handoff is
`handoff/SL-HEADERS-S07-TAIL-2.md`; that one continued `handoff/SL-HEADERS-S07-TAIL.md`). Code head 9ff179900; the
report json comes after it.

## What was done
1. Merged origin/next (1b1924f16) into the S07-TAIL-2 head (fcb294f3a). Merge commit c677046ea.
   The one conflict was in `spec/abi/diagattrs.tsv`, at the posix_spawn rows. This branch added 27 spawn.h rows
   (`posix_spawn*`, nonnull only). Next removed the `pow*` rows next to them under the errno veto (D-X.158: no
   const or pure on a function that sets errno). The resolution keeps both changes: the spawn rows are in and the
   pow rows are out. The veto does not affect the spawn rows, because they carry no const or pure.
2. On the merged tree, `cargo xtask headers-gen` failed with 6 problems: `headers/{spawn,sys/param,sys/wait}.h.in
   defines NULL, which macros.tsv owns`. These three templates came from this branch, and next's generator now
   refuses a hand-written NULL. Commit 9ff179900 replaces the hand-written C/C++ definition with `@@null NULL`
   inside the `# ifndef NULL` guard, the idiom time.h.in and string.h.in use. The checker's message says
   `@@value(NULL)`, but a `null`-type macros.tsv row is spelled with `@@null`.
   Result: headers-gen writes all 236 headers byte-identical to the committed libc-include/. A checksum compare of
   the whole role tree against the worktree found no other generated file that differs.
3. `net/if_ppp.h` and `net/if_shaper.h` still contain a plain `#include <sys/ioctl.h>` (line 10).
   `spec/tools/header_checks/s13.py` is still deleted.

## Proof (keke, role r1; labels `sl-headers-s07-tail-3-<x>`)
* `hgen2`: headers-gen + abi-gen on the merged tree with the template fix: 0 problems, 0 changed headers.
* `hc`: `python3 spec/tools/header_check.py`, all slices: 0 failed of 17084.
* `gate`: clippy -D warnings, `cargo xtask static`, `cargo xtask so`, `affected --run --base origin/next`:
  6 suites, 685 passed, 0 failed, 0 SKIPPED, affected ok, peak 2.5G. The changed generated headers and
  diagattrs.tsv were checked by the generator-check bounded set (their FULL run is the train's).
* Final-head jobs: `gate2` and `hc2` (the same commands as `gate` and `hc`), run after the last sync. The
  earlier `gate`, `hc` and `hgen2` are SUPERSEDED. They were green, but precheck saw the role's content change
  after they ran: the handoff-only sync listed s13.py as removed, and hgen2 rewrote the libc-include mtimes.
* Plants: none. The template change produces byte-identical generated output, so it changes no behaviour a plant
  could break (equivalent by construction).
* SUPERSEDED `hgen`: its clippy and static passed, but it reported rc 0 even though headers-gen failed (the chain
  used `;`). That job is how the NULL problem was found.

## Open / next
* No rulings are open.
* INFORMATIONAL: the headers-gen message "use @@value(NULL)" (xtask/src/headers.rs, near line 718) should name
  `@@null` for a `null`-type row.
* Next step: land via the CI train. S20 waits on this branch.
