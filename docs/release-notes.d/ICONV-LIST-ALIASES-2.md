---
section: changes
---
- **`iconv -l` lists more names:** it now also lists the names in the headers of the installed charmaps (the
  `code_set_name` and alias lines, plain or gzip), for example `BIG5-CP950`, `ISO-IR-13` and `CYRILLIC-ASIAN`: 1028
  names instead of 982 with the charmaps of the reference system, and every listed name converts.
