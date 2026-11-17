# SCRUB-REFERENCE-CRASHES: state 2026-11-17 (run 1)

Branch `task/scrub-reference-crashes` (base origin/next 7f7e6c9e4). Role mei `~/w/s4`.
Brief: `~/state/briefs/SCRUB-REFERENCE-CRASHES.md`. Owner ruling 2026-11-17: no text in HEAD says how
the reference fails on bad input; libniji's own rule is stated instead, and libniji must not share it.

## Done (steps 1-3)
- Commit "Describe libniji's rule on bad input": 31 files, 49 exact rewrites, plus the differences
  table (24 rows: the reference column reads "No defined result." or keeps only its non-failing
  compatibility detail), `spec/behaviour/efault.md` (sections rewritten to libniji's rule) and
  `efault.glibc.txt` (9 lines removed; it keeps the `EFAULT` answers only; no code reads it).
  §16.1's clause text in `glibc-functional-spec.md` and its `spec/clauses.tsv` line; `cargo xtask
  stubs` reproduces the tsv byte for byte (lint job).
- Row "A non-NULL but unwritable output struct" had its libniji and glibc columns swapped; it now
  matches efault.md and `conformance/c/s13_efault.c` (libniji faults at the caller's own address).
- Commit "Correct decision records": 16 records, 21 rewrites (D-2.7, D-4.22, D-7.9, D-7.12, D-11.11,
  D-12.18, D-13.13, D-13.15, D-16.11, D-19.2, D-19.3, D-23.2, D-23.3, D-24.11, D-25.5, D-26.2). Only
  the description of the reference is dropped; no decision changes. The 12 generated views under
  plan/07-decisions are left for the train (records-gen: "12 views pending regeneration").
- Only comments, doc text, test reason strings and records changed: no behaviour, so no plants.

## Search of step 1 on this head (brief's pattern)
Remaining hits, none a beacon: the generated plan/07-decisions views (train regenerates them from
the corrected records); `glibc-functional-spec.md:696` and its `clauses.tsv:150` line (malloc's
documented detection of caller misuse); `validation/corpus/printf.corpus` (signal names as printf
arguments); handoff TB-FOREIGN-WORDS plants files, AQOURS-SMOKE-RERUN-2.json and
DL-LD-PRELOAD-2.json (libniji's own failures); release-notes-1107.md (names a scan);
next-batch-1024-2.md:66 (a rule).

## Not done
1. **Step 5, probe data.** The products tests run each oracle against the reference live and key
   their known differences on the reference's line. Records and tables still holding the
   reference's failure lines: `spec/behaviour/{aio,catalogs,compat,dynlink,fstree,mallocdbg,
   mntent,posixipc,process,syslog,terminals,nss-resolv-compat}.glibc.txt` (grep `killed|signal|
   SIG[A-Z]`) and the KNOWN keys in the matching `crates/core/tests/*_products.rs` (aio, catalogs,
   fstree, mallocdbg, mntent, posixipc, syslog, terminals), `tools/probe/compat`, `tools/probe/dl`.
   Proposed mechanism: in each oracle, run those cases only in the libniji build (a `-DNIJI_ORACLE`
   in the niji build command), move the KNOWN entry to ADDED (libniji's own line, the reason kept),
   and delete the lines from the record. Plants: drop the libniji check, expect the ADDED miss.
2. **Step 4, test list.** Found (libniji's result checked): s23_time_symbols.rs (timer_create),
   s24_system_symbols.rs (gethostname), s07_locale_symbols.rs (rpmatch), terminals_products.rs,
   mntent_products.rs, syslog_products.rs, posixipc_products.rs, catalogs_products.rs,
   fstree_products.rs, aio_products.rs, mallocdbg_products.rs, s02_diagnostics_symbols.rs and
   conformance s02_backtrace.c (backtrace_symbols count), s15_sockets_symbols.rs
   (if_freenameindex, ruserpass), patterns_products.rs (regerror, re_exec), crates/ld/tests/dlclose.rs,
   s16_nameservices_symbols.rs, s22_threads_symbols.rs / conformance s22_more.c (affinity sizes),
   conformance s22_compat_condvar.c, timers_diff.c (stime), conformance s13_efault.c (opendir).
   **Missing: `argp_error`/`argp_state_help` with a NULL state** (argp.rs handles it: stderr and the
   short program name); add a conformance check to `conformance/c/s01_argp.c` with a plant.
   Each "found" entry still needs a one-line confirmation that it checks the input named in the
   brief's case (by test name), then the list goes in the report.

## Rulings / informational
- RULING NEEDED: records were corrected in place (CONTRIBUTING's correction path). The brief asked
  for the view-level path, but a view mapping leaves the text in `records/`, which the owner ruling
  removes from HEAD. Recommend: keep the in-place correction (separate commit da85cbe70, easy to drop).
- INFORMATIONAL: libniji faults by design, at the caller's own address, on a NULL with a small size
  in memcpy/memset (D-4.22, owner ruling) and on wild non-NULL pointers it reads or writes itself
  (realpath, gethostname, getdomainname, stat's result). These are caller errors, not data input.

## Next step
Continuation: step 5 per suite (mechanism above), the argp NULL-state test, the confirmed step-4
list, plants for new tests, gate.
