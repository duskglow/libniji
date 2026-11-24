---
section: changes
---
- **`tzselect`, `pldd` and `/usr/bin/ld.so` ([#14](https://github.com/duskglow/libniji/issues/14), the rest):**
  `libc-bin` ships `/usr/bin/tzselect` (a program reading the installed `zone1970.tab` and `iso3166.tab`; its prompts,
  menus, answers and printed value match glibc 2.39's in scripted sessions, and it exits 1 at the end of input at any
  prompt; `-c` lists zones in great-circle order, D-31.34), `/usr/bin/pldd` (lists the objects loaded in a running
  process, `dlopen`ed ones included, as glibc's does, from the debugger rendezvous of the stopped process, D-31.35)
  and the link `/usr/bin/ld.so` to the loader; the coexist flavour has the link as `/usr/lib/niji/usr/bin/ld.so`.
  Differences: rows "tzselect -c order", "tzselect -n" and "tzselect without its tables" in
  [differences-from-glibc.md](differences-from-glibc.md).
