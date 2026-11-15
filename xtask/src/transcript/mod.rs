//! `cargo xtask transcript`: the utilities' transcript harness (plan/utilities.md UTIL-1, docs/transcripts.md).
//!
//! A transcript (`validation/transcripts/<utility>/*.transcript`) gives one case of a utility: argv, environment,
//! stdin, the private root's files, and the outcome the reference gave when it was recorded. `check` runs Liella's
//! command and the reference (when it is installed) in private roots of their own and compares stdout, stderr, exit
//! status and the files written: Liella against the reference, and the reference against the record (a stale record
//! is a failure too). Without the reference, Liella is compared against the record and the run says so. `record`
//! runs the reference and writes its outcome into the transcript.
//!
//! It lives in xtask (std only, no third-party code, already linked with `liella-ld` for the cache writer and reader)
//! rather than as scripts under `validation/`: the comparator is Rust with unit tests, which the plants check, and
//! every utility story runs it the same way.

mod compare;
mod format;
mod run;
mod stub;

use format::{Namespace, Outcome, Transcript};
use run::Side;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask transcript (check | record) <file.transcript | dir>... [--reference <command>=<path>]... [--liella <command>=<path>]...";

/// Dispatch that must happen before the tree root is looked up: the stub (argv[0] `ldconfig`) and the namespace
/// helper both run with the private root as their working directory.
pub(crate) fn early() -> Option<ExitCode> {
    let mut args = std::env::args_os();
    let argv0 = args.next()?;
    let rest: Vec<OsString> = args.collect();
    if Path::new(&argv0).file_name().is_some_and(|n| n == "ldconfig") {
        return Some(stub::ldconfig(&argv0, &rest));
    }
    if rest.first().is_some_and(|a| a == run::EXEC_HELPER) {
        return Some(run::exec_helper(&rest[1..]));
    }
    None
}

/// The reference's path for a command: the §31.1 locations (`/usr/sbin`, then `/usr/bin`).
fn reference_default(command: &str) -> Option<PathBuf> {
    ["/usr/sbin", "/usr/bin"].iter().map(|d| Path::new(d).join(command)).find(|p| p.is_file())
}

/// Liella's program for a command: this binary as the stub for `ldconfig` (until UTIL-3); none for the others yet.
fn liella_default(command: &str) -> Option<PathBuf> {
    match command {
        "ldconfig" => std::env::current_exe().ok(),
        _ => None,
    }
}

/// `*.transcript` files: each file operand, and every one under each directory operand (sorted by path).
fn collect(root: &Path, operands: &[String]) -> Result<Vec<PathBuf>, String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        for e in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let p = e.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            if p.is_dir() {
                walk(&p, out)?;
            } else if p.extension().is_some_and(|x| x == "transcript") {
                out.push(p);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    for o in operands {
        let p = root.join(o);
        if p.is_dir() {
            let mut v = Vec::new();
            walk(&p, &mut v)?;
            v.sort();
            if v.is_empty() {
                return Err(format!("{o}: no *.transcript files"));
            }
            out.extend(v);
        } else if p.is_file() {
            out.push(p);
        } else {
            return Err(format!("{o}: no such file or directory"));
        }
    }
    Ok(out)
}

/// A fresh scratch directory under `$TMPDIR` for one transcript (removed by the caller).
fn scratch() -> Result<PathBuf, String> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    loop {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("liella-transcript-{}-{n}", std::process::id()));
        match std::fs::create_dir(&d) {
            Ok(()) => return Ok(d),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", d.display())),
        }
    }
}

