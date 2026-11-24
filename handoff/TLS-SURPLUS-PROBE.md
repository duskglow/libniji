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

## The fix for item 2 (coordinator note on #7999: state it here, do not build it)

* **What moves off the surplus:** the block of every `dlopen`ed object WITHOUT `DF_STATIC_TLS`
  (accessed only through global-dynamic, local-dynamic or TLS descriptors). Each such block becomes
  a dynamic block, one allocation per thread per module, found through the dtv slot. Objects with
  `DF_STATIC_TLS` (initial-exec relocations) keep a surplus place as today, and their `TPOFF`
  relocations need one. Start-time objects are unchanged (all static, D-27.20).
* **Accesses:** `__tls_get_addr` reads the dtv slot as today. A `TLSDESC` relocation against a
  dynamic block uses the existing `liella_machine_ld_tlsdesc_dynamic` trampoline (D-27.36) with a
  resolver that returns the block's address for the calling thread. The static descriptor stays for
  surplus blocks.
* **Release:** at thread exit (`tls_release`, D-27.36's hook, which exists for this) and at
  `dlclose` of the module (DL-17). Freeing at `dlclose` walks the threads under the thread-list
  lock, as D-27.39's walk does.
* **D-27.39 then says:** "a `dlopen`ed object with `DF_STATIC_TLS` gets a surplus place (as now);
  one without gets dynamic blocks". The exhaustion text stays only for `DF_STATIC_TLS` objects.
  **DL-19** becomes "dynamic blocks for objects without `DF_STATIC_TLS`" and drops "surplus
  placement" (done by D-27.39). **D-27.49** (`RTLD_DI_TLS_DATA`) keeps its answer if blocks are
  eager (choice b).
* **Choices that need a ruling:**
  (a) **May non-static objects still take surplus room while it lasts?** That would keep today's
  static descriptor and the absence of any allocation. The cost is that the room fills, and a later
  initial-exec object then fails where it would have fitted (probe E: the reference kept its whole
  initial-exec room after 64 KiB of global-dynamic objects). Recommend no: the surplus is for
  `DF_STATIC_TLS` only.
  (b) **Eager or lazy dynamic blocks?** Eager means: allocate for every live thread at `dlopen`, in
  D-27.39's walk, and for a new thread in `tls_init`. Running out is then a clean failure:
  `dlopen` fails, or `pthread_create` returns `EAGAIN`. Its cost is memory for threads that never
  touch the module. Lazy means: allocate on first access in `__tls_get_addr` or the descriptor
  resolver. That uses no memory for threads that never touch the module, but it has no error path,
  so running out could only abort. Recommend eager: ruling #5594 says "a clean error, never a
  crash", and it keeps D-27.49's answer. A lazy variant only for very large blocks could follow
  with numbers.
  (c) **Where blocks come from:** the loader's own arena (the DL-19 row's wording), not the
  program's `malloc`. That keeps interposed allocators out of the loader's lock order. Recommend as
  the DL-19 row says.

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
