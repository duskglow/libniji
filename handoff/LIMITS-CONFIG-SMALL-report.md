# LIMITS-CONFIG-SMALL report

Premise: plan/hardcoded-limits.md rows HL-16..HL-19 and the owner's ruling #5594 (no artificial caps;
growth with use; a clean error when memory runs out). Nothing that landed since the brief changes it.

## Commits (task/limits-config-small)
1. Remove the error_at_line name and getpass input caps (HL-18, HL-19): code, tests, D-17.10,
   docs/interfaces.md, spec/abi/native.tsv, differences row removed, plants file.
2. Regenerate native.h; make the name split a function.
3. Handoff note, this report, reports JSON.

## Results (keke, role r2)
- clippy (workspace, all targets, all features, -D warnings) and `cargo xtask static`: clean
  (limits-config-small-lint3).
- Plants (limits-config-small-plants): 4 plants, 4 CAUGHT, 0 WARN. name-cap-restored and
  getpass-cap-restored restore each old cap alone, so they stand in for next's behaviour: the new
  tests fail on next and pass on this head (the plants baseline).
- Reference probe (limits-config-small-refprobe): conformance/c/s02_report.c built against the host C
  library passes every clause, including the 8 KiB-name case of s2_3_c_3. getpass's reference: the
  removed differences row recorded that the reference returned a 3 MiB line whole.
- Gate: direct tier (limits-config-small-gate); see the hand-back for its result.
- Start-up counters: not run. No start-up path changed: `report.rs` repeats() runs only in
  error_at_line with error_one_per_line set, and getpass is not on a start-up path. The 4096-byte
  table is kept as it was, so .bss does not change; the new `Chunks` static adds 384 bytes of .bss
  (48 words).

## Probe-settled behaviour
- Reference error_one_per_line: an 8 KiB name repeated at another address is suppressed, and one
  that differs only in its last byte is printed.
- Linux canonical-mode terminals pass at most 4095 bytes of a line, so getpass's 4 MiB test goes
  through standard input (the brief said a pty; the kernel makes that impossible).

## RULING NEEDED
- D-17.10 (public native interface): keep `niji_getpass_set_limit` as an opt-in bound, with 0
  (NIJI_GETPASS_DEFAULT_LIMIT) meaning no bound and being the default. Recommend: keep, as
  implemented. D-17.9 delegated the choice to this story, and NIJI_NATIVE_1 is still open.

## INFORMATIONAL
- HL-16/HL-17 are not done: the challenge (#7807) was accepted and a follow-up is owed. The design is
  in handoff/LIMITS-CONFIG-SMALL.md.
- Extra cap seen: core/src/names/nss.rs rules() `[0u8; 256]` for one `[...]` action group.
- getpass, stream path: a line that could not be held (ENOMEM) used to return at once and leave the
  rest of the line unread on standard input. It is now drained and zeroed, like the EOVERFLOW path
  (plant getpass-nomem-undrained).
