//! `cargo xtask precheck [--jobs <label>,...] [--head <sha>] [--only <check>,...] [--base <ref>] [--logs <dir>]
//! [--base-src <dir>] [--host <name>] [--story <id>]` (HANDBACK-INLOOP-CHECKS): every cheap check that judges a
//! hand-back, run in the agent's own loop on its role before it hands back. One verdict line per check
//! (`precheck: <check> ok|RED|WAIT|note ...`), each red naming its fix; exit 0 = green, 1 = a red, 3 = no red but a
//! named job is still queued or running (a hand-back now is `PENDING JOBS`, never `COMPLETE`), 2 = usage.
//!
//! Why: of 30 header-slice reds (11-14 to 11-18), 29 were decided by a check that ran only after the hand-back or was
//! never read: jobs still queued at a budget checkpoint, named jobs that failed later, jobs that tested older content
//! than the hand-back head, a FULL-tier `stubs` red, an undiagnosed gate, placeholder record ids. Each cost a whole new
//! run. The checks, in order (`--only` runs a subset, for a quick rerun; its verdict says it is partial):
//!
//! 1. `stubs` and every other step of the FULL tier (`cargo xtask test`, `test_steps` in main.rs) that takes seconds:
//!    the policy audits, `trace`, `fuzz-check`, every `*-gen --check`, `remez --check`, `tables --check`
//!    ([`CHEAP_STEPS`], whatever the change touches), then the validation self-tests the FULL tier adds
//!    (`affected::VALIDATION_SELF_TESTS`). `stubs` writes into the tree: a file it rewrites is a red ("run
//!    `cargo xtask stubs` and commit"), since the committed tree is then not what the gate tests. Not here because
//!    they take minutes (times from a FULL gate on the Skylake-SP host, 11-18): `products` (424 s), the two nextest passes
//!    (1385 s and 875 s), `conformance` (27 s, needs the products), `native-examples` (54 s), `rust-dylib` (66 s).
//! 2. `static`: the rest of `cargo xtask static` (the list `~/bin/static-checks` and CI's precheck run), each check by
//!    name; the ones step 1 already ran are not run twice.
//! 3. `placeholders`: the ops hand-back check's step b (`ops/handback.py` `id_problems`, which reuses CI's
//!    `CloneGit.placeholders`): a line the branch adds that names a placeholder id (`COMPAT-`, `D-` or `SR-` with `NEW` as its number) is red, except
//!    under `handoff/`, the placeholder machinery (`xtask/src/records/`, `tools/records_import.py`) and
//!    `records/pending/` (a pending record is the documented hand-back without an id: a note, `pending_notes`). Also
//!    red (rules/thread_common.txt, "Ids are assigned"): an added line of `docs/differences-from-glibc.md` or
//!    `docs/compat-flags.md` that names an id as pending or to-be-assigned. The fix names `ops reserve`. Not mirrored:
//!    the reservation check of a real id (`decision_id_problems`) needs the ops store; `ops handback-check` runs it.
//! 4. `train-owned`: CI's train-owned rule (`ci.train_owned_changes`, also hand-back step b): a changed generated view
//!    (`path = "..."` in `records/views.toml`, `crates/math/KERNELS.md`, `docs/codemap/symbols.tsv`) that differs from
//!    the merge base is red.
//! 5. `jobs` (`--jobs`, the labels you will name in `JOBS:`; labels of another host are left to `ops handback-check`):
//!    hand-back step a (`jobs_problems`): no job at all, or an ended job with rc != 0, is red (with the first failure
//!    lines of its log); a plants job must have run through `cargo xtask plants` (a `plants: copy` line); a job with a
//!    log but no `jobs.tsv` row is still queued or running: WAIT. Also red: a `gate` job whose log has no `affected: ok`
//!    verdict (an `affected --list`, or a run that stopped before its verdict). Step a2 (`tested_problems`, the
//!    verifier's `dispatcher.tested_diff`): the job's `~/logs/.tested/<label>` content tree must be the tree the role
//!    holds now, outside prose (`handoff/`, `docs/`, `records/`, `results/`, `*.md`). Labels of jobs that measure the
//!    base on purpose (`before`, `base`, `baseline`, `pre`) are not judged, as there. See [`stale_files`] for how a
//!    role without git tells a prose-only difference.
//! 6. `head` (`--head <sha>`): the role holds that commit, synced from a clean worktree, so the jobs and checks above
//!    judged the head you hand back.
//! 7. `clippy`: `cargo xtask clippy` (the gate's flags), last because it is the slowest.
//!
//! Changed files and added lines come from git in a checkout (merge base with `--base`, default `origin/next`), and
//! in a role directory (no `.git`) from what the sync wrote: `.changed` (files changed against the merge base),
//! `.synced` (`merge_base`, `content_tree`) and the merge base's source primed on the host (`~/wl/src/<merge base>`,
//! or `--base-src`). Without that source an added line is any line of a changed file (stricter, never laxer).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Instant, SystemTime};

/// The steps of the FULL tier that take seconds, in `test_steps` order (main.rs runs them from this list), with
/// `stubs` first (the FULL gate's own first step). `affected` runs these as prechecks of an escalated run.
pub(crate) const CHEAP_STEPS: &[&str] = &[
    "stubs", "purity", "unsafe-audit", "csys-audit", "env-audit", "opaque-audit", "crash-audit", "reexec-audit", "trace",
    "fuzz-check", "csys-gen-check", "abi-gen-check", "unicode-gen-check", "encodings-gen-check", "locale-gen-check",
    "native-gen-check", "headers-gen-check", "compat-gen-check", "math-gen-check", "remez-check", "tables-check",
];

/// Steps of `cargo xtask static` that run the same function as a [`CHEAP_STEPS`] entry (not run twice).
const STATIC_SAME_AS_CHEAP: &[(&str, &str)] = &[
    ("purity", "purity"), ("unsafe-audit", "unsafe-audit"), ("csys-audit", "csys-audit"), ("env-audit", "env-audit"),
    ("opaque-audit", "opaque-audit"), ("crash-audit", "crash-audit"), ("reexec-audit", "reexec-audit"),
    ("trace", "trace"), ("fuzz-check", "fuzz-check"), ("compat-gen", "compat-gen-check"), ("tables-fresh", "tables-check"),
];


