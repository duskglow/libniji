# SYS-THREAD-DB: libthread_db.so.1 for debuggers (audit row 11)

Size: M. Kind: code. From plan/package-parity-audit.md.

1 of 4000 sampled trixie packages has `NEEDED libthread_db.so.1` (`consumers-users.tsv`), and debuggers load it by
name to list a process's threads, so thread debugging of a libniji program fails without it (`ref-elf.tsv:285`;
plan/system-libc.md lists it for phase 14).

1. `libthread_db.so.1` implementing the thread_db interface debuggers call (`td_ta_new`, `td_ta_thr_iter`,
   `td_thr_get_info`, TLS lookup …), with the version nodes the reference uses (names and versions from
   `ref-dynsyms.tsv` only), reading libniji's own thread structures through the debugger's `ps_*` callbacks.
2. `libthread_db.so` in libc6-dev.
3. A decision record for how the library finds libniji's thread list (its own symbols, not GLIBC_PRIVATE ones).

Proof: gdb lists the threads and their TLS of a libniji program with 4 threads; `info threads` in a core file;
plants.
