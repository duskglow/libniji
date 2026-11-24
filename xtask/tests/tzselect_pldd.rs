//! UTIL-TZSELECT-PLDD: libniji's `tzselect` and `pldd` (`cargo xtask so` test links under target/niji-test) against
//! the reference programs, run black-box on the host, and the `/usr/bin/ld.so` rows of the package manifest.
//!
//! * tzselect: scripted sessions (every continent, junk answers, every probed TZ string, No then Yes, the coord
//!   menu, a locale without UTF-8, narrow and wide terminals, options, missing tables) give the same exit status,
//!   standard output and standard error as the reference, the two clock lines of the information block masked. Every
//!   session ends on a confirmation or at a continent or country menu. `-c`: the reference's zones with every zone
//!   listed, in increasing great-circle distance (D-31.34), checked against a haversine computed here. End of input
//!   at every prompt of a session: status 1 and no value (libniji only).
//! * pldd: on a traceable process with dlopen'd objects and a second thread, the reference's listing; the errors
//!   (no operand, two, not a number, no such process, not traceable), `--usage` and option errors the same.
//! * The manifest: both flavours ship `usr/bin/ld.so` as a link that resolves to the loader the flavour ships.
//!
//! Trace: s31_11_c_1, s31_11_i_1, s31_11_i_2, s31_11_g_1, s31_11_f_1, s31_13_c_1, s31_13_i_1, s31_13_g_1,
//! s31_13_f_1, s31_13_f_2, s31_13_f_3.

#![allow(clippy::disallowed_methods)] // a host tool's test: S8 governs the library crates only

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

mod test_link;

const REF_TZSELECT: &str = "/usr/bin/tzselect";
const REF_PLDD: &str = "/usr/bin/pldd";
const ZONEINFO: &str = "/usr/share/zoneinfo";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root").to_path_buf()
}

/// libniji's test link of `name`, or None (with the reason printed) when it or a file the comparison needs is missing.
fn ours(name: &str, needs: &[&str]) -> Option<PathBuf> {
    let p = match test_link::usable(&root().join("target/niji-test").join(name)) {
        Ok(p) => p,
        Err(why) => {
            println!("SKIPPED: {why}");
            return None;
        }
    };
    for f in needs {
        if !Path::new(f).exists() {
            println!("SKIPPED: {f} is not installed on this host (needed to compare with the reference)");
            return None;
        }
    }
    Some(p)
}

#[derive(Debug, PartialEq)]
struct Run {
    rc: i32,
    out: String,
    err: String,
}

fn finish(mut child: Child, input: &str) -> Run {
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes()); // the program may end before reading it all
    }
    let o = child.wait_with_output().expect("wait for the program");
    Run { rc: o.status.code().unwrap_or(-1), out: String::from_utf8_lossy(&o.stdout).into_owned(), err: String::from_utf8_lossy(&o.stderr).into_owned() }
}

