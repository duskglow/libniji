# WL-PREP-HEADERS: the workload gate's static products carry libniji's C headers

State: branch task/wl-prep-headers, code commit 57ef4e7 (validation/wl_lock.sh, wl_lock_test.sh, wl_prep.sh comment);
this note, the plants results and handoff/reports/WL-PREP-HEADERS.json on top. Base origin/next 522900c6f.

## Cause
Since plan/headers.md slice 20, scripts/niji-cc adds `-nostdinc -isystem <products>/include ... <products>/uapi` to
every call it does not skip (no `-nostdinc`, no `-shared`), link-only calls included, and exits 1 when
`<products>/include/features.h` or `<products>/uapi` is missing. wl_link_set (validation/wl_lock.sh) cached only
libc.a, libm.a, niji-crt1.o and .profile in the static set, so the first static kit link (sort) failed. Reproduced
from the 11-25 nightly's own run (link-sort.log under the products key of 623550dac-release, the message above); the
dynamic link (wl_dyncc.sh) uses no headers and was not affected.

## Fix (products, as shipped; niji-cc unchanged)
- wl_link_set (static) copies `include/` and `uapi/` from the same build when it staged them and writes `.headers`
  (`staged` or `none`; a build of a commit from before the headers stages none and its niji-cc needs none).
- wl_products_ok (static, marked set = has .profile): no `.headers` -> incomplete (rc 1, rebuilt once: sets cached
  before this change are never served without headers); `staged` without include/features.h or uapi/ -> incomplete.
  Unmarked legacy fast sets predate the headers and are reused as before.
- niji-cc left as is: on a pure link it asks for headers it does not use, but the kits now link libniji's products
  as shipped, which include them, and changing niji-cc cannot help the already-landed shas the gates measure.
  Size: include/ 2.7 MB + uapi/ 8.8 MB per static set (1214 files).

## Proof (keke ~/w/r1, at 57ef4e7)
- gate `wl-prep-headers-gate`: affected direct tier = cargo xtask static + validation/wl_lock_test.sh: ok.
- plants `wl-prep-headers-plants`: 6 plants, 6 CAUGHT (handoff/WL-PREP-HEADERS-plants.txt).
- `wl-prep-headers-wlkits`: `bash validation/wl_kits.sh 623550dac...` (the failing nightly's sha; its source was
  primed, the head's sha was not): exit 0, 2120 s. prep-static 403 s (the cached set without headers was rebuilt,
  392 s, then 18 programs relinked), prep-dynamic 3 s (cached), both rule lines printed.
- clippy (workspace, all targets, all features, -D warnings) clean on the role.

## Rows against the 11-24 nightly (de86ab08b; not chased: many commits lie between, and the kit objects are compiled
with system headers, so the header switch itself cannot move them)
- geomean instructions:u static 0.8237 -> 0.8136, dynamic 0.8774 -> 0.8682; page faults static 0.9866 -> 0.9746,
  dynamic 1.1972 -> 1.1666.
- static page faults > 1.10: 1 -> 3: ls-l-enUS 1.125 -> 1.142, du 1.071 -> 1.128, bash-loop 1.073 -> 1.198.
- dynamic instructions:u > 1.10: printf-loop 1.881 -> 1.411. Dynamic page faults > 1.10: 24 -> 21 (wc, gzip-c and
  others below; bash-loop 1.438 -> 1.629, md5sum 1.415 -> 1.347).
- Both INVIOLABLE rule lines FAIL, as on 11-24 (static 3 rows, dynamic 22 rows over 1.10).

## Next step
Land; the next nightly re-runs the sweep. Nothing else open. Plan rows: none read or changed.
