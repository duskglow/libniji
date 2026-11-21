# SYS-DEV-LINKS: link-time names libc6-dev ships that ours do not (audit row 13)

Size: S. Kind: code (packaging). From plan/package-parity-audit.md.

The system flavour lacks `libBrokenLocale.so.1` (`ref-elf.tsv:268`; SL-SONAMES-2 landed, but the system packages do
not contain it) and these libc6-dev files (`gaps.tsv` class a): dev links `libanl.so`, `libBrokenLocale.so`,
`libc_malloc_debug.so`, `libnss_compat.so`, `libnss_hesiod.so` (`ref-files.tsv:1979` …), and archives `libg.a`,
`libmcheck.a`, `libpthread_nonshared.a`, `libBrokenLocale.a` (`ref-files.tsv:1493`, 1946). Without `libanl.so`,
`-lanl` in a dynamic link takes the static archive; without `libBrokenLocale.so`, `-lBrokenLocale` fails.
(`libresolv.so`, `libthread_db.so`, `libmvec.so` go with SYS-LIBRESOLV-2, SYS-THREAD-DB, SYS-LIBMVEC.)

1. Ship the placeholder `libBrokenLocale.so.1` (the D-27.9 rule) in the system libc6.
2. Ship each dev link and archive above in libc6-dev, with D-27.9's contents (empty archives where the reference's
   are placeholders; read names and sizes only).

Proof: `-lanl`, `-lBrokenLocale`, `-lmcheck`, `-lg` dynamic and static links of a test program succeed and run
against the packages; check-system.sh lists them; plants.