/// One tzselect session: `prog` with `args`, a small fixed environment plus `env`, `input` on standard input.
fn session(prog: &Path, args: &[&str], env: &[(&str, &str)], input: &str) -> Run {
    let mut c = Command::new(prog);
    // Messages name the program as it was run: the reference names its path.
    c.arg0(REF_TZSELECT).args(args).env_clear().env("PATH", "/usr/bin:/bin").env("HOME", "/tmp").env("LANG", "C.UTF-8");
    for (k, v) in env {
        c.env(k, v);
    }
    let child = c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("run tzselect");
    let mut r = finish(child, input);
    // The two clock lines of the information block differ between any two runs.
    r.err = r
        .err
        .lines()
        .map(|l| match l.split_once('\t') {
            Some((head @ ("Selected time is now:" | "Universal Time is now:"), _)) => head.to_string(),
            _ => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    r
}

fn same(what: &str, ours: &Run, reference: &Run) -> Result<(), String> {
    if ours == reference {
        return Ok(());
    }
    let first = ours.err.lines().zip(reference.err.lines()).position(|(a, b)| a != b);
    Err(format!(
        "{what}: status {} (reference {}), stdout {:?} (reference {:?}), stderr first differs at line {first:?}: {:?} vs {:?}",
        ours.rc,
        reference.rc,
        ours.out,
        reference.out,
        first.and_then(|i| ours.err.lines().nth(i)),
        first.and_then(|i| reference.err.lines().nth(i))
    ))
}

/// The number of the continent menu entry whose text starts `what` (the menu shown one entry per line).
fn entry(reference: &Path, what: &str) -> usize {
    let r = session(reference, &[], &[("COLUMNS", "1")], "");
    r.err
        .lines()
        .find_map(|l| l.trim_start().split_once(") ").filter(|(_, t)| t.starts_with(what)).and_then(|(n, _)| n.parse().ok()))
        .unwrap_or_else(|| panic!("no menu entry {what:?} in the reference's first menu:\n{}", r.err))
}

/// Answers that end any session on a confirmation: "1" picks the first entry of every menu and Yes.
const ONES: &str = "1\n1\n1\n1\n1\n1\n1\n1\n";

/// A scripted session: its name, the arguments, the environment and the input.
type Case<'a> = (String, Vec<&'a str>, Vec<(&'a str, &'a str)>, String);

/// TZ strings probed against the reference, accepted and refused ones.
const TZ_STRINGS: &[&str] = &[
    "EST5EDT,M3.2.0,M11.1.0", ":Europe/Paris", "AB5", "ABCDEFGHIJ5", "<A>5", "EST+5:30", "EST5:30:59", "EST25", "EST-24:59:59",
    "EST5EDT,M3.2.0/-1,M11.1.0/167", "EST5EDT,M13.1.0,M11.1.0", "UTC0", "<+0530>-5:30", "EST5EDT,M3.5.6,M11.1.0",
    "EST5EDT,M3.6.0,M11.1.0", "EST5EDT,J0,J365", "EST5EDT,J366,J300", "EST5EDT,365,0", "EST5EDT,366,0", "EST5EDT,M3.2.7,M11.1.0",
    "EST5EDT,M3.2.0/24:59:59,M11.1.0", "EST5EDT,M3.2.0/168,M11.1.0", "JST-9", "EST5EDT,M3.2.0", "EST5EDT6", "EST5EDT-26", "EST5:60",
    "EST5:5", "EST 5", "<A-1>5", "<ABC", "EST5EDT,M3.2.0,M11.1.0,", "EST05", "foo", "EST005", "<123>4", "EST",
    "EST5EDT4,M3.2.0/2,M11.1.0/2", "EST5EDT,J60,J300", "EST5EDT,60,300", "Europe/Paris",
];

/// s31_11_c_1, s31_11_i_2, s31_11_g_1: the interactive sessions match the reference's.
#[test]
fn tzselect_sessions_match_the_reference() {
    let Some(prog) = ours("tzselect", &[REF_TZSELECT, ZONEINFO]) else { return };
    let reference = Path::new(REF_TZSELECT);
    let (coord, tz) = (entry(reference, "coord - "), entry(reference, "TZ - "));
    let continents = coord - 1;
    let mut cases: Vec<Case> = Vec::new();
    for k in 1..=continents {
        cases.push((format!("continent {k}"), vec![], vec![], format!("{k}\n{ONES}")));
    }
    let bad = tz + 1;
    cases.push(("junk answers".into(), vec![], vec![], format!("0\n{bad}\nabc\n\n1.\n0x1\n1e0\n-1\n1a\n1 2\n 2 \n0\nabc\n+01\n{ONES}")));
    cases.push(("blanks and zeros".into(), vec![], vec![], format!("\t+0002\n 001\n{ONES}")));
    for s in TZ_STRINGS {
        // Refused: UTC0 is taken next; accepted: "UTC0" is a wrong menu answer, then Yes.
        cases.push((format!("TZ {s:?}"), vec![], vec![], format!("{tz}\n{s}\nUTC0\n1\n")));
    }
    cases.push(("TZ, No, then TZ and Yes".into(), vec![], vec![], format!("{tz}\nAEST-10\n2\n{tz}\nJST-9\n1\n")));
    cases.push(("LANG=C, Africa".into(), vec![], vec![("LANG", "C")], "1\n".into()));
    for cols in ["40", "200", "abc", "0", "-5"] {
        cases.push((format!("COLUMNS={cols}"), vec![], vec![("COLUMNS", cols)], "2\n".into()));
    }
    cases.push(("TZ in the environment".into(), vec![], vec![("TZ", "Asia/Tokyo")], ONES.to_string()));
    cases.push(("-x".into(), vec!["-x"], vec![], String::new()));
    cases.push(("-c without its value".into(), vec!["-c"], vec![], String::new()));
    cases.push(("an operand".into(), vec!["extra"], vec![], String::new()));
    cases.push(("--".into(), vec!["--"], vec![], format!("1\n{ONES}")));
    let (mut errors, mut finished) = (Vec::new(), 0);
    for (what, args, env, input) in &cases {
        let (a, b) = (session(&prog, args, env, input), session(reference, args, env, input));
        if let Err(e) = same(what, &a, &b) {
            errors.push(e);
        }
        finished += usize::from(a.rc == 0 && !a.out.trim().is_empty());
    }
    // A finished session prints the value; most of the cases above finish.
    assert!(finished > continents + TZ_STRINGS.len(), "only {finished} sessions printed a value");
    assert!(errors.is_empty(), "{} of {} sessions differ:\n{}", errors.len(), cases.len(), errors.join("\n"));
}

/// s31_11_i_1: --help (processed before a later bad option) is the reference's text without its bug-report line;
/// --version names libniji.
#[test]
fn tzselect_help_and_version() {
    let Some(prog) = ours("tzselect", &[REF_TZSELECT]) else { return };
    let (a, b) = (session(&prog, &["--help", "-x"], &[], ""), session(Path::new(REF_TZSELECT), &["--help", "-x"], &[], ""));
    assert_eq!((a.rc, b.rc), (0, 0));
    let rest = b.out.strip_prefix(a.out.as_str()).unwrap_or_else(|| panic!("ours is the reference's start:\n{}\n{}", a.out, b.out));
    assert!(rest.trim().is_empty() || rest.contains("bugs"), "only the bug-report line is left out: {rest:?}");
    let v = session(&prog, &["--version"], &[], "");
    assert_eq!((v.rc, v.out.as_str()), (0, "tzselect (libniji) 2.39\n"));
}

/// The missing-tables message: the reference's, without its script line number (differences row).
#[test]
fn tzselect_without_tables() {
    let Some(prog) = ours("tzselect", &[REF_TZSELECT]) else { return };
    let env = [("TZDIR", "/nonexistent-tzdir")];
    let (a, b) = (session(&prog, &[], &env, ""), session(Path::new(REF_TZSELECT), &[], &env, ""));
    let unnumbered: String = b
        .err
        .lines()
        .map(|l| match l.split_once(": line ") {
            Some((head, rest)) => format!("{head}: {}", rest.split_once(": ").map_or(rest, |(_, t)| t)),
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!((a.rc, a.out.as_str(), a.err.as_str()), (b.rc, b.out.as_str(), unnumbered.as_str()));
    assert_eq!(a.rc, 1);
}

/// One zone of zone1970.tab: its coordinates (degrees) and its -c label.
fn zones() -> Vec<(f64, f64, String)> {
    let iso = fs::read_to_string(Path::new(ZONEINFO).join("iso3166.tab")).expect("iso3166.tab");
    let names: Vec<(&str, &str)> = iso.lines().filter(|l| !l.starts_with('#')).filter_map(|l| l.split_once('\t')).collect();
    let tab = fs::read_to_string(Path::new(ZONEINFO).join("zone1970.tab")).expect("zone1970.tab");
    let mut out = Vec::new();
    for l in tab.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 3 {
            continue;
        }
        let label = f[0]
            .split(',')
            .map(|c| if c == "US" { c } else { names.iter().find(|(k, _)| *k == c).map_or(c, |(_, n)| n) })
            .collect::<Vec<_>>()
            .join(", ");
        let label = match f.get(3) {
            Some(comment) if !comment.is_empty() => format!("{label} - {comment}"),
            _ => label,
        };
        let (lat, lon) = iso6709(f[1]).expect("zone1970.tab coordinates");
        out.push((lat, lon, label));
    }
    out
}

/// ISO 6709 +-DD[MM[SS]][.f]+-DDD[MM[SS]][.f] in degrees.
fn iso6709(s: &str) -> Option<(f64, f64)> {
    let split = s[1..].find(['+', '-'])? + 1;
    let part = |p: &str, deg: usize| -> Option<f64> {
        let sign = if p.starts_with('-') { -1.0 } else { 1.0 };
        let (int, frac) = p[1..].split_once('.').map_or((&p[1..], ""), |(a, b)| (a, b));
        let mut v: f64 = int.get(..deg)?.parse().ok()?;
        let mut unit = 1.0;
        let mut rest = &int[deg..];
        while !rest.is_empty() {
            unit /= 60.0;
            v += rest.get(..2)?.parse::<f64>().ok()? * unit;
            rest = &rest[2..];
        }
        if !frac.is_empty() {
            v += format!("0.{frac}").parse::<f64>().ok()? * unit;
        }
        Some(sign * v)
    };
    Some((part(&s[..split], 2)?, part(&s[split..], 3)?))
}

/// The great-circle distance (radians) by the haversine formula.
fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    let r = std::f64::consts::PI / 180.0;
    let h = ((b.0 - a.0) * r / 2.0).sin().powi(2) + (a.0 * r).cos() * (b.0 * r).cos() * ((b.1 - a.1) * r / 2.0).sin().powi(2);
    2.0 * h.min(1.0).sqrt().asin()
}

/// The menu entries of a one-entry-per-line menu (COLUMNS=1) after its header line.
fn entries(err: &str, header: &str) -> Vec<String> {
    let mut lines = err.lines().skip_while(|l| !l.starts_with(header)).skip(1);
    let mut out = Vec::new();
    for l in lines.by_ref() {
        match l.trim_start().split_once(") ") {
            Some((n, t)) if n.parse::<usize>().is_ok() => out.push(t.to_string()),
            _ => break,
        }
    }
    out
}

/// s31_11_i_1, s31_11_g_1: -c lists the reference's zones, nearest first (D-31.34); -n bounds the list.
#[test]
fn tzselect_coordinates_list_nearest_first() {
    let Some(prog) = ours("tzselect", &[REF_TZSELECT, ZONEINFO]) else { return };
    let all = zones();
    let header = "Please select one of the following timezones, echo listed roughly";
    for coord in ["+4852+00220", "-35-058", "+3541+13945", "-4852-17740", "+48.85+002.35", "+485200+0022000", "+90+000", "-1755+17725"] {
        let here = iso6709(coord).expect("test coordinate");
        let n = all.len().to_string();
        let args = ["-c", coord, "-n", n.as_str()];
        let a = session(&prog, &args, &[("COLUMNS", "1")], "");
        let listed = entries(&a.err, header);
        assert_eq!(listed.len(), all.len(), "-c {coord}: every zone listed\n{}", a.err);
        let mut want: Vec<&str> = all.iter().map(|z| z.2.as_str()).collect();
        let mut got: Vec<&str> = listed.iter().map(String::as_str).collect();
        want.sort_unstable();
        got.sort_unstable();
        assert_eq!(got, want, "-c {coord}: the labels of zone1970.tab");
        let dist = |label: &str| all.iter().filter(|z| z.2 == label).map(|z| distance(here, (z.0, z.1))).fold(f64::INFINITY, f64::min);
        for w in listed.windows(2) {
            assert!(dist(&w[0]) <= dist(&w[1]) + 1e-12, "-c {coord}: {:?} ({}) listed before {:?} ({})", w[0], dist(&w[0]), w[1], dist(&w[1]));
        }
        assert_eq!((a.rc, a.out.as_str()), (1, "\n"), "-c {coord}: end of input at the menu");
        // The reference lists the same zones (its order differs: differences row "tzselect -c order").
        if coord == "+4852+00220" {
            let b = session(Path::new(REF_TZSELECT), &args, &[("COLUMNS", "1")], "");
            let mut theirs = entries(&b.err, header);
            theirs.sort_unstable();
            assert_eq!(got, theirs.iter().map(String::as_str).collect::<Vec<_>>(), "-c {coord}: the reference's zones");
        }
    }
    // The nearest zone chosen, through -c and through the coord menu entry: the reference's value.
    let coord = entry(Path::new(REF_TZSELECT), "coord - ");
    for (args, input) in [(vec!["-c", "+4852+00220"], "1\n1\n".to_string()), (vec![], format!("{coord}\n+4852+00220\n1\n1\n"))] {
        let (a, b) = (session(&prog, &args, &[], &input), session(Path::new(REF_TZSELECT), &args, &[], &input));
        assert_eq!((a.rc, a.out.as_str()), (b.rc, b.out.as_str()), "{args:?} {input:?}");
        assert_eq!(a.out, "Europe/Paris\n");
    }
    for (limit, n) in [("3", 3), ("1", 1), ("010", 10)] {
        let a = session(&prog, &["-c", "-35-058", "-n", limit], &[("COLUMNS", "1")], "");
        assert_eq!(entries(&a.err, header).len(), n, "-n {limit}");
    }
    for limit in ["0", "abc", "-3", "2x"] {
        let a = session(&prog, &["-c", "+4852+00220", "-n", limit], &[], "");
        assert_eq!(a.rc, 1, "-n {limit}: a usage error (D-31.34)");
        assert!(a.err.contains("LIMIT must be a positive number"), "-n {limit}: {}", a.err);
    }
}

/// s31_11_f_1: the end of input at any prompt of a session ends it with status 1 and no value.
#[test]
fn tzselect_end_of_input_at_every_prompt() {
    let Some(prog) = ours("tzselect", &[ZONEINFO]) else { return };
    let (coord, tz) = (entry(&prog, "coord - "), entry(&prog, "TZ - "));
    let mut sessions: Vec<String> = (1..coord).map(|k| format!("{k}\n{ONES}")).collect();
    sessions.push(format!("{tz}\n\nfoo\nUTC0\n2\n{coord}\n+4852+00220\n1\n1\n"));
    sessions.push(format!("{coord}\nhello\n\n1\n1\n"));
    let mut zone_menus = 0;
    for full in &sessions {
        let lines: Vec<&str> = full.split_inclusive('\n').collect();
        for cut in 0..lines.len() {
            let input: String = lines[..cut].concat();
            let r = session(&prog, &[], &[], &input);
            if r.rc == 0 {
                break; // confirmed: the rest of the session is not read
            }
            assert_eq!(r.rc, 1, "input {input:?}");
            assert!(r.out.is_empty() || r.out == "\n", "input {input:?}: printed {:?}", r.out);
            if r.err.rsplit("Please select").next().is_some_and(|t| t.starts_with(" one of the following timezones.")) {
                zone_menus += 1;
            }
        }
        let r = session(&prog, &[], &[], full);
        assert_eq!(r.rc, 0, "session {full:?} confirms");
    }
    assert!(zone_menus > 0, "no session ended at a zone menu");
}

// ---- pldd ----

const TARGET_C: &str = r#"
#include <dlfcn.h>
#include <pthread.h>
#include <stdio.h>
#include <sys/prctl.h>
#include <unistd.h>
static void *idle(void *a) { (void)a; for (;;) pause(); return 0; }
int main(int argc, char **argv) {
    prctl(PR_SET_PTRACER, PR_SET_PTRACER_ANY, 0, 0, 0);
    for (int i = 1; i < argc; i++)
        if (!dlopen(argv[i], RTLD_NOW)) { fprintf(stderr, "%s\n", dlerror()); return 1; }
    pthread_t t;
    pthread_create(&t, 0, idle, 0);
    printf("%d\n", (int)getpid());
    fflush(stdout);
    char b[64];
    while (read(0, b, sizeof b) > 0) {}
    return 0;
}
"#;

fn pldd(prog: &Path, args: &[&str]) -> Run {
    // Both run as `pldd`: messages begin with the name the program was run by.
    finish(Command::new(prog).arg0("pldd").args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("run pldd"), "")
}

/// A running target that waits for the end of its standard input: the child and its process id.
fn target(exe: &Path, args: &[&str]) -> (Child, String) {
    let mut child = Command::new(exe).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("run the target");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("target stdout")).read_line(&mut line).expect("target pid");
    (child, line.trim().to_string())
}

