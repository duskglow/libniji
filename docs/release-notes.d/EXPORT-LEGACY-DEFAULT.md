---
section: changes
---
- **`bindresvport` and `__libc_sa_len`:** `libc.so.6` and `libc.a` now export both at their default version
  (`@@GLIBC_2.2.5`, as stock), so a program built against libniji links them by name. `bindresvport` behaves as
  before (a free reserved port, 512 to 1023); `__libc_sa_len` returns the socket-address size of each address family
  stock's does (`0` for any other).
