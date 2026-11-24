---
section: changes
---
- **Thread exits without an unwinder ([#25](https://github.com/duskglow/libniji/issues/25)):** in a
  `-static-libgcc` program that loads no `libgcc_s.so.1`, a thread exit (`pthread_exit`, cancellation) with unwind work
  on its stack (`-fexceptions` cleanups, C++ destructors or handlers) no longer hangs: the process stops with a message
  naming the cause and the fix (D-22.23; registered compatibility option COMPAT-38, row in
  [differences-from-glibc.md](differences-from-glibc.md)).