/// The fix a red step names.
pub(crate) fn fix(name: &str) -> String {
    let regen = |g: &str| format!("run `cargo xtask {g}` and commit what it writes");
    match name {
        "stubs" => "run `cargo xtask stubs` and commit what it writes; a new test names its clause id or `no spec clause`".into(),
        "headers-gen-check" | "abi-gen-check" => "run `cargo xtask headers-gen` (it also runs abi-gen), then `cargo xtask stubs`, and commit".into(),
        "csys-gen-check" => regen("csys-gen"),
        "unicode-gen-check" => regen("unicode-gen"),
        "encodings-gen-check" => regen("encodings-gen"),
        "locale-gen-check" => regen("locale-gen"),
        "native-gen-check" => regen("native-gen"),
        "compat-gen-check" => regen("compat-gen"),
        "math-gen-check" => regen("math-gen"),
        "remez-check" => "run `cargo xtask remez <fit>` for the fit named above and commit the coefficients".into(),
        "tables-check" => regen("tables"),
        "trace" => "name the spec clause each new test covers (or `no spec clause`)".into(),
        "fuzz-check" => "make the fuzz harnesses build again (fuzz/)".into(),
        "clippy" => "fix the lints (`cargo xtask clippy`; full output in target/precheck-clippy.log)".into(),
        n if n.starts_with("validation:") => "run the self-test named above (`bash <it>`) and fix it".into(),
        _ => "see the message above (the check's own text names the rule)".into(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    Ok,
    Note,
    Wait,
    Red,
}

/// A check's outcome: its level and lines (`ok`: one summary; `red`: the problems).
#[derive(Debug)]
pub(crate) struct Verdict {
    pub level: Level,
    pub lines: Vec<String>,
}

impl Verdict {
    fn ok(s: impl Into<String>) -> Self {
        Verdict { level: Level::Ok, lines: vec![s.into()] }
    }
    fn from_problems(red: Vec<String>, wait: Vec<String>, notes: Vec<String>, ok: impl Into<String>) -> Self {
        if !red.is_empty() {
            Verdict { level: Level::Red, lines: red.into_iter().chain(wait).chain(notes).collect() }
        } else if !wait.is_empty() {
            Verdict { level: Level::Wait, lines: wait.into_iter().chain(notes).collect() }
        } else if !notes.is_empty() {
            Verdict { level: Level::Note, lines: notes }
        } else {
            Verdict::ok(ok)
        }
    }
}

/// Options of a run (the tests build these directly).
#[derive(Debug, Default, Clone)]
pub(crate) struct Opts {
    pub jobs: Vec<String>,
    pub head: Option<String>,
    pub only: Option<BTreeSet<String>>,
    pub base: String,
    pub logs: PathBuf,
    pub base_src: Option<PathBuf>,
    pub host: String,
    pub story: Option<String>,
}

const USAGE: &str = "usage: cargo xtask precheck [--jobs <label>,...] [--head <sha>] [--only <check>,...] [--base <ref>] [--logs <dir>] [--base-src <dir>] [--host <name>] [--story <id>]";

pub fn main(root: &Path, args: &[String]) -> ExitCode {
    let opts = match parse_args(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("precheck: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let started = Instant::now();
    let mut worst = Level::Ok;
    let mut reds: Vec<String> = Vec::new();
    let mut waits: Vec<String> = Vec::new();
    let mut n = 0usize;
    let mut report = |name: &str, secs: f64, v: Verdict| {
        n += 1;
        print_verdict(name, secs, &v);
        match v.level {
            Level::Red => reds.push(name.to_string()),
            Level::Wait => waits.push(name.to_string()),
            _ => {}
        }
        worst = worst.max(v.level);
    };
    for (name, secs, v) in run_checks(root, &opts) {
        report(&name, secs, v);
    }
    let partial = if opts.only.is_some() { " (partial: --only; not a hand-back precheck)" } else { "" };
    let total = started.elapsed().as_secs_f64();
    match worst {
        Level::Red => {
            println!("precheck: RED: {} ({n} checks, {total:.0}s){partial}: fix each red above; a hand-back with a red precheck is NOT READY", reds.join(", "));
            ExitCode::FAILURE
        }
        Level::Wait => {
            println!("precheck: WAIT: {} ({n} checks, {total:.0}s){partial}: no red, but a named job has not ended; jobwait it, or hand back PENDING JOBS", waits.join(", "));
            ExitCode::from(3)
        }
        _ => {
            println!("precheck: GREEN ({n} checks, {total:.0}s){partial}");
            ExitCode::SUCCESS
        }
    }
}

fn print_verdict(name: &str, secs: f64, v: &Verdict) {
    match v.level {
        Level::Ok => println!("precheck: {name} ok ({secs:.1}s){}", v.lines.first().filter(|l| !l.is_empty()).map(|l| format!(": {l}")).unwrap_or_default()),
        Level::Note => {
            for l in &v.lines {
                println!("precheck: {name} note ({secs:.1}s): {l}");
            }
        }
        Level::Wait => {
            for l in &v.lines {
                println!("precheck: {name} WAIT ({secs:.1}s): {l}");
            }
        }
        Level::Red => {
            for l in &v.lines {
                println!("precheck: {name} RED ({secs:.1}s): {l}");
            }
        }
    }
}

fn parse_args(args: &[String]) -> Result<Opts, String> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut o = Opts { base: "origin/next".into(), logs: home.join("logs"), host: own_host(), ..Opts::default() };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = || it.next().cloned().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--jobs" => o.jobs.extend(split_jobs(&val()?)),
            "--head" => o.head = Some(val()?),
            "--only" => o.only = Some(val()?.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()),
            "--base" => o.base = val()?,
            "--logs" => o.logs = PathBuf::from(val()?),
            "--base-src" => o.base_src = Some(PathBuf::from(val()?)),
            "--host" => o.host = val()?,
            "--story" => o.story = Some(val()?),
            x => return Err(format!("unknown argument `{x}`")),
        }
    }
    if let Some(only) = &o.only {
        let known = check_names();
        if let Some(bad) = only.iter().find(|c| !known.contains(c.as_str()) && c.as_str() != "cheap") {
            return Err(format!("--only: unknown check `{bad}` (known: cheap, {})", known.iter().copied().collect::<Vec<_>>().join(", ")));
        }
    }
    Ok(o)
}

/// `a, b (remark), host:c` -> labels; remarks in parentheses are dropped, as the hand-back's JOBS: line allows.
fn split_jobs(s: &str) -> Vec<String> {
    let mut out = String::new();
    let mut depth = 0;
    for ch in s.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = (depth - 1i32).max(0),
            c if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

fn own_host() -> String {
    fs::read_to_string("/proc/sys/kernel/hostname").map(|h| h.trim().split('.').next().unwrap_or("").to_string()).unwrap_or_default()
}

fn check_names() -> BTreeSet<&'static str> {
    let mut s: BTreeSet<&'static str> = CHEAP_STEPS.iter().copied().collect();
    s.extend(["validation", "static", "placeholders", "train-owned", "jobs", "head", "clippy"]);
    s
}

fn wanted(opts: &Opts, name: &str) -> bool {
    match &opts.only {
        None => true,
        Some(set) => set.contains(name) || (set.contains("cheap") && CHEAP_STEPS.contains(&name)),
    }
}

/// Every selected check, in order: (name, seconds, verdict).
pub(crate) fn run_checks(root: &Path, opts: &Opts) -> Vec<(String, f64, Verdict)> {
    let mut out = Vec::new();
    let mut timed = |name: &str, f: &mut dyn FnMut() -> Verdict| {
        let t = Instant::now();
        let v = f();
        out.push((name.to_string(), t.elapsed().as_secs_f64(), v));
    };
    for &step in CHEAP_STEPS {
        if !wanted(opts, step) {
            continue;
        }
        timed(step, &mut || {
            let before = if step == "stubs" { Some(stubs_snapshot(root)) } else { None };
            let r = crate::cheap_step(root, step);
            let rewrote: Vec<String> = before.map(|b| changed_paths(&b, &stubs_snapshot(root))).unwrap_or_default();
            step_verdict(step, r, &rewrote)
        });
    }
    if wanted(opts, "validation") {
        for s in crate::affected::VALIDATION_SELF_TESTS {
            let name = format!("validation:{s}");
            timed(&name, &mut || match Command::new("bash").arg(s).current_dir(root).output() {
                Ok(o) if o.status.success() => Verdict::ok(""),
                Ok(o) => Verdict { level: Level::Red, lines: vec![format!("{s}: {} ({})", o.status, first_lines(&String::from_utf8_lossy(&o.stderr), 3)), format!("fix: {}", fix(&name))] },
                Err(e) => Verdict { level: Level::Red, lines: vec![format!("cannot run bash {s}: {e}")] },
            });
        }
    }
    if wanted(opts, "static") {
        let same: BTreeSet<&str> = STATIC_SAME_AS_CHEAP.iter().filter(|(_, c)| wanted(opts, c)).map(|(s, _)| *s).collect();
        timed("static", &mut || {
            let mut red = Vec::new();
            let mut ran = 0;
            for (name, f) in crate::static_list() {
                if same.contains(name) {
                    continue;
                }
                ran += 1;
                if let Err(e) = f(root) {
                    red.push(format!("{name} FAILED: {}", first_lines(&e, 3)));
                }
            }
            if !red.is_empty() {
                red.push("fix: each FAILED check's message names its rule (`cargo xtask static` for the full text)".into());
            }
            Verdict::from_problems(red, vec![], vec![], format!("{ran} checks (the rest ran above)"))
        });
    }
    let needs_changes = ["placeholders", "train-owned"].iter().any(|c| wanted(opts, c));
    let changes = if needs_changes { Some(Changes::find(root, opts)) } else { None };
    if wanted(opts, "placeholders") {
        timed("placeholders", &mut || match &changes {
            Some(Ok(c)) => placeholder_verdict(c, opts),
            Some(Err(e)) => Verdict { level: Level::Red, lines: vec![format!("cannot tell the branch's changes: {e}")] },
            None => Verdict::ok(""),
        });
    }
    if wanted(opts, "train-owned") {
        timed("train-owned", &mut || match &changes {
            Some(Ok(c)) => train_owned_verdict(root, c),
            Some(Err(e)) => Verdict { level: Level::Red, lines: vec![format!("cannot tell the branch's changes: {e}")] },
            None => Verdict::ok(""),
        });
    }
    if wanted(opts, "jobs") {
        timed("jobs", &mut || jobs_verdict(root, opts));
    }
    if wanted(opts, "head") {
        timed("head", &mut || head_verdict(root, opts));
    }
    if wanted(opts, "clippy") {
        timed("clippy", &mut || clippy_verdict(root));
    }
    out
}

/// A cheap step's verdict: red when it failed, or when it passed but rewrote files (`stubs`: the committed tree is
/// then not the one the gate tests, which runs `stubs` first).
fn step_verdict(step: &str, r: Result<(), String>, rewrote: &[String]) -> Verdict {
    match r {
        Err(e) => Verdict { level: Level::Red, lines: vec![first_lines(&e, 4), format!("fix: {}", fix(step))] },
        Ok(()) if !rewrote.is_empty() => Verdict { level: Level::Red, lines: vec![format!("stubs rewrote {} (the committed tree is not what the gate tests)", list5(rewrote)), format!("fix: {}", fix(step))] },
        Ok(()) => Verdict::ok(""),
    }
}

fn first_lines(s: &str, n: usize) -> String {
    s.lines().map(str::trim).filter(|l| !l.is_empty()).take(n).collect::<Vec<_>>().join(" | ")
}

fn list5(v: &[String]) -> String {
    let mut s = v.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    if v.len() > 5 {
        s.push_str(&format!(", ... ({} in all)", v.len()));
    }
    s
}

// --- stubs: what it may rewrite ------------------------------------------------------------------------------------

/// Content hashes of the files `cargo xtask stubs` writes: `spec/{clauses,symbols,primitives}.tsv` and the test files
/// of every crate.
fn stubs_snapshot(root: &Path) -> BTreeMap<PathBuf, (u64, u64)> {
    let mut out = BTreeMap::new();
    let mut files: Vec<PathBuf> = ["clauses", "symbols", "primitives"].iter().map(|f| root.join(format!("spec/{f}.tsv"))).collect();
    if let Ok(rd) = fs::read_dir(root.join("crates")) {
        for e in rd.flatten() {
            walk_files(&e.path().join("tests"), &mut files);
        }
    }
    for f in files {
        if let Ok(b) = fs::read(&f) {
            let mut h = DefaultHasher::new();
            b.hash(&mut h);
            out.insert(f.strip_prefix(root).map(Path::to_path_buf).unwrap_or(f), (b.len() as u64, h.finish()));
        }
    }
    out
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            match e.file_type() {
                Ok(t) if t.is_dir() => walk_files(&p, out),
                Ok(t) if t.is_file() => out.push(p),
                _ => {}
            }
        }
    }
}

