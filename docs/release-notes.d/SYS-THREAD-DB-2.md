---
section: changes
---
- **`libthread_db.so.1` (D-22.17):** debuggers can list the threads of a statically linked libniji program, show
  each thread's backtrace and read its thread-local variables, live or from a core file (tested with gdb 15.1).
  `libc6` (`niji6` in the default flavour) ships the library and `libc6-dev` (`niji6-dev`) the `libthread_db.so`
  link. Thread debugging of dynamically linked programs waits for the loader's debugger interface (`_r_debug`).
