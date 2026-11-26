# HEADERS-FRAGMENTS: handoff (end of leg 2 of the run, REFRESH)

Branch task/headers-fragments, base origin/next cd31515dc (unchanged at 09:30Z). Role shiki:~/w/sh1. Brief:
~/state/briefs/HEADERS-FRAGMENTS.md. Scratch: ~/w/.scratch/HEADERS-FRAGMENTS/. Host scratch: ~/work/headers-fragments/.
Commits (all mine, pushed): 924b20dd9 probe/check fragments; f962c734a (WIP subject) template fragments; 358f64541
table keys; 4e37752d7 duplicate-definition check (+ plants file).

## Done and proven
1. Probe + check fragments (commit 924b20dd9). PROOF (job headers-fragments-checklist, shiki, rc 0): header_check
   `--list` on origin/next's spec/tools vs the branch: LISTS IDENTICAL (23264 units), FAIL LINES IDENTICAL
   (0 failed of 23694 both). Commit f962c734a touches neither spec/tools nor libc-include, so it still holds.
   probe candidates() identical (leg 1, local diff, 20048 lines).
2. Template fragments (f962c734a; the commit subject says WIP, the code is final and proven; no history rewrite):
   `cargo xtask headers-gen --check` on shiki: "headers-gen: ok, 242 headers" (libc-include byte-identical; one
   output tree, configurations are #if text inside it, so every configuration is identical).
3. Layout tests: appendix_c_layouts.rs is @generated from spec/abi/layouts.tsv (rows already); no split. Brief
   premise challenge (cheap): say so in the hand-back.
4. table_keys (358f64541 + 4e37752d7): spec/tools/table_keys.tsv (62 tables: every *.tsv under headers/ and
   spec/abi/, format documented at its top: path, key, sort key|-, unit row|block; layouts*.tsv are `block`: a
   flattened tree whose `<anon>` rows repeat, e.g. struct udphdr has two identical rows). xtask/src/table_keys.rs,
   static check `table-keys` (also `cargo xtask table-keys`), registered in main.rs static_list (45) and
   affected.rs XTASK_TOOLS. All tables on next pass. Job headers-fragments-lint1 (shiki, rc 0): clippy, 153 xtask
   tests, static.
5. Duplicate check (4e37752d7): spec/tools/header_checks/dup.py (problems(ctx) only, so no unit list change;
   standalone `python3 spec/tools/header_checks/dup.py [--rows]`, ~5 s). On next: 3808 macro names defined in 2+
   headers, ALL identical generator emissions (2953 = AT_* of elf.h + sys/auxv.h; 392 signal.h+unistd.h; 154
   inttypes.h+netinet/icmp6.h+ip6.h; utmp.h+utmpx.h ACCOUNTING etc.; __FD_SETSIZE in 41 headers). 0 typedef/tag/
   enumerator duplicates (scanner sees 327 typedefs, 286 tag bodies, 1432 enumerators in gnu). struct utmp compat
   names (ut_name/ut_time/ut_xtime/ut_addr) are defined ONCE on next now (fixed before this story; nothing to fix).
   Listed in headers/dup-listed.tsv (kind,name,files; ratchet: new or changed fails, stale row fails).
   EXCEPTIONS (stock does the same) is empty: no stock probe was run; the list is "the rest" the brief asks for.
   Standalone run on the branch: "dup: 0 failed".

## Jobs running (started 10:2xZ on 4e37752d7, synced tree 9adebbd7)
* shiki headers-fragments-test1 (kind test): headers-gen --check, clippy -D warnings, xtask static (passed in the
  log already), cargo test -p xtask --bin xtask (includes network_headers_pass_header_check = header_check.py with
  dup.py). `cargo xtask affected --list` says FULL (template fragments = generator inputs), so this + precheck is the
  gate: hand-back line `FULL: the train's (headers/*.h.d fragments: generator input)`.
* shiki headers-fragments-plants1 (plants file handoff/HEADERS-FRAGMENTS-plants.txt, 14 plants, dir
  /mnt/zram/claude/headers-fragments/plants-1). Baselines were ok.

## Exact next steps
a. `~/bin/jobwait headers-fragments-test1 headers-fragments-plants1` on shiki (one call, <=600 s per call); read
   ~/logs/<label>.log tails (plants: verdict lines + WARN lines; list every WARN in the hand-back). Fix any SURVIVED
   (msg regexes are guesses for dup-tag/dup-enumerator: headers-gen may reject the inserted text).
b. Merge proof (local only, never push): from HEAD make throwaway branches hf-a and hf-b; hf-a adds
   headers/math.h.d/185-plant-a.frag + spec/tools/header_probe.d/zz-a.py + spec/tools/header_checks/s12.d/80-a.py +
   a row in headers/reserved.tsv at a different place; hf-b the same with b; `git merge` hf-b into hf-a: no conflict
   expected for new files; the reserved.tsv rows conflict only if adjacent (say which). Delete the throwaway branches
   (`git branch -D`, local only). Describe in the handoff.
c. Precheck on the synced final head: `cargo xtask precheck --jobs headers-fragments-test1,headers-fragments-plants1
   --head <sha>` on shiki (exit 1 = fix).
d. Final handoff (this file, one page, + section "How a header story adds a family now" below), report json
   handoff/reports/HEADERS-FRAGMENTS.json (+ "jobs"), push, `ops handback-check HEADERS-FRAGMENTS --head <sha>
   --jobs "..."`.
e. Hand-back: RULING NEEDED: the 3808 cross-header identical macro definitions (recommend: generator emits a name
   once, in its provider header, and other headers include that one or guard with #ifndef; a separate story, the
   list shrinks as it lands). INFORMATIONAL: layouts premise (generated already); EXCEPTIONS empty; the
   local ssh of the job start hung once because a `mkdir &&` was chained before setsid (brief warns; jobs fine).

## How a header story adds a family now (for the briefs)
* Probe names: new file spec/tools/header_probe.d/<family>.py with `candidates()`.
* Checks: new file spec/tools/header_checks/<slice>.d/<NN>-<family>.py with `units(ctx)` (NN unused in that slice).
* Template text: new file headers/<h>.h.d/<NNN>-<family>.frag (its own `@@owns`/`@@if` lines); the template has
  one `@@fragments` line. Then `cargo xtask headers-gen`.
* Table rows: rows in the tables of spec/tools/table_keys.tsv; a repeated key fails `cargo xtask static`.
* Layouts: rows in spec/abi/layouts.tsv (block per type), then `cargo xtask abi-gen`/`headers-gen`.
* Never add a row to headers/dup-listed.tsv: a name your header defines that another header already defines fails
  dup.py (header_check.py); include or guard instead.
