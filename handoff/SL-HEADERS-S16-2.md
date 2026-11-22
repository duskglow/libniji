# SL-HEADERS-S16-2: slice 16 continuation (INCOMPLETE: thread_db, proc_service, a.out, stab not generated; P2, P3 and P6 not green)

Branch `task/sl-headers-s16-2`. Say "slice 16" in commits. Inherited head was clean on clippy and static (step 0).

**Done.**
* `sys/elf.h` (#6423): black-box probe on the reference (`<sys/elf.h>` alone, after `<elf.h>`, `<stdint.h>`+`<elf.h>`, `<link.h>`,
  `<sys/types.h>`, `<stddef.h>`; c89, c99, gnu17, c2x, C++17; 14 `_*_SOURCE`/`_TIME_BITS`/`_FORTIFY_SOURCE` levels; -m32; exit status only,
  diagnostics discarded; control: `<elf.h>` compiles, `__has_include(<sys/elf.h>)` is 1): every one fails. Unconditional, so libniji's
  generated header is the `#error` pointing to `<elf.h>` (no differences row).
* `sys/platform/x86.h` (#6424): `headers/sys/platform/x86.h.in`, generator marker `@@x86features`, table `spec/abi/x86-features.tsv`
  (172 names; leaf/sub-leaf/register/bit from the Intel SDM Vol. 2A CPUID tables, five AMD-only bits from AMD APM Vol. 3 App. E, the
  table named per row), written by `spec/tools/x86_probe.py` (nothing typed in templates). Probe table (every name: index, word, bit
  in both arrays): `handoff/SL-HEADERS-S16-2/x86-probe-table.tsv`. Facts probed: sizeof 32, alignof 4, `cpuid_array` at 0 (`unsigned int[4]`),
  `active_array` at 16; both macros have type `_Bool`; the header includes `<stdbool.h>`; the index of each name equals the reference's
  (0 leaf 1, 1 leaf 7.0, 2 0x80000001, 3 0xD.1, 4 0x80000007, 5 0x80000008, 6 leaf 7.1, 7 leaf 0x19, 8 leaf 0x14); indices 0..5 match the
  CPUID instruction on the probe host (`cpuid-match`), 6..8 are `sdm-only` (zero words on that Zen 1 host). `CPU_FEATURE_ACTIVE(SHSTK)` and
  `(IBT)` are false for every structure (`active=never`). 15 of 187 candidate names are not defined by the reference (list in the probe
  log); names the SDM has that the candidates lack are unprobed: the table is the intersection. `spec/tools/x86_header_check.py` proves
  the generated header against the table (run, C++98/17/23, sizeof/offsetof): 172 features, 0 failed.
* header_check: 0 failed of 2165 (was 71 of 2087). Fixes: `layouts.tsv` rows for `Dl_serpath`, `struct cpuid_feature`; anonymous unions
  whose row carries a `size N rows M` note; layout-only member aggregates (`La_x86_64_xmm/ymm/zmm/vector`) as guarded typedefs;
  `La_x86_64_regs/retval` also in C++; P1 of `sys/profil.h` only checks `struct prof` (see below).

**Open (in order).**
1. P2 (`proofs.log` kept on the host, `~/work/SL-HEADERS-S16-2/proofs.log`): 52 rows only in libniji, 285 only in the reference over the slice's
   headers. Not analysed row by row. Visible classes: reserved `__*` / `_X_H` names (omitted by `reserved.tsv` policy: the compare must
   ignore them, as P1 does); `gnu/lib-names.h`: libniji makes feature macros visible (the reference's rows have none) and has an
   `include-error` row for time64only; `link.h` La_x86_64_* typedefs in C++ (reference C only; libniji declares them because the C++
   spelling of the `la_*` prototypes in typedefs.tsv names them: ruling needed); `link.h`/`sys/gmon.h` `be16toh`/`le64toh`.
2. `sys/profil.h`: the reference includes `<sys/time.h>` (slice 7, not generated): `fd_set`, `FD_*`, `sigset_t`, pthread types, `itimerval`
   ... are visible there at every level. libniji's lacks them until slice 7 exists; add the include then and drop `PENDING_INCLUDE`
   in `header_check.py`.
3. P3: one row differs: `__monstartup` (sys/gmon.h) is not declared (reserved row `emit` added, but the generator still omits it:
   not debugged).
4. P6 (`conformance/run.sh --niji-headers s27_dl*`): 1 of 6 passes; four are "not covered" (sys/auxv.h, pthread.h, fcntl.h, sys/stat.h:
   other slices); `s27_dladdr` does not build: `Elf64_Phdr` had the member `__p_type` (layouts.tsv rows respelled `p_type` by hand,
   PROVENANCE line; abi_extract.py needs a look at why its DWARF names it so), and `RTLD_DL_LINKMAP`, `ELFMAG` are in no table
   (no `visibility.tsv`/`constants.tsv` row: a candidate-list gap of the probes).
5. Not generated: `thread_db.h`, `proc_service.h` (slices 8/9), `a.out.h`, `stab.h` (no tables), see the first handoff.
6. Decision record (series D-X): `@@sonames`, `@@x86features`, anonymous members, audit hooks, `sys/elf.h` as an error header, the profil.h
   scope and `NAMED_IN_CXX`; not written (pending records dir convention not checked).

**Jobs.** `s16-2-hc0..3` (header_check, last: 0 failed), `s16-2-proofs` (P2, P3, P6, x86), `SL-HEADERS-S16-2-plants`, `SL-HEADERS-S16-2-gate`
(started on the head named in the report). Time went to host builds per generator round and the slot queue.