fn changed_paths(a: &BTreeMap<PathBuf, (u64, u64)>, b: &BTreeMap<PathBuf, (u64, u64)>) -> Vec<String> {
    let keys: BTreeSet<&PathBuf> = a.keys().chain(b.keys()).collect();
    keys.into_iter().filter(|k| a.get(*k) != b.get(*k)).map(|k| k.display().to_string()).collect()
}

// --- the branch's changes --------------------------------------------------------------------------------------------

/// What the branch changed: files and, per file, the lines it added (relative to the merge base).
#[derive(Debug, Default)]
pub(crate) struct Changes {
    pub files: Vec<String>,
    pub added: BTreeMap<String, Vec<String>>,
    /// Files whose content equals the merge base's (restored), when that can be told.
    pub same_as_base: BTreeSet<String>,
    pub notes: Vec<String>,
}

/// The merge base's source primed on the host (`--base-src`, else `~/wl/src/<merge_base of .synced>`).
fn base_src(root: &Path, opts: &Opts) -> Option<PathBuf> {
    opts.base_src.clone().or_else(|| {
        let synced = read_kv(&root.join(".synced"));
        let mb = synced.get("merge_base").filter(|s| !s.is_empty())?;
        Some(std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("wl/src").join(mb))
    })
}

impl Changes {
    fn find(root: &Path, opts: &Opts) -> Result<Changes, String> {
        if root.join(".git").exists() {
            Changes::from_git(root, &opts.base)
        } else {
            let base_src = base_src(root, opts);
            let list = fs::read_to_string(root.join(".changed")).map_err(|e| format!("no .git and no .changed ({e}): sync the role with --base"))?;
            Ok(Changes::from_role(root, &list, base_src.as_deref()))
        }
    }

    fn from_git(root: &Path, base: &str) -> Result<Changes, String> {
        let git = |args: &[&str]| -> Result<String, String> {
            let o = Command::new("git").current_dir(root).args(args).output().map_err(|e| format!("git: {e}"))?;
            if !o.status.success() {
                return Err(format!("git {}: {}", args.join(" "), first_lines(&String::from_utf8_lossy(&o.stderr), 2)));
            }
            Ok(String::from_utf8_lossy(&o.stdout).into_owned())
        };
        let mb = git(&["merge-base", "HEAD", base])?.trim().to_string();
        let mut c = Changes::default();
        let mut files: BTreeSet<String> = git(&["diff", "--name-only", &mb])?.lines().map(str::to_string).collect();
        let untracked: Vec<String> = git(&["ls-files", "-o", "--exclude-standard"])?.lines().map(str::to_string).collect();
        let mut cur = String::new();
        for ln in git(&["diff", "-U0", "--no-color", &mb])?.lines() {
            if let Some(f) = ln.strip_prefix("+++ ") {
                cur = f.strip_prefix("b/").unwrap_or(f).to_string();
            } else if let Some(l) = ln.strip_prefix('+') {
                c.added.entry(cur.clone()).or_default().push(l.to_string());
            }
        }
        for f in &untracked {
            let text = fs::read(root.join(f)).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
            c.added.insert(f.clone(), text.lines().map(str::to_string).collect());
        }
        files.extend(untracked);
        c.files = files.into_iter().collect();
        Ok(c)
    }

