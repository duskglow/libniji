---
section: changes
---
- **`<complex.h>` and `<tgmath.h>` (D-10.135):** libniji now ships both headers, so programs that include them
  compile with `niji-gcc` and `niji-g++` (C++98 and C++03 `<complex.h>` and `<tgmath.h>` included). The complex
  functions in every floating type, `complex`, `I`, `_Complex_I` and `CMPLX`, `CMPLXF`, `CMPLXL` (and the `_FloatN`
  forms with `_GNU_SOURCE`) as constant expressions; each type-generic macro calls the same function, with the same
  result type, as on the reference system, for every argument type. The known limitation of these two headers is gone.
