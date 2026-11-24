---
section: changes
---
- **`C.UTF-8` source in `locales` ([#14](https://github.com/duskglow/libniji/issues/14)):** the `SUPPORTED` list names
  `C.UTF-8 UTF-8` and `locales` ships `usr/share/i18n/locales/C`, so `localedef -i C -f UTF-8` works on a system with
  only libniji's packages. `localedef` writes a source that places every character in code point order as the small
  `LC_COLLATE` the built-in `C.UTF-8` has, in place of tens of megabytes, and refuses an empty `LC_COLLATE` section
  with status 1 and no file.
