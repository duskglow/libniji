//! `cargo xtask plants <file> --dir <new-dir> [--timeout <secs>] [--prepare-timeout <secs>] [--copies <n>|auto] [--warm <dir>|--cold]`: run a story's plants (mutations
//! that its tests must catch) the one way the rules ask for, instead of a hand-written script per
//! story (XTASK-PLANTS; `docs/plants.md`).
//!
//! It removes four failure classes that repeated in hand-written plants scripts:
//! - a reused copy directory (rsync restores old mtimes, so cargo skipped the rebuild and tested a
//!   stale mutant): `--dir` must not exist, and nothing in it is ever deleted; the one copy that is reused, the role's
//!   warm copy (PLANTS-WARM-COPY), is synced by content with fresh mtimes ([`sync_warm`]);
//! - a copy without the products (`cargo xtask so` is run in the copy before the baseline);
//! - a products test run against the baseline products, blind to the mutant (XTASK-PLANTS-PRODUCTS-REBUILD: the
//!   products are rebuilt with each plant whose command may use them, see [`cmd_uses_products`]);
//! - an unanchored pattern that mutated the wrong site (every plant must change exactly one site,
//!   and a plant on a `.S` file must name its `niji_machine_<fn>:` label);
//! - a script that always exits 0 (any SURVIVED exits 1; a failed baseline exits 2);
//! - a copy missing a source file a test needs: after the copy every source path (`git ls-files`, tracked
//!   and untracked-not-ignored, in a git work tree; every file and symlink outside `/target` and
//!   `/fuzz/target` in a synced role, which has no `.git`) must exist in it, else the run is refused.
//!
//! A `--dir` under `/dev/shm` prints WARN-DEV-SHM: a test that mounts its own `/dev` or `/dev/shm`
//! hides such a copy (XTASK-PLANTS-TRACKED-COPY: `nscache_mode.sh` binds the copy back; the
//! `nodev` phase of `posixipc_run.sh` cannot).
//!
//! The plants file has one plant per line (blank lines and `#` comments are skipped):
//! `id | path | pattern | cmd | msg:<regex>` or `id | path | pattern | cmd | equivalent:<reason>`,
//! fields separated by ` | ` (space, bar, space). `pattern` is `lit:<old> ==> <new>` (a literal
//! that must occur exactly once) or `perl:s/<re>/<replacement>/<flags>` (applied by `perl`, which
//! must match exactly once; no `g` or `r` flag). `cmd` runs with `sh -c` in the copy's root.
//!
//! Everything is written under `--dir` (the copy in `<dir>/src` and the logs in `<dir>/logs`), except the warm copy.
//!
//! PLANTS-COPIES: `--copies <n>` runs the plants in `n` tree copies at the same time (`<dir>/src-2`, `<dir>/logs-2`, ...),
//! plant `i` in copy `i % n` ([`distribute`]); each copy builds and runs the baselines of its own plants, and the verdicts
//! and `results.txt` come out in the plants file's order, as a serial run's. The default, `auto`, is two copies when the
//! RAM this job may use holds two and the slot's CPUs or the file's plants make the split pay ([`auto_copies`],
//! [`ram_limit`]), else one.
//!
//! PLANTS-WARM-COPY: copy `k` runs in the role's warm copy `<warm>/copy-<k>` when no other run holds it (its `flock`),
//! instead of a cold copy in `--dir`: the cargo cache stays, so the baseline build costs what the change since the last
//! run costs, not a build from nothing. Before use the warm copy is brought to the tree by content ([`sync_warm`]: a
//! changed file gets a fresh mtime, a path the tree lacks goes, `target/` keeps cargo's cache only,
//! [`prune_to_cargo_cache`]). `<warm>` is `--warm <dir>`, else [`default_warm`]; `--cold` makes every copy cold. The
//! logs and `results.txt` stay in `--dir`.

use std::fs;
use std::io::Write as _;
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

/// Exit status: every plant CAUGHT (or EQUIVALENT).
const EXIT_OK: u8 = 0;
/// Exit status: at least one plant SURVIVED, or a step failed after the baseline.
const EXIT_SURVIVED: u8 = 1;
/// Exit status: the product build or a baseline command failed on the unmutated copy.
const EXIT_BASELINE: u8 = 2;
/// Exit status: the input was refused (usage, plants file, an ambiguous plant, an existing `--dir`).
const EXIT_REFUSED: u8 = 3;

/// The default limit for one command, in seconds (a mutant may loop forever). The project's per-plant limit
/// (owner, 2026-11-05): `--timeout 600`.
const DEFAULT_TIMEOUT: u64 = 600;

/// The default limit for the product build (`cargo xtask so`, baseline and per plant), in seconds. It is
/// separate from [`DEFAULT_TIMEOUT`]: that one decides CAUGHT-by-hang for a mutant's tests, and a traced cold
/// build takes longer than it on a slow host. A build that runs into this limit says nothing about the mutant.
const DEFAULT_PREPARE_TIMEOUT: u64 = 3600;

/// The exit status of `timeout(1)` when it ended the command: a mutant that hangs is caught by the limit.
const TIMED_OUT: i32 = 124;

/// How a command ended, for a message: a timeout is named with its limit and the flag that sets it, never left as a
/// bare `rc 124` (TOOLING-PAPERCUTS: a timed-out baseline printed only `rc 124`).
fn ended(rc: i32, limit: u64, flag: &str) -> String {
    if rc == TIMED_OUT { format!("timed out after {limit} s ({flag} <secs>)") } else { format!("rc {rc}") }
}

/// The plants a baseline step serves: every plant of the layout for a product build, else those whose command it is.
fn baseline_plants<'a>(plants: &'a [Plant], layouts: &[crate::hotlists::Layout], layout: crate::hotlists::Layout, argv: &[String], build: bool) -> Vec<&'a str> {
    plants.iter().zip(layouts).filter(|(p, l)| **l == layout && (build || sh(&p.cmd) == argv)).map(|(p, _)| p.id.as_str()).collect()
}

/// How a plant is expected to be judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expect {
    /// `msg:<regex>`: a failing `cmd` whose log matches the extended regular expression.
    Msg(String),
    /// `equivalent:<reason>`: the mutant behaves like the original by design.
    Equivalent(String),
    /// Neither: caught by exit status alone (WARN-RC-ONLY).
    RcOnly,
}

/// How a plant changes its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    /// `lit:<old> ==> <new>`.
    Literal { old: String, new: String },
    /// `perl:<s/// expression>`.
    Perl(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plant {
    pub id: String,
    pub path: String,
    pub pattern: Pattern,
    pub cmd: String,
    pub expect: Expect,
    /// PLANTS-UNGROUPED-MUTANTS: the plant is marked `layout:traced` (its command measures the start-up layout).
    /// A command that names a layout test is traced without the mark ([`targets_layout`]).
    pub layout: bool,
}

/// The verdict on one plant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Caught,
    Survived,
    Equivalent,
    WarnRcOnly,
    WarnNoMsg,
    /// The product build for the mutant ran into the prepare timeout: the mutant was not tested.
    BuildTimeout,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Verdict::Caught => "CAUGHT",
            Verdict::Survived => "SURVIVED",
            Verdict::Equivalent => "EQUIVALENT",
            Verdict::WarnRcOnly => "WARN-RC-ONLY",
            Verdict::WarnNoMsg => "WARN-NO-MSG",
            Verdict::BuildTimeout => "BUILD-TIMEOUT",
        }
    }
}

/// One run: the tree to copy, the plants, the new directory, the command that builds the
/// products in the copy (`cargo xtask so`; the unit tests use a toy tree without one), and which
/// plants' commands use the products (given the copy; [`plants_use_products`]).
pub struct Config {
    pub src: PathBuf,
    /// The plants file as read: `<dir>/results.txt` is this text with the results appended.
    pub text: String,
    pub plants: Vec<Plant>,
    pub dir: PathBuf,
    pub prepare: Vec<String>,
    pub timeout: u64,
    /// The limit for `prepare` (the product build), apart from `timeout`.
    pub prepare_timeout: u64,
    pub uses_products: fn(&Path, &[Plant]) -> Vec<bool>,
    /// PLANTS-PRODUCTS-ONCE: which plants' commands may read the products at all ([`plants_read_products`]); a plant
    /// that uses them reads them, whatever this says.
    pub reads_products: fn(&Path, &[Plant]) -> Vec<bool>,
    /// PLANTS-COPIES: how many tree copies run plants at the same time (1: serial); `--copies`, default
    /// [`auto_copies`].
    pub copies: usize,
    /// PLANTS-WARM-COPY: the role's warm copies, `<warm>/copy-<k>` (each locked by `<warm>/copy-<k>.lock` for one run
    /// at a time); `None`: every copy is cold, in `--dir` (`--cold`). Default [`default_warm`].
    pub warm: Option<PathBuf>,
}

/// XTASK-PLANTS-PRODUCTS-REBUILD: per plant, whether its command may use the products, so that
/// they are rebuilt with the mutant first. The test targets are judged by the rule
/// `cargo xtask affected` uses to build the products (`Repo::test_uses_products`).
pub fn plants_use_products(copy: &Path, plants: &[Plant]) -> Vec<bool> {
    let repo = crate::affected::Repo::load(copy);
    plants.iter().map(|p| cmd_uses_products(&p.cmd, |krate, name| repo.test_uses_products(krate, name).or_else(|| xtask_test_uses_products(copy, krate, name)))).collect()
}

/// PLANTS-PRODUCTS-ONCE: per plant, whether its command may read the products at all (so a copy that runs it builds
/// them before the baselines): it may use them ([`plants_use_products`]), or it runs unit tests that may read them
/// ([`crate::affected::unit_tests_may_read_products`], the rule the gate orders its steps with).
pub fn plants_read_products(copy: &Path, plants: &[Plant]) -> Vec<bool> {
    let repo = crate::affected::Repo::load(copy);
    let lookup = |krate: Option<&str>, name: &str| repo.test_uses_products(krate, name).or_else(|| xtask_test_uses_products(copy, krate, name));
    let units = |krate: Option<&str>, filters: &[String], exact: bool| crate::affected::unit_tests_may_read_products(copy, &repo, krate, filters, exact);
    plants.iter().map(|p| cmd_reads_products(&p.cmd, lookup, units)).collect()
}

/// PLANTS-PRODUCTS-ONCE: whether a change to `path` (relative to the tree's root) may change what `cargo xtask so`
/// builds. Not only for a file under a crate's `tests/`, `benches/` or `examples/` directory (cargo compiles those
/// for `cargo test`, `cargo bench` and `cargo run --example` only, and a product build that reads one fails its
/// build-tag check, `so::TAG_SKIPPED_DIRS`) and under `xtask/tests/` (xtask's integration tests, never part of the
/// `xtask` binary that builds the products). Anything else, an odd path included, may: a plant on it whose command
/// reads the products gets them rebuilt with its mutant.
pub fn may_change_products(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    if path.starts_with('/') || parts.iter().any(|p| p.is_empty() || *p == "." || *p == "..") {
        return true;
    }
    !matches!(parts.as_slice(), ["crates", _, dir, _, ..] if ["tests", "benches", "examples"].contains(dir)) && !matches!(parts.as_slice(), ["xtask", "tests", _, ..])
}

/// What one plant needs from the products ([`needs`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Need {
    /// Its command may read the products: the copy that runs it builds them (unmutated) before the baselines.
    pub reads: bool,
    /// They are rebuilt with its mutant before its command: the command may use them and the plant's file may change
    /// them ([`may_change_products`]). Never set without `reads`.
    pub mutant: bool,
}

/// PLANTS-PRODUCTS-ONCE: each plant's [`Need`], from what its command uses (`uses`, [`plants_use_products`]) and
/// reads (`reads`, [`plants_read_products`]) and its layout (a plant that measures the layout reads the products by
/// definition).
pub fn needs(plants: &[Plant], uses: &[bool], reads: &[bool], traced: &[bool]) -> Vec<Need> {
    plants.iter().enumerate().map(|(i, p)| {
        let reads = uses[i] || reads[i] || traced[i];
        Need { reads, mutant: (uses[i] || traced[i]) && may_change_products(&p.path) }
    }).collect()
}

/// PLANTS-PRODUCTS-ONCE: the plants each of `copies` tree copies runs, as indices into the plants file (each share in
/// file order, no share empty), so that the products are built in as few copies as possible: a copy builds them only
/// when one of its plants reads them. Plants that rebuild them with their mutant are dealt in turn over as many copies
/// as there are such plants (each such copy builds anyway); plants that only read the unmutated products join those
/// copies (one copy when no plant rebuilds), so they never make another copy build; the plants that read none fill
/// the copies that build nothing, or, when every copy builds, are dealt over all of them.
pub fn assign(needs: &[Need], copies: usize) -> Vec<Vec<usize>> {
    let copies = copies.clamp(1, needs.len().max(1));
    let class = |f: &dyn Fn(&Need) -> bool| -> Vec<usize> { needs.iter().enumerate().filter(|(_, n)| f(n)).map(|(i, _)| i).collect() };
    let (mutant, reads, none) = (class(&|n| n.mutant), class(&|n| n.reads && !n.mutant), class(&|n| !n.reads));
    let builders = if !mutant.is_empty() { copies.min(mutant.len()) } else { usize::from(!reads.is_empty()) };
    let mut shares = vec![Vec::new(); copies];
    for (k, &i) in mutant.iter().chain(&reads).enumerate() {
        shares[k % builders].push(i);
    }
    let (first, spread) = if builders < copies { (builders, copies - builders) } else { (0, copies) };
    for (k, &i) in none.iter().enumerate() {
        shares[first + k % spread].push(i);
    }
    shares.retain(|s| !s.is_empty());
    for s in &mut shares {
        s.sort_unstable();
    }
    if shares.is_empty() {
        shares.push(Vec::new());
    }
    shares
}

/// `Repo` leaves xtask's own crate out (it is the tool, not a product crate), so an xtask test target is judged here
/// by the same text rule ([`crate::affected::names_products`]). A plant on a header template or a generator then reuses
/// the baseline build for a test that runs the generator itself, instead of a product build that refuses the mutant's
/// stale `libc-include/` before the test runs (SL-HEADERS-VIS-FIX). Unsure (no package named, no such file, or the
/// file names a directory of `xtask/tests/`, which may be support code) is `None`: rebuild.
fn xtask_test_uses_products(copy: &Path, krate: Option<&str>, name: &str) -> Option<bool> {
    if krate != Some("xtask") || name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return None;
    }
    let tests = copy.join("xtask/tests");
    let text = fs::read_to_string(tests.join(format!("{name}.rs"))).ok()?;
    let names_a_dir = fs::read_dir(&tests).ok()?.flatten().any(|e| e.path().is_dir() && text.contains(e.file_name().to_string_lossy().as_ref()));
    if names_a_dir {
        return None;
    }
    Some(crate::affected::names_products(&text))
}

/// Whether `cmd` may use the products. It is known not to only when it is one plain `cargo test`
/// whose targets are all named (`--lib`, `--bin <name>`, `--test <name>`) and no named test target
/// uses them (`lookup(package, target) == Some(false)`). Anything else (shell syntax, another
/// program such as `cargo xtask affected`, an unknown option, all targets of a package, an unknown
/// target) may: when unsure, rebuild.
///
/// PLANTS-PRODUCTS-ONCE: a `sh -c '<list>'` (or the bare list) of simple commands joined by `&&`, `||` or `;` is
/// judged command by command ([`simple_commands`]); besides `cargo test`, the commands of [`RUNS_NO_PRODUCTS`] are
/// known not to use them (no argument naming them, [`crate::affected::names_products`]).
pub fn cmd_uses_products(cmd: &str, lookup: impl Fn(Option<&str>, &str) -> Option<bool>) -> bool {
    let Some(cmds) = simple_commands(cmd) else { return true };
    cmds.iter().any(|w| simple_uses(w, &lookup))
}

/// PLANTS-PRODUCTS-ONCE: whether `cmd` may read the products at all: it may use them ([`cmd_uses_products`]), or a
/// `cargo test` of it runs unit tests (`--lib`, `--bin`, `--bins`) that may (`units(package, filters, exact)`).
pub fn cmd_reads_products(cmd: &str, lookup: impl Fn(Option<&str>, &str) -> Option<bool>, units: impl Fn(Option<&str>, &[String], bool) -> bool) -> bool {
    let Some(cmds) = simple_commands(cmd) else { return true };
    cmds.iter().any(|w| simple_uses(w, &lookup) || cargo_test(w).is_some_and(|t| t.unit && units(t.package.as_deref(), &t.filters, t.exact)))
}

/// Commands known to read no products (PLANTS-PRODUCTS-ONCE), as word prefixes, with why: a plant whose command is
/// made of these and `cargo test`s of targets that read none needs no product build. `*` matches one plain name.
const RUNS_NO_PRODUCTS: &[(&[&str], &str)] = &[
    (&["cargo", "xtask", "headers-gen"], "writes libc-include/ from headers/ and spec/; builds nothing else"),
    (&["python3", "spec/tools/header_check.py"], "compiles units against libc-include/ (-nostdinc, -fsyntax-only); target/niji/include only when an argument names it"),
    (&["python3", "spec/tools/header_checks/*"], "a slice's header checks, run like header_check.py's"),
];

