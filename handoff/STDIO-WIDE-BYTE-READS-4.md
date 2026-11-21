# STDIO-WIDE-BYTE-READS-4 (GitHub #23, Yohane's differential probe): cut short at end of input

State: branch `task/stdio-wide-byte-reads-4`, base origin/next 7ac731794, code head f9b9a1f69 (this page and
the report after it). Role keke:~/w/r2 (Zen 1 host). Continues `handoff/STDIO-WIDE-BYTE-READS-3.md` (D-12.28).
Probe sources (black box, not committed): the run's scratch and keke `~/work/STDIO-WIDE-BYTE-READS-4/`
(`wbprobe.c`: 13 inputs x 3 patterns `wbwbwb`/`wbbbbb`/`wwwwww`, plus ad-hoc `hex pattern...` mode with `t` = ftell,
`e` = feof; `cases.sh`: boundary cases; `wbccs.c`: `ccs=UTF-8`).

## What the probes settled (reference 2.39 on the Zen 1 host, C.UTF-8)

* Yohane's 2.41 column is reproduced by 2.39 on every row of their table (`c3`, `e3 81` alone: `w:-1(EILSEQ)`, then
  byte reads EOF; `c3 a9 78`, `e3 81 82 6f 6b`, `c3 28 7a`: byte reads after a wide read return EOF; `f4 90 80 80`:
  `w:1114112`). Issue reproducer `61 fc 63 64` `wbwbwb`: `w:97 b:252 w:99` on both.
