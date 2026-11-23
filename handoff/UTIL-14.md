# UTIL-14: gencat in libc-dev-bin (GitHub issue #12)

**State.** Branch `task/util-14` from origin/next `b04f71229`, HEAD `d43bf38a0` (+ this note). Role keke:~/w/n2. Leg 2: built,
38 transcripts recorded and ALL EQUAL to the reference (byte comparison of every catalog), catgets test written. Leg 3
starts at "Next step (leg 3)".

## On the branch (leg 1)

* `csys/util/gencat.c`: the whole program (C front end, dynamic over libc.so.6, the zic precedent: a writer in C in
  `csys/util`). Options, parsing, merge, plane search, writer, -H header, all diagnostics.
* `xtask/src/so.rs`: `GENCAT_TEST`, built in `run_iconv` after zdump (`target/niji/gencat`, `target/niji-test/gencat`).
* `xtask/src/transcript/mod.rs`: `niji_default` "gencat" -> the test link.
* `tools/deb/manifest.tsv`: `system libc-dev-bin usr/bin/gencat file gencat 0755 debug`.

## Probe-settled (black box, reference gencat on keke, Ubuntu 2.39-0ubuntu8.9, LC_ALL=C.UTF-8 unless said)

Probe scripts: local `~/w/.scratch/UTIL-14/` (`pd_probe.py`, `syn_probe.py`, results `seq/sets/rand.txt`, `syn.txt`,
`hdr.txt`; fits `fit5.py`), copies on keke `~/work/util-14/`. Do not re-probe these:
* **Plane size/depth:** n = message count; P runs from n/5+1 (integer division) upward while P <= best P*D;
  D(P) = max bucket count of ((set+1)*msg) mod P (at least 1); take P*D <= best (tie -> larger P). 490 of 490 cases fit
  (seq 1..60, 30 sets, 400 random). Empty source: P1 D1, 36 bytes.
* **Pool order:** sets in the REVERSE of the order they were first named (h12: `$set 4,2,9` -> pool 9,2,4), messages
  ascending by number; no string sharing. Slots filled in pool order, first free plane. **Spec D.2.2 says "sets in
  descending order": wrong in general (only true for ascending sources): leg 2 corrects the spec text (UTIL-SPEC-3
  is pending on it; RULING/spec-errata per §10 if the spec file is not ours to edit).**
* **Merge (no --new, output not `-`):** the sources are parsed first, then the existing file's messages are added in
  table-slot order for (set,msg) not already defined (new sets appended to the named order); a source message
  silently replaces an existing one; a non-catalog file is ignored silently.
* **Lines:** `$ ` / `$\t` comment; bare `$` -> "unknown directive `': line ignored"; `$foo` same with foo; `$set N`
  (digits prefix, `9x` -> 9; numbers mod 2^32; `$set 0` ignored silently; re-naming by number clears a set's symbol);
  `$set Name` -> number max+1 over all sets; repeated name -> "duplicate set definition" + "this is the first
  definition" (continues in that set); `$set` alone -> "illegal set number"; `$delset` ignored; `$quote c` (non-ASCII
  -> "invalid quote character", quoting off; `$quote` alone off). Message: digits -> number (0 dropped silently),
  identifier -> symbolic number max+1 in the set, else "malformed line ignored" (`-1 x`). One blank after the token,
  more kept. Duplicate number -> "duplicated message number" + first definition (first kept); duplicate identifier
  -> "duplicated message identifier". Escapes \n\t\v\b\r\f\\, unknown -> char, octal greedy while value <= 0377,
  value > 0177 -> "invalid escape sequence" and the escape dropped (`ab\200cd` -> `abcd`); trailing `\` joins lines;
  bad multibyte in the locale (C: any byte >= 0x80) -> "invalid character: message ignored" (message dropped).
  Quoted text: closing quote ends it (rest ignored); unterminated -> "unterminated message", text kept. CR kept.
* Diagnostics `file:line: text` on stderr, exit 1 but the catalog is still written; missing input: ``cannot open
  input file `x': strerror``, exit 1, NO catalog written; output: ``cannot open output file `x': strerror``; write
  error `write of 12 bytes failed after 0: <strerror>: <strerror>` (the 12-byte head is written first).
