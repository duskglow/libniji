# STARTUP-RELAY-EXEC-SHIKI: the start-up tests' page_faults relay "cannot exec" on shiki

State: branch `task/startup-relay-exec-shiki`, cut from origin/next 997a3a8c3; HEAD = the commit that adds this note.
Kind: code (test harness only: `crates/core/tests/oracle/page_faults.c`, `crates/core/tests/hot_text_products.rs`).
Decisions in force: none new (harness change; no D-row). Code comments say `STARTUP-RELAY-EXEC` (the story id holds a
host name, which `cargo xtask static` lab-names refuses outside handoff/).

## Verdict

The 9-of-10 failure (PRINTF-NO-HEAP-4, `printf-no-heap-4-hotrep-1`, ruling #8228) was the rotating call-directory race
that RELEASE-STEP-EXEC-FAIL-2 removed (`call_dir`: `/tmp/nht-<pid>-<call % 10>` -> one directory per call). That fix
(merge ad8438b75) landed on next after 81bdd4b45, the base the 9-of-10 repeat ran on. Not a host configuration: shiki
needs no change.

Evidence (all on shiki, Haswell 26 vCPU, NIJI_FULL_PROFILE=1 `cargo xtask so` + `cargo test -p niji --test
hot_text_products`, the CI release pair):

* Before (diagnostic, described not named: a RAM copy of this branch with `% 10` put back into `call_dir`, i.e. the call
  directories of 81bdd4b45, 5 runs): 5 of 5 red; besides the planted `harness_fault_call_dirs_are_never_shared`, the race
  hit 4 of 5 runs, in the same three tests as #8228 (init_dynamic_startup_fault_count_does_not_depend_on_the_file_system
  3, init_dynamic_startup_only_program_stays_within_its_page_fault_bound 2, init_data_clusters_hold_at_every_text_phase
  1). The failures, now named:
  * `exit status: 127; stderr: "sh: 1: exec: page_faults: not found"` (4): the call's directory was removed by a call ten
    slots later before `sh` exec'd the probe;
  * `exit status: 127; stderr: "page_faults relay: execve /tmp/n<pid>-<n>/page_faults failed: errno 2"; errno 2: No such
    file or directory` (1): removed between `sh`'s exec and the relay child's exec (the anonymous 127 of #8228);
  * `exit status: 99; stderr: "mount: /usr/lib/locale: special device /tmp/n<pid>-<n>/locale does not exist."` (1): the
    bind mount's source removed (the 99 of #8228).
* Current next + this branch, `call_dir` as landed: `startup-relay-exec-shiki-rep-1` (first commit): the 35 existing
  tests green in 10 of 10 runs (only the new test failed, on the asm defect below); `startup-relay-exec-shiki-rep-2`
  (final code): **10 of 10 runs green, 36 passed, 0 failed each** (14.6 to 16.4 s per run).

Why shiki and not mei: the race needs one `faults_env` call to outlive ten later calls of the same test process (the
long dynamic-program calls against `init_data_clusters_hold_at_every_text_phase`'s run of short ones). RELEASE-STEP-
EXEC-FAIL measured at most 5 on the Skylake-SP host; on shiki it is routinely more. The overlap on shiki was not
measured call by call (NOT VERIFIED beyond the failure rate).

## Release step on mei (RELEASE-STEP-EXEC-FAIL, step 4)

The same mechanism gives exactly the release step's two exits (127 and 99), with no other fault needed, and it is now
shown to fire on a real host. It is the only mechanism known to give both. On mei it was measured at most 5 later calls
per call and not reproduced in 23 runs, so mei's 127/99 is consistent with this cause but still not proven. Either way
it is gone on next (ad8438b75), and any recurrence now names its step and errno.

## The change

1. `page_faults.c` relay: the vfork child stores execve's -errno in the parent's frame (vfork shares memory; the exec
   never happened, so no counted process ran) and exits 127; the parent prints `page_faults relay: execve <path>
   failed: errno <n>`. vfork and AT_EXECFN failures name themselves too. The counted (exec-succeeded) path is unchanged:
   all 35 existing tests, including every page-fault bound, green.
2. `run_failure` appends the OS error text to any `errno <n>`; the chain's first program (setpriv, or the probe without
   a namespace) names itself if it cannot be spawned. setpriv, unshare, sh and mount already print their own exec and
   mount failures on stderr (shown above), which `run_failure` carries.
3. New test `harness_relay_names_a_failed_exec_and_its_errno`: the relay exec'd as `/proc/self/fd/<n>` (close-on-exec),
   so the relay starts and its child's exec of AT_EXECFN fails with ENOENT. It caught a defect in my first version (an
   `"=a"` output without early clobber let the compiler keep the errno pointer in rax, which vfork overwrites: exit 6).

## Jobs (shiki, role sh2)

Code final at 2e1be74ee; d1ad284cf changes the plants file only, later commits handoff/ only.

* `startup-relay-exec-shiki-gate-1` (d1ad284cf): direct tier, `niji::hot_text_products` (1 of 322 targets), 36 passed,
  0 failed, 0 SKIPPED lines; peak 3.2G, rc 0.
* `startup-relay-exec-shiki-plants-2` (d1ad284cf): 4 of 4 CAUGHT, no WARN (`handoff/STARTUP-RELAY-EXEC-SHIKI-plants.txt`).
  The plant's message for a missing relay target: `page_faults_niji true failed: exit status: 127; stderr: "page_faults
  relay: execve /nonexistent/page_faults failed: errno 2"; errno 2: No such file or directory (os error 2)`.
* `startup-relay-exec-shiki-rep-2` (2e1be74ee, the final code): NIJI_FULL_PROFILE=1 `cargo xtask so` + 10 full
  hot_text_products runs, 10 of 10 green (the start-up check of the brief's rules).
* `startup-relay-exec-shiki-lint-2` (2e1be74ee): clippy (workspace, all targets, all features, -D warnings) and
  `cargo xtask static`, clean.
* Superseded: `-rep-1` (first commit; the new test's asm defect, 35 others green 10/10), `-lint-1` (static lab-names:
  the story id in code comments), `-plants-1` (relay-exec-missing WARN-NO-MSG: the plant replaced execve's argument,
  the report names AT_EXECFN; plant fixed, the code did not change), `-rot-1` (the diagnostic above).
* Not run: `cargo xtask test` (no start-up code changed: the change is test-harness only).

## Exact next step

Land (the train is the FULL gate). The start-up check on shiki needs nothing more; #8228's "do not admit start-up
stories to shiki" can be lifted once this lands (it was already true of next since ad8438b75).

Scratch: shiki ~/work/startup-relay-exec-shiki/ (rep.sh, rot.sh, per-run logs rep-1, rep-2, rot-rot-1).
