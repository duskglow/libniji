# TLS-SURPLUS-PROBE: the reference's static TLS surplus against libniji's 64 KiB (audit HL-33)

State: branch `task/tls-surplus-probe` (from next 0495c4684). Evidence only: no library code, test or
record changed (the brief: "change nothing here"). Probe, table and log in `results/tls-surplus/`.

## Result (job tls-surplus-probe-run3, build host mei, products of 0495c4684)

| | reference (GLIBC 2.39, Ubuntu) | libniji |
|---|---:|---:|
| largest single initial-exec block, tunables unset | 1712 | 65536 |
| the same, a thread created first | 1712 | 65536 |
| `glibc.rtld.optional_static_tls=N` | N + 1200 (0: 1200, 4096: 5296, 65536: 66736, 1 MiB: 1049776) | 65536 (not read) |
| `glibc.rtld.nns` 1 / 16 | 816 / 5168 | 65536 (not read) |
| 1 KiB initial-exec objects opened in a row | 1 | 64 |
| 16-byte initial-exec objects opened in a row | 107 | 1100 of 1100 |
| 16 KiB global-dynamic objects opened in a row | 64 of 64 (1 MiB) | 4 (5th: `cannot allocate memory in static TLS block`) |
| initial-exec room left after four 16 KiB global-dynamic objects | 1712 | 0 |

Threads created first change nothing on either side, in any row.

## Rulings needed

1. **HL-33 size: keep 64 KiB** (recommend). It is 38 times the reference's default room. The
   reference only holds more when `glibc.rtld.optional_static_tls` is set above 64336. Tunable
   (owner's call): recommend honouring `glibc.rtld.optional_static_tls` raise-only, so the surplus
   becomes max(64 KiB, value). It would be read cold, only when `GLIBC_TUNABLES` names it, and the
   cost is address space in each thread's area. Without it, a deployment that sets the tunable that
   high to fit a large initial-exec block gets the `dlopen` failure on libniji. It needs a D-27 row
   and the start-up checks (loader start-up code).
2. **The 64 KiB bounds ALL TLS of `dlopen`ed objects, not only static TLS** (flag #7999). D-27.39
   places every `dlopen`ed block in the surplus whatever its model (dynamic blocks are plan row
   DL-19's, not landed). On libniji, a process can open only 64 KiB of TLS in total through `dlopen`.
   On the reference, 1 MiB of global-dynamic TLS opened. This is the kind of artificial limit the
   owner's 11-17 ruling removes, and the audit's "design bound" premise only holds for
   `DF_STATIC_TLS` objects. Recommend: schedule DL-19's dynamic blocks, for objects without
   `DF_STATIC_TLS` only, so the surplus is kept for the objects that need it. Until it lands,
   record the limit as a known limitation / differences row (neutral wording). No grep hit in
   docs/differences-from-glibc.md or the release notes describes it today.

## Things not to redo / notes

* libniji's loader never takes `libc.so.6` from a program's `RUNPATH` (COMPAT-37, a recorded
  difference). A glibc-built probe runs on libniji with `PT_INTERP` set to libniji's loader and
  `LD_LIBRARY_PATH` set for the probe process only. Never put `LD_LIBRARY_PATH` on `env`/`timeout`,
  because the host's own programs then map libniji's `libc.so.6` (runs 1 and 2 measured nothing on
  libniji for those two reasons; run 3 is the valid one).
* Rerun: `bash results/tls-surplus/run.sh target/niji <new RAM dir>` after `cargo xtask so` (about
  60 s).

## Next step

The owner's rulings on 1 and 2. If 2 is accepted: a DL-19 brief (dynamic blocks for objects without
`DF_STATIC_TLS`), and the D-27.39 / differences rows. Plan rows: D-27.39, D-27.36, D-27.65,
plan/06-phases/17-phase-14-dynamic-linking.md item 23 (DL-19), plan/hardcoded-limits.md HL-33.
