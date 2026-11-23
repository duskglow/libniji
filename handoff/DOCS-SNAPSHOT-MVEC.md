# DOCS-SNAPSHOT-MVEC

State: branch task/docs-snapshot-mvec, docs only; `cargo xtask static` green on keke (job docs-snapshot-mvec-static).
Premise: none stated; the work rests on tools/deb/manifest.tsv and records on this head.

Done (before -> after):
- install-packages.md: niji6, niji6-dev, libc6, libc6-dev rows now list `libmvec.so.1`, `libmvec.a`, `libmvec.so`.
- libc6-dbg: "symbol table and line tables of every shared object" -> optional; every object resolves by build-id; full
  debug information for some pending SYS-DBG-DWARF; covers every libc-bin program except /usr/bin/iconv.
- libc-bin row lists /usr/bin/iconv (dynamic); libc6-dev: system flavour replaces Debian's and ships no headers (until SL-HEADERS).
  (The "plain gcc compiles with system headers" sentence is already scoped to niji6-dev in the file; unchanged.)
- differences-from-glibc.md: records/pending/SYS-LIBMVEC-2-1.toml -> records/decisions/D-10/D-10.124.toml;
  UTIL-8-2-1 -> D-31.17; DL-ATEXIT-OWNER-2-1 -> COMPAT-30; D-31.15 row lists GLIBC_2.14 and GLIBC_2.2.5.
  `grep -rn records/pending/ docs/` now finds nothing.
- docs/RELEASE-NOTES.md (the root file is a pointer): sections for snapshot-2026-11-17.2 and .3; "ldd is next, then iconv" trimmed.

Not done: records/pending/ is still named outside docs/ (tools/deb scripts, crates, spec, glibc-functional-spec.md,
tools/records_import.py): out of this docs-only brief. Next step: land.
