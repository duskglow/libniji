# HEADERS-STAGE2-GAPS-B (Yohane stage 2: openssh, apt, binutils, krb5, systemd, apparmor, babeltrace, avahi)

**State:** branch `task/headers-stage2-gaps-b` (from origin/next e125f7e34). Leg 2 ends at the WIP head pushed with
this note. Role keke:~/w/n2. Local scratch `~/w/.scratch/HEADERS-STAGE2-GAPS-B/` (p1..p5.sh, probe1/2.sh,
check1.sh, splice.py, r1/ probe outputs); on keke `~/work/headers-stage2-gaps-b/`. Decisions: none new (probes).

## Done
* Leg 1 (d687e907a): attr_probe sentinel + `format_arg` (gettext family), macro-aliases.tsv + aliases.py module,
  headers.rs `attr_text` format_arg, enums.rs `macro_alias`. Details: openssh premise corrected (stock signal.h DOES
  declare execl* under _GNU_SOURCE; real cause our `__sentinel__ (0)` spelling, now none, as stock).
* Leg 2 fdc3f38ea: header_probe `stage2_names()`, abi_extract ELF pair types; tables spliced (visibility +109,
  layouts +61, typedefs +20, macros +15, enumerators +14; whole regenerations of layouts/enumerators move unrelated
  rows, so splice only); `__RES` 19991006 via headers/reserved.tsv emit; IN6ADDR_*_INIT in netinet/in.h.in;
  headers.rs prefers macro_alias over enum_macro (MSG_TRYHARD -> MSG_DONTROUTE, one-step stringize stock vs ours);
  header_check units s12 `initializers`, s16 `dl_requests`.
* Leg 2 WIP commit: s10 `format_arg libintl.h` unit (pragma -Wformat-security error), plants file
  handoff/HEADERS-STAGE2-GAPS-B-plants.txt (5 plants), regenerated libc-include + crates/abi (from hgen2: NOT yet
  with the __RES row and the alias precedence; regenerate again).
* Probe p5 (keke, before the precedence fix): all items compile C and C++, stock and ours, with
  -Werror=format-security; AF_/PF_/MSG_/SOL_/SO_/SCM_ one-step forms equal except MSG_TRYHARD (fixed since);
  systemd's af list step finds the same 48 names with both.

## Open / for the hand-back
* RTLD_DI_CONFIGADDR/PROFILENAME/PROFILEOUT: declared (reference enumerators, man dlinfo documents none of them); our
  dlinfo serves LMID, LINKMAP, ORIGIN, SERINFO, SERINFOSIZE, TLS_MODID, TLS_DATA, PHDR and refuses those three
  (-1, dlerror). RULING NEEDED: recommend keep declared (source compatibility, refused cleanly).
* Hand-back: include the section in local scratch `security-private.txt` verbatim (never in the repo).
* INFORMATIONAL: CHAR_MAX is SCHAR_MAX unconditionally in our limits.h (pre-existing; not checked under
  -funsigned-char). ELF32_ST_*/ELF64_R_* function macros absent in both halves (no table rows). Leg 1's items too
  (diagattrs `pure` drift 182 cells; pthread_create nonnull 1,2,3,4 vs stock 1,3).
* Static: private-paths failed only on this note's old home path (fixed here).

## Exact next steps (leg 3)
1. `~/bin/jobwait headers-stage2-gaps-b-check1` on keke (NIJI_ALIAS_ALL=1 header_check + clippy + static on fdc3f38ea
   minus s10): read `~/work/headers-stage2-gaps-b/hc.log` FAIL lines; ALIAS-DIFF NOTE lines for other headers are
   information (ENFORCED is sys/socket.h only). Superseded label: it ran without the s10 unit.
2. Sync this head; job: `cargo xtask headers-gen` + `python3 spec/tools/header_check.py --only "format_arg|initializers|S16 dlinfo"`;
   rsync libc-include/ and crates/abi/ back, commit "Regenerate headers" (generated files only).
3. Gate: `cargo xtask affected --list` (likely FULL: generated tables -> precheck + header_check job, write
   `FULL: the train's (...)`), plants job via `cargo xtask plants handoff/HEADERS-STAGE2-GAPS-B-plants.txt --dir
   /dev/shm/claude/headers-stage2-gaps-b/plants-1 --timeout 600` (keke is Zen 1). Append results to the plants file.
4. Proof builds (blocking per brief): no sources fetched yet. openssh portable (trixie) configure+make ssh/sshd,
   krb5 src/util/support, against libniji's headers (system flavour). If not reachable in budget: hand back with the
   p5 compile proofs and say NOT VERIFIED for the package builds.
5. clippy + static foreground, precheck, report json, handback-check.
