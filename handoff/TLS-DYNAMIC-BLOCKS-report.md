# TLS-DYNAMIC-BLOCKS report (DL-19): on-demand TLS blocks for dlopened objects

Branch `task/tls-dynamic-blocks`. Records: D-27.94 (amends D-27.39), D-27.95 (amends D-27.49).
All measurements on a Zen 1 host (60 vCPU), user-space instructions (perf_event_open, `exclude_kernel`),
deterministic between runs (3 runs each, identical). Probe sources: scratch `threadcost.c`,
`tlsacc.c` (not committed: measurement tools, not tests).

## What changed

* Objects without `DF_STATIC_TLS` get their TLS block per thread at first access (`__tls_get_addr`,
  a TLS descriptor, `dlsym` of a TLS symbol), freed at thread exit and at `dlclose`; no static place.
* Static surplus = `glibc.rtld.optional_static_tls` + 1200 bytes, 1712 by default, as on the
  reference (owner 2026-11-23 on #8009; replaces the 64 KiB and the provisional raise-only rule; no
  differences row). `--list-tunables` prints the tunable's value (0x200 by default, as the reference).
* On-demand TLS descriptors: `niji_machine_ld_tlsdesc_ondemand` (tlsdesc.S) reads the thread's
  vector (generation, slot count, slot) and only falls back to the state-saving trampoline when the
  thread has no block yet. Records are four words `[resolver, module, offset, &GENERATION]`.

## Per-thread cost of the surplus size (the evidence for the decision)

Same libniji build (this branch), surplus 1712 (default) vs 65536 (`optional_static_tls=64336`):

| row | libniji 1712 | libniji 65536 | reference 1712 | reference 65536+1200 |
|---|---:|---:|---:|---:|
| pthread_create+join, creator thread instr/call | 20,448 | 20,448 | 1,226 | 1,226 |
| pthread_create+join, all threads instr/call | 40,231 | 40,231 | 2,252 | 2,266 |
| RSS per idle thread (100 threads) | 16.40 KiB | 16.40 KiB | 8.20 KiB | 12.16 KiB |
| VSZ per idle thread | 8185.3 KiB | 8248.7 KiB | 8114.0 KiB | 8114.0 KiB |
| usable stack, 16 KiB (`PTHREAD_STACK_MIN`) thread | 16,175 B | 16,175 B | 11,871 B | pthread_create EINVAL |
| usable stack, 64 KiB thread | 65,327 B | 65,327 B | 61,023 B | pthread_create EINVAL |
| start-up, empty `main` (ld.so run as a program) instr | 189,510 | 194,544 (tunable parsed) | 122,117 | 124,506 |
| start-up page faults | 66 | 68 | 52 | 52 |

Reading: on libniji the surplus is untouched anonymous memory beside the stack (not carved from it),
so 64 KiB costs 63 KiB of address space per thread and nothing measurable in instructions, RSS or
usable stack. The surplus is NOT part of the THREAD-CREATE-COST gap (40,231 at both sizes). The
decision (owner): match the reference anyway (1712), since a larger default needs a good reason and
the surveys (1,210 objects on a build host; 2,955 on a desktop-like root) found no `DF_STATIC_TLS`
object needing more than 136 bytes.

## Before/after (like-for-like: origin/next 0495c4684 built with the same `cargo xtask so`)

| row | before (next) | after | reference |
|---|---:|---:|---:|
| create+join, all threads instr/call | 40,228 | 40,231 | 2,252 |
| create+join, creator instr/call | 20,450 | 20,448 | 1,226 |
| RSS per idle thread | 16.40 KiB | 16.40 KiB | 8.20 KiB |
| start-up instr (empty main) / faults | 190,257 / 66 | 189,510 / 66 | 122,117 / 52 |
| TLS access, start-up object, `__tls_get_addr` (instr/access incl. 10 loop+call) | 55 | 55 | 27 |
| TLS access, start-up object, TLS descriptor | 16 | 16 | 17 |
| TLS access, dlopened GD object, `__tls_get_addr` | 55 | 55 | 27 |
| TLS access, dlopened object, TLS descriptor | 16 (static place) | 38 (84 before the fast path) | 17 |
| create+join, creator user cycles/call (indicative: cycles, loaded host) | 17,554-22,978 (64 KiB zeroed on each stack reuse) | 10,631-11,830 (1712 B) | 1,972-2,548 |

Per-reuse zeroing (coordinator 16:11, D-22.6): a cached thread stack clears its whole static area,
the surplus included, on every reuse (`take_mapping`, thread.rs). That is one fill call, so it does
not show in instruction counts (identical at both sizes) but does in cycles: on this branch 1712 B
gives 10.6-11.8k creator cycles per create+join, 64 KiB requested through the tunable 18.1k (one
run); next's fixed 64 KiB 17.5-23.0k. About 7k cycles per thread start saved by the smaller default.

The dlopened TLS-descriptor row is a COST against next (16 -> 38): the block is no longer static,
so the descriptor reads the thread's vector (24 instructions: 3 saves, generation, count and slot
checks). Start-up objects and surplus-placed (initial-exec) objects are unchanged (55 / 16), as the
brief requires. Fix-forward: a shorter descriptor path (2 saves; the reference's row is 17).

(The ~/wl `-release-dyn` products of next are another build flavour: 36,026 create+join, 179,166
start-up; not comparable with a `cargo xtask so` build, so not used.)

`__tls_get_addr` at 55 against the reference's 27 is next's (unchanged here): INFORMATIONAL.

## Table D and the other probe tables (results/tls-surplus/run.sh, after)

* D (16 KiB global-dynamic objects, of 64): libniji 64 of 64, no thread and thread first (was 4).
* A (largest initial-exec block): libniji = the reference + 16 bytes in every `optional_static_tls`
  row (1728 vs 1712 at the default, 1216 vs 1200 at 0, 66752 vs 66736 at 65536): libniji's surplus
  starts after its initial blocks without the reference's internal slack. `glibc.rtld.nns` rows
  differ (not read: one namespace while `dlmopen`, DL-22, is absent).
* Survey sizes (coordinator 15:32): global-dynamic objects of 46,148 and 30,400 bytes (numpy
  `_multiarray_umath`, libcholmod) open together and work, threads 0 and 1, both libraries.

## Tests and plants

* `s27_dlopen_tls_dynamic` (s27_3_f_0): table D's case, per-thread blocks, dlinfo/dl_iterate_phdr
  NULL before first access, IE object after 64 GD objects, a 2 KiB IE object refused at the default
  room, 200 open/close cycles without growth, OOM exit 127, tunable 130048 opens a 128 KiB IE object
  (only under value + 1200).
* `dlopen_tls_modules_past_the_vectors_in_place` (niji-ld): 1100 objects with initial-exec variables
  need ~17 KB of static room; the run now sets `optional_static_tls=32768`.
* Plants (`cargo xtask plants`): PLANTS_RESULT

## Gate

GATE_RESULT

## Rule slips

Leg 1: one `sed -i` and one python in-place rewrite. Leg 3: one python in-place rewrite of
`s27_dlopen_tls_dynamic.c` (header comment and helper list); should have been Edit-tool edits; not
refused, content verified by the gate's conformance run.