fn build_target(dir: &Path, extra: &[&str]) -> Result<PathBuf, String> {
    let src = dir.join("target.c");
    fs::write(&src, TARGET_C).map_err(|e| e.to_string())?;
    let exe = dir.join(if extra.is_empty() { "target" } else { "target-static" });
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let o = Command::new(&cc).args(extra).arg("-pthread").arg("-o").arg(&exe).arg(&src).output().map_err(|e| format!("{cc}: {e}"))?;
    if !o.status.success() {
        return Err(format!("{cc} {extra:?}: {}", String::from_utf8_lossy(&o.stderr)));
    }
    Ok(exe)
}

/// s31_13_c_1, s31_13_g_1, s31_13_f_*, s31_13_i_1: pldd lists what the reference lists, dlopen'd objects included,
/// and fails as the reference fails.
#[test]
fn pldd_matches_the_reference() {
    let Some(prog) = ours("pldd", &[REF_PLDD]) else { return };
    let scope = fs::read_to_string("/proc/sys/kernel/yama/ptrace_scope").unwrap_or_default();
    if scope.trim().parse::<u32>().unwrap_or(0) >= 2 {
        println!("SKIPPED: kernel.yama.ptrace_scope is {} here: no process may be traced by a non-parent", scope.trim());
        return;
    }
    let dir = std::env::temp_dir().join(format!("niji-pldd-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch directory");
    let exe = match build_target(&dir, &[]) {
        Ok(e) => e,
        Err(why) => {
            println!("SKIPPED: the target program does not build here: {why}");
            return;
        }
    };
    let (mut child, pid) = target(&exe, &["libm.so.6", "libresolv.so.2"]);
    let mut errors = Vec::new();
    let mut check = |what: &str, args: &[&str]| -> Run {
        let (a, b) = (pldd(&prog, args), pldd(Path::new(REF_PLDD), args));
        if a != b {
            errors.push(format!("{what} {args:?}: ours {a:?}\nreference {b:?}"));
        }
        a
    };
    let listing = check("listing", &[&pid]);
    for form in [format!("+{pid}"), format!(" {pid}"), format!("0{pid}")] {
        check("process id form", &[&form]);
    }
    let self_pid = std::process::id().to_string();
    check("the caller's parent", &[&self_pid]);
    check("no operand", &[]);
    check("two operands", &[&pid, &pid]);
    check("not a number", &["abc"]);
    check("negative", &["-5"]);
    check("process 0", &["0"]);
    check("no such process", &["999999999"]);
    check("--usage", &["--usage"]);
    check("bad short option", &["-x"]);
    check("bad long option", &["--bogus"]);
    // Process 1 refuses a tracer that is not root (root may trace it).
    let root_user = fs::read_to_string("/proc/self/status").unwrap_or_default().lines().any(|l| l.starts_with("Uid:") && l.split_whitespace().nth(2) == Some("0"));
    if !root_user {
        check("process 1", &["1"]);
    }
    drop(child.stdin.take());
    let _ = child.wait();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    assert_eq!(listing.rc, 0, "{listing:?}");
    let lines: Vec<&str> = listing.out.lines().collect();
    assert!(lines.first().is_some_and(|l| l.starts_with(&format!("{pid}:\t"))), "{listing:?}");
    for lib in ["libm.so.6", "libresolv.so.2"] {
        assert!(lines.iter().any(|l| l.ends_with(&format!("/{lib}"))), "dlopen'd {lib} listed: {listing:?}");
    }
    // A static program has no rendezvous: nothing listed, status 1, as the reference.
    match build_target(&dir, &["-static"]) {
        Ok(stat) => {
            let (mut child, pid) = target(&stat, &[]);
            let (a, b) = (pldd(&prog, &[&pid]), pldd(Path::new(REF_PLDD), &[&pid]));
            drop(child.stdin.take());
            let _ = child.wait();
            assert_eq!(a, b, "static target");
        }
        Err(why) => println!("SKIPPED (static target only): {why}"),
    }
    let _ = fs::remove_dir_all(&dir);
}

/// s31_13_i_1: --help is the reference's without its bug-report lines; -V names libniji.
#[test]
fn pldd_options() {
    let Some(prog) = ours("pldd", &[REF_PLDD]) else { return };
    for opt in ["--help", "-?", "--he"] {
        let (a, b) = (pldd(&prog, &[opt]), pldd(Path::new(REF_PLDD), &[opt]));
        assert_eq!((a.rc, b.rc), (0, 0), "{opt}");
        let rest = b.out.strip_prefix(a.out.as_str()).unwrap_or_else(|| panic!("{opt}: ours is the reference's start:\n{}\n{}", a.out, b.out));
        assert!(rest.trim().is_empty() || rest.contains("bug"), "{opt}: only the bug-report lines are left out: {rest:?}");
    }
    for opt in ["-V", "--version"] {
        let a = pldd(&prog, &[opt]);
        assert_eq!((a.rc, a.out.as_str()), (0, "pldd (libniji) 2.39\n"), "{opt}");
    }
}

/// The package manifest ships `/usr/bin/ld.so` in both flavours, a link that resolves to the loader the flavour
/// ships, and the two commands in libc-bin (plan/utilities.md §2). No spec clause: release packaging.
#[test]
fn manifest_ships_ld_so_link_and_commands() {
    let text = fs::read_to_string(root().join("tools/deb/manifest.tsv")).expect("manifest");
    let rows: Vec<Vec<&str>> = text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).map(|l| l.split('\t').collect()).collect();
    for (flavour, prefix) in [("system", ""), ("coexist", "usr/lib/niji/")] {
        let in_flavour = |path: &str| {
            // A both row names the system path; the coexist flavour ships it under the sysroot.
            rows.iter().any(|r| (r[0] == flavour && r[2] == path) || (r[0] == "both" && format!("{prefix}{}", r[2]) == path))
        };
        let link = format!("{prefix}usr/bin/ld.so");
        let row = rows.iter().find(|r| r[0] == flavour && r[2] == link).unwrap_or_else(|| panic!("{flavour}: no {link} row"));
        assert_eq!(row[3], "link", "{flavour}: {link} is a link");
        let mut parts: Vec<&str> = link.split('/').collect();
        parts.pop();
        for c in row[4].split('/') {
            match c {
                ".." => {
                    parts.pop();
                }
                c => parts.push(c),
            }
        }
        let resolved = parts.join("/");
        assert!(in_flavour(&resolved), "{flavour}: {link} -> {} resolves to {resolved}, which the flavour does not ship", row[4]);
        assert!(resolved.ends_with("ld-linux-x86-64.so.2"), "{flavour}: {link} names the loader");
    }
    for cmd in ["usr/bin/tzselect", "usr/bin/pldd"] {
        assert!(rows.iter().any(|r| r[0] == "system" && r[1] == "libc-bin" && r[2] == cmd && r[3] == "file"), "libc-bin ships {cmd}");
    }
}