/// Whether simple command `words` is one of [`RUNS_NO_PRODUCTS`] with no argument naming the products.
fn runs_no_products(words: &[String]) -> bool {
    let plain = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)) && !w.starts_with('.');
    let prefix_ok = |prefix: &[&str]| {
        words.len() >= prefix.len()
            && prefix.iter().zip(words).all(|(p, w)| match p.strip_suffix('*') {
                Some(dir) => w.strip_prefix(dir).is_some_and(plain),
                None => p == w,
            })
    };
    RUNS_NO_PRODUCTS.iter().any(|(prefix, _)| prefix_ok(prefix)) && !words.iter().any(|w| crate::affected::names_products(w))
}

/// One simple command's verdict for [`cmd_uses_products`]: a `cargo test` by its targets, a [`RUNS_NO_PRODUCTS`]
/// command no, anything else may.
fn simple_uses(words: &[String], lookup: &impl Fn(Option<&str>, &str) -> Option<bool>) -> bool {
    if runs_no_products(words) {
        return false;
    }
    match cargo_test(words) {
        None => true,
        Some(t) if !t.unit && t.tests.is_empty() => true,
        Some(t) => t.tests.iter().any(|n| lookup(t.package.as_deref(), n) != Some(false)),
    }
}

/// A `cargo test` whose targets are all named, as [`cargo_test`] reads it.
#[derive(Debug, PartialEq, Eq)]
struct CargoTest {
    package: Option<String>,
    /// `--lib`, `--bin <name>` or `--bins`: the package's unit tests.
    unit: bool,
    tests: Vec<String>,
    /// libtest's name filters (none: every test), and `--exact`.
    filters: Vec<String>,
    exact: bool,
}

/// `words` as a plain `cargo test` of named targets (`--lib`, `--bin <name>`, `--bins`, `--test <name>`; no target
/// flag: all of them, `unit` and `tests` empty); `None` for anything else (another program, an unknown option, a
/// second package).
fn cargo_test(words: &[String]) -> Option<CargoTest> {
    // `--opt=value` is `--opt value`.
    let words: Vec<&str> = words.iter().flat_map(|w| match w.split_once('=') {
        Some((k, v)) if k.starts_with("--") => vec![k, v],
        _ => vec![w.as_str()],
    }).collect();
    let mut it = words.into_iter();
    if it.next() != Some("cargo") || it.next() != Some("test") {
        return None;
    }
    let mut t = CargoTest { package: None, unit: false, tests: Vec::new(), filters: Vec::new(), exact: false };
    let mut harness = false;
    while let Some(w) = it.next() {
        match w {
            // The test binaries' own arguments: libtest's filters and options.
            "--" if !harness => harness = true,
            "--exact" if harness => t.exact = true,
            "--skip" | "--test-threads" | "--format" | "--color" | "-Z" if harness => {
                it.next()?;
            }
            w if harness && w.starts_with('-') => {}
            "-p" | "--package" if t.package.is_none() => t.package = Some(it.next()?.to_string()),
            "--lib" | "--bins" => t.unit = true,
            "--bin" => {
                it.next()?;
                t.unit = true;
            }
            "--test" => t.tests.push(it.next()?.to_string()),
            "-q" | "--quiet" | "--release" | "--no-fail-fast" | "--locked" | "--offline" | "--frozen" => {}
            "--features" | "-F" | "-j" | "--jobs" => {
                it.next();
            }
            // `--tests`, `--all-targets`, `--workspace`, `--doc`, a second package, ...
            w if w.starts_with('-') => return None,
            // A test name filter.
            w => t.filters.push(w.to_string()),
        }
    }
    Some(t)
}

/// PLANTS-PRODUCTS-ONCE: `cmd` as its simple commands (words), when it is a list of them joined by `&&`, `||` or `;`,
/// bare or as the one argument of `sh -c '...'` / `bash -c '...'`. Words may be quoted (`'...'`, or `"..."` without
/// `$`, `` ` `` or `\`); `None` (so: may use the products) for anything else the shell would read: a pipe,
/// redirection, expansion, glob, subshell, background job, comment, a leading variable assignment or a nested `sh`.
pub fn simple_commands(cmd: &str) -> Option<Vec<Vec<String>>> {
    let t = cmd.trim();
    let inner = ["sh -c '", "bash -c '"].iter().find_map(|p| t.strip_prefix(p)?.strip_suffix('\'')).filter(|i| !i.contains('\''));
    let src = inner.unwrap_or(t);
    let (mut cmds, mut cur, mut word, mut in_word) = (Vec::new(), Vec::<String>::new(), String::new(), false);
    let mut chars = src.chars().peekable();
    let end_cmd = |cur: &mut Vec<String>, cmds: &mut Vec<Vec<String>>| -> Option<()> {
        if cur.is_empty() {
            return None;
        }
        cmds.push(std::mem::take(cur));
        Some(())
    };
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => {
                if in_word {
                    cur.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            '\'' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '\'' => break,
                        ch => word.push(ch),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '$' | '`' | '\\' => return None,
                        ch => word.push(ch),
                    }
                }
            }
            '&' | '|' | ';' => {
                if c != ';' && chars.next() != Some(c) {
                    return None;
                }
                if in_word {
                    cur.push(std::mem::take(&mut word));
                    in_word = false;
                }
                end_cmd(&mut cur, &mut cmds)?;
            }
            c if "$`\\()<>*?[]{}~#\n".contains(c) => return None,
            c => {
                in_word = true;
                word.push(c);
            }
        }
    }
    if in_word {
        cur.push(word);
    }
    end_cmd(&mut cur, &mut cmds)?;
    // A variable assignment before the command, or a shell inside the list.
    if cmds.iter().any(|w| w[0].contains('=') || ["sh", "bash", "env", "eval", "exec"].contains(&w[0].as_str())) {
        return None;
    }
    Some(cmds)
}

/// PLANTS-UNGROUPED-MUTANTS: whether `cmd` measures the start-up layout, so its products must be built traced
/// (`NIJI_LAYOUT_TRACE=1`) rather than ungrouped: it names a start-up layout test (`cargo xtask affected`'s
/// [`crate::affected::LAYOUT_TESTS`]) or `layout-check`, or it is a `cargo xtask affected ... --run` without
/// `--only` (a change that reaches `libc.a` selects the layout tests). Any other plant can mark itself `layout:traced`.
pub fn targets_layout(cmd: &str) -> bool {
    let names = |w: &str| w.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-')).any(|t| t == "layout-check" || crate::affected::LAYOUT_TESTS.iter().any(|(_, name)| t == *name));
    let words: Vec<&str> = cmd.split_whitespace().collect();
    let affected_run = words.windows(2).any(|w| w == ["xtask", "affected"]) && words.contains(&"--run") && !words.iter().any(|w| w.starts_with("--only"));
    affected_run || words.iter().any(|w| names(w))
}

/// The start-up layout of a plants run's product build (`NIJI_LAYOUT_TRACE`, [`crate::hotlists::TRACE_ENV`]).
fn layout_env(layout: crate::hotlists::Layout) -> &'static str {
    match layout {
        crate::hotlists::Layout::Traced => "1",
        crate::hotlists::Layout::Ungrouped => "ungrouped",
    }
}

fn layout_name(layout: crate::hotlists::Layout) -> &'static str {
    match layout {
        crate::hotlists::Layout::Traced => "traced",
        crate::hotlists::Layout::Ungrouped => "ungrouped",
    }
}

/// The outcome of a run: the exit status and the verdict per plant (in file order).
#[derive(Debug)]
pub struct Outcome {
    pub code: u8,
    pub verdicts: Vec<(String, Verdict)>,
}

/// `cargo xtask plants`: parse the arguments and run.
pub fn main(root: &Path, args: &[String]) -> ExitCode {
    match parse_args(root, args) {
        Ok(cfg) => {
            let out = run(&cfg);
            // The rules ask for every WARN line in the hand-back: repeat them together at the end.
            let warns: Vec<String> = out.verdicts.iter().filter(|(_, v)| matches!(v, Verdict::WarnRcOnly | Verdict::WarnNoMsg)).map(|(id, v)| format!("{id} {}", v.label())).collect();
            if !warns.is_empty() {
                println!("plants: WARN lines for the hand-back: {}", warns.join(", "));
            }
            ExitCode::from(out.code)
        }
        Err(e) => {
            eprintln!("xtask plants: REFUSED: {e}");
            ExitCode::from(EXIT_REFUSED)
        }
    }
}

const USAGE: &str = "usage: cargo xtask plants <plants-file> --dir <new-directory> [--timeout <secs>] [--prepare-timeout <secs>] [--copies <n>|auto] [--warm <dir>|--cold]";

fn parse_args(root: &Path, args: &[String]) -> Result<Config, String> {
    let (mut file, mut dir, mut timeout, mut prepare_timeout) = (None, None, DEFAULT_TIMEOUT, DEFAULT_PREPARE_TIMEOUT);
    // `None`: auto.
    let mut copies: Option<usize> = None;
    // `None`: the default ([`default_warm`]); `Some(None)`: `--cold`.
    let mut warm: Option<Option<PathBuf>> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--dir" => dir = Some(PathBuf::from(it.next().ok_or("--dir needs a directory")?)),
            "--warm" => warm = Some(Some(PathBuf::from(it.next().ok_or("--warm needs a directory")?))),
            "--cold" => warm = Some(None),
            "--copies" => {
                copies = match it.next().map(String::as_str) {
                    Some("auto") => None,
                    v => Some(v.and_then(|c| c.parse().ok()).filter(|c| (1..=8).contains(c)).ok_or("--copies needs a number from 1 to 8, or auto")?),
                }
            }
            "--timeout" => timeout = it.next().and_then(|t| t.parse().ok()).filter(|t| *t > 0).ok_or("--timeout needs a positive number of seconds")?,
            "--prepare-timeout" => prepare_timeout = it.next().and_then(|t| t.parse().ok()).filter(|t| *t > 0).ok_or("--prepare-timeout needs a positive number of seconds")?,
            s if s.starts_with('-') => return Err(format!("unknown option {s} ({USAGE})")),
            s if file.is_none() => file = Some(PathBuf::from(s)),
            s => return Err(format!("unexpected argument {s} ({USAGE})")),
        }
    }
    let file = file.ok_or(USAGE)?;
    let dir = dir.ok_or(format!("--dir is required: a NEW directory in RAM, e.g. /dev/shm/claude/<story>/plants-<n> ({USAGE})"))?;
    let text = fs::read_to_string(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let absolute = |p: PathBuf| -> Result<PathBuf, String> { Ok(if p.is_absolute() { p } else { std::env::current_dir().map_err(|e| e.to_string())?.join(p) }) };
    let dir = absolute(dir)?;
    let warm = match warm {
        None => default_warm(&dir, root),
        Some(w) => w.map(absolute).transpose()?,
    };
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let plants = parse_plants(&text)?;
    let copies = copies.unwrap_or_else(|| {
        let limit = ram_limit();
        let cpus = std::thread::available_parallelism().map_or(1, usize::from);
        let n = auto_copies(limit, cpus, plants.len());
        let gib = |b: u64| format!("{:.1} GiB", b as f64 / (1u64 << 30) as f64);
        println!("plants: --copies auto: {n} (RAM this job may use: {}; one copy needs up to {}; {cpus} CPUs, {} plants; --copies <n> overrides)", limit.map_or("unknown".into(), gib), gib(COPY_PEAK_BYTES), plants.len());
        n
    });
    Ok(Config { src: root.to_path_buf(), plants, text, dir, prepare: vec![cargo, "xtask".into(), "so".into()], timeout, prepare_timeout, uses_products: plants_use_products, reads_products: plants_read_products, copies, warm })
}

/// Parse a plants file (see the module comment). Every line is checked; the errors are joined.
pub fn parse_plants(text: &str) -> Result<Vec<Plant>, String> {
    let mut plants: Vec<Plant> = Vec::new();
    let mut errors = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_line(line) {
            Ok(p) if plants.iter().any(|q| q.id == p.id) => errors.push(format!("line {}: duplicate plant id {}", n + 1, p.id)),
            Ok(p) => plants.push(p),
            Err(e) => errors.push(format!("line {}: {e}", n + 1)),
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    if plants.is_empty() {
        return Err("the plants file has no plants".into());
    }
    Ok(plants)
}

fn parse_line(line: &str) -> Result<Plant, String> {
    let mut fields: Vec<&str> = line.split(" | ").map(str::trim).collect();
    // PLANTS-UNGROUPED-MUTANTS: an optional last field `layout:traced`, after the expectation.
    let mut layout = false;
    if fields.len() >= 5 {
        if let Some(v) = fields[fields.len() - 1].strip_prefix("layout:") {
            if v.trim() != "traced" {
                return Err(format!("`layout:{v}`: the only layout mark is `layout:traced` (the plant's products are built with the traced start-up layout)"));
            }
            layout = true;
            fields.pop();
        }
    }
    if fields.len() < 4 {
        return Err("expected `id | path | pattern | cmd | msg:<regex>` (or `equivalent:<reason>` last)".into());
    }
    let (id, path, pattern) = (fields[0], fields[1], fields[2]);
    // The last field (from the fifth on) is the expectation; a `cmd` may itself contain ` | `.
    let last = if fields.len() >= 5 { fields[fields.len() - 1] } else { "" };
    let expect = if let Some(re) = last.strip_prefix("msg:") {
        if re.is_empty() { return Err("msg: needs a regex".into()) }
        Expect::Msg(re.to_string())
    } else if let Some(why) = last.strip_prefix("equivalent:") {
        if why.trim().is_empty() { return Err("equivalent: needs a reason".into()) }
        Expect::Equivalent(why.trim().to_string())
    } else {
        Expect::RcOnly
    };
    let cmd_end = if expect == Expect::RcOnly { fields.len() } else { fields.len() - 1 };
    let cmd = fields[3..cmd_end].join(" | ");
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) {
        return Err(format!("bad plant id `{id}` (letters, digits, - _ . only)"));
    }
    let rel = Path::new(path);
    if path.is_empty() || rel.is_absolute() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(format!("{id}: path `{path}` must be relative to the tree, without `..`"));
    }
    if cmd.trim().is_empty() {
        return Err(format!("{id}: empty cmd"));
    }
    let pattern = if let Some(lit) = pattern.strip_prefix("lit:") {
        // `<old> ==>` with nothing after it deletes `<old>` (the field is trimmed).
        let (old, new) = lit.split_once(" ==> ").or_else(|| lit.strip_suffix(" ==>").map(|o| (o, ""))).ok_or(format!("{id}: lit: needs `<old> ==> <new>`"))?;
        if old.is_empty() { return Err(format!("{id}: empty literal")) }
        Pattern::Literal { old: old.to_string(), new: new.to_string() }
    } else if let Some(e) = pattern.strip_prefix("perl:") {
        check_perl(e).map_err(|m| format!("{id}: {m}"))?;
        Pattern::Perl(e.to_string())
    } else {
        return Err(format!("{id}: pattern must start with lit: or perl:"));
    };
    if path.ends_with(".S") {
        let text = match &pattern { Pattern::Literal { old, .. } => old.as_str(), Pattern::Perl(e) => e.as_str() };
        if !names_machine_anchor(text) {
            return Err(format!("{id}: a plant on a .S file must name its `niji_machine_<fn>:` label (e.g. perl:s/(niji_machine_<fn>:.*?)<pattern>/$1<replacement>/s)"));
        }
    }
    Ok(Plant { id: id.to_string(), path: path.to_string(), pattern, cmd, expect, layout })
}

/// A `perl:` pattern is one `s` substitution changing one site: no `g` (every site) and no `r`
/// (which would leave the file unchanged).
fn check_perl(e: &str) -> Result<(), String> {
    let mut chars = e.chars();
    let delim_ok = chars.next() == Some('s') && chars.next().is_some_and(|d| !d.is_alphanumeric() && !d.is_whitespace());
    if !delim_ok {
        return Err(format!("perl: must be one substitution s/<re>/<replacement>/<flags>, got `{e}`"));
    }
    let flags: String = e.chars().rev().take_while(char::is_ascii_lowercase).collect();
    if flags.contains('g') || flags.contains('r') {
        return Err("perl: the /g and /r flags are refused (a plant changes exactly one site)".into());
    }
    Ok(())
}

fn names_machine_anchor(text: &str) -> bool {
    text.match_indices("niji_machine_").any(|(i, m)| {
        let rest = &text[i + m.len()..];
        let name_len = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').count();
        name_len > 0 && rest[name_len..].starts_with(':')
    })
}

/// The mutant of `src`, or why the plant is refused (no site, more than one site, no change).
pub fn mutate(src: &str, pattern: &Pattern) -> Result<String, String> {
    let (sites, out) = match pattern {
        Pattern::Literal { old, new } => (src.matches(old.as_str()).count(), src.replacen(old.as_str(), new, 1)),
        Pattern::Perl(e) => perl_substitute(src, e)?,
    };
    match sites {
        0 => Err("the pattern matches no site".into()),
        1 if out == src => Err("the pattern matches one site but the replacement changes nothing".into()),
        1 => Ok(out),
        n => Err(format!("the pattern matches {n} sites; a plant must change exactly one (anchor it)")),
    }
}

