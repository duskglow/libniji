---
section: changes
---
- **`locale-gen`, `update-locale` and `validlocale` (D-31.36, D-31.37):** the `locales` package ships
  `/usr/sbin/locale-gen`, `/usr/sbin/update-locale` and `/usr/sbin/validlocale`, commands of the shipped `localedef`:
  `locale-gen` compiles every locale `/etc/locale.gen` selects into `/usr/lib/locale/locale-archive` in one process
  (`--keep-existing` keeps the locales the archive holds), `update-locale` edits `/etc/locale.conf`
  (`/etc/default/locale` links to it) and `validlocale` says whether a locale is available and which `/etc/locale.gen`
  line would add it; their files, output and exit statuses match the commands of Debian trixie's `locales` package. The
  package's postinst creates `/etc/locale.gen` (every `SUPPORTED` entry, commented out), compiles the selected locales
  and, under debconf, applies the locales and default chosen in `dpkg-reconfigure locales`. Differences: rows
  "`/etc/locale.alias`" and "`update-locale` on a value that is not available" in
  [differences-from-glibc.md](differences-from-glibc.md).
