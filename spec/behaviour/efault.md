# efault

**Spec:** §0.6 C0.1 (error reporting; the codes are the kernel's), §13 (path-taking calls).
**Clauses:** `s0_6_c0_1_1`, `s0_6_c0_1_3`.

**Question (LV-4 finding, 2026-10-09):** a path-taking call is given a pointer the process
cannot read. Which calls fail with `EFAULT`?

**Probe:** `conformance/c/s13_efault.c` run with the argument `all` (the same program is the
conformance test, which runs without arguments). **Record:** `efault.glibc.txt` (one line per
call and pointer kind). Built with `cc -std=gnu17 -O1`, run on Zen 1 (glibc 2.39, Ubuntu 24.04).
Every trial runs in a forked child. Public interfaces only.

## Pointer kinds

* `PROT_NONE`: the first byte of a `PROT_NONE` page.
* `ptr1`: the address 1.
* `unterminated`: 'a' bytes up to the end of a readable page, `PROT_NONE` after: a string whose
  terminator is never reached.

Calls with a second, valid path argument (`rename`, `link`, `symlink`) test each argument in
turn. `link` uses an existing first path for the new-path trial: the kernel looks the old path
up before it reads the new one, so with a missing old path the answer is `ENOENT`.

## Result

Every path-taking call passes the pointer to the kernel and reports `-1`/`EFAULT` for all three
kinds: `access`, `euidaccess`, `faccessat`, `open` (also with `O_CREAT`), `openat`, `creat`,
`stat`, `lstat`, `fstatat`, `unlink`, `unlinkat`, `mkdir`, `rmdir`, `rename` (both arguments),
`chdir`, `chmod`, `chown`, `lchown`, `readlink`, `symlink` (both), `link` (both), `truncate`,
`execve`, `statfs`, `statvfs`, `utimensat`, `utime`, `utimes`, `mknod`, `mkfifo`, `fopen`.
`getcwd` and `readlink` with an unwritable result buffer, and `stat` with an unwritable
`struct stat`, also give `EFAULT` (the kernel writes the result).

The record lists the `EFAULT` answers only; calls that read the text or fill the buffer in user
space are not part of it.

## libniji

`KStr` holds the address and measures only when a consumer asks for the bytes, so a forwarded
path reaches the kernel unread and gets its `EFAULT`: every call listed above, and `opendir` for
all three kinds (S6: a defined error where the contract has an error channel).

Calls that read the text or fill a caller structure in user space themselves (`realpath`,
`gethostname`, `getdomainname`, `stat`'s result, which libniji receives from the kernel by value
and stores into the caller's `Option<&mut Stat>`, `execve`'s argument lists, `posix_spawn`,
`mq_open`) treat a pointer the process cannot use as the caller's error: the fault is at the
caller's own address, and nothing is written elsewhere. `conformance/c/s13_efault.c` checks every
`EFAULT` answer above.
