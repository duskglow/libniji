---
section: changes
---
- **`<thread_db.h>`, `<proc_service.h>`, `<aio.h>` and `<mqueue.h>` ship (D-X.168):** a debugger's thread support
  (`td_*` calls into `libthread_db.so.1`, `ps_*` callbacks) and programs using POSIX asynchronous I/O or message queues
  now compile with `niji-gcc` against libniji's own headers. `mq_open` called without its mode and attributes under
  `_FORTIFY_SOURCE` goes through the checked entry `__mq_open_2`. The Fortran `math-vector-fortran.h` and
  `<complex.h>`/`<tgmath.h>` remain unshipped (Known limitations).
