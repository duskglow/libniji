# DL-DYNAMIC-RELOC handoff (end of leg 3 of run 1)

Branch `task/dl-dynamic-reloc`, base origin/next de4cdfc8d. Role mei:~/w/s3. Brief: ~/state/briefs/DL-DYNAMIC-RELOC.md.
Decision record in force: **D-27.60** (records/decisions/D-27/D-27.60.toml). No earlier record or differences row
says the opposite (searched records/ and docs/differences-from-glibc.md; D-27.18 reads the program from its image:
consistent). Report: this file and handoff/reports/DL-DYNAMIC-RELOC.json.

## What the branch does

ld.so writes the address entries of each loaded object's dynamic array as run-time addresses (bias added), before
any user code: its own at its start (`niji_ld_start`, after its relocation), the program without objects (`run`),
every start-time object in `link` (after relocation, before RELRO protection), every `dlopen`ed object in
`load_set`. Only an array wholly inside a writable `PT_LOAD` is written (`elf::dynamic_published`, the one rule
for writers and readers). Readers that need link-time values undo it on their copy (`image_view(.., published)`)
or read with the rule (`readers::soname_at`). Tag set `elf::RUNTIME_ADDRESS_TAGS`.

## Probe table (tools/probe/dl/dynreloc; spec/behaviour/dynlink.md "The loaded dynamic array")

| object | reference | libniji origin/next | libniji head |
|---|---|---|---|
| program, start lib, libc.so.6, ld.so, dlopen'ed lib | PLTGOT HASH STRTAB SYMTAB RELA JMPREL RELR GNU_HASH VERSYM + base; all others as linked | all as linked | as the reference |
| vDSO | untouched | untouched | untouched |
| `_DYNAMIC` of the program, systemd pattern (DT_RUNPATH via DT_STRTAB) | reads "$ORIGIN" | link-time offset, not a pointer (conformance exits 97) | reads "$ORIGIN" |

Raw outputs (mei): reference `~/work/dl-dynamic-reloc/ref3.txt`, before `niji-before.txt`, after: log of job
`dl-dynamic-reloc-step1b` section `### probe`.
INFORMATIONAL (not this story): a static-pie's own array (reference adjusts, libniji not); DT_DEBUG not set by
libniji; libniji's `dlpi_name` of start libs is the SONAME (reference: the path); DT_REL unobservable on x86-64.

## Proof

- Conformance `s27_dynamic_address_entries` (clause test `s27_1_c_8`): FAIL on the base products, PASS on head
  (job dl-dynamic-reloc-step1b). Unit tests: `boot::tests::publish_dynamic_adds_bias_to_address_tags`,
  `reloc_publish_dynamic_in_the_model`.
- Plants: handoff/DL-DYNAMIC-RELOC-plants.txt (6: dlopen skip, link skip, ld.so skip, reader undo skip, soname
  bias, tag set short of STRTAB). Job dl-dynamic-reloc-plants.

## Start-up counts (fast profile, mei Zen 1; base = a RAM copy of the role with the story's files at the base)

| measure | base | head |
|---|---|---|
| hot_text_products dynamic startup-only (bound 89) | 87 | 89 |
| other hot_text_products rows (coreutils, localtime(+mktime), startup-only, setlocale+printf) | 44 36 36 11 43 | same |
| page_faults_dyn `true`, perf instructions:u / faults (rpath build, setarch -R, min of 7, deterministic) | 85811 / 36 | 88306 / 38 |
| libc layout-bins.tsv | - | only llvm hash suffixes differ, no size change |
| ld.so text | 0x46a9e | 0x49042 (+9.6 KB) |

Where the 2 faults come from (perf page-fault addresses, base vs head): the array writes cost none (every
`.dynamic` lies inside RELRO; ld.so rw-p and program rw-p file-page faults 4 and 2 on both). +1 ld.so text page
(ld.so grew 9.6 KB; most is inlining churn: sort network +4983 / quicksort -4970, `fini_all` -4618 /
`exit_order` +4347, `list` +2951; the story's own functions ~2.6 KB), +1 ld.so bss page (`cpu::STATE` and
`AVX2_ERMS_TO` now placed after the 20 KB `readers::TABLE`, on a page of their own). ld.so has no traced layout
until LAYOUT-LDSO-ORDER (§9: loader fault deltas move with loader growth). A rebuild with an unrelated code change
measured 88 (jitter). Bound holds; no bound changed.
Tried and reverted: a bitmask tag test in place of the 9-way `contains` (88313 instructions vs 88306: no gain).
Fix-forward for the +2.5K instructions: `Elf::published_dynamic` walks the program headers twice per call (three
calls per start); one pass reading only type/flags/vaddr/memsz is the next thing to measure (not done: budget).

## Jobs (final head)

dl-dynamic-reloc-lint, -gate (1134 passed, 0 failed; conformance 140/0; peak 2.3G), -xtest, -fullhtp (chained by
~/work/dl-dynamic-reloc/final.sh), -plants4 (plants file passed from ~/work, same as the committed one; role was
busy), -counts1 (base vs head counts). SUPERSEDED: -plants and -plants2 (refused: unanchored ldso-skip), -plants3
(killed: msg patterns fixed), -step2 (reverted bitmask variant), -static1 (trace fix since).
Coverage gap (INFORMATIONAL): the no-objects program path in `run` publishes too but no test reaches it (no plant).

## Exact next step

Read the report json. If landing: nothing left on the branch. Optional: the one-pass `published_dynamic` above.
