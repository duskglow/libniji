---
section: changes
---
- **Header fixes for gcc's Go dumper, C++ without exceptions, `QCMD` and `__cpu_mask`:** a macro defined as another
  macro now comes after that macro (`AF_INET` after `PF_INET`, `ENOTSUP` after `EOPNOTSUPP`), so
  `gcc -fdump-go-spec` keeps it and libgo's generated syscall constants include `AF_*` and `ENOTSUP`. In C++ built
  with `-fno-exceptions`, `<pthread.h>` declares the functions its `pthread_cleanup_push` / `pthread_cleanup_pop`
  macros call (compiler-rt's tsan runtime). `<sys/quota.h>` defines `QCMD` (systemd), and `<sched.h>` declares
  `__cpu_mask`, the element type of `cpu_set_t` (elogind).
