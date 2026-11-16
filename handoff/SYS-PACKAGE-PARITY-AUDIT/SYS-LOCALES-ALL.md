# SYS-LOCALES-ALL: a locales-all package (audit row 5)

Size: M. Kind: code (packaging). Needs SL-LOCALEDEF and ruling R5. From plan/package-parity-audit.md.

Exact counts in trixie (`consumers-packages.tsv`): 108 source packages Build-Depend on `locales-all` and 98 binary
packages Depend on it (10 Recommend it). With no such package, none of the 108 can build and none of the 98 can be
installed on a libniji system (`ref-control.tsv:336`).

1. Build `locales-all` (`Architecture: amd64`, `Multi-Arch: foreign`, `Depends: libc-l10n`) holding every locale
   SL-LOCALEDEF compiles from the locale sources R5 allows, in libniji's compiled format (D.1).
2. Its relation to `locales` as the reference's control fields state them (names only).

Proof: `locale -a` lists the set; `setlocale` of each succeeds; one of the 108 sources builds in the Aqours root;
plants.