    /// A role directory: `.changed` names the files; a line is added when the merge base's copy of the file lacks it
    /// (all lines when there is no base source: stricter, never laxer).
    pub(crate) fn from_role(root: &Path, changed: &str, base_src: Option<&Path>) -> Changes {
        let mut c = Changes::default();
        let base_ok = base_src.filter(|b| b.is_dir());
        if base_ok.is_none() {
            c.notes.push(format!("no merge-base source ({}): every line of a changed file counts as added", base_src.map_or("none".into(), |b| b.display().to_string())));
        }
        for f in changed.lines().map(str::trim).filter(|l| !l.is_empty()) {
            c.files.push(f.to_string());
            let Ok(new) = fs::read(root.join(f)) else { continue };
            let new = String::from_utf8_lossy(&new).into_owned();
            let old = base_ok.and_then(|b| fs::read(b.join(f)).ok()).map(|b| String::from_utf8_lossy(&b).into_owned());
            if old.as_deref() == Some(new.as_str()) {
                c.same_as_base.insert(f.to_string());
                continue;
            }
            let old_lines: BTreeSet<&str> = old.as_deref().map(|o| o.lines().collect()).unwrap_or_default();
            let added: Vec<String> = new.lines().filter(|l| !old_lines.contains(l)).map(str::to_string).collect();
            if !added.is_empty() {
                c.added.insert(f.to_string(), added);
            }
        }
        c
    }
}

/// CI's PLACEHOLDER_RE: `COMPAT`, `D` or `SR`, a hyphen and `NEW`, as whole words.
pub(crate) fn has_placeholder(line: &str) -> bool {
    let b = line.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut i = 0;
    while let Some(p) = line[i..].find("-NEW") {
        let at = i + p;
        let end = at + 4;
        let after_ok = end >= b.len() || !word(b[end]);
        let pre = &line[..at];
        let hit = ["COMPAT", "D", "SR"].iter().any(|w| pre.ends_with(w) && (pre.len() == w.len() || !word(b[pre.len() - w.len() - 1])));
        if hit && after_ok {
            return true;
        }
        i = end;
    }
    false
}

/// An added docs row that names an id as not yet given (rules: "A row in docs/ must name a real id, never 'pending'
/// or 'assigned when it lands'").
pub(crate) fn docs_unassigned_id(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    l.contains("assigned when it lands") || ["compat-", "d-x.", "sr-"].iter().any(|p| ["tbd", "pending", "xx", "?", "next"].iter().any(|s| l.contains(&format!("{p}{s}"))))
}

const PLACEHOLDER_EXEMPT: &[&str] = &["handoff/", "xtask/src/records/", "tools/records_import.py"];
const DOCS_ID_FILES: &[&str] = &["docs/differences-from-glibc.md", "docs/compat-flags.md"];

fn placeholder_verdict(c: &Changes, opts: &Opts) -> Verdict {
    let story = opts.story.clone().unwrap_or_else(|| "<your story>".into());
    let mut red = Vec::new();
    let mut notes = c.notes.clone();
    for (f, lines) in &c.added {
        if PLACEHOLDER_EXEMPT.iter().any(|p| f.starts_with(p)) {
            continue;
        }
        for l in lines {
            if has_placeholder(l) {
                if f.starts_with("records/pending/") {
                    notes.push(format!("pending record {f}: the coordinator assigns its id; CI holds the branch until then"));
                    break;
                }
                red.push(format!("placeholder {f}: {}: reserve the id (`python3 ~/src/liella-ops/bin/ops reserve --story {story} COMPAT` or `D-X`) and use it, or move the record to records/pending/", l.trim().chars().take(120).collect::<String>()));
            } else if DOCS_ID_FILES.contains(&f.as_str()) && docs_unassigned_id(l) {
                red.push(format!("unassigned id {f}: {}: a docs row names a real id (`ops reserve --story {story} COMPAT`)", l.trim().chars().take(120).collect::<String>()));
            }
        }
    }
    Verdict::from_problems(red, vec![], notes, format!("{} changed files", c.files.len()))
}

/// The generated views only the train writes (CI `train_owned_files`): `path = "..."` lines of records/views.toml,
/// KERNELS.md, the codemap symbol table. Empty without records/views.toml.
pub(crate) fn train_owned_files(root: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(root.join("records/views.toml")) else { return Vec::new() };
    let mut out: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("path")?.trim_start().strip_prefix('=')?.trim();
            Some(rest.strip_prefix('"')?.strip_suffix('"')?.to_string())
        })
        .collect();
    out.push("crates/math/KERNELS.md".into());
    if root.join("docs/codemap/symbols.tsv").exists() {
        out.push("docs/codemap/symbols.tsv".into());
    }
    out.dedup();
    out
}

fn train_owned_verdict(root: &Path, c: &Changes) -> Verdict {
    let owned: BTreeSet<String> = train_owned_files(root).into_iter().collect();
    let red: Vec<String> = c
        .files
        .iter()
        .filter(|f| owned.contains(*f) && !c.same_as_base.contains(*f))
        .map(|f| format!("train-owned file {f} changed: the train generates it; restore it (`git checkout $(git merge-base HEAD origin/next) -- {f}`) and write a record instead (`cargo xtask records new ...`)"))
        .collect();
    Verdict::from_problems(red, vec![], vec![], format!("{} generated views checked", owned.len()))
}

// --- jobs -------------------------------------------------------------------------------------------------------------

pub(crate) fn read_kv(p: &Path) -> BTreeMap<String, String> {
    fs::read_to_string(p)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('=').map(|(k, v)| (k.trim().to_string(), v.trim().to_string())))
        .collect()
}

/// The dispatcher's `_BASELINE_LABEL`: `(?:^|[-_.])(?:before|base|baseline|pre)(?:[-_.0-9]|$)`.
pub(crate) fn is_baseline_label(label: &str) -> bool {
    let b = label.as_bytes();
    for w in ["before", "baseline", "base", "pre"] {
        let mut i = 0;
        while let Some(p) = label[i..].find(w) {
            let at = i + p;
            let end = at + w.len();
            let pre_ok = at == 0 || matches!(b[at - 1], b'-' | b'_' | b'.');
            let post_ok = end == b.len() || matches!(b[end], b'-' | b'_' | b'.' | b'0'..=b'9');
            if pre_ok && post_ok {
                return true;
            }
            i = at + 1;
        }
    }
    false
}

/// The last `jobs.tsv` row per label: (kind, rc).
fn jobs_rows(logs: &Path, labels: &BTreeSet<String>) -> BTreeMap<String, (String, i64)> {
    let mut out = BTreeMap::new();
    for line in fs::read_to_string(logs.join("jobs.tsv")).unwrap_or_default().lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 || !labels.contains(f[0]) {
            continue;
        }
        if let Ok(rc) = f[4].trim().parse::<i64>() {
            out.insert(f[0].to_string(), (f[1].to_string(), rc));
        }
    }
    out
}

/// The failure lines of a job's log worth showing (the summary's FAILED lines, its verdict, errors, survivors).
fn log_digest(log: &str) -> String {
    let keep = |l: &str| l.starts_with("FAILED: ") || l.starts_with("affected: ") || l.starts_with("error") || l.contains("SURVIVED") || l.contains("panicked at") || l.starts_with("xtask ");
    let v: Vec<&str> = log.lines().map(str::trim).filter(|l| keep(l)).take(4).collect();
    if v.is_empty() { "no failure line in the log; read it".into() } else { v.join(" | ") }
}