/// Apply a `s///` expression with perl: the number of sites it matches (the same expression with
/// `/g`) and the text after one substitution.
fn perl_substitute(src: &str, e: &str) -> Result<(usize, String), String> {
    const SCRIPT: &str = r#"binmode STDIN; binmode STDOUT; local $/; my $src = <STDIN>; my $e = $ENV{NIJI_PLANT_EXPR};
my $all = $src; my $n = eval "\$all =~ ${e}g"; die "bad perl: expression: $@" if $@;
my $one = $src; eval "\$one =~ $e"; die "bad perl: expression: $@" if $@;
print(($n || 0), "\n", $one);"#;
    let mut child = Command::new("perl")
        .args(["-e", SCRIPT])
        .env("NIJI_PLANT_EXPR", e)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("cannot run perl: {err}"))?;
    let mut stdin = child.stdin.take().ok_or("perl: no stdin")?;
    let input = src.as_bytes().to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let out = child.wait_with_output().map_err(|err| format!("perl: {err}"))?;
    let _ = writer.join();
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let text = String::from_utf8(out.stdout).map_err(|_| "perl: the result is not UTF-8")?;
    let (n, rest) = text.split_once('\n').ok_or("perl: no result")?;
    Ok((n.trim().parse().map_err(|_| format!("perl: bad site count {n}"))?, rest.to_string()))
}

/// Check every plant against the tree it will run on.
fn check_all(tree: &Path, plants: &[Plant]) -> Result<(), String> {
    let errors: Vec<String> = plants
        .iter()
        .filter_map(|p| {
            let src = fs::read_to_string(tree.join(&p.path)).map_err(|e| format!("cannot read {}: {e}", p.path));
            src.and_then(|s| mutate(&s, &p.pattern)).err().map(|e| format!("{} ({}): {e}", p.id, p.path))
        })
        .collect();
    if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
}

/// Whether `dir` lies on a RAM file system (tmpfs, or a zram device), from `/proc/mounts`.
fn on_ram(dir: &Path) -> Option<bool> {
    let mounts = fs::read_to_string("/proc/mounts").ok()?;
    let best = mounts
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() >= 3).then(|| (f[0], f[1].replace("\\040", " "), f[2]))
        })
        .filter(|(_, mnt, _)| dir.starts_with(mnt))
        .max_by_key(|(_, mnt, _)| mnt.len())?;
    Some(best.2 == "tmpfs" || best.2 == "ramfs" || best.0.starts_with("/dev/zram"))
}

/// The one build tag of this plants run (PLANTS-BUILD-2): the baseline and every mutant copy build their products with
/// it (`so::build_tag`), so a mutant outside the crates that carry the tag does not rebuild them in every product
/// target directory. Constant for the run, and another for each run (the process id; a run's copy and target
/// directories are new). The products of a plants copy carry it instead of a hash of their sources: the ld.so
/// start-up check compares the two products of one build, which carry the same one.
fn run_build_tag() -> String {
    format!("plants-{}", std::process::id())
}

/// Run `argv` (or `sh -c <cmd>`) in `cwd` under `timeout`, its output into `log`, with `NIJI_LAYOUT_TRACE` set
/// for `layout` (a product build, and a command that builds the products itself, get the plant's layout); the exit code.
fn run_logged(cwd: &Path, argv: &[String], log: &Path, timeout: u64, layout: crate::hotlists::Layout) -> Result<i32, String> {
    let file = fs::File::create(log).map_err(|e| format!("cannot create {}: {e}", log.display()))?;
    let err = file.try_clone().map_err(|e| e.to_string())?;
    // The child leads its own process group (timeout(1) makes the same group), so one group signal reaches every
    // grandchild, such as the gdb and probes a build-time trace starts, however the step ended.
    let mut child = Command::new("timeout")
        .args(["-k", "10"])
        .arg(timeout.to_string())
        .args(argv)
        .current_dir(cwd)
        // A target directory from the environment would build (and test) outside the copy.
        .env_remove("CARGO_TARGET_DIR")
        .env(crate::hotlists::TRACE_ENV, layout_env(layout))
        .env(crate::so::PLANTS_TAG_ENV, run_build_tag())
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(err)
        .process_group(0)
        .spawn()
        .map_err(|e| format!("cannot run {argv:?}: {e}"))?;
    let pgid = child.id();
    let status = child.wait().map_err(|e| format!("cannot wait for {argv:?}: {e}"))?;
    kill_group(pgid);
    Ok(status.code().unwrap_or(128))
}

/// End every process left in group `pgid`: TERM, then KILL when the group is still there after a grace period.
fn kill_group(pgid: u32) {
    let signal = |sig: &str| Command::new("kill").args([sig, "--"]).arg(format!("-{pgid}")).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success());
    if signal("-TERM") {
        std::thread::sleep(std::time::Duration::from_millis(300));
        signal("-KILL");
    }
}

fn sh(cmd: &str) -> Vec<String> {
    vec!["sh".into(), "-c".into(), cmd.into()]
}

fn tail(log: &Path, n: usize) -> String {
    let text = fs::read_to_string(log).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// Whether the log matches the extended regular expression (`grep -E`).
fn log_matches(log: &Path, re: &str) -> bool {
    Command::new("grep").args(["-q", "-E", "-e", re]).arg(log).stdin(Stdio::null()).status().is_ok_and(|s| s.success())
}

/// XTASK-PLANTS-CATCHER: what caught a plant, read from its command's log: `<test binary> <test name>` of the
/// first failing test the log names, so a run's record says which test is the evidence for the plant.
///
/// It reads a `cargo test` log (`Running <src> (<path>/<target>-<hash>)` names the binary, `test <name> ... FAILED`
/// the test, `Doc-tests <crate>` the doctests) and a nextest one (`FAIL [ 0.1s] (1/9) <crate>::<target> <name>`, or
/// any other failing status). Without a failing test line: the first panic of a thread named after a test, then a
/// panic outside a named test (`thread 'main'`, `'<unnamed>'`) with the binary it happened in, then the binary
/// cargo asks to rerun, then a build that does not compile.
pub fn catcher(log: &str) -> String {
    let mut binary: Option<String> = None;
    let mut panic: Option<(Option<String>, String)> = None;
    let mut rerun: Option<String> = None;
    let mut compile = false;
    let named = |binary: &Option<String>, test: &str| match binary {
        Some(b) => format!("{b} {test}"),
        None => format!("{test} (the test binary is not named in the log)"),
    };
    for line in log.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Running ") {
            // `Running \`target/debug/xtask ...\`` is cargo running a program, not a test binary.
            if !rest.starts_with('`') {
                let path = rest.rsplit_once('(').map_or(rest, |(_, p)| p.trim_end_matches(')'));
                let file = path.rsplit('/').next().unwrap_or(path);
                let target = file.rsplit_once('-').filter(|(_, h)| !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit())).map_or(file, |(s, _)| s);
                binary = Some(target.to_string());
            }
        } else if let Some(krate) = t.strip_prefix("Doc-tests ") {
            binary = Some(format!("doctests::{}", krate.trim()));
        } else if let Some(test) = t.strip_prefix("test ").and_then(|r| r.strip_suffix(" ... FAILED")) {
            return named(&binary, test);
        } else if let Some(id) = nextest_failure(t) {
            return id;
        } else if let Some((thread, after)) = t.strip_prefix("thread '").and_then(|r| r.split_once('\'')) {
            if panic.is_none() && after.contains("panicked at") {
                panic = Some((binary.clone(), thread.to_string()));
            }
        } else if let Some(args) = t.strip_prefix("error: test failed, to rerun pass `") {
            rerun = rerun.or_else(|| Some(args.split('`').next().unwrap_or("").to_string()));
        } else if t.starts_with("error: could not compile") {
            compile = true;
        }
    }
    let binary = binary.or(rerun.map(|args| format!("`{args}`")));
    match panic {
        Some((b, thread)) if thread != "main" && thread != "<unnamed>" => named(&b, &thread),
        Some((b, thread)) => format!("{} (a panic outside a named test, thread '{thread}')", b.or(binary).unwrap_or_else(|| "a binary the log does not name".into())),
        None => match binary {
            Some(b) => format!("{b} (the log names no failing test)"),
            None if compile => "the build (the mutant does not compile)".into(),
            None => "no test the log names".into(),
        },
    }
}

/// A nextest line reporting a test that did not pass: `<STATUS> [ <secs>s] [(<n>/<m>)] <binary> <test>`.
fn nextest_failure(t: &str) -> Option<String> {
    let (status, rest) = t.split_once(' ')?;
    if !crate::nextest::failing_status(status) {
        return None;
    }
    let (_, mut rest) = rest.trim_start().strip_prefix('[')?.split_once(']')?;
    rest = rest.trim_start();
    if rest.starts_with('(') {
        rest = rest.split_once(')')?.1.trim_start();
    }
    let id = rest.trim_end();
    (!id.is_empty()).then(|| id.to_string())
}

/// The first line of the results block `results.txt` appends to the plants file.
const RESULTS_HEADER: &str = "# cargo xtask plants results";

/// The plants file with the run's results appended as comments (the parser skips them), replacing the block an
/// earlier run left in it.
fn with_results(text: &str, summary: &str, results: &[String]) -> String {
    let kept = text.find(&format!("\n{RESULTS_HEADER}")).map_or(text, |i| &text[..=i]);
    let kept = if kept.starts_with(RESULTS_HEADER) { "" } else { kept };
    let mut out = kept.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!("{RESULTS_HEADER} ({summary}):\n"));
    for r in results {
        out.push_str(r);
        out.push('\n');
    }
    out
}

