# HEADERS-STAGE2-GAPS-F: handoff (leg 2 of run 1, REFRESH)

Branch task/headers-stage2-gaps-f from origin/next 05995ca18; role shiki:~/w/sh1 (synced at 6adf24eb2's tree).
Scratch: local `~/w/.scratch/HEADERS-STAGE2-GAPS-F/` (probe*.py, gen2.sh, gen3.sh, pkgproof.sh, msg*.txt); shiki
`~/work/headers-stage2-gaps-f/` (stock/ niji2/ niji3/ probe tables, p4-*.txt, forms-stock.tsv, libc-include.tgz);
keke `~/work/headers-stage2-gaps-f/` (pkgproof.sh, pkg/). No decision rows; black-box probes only.

## Done (pushed)
* 4d268813a (leg 1): port I/O fragment, telnet name tables fragment, 19 typedefs + 54 reserved emit rows + visibility.
* cb45747a5: header_check fragments s15.d/10 (port I/O: type, static, not macro, not noexcept, calls), s13.d/10 (telnet
  macros exact, tables extern char *[], *_OK truth/type, counts with switches, SLC_NAMELIST C++11), s05.d/10 (exact
  for the 19 __ types). 431 units, 0 failed.
* 6adf24eb2: forms. guards() wraps a guard in `#ifndef` when macro-forms.tsv says not replaced (xtask/src/headers.rs
  ~line 621); FD_SET blocks guard on `__NIJI_FD_BITS` in stdlib.h.in, sys/{time,procfs,types,select}.h.in; FD_SET
  out of features.py KNOWN; macro-forms.tsv rows ifaddrs.h ifa_broadaddr/ifa_dstaddr kept, resolv.h _PATH_RESCONF
  kept (feature_probe forms on stock: forms-stock.tsv; probe4.py); ifaddrs.h.in `#ifndef`s; check s05.d/20-guard-forms.py
  (271/271 fail on old headers, 0 after). shiki job headers-stage2-gaps-f-gen3: header_check 0 failed of 24160;
  features forms (19 headers) 0 differ; probe4 0 differ; family probes 5564 + 59 lines, 0 diff vs stock.

## OPEN DEFECT (fix first)
`cargo test -p xtask every_table_row_is_emitted_and_matches_the_tables` FAILS (job headers-stage2-gaps-f-rows1):
"visibility function inb ... outsl" (18) and "visibility object authtype_names/encrypt_names/enctype_names/
slc_names/telcmds" (5): "kind X: no table row the emitter of that kind reads" (xtask/src/headers/rows.rs ~line 594).
Caused by leg 1's visibility rows for fragment-owned names. Options: (a) typedefs.tsv rows kind `function`/`object`
with a `cxx` type (rows.rs line 373-396 then also checks `__typeof__`; see what item() emits for a typedefs
function row, and headers.rs ~line 509-516's function check); (b) prototypes.tsv rows (risk: callsite/nm symbol
checks expect an exported symbol; these are static inline). Prefer (a) if item() is harmless; check that the
generator still emits nothing extra in sys/io.h / arpa/telnet.h (headers-gen --check, diff libc-include).
Other xtask headers tests: 28 passed.

## Running
keke job headers-stage2-gaps-f-pkg1 (pkgproof.sh: util-linux hwclock-cmos.c, inetutils libtelnet auth.c/encrypt.c,
with and without -DAUTHENTICATION -DENCRYPTION, stock vs libniji headers of 6adf24eb2). Read
`~/logs/headers-stage2-gaps-f-pkg1.log` on keke (ssh claude@<keke>), `~/bin/jobwait headers-stage2-gaps-f-pkg1`.
If libc-include changes again, re-copy libc-include.tgz to keke and rerun (new label pkg2).

## Next steps (exact)
1. Fix the rows test (above); sync; rerun it + headers-gen --check.
2. Read pkg1; report any further header gap (header, name, unit), do not chase.
3. Plants file handoff/HEADERS-STAGE2-GAPS-F-plants.txt (format: see handoff/HEADERS-STAGE2-GAPS-D-plants.txt), via
   `cargo xtask plants` (/dev/shm/claude/headers-stage2-gaps-f/plants-1 on shiki, Zen 1). Candidates: swap outb's
   parameters in headers/sys/io.h.d/010-port-io.frag (msg port I/O); drop `static` from inb; `<` -> `<=` in
   AUTHTYPE_NAME_OK; drop "DES_OFB64" from enctype_names; typedefs.tsv __int8_t cxx -> char (exact sys/types.h);
   `kept` -> false in guards() (guard forms); one template back to `# ifndef FD_SET` (FEATURES forms). Each cmd
   regenerates (cargo xtask headers-gen) then runs header_check --only / features.py --only.
4. clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`), `cargo xtask static`, affected
   gate (jobrun gate), precheck, report json (handoff/reports/HEADERS-STAGE2-GAPS-F.json, with jobs), handback-check.
INFORMATIONAL for the report: stock skips a whole header whose guard a program defined first; libniji keeps the
guard's value but still declares the header's contents (its own guards are __NIJI_ names).