* **-H header:** sets in reverse named order; a named set prints `\n` first unless it is the first in that list, then
  `#define <Name>Set 0x<n>\t/* file:line */`; each symbolic message `#define <Name|AutomaticSet<set+1>><Id> 0x<m>\t/*
  file:line */`; then one final `\n` (a source with no symbols gives a header of just "\n").
* Options: argp texts as in the code (`--help` has a line of 28 spaces after `--new`), exit 64 for option errors
  with the `Try` line; `--h`/`--he` ambiguous (header, help); no operands -> stdin to stdout; `-o` absent -> first
  operand is the output; `-` is stdin/stdout. Not probed: the file name in diagnostics for stdin (a transcript will
  show it), `$set` with only an empty set, the header to `-`.

## Leg 2 (done; do not redo)

* Fixed from transcripts (probes `~/w/.scratch/UTIL-14/setorder.py, edge.py, edge2.py`, results `setorder.txt`, `edge.txt`):
  the default set 1 always counts as named FIRST (so last in pool and header), whenever first used; messages sort
  SIGNED int32; symbolic `$set Name` = 1 + signed max (from 0) over sets named by `$set` lines only (the implicit
  default set does not count; Name may take number 1 = the default set, which then gets the name and its line);
  symbolic message = 1 + signed max (from 0) in the set; a numeric message hitting a symbolic message's number moves the
  symbolic one to the set's signed max + 1 (no diagnostic); the -H header has NO final newline (leg 1's "final \n"
  was a display artefact). The pool-order bullet above is superseded by these.
* `validation/transcripts/gencat/` (38, generator `~/w/.scratch/UTIL-14/gen_transcripts.py <dir>`, then record on
  keke and rsync back; help/version use `normalise:` for the identity lines); `xtask/tests/transcripts.rs`
  `the_committed_gencat_transcripts_hold` (green, job util-14-tr4); `crates/core/tests/gencat_products.rs` +
  `oracle/gencat_lookup.c` (catgets on libniji and host libc over both catalogs + byte equality; tr4 failed on MY
  expected-text typo "continued" -> fixed to "contnued" in d43bf38a0, re-run as job util-14-t5).
* D-31.28 record written by hand (no local cargo): `records/decisions/D-31/D-31.28.toml` -- run `cargo xtask records
  check` on keke. Docs: tools/deb/system.sh, docs/install-packages.md, RELEASE-NOTES.
* Spec D.2.2 NOT edited: CONTRIBUTING's three-way rule (spec vs probe, docs silent -> change nothing and ask): hand back
  RULING NEEDED: correct D.2.2's gencat-output bullet to the rules above (recommend yes).
* Mistake in leg 2: one Bash call used a here-string (`git commit -F /dev/stdin <<< ""`); it committed nothing. Report it.

## Next step (leg 3)

1. `~/bin/jobwait util-14-t5 util-14-plants` on keke (plants started 04:20Z, 7 plants x ~11 min `cargo xtask so`
   each; file handoff/UTIL-14-plants.txt; dir /dev/shm/claude/util-14/plants-1). t5 must pass; every plant CAUGHT
   (plane-start may be EQUIVALENT if no transcript changes P: then add a source that does, or declare with reason).
2. On the role: `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo xtask static`,
   `cargo xtask records check`; then the gate: `cargo xtask affected --run` as util-14-gate (jobrun gate, ~/bin/gate 16G).
   Check `cargo xtask affected --list` selects gencat_products and the transcripts test for csys/util/gencat.c.
3. Both package flavours build + check-system lists /usr/bin/gencat (brief Proof); precheck --jobs <labels> --head <sha>.
4. Report: handoff/UTIL-14-report.md + handoff/reports/UTIL-14.json (+ "jobs"), handback-check, hand back.
