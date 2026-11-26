# header_probe.d: the probe's candidate names, one file per header family

`spec/tools/header_probe.py` tests candidate names (plan/headers.md section 2.2). Most come from the spec/abi
tables; the names no table spells come from the files here, one per header family or story.

* A file `<family>.py` defines `candidates()`, which returns `(ordinary names, tag names)` (any iterables).
* The probe loads every `*.py` here in file-name order and takes the union, so the order of the files never changes
  the result. Shared helpers (`tsv`, `ROOT`, `public_headers`, `guard_candidates`, ...) are imported from
  `header_probe` by name.
* `header_probe.family("<family>")` returns one fragment's module (header_checks/s07.py reads stage2-gaps-c).

To add names for a new header family, add a new file here. Do not add to another family's file or to
`header_probe.py`: two stories that each add a file merge without a conflict (HEADERS-FRAGMENTS).
