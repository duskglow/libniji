---
section: known-limitations
---
- **Headers not shipped yet (D-X.168):** `<complex.h>`, `<tgmath.h>` and the Fortran `math-vector-fortran.h` are
  not among libniji's headers, so a program that includes one does not compile with `niji-gcc` (nor C++98 or C++03
  `<complex.h>` and `<tgmath.h>`, which include the C header); `math-vector-fortran.h` waits for a Fortran output
  form of the header generator. Workaround: build that package with `NIJI_SYSTEM_HEADERS=1` (the system's headers, which describe the same
  ABI); the system flavour has no workaround. The list is `validation/headers_unshipped.tsv`.
