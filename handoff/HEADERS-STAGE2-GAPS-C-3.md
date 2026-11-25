# HEADERS-STAGE2-GAPS-C-3: merge with next (CI conflict in utmp.h.in), no new header rows

Continues `handoff/HEADERS-STAGE2-GAPS-C-2.md` (read it; do not redo it).

**State.** Branch `task/headers-stage2-gaps-c-3` (from task/headers-stage2-gaps-c-2 0bfa5f1c4), merged with origin/next of
2026-11-25 (21 commits). Build role: the Zen 1 host (60 vCPU).

## Done this leg
1. **Merge conflict** (the CI conflict of the last run): `headers/utmp.h.in`. Both sides had added the older struct utmp
   member names (ut_name, ut_time, ut_xtime, ut_addr) as macros. Kept next's single block (`@@when utmp`, the same four
   macros with the same expansions); my per-name `@@when` copy was dropped. `libc-include/utmp.h` regenerated with
   `cargo xtask headers-gen` (only that generated file differed). The tests of both sides stay (S13 THIN units and next's).
2. Clippy (all targets, -D warnings) and `cargo xtask static`: clean on the merged head, before any other change.
3. Gate (direct tier, `--base origin/next`) and the 5 plants of `handoff/HEADERS-STAGE2-GAPS-C-2-plants.txt` re-run on
   the merged head; results in the hand-back and in `handoff/reports/HEADERS-STAGE2-GAPS-C-3.json`.

## Decisions in force
Ruling #8587 / #8649: every probed guard of every header is defined; narrowing is one filter in
`xtask/src/headers.rs` `Gen::guards()` (recommendation: keep).

## Not done (and why)
The "Open / next" list of the C-2 handoff is unchanged. `struct ns_tsig_key` / `struct ns_tcp_tsig_state` need
`abi_extract.py` tags plus x86-64 and aarch64 layout rows (the aarch64 rows cannot be probed on this host) and the
TELCMD table is a template form; none is named by Yohane's stage-2 rows, and the handoff says candidate lists must come
from the owning spec section first. They are left for a story that takes one family at a time.

## Exact next step
Land this branch. A later story: take one family of the open list (e.g. arpa/nameser.h tsig structures, then
netinet/icmp6.h) from its spec section, probe the names, and add rows as C-2 did.
