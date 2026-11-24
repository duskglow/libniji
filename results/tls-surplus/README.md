# Static TLS surplus: the reference against libniji (audit row HL-33)

Story TLS-SURPLUS-PROBE, 2026-11-23. A black-box probe of how much TLS objects opened by `dlopen`
after start-up can use before `dlopen` fails with `<path>: cannot allocate memory in static TLS
block`, on the reference (host C library, Ubuntu GLIBC 2.39-0ubuntu8.9) and on libniji (its
`ld-linux-x86-64.so.2` and `libc.so.6` from `cargo xtask so`, revision 0495c4684 of next), on a
Skylake-SP build host. No spec clause: a measurement, not a test.

* `tls_obj.c`: one shared object holding one TLS block of `SIZE` bytes, alignment 1, accessed with
  the TLS model `MODEL`. `initial-exec` objects carry `DF_STATIC_TLS` (their block must be in every
  thread's static area); `global-dynamic` objects do not.
* `probe.c`: opens the objects it is given in order (`RTLD_NOW`), touches each block, stops at the
  first failure and prints how many opened and the `dlerror` text. With `threads=1` a thread is
  created before the first `dlopen`, stays alive, and touches each new block too (it must get a
  block of its own). The same binary, built with the host's headers, runs on both: libniji's run
  only has `PT_INTERP` set to libniji's loader, and `LD_LIBRARY_PATH` for the probe process alone
  (libniji's loader never takes `libc.so.6` from a `RUNPATH`, COMPAT-37). Every libniji line shows
  `libc=libniji` (the probe reads `/proc/self/maps`).
* `run.sh <target/niji> <new work dir>`: the five measurements below; `probe.log` is its output
  (job `tls-surplus-probe-run3`).

Tunables: `GLIBC_TUNABLES` set to the reference's documented surplus tunables
(`glibc.rtld.optional_static_tls`, `glibc.rtld.nns`; the GNU C Library manual's dynamic linking
tunables), or unset (`-`).

## A. Largest single initial-exec block a `dlopen`ed object can have (bytes, 16-byte steps)

| tunables | reference, no thread | reference, thread first | libniji, no thread | libniji, thread first |
|---|---:|---:|---:|---:|
| unset | 1712 | 1712 | 65536 | 65536 |
| `optional_static_tls=0` | 1200 | 1200 | 65536 | 65536 |
| `optional_static_tls=512` | 1712 | 1712 | 65536 | 65536 |
| `optional_static_tls=4096` | 5296 | 5296 | 65536 | 65536 |
| `optional_static_tls=65536` | 66736 | 66736 | 65536 | 65536 |
| `optional_static_tls=1048576` | 1049776 | 1049776 | 65536 | 65536 |
| `nns=1` | 816 | 816 | 65536 | 65536 |
| `nns=16` | 5168 | 5168 | 65536 | 65536 |

On the reference the largest block is the tunable's value plus 1200 bytes (512 is the default:
1712); `glibc.rtld.nns` moves it too (1: 816, the default: 1712, 16: 5168). A thread created
before the `dlopen` changes nothing on either. libniji does not read either tunable: 65536 bytes,
its whole surplus, in every row.

## B. 1 KiB initial-exec objects opened one after another (of 160)

The reference opens 1 (unset, `optional_static_tls` 0 and 512), 5 (4096, `nns=16`), 65 (65536),
all 160 (1048576) and 0 (`nns=1`); libniji opens 64 in every row (64 KiB, no per-object overhead).
Threads first: the same on both.

## C. 16-byte initial-exec objects opened one after another (of 1100, tunables unset)

Reference: 107 (107 x 16 = 1712: no per-object overhead either). libniji: all 1100 (17600 bytes;
its module table grows past 512 modules). Threads first: the same.

## D. 16 KiB global-dynamic objects opened one after another (of 64 = 1 MiB, tunables unset)

| | no thread | thread first |
|---|---:|---:|
| reference | 64 (all) | 64 (all) |
| libniji | 4, the 5th fails: `cannot allocate memory in static TLS block` | 4, the same |

## E. Largest single initial-exec block after four 16 KiB global-dynamic objects (tunables unset)

Reference: 1712 (the same as with none open: the 64 KiB of global-dynamic TLS took none of the
initial-exec room). libniji: none (even a 16-byte block fails: the four objects filled the surplus).

## Reading

* Initial-exec (static) TLS: libniji's 64 KiB is 38 times the reference's default room (1712 bytes)
  and holds more in every row except `glibc.rtld.optional_static_tls` set above 64336, where the
  reference holds more (66736 at 65536). A thread created first changes neither.
* All TLS of `dlopen`ed objects: libniji places every `dlopen`ed object's block in the 64 KiB
  surplus, whatever its model (D-27.39; dynamic blocks are plan row DL-19's), so the total TLS of
  all `dlopen`ed objects is capped at 64 KiB. On the reference, 1 MiB of global-dynamic TLS opened
  and left the initial-exec room untouched. See the story's hand-back (handoff/TLS-SURPLUS-PROBE.md)
  for the rulings this raises.
