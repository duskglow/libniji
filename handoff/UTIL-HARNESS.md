# UTIL-HARNESS (UTIL-1): the utilities' transcript harness

**State.** Branch `task/util-harness` (from origin/next `2d5625a3e`). Code head `7c6b47e24`; this note and the report
json are the commit after it. Role mei:~/w/s3. COMPLETE.

## What is on the branch

* `xtask/src/transcript/`: `cargo xtask transcript (check | record) <file | dir>... [--reference C=PATH] [--liella C=PATH]`.
  * `format.rs`: the transcript format (docs/transcripts.md): directives, escapes, `|` blocks, recorded outcome last.
  * `run.rs`: one side in its own private root under `$TMPDIR`: setup items (`dir`, `file`, `link`, `cache` through
    `liella_ld::cache::write`), exact env, stdin, argv[0] = bare command name, cwd = root; `namespace: user|root` via
    `unshare --user --map-root-user --mount` and the hidden `transcript-exec` helper (bind mounts, then for `user` a nested
    user namespace mapping the caller's uid/gid back); tree snapshot before/after (sorted) gives `written`;
    normalisation = root path -> `@ROOT@` plus explicit `normalise:` literals only. Helper failures (exit 125 with its
    prefix) are harness errors, never compared.
  * `compare.rs`: status, stdout, stderr (separately; a note when the text is on the other stream) and each written path;
    LCS line diff with escapes and 2 lines of context; digest-vs-bytes comparison for recorded binary files.
  * `stub.rs`: Liella's `ldconfig -p [-C CACHE]` until UTIL-3 (argv[0] `ldconfig` dispatch in `main.rs`), over
    `liella_ld::cache::Cache`; the flags texts are exactly the words p-flags recorded; others are refused (exit 2).
* `validation/transcripts/ldconfig/`: `p-flags` (20 entries: every probed flags word, a hwcaps entry), `p-default-cache`
  (namespace user, bind over `/etc/ld.so.cache`, digit-run keys, i386 entry, hwcaps), `p-missing-cache` (exit 1,
  `normalise: /sbin/ldconfig.real => ldconfig`, reason in the file). All recorded from the reference on mei.
* `xtask/tests/transcripts.rs`: check of the committed transcripts; a one-byte-changed record must give MISMATCH.
* `docs/transcripts.md`; `records/pending/UTIL-HARNESS-1.toml` (D-NEW, series D-31: the location decision and the
  normalisation rule); `affected.rs` lists `transcript` in XTASK_TOOLS; plants `handoff/UTIL-HARNESS-plants.txt`.

## Probe-settled (reference, black box, recorded in p-flags.transcript)

* `-p` flags text: low byte 3 = `libc6`, any other low byte = `unknown or unsupported flag` (whole text); with libc6 the
  high part 0x100/0x400/0x500/0x700 `64bit`, 0x300 `x86-64`, 0x600 `N32`, 0x800 `x32`, 0x900 `hard-float`, 0xa00 `AArch64`,
  0xb00/0xf00 `soft-float`, unknown parts in decimal (`512`, `4864`). Hwcaps entries: `(libc6,x86-64, hwcap: "<name>")`
  (closes the UTIL-SPEC gap "`ldconfig -p` hwcaps label"). The reference's message name is `/sbin/ldconfig.real`.

## Results (mei:~/w/s3)

* `util-harness-dev1`, `-dev2` (development, superseded by the gate): 11 unit + 2 integration tests passed, xtask
  clippy clean, records stub generated.
* Final tree `f5e2f5ac4` (code `7c6b47e24` + record and handoff fixes): `util-harness-gate-3` rc 0: workspace clippy
  (all targets, all features), `xtask clippy`, `xtask static`, `records check`, `cargo test -p xtask` (405 unit, 3 + 2
  integration, transcripts included), `cargo xtask transcript check validation/transcripts` 3 ok, 0 mismatched, 0 skipped;
  peak 1.2G. `util-harness-plants-3` rc 0: 5 of 5 CAUGHT, 0 WARN.
* SUPERSEDED: `util-harness-gate` and `-gate-2` (rc 1: static `records`, the pending record's body layout, fixed);
  `util-harness-plants`, `-plants-2` (5 of 5 CAUGHT, older trees).

## Open

* INFORMATIONAL: `cargo xtask affected --list --base origin/next` escalates to FULL because `xtask/src/main.rs` changed
  (two lines of dispatch). The developer gate therefore ran the xtask suites, clippy and static directly; the FULL
  tier is the train's.
* Not covered yet (docs/transcripts.md): file modes/owners of written files; a hang limit; identity-string masking (Q3).

## Next step

UTIL-3 (ldconfig) replaces `stub.rs` with the real command (`liella_default` in `mod.rs`) and adds root/`-r`
transcripts (`namespace: root`, binds). Do not re-probe the -p flags texts. Read: plan/utilities.md §5-6, this note,
docs/transcripts.md.
