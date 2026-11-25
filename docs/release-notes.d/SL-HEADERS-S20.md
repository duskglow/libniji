---
section: changes
---
- **libniji's own C headers (D-X.168, D-X.169):** `niji6-dev` installs them in the sysroot
  (`/usr/lib/niji/usr/include` and its `x86_64-linux-gnu` directory) and the system flavour's `libc6-dev` in
  `/usr/include`. `niji-gcc` and `niji-g++` compile against them by default (`-nostdinc`, then the C++
  library's directories for C++, libniji's headers, the compiler's own directory and the kernel headers);
  `NIJI_SYSTEM_HEADERS=1` compiles with the system's headers instead. The C++ library's headers compile on them in
  every `-std` from `c++98` to `c++23`, but for the headers listed under
  Known limitations.