/// Paths a prose-only difference may touch (the verifier's TESTED_EXCLUDES).
pub(crate) fn is_prose(path: &str) -> bool {
    ["handoff", "docs", "records", "results"].iter().any(|p| path == *p || path.starts_with(&format!("{p}/"))) || path.ends_with(".md")
}

/// The ignore patterns of one `.gitignore`: (the directory it sits in, as a `dir/` prefix or "" at the root, its
/// patterns). Negations (`!`) are not read: an entry they would keep counts as ignored, which errs only on paths
/// the tree marks as build output.
type Ignores = Vec<(String, Vec<String>)>;

fn read_ignores(root: &Path, dir: &str, into: &mut Ignores) {
    let p = if dir.is_empty() { root.join(".gitignore") } else { root.join(dir).join(".gitignore") };
    let Ok(text) = fs::read_to_string(p) else { return };
    let pats: Vec<String> = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('!')).map(str::to_string).collect();
    into.push((if dir.is_empty() { String::new() } else { format!("{dir}/") }, pats));
}

/// The role-root entries the sync never copies (sync_role.sh's excludes) or the tree ignores (each `.gitignore` for
/// the paths below its own directory, as git reads them: `fuzz/.gitignore`'s `target/` is `fuzz/**/target`).
fn role_skip(rel: &str, ignores: &Ignores) -> bool {
    let first = rel.split('/').next().unwrap_or("");
    if matches!(first, "target" | ".git" | ".synced" | ".released" | ".changed" | ".changed.hdr" | ".changed.xtask") {
        return true;
    }
    ignores.iter().any(|(dir, pats)| {
        let Some(rel) = rel.strip_prefix(dir.as_str()) else { return false };
        let base = rel.rsplit('/').next().unwrap_or(rel);
        pats.iter().any(|p| {
            if let Some(anch) = p.strip_prefix('/') {
                rel == anch || rel.starts_with(&format!("{anch}/"))
            } else if let Some(d) = p.strip_suffix('/') {
                rel.split('/').any(|c| c == d)
            } else if let Some(ext) = p.strip_prefix('*') {
                !ext.contains(['*', '?', '[']) && base.ends_with(ext)
            } else {
                !p.contains(['*', '?', '[']) && rel.split('/').any(|c| c == p)
            }
        })
    })
}

/// Step a2 in a role without git. jobrun copied the role's `.synced` to `.tested/<label>` when the job started, and a
/// sync refuses (BUSY) while a job runs in the role, so every file a later sync wrote has a newer modification time
/// than that copy (the sync copies by content and never preserves times, so an unchanged file is not touched). The
/// files newer than the copy, outside prose and outside what the sync never copies or the tree ignores, are what the
/// job did not test.
///
/// A removed file shows only as a newer directory, but so does an ignored build output a job or the sync added or
/// removed there (the gate's `conformance/build`, the fuzz check's `fuzz/target`). With the merge base's source
/// (`base`): a newer directory counts when it lacks an entry the merge base has there (removed after the job; or
/// before it, in a directory a build output touched since, which only errs strict). A file the branch added and a
/// later sync removed is not seen here, nor a removed top-level file (the root is newer after every sync, which
/// relinks `target`); `ops handback-check` compares the content trees and sees both. Without the merge base's source
/// every newer directory counts (strict).
pub(crate) fn stale_files(root: &Path, since: SystemTime, base: Option<&Path>) -> Vec<String> {
    let base = base.filter(|b| b.is_dir());
    let mut ignores = Ignores::new();
    read_ignores(root, "", &mut ignores);
    let mut out = Vec::new();
    let mut stack = vec![String::new()];
    while let Some(rel) = stack.pop() {
        let Ok(rd) = fs::read_dir(if rel.is_empty() { root.to_path_buf() } else { root.join(&rel) }) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let rs = if rel.is_empty() { name } else { format!("{rel}/{name}") };
            if role_skip(&rs, &ignores) {
                continue;
            }
            let Ok(md) = fs::symlink_metadata(e.path()) else { continue };
            let newer = md.modified().is_ok_and(|m| m > since);
            if md.is_dir() {
                read_ignores(root, &rs, &mut ignores);
                if newer && !is_prose(&rs) {
                    match base {
                        Some(b) => out.extend(removed_entries(root, b, &rs, &ignores)),
                        None => out.push(format!("{rs}/")),
                    }
                }
                stack.push(rs);
            } else if newer && !is_prose(&rs) {
                out.push(rs);
            }
        }
    }
    out.sort();
    out
}

/// The entries of the merge base's `dir` (outside prose and what the tree ignores) that the role's `dir` lacks.
fn removed_entries(root: &Path, base: &Path, dir: &str, ignores: &Ignores) -> Vec<String> {
    let Ok(rd) = fs::read_dir(base.join(dir)) else { return vec![] };
    rd.flatten()
        .map(|e| format!("{dir}/{}", e.file_name().to_string_lossy()))
        .filter(|r| !role_skip(r, ignores) && !is_prose(r) && fs::symlink_metadata(root.join(r)).is_err())
        .map(|r| format!("{r} (removed)"))
        .collect()
}

fn jobs_verdict(root: &Path, opts: &Opts) -> Verdict {
    if opts.jobs.is_empty() {
        return Verdict { level: Level::Note, lines: vec!["no --jobs: name the labels you will put in JOBS: to check them".into()] };
    }
    let synced = read_kv(&root.join(".synced"));
    let story = opts.story.clone().or_else(|| synced.get("story").cloned()).unwrap_or_default().to_ascii_lowercase();
    let (mut red, mut wait, mut notes) = (Vec::new(), Vec::new(), Vec::new());
    let mut mine = BTreeSet::new();
    for lab in &opts.jobs {
        match lab.rsplit_once(':') {
            Some((h, l)) if h != opts.host => notes.push(format!("{l} on {h}: another host; `ops handback-check` judges it (or run precheck there)")),
            Some((_, l)) => {
                mine.insert(l.to_string());
            }
            None => {
                mine.insert(lab.clone());
            }
        }
    }
    let rows = jobs_rows(&opts.logs, &mine);
    for lab in &mine {
        let log_path = opts.logs.join(format!("{lab}.log"));
        let log = fs::read(&log_path).map(|b| String::from_utf8_lossy(&b).into_owned());
        let Some((kind, rc)) = rows.get(lab) else {
            if log.is_ok() {
                wait.push(format!("{lab}: queued or running (a log, no jobs.tsv row): `~/bin/jobwait {lab}`; handing back now is PENDING JOBS"));
            } else {
                red.push(format!("{lab}: no job (no log, no jobs.tsv row in {}): a label is exactly the one given to ~/bin/jobrun", opts.logs.display()));
            }
            continue;
        };
        let log = log.unwrap_or_default();
        if *rc != 0 {
            red.push(format!("{lab} ended rc={rc}: {} (log {}): fix and rerun under a new label, or move it to SUPERSEDED:", log_digest(&log), log_path.display()));
            continue;
        }
        let rest = lab.to_ascii_lowercase();
        let rest = if !story.is_empty() { rest.strip_prefix(&story).unwrap_or(&rest).to_string() } else { rest };
        if (kind == "plants" || rest.contains("plants")) && !log.lines().any(|l| l.starts_with("plants: copy ")) {
            red.push(format!("{lab}: a plants job that did not run through `cargo xtask plants` (no `plants: copy` line)"));
        }
        if kind == "gate" && !log.lines().any(|l| l.starts_with("affected: ok")) {
            red.push(format!("{lab}: a gate job with rc 0 but no `affected: ok` verdict in its log (an `affected --list`, or a run that stopped early): read the log, rerun with `affected --run`"));
        }
        if is_baseline_label(lab) {
            continue;
        }
        let tested_path = opts.logs.join(".tested").join(lab);
        let tested = read_kv(&tested_path);
        let tree = tested.get("content_tree").cloned().unwrap_or_default();
        if tree.len() != 40 || !tree.bytes().all(|c| c.is_ascii_hexdigit()) {
            notes.push(format!("{lab}: no tested-content record ({}): not judged", tested_path.display()));
            continue;
        }
        if root.join(".git").exists() {
            match git_tested_diff(root, &tree) {
                Some(files) if !files.is_empty() => red.push(stale_msg(lab, &files)),
                Some(_) => {}
                None => notes.push(format!("{lab}: tested tree {tree} not in this checkout: not judged")),
            }
            continue;
        }
        let now_tree = synced.get("content_tree").cloned().unwrap_or_default();
        if now_tree == tree {
            continue;
        }
        let since = fs::metadata(&tested_path).and_then(|m| m.modified());
        match since {
            Ok(since) => {
                let files = stale_files(root, since, base_src(root, opts).as_deref());
                if files.is_empty() {
                    notes.push(format!("{lab}: tested content {} differs from the role's {} only in prose since the job started (handoff/, docs/, records/, results/, *.md): counts", &tree[..9], now_tree.get(..9).unwrap_or("?")));
                } else {
                    red.push(stale_msg(lab, &files));
                }
            }
            Err(e) => red.push(format!("{lab}: tested other content than the role holds and its record has no time ({e})")),
        }
    }
    Verdict::from_problems(red, wait, notes, format!("{} named jobs: ended rc 0, tested the role's content", mine.len()))
}

