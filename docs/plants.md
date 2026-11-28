# Plants: `cargo xtask plants`

A plant is a deliberate mutation of the code (or test) a story adds. The story's tests must catch it. `cargo xtask
plants` runs a list of plants in the way the project rules ask for, so a story does not need a hand-written plants
script (XTASK-PLANTS, plan/next-batch-1024-2.md).

**The tool is the required way to run plants (owner, 2026-11-05).** Do not write your own plants script: hand-written
scripts printed CAUGHT next to passing tests, hung for 150 minutes and silently skipped plants. The project's
per-plant limit is `--timeout 600` (the default); a plant whose command runs into it is CAUGHT ("caught by timeout"). The product build (`cargo xtask so`, the baseline and one per plant that uses the products)
has its own limit, `--prepare-timeout <secs>` (default 3600), because a cold traced build can exceed 600 s. A baseline
build that runs into it ends `BASELINE FAILED: so timed out after N s (prepare timeout)`; a mutant's build that does
is reported `BUILD-TIMEOUT` (not CAUGHT: the mutant was not tested) and fails the run.

```
cargo xtask plants <plants-file> --dir <new-directory> [--timeout <secs>] [--prepare-timeout <secs>] [--copies <n>|auto] [--warm <dir>|--cold]   # test limit 600 (project rule); build limit 3600
```

**Copies (PLANTS-COPIES).** `--copies <n>` (1 to 8) runs the plants in `n` copies of the tree at the same time: plant
`i` of the file goes to copy `i % n`. Each copy does steps 2 to 4 below for its own plants (its own `cargo xtask so`
and the baselines of its own plants' commands), in `<dir>/src` and `<dir>/logs` (the first copy, the same paths as a
serial run) or `<dir>/src-<k>` and `<dir>/logs-<k>`. Lines from a copy after the first start `plants[copy <k>]:`. The
verdicts, the summary and `results.txt` come out in the plants file's order and read as a serial run's, each plant
judged as a serial run judges it; a copy that stops (refused, baseline failed) stops the run with the exit status a
serial run gives. The default, `auto`, prints its choice: two copies when the RAM this job may use holds two copies
at the measured per-copy peak (`COPY_PEAK_BYTES` in `plants.rs`, 14 GiB), the file has two plants or more, and the
job may use 8 CPUs or more (`SPLIT_CPUS`) or the file has more than 6 plants (`SPLIT_PLANTS`: two copies share a
small slot's CPUs, so the split pays only over enough plants); else one. "The RAM this job may use" is
the smallest of `NIJI_PLANTS_RAM_GB` (GiB: a role's RAM budget, set by whoever starts the job), the `memory.max` of
the job's cgroup and every cgroup above it, and `MemAvailable`; none known means one copy. Measured wall time and peak
RAM, serial against two copies: `handoff/PLANTS-COPIES.md`.

Run it from the role tree on the build host, under plain `jobrun` (not `~/bin/gate`, which gives the job a private
`/dev/shm`). Use a directory that is NEW for every run and that is in RAM: `/dev/shm/claude/<story>/plants-<n>` on Zen 1 host,
`/mnt/zram/claude/<story>/plants-<n>` on Skylake-SP host (the RAM-disk host). Do not delete it: ram-sweep removes it later.

**Warm copies (PLANTS-WARM-COPY).** A cold copy has no `target/`, so its baseline `cargo xtask so` builds everything
from nothing (median 7 to 15 minutes a job, about 45% of a plants job, handoff/PIPELINE-TEST-TIME-1127-report.md). So
copy `k` of a run uses the role's **warm copy** `<warm>/copy-<k>` instead, kept from run to run with its cargo cache:

- `<warm>` is `--warm <dir>`, else `plants-warm/<tree name>` beside the story directories (`--dir`
  `/dev/shm/claude/<story>/plants-<n>` in the role `~/w/<role>` gives `/dev/shm/claude/plants-warm/<role>`).
  `--cold` uses none (every copy cold, in `--dir`, as before).
- One run at a time: a run holds `<warm>/copy-<k>.lock` (an exclusive `flock`, released when the run ends however it
  ends). When another run holds it, the copy is cold, in `--dir`, and the run says so (`warm copy ... is busy`).
- Before use the warm copy is brought to the tree **by content**: its `target/` and `fuzz/target/` are pruned to
  cargo's build cache (the profile directories' `.fingerprint`, `build`, `deps`, `examples`, `incremental`, nested
  target directories such as `target/product` pruned alike, `CACHEDIR.TAG`; everything else goes, among it the
  products in `target/niji`, `target/tmp`, test caches, logs and the uplifted binaries, which cargo links again for
  every unit a build uses); then `rsync -rlpDc --delete` compares every file by content and keeps no times, so a file
  that differs (such as a mutant a killed run left) is written with a fresh mtime, newer than any build made from its
  old text, and a path the tree lacks is deleted. A second `rsync --dry-run` must then find nothing to change. Any
  failure here makes the copy cold instead. The run prints `warm copy <tree> -> <copy> (synced by content ...)`.
- So a warm copy holds what a cold copy holds plus what cargo itself judges fresh by its mtime checks; the verdicts
  are those of a cold run (measured: handoff/PLANTS-WARM-COPY.md).

## What it does

1. It refuses the run (exit 3, nothing created) when `--dir` exists, when the plants file is malformed, or when any
   plant does not change exactly one site of its file in the current tree. It warns (`WARN-NOT-RAM`) when `--dir` is
   not on tmpfs or zram.
2. It copies the tree to `<dir>/src` with `rsync -a --exclude /target --exclude /fuzz/target` (the source only), or
   syncs it into the role's warm copy (above).
3. In the copy it runs `cargo xtask so` (the products) when one of the copy's plants reads them (below), then each
   distinct plant command once on the unmutated copy. If one of them fails, it prints the last lines of its log and
   exits 2 before any plant is applied.
4. For each plant: it writes the mutated file (a fresh mtime, so cargo rebuilds), runs the command with `sh -c` in the
   copy (under `timeout`, default 600 s, the project's per-plant limit), then restores the file by writing it again (again a fresh mtime).
   Every command it runs in the copy (the product build, the baseline commands, each plant's command) has
   `NIJI_PLANTS_BUILD_TAG=plants-<pid>` in its environment, one value for the whole run. `cargo xtask so` uses it as the
   products' build tag instead of the hash of the sources, so a mutant outside `niji-boundary` and `niji-ld` does not
   rebuild them in every product target directory (the hash changes with every mutant). Products built outside a
   plants run keep the hash tag. `cargo xtask so` also prints a `step: so/<phase> <seconds>s` line per phase.
5. It prints one verdict per plant and a summary line. It exits 1 if any plant SURVIVED (or a step failed), else 0.
   With more than one copy it prints the verdicts again in the file's order before the summary (the copies' lines interleave).

**Which commands rebuild the products** (XTASK-PLANTS-PRODUCTS-REBUILD). A test that links `libc.a` or
`niji-crt1.o` (a `*_products` test, a conformance program) sees a plant only if the products are rebuilt with
it, so for each plant whose command may use the products the tool runs `cargo xtask so` in the copy after
applying the plant and before the command (log `<id>.so.log`; if the mutant does not build, the plant is judged
on that log and the command is not run). A command is known not to use them only when it is one plain
`cargo test` whose targets are all named (`--lib`, `--bin <name>`, `--test <name>`) and no named test target
uses the products by the rule `cargo xtask affected` builds them for (its source or support files name
`target/niji`, `join("niji")` (the ld tests' path to the built `ld.so`), `libc.a`, `niji-crt1` or
`conformance`; xtask's own test targets, which `affected` does not list, by the same text rule, and unsure
when the file names a directory of `xtask/tests/`). Such a plant reuses the baseline build; anything
else (shell syntax, `cargo xtask affected`, all targets of a package, an unknown target or option) rebuilds.
A list of simple commands (`a && b`, `a; b`, `a || b`, bare or as `sh -c '...'`, words plainly quoted) is judged
command by command; a pipe, redirection, expansion, glob, subshell, variable assignment or nested shell counts as
shell syntax. Besides `cargo test`, `cargo xtask headers-gen`, `python3 spec/tools/header_check.py` and
`python3 spec/tools/header_checks/<file>` are known not to use the products (unless an argument names them).
A plant on a file no product build reads (under a crate's `tests/`, `benches/` or `examples/`, or `xtask/tests/`:
the build tag check fails a build that reads one) reuses the baseline build even when its command uses the products.
After a plant that rebuilt the products, the next plant whose command reads them first rebuilds them unmutated
(`<id>.so-restore.log`), so no command sees an earlier plant's mutant. The verdict line names the build
used and its layout (`build: the baseline products (layout ungrouped), not rebuilt`, `rebuilt with the mutant,
layout ungrouped, in <n>s`, or `none (its command reads no products)`).

**Which copies build the products** (PLANTS-PRODUCTS-ONCE). A command *reads* the products when it may use them
(above), measures the layout, or is a `cargo test` of unit tests (`--lib`, `--bin`, `--bins`) that may read them by
the rule `cargo xtask affected` orders the gate's steps with (an xtask unit test whose text joins a products path,
matched against libtest's name filters over the whole test path; another package's when a source under its `src/`
names them). A copy builds the products only when one of its plants reads them, so plants are dealt to copies by
need: plants that rebuild with their mutant in turn over as many copies as there are such plants (at most
`--copies`); plants that only read the unmutated build join those copies (one copy when none rebuilds); plants that
read nothing fill the copies that build nothing, or are spread over all when every copy builds. The run prints
`products: <r> of <n> plants read them, <m> rebuilt with their mutant; built in <b> of <c> copies`. A plant that
reads nothing runs in a copy that may hold no products at all: a command that fails without them fails its
baseline there (exit 2), so such a wrong "reads nothing" shows before any plant is applied.

**The layout of each build** (PLANTS-UNGROUPED-MUTANTS). A mutant's behaviour does not depend on the start-up layout,
and a traced product build costs minutes per mutant, so the tool builds the products of an ordinary plant (and its
baseline) **ungrouped**: it sets `NIJI_LAYOUT_TRACE=ungrouped` for the build and for the plant's command (a command
such as `cargo xtask affected ... --run` that builds the products itself builds them the same way). A plant that
measures the start-up layout is built **traced** (`NIJI_LAYOUT_TRACE=1`, the build and its command): a plant whose
command names a start-up layout test (`hot_text_products`, `heap_meta_products`, `cold_bss_products`: `cargo xtask
affected`'s layout tests) or `layout-check`, or runs `cargo xtask affected ... --run` without `--only` (which selects
the layout tests for a change that reaches `libc.a`), or that is marked `layout:traced` (below). Such a plant always
rebuilds the products. The baseline builds once per layout the file uses (`so.log` ungrouped first, then
`so-traced.log` when the file has both) and runs each command in its plants' layout. The tool sets the variable
itself, so a value in the caller's environment does not matter. A baseline command that fails on the ungrouped
build says so: if it measures the layout, mark its plants `layout:traced`.

Logs go to `<dir>/logs/` (`so.log`, `baseline-<n>.log`, `<id>.log`, `<id>.so.log`, `<id>.so-restore.log`). The tool writes nothing outside `--dir` but the warm copies (above).
`CARGO_TARGET_DIR` is removed from the commands' environment, so the copy builds into its own `target/`.

## The plants file

One plant per line. Blank lines and lines starting with `#` are skipped. Fields are separated by ` | ` (space, bar,
space):

