# AUDIT-HARDCODED-LIMITS (planning, no product change)

**State.** Branch `task/audit-hardcoded-limits`, from `next` 575214f4b. The deliverable is
`plan/hardcoded-limits.md`. Probes, measurements and the rerunnable search are in
`results/hardcoded-limits/` (`search.sh`, `probes.c`, `run.sh`, `objects.sh`, `probes-keke.log`,
`objects-keke.log`). The report json is `handoff/reports/AUDIT-HARDCODED-LIMITS.json`.

**What it found.** 20 remove candidates (resource caps, HL-1..HL-20), 13 keep candidates (security
bounds HL-21..26; correctness and representation guards HL-27..33), an ABI/spec-fixed list, a list of
caches checked and found not to be limits, a stewardship section and a ruling table. Probes on keke:
the reference passed every count tried. libniji stops at 509 dlopen'd objects ("cannot load more
objects"), at 64 new objects in one `dlopen` call, at 64 objects at start ("too many objects"), at 256
named semaphores (`EMFILE`), at 256 `addseverity` levels, at 78 directory-form locales (`newlocale`
`ENOENT`), at 2^20−1 exit handlers, and at `fnmatch`/`wordexp` nesting of 65/129. On keke, 7 of 1504
installed programs and objects need more than 64 objects at start, and `libgtk-3` alone needs 67.

**Decisions in force.** Ruling #5594 (regcomp: no artificial caps, process limits bound memory);
owner 2026-11-17 (no artificial limits, but use memory responsibly). Rows mentioned: D-17.8 (getpass
1 MiB), D-22.5 (4096 keys), D-22.9 (stack cache), D-27.16 (`--list`), D-2.6 (addseverity), D-X.13
(locale invalidation).

**Open questions (for the owner, in the ruling table):** every HL row. HL-19 (getpass) needs an
explicit choice between ruling #5594 and D-17.8.

**Do not redo.** The five search routes (rerun `search.sh`), the probes (rerun `run.sh <dir>
<niji products>` on a host with a compiler; link libniji programs as `run.sh` does, and set
`LD_LIBRARY_PATH` only for the probe process, through `env`, never for `timeout`).

**Not verified:** listed in section 5 of the plan (aio ring sizes, rpc text buffer, the `--list` cap
under libniji's own loader, `getaddrinfo_a` past 4096 pending). The products probed were a `next`-era
build on keke (`~/w/n3`, 2026-11-17 07:31), not built from this branch's head (which changes no
product code).

**Next step.** The coordinator turns the owner's decisions into stories. Suggested order by
likelihood: HL-1/2 (in progress), HL-4, HL-10, HL-7, then HL-3/5, HL-9, the rest.