fn stale_msg(lab: &str, files: &[String]) -> String {
    format!("{lab} tested other content than this head ({}): it ran before your last change or on an older sync; sync your final head, rerun it under a new label and name that (or move it to SUPERSEDED:)", list5(files))
}

/// Exactly the verifier's rule where git is at hand: `git diff --name-only <tree> HEAD` outside prose.
fn git_tested_diff(root: &Path, tree: &str) -> Option<Vec<String>> {
    let o = Command::new("git").current_dir(root).args(["diff", "--name-only", tree, "HEAD", "--", ".", ":!handoff/", ":!docs/", ":!records/", ":!results/", ":!*.md"]).output().ok()?;
    if !o.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&o.stdout).lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect())
}

fn head_verdict(root: &Path, opts: &Opts) -> Verdict {
    let Some(want) = opts.head.as_deref().map(str::trim).filter(|h| h.len() >= 7) else {
        return Verdict { level: Level::Note, lines: vec!["no --head (7+ hex): give the commit you hand back to check the role holds it".into()] };
    };
    let pfx = |a: &str, b: &str| a.starts_with(b) || b.starts_with(a);
    if root.join(".git").exists() {
        let out = |args: &[&str]| Command::new("git").current_dir(root).args(args).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
        let head = out(&["rev-parse", "HEAD"]);
        let dirty = !out(&["status", "--porcelain"]).is_empty();
        let mut red = Vec::new();
        if !pfx(&head, want) {
            red.push(format!("HEAD is {head}, not {want}"));
        }
        if dirty {
            red.push("uncommitted changes: commit them (the hand-back head is a commit)".into());
        }
        return Verdict::from_problems(red, vec![], vec![], format!("HEAD {}", &head[..head.len().min(9)]));
    }
    let s = read_kv(&root.join(".synced"));
    let head = s.get("head").cloned().unwrap_or_default();
    let mut red = Vec::new();
    if !pfx(&head, want) || head.is_empty() {
        red.push(format!("the role holds {} (synced {}), not {want}: sync your final head, then rerun the jobs it changes", if head.is_empty() { "no synced head".into() } else { head.clone() }, s.get("time").cloned().unwrap_or_default()));
    }
    if !s.get("tree").is_some_and(|t| t.ends_with("-clean")) {
        red.push("the role was synced from a worktree with uncommitted changes: commit, sync again".into());
    }
    Verdict::from_problems(red, vec![], vec![], format!("role holds {} (clean)", &head[..head.len().min(9)]))
}