```
id | path | pattern | cmd | msg:<regex>
id | path | pattern | cmd | equivalent:<reason>
id | path | pattern | cmd
```

- `id`: letters, digits, `-`, `_`, `.`; unique in the file. It names the log file.
- `path`: relative to the tree root, without `..`.
- `pattern`, one of:
  - `lit:<old> ==> <new>`: `<old>` must occur exactly once in the file (one line, no escapes). `lit:<old> ==>` deletes it.
  - `perl:s/<re>/<replacement>/<flags>`: one perl substitution (any delimiter). The same expression with `/g` must match
    exactly once. The `/g` and `/r` flags are refused. Use this form for multi-line patterns, or when the text contains
    ` | ` (write `\|` in the regex).
  - On a `.S` file, the pattern must name its function's label, for example
    `perl:s/(niji_machine_strchr:.*?)<pattern>/$1<replacement>/s`. An unanchored pattern mutates the first match in the
    file, which is often another function.
- `cmd`: the direct tests of the planted file, for example `cargo test -p niji --test stdio_short_io` (crates/core is package `niji`). For code
  reached only from C programs: `cargo xtask affected --only conformance:<prog> --with-exports --run`; a failed program's
  status and the last 40 lines of its stdout and stderr are on the console (D-X.27), so `msg:` can name the program's own
  failure message. A `cmd` may contain ` | `, but a job's output should not be piped.
