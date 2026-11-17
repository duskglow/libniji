# libc.so.6's old `pthread_sigmask@GLIBC_2.2.5` against its default `pthread_sigmask@GLIBC_2.32` (spec s0_6_c0_6_0, s20_3_c_1; SYS-COMPAT-VERSIONS-REST)

Black-box observations of the reference library (glibc 2.39, Ubuntu 24.04, x86_64, Zen 1 host), made
on 2026-11-17. Debian trixie's btscanner, pbzip2 and wavemon bind `pthread_sigmask@GLIBC_2.2.5`
(their version need names `libpthread.so.0`; the reference's `libpthread.so.0` defines the version
node and its `libc.so.6` the symbol). `spec/abi/compat-plan.tsv` gave the row `behaviour`, "LG-0b probe".

Probe: `crates/core/tests/oracle/pthread_sigmask_compat.c` binds both versions with `.symver` and calls
each on the same 140 cases: `how` = `SIG_BLOCK`, `SIG_UNBLOCK`, `SIG_SETMASK`, 3 and -1; a null set
and six sets (`SIGUSR1`; `SIGUSR1` with the kernel's signals 32 and 33; `SIGKILL`, `SIGSTOP` and
`SIGUSR2`; all 64 bits; 34, 50 and 64; `SIGINT` and `SIGTERM`); a null and a non-null old set; from a
starting mask of `SIGINT` alone and of `SIGINT` with 32 and 33. The starting mask is set, and the
mask afterwards read, with the raw `rt_sigprocmask` system call. Observed per call: the return value,
`errno` (preset to 999), the first 8 bytes of the old set (preset to `0xa5` bytes), how many of its
other 120 bytes changed, and the kernel mask afterwards. Record:
[`pthread-sigmask-compat.glibc.txt`](pthread-sigmask-compat.glibc.txt).

## Results

* The old version gives the default version's observations in all 140 cases: `pthread_sigmask@GLIBC_2.2.5`
  is an `alias` row, exported as a `.symver` stub that jumps to the default (`cargo xtask compat-gen`).
* For both versions: an invalid `how` returns `EINVAL` (22) with `errno` unchanged, the mask unchanged
  and the old set not written; a valid call writes 8 bytes of the old set and leaves the other 120
  unchanged; signals 32 and 33 in `set` are ignored by `SIG_BLOCK` and `SIG_UNBLOCK` (a blocked 32
  or 33 stays blocked), and `SIG_SETMASK` leaves both unblocked whatever the set and the starting
  mask; `SIGKILL` and `SIGSTOP` are dropped by the kernel.

libniji's default version differs from the reference's in ways this compat version does not touch
(both versions alike in each library): it writes the whole 128-byte old set (zeros after the
kernel's 8 bytes); a set can block signal 33, and the old set it reports hides a blocked 32; and with
a null set it still rejects an invalid `how` with `EINVAL`, where the reference ignores `how` (POSIX:
"if set is a null pointer, the value of the argument how is not significant"). The test masks
exactly these.

Test: `old_pthread_sigmask_binds_and_matches_the_reference` (crates/core/tests/crt_products.rs) runs
the probe linked against libniji's products and against the reference and requires the same text.