/// Write a file anew (a fresh mtime, so cargo rebuilds what depends on it).
fn write_fresh(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn refused(msg: String) -> Outcome {
    eprintln!("plants: REFUSED: {msg}");
    Outcome { code: EXIT_REFUSED, verdicts: Vec::new() }
}

/// The source paths the copy must hold, sorted, and how they were found. In a git work tree (`.git` a directory or a
/// worktree's file): `git ls-files` of the tracked and the untracked-not-ignored files that exist (a tracked file
/// deleted in the work tree is not expected); a tracked file under `/target` is expected too (the copy then lacks it
/// and the run is refused). Otherwise (a synced role: no `.git`) every file and symlink below `src` outside `/target`
/// and `/fuzz/target`, symlinks not followed.
fn source_paths(src: &Path) -> Result<(Vec<String>, &'static str), String> {
    if src.join(".git").exists() {
        let out = Command::new("git").arg("-C").arg(src).args(["ls-files", "-z", "--cached", "--others", "--exclude-standard"]).output().map_err(|e| format!("git ls-files: {e}"))?;
        if !out.status.success() {
            return Err(format!("git ls-files: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        let mut paths: Vec<String> = String::from_utf8_lossy(&out.stdout).split('\0').filter(|p| !p.is_empty() && fs::symlink_metadata(src.join(p)).is_ok()).map(str::to_string).collect();
        paths.sort();
        paths.dedup();
        return Ok((paths, "git ls-files"));
    }
    fn walk(dir: &Path, rel: &str, out: &mut Vec<String>) -> Result<(), String> {
        for e in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let e = e.map_err(|e| e.to_string())?;
            let name = e.file_name().to_string_lossy().into_owned();
            let path = if rel.is_empty() { name } else { format!("{rel}/{name}") };
            if path == "target" || path == "fuzz/target" {
                continue;
            }
            if e.file_type().map_err(|e| e.to_string())?.is_dir() {
                walk(&e.path(), &path, out)?;
            } else {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut paths = Vec::new();
    walk(src, "", &mut paths)?;
    paths.sort();
    Ok((paths, "every file of a tree without .git"))
}

/// The paths of `paths` that `copy` lacks (as a file, symlink or anything else; a dangling symlink counts as present).
fn missing_in(copy: &Path, paths: &[String]) -> Vec<String> {
    paths.iter().filter(|p| fs::symlink_metadata(copy.join(p)).is_err()).cloned().collect()
}

/// The rsync arguments that leave a copy's build directories out of it: the source only.
const COPY_EXCLUDES: [&str; 4] = ["--exclude", "/target", "--exclude", "/fuzz/target"];

/// PLANTS-WARM-COPY: the role's warm copies when `--warm` and `--cold` are not given: `plants-warm/<tree name>` in
/// `--dir`'s grandparent, beside the story directories (`/dev/shm/claude/<story>/plants-<n>` gives
/// `/dev/shm/claude/plants-warm/<role>`). `None` (every copy cold) when that grandparent is `/` or absent.
pub fn default_warm(dir: &Path, src: &Path) -> Option<PathBuf> {
    let grand = dir.parent()?.parent()?;
    grand.parent()?;
    Some(grand.join("plants-warm").join(src.file_name()?))
}

/// PLANTS-WARM-COPY: one warm copy, held by this run: its path and its lock (released when dropped, or when the process
/// ends however it ends).
struct Warm {
    path: PathBuf,
    _lock: fs::File,
}

/// PLANTS-WARM-COPY: take warm copy `k` (from 0) of `root`, `<root>/copy-<k+1>`, by its exclusive `flock`
/// (`<root>/copy-<k+1>.lock`); `None` when another run holds it.
// xtask is host tooling built with the pinned stable toolchain; `File::try_lock` needs 1.89 (as in `so::lock_exclusive`).
#[allow(clippy::incompatible_msrv)]
fn take_warm(root: &Path, k: usize) -> Result<Option<Warm>, String> {
    fs::create_dir_all(root).map_err(|e| format!("{}: {e}", root.display()))?;
    let lock = root.join(format!("copy-{}.lock", k + 1));
    let file = fs::OpenOptions::new().create(true).truncate(false).write(true).open(&lock).map_err(|e| format!("{}: {e}", lock.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(Warm { path: root.join(format!("copy-{}", k + 1)), _lock: file })),
        Err(fs::TryLockError::WouldBlock) => Ok(None),
        Err(fs::TryLockError::Error(e)) => Err(format!("{}: {e}", lock.display())),
    }
}

/// PLANTS-WARM-COPY: cargo's own entries of a profile directory (`target/debug`, `target/product/release`, ...), the
/// ones a warm copy keeps. The uplifted binaries and libraries beside them go: cargo links them again for every unit
/// a build uses, fresh or rebuilt, so a command finds one only when its build ran in this run.
const CARGO_PROFILE_ENTRIES: &[&str] = &[".fingerprint", "build", "deps", "examples", "incremental"];

/// PLANTS-WARM-COPY: prune the cargo target directory `dir` of a warm copy (`target`, `fuzz/target`) to cargo's build
/// cache, so the copy holds what a cold copy holds plus what cargo itself judges fresh. Kept: in a target directory
/// its `CACHEDIR.TAG`, its profile directories (holding `.fingerprint`) with [`CARGO_PROFILE_ENTRIES`] only, and its
/// nested target directories (holding `CACHEDIR.TAG` or a profile directory, as `target/product`), pruned the same
/// way. Everything else goes: the products (`target/niji`), test scratch (`target/tmp`), test caches and logs, and
/// what a test or a killed run left. The number of entries removed.
pub fn prune_to_cargo_cache(dir: &Path) -> Result<usize, String> {
    let is_profile = |d: &Path| d.join(".fingerprint").is_dir();
    let mut removed = 0;
    for e in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let e = e.map_err(|e| e.to_string())?;
        let (path, name) = (e.path(), e.file_name());
        let is_dir = e.file_type().map_err(|e| e.to_string())?.is_dir();
        if !is_dir && name == "CACHEDIR.TAG" {
            continue;
        }
        if is_dir && is_profile(&path) {
            for p in fs::read_dir(&path).map_err(|e| format!("{}: {e}", path.display()))? {
                let p = p.map_err(|e| e.to_string())?;
                let keep = p.file_type().map_err(|e| e.to_string())?.is_dir() && CARGO_PROFILE_ENTRIES.iter().any(|k| p.file_name() == *k);
                if !keep {
                    remove_all(&p.path())?;
                    removed += 1;
                }
            }
            continue;
        }
        if is_dir && holds_cargo(&path, 2) {
            removed += prune_to_cargo_cache(&path)?;
        } else {
            remove_all(&path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// Whether the directory `d` is a cargo target directory: it holds `CACHEDIR.TAG` or a profile directory (holding
/// `.fingerprint`), or, `depth` levels down, a directory that does (`target/product` holding only `ld-pie/release`).
fn holds_cargo(d: &Path, depth: usize) -> bool {
    d.join("CACHEDIR.TAG").is_file()
        || fs::read_dir(d).into_iter().flatten().flatten().any(|c| c.file_type().is_ok_and(|t| t.is_dir()) && (c.path().join(".fingerprint").is_dir() || (depth > 0 && holds_cargo(&c.path(), depth - 1))))
}

/// Remove `path` (a file, a symlink, or a directory with everything in it, symlinks not followed), first making
/// every directory below it writable and searchable when a test left one that is not.
fn remove_all(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt as _;
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_dir() {
        return fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()));
    }
    if fs::remove_dir_all(path).is_ok() {
        return Ok(());
    }
    fn open_up(d: &Path) {
        let _ = fs::set_permissions(d, fs::Permissions::from_mode(0o700));
        for e in fs::read_dir(d).into_iter().flatten().flatten() {
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                open_up(&e.path());
            }
        }
    }
    open_up(path);
    fs::remove_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// What a warm sync did.
pub struct Synced {
    /// Files written (changed or new in the tree), each with a fresh mtime.
    pub written: usize,
    /// Paths removed from the copy because the tree has none.
    pub deleted: usize,
    /// Entries removed from the copy's target directories ([`prune_to_cargo_cache`]).
    pub pruned: usize,
}

/// PLANTS-WARM-COPY: bring the warm copy `copy` to the tree `src`. Its target directories are pruned to cargo's cache;
/// then rsync compares every file by content (`-c`) and keeps no times (no `-t`): a file that differs, such as a
/// plant's mutant a killed run left, is written again with a fresh mtime, newer than every build cargo made from the
/// old text, and a path the tree lacks is deleted. A second rsync pass that only compares (`-n`) must then find
/// nothing to change, else the copy is not used.
pub fn sync_warm(src: &Path, copy: &Path) -> Result<Synced, String> {
    let mut pruned = 0;
    for t in ["target", "fuzz/target"] {
        let t = copy.join(t);
        if fs::symlink_metadata(&t).is_ok_and(|m| m.is_dir()) {
            pruned += prune_to_cargo_cache(&t)?;
        }
    }
    let rsync = |check: bool| -> Result<String, String> {
        let mut cmd = Command::new("rsync");
        cmd.args(["-rlpDc", "--delete", "--itemize-changes"]).args(COPY_EXCLUDES);
        if check {
            cmd.arg("--dry-run");
        }
        let out = cmd.arg(format!("{}/", src.display())).arg(copy).stdin(Stdio::null()).output().map_err(|e| format!("rsync: {e}"))?;
        if !out.status.success() {
            return Err(format!("rsync{} failed ({}): {}", if check { " --dry-run" } else { "" }, out.status, String::from_utf8_lossy(&out.stderr).trim()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let changes = rsync(false)?;
    let written = changes.lines().filter(|l| l.starts_with(">f")).count();
    let deleted = changes.lines().filter(|l| l.starts_with("*deleting")).count();
    let left = rsync(true)?;
    if !left.trim().is_empty() {
        let shown: Vec<&str> = left.lines().take(10).collect();
        return Err(format!("after the sync the copy still differs from the tree: {}", shown.join("; ")));
    }
    Ok(Synced { written, deleted, pruned })
}

/// PLANTS-COPIES: the peak RAM one plants copy needs, in bytes: its tree and target directories on the RAM file system plus
/// the build and the tests. Measured per copy on the current head (docs/plants.md, "Copies"); the larger host's figure,
/// rounded up.
pub const COPY_PEAK_BYTES: u64 = 14 << 30;

/// PLANTS-WARM-COPY: below this many CPUs a slot runs two copies only for a file of more than [`SPLIT_PLANTS`] plants:
/// the copies share the CPUs, so each copy's builds slow down and the split must win it back over enough plants.
pub const SPLIT_CPUS: usize = 8;

/// PLANTS-WARM-COPY: in a slot of fewer than [`SPLIT_CPUS`] CPUs, the plant count past which two copies pay. Cold, on
/// a 4-CPU slot, the split paid only past about 6 plants (one copy's baseline 10.2 min, two copies' 14.5 min each,
/// handoff/PIPELINE-TEST-TIME-1127-report.md); warm, on 4 CPUs, a 6-plant file took 1221 s in two copies against
/// 1433 s in one (handoff/PLANTS-WARM-COPY.md), so two copies from 6 plants on.
pub const SPLIT_PLANTS: usize = 5;

/// PLANTS-COPIES: how many copies `--copies auto` (the default) runs, given the RAM this job may use (`None`: unknown),
/// the CPUs it may use and the plants in the file: two when that RAM holds two copies at [`COPY_PEAK_BYTES`] each, the
/// file has two plants or more, and (PLANTS-WARM-COPY) the slot has [`SPLIT_CPUS`] CPUs or the file more than
/// [`SPLIT_PLANTS`] plants; else one (a serial run).
pub fn auto_copies(limit: Option<u64>, cpus: usize, plants: usize) -> usize {
    match limit {
        Some(l) if l >= 2 * COPY_PEAK_BYTES && plants >= 2 && (cpus >= SPLIT_CPUS || plants > SPLIT_PLANTS) => 2,
        _ => 1,
    }
}

/// The RAM this job may use, in bytes: the smallest of `NIJI_PLANTS_RAM_GB` (GiB; a role's RAM budget, given by whoever
/// starts the job), the `memory.max` of this process's cgroup and every cgroup above it, and `MemAvailable`. `None` when
/// none of them is known.
fn ram_limit() -> Option<u64> {
    let mut maxes = Vec::new();
    if let Some(cg) = fs::read_to_string("/proc/self/cgroup").ok().and_then(|t| t.lines().find_map(|l| l.strip_prefix("0::").map(str::to_string))) {
        let mut dir = PathBuf::from(format!("/sys/fs/cgroup{}", cg.trim()));
        loop {
            maxes.extend(fs::read_to_string(dir.join("memory.max")));
            if dir == Path::new("/sys/fs/cgroup") || !dir.pop() {
                break;
            }
        }
    }
    smallest_limit(std::env::var("NIJI_PLANTS_RAM_GB").ok().as_deref(), &maxes, &fs::read_to_string("/proc/meminfo").unwrap_or_default())
}

/// [`ram_limit`] from what it read: the `NIJI_PLANTS_RAM_GB` value (GiB), the cgroups' `memory.max` texts (`max` = no
/// limit) and `/proc/meminfo`. The smallest limit given, in bytes; an unreadable value gives none.
pub fn smallest_limit(env_gib: Option<&str>, memory_max: &[String], meminfo: &str) -> Option<u64> {
    let env = env_gib.and_then(|v| v.trim().parse::<f64>().ok()).filter(|g| g.is_finite() && *g >= 0.0).map(|g| (g * (1u64 << 30) as f64) as u64);
    let cgroups = memory_max.iter().filter_map(|v| v.trim().parse::<u64>().ok());
    let available = meminfo.lines().find_map(|l| l.strip_prefix("MemAvailable:").and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok())).map(|kb| kb * 1024);
    env.into_iter().chain(cgroups).chain(available).min()
}

/// What one copy's worker ended with.
struct CopyRun {
    /// `Some(code)`: the copy stopped early (refused, baseline failed, or a step that leaves the copy unusable), with the
    /// exit status a serial run would have stopped with.
    stop: Option<u8>,
    /// The copy stopped because another copy did (its own state says nothing).
    aborted: bool,
    /// Per plant judged (its index in the file): the verdict, if one was reached, and its line in `results.txt`.
    plants: Vec<(usize, Option<Verdict>, String)>,
    /// An error was recorded for a plant (the run goes on, but exits 1).
    failed: bool,
}

/// The whole run: refuse what cannot be trusted, then one worker per copy (PLANTS-COPIES; one copy is the serial run),
/// each copying the tree, building, running the baselines of its plants, then each of its plants; the verdicts merged in
/// the plants file's order.
pub fn run(cfg: &Config) -> Outcome {
    if cfg.dir.exists() {
        return refused(format!("{} exists: a plants run needs a NEW directory (no reuse, no delete)", cfg.dir.display()));
    }
    if cfg.dir.starts_with(&cfg.src) {
        return refused(format!("{} is inside the tree it copies", cfg.dir.display()));
    }
    if let Some(warm) = &cfg.warm {
        if warm.starts_with(&cfg.src) || cfg.src.starts_with(warm) || cfg.dir.starts_with(warm) || warm.starts_with(&cfg.dir) {
            return refused(format!("the warm copies' directory {} overlaps the tree {} or --dir {} (--warm <dir> elsewhere, or --cold)", warm.display(), cfg.src.display(), cfg.dir.display()));
        }
    }
    if let Err(e) = check_all(&cfg.src, &cfg.plants) {
        return refused(e);
    }
    if on_ram(&cfg.dir) == Some(false) {
        println!("plants: WARN-NOT-RAM: {} is not on tmpfs or zram (plant copies belong in RAM: /dev/shm/claude/<story>/ or /mnt/zram/claude/<story>/)", cfg.dir.display());
    }
    if cfg.dir.starts_with("/dev/shm") {
        println!("plants: WARN-DEV-SHM: {} is under /dev/shm: a test that mounts its own /dev or /dev/shm cannot see the copy (posixipc_products; nscache_mode.sh binds it back)", cfg.dir.display());
    }
    if let Some(parent) = cfg.dir.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return refused(format!("cannot create {}: {e}", parent.display()));
        }
    }
    // `create_dir` (not `_all`) fails if the directory appeared meanwhile: still no reuse.
    if let Err(e) = fs::create_dir(&cfg.dir) {
        return refused(format!("cannot create {}: {e}", cfg.dir.display()));
    }
    // PLANTS-PRODUCTS-ONCE: what each plant needs from the products, judged on the source tree (each copy is a copy of
    // it), decides which copies build them.
    let traced: Vec<bool> = cfg.plants.iter().map(|p| p.layout || targets_layout(&p.cmd)).collect();
    let needs = needs(&cfg.plants, &(cfg.uses_products)(&cfg.src, &cfg.plants), &(cfg.reads_products)(&cfg.src, &cfg.plants), &traced);
    let shares = assign(&needs, cfg.copies);
    let builders = shares.iter().filter(|s| s.iter().any(|&i| needs[i].reads)).count();
    println!("plants: products: {} of {} plants read them, {} rebuilt with their mutant; built in {builders} of {} copies", needs.iter().filter(|n| n.reads).count(), needs.len(), needs.iter().filter(|n| n.mutant).count(), shares.len());
    let abort = std::sync::atomic::AtomicBool::new(false);
    let runs: Vec<CopyRun> = if shares.len() == 1 {
        vec![run_copy(cfg, 0, &shares[0], &needs, &abort)]
    } else {
        let sizes: Vec<String> = shares.iter().map(|s| s.len().to_string()).collect();
        println!("plants: {} copies, running at the same time (plants each: {}; the plants file's order is kept in the results)", shares.len(), sizes.join(", "));
        std::thread::scope(|s| {
            let handles: Vec<_> = shares.iter().enumerate().map(|(k, share)| {
                let (abort, needs) = (&abort, &needs);
                s.spawn(move || run_copy(cfg, k, share, needs, abort))
            }).collect();
            handles.into_iter().map(|h| h.join().unwrap_or(CopyRun { stop: Some(EXIT_SURVIVED), aborted: false, plants: Vec::new(), failed: true })).collect()
        })
    };
    merge(cfg, shares.len(), runs)
}

/// The run's outcome from its copies' (PLANTS-COPIES): verdicts and result lines in the plants file's order. A copy that
/// stopped decides the exit status as a serial run's stop would (refused 3, then baseline 2, then 1), and, as there, no
/// summary or `results.txt` follows.
fn merge(cfg: &Config, copies: usize, runs: Vec<CopyRun>) -> Outcome {
    let mut judged: Vec<(usize, Option<Verdict>, String)> = runs.iter().flat_map(|r| r.plants.iter().cloned()).collect();
    judged.sort_by_key(|(i, _, _)| *i);
    let verdicts: Vec<(String, Verdict)> = judged.iter().filter_map(|(i, v, _)| v.map(|v| (cfg.plants[*i].id.clone(), v))).collect();
    let stops: Vec<u8> = runs.iter().filter(|r| !r.aborted).filter_map(|r| r.stop).collect();
    if !stops.is_empty() {
        let code = [EXIT_REFUSED, EXIT_BASELINE].into_iter().find(|c| stops.contains(c)).unwrap_or(EXIT_SURVIVED);
        if copies > 1 {
            println!("plants: stopped (a copy stopped; see its lines above), exit {code}");
        }
        return Outcome { code, verdicts };
    }
    let failed = runs.iter().any(|r| r.failed || r.aborted);
    if copies > 1 {
        // The copies' lines interleave: the verdicts again, in the file's order.
        for (i, v, _) in &judged {
            println!("plants: {}: {}", cfg.plants[*i].id, v.map_or("ERROR", Verdict::label));
        }
    }
    let results: Vec<String> = judged.into_iter().map(|(_, _, line)| line).collect();
    let count = |v: Verdict| verdicts.iter().filter(|(_, x)| *x == v).count();
    let survived = count(Verdict::Survived);
    let summary = format!(
        "{} plants: {} CAUGHT, {} SURVIVED, {} EQUIVALENT, {} WARN-RC-ONLY, {} WARN-NO-MSG, {} BUILD-TIMEOUT{}",
        cfg.plants.len(),
        count(Verdict::Caught),
        survived,
        count(Verdict::Equivalent),
        count(Verdict::WarnRcOnly),
        count(Verdict::WarnNoMsg),
        count(Verdict::BuildTimeout),
        if failed { ", and errors (see above)" } else { "" }
    );
    println!("plants: {summary}");
    let record = cfg.dir.join("results.txt");
    let mut failed = failed;
    if let Err(e) = write_fresh(&record, &with_results(&cfg.text, &summary, &results)) {
        println!("plants: ERROR: cannot write {}: {e}", record.display());
        failed = true;
    } else {
        println!("plants: the plants file with these results: {} (commit it as handoff/<STORY>-plants.txt)", record.display());
    }
    let code = if survived > 0 || count(Verdict::BuildTimeout) > 0 || failed { EXIT_SURVIVED } else { EXIT_OK };
    Outcome { code, verdicts }
}

/// One copy (`k` from 0): `<dir>/src` and `<dir>/logs` for the first, `<dir>/src-<k+1>` and `<dir>/logs-<k+1>` for the
/// others; copy the tree, build, run the baseline of each distinct command of its plants, then each of its plants
/// (`share`, indices into the plants file). It stops before its next step when `abort` is set (another copy stopped).
fn run_copy(cfg: &Config, k: usize, share: &[usize], needs: &[Need], abort: &std::sync::atomic::AtomicBool) -> CopyRun {
    use std::sync::atomic::Ordering;
    let (cold, logs) = if k == 0 { (cfg.dir.join("src"), cfg.dir.join("logs")) } else { (cfg.dir.join(format!("src-{}", k + 1)), cfg.dir.join(format!("logs-{}", k + 1))) };
    // The first copy's lines read as a serial run's; another copy's say which copy.
    let pre = if k == 0 { "plants:".to_string() } else { format!("plants[copy {}]:", k + 1) };
    let mut out = CopyRun { stop: None, aborted: false, plants: Vec::new(), failed: false };
    let stop = |mut out: CopyRun, code: u8| {
        abort.store(true, Ordering::SeqCst);
        out.stop = Some(code);
        out
    };
    let step = |what: &str, r: Result<(), String>| r.map_err(|e| eprintln!("{pre} {what} failed: {e}"));
    if step("mkdir", fs::create_dir(&logs).map_err(|e| e.to_string())).is_err() {
        return stop(out, EXIT_SURVIVED);
    }
    // PLANTS-WARM-COPY: this copy's warm copy when it is free and syncs, held (locked) until the copy's work ends;
    // otherwise a cold copy in `--dir`, as without one.
    let mut warm: Option<Warm> = None;
    if let Some(root) = &cfg.warm {
        match take_warm(root, k) {
            Ok(Some(w)) => {
                let start = Instant::now();
                match sync_warm(&cfg.src, &w.path) {
                    Ok(s) => {
                        println!("{pre} warm copy {} -> {} (synced by content in {:.1}s: {} files written with a fresh mtime, {} paths deleted; target directories pruned to cargo's cache, {} entries removed)", cfg.src.display(), w.path.display(), start.elapsed().as_secs_f64(), s.written, s.deleted, s.pruned);
                        warm = Some(w);
                    }
                    Err(e) => println!("{pre} warm copy {} not used ({e}); a cold copy instead", w.path.display()),
                }
            }
            Ok(None) => println!("{pre} warm copy {}/copy-{} is busy (another run holds its lock); a cold copy instead", root.display(), k + 1),
            Err(e) => println!("{pre} warm copy in {} not used ({e}); a cold copy instead", root.display()),
        }
    }
    let copy = warm.as_ref().map_or_else(|| cold.clone(), |w| w.path.clone());
    if warm.is_none() {
        // The source only: `target/` holds deliberately unreadable test files and stale builds.
        let mut rsync = Command::new("rsync");
        rsync.arg("-a").args(COPY_EXCLUDES).arg(format!("{}/", cfg.src.display())).arg(&copy);
        if step("copy", crate::run(&mut rsync)).is_err() {
            return stop(out, EXIT_SURVIVED);
        }
        println!("{pre} copy {} -> {}", cfg.src.display(), copy.display());
    }
    match source_paths(&cfg.src) {
        Ok((paths, how)) => {
            let missing = missing_in(&copy, &paths);
            if !missing.is_empty() {
                let shown: Vec<&str> = missing.iter().take(20).map(String::as_str).collect();
                eprintln!("{pre} REFUSED: the copy lacks {} of {} source paths ({how}), so a test could miss a file it needs: {}{}", missing.len(), paths.len(), shown.join(", "), if missing.len() > 20 { ", ..." } else { "" });
                return stop(out, EXIT_REFUSED);
            }
            println!("{pre} copy check: all {} source paths ({how}) are in the copy", paths.len());
        }
        Err(e) => {
            eprintln!("{pre} REFUSED: cannot list the source paths of {}: {e}", cfg.src.display());
            return stop(out, EXIT_REFUSED);
        }
    }
    let plants: Vec<&Plant> = share.iter().map(|&i| &cfg.plants[i]).collect();
    // PLANTS-UNGROUPED-MUTANTS: a plant's products are built ungrouped (a compile only: a mutant's behaviour does not
    // depend on the start-up layout) unless the plant measures the layout ([`targets_layout`] or `layout:traced`).
    use crate::hotlists::Layout;
    // This copy's plants, owned: the products check and a failed baseline's message take a slice of them.
    let owned: Vec<Plant> = plants.iter().map(|p| (*p).clone()).collect();
    let layouts: Vec<Layout> = plants.iter().map(|p| if p.layout || targets_layout(&p.cmd) { Layout::Traced } else { Layout::Ungrouped }).collect();
    // The products, then each distinct command on the unmutated copy, once per layout the plants use (ungrouped
    // first): each command's baseline sees the products in the layout its plants are built with.
    let mut baseline: Vec<(String, Vec<String>, Layout)> = Vec::new();
    let mut n = 0;
    for layout in [Layout::Ungrouped, Layout::Traced] {
        if !layouts.contains(&layout) {
            continue;
        }
        let so = if layout == Layout::Traced && layouts.contains(&Layout::Ungrouped) { "so-traced" } else { "so" };
        // PLANTS-PRODUCTS-ONCE: built only when a plant of this layout here reads them.
        if share.iter().zip(&layouts).any(|(&i, l)| *l == layout && needs[i].reads) {
            baseline.push((so.into(), cfg.prepare.clone(), layout));
        }
        let start = baseline.len();
        for (p, _) in plants.iter().zip(&layouts).filter(|(_, l)| **l == layout) {
            if !baseline[start..].iter().any(|(_, argv, _)| argv[2] == p.cmd) {
                n += 1;
                baseline.push((format!("baseline-{n}"), sh(&p.cmd), layout));
            }
        }
    }
    // The layout the products in the copy hold (the last build's).
    let mut held = Layout::Ungrouped;
    for (name, argv, layout) in &baseline {
        if abort.load(Ordering::SeqCst) {
            out.aborted = true;
            return out;
        }
        let log = logs.join(format!("{name}.log"));
        // A product build has its own limit.
        let build = name.starts_with("so");
        let limit = if build { cfg.prepare_timeout } else { cfg.timeout };
        let start = Instant::now();
        match run_logged(&copy, argv, &log, limit, *layout) {
            Ok(0) if build => {
                held = *layout;
                println!("{pre} {name} ok ({}; layout {}, NIJI_LAYOUT_TRACE={}, in {:.0}s)", argv.join(" "), layout_name(*layout), layout_env(*layout), start.elapsed().as_secs_f64());
            }
            Ok(0) => println!("{pre} {name} ok ({}; layout {})", argv.join(" "), layout_name(*layout)),
            Ok(TIMED_OUT) if build => {
                println!("{pre} BASELINE FAILED: {name} timed out after {limit} s (prepare timeout; --prepare-timeout <secs>); no plant was applied. Last lines of {}:\n{}", log.display(), tail(&log, 20));
                return stop(out, EXIT_BASELINE);
            }
            Ok(rc) => {
                let hint = if *layout == Layout::Ungrouped && !build && rc != TIMED_OUT { " (built ungrouped; a command that measures the start-up layout needs `layout:traced` on its plants, docs/plants.md)" } else { "" };
                // The plants whose command this baseline is: a failed baseline names them, not only `baseline-N`.
                let of = baseline_plants(&owned, &layouts, *layout, argv, build);
                let how = ended(rc, limit, if build { "--prepare-timeout" } else { "--timeout" });
                println!("{pre} BASELINE FAILED: {name} {how} ({}; layout {}; the command of plant(s) {}){hint}; no plant was applied. Last lines of {}:\n{}", argv.join(" "), layout_name(*layout), of.join(", "), log.display(), tail(&log, 20));
                return stop(out, EXIT_BASELINE);
            }
            Err(e) => {
                println!("{pre} BASELINE FAILED: {name}: {e}");
                return stop(out, EXIT_BASELINE);
            }
        }
    }
    // XTASK-PLANTS-PRODUCTS-REBUILD: a command that uses the products gets them rebuilt with the
    // mutant; the others reuse the baseline build, rebuilt unmutated first if a plant left a mutant in it.
    // PLANTS-PRODUCTS-ONCE: a plant that may change the products and whose command uses them gets them rebuilt with its
    // mutant ([`Need::mutant`]); one that only reads them runs on the unmutated build in its layout (rebuilt so first
    // when a plant left its mutant or another layout in it); one that reads none needs nothing.
    let mut products_hold_mutant = false;
    for ((&i, p), &layout) in share.iter().zip(&plants).zip(&layouts) {
        if abort.load(Ordering::SeqCst) {
            out.aborted = true;
            return out;
        }
        let need = needs[i];
        let products = need.mutant;
        let mut build = if need.reads { format!("the baseline products (layout {}), not rebuilt", layout_name(held)) } else { "none (its command reads no products)".to_string() };
        // A command that reads no products needs none here, as in a copy that builds none ([`assign`]).
        let restore = need.reads && !products && (products_hold_mutant || held != layout);
        let restore_layout = layout;
        if restore {
            let log = logs.join(format!("{}.so-restore.log", p.id));
            let start = Instant::now();
            match run_logged(&copy, &cfg.prepare, &log, cfg.prepare_timeout, restore_layout) {
                Ok(0) => {
                    products_hold_mutant = false;
                    held = restore_layout;
                    build = format!("the products, rebuilt unmutated after the last plant, layout {}, in {:.0}s (log {})", layout_name(restore_layout), start.elapsed().as_secs_f64(), log.display());
                }
                rc => {
                    let how = match rc { Ok(rc) => ended(rc, cfg.prepare_timeout, "--prepare-timeout"), Err(e) => e };
                    println!("{pre} {}: ERROR: rebuilding the unmutated products ({}): {how}; stopping (log {})", p.id, cfg.prepare.join(" "), log.display());
                    return stop(out, EXIT_SURVIVED);
                }
            }
        }
        let error = |out: &mut CopyRun, e: String| {
            println!("{pre} {}: ERROR: {e}", p.id);
            out.plants.push((i, None, format!("# {}: ERROR: {e}", p.id)));
            out.failed = true;
        };
        let file = copy.join(&p.path);
        let original = match fs::read_to_string(&file) {
            Ok(s) => s,
            Err(e) => {
                error(&mut out, format!("cannot read {}: {e}", p.path));
                continue;
            }
        };
        let mutant = match mutate(&original, &p.pattern) {
            Ok(m) => m,
            Err(e) => {
                error(&mut out, e);
                continue;
            }
        };
        if let Err(e) = write_fresh(&file, &mutant) {
            error(&mut out, e);
            continue;
        }
        let mut log = logs.join(format!("{}.log", p.id));
        let mut built = Ok(0);
        if products {
            products_hold_mutant = true;
            let so_log = logs.join(format!("{}.so.log", p.id));
            let start = Instant::now();
            built = run_logged(&copy, &cfg.prepare, &so_log, cfg.prepare_timeout, layout);
            held = layout;
            build = format!("the products, rebuilt with the mutant, layout {}, in {:.0}s (log {})", layout_name(layout), start.elapsed().as_secs_f64(), so_log.display());
            if !matches!(built, Ok(0)) {
                // The mutant does not build: judged on the build's log, the command is not run.
                build = format!("the products build failed on the mutant: {build}");
                log = so_log;
            }
        }
        let build_timed_out = products && matches!(built, Ok(TIMED_OUT));
        let rc = if matches!(built, Ok(0)) { run_logged(&copy, &sh(&p.cmd), &log, cfg.timeout, layout) } else { built };
        // Restore by writing (a fresh mtime) before anything else can go wrong.
        if let Err(e) = write_fresh(&file, &original) {
            println!("{pre} {}: ERROR: restore: {e}; stopping (the copy holds the mutant)", p.id);
            return stop(out, EXIT_SURVIVED);
        }
        let rc = match rc {
            Ok(rc) => rc,
            Err(e) => {
                error(&mut out, e);
                continue;
            }
        };
        let (verdict, detail) = match (&p.expect, rc) {
            _ if build_timed_out => (Verdict::BuildTimeout, format!("the products build timed out after {} s (prepare timeout); the mutant was not tested", cfg.prepare_timeout)),
            (Expect::Equivalent(why), 0) => (Verdict::Equivalent, why.clone()),
            (_, 0) => (Verdict::Survived, format!("`{}` passed on the mutant", p.cmd)),
            (_, TIMED_OUT) => (Verdict::Caught, format!("caught by timeout: `{}` timed out after {} s (--timeout <secs>)", p.cmd, cfg.timeout)),
            (Expect::Msg(re), rc) if log_matches(&log, re) => (Verdict::Caught, format!("rc {rc}, /{re}/ in the log")),
            (Expect::Msg(re), rc) => (Verdict::WarnNoMsg, format!("rc {rc}, but /{re}/ is not in {}", log.display())),
            (Expect::Equivalent(_), rc) => (Verdict::Caught, format!("rc {rc} (declared equivalent, but caught)")),
            (Expect::RcOnly, rc) => (Verdict::WarnRcOnly, format!("rc {rc}, no msg: given")),
        };
        println!("{pre} {}: {} ({}; log {}; build: {build})", p.id, verdict.label(), detail, log.display());
        // XTASK-PLANTS-CATCHER: a failed command names what failed, kept beside the plants in results.txt.
        let line = if rc == 0 || build_timed_out {
            format!("# {}: {}", p.id, verdict.label())
        } else {
            let by = if rc == TIMED_OUT { format!("timeout: `{}` timed out after {} s", p.cmd, cfg.timeout) } else { catcher(&String::from_utf8_lossy(&fs::read(&log).unwrap_or_default())) };
            println!("{pre} {}: caught by {by}", p.id);
            format!("# {}: {}, caught by {by}", p.id, verdict.label())
        };
        out.plants.push((i, Some(verdict), line));
    }
    out
}

#[cfg(test)]
mod tests {
    // XTASK-PLANTS: no spec clause (developer tooling): the plants runner on a toy tree.
    use super::*;

    /// A toy tree in a fresh temporary directory, and a `--dir` beside it that does not exist yet.
    fn toy(name: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("xtask-plants-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let src = base.join("tree");
        fs::create_dir_all(src.join("src")).unwrap();
        fs::write(src.join("src/value.txt"), "answer=42\nother=7\nother=7\n").unwrap();
        fs::create_dir_all(src.join("target")).unwrap();
        fs::write(src.join("target/stale"), "not copied").unwrap();
        (src, base.join("run"))
    }

    const CHECK: &str = "grep -q answer=42 src/value.txt || { echo WRONG-ANSWER; exit 1; }";

    fn config(src: &Path, dir: &Path, plants: &str) -> Config {
        Config { src: src.to_path_buf(), text: plants.to_string(), plants: parse_plants(plants).unwrap(), dir: dir.to_path_buf(), prepare: sh("true"), timeout: 60, prepare_timeout: 60, uses_products: |_, ps| vec![false; ps.len()], reads_products: |_, ps| vec![false; ps.len()], copies: 1, warm: None }
    }

    /// The toy's products: `product.txt`, a copy of `src/value.txt`; every build appends a line to `builds.txt`.
    const BUILD: &str = "cp src/value.txt product.txt && echo built >> builds.txt";
    const PRODUCT_CHECK: &str = "grep -q answer=42 product.txt || { echo WRONG-PRODUCT; exit 1; }";

    // no spec clause (XTASK-PLANTS-PRODUCTS-REBUILD): a plant seen only through the products is CAUGHT (the baseline
    // products alone would let it survive); a command that does not use them reuses the build, which is first rebuilt
    // unmutated when the plant before left its mutant in it and the command reads them (PLANTS-PRODUCTS-ONCE: one that
    // reads none is not rebuilt for)
    #[test]
    fn a_products_command_gets_the_products_rebuilt_with_the_mutant() {
        let (src, dir) = toy("products");
        let plants = format!(
            "u1 | src/value.txt | perl:s/(answer=42\\n)other=7/$1other=8/ | {CHECK} | equivalent:other= is not read\n\
             pr | src/value.txt | lit:answer=42 ==> answer=43 | {PRODUCT_CHECK} | msg:WRONG-PRODUCT\n\
             u3 | src/value.txt | perl:s/answer=42/answer=46/ | {CHECK} | msg:WRONG-ANSWER\n\
             u2 | src/value.txt | perl:s/answer=42/answer=44/ | {CHECK} | msg:WRONG-ANSWER\n"
        );
        let mut cfg = config(&src, &dir, &plants);
        cfg.prepare = sh(BUILD);
        cfg.uses_products = |_, ps| ps.iter().map(|p| p.cmd.contains("product.txt")).collect();
        // `u2`'s command reads the products (as a unit test may) without using its plant's.
        cfg.reads_products = |_, ps| ps.iter().map(|p| p.id == "u2").collect();
        let out = run(&cfg);
        assert_eq!(out.verdicts, vec![("u1".into(), Verdict::Equivalent), ("pr".into(), Verdict::Caught), ("u3".into(), Verdict::Caught), ("u2".into(), Verdict::Caught)]);
        assert_eq!(out.code, EXIT_OK);
        // The baseline, `pr` with its mutant, the unmutated rebuild before `u2`; none for `u1` or `u3`.
        assert_eq!(fs::read_to_string(dir.join("src/builds.txt")).unwrap().lines().count(), 3);
        assert!(dir.join("logs/pr.so.log").is_file() && dir.join("logs/u2.so-restore.log").is_file());
        assert!(!dir.join("logs/u1.so.log").exists() && !dir.join("logs/u1.so-restore.log").exists());
        assert!(!dir.join("logs/u3.so.log").exists() && !dir.join("logs/u3.so-restore.log").exists());
        assert_eq!(fs::read_to_string(dir.join("src/product.txt")).unwrap(), "answer=42\nother=7\nother=7\n");
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause (PLANTS-UNGROUPED-MUTANTS): an ordinary plant's products (and its baseline) are built ungrouped, a
    // plant that measures the layout (marked `layout:traced`, or naming a layout test) is built traced, and each command
    // sees its plant's layout in NIJI_LAYOUT_TRACE
    #[test]
    fn ordinary_plants_build_ungrouped_and_layout_plants_traced() {
        let (src, dir) = toy("layout");
        let product = "grep -q answer=42 product.txt || { echo WRONG-PRODUCT; exit 1; }";
        let plants = format!(
            "or | src/value.txt | lit:answer=42 ==> answer=43 | echo cmd-or=$NIJI_LAYOUT_TRACE >> cmds.txt; {product} | msg:WRONG-PRODUCT\n\
             mk | src/value.txt | perl:s/answer=42/answer=44/ | echo cmd-mk=$NIJI_LAYOUT_TRACE >> cmds.txt; {product} | msg:WRONG-PRODUCT | layout:traced\n\
             nm | src/value.txt | lit:answer=42 ==> answer=45 | echo hot_text_products cmd-nm=$NIJI_LAYOUT_TRACE >> cmds.txt; {product} | msg:WRONG-PRODUCT\n"
        );
        let mut cfg = config(&src, &dir, &plants);
        cfg.prepare = sh("cp src/value.txt product.txt && echo \"$(head -1 src/value.txt) layout=$NIJI_LAYOUT_TRACE\" >> builds.txt");
        // Only the ordinary plant's command is known to read the products: the layout plants rebuild regardless.
        cfg.uses_products = |_, ps| ps.iter().map(|p| p.id == "or").collect();
        let out = run(&cfg);
        assert_eq!(out.verdicts, vec![("or".into(), Verdict::Caught), ("mk".into(), Verdict::Caught), ("nm".into(), Verdict::Caught)]);
        assert_eq!(out.code, EXIT_OK);
        // Baseline ungrouped, then traced; then each plant's mutant in its own layout.
        let builds = fs::read_to_string(dir.join("src/builds.txt")).unwrap();
        assert_eq!(builds, "answer=42 layout=ungrouped\nanswer=42 layout=1\nanswer=43 layout=ungrouped\nanswer=44 layout=1\nanswer=45 layout=1\n");
        assert!(dir.join("logs/so.log").is_file() && dir.join("logs/so-traced.log").is_file());
        // The baseline of each command, then the plant's run, each in the plant's layout.
        let cmds = fs::read_to_string(dir.join("src/cmds.txt")).unwrap();
        assert_eq!(cmds, "cmd-or=ungrouped\ncmd-mk=1\nhot_text_products cmd-nm=1\ncmd-or=ungrouped\ncmd-mk=1\nhot_text_products cmd-nm=1\n");
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause (PLANTS-UNGROUPED-MUTANTS): a file without layout plants never builds traced
    #[test]
    fn a_file_without_layout_plants_builds_ungrouped_only() {
        let (src, dir) = toy("ungrouped");
        let plants = format!("pr | src/value.txt | lit:answer=42 ==> answer=43 | {PRODUCT_CHECK} | msg:WRONG-PRODUCT\n");
        let mut cfg = config(&src, &dir, &plants);
        cfg.prepare = sh("cp src/value.txt product.txt && echo layout=$NIJI_LAYOUT_TRACE >> builds.txt");
        cfg.uses_products = |_, ps| vec![true; ps.len()];
        let out = run(&cfg);
        assert_eq!(out.verdicts, vec![("pr".into(), Verdict::Caught)]);
        assert_eq!(fs::read_to_string(dir.join("src/builds.txt")).unwrap(), "layout=ungrouped\nlayout=ungrouped\n");
        assert!(!dir.join("logs/so-traced.log").exists());
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause (PLANTS-BUILD-2): the baseline build and every mutant's build of one run get ONE build tag (the
    // variable `so::build_tag` reads), a plants-run tag and not an empty one, so a mutant outside the crates that
    // carry the tag does not rebuild them; and the command that tests a mutant sees the same tag
    #[test]
    fn every_build_of_a_run_gets_one_build_tag() {
        let (src, dir) = toy("tag");
        let plants = format!(
            "pr | src/value.txt | lit:answer=42 ==> answer=43 | echo cmd=${} >> cmds.txt; {PRODUCT_CHECK} | msg:WRONG-PRODUCT\n\
             p2 | src/value.txt | lit:answer=42 ==> answer=44 | {PRODUCT_CHECK} | msg:WRONG-PRODUCT\n",
            crate::so::PLANTS_TAG_ENV
        );
        let mut cfg = config(&src, &dir, &plants);
        cfg.prepare = sh(&format!("cp src/value.txt product.txt && echo ${} >> builds.txt", crate::so::PLANTS_TAG_ENV));
        cfg.uses_products = |_, ps| vec![true; ps.len()];
        let out = run(&cfg);
        assert_eq!(out.verdicts, vec![("pr".into(), Verdict::Caught), ("p2".into(), Verdict::Caught)]);
        let builds = fs::read_to_string(dir.join("src/builds.txt")).unwrap();
        let tags: std::collections::BTreeSet<&str> = builds.lines().collect();
        assert!(builds.lines().count() >= 3, "the baseline and each mutant built: {builds:?}");
        assert_eq!(tags.len(), 1, "one tag for every build of the run: {builds:?}");
        // This process's tag, not whatever the environment brought: when this test itself runs under a plants run, a
        // child inherits that run's tag, which would hide a `run_logged` that sets none.
        assert_eq!(tags, std::collections::BTreeSet::from([run_build_tag().as_str()]), "{builds:?}");
        assert!(run_build_tag().starts_with("plants-") && run_build_tag().len() > "plants-".len());
        let cmds = fs::read_to_string(dir.join("src/cmds.txt")).unwrap();
        assert!(cmds.lines().all(|l| l == format!("cmd={}", tags.iter().next().unwrap())), "{cmds:?}");
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause (PLANTS-UNGROUPED-MUTANTS): the commands that measure the start-up layout
    #[test]
    fn which_commands_target_the_layout() {
        for cmd in [
            "cargo test -p niji --test hot_text_products",
            "cargo test -p niji --test=heap_meta_products",
            "cargo test -p niji --test cold_bss_products startup",
            "cargo xtask layout-check",
            "cargo xtask affected --only test:niji:hot_text_products --run",
            "cargo xtask affected --files csys/src/x.c --run",
        ] {
            assert!(targets_layout(cmd), "{cmd}");
        }
        for cmd in [
            "cargo test -p niji --test stdio_short_io",
            "cargo xtask affected --only conformance:s27_dladdr --with-exports --run",
            "cargo xtask affected --files csys/src/x.c --list",
            "cargo test -p xtask --bin xtask affected::tests --run",
            "cargo test -p xtask --bin xtask layout::tests",
            "cargo test -p niji --test hot_text_products_helper",
        ] {
            assert!(!targets_layout(cmd), "{cmd}");
        }
    }

    // no spec clause (XTASK-PLANTS-PRODUCTS-REBUILD): a mutant whose products do not build is judged on the build's log
    #[test]
    fn a_mutant_that_breaks_the_products_build_is_judged_on_its_log() {
        let (src, dir) = toy("nobuild");
        let plants = format!("nb | src/value.txt | lit:answer=42 ==> answer=99 | {PRODUCT_CHECK} | msg:BUILD-BROKE\n");
        let mut cfg = config(&src, &dir, &plants);
        cfg.prepare = sh("grep -q answer=99 src/value.txt && { echo BUILD-BROKE; exit 1; }; cp src/value.txt product.txt");
        cfg.uses_products = |_, ps| vec![true; ps.len()];
        let out = run(&cfg);
        assert_eq!(out.verdicts, vec![("nb".into(), Verdict::Caught)]);
        assert!(!dir.join("logs/nb.log").exists(), "the command must not run on a mutant that does not build");
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause (XTASK-PLANTS-PRODUCTS-REBUILD): only a plain `cargo test` of named targets none of which uses the
    // products skips the rebuild
    #[test]
    fn which_commands_use_the_products() {
        let lookup = |k: Option<&str>, t: &str| match (k, t) {
            (Some("niji") | None, "catalogs_products") => Some(true),
            (Some("niji") | None, "stdio_short_io") => Some(false),
            _ => None,
        };
        for (cmd, want) in [
            ("cargo test -p niji --test stdio_short_io", false),
            ("cargo test -p niji --test=stdio_short_io short_write -- --test-threads 1", false),
            ("cargo test -p xtask --bin xtask plants::", false),
            ("cargo test -p niji --lib diag", false),
            ("cargo test -p niji --test catalogs_products", true),
            ("cargo test -p niji --test stdio_short_io --test catalogs_products", true),
            ("cargo test -p niji --test unknown_target", true),
            ("cargo test -p niji", true),
            ("cargo test -p niji --tests", true),
            ("cargo test -p niji --test stdio_short_io && true", true),
            ("cargo xtask affected --only conformance:printf --with-exports --run", true),
            ("sh tests/oracle/run.sh", true),
        ] {
            assert_eq!(cmd_uses_products(cmd, lookup), want, "{cmd}");
        }
    }

    // no spec clause: an xtask test target is judged by the products text rule (Repo leaves xtask out); one naming
    // the products, a support directory, another package or no file at all stays unsure (rebuild)
    #[test]
    fn xtask_test_targets_are_judged_by_their_text() {
        let copy = std::env::temp_dir().join(format!("plants-xtask-tests.{}", std::process::id()));
        let _ = fs::remove_dir_all(&copy);
        let tests = copy.join("xtask/tests");
        fs::create_dir_all(tests.join("helpers")).unwrap();
        fs::write(tests.join("plain.rs"), "fn main() { run(env!(\"CARGO_BIN_EXE_xtask\")); }\n").unwrap();
        fs::write(tests.join("linked.rs"), "let a = root.join(\"target/niji/lib\");\n").unwrap();
        fs::write(tests.join("helped.rs"), "mod helpers;\n").unwrap();
        let got = |k: Option<&str>, n: &str| xtask_test_uses_products(&copy, k, n);
        assert_eq!(got(Some("xtask"), "plain"), Some(false));
        assert_eq!(got(Some("xtask"), "linked"), Some(true));
        assert_eq!(got(Some("xtask"), "helped"), None);
        assert_eq!(got(Some("xtask"), "absent"), None);
        assert_eq!(got(Some("xtask"), "../plain"), None);
        assert_eq!(got(None, "plain"), None);
        assert_eq!(got(Some("niji"), "plain"), None);
        let _ = fs::remove_dir_all(&copy);
    }

    // no spec clause: a plant its command catches is CAUGHT, a declared one with no msg is WARN-RC-ONLY, the copy has no target/
    #[test]
    fn a_caught_plant_exits_zero_and_is_restored() {
        let (src, dir) = toy("caught");
        let plants = format!("p1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:WRONG-ANSWER\np2 | src/value.txt | perl:s/answer=4\\d/answer=0/ | {CHECK}\n");
        let out = run(&config(&src, &dir, &plants));
        assert_eq!(out.verdicts, vec![("p1".to_string(), Verdict::Caught), ("p2".to_string(), Verdict::WarnRcOnly)]);
        assert_eq!(out.code, EXIT_OK);
        assert_eq!(fs::read_to_string(dir.join("src/src/value.txt")).unwrap(), "answer=42\nother=7\nother=7\n");
        assert!(!dir.join("src/target").exists(), "target/ must not be copied");
        assert!(dir.join("logs/p1.log").is_file());
        let results = fs::read_to_string(dir.join("results.txt")).unwrap();
        assert!(results.starts_with(&plants), "results.txt keeps the plants file: {results}");
        assert!(results.contains("\n# p1: CAUGHT, caught by no test the log names\n# p2: WARN-RC-ONLY, caught by "), "{results}");
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause (PLANTS-WARM-COPY): a run in the role's warm copy judges as a cold run does, and the next run brings
    // the copy back to the tree by content: a mutant a killed run left (its mtime older than the build made from it) is
    // written again with a fresh mtime, an unchanged file keeps its mtime, a path the tree lacks goes, and target/ keeps
    // cargo's cache only (no products, test scratch or uplifted binaries), so no run tests a left mutant
    #[test]
    fn a_warm_copy_is_synced_by_content_and_never_tests_a_left_mutant() {
        let (src, dir) = toy("warm");
        fs::write(src.join("src/keep.txt"), "kept\n").unwrap();
        fs::create_dir_all(src.join("fuzz")).unwrap();
        fs::write(src.join("fuzz/Cargo.toml"), "").unwrap();
        let base = src.parent().unwrap().to_path_buf();
        let plants = format!("p1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:WRONG-ANSWER\n");
        let warm_cfg = |dir: &Path| Config { warm: Some(base.join("warm")), ..config(&src, dir, &plants) };
        let out = run(&warm_cfg(&dir));
        assert_eq!((out.verdicts, out.code), (vec![("p1".to_string(), Verdict::Caught)], EXIT_OK));
        let copy = base.join("warm/copy-1");
        assert!(!dir.join("src").exists() && copy.join("src/keep.txt").is_file(), "the warm copy is used, not a cold one");
        // What a killed run leaves behind.
        let left = copy.join("src/value.txt");
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        fs::write(&left, "answer=43\nother=7\nother=7\n").unwrap();
        for f in [&left, &copy.join("src/keep.txt")] {
            fs::File::options().write(true).open(f).unwrap().set_modified(old).unwrap();
        }
        fs::write(copy.join("stray.txt"), "x").unwrap();
        for d in ["target/niji", "target/tmp/t", "target/debug/.fingerprint/a", "target/debug/deps", "target/debug/niji-deps", "target/product/ld-pie/release/.fingerprint", "fuzz/target/x"] {
            fs::create_dir_all(copy.join(d)).unwrap();
        }
        for (f, text) in [("target/niji/libc.a", "stale"), ("target/debug/xtask", "uplifted"), ("target/debug/deps/kept", "cache"), ("target/CACHEDIR.TAG", "tag"), ("target/needed.sh", "log")] {
            fs::write(copy.join(f), text).unwrap();
        }
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(copy.join("target/tmp/t"), fs::Permissions::from_mode(0o000)).unwrap();
        }
        let before = std::time::SystemTime::now();
        let s = sync_warm(&src, &copy).unwrap();
        assert_eq!((s.written, s.deleted), (1, 1), "only the left mutant rewritten, only stray.txt deleted");
        assert_eq!(fs::read_to_string(&left).unwrap(), "answer=42\nother=7\nother=7\n");
        assert!(fs::metadata(&left).unwrap().modified().unwrap() >= before, "the rewritten file has a fresh mtime");
        assert_eq!(fs::metadata(copy.join("src/keep.txt")).unwrap().modified().unwrap(), old, "an unchanged file is not rewritten");
        for gone in ["stray.txt", "target/niji", "target/tmp", "target/needed.sh", "target/debug/niji-deps", "target/debug/xtask", "fuzz/target/x"] {
            assert!(fs::symlink_metadata(copy.join(gone)).is_err(), "{gone} kept in the warm copy");
        }
        for kept in ["target/CACHEDIR.TAG", "target/debug/deps/kept", "target/debug/.fingerprint/a", "target/product/ld-pie/release/.fingerprint"] {
            assert!(copy.join(kept).exists(), "{kept} (cargo's cache) removed");
        }
        // The mutant again, and a full run: its baseline would fail on it.
        fs::write(&left, "answer=43\nother=7\nother=7\n").unwrap();
        let out = run(&warm_cfg(&base.join("run-2")));
        assert_eq!((out.verdicts, out.code), (vec![("p1".to_string(), Verdict::Caught)], EXIT_OK));
        assert_eq!(fs::read_to_string(&left).unwrap(), "answer=42\nother=7\nother=7\n");
        // A copy the sync cannot bring to the tree is not used: the tree loses fuzz/, which the copy keeps around its
        // protected fuzz/target.
        fs::remove_dir_all(src.join("fuzz")).unwrap();
        let e = sync_warm(&src, &copy).err().expect("a copy that still differs from the tree is refused");
        assert!(e.contains("still differs") || e.contains("rsync failed"), "{e}");
        let _ = fs::remove_dir_all(&base);
    }

    // no spec clause (PLANTS-WARM-COPY): a warm copy another run holds is never shared: this run copies cold into --dir,
    // and the warm copy is free again once its holder ends
    #[test]
    fn a_busy_warm_copy_gives_a_cold_copy() {
        let (src, dir) = toy("warmbusy");
        let warm = src.parent().unwrap().join("warm");
        let held = take_warm(&warm, 0).unwrap().expect("a new warm copy is free");
        let plants = format!("p1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:WRONG-ANSWER\n");
        let out = run(&Config { warm: Some(warm.clone()), ..config(&src, &dir, &plants) });
        assert_eq!((out.verdicts, out.code), (vec![("p1".to_string(), Verdict::Caught)], EXIT_OK));
        assert!(dir.join("src/src/value.txt").is_file() && !warm.join("copy-1").exists(), "a cold copy, the busy warm copy untouched");
        drop(held);
        assert!(take_warm(&warm, 0).unwrap().is_some(), "the lock ends with its holder");
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause (XTASK-PLANTS-CATCHER): one failing test in a recorded `cargo test` log names its binary and test
    #[test]
    fn catcher_names_the_failing_test_of_a_cargo_test_log() {
        let log = "   Compiling niji-ld v0.1.0\n     Running `target/debug/xtask affected --run`\n     Running unittests src/lib.rs (target/debug/deps/niji_ld-0f3a9c2b1d4e5f60)\n\nrunning 3 tests\ntest elf::ok ... ok\n\n     Running tests/elf_tables.rs (target/debug/deps/elf_tables-a1b2c3d4e5f60718)\n\nrunning 2 tests\ntest tables_read_back ... ok\ntest sysv_chain_loop ... FAILED\n\nfailures:\n\n---- sysv_chain_loop stdout ----\nthread 'sysv_chain_loop' panicked at crates/ld/tests/elf_tables.rs:9:5:\nloop not found\n\nerror: test failed, to rerun pass `-p niji-ld --test elf_tables`\n";
        assert_eq!(catcher(log), "elf_tables sysv_chain_loop");
        assert_eq!(catcher("     Running unittests src/lib.rs (target/debug/deps/niji_ld-0f3a9c2b1d4e5f60)\ntest elf::tests::bad ... FAILED\n"), "niji_ld elf::tests::bad");
        assert_eq!(catcher("   Doc-tests niji_ld\ntest crates/ld/src/elf.rs - Elf (line 3) ... FAILED\n"), "doctests::niji_ld crates/ld/src/elf.rs - Elf (line 3)");
    }

    // no spec clause (XTASK-PLANTS-CATCHER): with several failing tests the first one in the log is the catcher
    #[test]
    fn catcher_names_the_first_of_several_failing_tests() {
        let cargo = "     Running tests/stdio.rs (target/debug/deps/stdio-00112233aabbccdd)\ntest b_second ... ok\ntest c_third ... FAILED\ntest a_first ... FAILED\n";
        assert_eq!(catcher(cargo), "stdio c_third");
        let nextest = "    Starting 9 tests across 2 binaries\n        PASS [   0.010s] (1/9) niji-core::stdio ok_one\n        FAIL [   0.004s] (2/9) niji-core::stdio short_write\n     SIGSEGV [   0.020s] (3/9) niji-core::stdio crash\n------------\n     Summary [   0.100s] 9 tests run: 7 passed, 2 failed\n        FAIL [   0.004s] (2/9) niji-core::stdio short_write\n";
        assert_eq!(catcher(nextest), "niji-core::stdio short_write");
        assert_eq!(catcher("     SIGABRT [   0.020s] niji-core crash_test\n        FAIL [   0.004s] niji-core other\n"), "niji-core crash_test");
    }

    // no spec clause (XTASK-PLANTS-CATCHER): a panic with no test name, a binary that aborted, a build that fails, nothing
    #[test]
    fn catcher_without_a_failing_test_line() {
        let main = "     Running tests/posixipc_run.rs (target/debug/deps/posixipc_run-0123456789abcdef)\nthread 'main' panicked at crates/core/tests/posixipc_run.rs:40:9:\nphase nodev failed\nerror: test failed, to rerun pass `-p niji-core --test posixipc_run`\n";
        assert_eq!(catcher(main), "posixipc_run (a panic outside a named test, thread 'main')");
        assert_eq!(catcher("thread '<unnamed>' panicked at src/x.rs:1:1:\nboom\n"), "a binary the log does not name (a panic outside a named test, thread '<unnamed>')");
        let named = "     Running tests/t.rs (target/debug/deps/t-0123456789abcdef)\nthread 'mmap_guard' panicked at tests/t.rs:3:3:\nbad\n";
        assert_eq!(catcher(named), "t mmap_guard");
        let aborted = "     Running tests/t.rs (target/debug/deps/t-0123456789abcdef)\nrunning 1 test\nerror: test failed, to rerun pass `--test t`\n\nCaused by:\n  process didn't exit successfully (signal: 6, SIGABRT)\n";
        assert_eq!(catcher(aborted), "t (the log names no failing test)");
        assert_eq!(catcher("error: test failed, to rerun pass `--test t`\n"), "`--test t` (the log names no failing test)");
        assert_eq!(catcher("error[E0308]: mismatched types\nerror: could not compile `niji-ld` (lib) due to 1 previous error\n"), "the build (the mutant does not compile)");
        assert_eq!(catcher("WRONG-ANSWER\n"), "no test the log names");
    }

    // no spec clause (XTASK-PLANTS-CATCHER): results.txt replaces the block an earlier run appended
    #[test]
    fn results_replace_an_earlier_block() {
        let first = with_results("# story\np | a | lit:x ==> y | true\n", "1 plants: 1 CAUGHT", &["# p: CAUGHT, caught by t x".into()]);
        assert_eq!(first, "# story\np | a | lit:x ==> y | true\n# cargo xtask plants results (1 plants: 1 CAUGHT):\n# p: CAUGHT, caught by t x\n");
        assert_eq!(with_results(&first, "1 plants: 1 SURVIVED", &["# p: SURVIVED".into()]), "# story\np | a | lit:x ==> y | true\n# cargo xtask plants results (1 plants: 1 SURVIVED):\n# p: SURVIVED\n");
        assert_eq!(parse_plants(&first).unwrap().len(), 1);
        assert_eq!(with_results("p | a | lit:x ==> y | true", "s", &[]), "p | a | lit:x ==> y | true\n# cargo xtask plants results (s):\n");
    }

    // no spec clause: a surviving plant exits non-zero; a wrong msg is WARN-NO-MSG; an equivalent one is EQUIVALENT
    #[test]
    fn a_surviving_plant_exits_non_zero() {
        let (src, dir) = toy("survived");
        let plants = format!("s1 | src/value.txt | perl:s/(answer=42\\n)other=7/$1other=8/ | {CHECK} | msg:WRONG-ANSWER\ns2 | src/value.txt | lit:answer=42 ==> answer=41 | {CHECK} | msg:NOT-PRINTED\ns3 | src/value.txt | perl:s/(answer=42\\n)other=7/$1other=9/ | {CHECK} | equivalent:other= is not read\n");
        let out = run(&config(&src, &dir, &plants));
        assert_eq!(out.verdicts, vec![("s1".into(), Verdict::Survived), ("s2".into(), Verdict::WarnNoMsg), ("s3".into(), Verdict::Equivalent)]);
        assert_eq!(out.code, EXIT_SURVIVED);
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause: a plant matching two sites is refused before anything is copied
    #[test]
    fn a_plant_matching_two_sites_is_refused() {
        let (src, dir) = toy("twosites");
        for pattern in ["lit:other=7 ==> other=8", "perl:s/other=7/other=8/"] {
            let out = run(&config(&src, &dir, &format!("t1 | src/value.txt | {pattern} | {CHECK} | msg:X\n")));
            assert_eq!(out.code, EXIT_REFUSED, "{pattern}");
            assert!(!dir.exists(), "a refused run creates nothing");
        }
        assert!(mutate("a\na\n", &Pattern::Perl("s/a/b/".into())).unwrap_err().contains("2 sites"));
        assert!(mutate("a\n", &Pattern::Perl("s/z/b/".into())).unwrap_err().contains("no site"));
        assert!(parse_plants("t | x.txt | perl:s/a/b/g | true | msg:X").unwrap_err().contains("/g"));
        assert!(parse_plants("t | csys/src/machine/x.S | perl:s/ret/nop/ | true | msg:X").unwrap_err().contains("niji_machine_"));
        assert!(parse_plants("t | csys/src/machine/x.S | perl:s/(niji_machine_toyfn:.*?)ret/$1nop/s | true | msg:X").is_ok());
        assert!(parse_plants("t | ../x.txt | lit:a ==> b | true | msg:X").is_err());
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause: an existing --dir is refused and left untouched
    #[test]
    fn a_reused_dir_is_refused() {
        let (src, dir) = toy("reused");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("keep"), "x").unwrap();
        let out = run(&config(&src, &dir, &format!("r1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:WRONG-ANSWER\n")));
        assert_eq!(out.code, EXIT_REFUSED);
        assert!(out.verdicts.is_empty());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "nothing written into a reused dir");
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause: a failing baseline (products or command) exits 2 before any plant is applied
    #[test]
    fn a_failed_baseline_exits_two() {
        let (src, dir) = toy("baseline");
        let plants = "b1 | src/value.txt | lit:answer=42 ==> answer=43 | grep -q answer=99 src/value.txt | msg:X\n";
        let out = run(&config(&src, &dir, plants));
        assert_eq!(out.code, EXIT_BASELINE);
        assert!(out.verdicts.is_empty());
        assert_eq!(fs::read_to_string(dir.join("src/src/value.txt")).unwrap(), "answer=42\nother=7\nother=7\n");
        let dir2 = dir.with_file_name("run2");
        let mut cfg = config(&src, &dir2, &format!("b1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:X\n"));
        cfg.prepare = sh("echo the products are missing; exit 1");
        // A plant whose command reads the products: the copy builds them first.
        cfg.reads_products = |_, ps| vec![true; ps.len()];
        assert_eq!(run(&cfg).code, EXIT_BASELINE);
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause: the plants file format. The labels here are made up: naming a real machine export in xtask makes
    // `affected` count this file as a user of that `.S` file and escalate its changes to FULL.
    #[test]
    fn plants_lines_parse() {
        let p = parse_plants("# comment\n\nx-1 | a/b.rs | lit:a || b ==> a && b | cargo test -p q --test t | msg:t1.*FAILED\ny | a/b.rs | perl:s{a}{b} | true | equivalent: same result\n").unwrap();
        assert_eq!(p[0].pattern, Pattern::Literal { old: "a || b".into(), new: "a && b".into() });
        assert_eq!(p[0].expect, Expect::Msg("t1.*FAILED".into()));
        assert_eq!(p[1].expect, Expect::Equivalent("same result".into()));
        assert!(parse_plants("x | a | lit:a ==> b | true | msg:X\nx | a | lit:a ==> c | true | msg:X").unwrap_err().contains("duplicate"));
        assert!(parse_plants("x | a | a ==> b | true | msg:X").is_err());
        assert_eq!(parse_plants("d | a | lit:drop me ==> | true").unwrap()[0].pattern, Pattern::Literal { old: "drop me".into(), new: String::new() });
        assert!(names_machine_anchor("(niji_machine_toyfn:.*?)x") && !names_machine_anchor("niji_machine_:") && !names_machine_anchor("niji_machine_x"));
        // PLANTS-UNGROUPED-MUTANTS: `layout:traced` after the expectation (or alone after the command).
        let l = parse_plants("a | f | lit:x ==> y | c1 | c2 | msg:BAD | layout:traced\nb | f | lit:x ==> y | c3 | layout:traced\nc | f | lit:x ==> y | c4 | msg:Z\n").unwrap();
        assert_eq!((l[0].cmd.as_str(), &l[0].expect, l[0].layout), ("c1 | c2", &Expect::Msg("BAD".into()), true));
        assert_eq!((l[1].cmd.as_str(), &l[1].expect, l[1].layout), ("c3", &Expect::RcOnly, true));
        assert!(!l[2].layout);
        assert!(parse_plants("a | f | lit:x ==> y | c | msg:X | layout:yes").unwrap_err().contains("layout:traced"));
    }

    // no spec clause: the default per-plant limit is 600 s (the project rule), and --timeout overrides it
    #[test]
    fn default_timeout_is_600_seconds() {
        assert_eq!(DEFAULT_PREPARE_TIMEOUT, 3600);
        let (src, dir) = toy("defaulttimeout");
        let file = src.join("plants.txt");
        fs::write(&file, format!("d1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:WRONG-ANSWER\n")).unwrap();
        let (f, d) = (file.display().to_string(), dir.display().to_string());
        let cfg = parse_args(&src, &[f.clone(), "--dir".into(), d.clone()]).unwrap();
        assert_eq!(cfg.timeout, 600);
        assert_eq!(DEFAULT_TIMEOUT, 600);
        let cfg = parse_args(&src, &[f, "--dir".into(), d, "--timeout".into(), "7".into()]).unwrap();
        assert_eq!(cfg.timeout, 7);
        assert_eq!(cfg.prepare_timeout, 3600);
        let cfg = parse_args(&src, &[file.display().to_string(), "--dir".into(), dir.display().to_string(), "--prepare-timeout".into(), "9".into()]).unwrap();
        assert_eq!((cfg.timeout, cfg.prepare_timeout), (600, 9));
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    fn git(dir: &Path, args: &[&str]) -> bool {
        Command::new("git").arg("-C").arg(dir).args(args).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
    }

    // XTASK-PLANTS-TRACKED-COPY, no spec clause: every source path of a git work tree reaches the copy, including
    // unusual kinds (a symlink, a dangling symlink, an executable script, a dot directory, a name with a space and a
    // newline, a `target` directory below the top), and the copy check lists them all; without git, every file but /target.
    #[test]
    fn every_tracked_path_of_an_unusual_kind_is_copied() {
        let (src, dir) = toy("tracked");
        if !git(&src, &["init", "-q"]) {
            println!("skipped: no git (the tracked-path case needs `git init`)");
            return;
        }
        fs::create_dir_all(src.join(".config")).unwrap();
        fs::write(src.join(".config/x.toml"), "x = 1\n").unwrap();
        fs::create_dir_all(src.join("crates/x/target")).unwrap();
        fs::write(src.join("crates/x/target/keep.txt"), "source, not a build\n").unwrap();
        fs::write(src.join("src/a b\nc.txt"), "odd name\n").unwrap();
        fs::write(src.join("src/run.sh"), "#!/bin/sh\nexit 0\n").unwrap();
        let mut mode = fs::metadata(src.join("src/run.sh")).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
        fs::set_permissions(src.join("src/run.sh"), mode).unwrap();
        std::os::unix::fs::symlink("value.txt", src.join("src/link")).unwrap();
        std::os::unix::fs::symlink("nowhere", src.join("src/dangling")).unwrap();
        fs::write(src.join(".gitignore"), "/target\n").unwrap();
        assert!(git(&src, &["add", "-A"]), "git add");
        fs::write(src.join("src/untracked.txt"), "new, not added yet\n").unwrap();
        let (paths, how) = source_paths(&src).unwrap();
        assert_eq!(how, "git ls-files");
        for p in [".config/x.toml", "crates/x/target/keep.txt", "src/a b\nc.txt", "src/run.sh", "src/link", "src/dangling", "src/untracked.txt"] {
            assert!(paths.iter().any(|x| x == p), "{p} listed: {paths:?}");
        }
        assert!(!paths.iter().any(|x| x.starts_with("target/")), "ignored target/ is not a source path");
        let out = run(&config(&src, &dir, &format!("c1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} && test -x src/run.sh && test -L src/dangling | msg:WRONG-ANSWER\n")));
        assert_eq!(out.code, EXIT_OK, "{:?}", out.verdicts);
        assert!(missing_in(&dir.join("src"), &paths).is_empty());
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // XTASK-PLANTS-TRACKED-COPY, no spec clause: a tracked file the copy lacks (here one force-added under the excluded
    // /target) refuses the run before the build or any plant, naming the path
    #[test]
    fn a_tracked_path_missing_from_the_copy_refuses_the_run() {
        let (src, dir) = toy("missing");
        if !git(&src, &["init", "-q"]) {
            println!("skipped: no git (the tracked-path case needs `git init`)");
            return;
        }
        fs::write(src.join("target/needed.sh"), "echo a test needs me\n").unwrap();
        fs::write(src.join(".gitignore"), "/target\n").unwrap();
        assert!(git(&src, &["add", "src/value.txt", ".gitignore"]) && git(&src, &["add", "-f", "target/needed.sh"]), "git add");
        let mut cfg = config(&src, &dir, &format!("m1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:WRONG-ANSWER\n"));
        cfg.prepare = sh("echo BUILT > built.txt");
        let out = run(&cfg);
        assert_eq!(out.code, EXIT_REFUSED);
        assert!(out.verdicts.is_empty() && !dir.join("src/built.txt").exists(), "refused before the build");
        assert_eq!(missing_in(&dir.join("src"), &source_paths(&src).unwrap().0), vec!["target/needed.sh".to_string()]);
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause: the build step has its own limit; a slow build is not cut by the test timeout
    #[test]
    fn slow_prepare_is_not_cut_by_the_test_timeout() {
        let (src, dir) = toy("slowprep");
        let mut cfg = config(&src, &dir, "q1 | src/value.txt | lit:answer=42 ==> answer=43 | grep -q answer=42 src/value.txt || { echo WRONG-ANSWER; exit 1; } | msg:WRONG-ANSWER\n");
        cfg.prepare = sh("sleep 2");
        cfg.timeout = 1;
        cfg.prepare_timeout = 30;
        let out = run(&cfg);
        assert_eq!(out.verdicts, vec![("q1".to_string(), Verdict::Caught)]);
        assert_eq!(out.code, EXIT_OK);
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause (TOOLING-PAPERCUTS): a timeout is reported as one with its limit and flag, never a bare rc 124,
    // and a failed baseline names the plants whose command it ran (one command shared by two plants, another not)
    #[test]
    fn a_timeout_names_its_limit_and_the_baseline_names_its_plants() {
        assert_eq!(ended(TIMED_OUT, 600, "--timeout"), "timed out after 600 s (--timeout <secs>)");
        assert_eq!(ended(3, 600, "--timeout"), "rc 3");
        let ps = parse_plants("a | f | lit:x ==> y | sleep 9 | msg:z\nb | f | lit:x ==> w | sleep 9 | msg:z\nc | f | lit:x ==> v | true | msg:z\n").unwrap();
        use crate::hotlists::Layout::{Traced, Ungrouped};
        let ls = [Ungrouped, Ungrouped, Ungrouped];
        assert_eq!(baseline_plants(&ps, &ls, Ungrouped, &sh("sleep 9"), false), vec!["a", "b"]);
        assert_eq!(baseline_plants(&ps, &ls, Ungrouped, &sh("cargo xtask so"), true), vec!["a", "b", "c"]);
        assert!(baseline_plants(&ps, &ls, Traced, &sh("sleep 9"), false).is_empty());
    }

    // no spec clause: a baseline build that runs into the prepare timeout is named as such
    #[test]
    fn baseline_prepare_timeout_is_reported() {
        let (src, dir) = toy("prepbase");
        let mut cfg = config(&src, &dir, "q1 | src/value.txt | lit:answer=42 ==> answer=43 | true | msg:x\n");
        cfg.prepare = sh("sleep 30");
        cfg.prepare_timeout = 1;
        cfg.reads_products = |_, ps| vec![true; ps.len()];
        let out = run(&cfg);
        assert_eq!(out.code, EXIT_BASELINE);
        assert!(out.verdicts.is_empty());
        assert!(dir.join("logs/so.log").exists());
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause: a mutant whose build times out is BUILD-TIMEOUT, never CAUGHT, and fails the run
    #[test]
    fn a_plant_build_that_times_out_is_not_caught() {
        let (src, dir) = toy("prepplant");
        let mut cfg = config(&src, &dir, "q1 | src/value.txt | lit:answer=42 ==> answer=43 | true | msg:x\n");
        cfg.prepare = sh("if grep -q answer=43 src/value.txt; then sleep 30; fi");
        cfg.prepare_timeout = 1;
        cfg.uses_products = |_, ps| vec![true; ps.len()];
        let out = run(&cfg);
        assert_eq!(out.verdicts, vec![("q1".to_string(), Verdict::BuildTimeout)]);
        assert_ne!(out.code, EXIT_OK);
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause: a plant whose command hangs is CAUGHT by the timeout, even with a msg: its log cannot match
    #[test]
    fn a_plant_that_times_out_is_caught() {
        let (src, dir) = toy("timeout");
        let mut cfg = config(&src, &dir, "h1 | src/value.txt | lit:answer=42 ==> answer=43 | if grep -q answer=43 src/value.txt; then sleep 30; fi | msg:WRONG-ANSWER\n");
        cfg.timeout = 1;
        let out = run(&cfg);
        assert_eq!(out.verdicts, vec![("h1".to_string(), Verdict::Caught)]);
        assert_eq!(out.code, EXIT_OK);
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause: a step that ends at its limit leaves no process of its group alive (a grandchild that ignores TERM, as the step's own leader does not)
    #[test]
    fn a_timed_out_step_leaves_no_process_of_its_group() {
        let (src, dir) = toy("killtree");
        fs::create_dir_all(&dir).unwrap();
        let pidfile = dir.join("grandchild.pid");
        let log = dir.join("step.log");
        let cmd = format!("(trap '' TERM; exec sleep 300) & echo $! > {}; wait", pidfile.display());
        let rc = run_logged(&dir, &sh(&cmd), &log, 1, crate::hotlists::Layout::Ungrouped).unwrap();
        assert_eq!(rc, TIMED_OUT);
        let pid = fs::read_to_string(&pidfile).unwrap().trim().to_string();
        let alive = |pid: &str| fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|st| !st.rsplit(')').next().unwrap_or("").trim_start().starts_with('Z'));
        // Reaping by init may lag the kill: poll a bounded number of times.
        for _ in 0..500 {
            if !alive(&pid) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!alive(&pid), "grandchild {pid} survived the step's timeout");
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    const MUT: Need = Need { reads: true, mutant: true };
    const READ: Need = Need { reads: true, mutant: false };
    const NONE: Need = Need { reads: false, mutant: false };

    // no spec clause (PLANTS-COPIES, PLANTS-PRODUCTS-ONCE): plants that rebuild the products are dealt to the copies in
    // turn, every plant once, no copy without a plant
    #[test]
    fn plants_are_dealt_to_the_copies_in_turn() {
        assert_eq!(assign(&[MUT; 5], 2), vec![vec![0, 2, 4], vec![1, 3]]);
        assert_eq!(assign(&[MUT; 3], 1), vec![vec![0, 1, 2]]);
        assert_eq!(assign(&[MUT; 2], 4), vec![vec![0], vec![1]]);
        assert_eq!(assign(&[NONE; 5], 2), vec![vec![0, 2, 4], vec![1, 3]]);
        assert_eq!(assign(&[], 2), vec![Vec::<usize>::new()]);
    }

    // no spec clause (PLANTS-PRODUCTS-ONCE): the products are built in as few copies as the plants allow: one per plant
    // that rebuilds them (up to the copies), one for plants that only read them, none for plants that read none; a
    // plant that only reads joins a building copy, one that reads none fills a copy that builds nothing
    #[test]
    fn the_products_are_built_in_as_few_copies_as_possible() {
        let building = |needs: &[Need], shares: &[Vec<usize>]| shares.iter().filter(|s| s.iter().any(|&i| needs[i].reads)).count();
        let cases: &[(&[Need], usize, usize)] = &[
            (&[NONE, NONE, NONE, NONE], 2, 0),
            (&[READ, NONE, READ, NONE], 2, 1),
            (&[NONE, MUT, NONE, READ, NONE], 2, 1),
            (&[MUT, READ, MUT, NONE], 2, 2),
            (&[MUT, MUT, MUT, READ, NONE], 2, 2),
            (&[READ, READ, READ], 3, 1),
            (&[MUT, READ, READ, READ], 1, 1),
        ];
        for &(needs, copies, builders) in cases {
            let shares = assign(needs, copies);
            let mut all: Vec<usize> = shares.iter().flatten().copied().collect();
            all.sort_unstable();
            assert_eq!(all, (0..needs.len()).collect::<Vec<_>>(), "every plant once: {needs:?} {shares:?}");
            assert!(shares.len() <= copies && shares.iter().all(|s| !s.is_empty() && s.windows(2).all(|w| w[0] < w[1])), "{needs:?} {shares:?}");
            assert_eq!(building(needs, &shares), builders, "copies that build: {needs:?} {shares:?}");
        }
        // Plants that read none go to the copy that builds nothing; with every copy building, they are spread.
        assert_eq!(assign(&[NONE, MUT, NONE, READ, NONE], 2), vec![vec![1, 3], vec![0, 2, 4]]);
        assert_eq!(assign(&[MUT, NONE, MUT, NONE], 2), vec![vec![0, 1], vec![2, 3]]);
    }

    // no spec clause (PLANTS-PRODUCTS-ONCE): only a plant whose file may change the products and whose command uses
    // them (or measures the layout) gets them rebuilt with its mutant; reading is any of the three
    #[test]
    fn what_each_plant_needs_from_the_products() {
        let plant = |path: &str| parse_plants(&format!("x | {path} | lit:a ==> b | true | msg:X\n")).unwrap().remove(0);
        let ps = [plant("crates/a/src/lib.rs"), plant("crates/a/tests/t.rs"), plant("crates/a/src/lib.rs"), plant("crates/a/src/lib.rs"), plant("crates/a/tests/t.rs"), plant("crates/a/src/lib.rs")];
        let uses = [true, true, false, false, false, false];
        let reads = [false, false, true, false, false, false];
        let traced = [false, false, false, false, true, true];
        assert_eq!(needs(&ps, &uses, &reads, &traced), vec![MUT, READ, READ, NONE, READ, MUT]);
    }

    // no spec clause (PLANTS-PRODUCTS-ONCE): a file under a crate's tests/, benches/ or examples/ or under xtask/tests/
    // cannot change the products; anything else (a source, a header, xtask's own source, an odd path) may
    #[test]
    fn which_files_may_change_the_products() {
        for p in ["crates/niji/src/lib.rs", "crates/x/src/tests/a.rs", "csys/src/machine/x.S", "headers/stdio.h.in", "xtask/src/so.rs", "crates/x/build.rs", "crates/tests/src/a.rs", "crates/x/tests", "/crates/x/tests/a.rs", "crates/x/tests/../src/a.rs", "crates//x/tests/a.rs", "tests/oracle/a.c", ""] {
            assert!(may_change_products(p), "{p}");
        }
        for p in ["crates/niji/tests/stdio.rs", "crates/x/tests/support/mod.rs", "crates/x/benches/b.rs", "crates/x/examples/e.rs", "xtask/tests/plain.rs"] {
            assert!(!may_change_products(p), "{p}");
        }
    }

    // no spec clause (PLANTS-PRODUCTS-ONCE): a list of simple commands is split into words; any other shell syntax is
    // not read (None: the caller treats it as using the products)
    #[test]
    fn simple_commands_split_a_list_and_refuse_other_syntax() {
        let w = |v: &[&[&str]]| Some(v.iter().map(|c| c.iter().map(|s| s.to_string()).collect()).collect::<Vec<Vec<String>>>());
        assert_eq!(simple_commands("cargo test -p a"), w(&[&["cargo", "test", "-p", "a"]]));
        assert_eq!(simple_commands("sh -c 'a b && c \"d e\"; f || g'"), w(&[&["a", "b"], &["c", "d e"], &["f"], &["g"]]));
        assert_eq!(simple_commands("bash -c 'x'"), w(&[&["x"]]));
        assert_eq!(simple_commands("a 'b c'd"), w(&[&["a", "b cd"]]));
        for bad in ["a | b", "a > f", "a < f", "a & b", "a $X", "a \"$X\"", "a `b`", "a \\ b", "(a)", "a *", "a ?", "a [b]", "a {b}", "~/a", "a # c", "a\nb", "X=1 a", "a && sh x", "a; env b", "eval a", "exec a", "a &&", "&& a", "a ;; b", "'a", "sh -c 'a' 'b'", "a 2>&1"] {
            assert_eq!(simple_commands(bad), None, "{bad}");
        }
    }

    // no spec clause (PLANTS-PRODUCTS-ONCE): a header story's commands (the generator, then the header checks) read no
    // products unless an argument names them; anything else in the list still decides
    #[test]
    fn header_checks_read_no_products() {
        let lookup = |_: Option<&str>, t: &str| (t == "plain").then_some(false);
        let units = |_: Option<&str>, f: &[String], _: bool| f.iter().any(|f| f == "reads_them");
        for (cmd, want) in [
            ("sh -c 'cargo xtask headers-gen && python3 spec/tools/header_check.py --only stdio'", false),
            ("cargo xtask headers-gen", false),
            ("python3 spec/tools/header_checks/slice_12.py", false),
            ("sh -c 'cargo xtask headers-gen && cargo test -p xtask --test plain'", false),
            ("sh -c 'cargo xtask headers-gen && python3 spec/tools/header_check.py --include target/niji/include'", true),
            ("sh -c 'cargo xtask headers-gen && python3 spec/tools/header_checks/../x.py'", true),
            ("python3 spec/tools/header_checks/sub/x.py", true),
            ("python3 spec/tools/other.py", true),
            ("sh -c 'cargo xtask headers-gen && cargo xtask so'", true),
            ("sh -c 'cargo xtask headers-gen | tee x'", true),
            ("sh -c 'cargo xtask headers-gen && make check'", true),
        ] {
            assert_eq!(cmd_uses_products(cmd, lookup), want, "uses: {cmd}");
            assert_eq!(cmd_reads_products(cmd, lookup, units), want, "reads: {cmd}");
        }
    }

    // no spec clause (PLANTS-PRODUCTS-ONCE): unit tests do not use the products (their plants reuse the baseline) but
    // may read them: as the package's unit-test rule says, given the libtest filters
    #[test]
    fn unit_tests_read_the_products_by_the_unit_rule() {
        let lookup = |_: Option<&str>, _: &str| None;
        let units = |k: Option<&str>, f: &[String], exact: bool| k == Some("xtask") && f.iter().any(|f| f == "reads_them") && !exact;
        for (cmd, reads) in [
            ("cargo test -p xtask --bin xtask reads_them", true),
            ("cargo test -p xtask --bins -- reads_them --test-threads 1", true),
            ("cargo test -p xtask --bin xtask reads_them -- --exact", false),
            ("cargo test -p xtask --bin xtask other", false),
            ("cargo test -p xtask --lib -- --skip reads_them", false),
        ] {
            assert!(!cmd_uses_products(cmd, lookup), "uses: {cmd}");
            assert_eq!(cmd_reads_products(cmd, lookup, units), reads, "reads: {cmd}");
        }
    }

    // no spec clause (PLANTS-COPIES): auto runs two copies only when the job's RAM holds two at the measured peak, and
    // (PLANTS-WARM-COPY) only when the slot's CPUs or the file's plants make the split pay
    #[test]
    fn auto_copies_follows_the_ram_limit() {
        let big = SPLIT_CPUS;
        assert_eq!(auto_copies(None, big, 9), 1);
        assert_eq!(auto_copies(Some(2 * COPY_PEAK_BYTES - 1), big, 9), 1);
        assert_eq!(auto_copies(Some(2 * COPY_PEAK_BYTES), big, 9), 2);
        assert_eq!(auto_copies(Some(u64::MAX), big, 9), 2);
        assert_eq!(auto_copies(Some(u64::MAX), big, 1), 1, "one plant needs one copy");
        // A small slot: one copy up to SPLIT_PLANTS plants, two past it.
        assert_eq!(auto_copies(Some(u64::MAX), SPLIT_CPUS - 1, SPLIT_PLANTS), 1);
        assert_eq!(auto_copies(Some(u64::MAX), SPLIT_CPUS - 1, SPLIT_PLANTS + 1), 2);
        assert_eq!(auto_copies(Some(2 * COPY_PEAK_BYTES - 1), SPLIT_CPUS - 1, 50), 1, "RAM still decides first");
    }

    // no spec clause (PLANTS-COPIES): the RAM a job may use is the smallest of the role budget, every cgroup's
    // memory.max (`max` is no limit) and MemAvailable
    #[test]
    fn the_ram_limit_is_the_smallest_known() {
        const GIB: u64 = 1 << 30;
        let meminfo = "MemTotal:       79691776 kB\nMemAvailable:   62914560 kB\n";
        let maxes = |v: &[&str]| v.iter().map(|s| format!("{s}\n")).collect::<Vec<String>>();
        assert_eq!(smallest_limit(None, &maxes(&["max", "max"]), meminfo), Some(60 * GIB));
        assert_eq!(smallest_limit(None, &maxes(&["max", &(20 * GIB).to_string()]), meminfo), Some(20 * GIB));
        assert_eq!(smallest_limit(Some("8"), &maxes(&[&(20 * GIB).to_string()]), meminfo), Some(8 * GIB));
        assert_eq!(smallest_limit(Some("100"), &[], meminfo), Some(60 * GIB));
        assert_eq!(smallest_limit(Some("lots"), &maxes(&["max"]), ""), None);
    }

    // no spec clause (PLANTS-COPIES): --copies takes a number from 1 to 8 or auto
    #[test]
    fn copies_option_is_parsed_and_bounded() {
        let (src, dir) = toy("copiesarg");
        let file = src.join("plants.txt");
        fs::write(&file, format!("d1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:WRONG-ANSWER\n")).unwrap();
        let args = |c: &str| vec![file.display().to_string(), "--dir".into(), dir.display().to_string(), "--copies".into(), c.to_string()];
        assert_eq!(parse_args(&src, &args("2")).unwrap().copies, 2);
        assert!((1..=2).contains(&parse_args(&src, &args("auto")).unwrap().copies));
        for bad in ["0", "9", "two"] {
            assert!(parse_args(&src, &args(bad)).is_err(), "--copies {bad} accepted");
        }
        // PLANTS-WARM-COPY: warm copies by default beside the story directories, `--warm <dir>` elsewhere, `--cold` none.
        let base = src.parent().unwrap();
        assert_eq!(parse_args(&src, &args("1")).unwrap().warm, Some(base.parent().unwrap().join("plants-warm/tree")));
        assert_eq!(default_warm(Path::new("/dev/shm/claude/S-1/plants-2"), Path::new("/w/sh1")), Some(PathBuf::from("/dev/shm/claude/plants-warm/sh1")));
        assert_eq!(default_warm(Path::new("/s/plants-1"), Path::new("/w/sh1")), None, "no warm copies in /");
        let with = |extra: &[&str]| parse_args(&src, &[args("1"), extra.iter().map(|s| s.to_string()).collect()].concat()).map(|c| c.warm);
        assert_eq!(with(&["--warm", "/r/warm"]).unwrap(), Some(PathBuf::from("/r/warm")));
        assert_eq!(with(&["--cold"]).unwrap(), None);
        assert!(with(&["--warm"]).is_err(), "--warm without a directory accepted");
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    // no spec clause (PLANTS-COPIES): two copies give each plant the verdict, and results.txt the text, of a serial run;
    // a plant that survives in the second copy is still SURVIVED and the run exits 1
    #[test]
    fn two_copies_report_as_a_serial_run() {
        let plants = format!(
            "c1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:WRONG-ANSWER\n\
             s2 | src/value.txt | perl:s/(answer=42\\n)other=7/$1other=8/ | {CHECK} | msg:WRONG-ANSWER\n\
             e3 | src/value.txt | perl:s/(answer=42\\n)other=7/$1other=9/ | {CHECK} | equivalent:other= is not read\n\
             p4 | src/value.txt | perl:s/answer=42/answer=44/ | {PRODUCT_CHECK} | msg:WRONG-PRODUCT\n\
             c5 | src/value.txt | lit:answer=42 ==> answer=45 | {CHECK} | msg:WRONG-ANSWER\n"
        );
        let mut outs = Vec::new();
        for copies in [1, 2] {
            let (src, dir) = toy(&format!("copies{copies}"));
            let mut cfg = config(&src, &dir, &plants);
            cfg.prepare = sh(BUILD);
            cfg.uses_products = |_, ps| ps.iter().map(|p| p.cmd.contains("product.txt")).collect();
            cfg.copies = copies;
            let out = run(&cfg);
            outs.push((out.code, out.verdicts, fs::read_to_string(dir.join("results.txt")).unwrap()));
            if copies == 2 {
                // PLANTS-PRODUCTS-ONCE: p4, the one plant that reads the products, ran in the first copy, which built
                // them (baseline, mutant); the others ran in the second copy, with its own tree and logs and no build.
                assert!(dir.join("logs/p4.so.log").is_file() && dir.join("logs/so.log").is_file() && dir.join("logs-2/c1.log").is_file() && dir.join("logs-2/s2.log").is_file());
                assert!(!dir.join("logs/c1.log").exists() && !dir.join("logs-2/p4.log").exists());
                assert_eq!(fs::read_to_string(dir.join("src/builds.txt")).unwrap().lines().count(), 2);
                assert!(!dir.join("src-2/builds.txt").exists() && !dir.join("logs-2/so.log").exists(), "a copy whose plants read no products built them");
                assert_eq!(fs::read_to_string(dir.join("src-2/src/value.txt")).unwrap(), "answer=42\nother=7\nother=7\n");
            }
            let _ = fs::remove_dir_all(src.parent().unwrap());
        }
        assert_eq!(outs[0], outs[1]);
        let (code, verdicts, _) = &outs[1];
        assert_eq!(*code, EXIT_SURVIVED);
        let got: Vec<(&str, Verdict)> = verdicts.iter().map(|(id, v)| (id.as_str(), *v)).collect();
        assert_eq!(got, vec![("c1", Verdict::Caught), ("s2", Verdict::Survived), ("e3", Verdict::Equivalent), ("p4", Verdict::Caught), ("c5", Verdict::Caught)]);
    }

    // no spec clause (PLANTS-COPIES): a baseline that fails in one copy fails the run (exit 2, no results.txt), as in a
    // serial run
    #[test]
    fn a_baseline_failing_in_one_copy_fails_the_run() {
        let (src, dir) = toy("copiesbaseline");
        let plants = format!(
            "c1 | src/value.txt | lit:answer=42 ==> answer=43 | {CHECK} | msg:WRONG-ANSWER\n\
             b2 | src/value.txt | perl:s/answer=42/answer=44/ | false | msg:X\n"
        );
        let mut cfg = config(&src, &dir, &plants);
        cfg.copies = 2;
        let out = run(&cfg);
        assert_eq!(out.code, EXIT_BASELINE);
        assert!(!dir.join("results.txt").exists());
        let _ = fs::remove_dir_all(src.parent().unwrap());
    }
}
