---
section: known-limitations
---
- **Headers not shipped yet (D-X.168):** `<aio.h>`, `<mqueue.h>`, `<proc_service.h>`, `<thread_db.h>` and the
  Fortran `math-vector-fortran.h` are not among libniji's headers, so a program that includes one does not compile
  with `niji-gcc`. Workaround: build that package with `NIJI_SYSTEM_HEADERS=1` (the system's headers, which describe
  the same ABI); the system flavour has no workaround. The list is `validation/headers_unshipped.tsv`.
