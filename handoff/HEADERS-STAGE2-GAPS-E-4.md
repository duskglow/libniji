# HEADERS-STAGE2-GAPS-E-4 (merge-only continuation of E-3)

State: branch task/headers-stage2-gaps-e-4. Merged origin/next. Conflicts: shared tables (macros.tsv, visibility.tsv
resolved by keeping both sides, sorted), s05.py, s07.py, header_probe.py (both sides kept), arpa/nameser.h.in
(origin/next's text taken: it already carries ns_msg_*, ns_rr_*, NS_GET/PUT and GETSHORT family).

Follow-up fixes on the merged head: nameser_compat.h.in and its visibility rows no longer alias GETSHORT etc.
(nameser.h defines them now; they were a redefinition); the plants file's NS_GET16 and NS_PUT16 patterns follow the
landed text.

Results: clippy and `cargo xtask precheck` green on the merge head; `headers-gen --check` ok; header_check 0 FAIL;
gate2 (so + affected --run) green; plants4 on the final head: see the hand-back.
Not to redo: the table merges. Next step: land.