/// Run one side of `t` in its own scratch directory.
fn run_one(t: &Transcript, label: &str, program: PathBuf) -> Result<Outcome, String> {
    let dir = scratch()?;
    let r = run::run_side(t, &Side { label, program }, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    r
}

/// What one `check` of a transcript found.
enum Verdict {
    Ok(String),
    Mismatch(Vec<String>),
    Skipped(String),
}

fn check_one(t: &Transcript, reference: Option<PathBuf>, liella: PathBuf) -> Result<Verdict, String> {
    if t.namespace != Namespace::None {
        if let Err(why) = run::namespaces() {
            return Ok(Verdict::Skipped(format!("needs user and mount namespaces: {why}")));
        }
    }
    let Some(record) = &t.expected else { return Err("no recorded outcome: run `cargo xtask transcript record` on it".into()) };
    let ours = run_one(t, "liella", liella)?;
    let mut lines = Vec::new();
    let note = match reference {
        Some(path) => {
            let theirs = run_one(t, "reference", path.clone())?;
            lines.extend(compare::compare("reference", &theirs, "liella", &ours).iter().map(ToString::to_string));
            let stale = compare::compare("record", record, "reference", &theirs);
            if !stale.is_empty() {
                lines.push(format!("the record is stale: {} gives another outcome than the record (re-record it after checking why)", path.display()));
                lines.extend(stale.iter().map(ToString::to_string));
            }
            format!("against {}", path.display())
        }
        None => {
            lines.extend(compare::compare("record", record, "liella", &ours).iter().map(ToString::to_string));
            format!("against the record only: the reference `{}` is not installed here", t.command)
        }
    };
    Ok(if lines.is_empty() { Verdict::Ok(note) } else { Verdict::Mismatch(lines) })
}

/// `cargo xtask transcript ...`.
pub(crate) fn main(root: &Path, args: &[String]) -> ExitCode {
    let usage = |m: &str| {
        eprintln!("transcript: {m}\n{USAGE}");
        ExitCode::from(2)
    };
    let Some(mode) = args.first().map(String::as_str) else { return usage("no mode") };
    if mode != "check" && mode != "record" {
        return usage(&format!("unknown mode {mode:?}"));
    }
    let mut operands = Vec::new();
    let mut refs: Vec<(String, PathBuf)> = Vec::new();
    let mut ours: Vec<(String, PathBuf)> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if a == "--reference" || a == "--liella" {
            let Some((c, p)) = args.get(i + 1).and_then(|v| v.split_once('=')) else { return usage(&format!("{a} <command>=<path>")) };
            let list = if a == "--reference" { &mut refs } else { &mut ours };
            list.push((c.to_string(), PathBuf::from(p)));
            i += 2;
            continue;
        }
        if a.starts_with('-') {
            return usage(&format!("unknown option {a}"));
        }
        operands.push(a.clone());
        i += 1;
    }
    if operands.is_empty() {
        return usage("no transcripts named");
    }
    let files = match collect(root, &operands) {
        Ok(f) => f,
        Err(e) => return usage(&e),
    };
    let pick = |list: &[(String, PathBuf)], c: &str| list.iter().rev().find(|(n, _)| n == c).map(|(_, p)| p.clone());
    let (mut ok, mut bad, mut skipped, mut errors) = (0, 0, 0, 0);
    for f in &files {
        let shown = f.strip_prefix(root).unwrap_or(f).display().to_string();
        let t = match std::fs::read_to_string(f).map_err(|e| e.to_string()).and_then(|s| format::parse(&s, &shown)) {
            Ok(t) => t,
            Err(e) => {
                println!("ERROR {shown}: {e}");
                errors += 1;
                continue;
            }
        };
        let reference = pick(&refs, &t.command).or_else(|| reference_default(&t.command));
        if mode == "record" {
            let Some(path) = reference else {
                println!("ERROR {shown}: the reference `{}` is not installed here", t.command);
                errors += 1;
                continue;
            };
            match run_one(&t, "reference", path) {
                Ok(o) => {
                    let text = format!("{}{}", t.head, format::render_outcome(&o));
                    if let Err(e) = std::fs::write(f, text) {
                        println!("ERROR {shown}: {e}");
                        errors += 1;
                    } else {
                        println!("recorded {shown}: {}", o.status);
                        ok += 1;
                    }
                }
                Err(e) => {
                    println!("ERROR {shown}: {e}");
                    errors += 1;
                }
            }
            continue;
        }
        let Some(liella) = pick(&ours, &t.command).or_else(|| liella_default(&t.command)) else {
            println!("ERROR {shown}: Liella has no `{}` yet (pass --liella {}=<path>)", t.command, t.command);
            errors += 1;
            continue;
        };
        match check_one(&t, reference, liella) {
            Ok(Verdict::Ok(note)) => {
                println!("ok {shown} ({note})");
                ok += 1;
            }
            Ok(Verdict::Mismatch(lines)) => {
                println!("MISMATCH {shown} ({})", t.name);
                for l in lines {
                    println!("  {}", l.replace('\n', "\n  "));
                }
                bad += 1;
            }
            Ok(Verdict::Skipped(why)) => {
                println!("SKIPPED {shown}: {why}");
                skipped += 1;
            }
            Err(e) => {
                println!("ERROR {shown}: {e}");
                errors += 1;
            }
        }
    }
    let done = if mode == "record" { "recorded" } else { "ok" };
    println!("transcript {mode}: {ok} {done}, {bad} mismatched, {skipped} skipped, {errors} errors ({} files)", files.len());
    if bad + errors > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
}
