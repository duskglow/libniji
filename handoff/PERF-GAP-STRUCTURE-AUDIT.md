# PERF-GAP-STRUCTURE-AUDIT (investigate; no product change)

**State.** Branch `task/perf-gap-structure-audit` from `next` 0495c4684. Finished. Deliverable:
`results/perf-gap-structure/README.md` (gaps, attribution, assumption verdicts, ranked paths to parity); probes in
`results/perf-gap-structure/probes/`. Report json: `handoff/reports/PERF-GAP-STRUCTURE-AUDIT.json`. Measured on
mei (role s2 for the link drivers; products `~/wl/products/0495c4684...-release{,-dyn}`; work dir
`~/work/perf-gap-structure-audit/`, outputs in `out1/`, `out5/`). Jobs (mei, kind probe):
perf-gap-structure-audit-probe1 .. probe8.

**Findings (README sections 0 to 5).**
1. Per call and static start-up libniji is at or ahead of the reference. The gaps: dynamic start-up (every
   process: 0.92 to 1.26 total cycles on nine distribution binaries, faults 1.11 to 1.40), thread create (36,015
   vs 2,265), `strcoll` UTF-8 (1.9x instructions, 2.8x cycles), `towupper` (1.65x), first-use faults.
2. Loader: 1,510 user instructions per imported symbol (reference 643 eager, 37 lazy); libniji binds eagerly even
   under `-z lazy`; distribution binaries are `BIND_NOW` anyway. Gap model: ~55k fixed + ~870 per symbolic
   relocation (true: 0.1% off, ls: 17%). Exact gdb trace of the empty dynamic main: symbolic relocations 63.5k
   (40%), per-object tables 21k, views/checks ~22k, RELR 16k; libc.so.6 init only 2.5k.
3. Faults: read faults on file mappings already fewer (19 vs 28); the excess is written pages (anon, stack,
   writable data): in `ls` +28 anon, +10 stack, +6 libc rw, +7 libc text; identical system libraries fault
   identically under both loaders. Heap writes 9 of the anon pages in `ls`; loader frames > 4 KiB.
4. Kernel cycles are ~80% of dynamic start-up; libniji's total-cycle excess (+8.8%) is ~95% kernel-side; ~+160k
   kernel instructions per process are unattributed (fault mix priced by unit costs is ~60k cheaper for libniji).
5. Thread-create cause (KeyTable::span, D-22.5) reproduced on Skylake-SP.

**RULING NEEDED (in the hand-back).** (a) Read-only root on one build host to copy kallsyms for a kernel-symbol
profile of start-up (the kernel residual; recommend yes). (b) A RETHINK of the loader's symbol lookup (validate
once at map time, version index per object pair; target <= 300 instructions per symbol) before more layout or
relocation-loop stories on dynamic start-up (recommend). (c) Add kernel instructions/cycles to the start-up rows
the gate watches (recommend).

**Do not redo.** The four-build matrix, the exact start-up trace, the per-import probe, the distribution-binary
runs, the kernel unit costs (probes 1 to 8). Sampled start-up profiles on these VMs are throttle-biased: use
`probes/trace_start.py` (gdb single-step, ~40 s for 158k instructions).

**Next step.** Brief, in rank order (README section 5): THREAD-KEY-WATERMARK (DECISION-EVIDENCE-AUDIT's
proposal), a loader lookup RETHINK, lazy per-object tables, written-page reduction (loader frames, heap first-use
metadata, libc start-up-written data), the kernel-residual profile, a `strcoll` collation redesign, `towupper`
trace. Then re-run probes 1, 5 and 7 on the result.