- The last field:
  - `msg:<regex>` (a `grep -E` regular expression): the failing test or message the plant must produce. Name the
    failure, not just the test, because a passing run prints the test's name too (for example `msg:my_test.*FAILED|panicked`).
  - `equivalent:<reason>`: the mutant behaves like the original by design. If its command passes, it is EQUIVALENT.
  - none: the plant is judged by exit status alone.
- An optional field after all of them, `layout:traced`: the plant's products are built with the traced start-up
  layout (see "The layout of each build"). Use it for a command that measures the layout without naming a layout test,
  for example `id | path | pattern | cmd | msg:<regex> | layout:traced`. Any other `layout:` value is refused.

## Verdicts

| Verdict | Meaning | Exit |
|---|---|---|
| `CAUGHT` | the command failed and its log matches `msg:` (or a plant declared equivalent was caught anyway), or it ran into the timeout | 0 |
| `WARN-NO-MSG` | the command failed, but its log does not match `msg:` | 0 |
| `WARN-RC-ONLY` | the command failed, and the plant has no `msg:` | 0 |
| `EQUIVALENT` | the command passed, and the plant is declared `equivalent:` | 0 |
| `SURVIVED` | the command passed on the mutant | 1 |

Every plant whose command failed (CAUGHT, WARN-NO-MSG, WARN-RC-ONLY) gets a second line,
`plants: <id>: caught by <test binary> <test name>`: the first failing test of its log (`cargo test`'s
`test <name> ... FAILED` under the binary of the last `Running` line, or nextest's `FAIL [..] <crate>::<target> <name>`).
Without a failing test line it names a panic's thread, a panic outside a named test with its binary, the binary cargo
asks to rerun, `the build (the mutant does not compile)`, or `no test the log names`; a timeout is `caught by timeout`.
EQUIVALENT and SURVIVED lines are unchanged (XTASK-PLANTS-CATCHER).

At the end the tool writes `<dir>/results.txt`: the plants file with a `# cargo xtask plants results (...)` comment
block, one line per plant (`# <id>: CAUGHT, caught by <binary> <test>`). Commit that file as
`handoff/<STORY>-plants.txt`, so which test caught each plant stays on record after the logs are gone; a later run from
it replaces the block.

List every WARN line in your hand-back. Exit codes: 0 all caught or equivalent; 1 a plant SURVIVED or a step failed;
2 the products or a baseline command failed; 3 refused input.

## Example

```
# STDIO-SHORT-IO plants
retry | crates/boundary/src/stdio.rs | lit:while done < len { ==> if done < len { | cargo test -p niji --test stdio_short_io | msg:short_write.*(FAILED|panicked)
memchr-tail | csys/src/machine/scan.S | perl:s/(niji_machine_memchr:.*?)jb\s+\.Ltail/$1jmp .Ltail/s | cargo test -p niji-boundary --test small_copy | msg:memchr
```

```
~/bin/jobrun short-io-plants plants -- sh -c '. ~/.cargo/env; cd ~/w/<role> && cargo xtask plants ~/work/<story>/plants.txt --dir /dev/shm/claude/<story>/plants-1'
```
