# DYN-START-SPEED (handoff)

## State
Branch `task/dyn-start-speed` (stacked on DYN-START-PARITY / -2, not yet in next; no merge done). Code commits:
8ab452436 (step 2, RELACOUNT prefix loop), 4cb8daf5b (step 1a, verdef record scan), 348a4f666 (step 1b, per-pass
version-name memo), fa6338555 (bounds + tests + plants). Final head = the commit carrying this note.
Code/test head 3be912577; final jobs on mei role `~/w/s1` (Skylake-SP), all rc 0 on it:
xtest1 (clippy -D warnings + xtask static + `cargo xtask test`: 6858 + 5333 passed), gate2 (direct tier
`affected --base origin/next --run`, not escalated), plants2 (`handoff/DYN-START-SPEED-plants.txt`: 4/4 CAUGHT),
release1 (`NIJI_FULL_PROFILE=1 cargo xtask so` + hot_text_products: 29 passed; release empty main 807,500
instructions vs bound 812,400, startup-only faults 74 vs bound 76), precheck1.
Superseded: lint1, gate1 (head fa6338555, green), plants1 (2 survivors: the verdef plant was self-correcting and the
memo-key plant needed slot collisions the corpus could not make; fixed in 3be912577). Measurement runs (not proofs):
base0/cnt0/prof0, s2, s1, prof1, s1b, fast1.
Note: the release bound 812,400 sits 0.6% above the measured 807,500 (set from execcount before the in-test value
existed); instruction counts are deterministic for given products, so it was left rather than rerun every job.

## Measurements (execcount, ASLR off, median of 101, release products, Skylake-SP, load avg 3-7 of 112 threads)
Counter: `~/work/DYN-START-SPEED/execcount.c` = the repo's oracle/exec_instr.c plus a software page-fault counter
(enable_on_exec, kernel-mode faults included: glibc reads 51 here where DYN-START-PARITY's tool read 49).
Programs: `~/work/DYN-START-SPEED/progs` (m0 empty main, m3 three libraries, mt clock_gettime); script
`~/work/DYN-START-SPEED/parity.sh PRODUCTS TAG RUNS` (GLIBC=1 adds the host C library), `relmeasure.sh TAG` (builds
release products into /mnt/zram/claude/DYN-START-SPEED/TAG and counts), `prof.sh TAG` (perf by symbol).
The old `/mnt/zram/claude/coord-parity` programs and `dl-23-2/execcount` were gone, hence the new ones.

| build | empty main | 3 libs | clock_gettime | faults (pie0/pie3/time) |
|---|---|---|---|---|
| glibc 2.39 (host) | 104,827 | 140,077 | 106,937 | 51 / 67 / 52 |
| base (c91e5e3bb) | 1,579,278 | 1,834,900 | 1,613,340 | 69 / 91 / 73 |
| step 2: RELACOUNT prefix loop | 1,387,378 | 1,643,072 | 1,421,510 | 69 / 91 / 73 |
| step 1a: verdef record scan | 1,116,069 | 1,371,763 | 1,149,840 | 69 / 91 / 73 |
| step 1b: version-name memo | 785,631 | 1,042,024 | 819,571 | 69 / 91 / 73 |

In-test (hot_text_products, fast, Skylake-SP): 854,405 (was 1,718,270). Total: -50.3% release, faults unchanged.

## Decisions (local, recorded here; no D-row reserved)
- Step 1 is not a per-object index table: `Object` is pinned at 0x128 bytes (the start keeps 16 views on its stack; a
  field cost a stack page before). Instead (a) `Versions::def_name` scans entries as whole 20-byte records with the
  byte order fixed per call, falling back to the field walk on anything irregular (same answers and errors), and (b)
  a pass-local `VersionMemo` (16 direct-mapped slots, 384 B on pass1's stack, keyed by string-table address + index
  + reference/definition, value = offset/length in that string table: no lifetime) carried on `Request::with_memo`.
  Only relocation passes use it; dlsym/dlvsym/canonical slots are unchanged.
- Step 2: `Relas::relative_prefix` runs the DT_RELACOUNT prefix with the byte order chosen once; each entry is still
  classified (a non-RELATIVE entry ends the run, so a wrong count changes nothing); `Loads::hit_word` tries the last
  segment first. B + A computed directly as for RELR.

## Not done (budget: 150 calls)
3. RELR: not measured. ld.so is already linked with `-z pack-relative-relocs` (xtask/src/so.rs ld link line); libc.so.6
   (so.rs ~1382) is not. Next: add the flag on a branch, check `readelf -V` for any new version need, measure.
4. Faults 69 vs 51: not worked. DYN-START-PARITY-2's finding stands (kernel places the whole-file mapping; aligning the
   reservation ourselves would make the count independent of the file system).
5. Zen 1 fast bound: not measured; the Zen 1 release bound was set equal to the Skylake-SP one (inferred).

## Profile left (step 1a head, by symbol; the memo since removed most of version_name's 16.8%)
pass1 14.5% (symbolic entries' general loop), Object::new 9.6% (symbol count walks every GNU bucket; versym_with walks
every verdef/verneed name), lookup + its closure 12% (`Dynamic::string` scans each candidate name for its NUL
byte by byte), read_phdr 6.5% (program headers decoded again by find/Loads::of/image_span_kind/text_segments),
target 6%, one 5.6%, Loads::inside 3.3%, fill_sse2 3.2% (zeroing start tables), Dynamic::once 3.2%,
image_span_kind 3.2%, memmove 3.2%.
Next steps in order: (1) re-profile the final head; (2) candidate-name compare without a NUL scan (compare
req.name then check the NUL at its end); (3) Object::new: count symbols from the GNU table's last bucket only, skip the
name walk in versym_with for objects already validated; (4) decode program headers once per object; (5) RELR, faults,
Zen 1 as above.
