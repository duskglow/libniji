# UTIL-14: gencat in libc-dev-bin (GitHub issue #12)

**State.** Branch `task/util-14` from origin/next `b04f71229`. Role keke:~/w/n2. Leg 1 wrote the program and the
wiring; nothing is built or tested yet. Leg 2 starts at "Next step".

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

## Next step (leg 2)

1. Sync (`sync_role.sh --story UTIL-14 --base origin/next ~/w/UTIL-14 keke n2`), then on the role `cargo xtask so`
   (jobrun kind build) and fix compile errors (-Wall -Wextra -Werror).
2. Transcripts `validation/transcripts/gencat/*.transcript` (record on keke with `cargo xtask transcript record`,
   `--niji gencat=target/niji-test/gencat` if needed): help, usage, version (normalise identity), bad option,
   ambiguous, compile (written-file sha256 = byte comparison), symbolic+header, merge, escapes/quote, errors,
   C-locale invalid char, nofile, empty, a large catalog (>= 200 messages, several sets, reverse-named order).
   Clauses s31_12_*, s31_1_c_2, s31_1_c_r3.
3. `catgets` equality: extend `crates/core/tests/catalogs_products.rs` or a new test running both gencats on one source
   and catgets over every (set,msg) on libniji (see how catalogs_products builds the fixture).
4. Spec D.2.2 pool-order wording; `tools/deb/system.sh` line 39 and `docs/install-packages.md` line 150 still say
   libc-dev-bin is a placeholder; RELEASE-NOTES; record D-NEW (the C front end choice) via ops reserve.
5. Plants (docs/plants.md format, `handoff/UTIL-14-plants.txt`): plane start n/5 -> n/4, pool order, octal limit,
   merge skip, header blank rule. Then clippy, static, gate (`cargo xtask affected --run`), precheck, report json.