* Kind 1 is NOT one rule on the reference. EILSEQ + bytes consumed + end-of-file indicator clear (ftell counts the
  bytes) only when the cut-short character is the first input the read fetched: `c3`, `e381`, `a`x4096 then `e381`.
  After other buffered input the bytes come back with `WEOF` + feof, exactly D-12.28: `61c3`, `6162e381`,
  `656e64e282` (STDIO-WIDE-BYTE-READS-3's own probe case), `ffc3`, `8071c3`, `c3a9c3`. Same on `ccs=UTF-8`.
  At a 4096-byte buffer boundary it follows the reference's convert-ahead: `a`x4095 `e3|81` drops, `a`x4096 `|c3` keeps.
* 2.41 (stock trixie root, job trixie3, the same 2.39-built binary run under the root's own C library): the
  39-row table and the 23 boundary rows are byte-identical to 2.39 (`diff` empty). So the reference version does
  not matter here; D-12.28's case was simply a different case (other input before the cut-short character).

## Probe table, head f9b9a1f69 vs the reference (keke `~/work/STDIO-WIDE-BYTE-READS-4/head.txt` vs `ref239.txt`)

Base = snapshot 21.1 = Yohane's 21.1 column (no kind-1 code change since). Head: every kind-1 row now matches
(`c3`, `e381` x 3 patterns: `w:-1(EILSEQ)` then EOF; `ccs=UTF-8` too: `ccs-head.txt` = `ccs239.txt`). Rows still
different in Yohane's format (result + EILSEQ): kind 2 only (a byte read after a successful wide read returns the
next byte: `61fc6364`/`616263`/`c3a978`/`e381826f6b`/`c3287a`/`e282ac7478`/`f09f988021`, `wbwbwb` and `wbbbbb`)
and kind 3 (`f490808073`, all three patterns: EILSEQ, bytes back). The issue's reproducer `61fc6364` `wbwbwb` starts
`w:97 b:252 w:99` on both. Boundary rows: the kind-2 ones and positions (`ftell`) after kind-2 reads differ, plus
the two buffer-boundary kind-1 rows (`a`x4095 `e3|81`, `a`x4096 `|c3`), as the differences row says.

## Change (f9b9a1f69): D-12.29 (reserved id; amends D-12.28)

* `getwc_inner` (crates/legacy/src/stdio_compat.rs): on a multibyte lead byte (`Decoded::Incomplete` with
  `partial == 0`; for `Codec::Charset` the first pending byte) it notes `Wide::fresh = rpos == RESERVE + 1` (the byte
  was the first of the buffer's latest fill). ASCII and byte paths untouched.
* Cold `end_of_input`: `fresh` -> consume the bytes, clear eof, set the error indicator, `EILSEQ`; otherwise D-12.28.
* Differences row "Byte operations on a wide-oriented stream" states libniji's rule and the buffer-boundary cases;
  row "UTF-8 above U+10FFFF" now covers the wide stream reads (kind 3 kept, RFC 3629).
* Conformance `s12_wide_byte_reads.c` checks 38-40 (`e3 81` alone, both encodings).

## Kind 2 (kept; reason)

The row's reason still holds. Matching means a byte read after a wide read returns `EOF` with neither indicator
whenever the reference's wide layer has converted ahead, and which bytes that covers is set by its buffer sizes
and conversion chunks: the boundary probes show `b:227 w:97 w:-1(EILSEQ) b:129` for `a`x4095 `e3|81`, i.e. byte and
wide reads interleaving by its internal chunking. libniji would have to rebuild that wide buffer and its
capacity to reproduce it, and an `EOF` with no indicator set is not a correct end of input for any program that
loops on `getc`. Yohane's reproducer (`97 252 99`) and the col pass-through cases (checks 26-27) already match
stock without it, and #23 is held open only for kind 1. The owed compat-flag registration for the row stays as it was
(no option: a buffer-size-dependent result).

## Jobs (keke, all on code head f9b9a1f69)

* check1 (rc 2, SUPERSEDED for the probe part only): records check, clippy `--all-features -D warnings`, static and
  `cargo xtask so` passed; the job then stopped on my script's loader lookup (`ls` of a missing name under pipefail).
  The probe was run by hand afterwards (`probehead.sh`, results above); the conformance program runs in gate1.
* plants1 (rc 0): `handoff/STDIO-WIDE-BYTE-READS-4-plants.txt`: 5 plants, 5 CAUGHT, 0 SURVIVED, 0 WARN-RC-ONLY,
  0 WARN-NO-MSG (fresh-never, fresh-always, charset-fresh-never, drop-sets-eof, drop-no-error).
* gate1 (rc 0): direct tier, `affected --run --base origin/next`: 12 suites ok, 6741 passed / 0 failed (550 ignored),
  conformance 15 / 0, SKIPPED lines 0, peak 11G, oom_kill 0.
* post1: waits for gate1, then `wl_percall.sh --spec ~/work/STDIO-WIDE-BYTE-READS-4/stdio_wide.c .` (byte reads
  fgetc/getc/getc_unlocked; wide reads fgetwc/getwc/fgetwc_unlocked on NULs and on U+00E9; base = synced merge
  base 7ac731794) and the hot text check (`NIJI_FULL_PROFILE=1` so + `hot_text_products`).
  post1 rc 0; hot text: `NIJI_FULL_PROFILE=1` so ok, `hot_text_products` 32 passed / 0 failed.
  percall (instr/call, glibc / base / head): fgetwc_unlocked_2byte 58.2 / 636 / 639; getwc_2byte and fgetwc_2byte
  90.2 / 646 / 649; fgetwc_unlocked_nul 35.6 / 356 / 356; getwc_nul, fgetwc_nul 67.7 / 366 / 366; getc_unlocked
  13 / 13 / 13; getc, fgetc 31 / 24 / 24. COST: +3 on the 2-byte wide rows (the `fresh` note on a multibyte lead
  byte), reported for a ruling (recommend accept; fix-forward: derive the flag in the cold `end_of_input` path from
  the buffer position and pending length so the hot path is byte-for-byte the base). Not reverted: the fix is the
  story's correctness change and the rework needs a new gate and plants cycle, which the run's deadline does not allow.

## EXACT NEXT STEP (done in refresh leg 2: jobs collected, report json written; below kept for the record)

1. `~/bin/jobwait stdio-wide-byte-reads-4-plants1 stdio-wide-byte-reads-4-gate1 stdio-wide-byte-reads-4-post1` on keke
   (ssh via vm_ssh.sh). Plants: all CAUGHT (list any WARN). Gate: passed/failed counts + peak. post1: percall rows
   base -> head (a row slower = revert or explain; the only hot change is one compare per multibyte lead byte).
2. Write `handoff/reports/STDIO-WIDE-BYTE-READS-4.json` (thread_common shape + `"jobs"`), fill the numbers here,
   commit, push, sync (`sync_role.sh --story STDIO-WIDE-BYTE-READS-4 --base origin/next <wt> keke r2`), then on the
   role `cargo xtask precheck --jobs <labels> --head <sha>` and `ops handback-check`.
3. Hand back COMPLETE with JOBS: keke:stdio-wide-byte-reads-4-gate1, -plants1, -post1, -trixie3; SUPERSEDED: check1
   (script error after clippy/static/so passed), trixie (keyring), trixie2 (keyring). Kind 2 kept: paragraph above.
   Report the premise correction (kind 1 is not unconditional on the reference) as INFORMATIONAL.

## Open / next

* Next step: land; Yohane's probe should then differ only in kind 2 (byte reads after a successful wide read)
  and kind 3 rows.
* Not run: `cargo xtask test` (FULL tier, the train's per the owner's 11-19 rule).
