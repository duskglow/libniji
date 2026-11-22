# SL-HEADERS-S07-TAIL-2: merge next, regenerate net/if_ppp.h and net/if_shaper.h, drop slice 13's deferral

State: branch `task/sl-headers-s07-tail-2` (stacks on SL-HEADERS-S07-TAIL; its handoff is `handoff/SL-HEADERS-S07-TAIL.md`).

## What was done
1. Merged origin/next (1d4d07280) into the S07-TAIL head (1d5038714). The merge had no conflicts.
2. `cargo xtask headers-gen` on the merged tree: of 236 headers, only `libc-include/net/if_ppp.h` and
   `libc-include/net/if_shaper.h` changed. In both, the `__has_include(<sys/ioctl.h>)` guard became a plain
   `#include <sys/ioctl.h>`, because the `@@needs` header now exists. abi-gen output did not change. This was
   the train-923 `headers-gen-check` red.
3. Deleted `spec/tools/header_checks/s13.py` (slice 13's deferral of the names from <sys/ioctl.h>) and set
   `p1` = 1 for both rows in `headers/slices/s13.tsv`, as slice 13's handoff asked. The shared table P1 now
   checks both headers in full.

## Proof (mei, role s3; labels `sl-headers-s07-tail-2-<x>`)
* `hc`: `python3 spec/tools/header_check.py`, all slices (7 and 13 included): 0 failed of 16777.
* `lint`: `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo xtask static`, `affected --list`
  (selects only the stubs/purity/unsafe/csys checks, the xtask unit tests and static; no test targets). All clean.
* `gate2`: `cargo xtask so && cargo xtask affected --run --base origin/next`: affected ok, 0 SKIPPED lines.
* Final content: `hc2` (header_check 0 failed of 16777) and `gate3` (clippy -D warnings, `cargo xtask static`,
  `affected --run --base origin/next`: ok, 0 SKIPPED, peak 1.7G). Then `cargo xtask precheck`.
* SUPERSEDED `hc`, `lint`, `gate2`: precheck found that the role still had the deleted s13.py when they ran.
* SUPERSEDED `gate`: red only in `xtask --test transcripts` (iconv). The role's `target/niji-test/iconv` came from an
  earlier build, and its NEEDED `libc-by-path.so` was gone (ENOENT on exec). After `cargo xtask so` rebuilt the
  products, `gate2` was green. This was a stale product on the role, not this change.

## Open / next
* No rulings are open. The rest of slice 7's findings belong to other stories (HEADERS-GEN-SHARED, HEADERS-SYS-PARAM).
* Next step: land via the CI train.