fn clippy_verdict(root: &Path) -> Verdict {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let o = Command::new(&cargo).current_dir(root).args(["clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings"]).output();
    match o {
        Ok(o) if o.status.success() => Verdict::ok(""),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).into_owned();
            let _ = fs::create_dir_all(root.join("target"));
            let _ = fs::write(root.join("target/precheck-clippy.log"), &err);
            let lines: Vec<&str> = err.lines().filter(|l| l.starts_with("error") || l.starts_with("warning") || l.trim_start().starts_with("-->")).take(6).collect();
            Verdict { level: Level::Red, lines: vec![lines.join(" | "), format!("fix: {}", fix("clippy"))] }
        }
        Err(e) => Verdict { level: Level::Red, lines: vec![format!("cannot run {cargo} clippy: {e}")] },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn placeholder_pattern_is_cis() {
        // no spec clause: CI's PLACEHOLDER_RE (built from parts: CI's own check reads this file's lines)
        let n = "NEW";
        for hit in [format!("| COMPAT-{n} |"), format!("id = \"D-{n}\""), format!("SR-{n}"), format!("(D-{n})")] {
            assert!(has_placeholder(&hit), "{hit}");
        }
        for miss in [format!("XD-{n}"), format!("COMPAT-{n}S"), format!("D-{n}ER"), "COMPAT-12".into(), format!("ID-{n}"), n.into()] {
            assert!(!has_placeholder(&miss), "{miss}");
        }
        assert!(docs_unassigned_id("| x | COMPAT-pending |") && docs_unassigned_id("id assigned when it lands"));
        assert!(!docs_unassigned_id("| x | COMPAT-12 |"));
    }

    #[test]
    fn baseline_labels_are_the_dispatchers() {
        // no spec clause: dispatcher _BASELINE_LABEL
        for l in ["x-counts-before", "x-base", "x-percall-base-2", "x_pre.1", "pre-x", "x-baseline"] {
            assert!(is_baseline_label(l), "{l}");
        }
        for l in ["x-gate", "x-prefix", "x-database", "x-basement", "x-plants"] {
            assert!(!is_baseline_label(l), "{l}");
        }
    }

    #[test]
    fn jobs_list_drops_remarks() {
        // no spec clause: the JOBS: line's remarks in parentheses
        assert_eq!(split_jobs("a-gate, b-plants (all caught), otherhost:c"), ["a-gate", "b-plants", "otherhost:c"]);
    }

    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap().flatten() {
            let t = to.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copy_dir(&e.path(), &t);
            } else {
                fs::copy(e.path(), &t).unwrap();
            }
        }
    }

    fn set_all(dir: &Path, t: SystemTime) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            if e.file_type().unwrap().is_dir() {
                set_all(&e.path(), t);
            }
            fs::File::open(e.path()).unwrap().set_modified(t).unwrap();
        }
    }

    /// The fixture `xtask/tests/fixtures/precheck/` laid out as a role and its host's logs: `role/` plus `synced` and
    /// `changed` (the role's `.synced` and `.changed`), `base/` (the merge base's source), `logs/` with `tested/` as
    /// `.tested/`, `jobs` (the JOBS: list), `head`; `newer` lists the role paths a later sync wrote (their time is set
    /// after the jobs started, every other time before).
    fn fixture_run(fixture: &Path) -> Vec<(String, f64, Verdict)> {
        fixture_run_with(fixture, &|_| {}, &[])
    }

    /// [`fixture_run`] with `edit` applied to the copy (its root holds `role/`, `logs/` and `base/`) and `newer`
    /// role paths written after the jobs' tested records.
    fn fixture_run_with(fixture: &Path, edit: &dyn Fn(&Path), newer: &[&str]) -> Vec<(String, f64, Verdict)> {
        let tmp = std::env::temp_dir().join(format!("xtask-precheck-fixture-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = fs::remove_dir_all(&tmp);
        let (role, logs, base) = (tmp.join("role"), tmp.join("logs"), tmp.join("base"));
        copy_dir(&fixture.join("role"), &role);
        copy_dir(&fixture.join("base"), &base);
        copy_dir(&fixture.join("logs"), &logs);
        fs::rename(logs.join("tested"), logs.join(".tested")).unwrap();
        fs::copy(fixture.join("synced"), role.join(".synced")).unwrap();
        fs::copy(fixture.join("changed"), role.join(".changed")).unwrap();
        // The fixture writes a placeholder as `{{NEW}}` (CI's placeholder check reads the tree's own lines).
        let mut files = Vec::new();
        walk_files(&role, &mut files);
        for f in files {
            let t = fs::read_to_string(&f).unwrap();
            if t.contains("{{NEW}}") {
                fs::write(&f, t.replace("{{NEW}}", "NEW")).unwrap();
            }
        }
        edit(&tmp);
        let now = SystemTime::now();
        let (t0, t1, t2) = (now - Duration::from_secs(3 * 3600), now - Duration::from_secs(2 * 3600), now - Duration::from_secs(3600));
        set_all(&tmp, t0);
        set_all(&logs.join(".tested"), t1);
        let listed = fs::read_to_string(fixture.join("newer")).unwrap();
        for p in listed.lines().map(str::trim).filter(|l| !l.is_empty()).chain(newer.iter().copied()) {
            fs::File::open(role.join(p)).unwrap().set_modified(t2).unwrap();
        }
        let opts = Opts {
            jobs: split_jobs(&fs::read_to_string(fixture.join("jobs")).unwrap()),
            head: Some(fs::read_to_string(fixture.join("head")).unwrap().trim().to_string()),
            only: Some(["placeholders", "train-owned", "jobs", "head"].iter().map(|s| s.to_string()).collect()),
            base: "origin/next".into(),
            logs,
            base_src: Some(base),
            host: "fixturehost".into(),
            story: None,
        };
        let r = run_checks(&role, &opts);
        let _ = fs::remove_dir_all(&tmp);
        r
    }

    /// The six red classes of the hand-back (HANDBACK-INLOOP-CHECKS): the fixture is a hand-back that passes every
    /// one (two jobs ended rc 0 on the content the role holds, with a prose-only commit since; a pending record; a
    /// handoff naming a placeholder), so this test is green; each plant in handoff/HANDBACK-INLOOP-CHECKS-plants.txt
    /// puts one class into the fixture and the verdict names it.
    #[test]
    fn precheck_fixture_hand_back_is_green() {
        // no spec clause: the ops hand-back check's rules (ops/handback.py steps a, a2, b; ci.py placeholders)
        let fx = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/precheck");
        let r = fixture_run(&fx);
        let lines: Vec<String> = r.iter().flat_map(|(n, _, v)| v.lines.iter().map(move |l| format!("{n} {:?}: {l}", v.level))).collect();
        println!("{}", lines.join("\n"));
        let bad: Vec<&String> = lines.iter().filter(|l| l.contains(" Red: ") || l.contains(" Wait: ")).collect();
        assert!(bad.is_empty(), "precheck fixture not green:\n{}", bad.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
        assert_eq!(r.len(), 4, "placeholders, train-owned, jobs, head");
        assert!(lines.iter().any(|l| l.contains("pending record records/pending/")), "the pending record is a note");
        assert!(lines.iter().any(|l| l.contains("only in prose since")), "the prose-only commit after the jobs is told apart");
    }

    #[test]
    fn stubs_rewrite_is_red_even_when_stubs_passes() {
        // no spec clause: HANDBACK-INLOOP-CHECKS. The FULL gate runs `stubs` first and tests what it writes, so a
        // stub file or spec/clauses.tsv it rewrites (same length, other text included) is a red with its fix.
        let tmp = std::env::temp_dir().join(format!("xtask-precheck-stubs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("spec")).unwrap();
        fs::create_dir_all(tmp.join("crates/k/tests")).unwrap();
        fs::write(tmp.join("spec/clauses.tsv"), "id\ttext\n").unwrap();
        fs::write(tmp.join("crates/k/tests/s07_x_clauses.rs"), "// a\n").unwrap();
        let before = stubs_snapshot(&tmp);
        assert!(changed_paths(&before, &stubs_snapshot(&tmp)).is_empty(), "nothing written");
        fs::write(tmp.join("crates/k/tests/s07_x_clauses.rs"), "// b\n").unwrap();
        let rewrote = changed_paths(&before, &stubs_snapshot(&tmp));
        assert_eq!(rewrote, ["crates/k/tests/s07_x_clauses.rs"], "same length, other text");
        let _ = fs::remove_dir_all(&tmp);
        let v = step_verdict("stubs", Ok(()), &rewrote);
        assert!(v.level == Level::Red && v.lines.iter().any(|l| l.contains("stubs rewrote crates/k/tests/s07_x_clauses.rs")) && v.lines.iter().any(|l| l.contains("run `cargo xtask stubs`")), "{:?}", v.lines);
        assert_eq!(step_verdict("stubs", Ok(()), &[]).level, Level::Ok);
        let v = step_verdict("abi-gen-check", Err("stale".into()), &[]);
        assert!(v.level == Level::Red && v.lines.iter().any(|l| l.contains("headers-gen")), "{:?}", v.lines);
    }

    /// One fixture edit: `from` must occur exactly once in `rel` (under the copy's root).
    fn edit_once(root: &Path, rel: &str, from: &str, to: &str) {
        let p = root.join(rel);
        let t = fs::read_to_string(&p).unwrap();
        assert_eq!(t.matches(from).count(), 1, "{rel}: `{from}`");
        fs::write(&p, t.replacen(from, to, 1)).unwrap();
    }

    #[test]
    fn precheck_fixture_names_each_red_class() {
        // no spec clause: HANDBACK-INLOOP-CHECKS. Each red class of the hand-back put into the green fixture is named
        // by its check (ops/handback.py steps a, a2, b): a named job still queued (WAIT), a failed job, a job that
        // tested older code, a gate log without its verdict, a plants job outside `cargo xtask plants`, a placeholder
        // id in a docs row. (The sixth class, a FULL-tier `stubs` red, needs the real tree: the plants file has it.)
        let fx = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/precheck");
        let placeholder = format!("COMPAT-{}", "NEW");
        type Case<'a> = (&'a str, Option<(&'a str, &'a str, &'a str)>, &'a [&'a str], &'a str, Level, &'a str);
        let cases: [Case; 6] = [
            ("queued", Some(("logs/jobs.tsv", "fixture-story-gate\tgate", "fixture-story-other\tgate")), &[], "jobs", Level::Wait, "fixture-story-gate"),
            ("failed", Some(("logs/jobs.tsv", "\t0\t540\t", "\t1\t540\t")), &[], "jobs", Level::Red, "fixture-story-plants ended rc=1"),
            ("stale", None, &["src/a.rs"], "jobs", Level::Red, "src/a.rs"),
            ("undiagnosed gate", Some(("logs/fixture-story-gate.log", "affected: ok", "affected: stopped")), &[], "jobs", Level::Red, "no `affected: ok` verdict"),
            ("plants outside the tool", Some(("logs/fixture-story-plants.log", "plants: copy", "plants: copied")), &[], "jobs", Level::Red, "did not run through `cargo xtask plants`"),
            ("placeholder", Some(("role/docs/differences-from-glibc.md", "COMPAT-12", &placeholder)), &[], "placeholders", Level::Red, "differences-from-glibc.md"),
        ];
        for (what, edit, newer, check, level, msg) in cases {
            let r = fixture_run_with(&fx, &|root| {
                if let Some((rel, from, to)) = edit {
                    edit_once(root, rel, from, to);
                }
            }, newer);
            let (_, _, v) = r.iter().find(|(n, _, _)| n == check).unwrap_or_else(|| panic!("{what}: no {check} verdict"));
            assert_eq!(v.level, level, "{what}: {check}: {:?}", v.lines);
            assert!(v.lines.iter().any(|l| l.contains(msg)), "{what}: {check} does not name `{msg}`: {:?}", v.lines);
            let others: Vec<&String> = r.iter().filter(|(n, _, v)| n != check && v.level >= Level::Wait).map(|(n, _, _)| n).collect();
            assert!(others.is_empty(), "{what}: only {check} changes, not {others:?}");
        }
    }

    #[test]
    fn a_gate_that_left_full_to_the_train_is_green() {
        // no spec clause: GATE-DEV-NO-FULL. A per-fix gate never runs FULL; its log carries the train's line (where
        // the FULL line was) before its verdict, and the hand-back checks take it as an ordinary gate.
        let fx = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/precheck");
        let train = "FULL: the train's (xtask/src/main.rs: xtask (its commands are part of the gate))\naffected: ok";
        let r = fixture_run_with(&fx, &|root| edit_once(root, "logs/fixture-story-gate.log", "affected: ok", train), &[]);
        let (_, _, v) = r.iter().find(|(n, _, _)| n == "jobs").expect("a jobs verdict");
        assert_eq!(v.level, Level::Ok, "{:?}", v.lines);
    }

    #[test]
    fn stale_files_sees_code_and_removed_entries_not_prose() {
        // no spec clause: the role-side form of dispatcher.tested_diff
        let tmp = std::env::temp_dir().join(format!("xtask-precheck-stale-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        for d in ["src/sub", "handoff", "target/debug", "conformance/build"] {
            fs::create_dir_all(tmp.join(d)).unwrap();
        }
        for f in ["src/a.rs", "src/sub/b.rs", "handoff/x.md", "target/debug/y", "README.md", "conformance/build/p", ".gitignore"] {
            fs::write(tmp.join(f), "x").unwrap();
        }
        fs::write(tmp.join(".gitignore"), "/target\n/conformance/build\n__pycache__/\n*.o\n").unwrap();
        let old = SystemTime::now() - Duration::from_secs(7200);
        let since = SystemTime::now() - Duration::from_secs(3600);
        set_all(&tmp, old);
        let newer = |p: &str| fs::File::open(tmp.join(p)).unwrap().set_modified(SystemTime::now()).unwrap();
        for p in ["handoff/x.md", "README.md", "target/debug/y", "conformance/build/p", "handoff"] {
            newer(p);
        }
        assert!(stale_files(&tmp, since, None).is_empty(), "{:?}", stale_files(&tmp, since, None));
        newer("src/sub");
        assert_eq!(stale_files(&tmp, since, None), ["src/sub/"], "without the merge base's source: a newer code directory");
        newer("src/a.rs");
        assert_eq!(stale_files(&tmp, since, None), ["src/a.rs", "src/sub/"]);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn stale_files_tells_a_removed_file_from_a_build_output() {
        // no spec clause: HANDBACK-INLOOP-CHECKS-2. After a gate, a handoff-only sync removed the ignored
        // `conformance/build` and precheck's own fuzz check wrote the nested-ignored `fuzz/target`: both directories
        // are newer, nothing tested changed. A file the merge base has and the role lost is a change.
        let tmp = std::env::temp_dir().join(format!("xtask-precheck-stale-base-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let (r, b) = (tmp.join("r"), tmp.join("b"));
        for d in ["r/src/sub", "r/conformance", "r/fuzz/target/debug", "b/src/sub", "b/conformance", "b/fuzz"] {
            fs::create_dir_all(tmp.join(d)).unwrap();
        }
        for f in ["src/sub/b.rs", "conformance/run.sh", "fuzz/Cargo.toml"] {
            fs::write(r.join(f), "x").unwrap();
            fs::write(b.join(f), "x").unwrap();
        }
        fs::write(r.join("fuzz/target/debug/x"), "x").unwrap();
        fs::write(b.join("src/sub/c.rs"), "x").unwrap();
        fs::write(b.join("conformance/README.md"), "x").unwrap();
        for t in [&r, &b] {
            fs::write(t.join(".gitignore"), "/target\n/conformance/build\n").unwrap();
            fs::write(t.join("fuzz/.gitignore"), "target/\n").unwrap();
        }
        let since = SystemTime::now() - Duration::from_secs(3600);
        set_all(&tmp, SystemTime::now() - Duration::from_secs(7200));
        let newer = |p: &str| fs::File::open(r.join(p)).unwrap().set_modified(SystemTime::now()).unwrap();
        for p in ["conformance", "fuzz", "fuzz/target", "fuzz/target/debug", "fuzz/target/debug/x"] {
            newer(p);
        }
        assert!(stale_files(&r, since, Some(&b)).is_empty(), "build outputs only: {:?}", stale_files(&r, since, Some(&b)));
        assert_eq!(stale_files(&r, since, None), ["conformance/", "fuzz/"], "strict without the base; fuzz/target ignored by fuzz/.gitignore");
        assert_eq!(stale_files(&r, since, Some(&tmp.join("absent"))), ["conformance/", "fuzz/"], "a base not primed is no base");
        newer("src/sub");
        assert_eq!(stale_files(&r, since, Some(&b)), ["src/sub/c.rs (removed)"]);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn role_changes_count_lines_the_base_lacks() {
        // no spec clause: CI's placeholders read added lines only
        let tmp = std::env::temp_dir().join(format!("xtask-precheck-changes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("r/docs")).unwrap();
        fs::create_dir_all(tmp.join("b/docs")).unwrap();
        fs::write(tmp.join("b/docs/x.md"), "old line\n").unwrap();
        fs::write(tmp.join("r/docs/x.md"), "old line\nnew line\n").unwrap();
        let c = Changes::from_role(&tmp.join("r"), "docs/x.md\n", Some(&tmp.join("b")));
        assert_eq!(c.added["docs/x.md"], ["new line"]);
        let c = Changes::from_role(&tmp.join("r"), "docs/x.md\n", None);
        assert_eq!(c.added["docs/x.md"].len(), 2, "no base source: every line counts");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_steps_and_precheck_share_the_cheap_list() {
        // no spec clause: every cheap step has a runner and a fix
        for s in CHEAP_STEPS {
            assert!(!fix(s).is_empty(), "{s}");
        }
        let static_names: BTreeSet<&str> = crate::static_list().iter().map(|(n, _)| *n).collect();
        for (s, c) in STATIC_SAME_AS_CHEAP {
            assert!(static_names.contains(s) && CHEAP_STEPS.contains(c), "{s} / {c}");
        }
    }
}
