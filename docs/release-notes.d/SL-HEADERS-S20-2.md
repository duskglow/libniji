---
section: known-limitations
---
- **Headers not shipped yet (D-X.168):** the Fortran `math-vector-fortran.h` is not among libniji's headers (it
  waits for a Fortran output form of the header generator), so a Fortran program that includes it does not compile
  with `niji-gcc`. Workaround: build that package with `NIJI_SYSTEM_HEADERS=1` (the system's headers, which describe
  the same ABI); the system flavour has no workaround. The list is `validation/headers_unshipped.tsv`.
