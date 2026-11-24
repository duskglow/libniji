//! Message catalogs end to end (spec §7.7, §7.8; D-7.5, D-7.6; Appendix D.2):
//! `tests/oracle/catalogs_diff.c` built against the host C library and statically against
//! libniji's products, each run by `tests/oracle/catalogs_run.sh` (fixture catalogs built with
//! `msgfmt` and `gencat`, every other catalog written by the probe below a scratch directory under
//! `target/`), with `LOCPATH` naming the test locales; libniji with the name-service cache on only (`nscache_mode.sh`, D-16.10; one mode, D-16.18: the program makes no lookup, `tests/nscache_reach.rs`); every run, the reference's included, with the system's locale
//! directory hidden but for `C.utf8`, so the host's installed locales never decide a case
//! (`setlocale` searches that directory after `LOCPATH`; D-7.23). The outputs must be identical line by line, except the
//! recorded differences: [`KNOWN`] (a line that differs), [`ADDED`] (a line only libniji prints) and
//! [`DROPPED`] (a line only the reference prints). Every entry names the case it belongs to: the
//! text of the last `== ` or `-- ` header line before it starts with the entry's first string, so a
//! line repeated in many cases is recorded only where it differs.
//!
//! The record (`spec/behaviour/catalogs.glibc.txt`) was made on the test machine; the test reports
//! whether the reference still equals it.
//!
//! Needs the products (`cargo xtask so`), a C compiler, `localedef`, `msgfmt` (package `gettext`),
//! `gencat` and passwordless `sudo` (the name-service cache modes); without them the test says so
//! and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

#[path = "support/timeout.rs"]
mod timeout;
#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/scratch.rs"]
mod scratch;
use oracle::{root, with_headers, without};
use std::path::Path;
use std::process::Command;

fn ensure_locale(dir: &Path, name: &str) -> bool {
    if dir.join(name).join("LC_COLLATE").exists() {
        return true;
    }
    let Some((lang, charset)) = name.split_once('.') else { return false };
    oracle::locale(&dir.join(name), lang, charset, "LC_COLLATE")
}

fn build(out: &Path, niji: bool) -> bool {
    oracle::build_pair("crates/core/tests/oracle/catalogs_diff.c").quiet().build(out, niji)
}

const REPLACED: &str = "D-7.5 (caching; owner ruling 2026-10-05): a loaded catalog is checked against its file at most once per second and at once after a setting changes, so a replaced catalog is loaded again; the reference never re-reads a file whose binding string is unchanged";
const LANGUAGE: &str = "D-7.5 (caching): LANGUAGE and the locale are read at every lookup, nothing is cached per msgid; the reference keeps a msgid's earlier translation until _nl_msg_cat_cntr moves";
const PATHS: &str = "D-7.5 (security default, reported): a locale name with `/` is never a catalog directory, so LANGUAGE cannot leave the bound directory; the reference uses it as a path";
const FORMS: &str = "D-7.5: plural forms are found in the stored translation and each is converted on its own; the reference splits the converted text at its first NUL byte, which in UTF-16 is inside the first character";
const TRANSLIT: &str = "D-6.9: the built-in C locale's transliteration is derived from the Unicode database, which has no Latin rendering of Cyrillic; the reference's own table gives ZHa";
const NO_TRAP: &str = "§7.7 never fails visibly (three-way: the spec and gettext(3) agree): a division or remainder by zero in Plural-Forms selects the default rule n != 1";
const HASH: &str = "D-7.5 (decision 2026-10-04): the sorted originals are searched by binary search and the hash table is not used, so every entry of a catalog with a damaged hash table is found; the reference trusts the table alone";
const DAMAGED_MO: &str = "D-7.5: a .mo whose strings start beyond the file is not a catalog (absent): nothing outside the file is read";
const FIFO: &str = "D-7.5/D-7.6: a catalog that is not a regular file (here a FIFO) is opened non-blocking and treated as absent (catopen: EINVAL, as for a directory)";
const CATGETS_ERRNO: &str = "§7.8 and C0.1 (three-way: the spec and POSIX catgets agree): a message the catalog lacks sets ENOMSG, negative numbers and set INT_MAX included; the reference leaves errno alone for those";
const CATGETS_BADF: &str = "§7.8 and C0.1 (three-way: the spec and POSIX catgets, EBADF, agree): an invalid descriptor sets EBADF; the reference leaves errno alone";
const PERCENT: &str = "D-7.6: an NLSPATH template ending in a lone % is skipped like one with an unknown conversion; the reference abandons the whole search there";
const NOT_CAT: &str = "§7.8 and C0.1 (three-way: the spec and POSIX catopen agree that a failure sets errno): a file that is not a catalog fails with EINVAL; the reference leaves errno 0";
const LANG_PATH: &str = "D-7.6 (security default, reported): a locale value with `/` (or `.`, `..`) is never substituted into an NLSPATH or default template; the reference substitutes it unchecked";
const CAT_CUT: &str = "D-7.6: a .cat whose strings start beyond the file is not a catalog: nothing outside the file is read";
const CAT_ZERO: &str = "D-7.6: a .cat with plane size 0 is not a catalog (EINVAL)";

/// Lines that differ from the reference by decision: the case (the start of the last header
/// line), the text the reference's line starts with, and why. A recorded line that no longer
/// differs fails the test.
const KNOWN: [(&str, &str, &str); 36] = [
    ("-- catalog file replaced on disk after it was loaded", "  lookup 1.1 s after the file was replaced: [old]", REPLACED),
    ("-- catalog file replaced on disk after it was loaded", "  after bindtextdomain with the same directory: [old]", REPLACED),
    ("-- UTF-8 catalog, de_DE.UTF-8 locale, codeset [UTF-16LE]", "  file n=2 [\\xe9]", FORMS),
    ("-- UTF-8 catalog, LC_MESSAGES de_DE.UTF-8, LC_CTYPE C\n", "  cyr [ZHa]", TRANSLIT),
    ("-- LANG=fr LANGUAGE=../sel/it", "  LC_MESSAGES [fr_FR.UTF-8] where [it]", PATHS),
    ("-- LANG=fr LANGUAGE=/it", "  LC_MESSAGES [fr_FR.UTF-8] where [it]", PATHS),
    ("-- LANGUAGE changed at run time", "  LANGUAGE=nl, nothing else: [it]", LANGUAGE),
    ("-- LANGUAGE changed at run time", "  LANGUAGE unset: [nl]", LANGUAGE),
    ("-- LANGUAGE changed at run time", "  LANGUAGE=it again: [nl]", LANGUAGE),
    ("-- environment changed after setlocale", "  LC_MESSAGES/LANG now fr_FR.UTF-8: [nl]", LANGUAGE),
    ("-- division by zero", "  child killed by signal SIGFPE", NO_TRAP),
    ("-- remainder by zero", "  child killed by signal SIGFPE", NO_TRAP),
    ("-- division by n", "  child killed by signal SIGFPE", NO_TRAP),
    ("-- msgfmt catalog big.mo with its hash table altered", "  every hash entry 0 (empty): 0 right", HASH),
    ("-- msgfmt catalog big.mo with its hash table altered", "  every hash entry 2 (a valid index, the wrong string):  child killed", HASH),
    ("-- half the file", "  hello []", DAMAGED_MO),
    ("-- a FIFO in place of the file (3 s limit)", "  child killed by signal SIGALRM", FIFO),
    ("-- the gencat fixture main.cat", "  1,-1: (default) errno 0", CATGETS_ERRNO),
    ("-- the gencat fixture main.cat", "  -1,1: (default) errno 0", CATGETS_ERRNO),
    ("-- the gencat fixture main.cat", "  2147483647,1: (default) errno 0", CATGETS_ERRNO),
    ("-- the gencat fixture main.cat", "  -2147483648,-2147483648: (default) errno 0", CATGETS_ERRNO),
    ("-- the gencat fixture main.cat", "  -2,1: (default) errno 0", CATGETS_ERRNO),
    ("-- the gencat fixture main.cat", "  catgets((nl_catd)-1, 1, 1): (default) errno 0", CATGETS_BADF),
    ("-- trailing % in the first template, a good second one", "  catopen: -1 errno ENOENT", PERCENT),
    ("-- first candidate is not a catalog", "  catopen: -1 errno 0", NOT_CAT),
    ("-- only candidate is not a catalog", "  catopen: -1 errno 0", NOT_CAT),
    ("-- LANG with a slash", "  catopen: ok", LANG_PATH),
    ("-- LANG=../x (parent directory)", "  catopen: ok", LANG_PATH),
    ("-- 12 bytes (header only)", "  catopen: -1 errno 0", NOT_CAT),
    ("-- tables cut (40 bytes)", "  catopen: -1 errno 0", NOT_CAT),
    ("-- strings cut (90 bytes)", "  catopen: ok; 1,1 [one-on]", CAT_CUT),
    ("-- wrong magic\n", "  catopen: -1 errno 0", NOT_CAT),
    ("-- a big-endian writer's file", "  catopen: -1 errno 0", NOT_CAT),
    ("-- plane size 0", "  child killed by signal SIGFPE", CAT_ZERO),
    ("-- string offset of 1,1 beyond the file", "  catopen: -1 errno 0", NOT_CAT),
    ("-- a FIFO (3 s limit)", "  child killed by signal SIGALRM", FIFO),
];

/// Lines only libniji prints: the case, the text the line starts with, and why (the first such
/// line of the case is taken; a repeated entry takes the next one).
const ADDED: [(&str, &str, &str); 6] = [
    ("-- division by zero", "  22:[f1]", NO_TRAP),
    ("-- division by zero", "  singular-only entry:", NO_TRAP),
    ("-- remainder by zero", "  22:[f1]", NO_TRAP),
    ("-- remainder by zero", "  singular-only entry:", NO_TRAP),
    ("-- division by n", "  22:[f0]", NO_TRAP),
    ("-- division by n", "  singular-only entry:", NO_TRAP),
];

/// Lines only the reference prints: as [`ADDED`].
const DROPPED: [(&str, &str, &str); 0] = [];

/// The header lines a recorded entry's case is matched against (`oracle::with_headers`).
const HEADERS: &[&str] = &["== ", "-- "];

/// A line diff of `want` (glibc) and `got` (libniji) by longest common subsequence: each run of
/// differing lines under the header of its case, `-` for glibc and `+` for libniji, at most 200 lines.
/// A run that only replaces lines recorded in [`KNOWN`] is marked `(recorded)` and comes after the
/// others: a log that keeps only the start of the message shows a difference nobody recorded.
fn line_diff(want: &[(String, String)], got: &[(String, String)]) -> String {
    struct Run {
        header: String,
        at: usize,
        lines: Vec<String>,
        dropped: Vec<(String, String)>,
        added: usize,
    }
    let mut runs: Vec<Run> = Vec::new();
    let shown = lcs_walk(want, got, |h, line, at, dropped| {
        if runs.last().is_none_or(|r| r.header != h) {
            runs.push(Run { header: h.to_string(), at, lines: Vec::new(), dropped: Vec::new(), added: 0 });
        }
        let r = runs.last_mut().unwrap();
        r.lines.push(line);
        match dropped {
            Some(d) => r.dropped.push(d.clone()),
            None => r.added += 1,
        }
    });
    let known = |(h, w): &(String, String)| KNOWN.iter().any(|(case, needle, _)| h.starts_with(case) && w.starts_with(needle));
    let recorded = |r: &Run| !r.dropped.is_empty() && r.dropped.len() == r.added && r.dropped.iter().all(known);
    let mut out = Vec::new();
    for r in runs.iter().filter(|r| !recorded(r)).chain(runs.iter().filter(|r| recorded(r))) {
        out.push(format!("@ line {} in {}{}", r.at + 1, r.header.trim_end(), if recorded(r) { " (recorded)" } else { "" }));
        out.extend(r.lines.iter().cloned());
    }
    if shown == 200 {
        out.push("(diff cut at 200 lines)".to_string());
    }
    out.join("\n")
}

/// Walk `want` and `got` by longest common subsequence, calling `emit` with the header, the
/// `+`/`-` line, the line number in `want` and (for a `-` line) the reference line, for at most
/// 200 differing lines; returns how many were emitted.
fn lcs_walk(want: &[(String, String)], got: &[(String, String)], mut emit: impl FnMut(&str, String, usize, Option<&(String, String)>)) -> usize {
    let (n, m) = (want.len(), got.len());
    // lcs[i][j]: the common subsequence length of want[i..] and got[j..].
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if want[i].1 == got[j].1 { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut shown) = (0, 0, 0usize);
    while (i < n || j < m) && shown < 200 {
        if i < n && j < m && want[i].1 == got[j].1 {
            i += 1;
            j += 1;
        } else if j < m && (i == n || lcs[i][j + 1] >= lcs[i + 1][j]) {
            emit(&got[j].0, format!("+ {}", got[j].1), i, None);
            j += 1;
            shown += 1;
        } else {
            emit(&want[i].0, format!("- {}", want[i].1), i, Some(&want[i]));
            i += 1;
            shown += 1;
        }
    }
    shown
}

/// The damaged catalogs the record leaves out (offsets and sizes outside the file;
/// catalogs.md, "Unstable cases") are safe in libniji
/// (D-7.5, D-7.6: a damaged catalog is absent): every case ends normally, a `.mo` translates
/// nothing it cannot read, and `catopen` fails with `EINVAL`.
#[test]
fn damaged_catalogs_outside_the_record_are_safe() {
    // Clauses: s7_7_g_0, s7_7_c_1, s7_7_c_2, s7_8_f_0 (D-7.5, D-7.6).
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    let absent: Vec<&str> = ["cc", "msgfmt", "gencat"].into_iter().filter(|t| skip_note::missing(t)).collect();
    if !absent.is_empty() {
        skip_note::skipped(&format!("no {} (the fixture catalogs and the probe need them)", absent.join(", ")));
        return;
    }
    let locales = r.join("target/test-locales");
    std::fs::create_dir_all(&locales).unwrap();
    if !ensure_locale(&locales, "de_DE.UTF-8") {
        skip_note::skipped("skipped: no localedef");
        return;
    }
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let ours = dir.join("catalogs_unstable_niji");
    assert!(build(&ours, true), "catalogs_diff.c does not link against the products");
    let o = Command::new("timeout")
        .arg(timeout::secs(600))
        .arg("env")
        .arg(format!("LOCPATH={}", locales.display()))
        .arg("sh")
        .arg(r.join("crates/core/tests/oracle/catalogs_run.sh"))
        .arg(&ours)
        .arg(dir.join("catalogs-unstable"))
        .arg("unstable")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(o.status.success(), "{:?}\n{}", o.status, String::from_utf8_lossy(&o.stderr));
    let out = String::from_utf8_lossy(&o.stdout);
    eprintln!("{out}");
    assert!(out.lines().last() == Some("done"), "the run ends");
    assert!(!out.contains("child killed"), "no case is killed");
    let mut cases = 0;
    for (h, l) in with_headers(&out, HEADERS) {
        if !h.starts_with("-- ~") || l.starts_with("-- ") || l == "done" {
            continue;
        }
        cases += 1;
        if h.contains(".cat") || l.starts_with("  catopen") {
            assert_eq!(l, "  catopen: -1 errno EINVAL", "{h}");
        } else if !h.contains("hash size") {
            assert_eq!(l, "  hello [hello] zz [zz] hell [hell] file n=2 (msgid_plural) errno EINTR", "{h}");
        } else {
            assert_eq!(l, "  hello [hallo] zz [ZZ] hell [hell] file n=2 [Dateien] errno EINTR", "{h}: the hash table is not used");
        }
    }
    assert_eq!(cases, 14, "every unstable .mo and .cat case printed its result");
    assert_eq!(out.matches(": 500 right, 0 untranslated, 0 other").count(), 3, "hash entries beyond N: the table is not used");
}

#[test]
fn catalogs_equal_the_reference() {
    // Clauses: s7_7_c_0, s7_7_c_1, s7_7_c_2, s7_7_c_3, s7_7_c_4, s7_7_c_5, s7_7_g_0, s7_8_c_0, s7_8_c_p1,
    // s7_8_f_0 (D-7.5, D-7.6, D-7.12).
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler");
        return;
    }
    for tool in ["msgfmt", "gencat"] {
        if skip_note::missing(tool) {
            skip_note::skipped(&format!("skipped: no {tool} (the fixture catalogs need it)"));
            return;
        }
    }
    if !Command::new("sudo").args(["-n", "true"]).status().is_ok_and(|s| s.success()) {
        skip_note::skipped("skipped: no passwordless sudo (the name-service cache modes need a private mount namespace)");
        return;
    }
    let locales = r.join("target/test-locales");
    std::fs::create_dir_all(&locales).unwrap();
    let needed = ["de_DE.UTF-8", "fr_FR.UTF-8", "fr_FR.ISO-8859-1", "cs_CZ.UTF-8", "hu_HU.UTF-8", "it_IT.UTF-8", "ru_RU.KOI8-R", "ja_JP.EUC-JP"];
    if !needed.iter().all(|n| ensure_locale(&locales, n)) {
        skip_note::skipped("skipped: no localedef");
        return;
    }
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let (host, ours) = (dir.join("catalogs_diff_host"), dir.join("catalogs_diff_niji"));
    assert!(build(&host, false), "catalogs_diff.c does not build against the host C library");
    assert!(build(&ours, true), "catalogs_diff.c does not link against the products");
    let script = r.join("crates/core/tests/oracle/catalogs_run.sh");
    let wrap = r.join("crates/core/tests/oracle/nscache_mode.sh");
    // Every run, the reference's included, sees only C.utf8 (installed with the C library) in the
    // system locale directory: setlocale searches it after LOCPATH, so otherwise the host's
    // installed locales decide cases such as `LC_ALL=de_DE.UTF-8@euro` (catalogs_run.sh refuses
    // to run without it).
    let run = |exe: &Path, scratch: &str, mode: &str| -> String {
        let mut cmd = Command::new("timeout");
        cmd.arg(timeout::secs(1200));
        cmd.arg("sh").arg(&wrap).arg("--hide-system-locales").arg(mode);
        // nscache_mode.sh clears the environment: LOCPATH is passed on through `env`.
        cmd.arg("env").arg(format!("LOCPATH={}", locales.display()));
        let start = std::time::Instant::now();
        // The run's root processes (sudo) are contained (support/contain.rs).
        let o = oracle::contain::output(cmd.arg("sh").arg(&script).arg(exe).arg(dir.join(scratch)).env_clear().env("PATH", "/usr/bin:/bin")).unwrap();
        let err = String::from_utf8_lossy(&o.stderr);
        // Diagnostics for a failure: how long the run took, the load, and what the probe said on stderr
        // (a child that met its watchdog is named there).
        let load = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
        eprintln!("{scratch}: {:.1} s, loadavg {}", start.elapsed().as_secs_f64(), load.trim_end());
        if !err.is_empty() {
            eprintln!("{scratch} stderr:\n{err}");
        }
        if !o.status.success() {
            // The end of stdout says which step the run was in (the probe's stderr says only which child).
            let out = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = out.lines().collect();
            let tail = lines.get(lines.len().saturating_sub(40)..).unwrap_or_default().join("\n");
            panic!("{} failed: {:?}\n{err}\nlast lines of stdout:\n{tail}", exe.display(), o.status);
        }
        String::from_utf8_lossy(&o.stdout).into_owned()
    };
    let reference = run(&host, "catalogs-host", "plain");
    let record = std::fs::read_to_string(r.join("spec/behaviour/catalogs.glibc.txt")).unwrap();
    if reference == record {
        eprintln!("the reference equals the record");
    } else {
        eprintln!("note: the reference differs from the record");
    }
    // One mode (D-16.18): the program reaches no name-service cache lookup (tests/nscache_reach.rs).
    let mode = "on";
    eprintln!("name-service cache {mode}");
    let got = run(&ours, &format!("catalogs-niji-{mode}"), mode);
    let want = without(with_headers(&reference, HEADERS), &DROPPED, "the reference");
    let got = without(with_headers(&got, HEADERS), &ADDED, "libniji");
    let mut diffs = 0;
    let mut seen = [false; KNOWN.len()];
    for (k, ((h, w), (_, g))) in want.iter().zip(got.iter()).enumerate() {
        if let Some(i) = KNOWN.iter().position(|(case, needle, _)| h.starts_with(case) && w.starts_with(needle)) {
            if w != g {
                seen[i] = true;
                eprintln!("recorded difference ({}):\n  glibc  {w}\n  niji {g}", KNOWN[i].2);
            }
            continue;
        }
        if w != g {
            diffs += 1;
            if diffs <= 400 {
                eprintln!("line {} ({}):\n  glibc  {w}\n  niji {g}", k + 1, h.trim_end());
            }
        }
    }
    let text = |v: &[(String, String)]| v.iter().map(|(_, l)| l.as_str()).collect::<Vec<_>>().join("\n");
    if want.len() != got.len() {
        // The diff comes first: a log that keeps only the start of a long message still shows it.
        let (wf, gf) = (dir.join("catalogs-failed-glibc.txt"), dir.join(format!("catalogs-failed-niji-{mode}.txt")));
        let _ = std::fs::write(&wf, text(&want));
        let _ = std::fs::write(&gf, text(&got));
        panic!(
            "line counts differ (glibc {}, niji {}; name-service cache {mode}); line diff (- glibc, + niji):\n{}\nfull outputs (recorded lines removed): {} and {}\n--- glibc\n{}\n--- niji\n{}",
            want.len(),
            got.len(),
            line_diff(&want, &got),
            wf.display(),
            gf.display(),
            text(&want),
            text(&got)
        );
    }
    assert_eq!(diffs, 0, "lines that differ from the reference");
    for (i, s) in seen.iter().enumerate() {
        assert!(s, "recorded difference no longer seen: {:?}", KNOWN[i]);
    }
}

/// `gencat` (UTIL-14, spec §31.12, Appendix D.2.2): libniji's program (`target/niji-test/gencat`, run on this
/// tree's loader and library) and the reference's compile one generated source; the two catalogs must be the
/// same bytes, and `catgets` (§7.8) must return every message of the source, and the default for keys the source
/// does not define, from libniji's catalog both on libniji (`tests/oracle/gencat_lookup.c` linked statically
/// against the products) and on the reference (the same probe built against the host C library), and from the
/// reference's catalog on libniji: catalogs built either way are interchangeable.
///
/// Needs the products (`cargo xtask so`) and a C compiler; the reference's `gencat` is optional (without it the
/// byte comparison and the reference's catalog are skipped, and the test says so).
mod gencat {
    use super::{oracle, root, scratch, skip_note, timeout};
    use std::fmt::Write as _;
    use std::path::Path;
    use std::process::{Command, Stdio};

    /// The source and, for every key it defines, the text `catgets` must return (in the probe's escaped form).
    /// Sets are named out of order, the default set 1 in the middle, with gaps, a symbolic set and symbolic
    /// messages, escapes, a quoted text and a continued line: the pool order, the plane search and the slot
    /// collisions all matter for the bytes, and every key must still be found.
    fn source() -> (String, Vec<(u32, u32, String)>) {
        let mut src = String::from("$ generated by catalogs_products.rs\n");
        let mut want = Vec::new();
        let mut x: u32 = 12345;
        let mut next = |m: u32| {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (x >> 8) % m
        };
        for &set in &[7u32, 3, 1, 250, 12] {
            writeln!(src, "$set {set}").unwrap();
            let mut used = std::collections::BTreeSet::new();
            while used.len() < 60 {
                used.insert(1 + next(500));
            }
            for &m in &used {
                let pad = "z".repeat(usize::try_from(next(12)).unwrap());
                match m % 7 {
                    0 => {
                        writeln!(src, "{m} tab\\there\\101 {pad}").unwrap();
                        want.push((set, m, format!("tab\\011hereA {pad}")));
                    }
                    1 => {
                        writeln!(src, "{m} cont\\\nnued {set}/{m}").unwrap();
                        want.push((set, m, format!("contnued {set}/{m}"))); // a trailing backslash joins the lines
                    }
                    _ => {
                        writeln!(src, "{m} set {set} message {m} {pad}").unwrap();
                        want.push((set, m, format!("set {set} message {m} {pad}")));
                    }
                }
            }
        }
        // A symbolic set takes the next number (251); its symbolic messages are numbered 1, 2, after none.
        src.push_str("$set Named\nFirst one\nSecond \u{e9}t\u{e9}\n$quote \"\n9 \"quoted\" rest\n");
        want.push((251, 1, "one".into()));
        want.push((251, 2, "\\303\\251t\\303\\251".into()));
        want.push((251, 9, "quoted".into()));
        (src, want)
    }

    fn run(cmd: &mut Command, what: &str) -> Vec<u8> {
        let o = cmd.stdin(Stdio::null()).output().unwrap_or_else(|e| panic!("{what}: {e}"));
        assert!(o.status.success(), "{what}: {:?}\n{}", o.status, String::from_utf8_lossy(&o.stderr));
        o.stdout
    }

    fn gencat(prog: &Path, src: &Path, out: &Path) {
        let mut cmd = Command::new("timeout");
        cmd.arg(timeout::secs(60)).arg(prog).args(["--new", "-o"]).arg(out).arg(src).env_clear().env("LC_ALL", "C.UTF-8");
        run(&mut cmd, &format!("{} {}", prog.display(), src.display()));
    }

    fn lookup(probe: &Path, cat: &Path, keys: &str) -> String {
        use std::io::Write as _;
        let mut child = Command::new("timeout")
            .arg(timeout::secs(60))
            .arg(probe)
            .arg(cat)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(keys.as_bytes()).unwrap();
        let o = child.wait_with_output().unwrap();
        assert!(o.status.success(), "{} {}: {:?}\n{}", probe.display(), cat.display(), o.status, String::from_utf8_lossy(&o.stderr));
        String::from_utf8(o.stdout).unwrap()
    }

    #[test]
    fn catalogs_from_both_gencats_are_interchangeable() {
        // Clauses: s31_12_c_1, s31_12_g_1, s31_1_c_r3, s7_8_c_0.
        let r = root();
        let ours = r.join("target/niji-test/gencat");
        assert!(ours.exists() && r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
        if skip_note::missing("cc") {
            skip_note::skipped("no cc (the catgets probe needs it)");
            return;
        }
        let dir = scratch::Scratch::dir("gencat");
        let d: &Path = &dir;
        let (src, want) = source();
        let msg = d.join("i.msg");
        std::fs::write(&msg, &src).unwrap();
        // Every defined key, then keys the source does not define: a missing message in a set, sets never named,
        // the empty set 0 and message 0, and a key whose plane slot is taken by another (set 2 is never named).
        let mut keys = String::new();
        let mut expect = String::new();
        for (s, m, t) in &want {
            writeln!(keys, "{s} {m}").unwrap();
            writeln!(expect, "{s} {m} {t}").unwrap();
        }
        let defined: std::collections::BTreeSet<(u32, u32)> = want.iter().map(|(s, m, _)| (*s, *m)).collect();
        for k in [(7, 0), (0, 1), (2, 1), (4, 4), (252, 1), (251, 3)].into_iter().chain((1..40).map(|m| (1, m))).filter(|k| !defined.contains(k)) {
            writeln!(keys, "{} {}", k.0, k.1).unwrap();
            writeln!(expect, "{} {} (default)", k.0, k.1).unwrap();
        }

        let src_c = r.join("crates/core/tests/oracle/gencat_lookup.c");
        let niji_probe = d.join("lookup_niji");
        let mut cmd = Command::new("cc");
        cmd.args(["-std=gnu17", "-O1", "-static", "-nostdlib", "-fno-builtin"]).arg(r.join("target/niji/niji-crt1.o")).arg(&src_c).arg(r.join("target/niji/libc.a")).arg("-lgcc");
        assert!(oracle::build_quiet(cmd, &niji_probe), "gencat_lookup.c does not link against the products");
        let ref_probe = d.join("lookup_ref");
        let mut cmd = Command::new("cc");
        cmd.args(["-std=gnu17", "-O1"]).arg(&src_c);
        assert!(oracle::build_quiet(cmd, &ref_probe), "gencat_lookup.c does not build against the host C library");

        let ours_cat = d.join("niji.cat");
        gencat(&ours, &msg, &ours_cat);
        assert_eq!(lookup(&niji_probe, &ours_cat, &keys), expect, "catgets on libniji over libniji's catalog");
        assert_eq!(lookup(&ref_probe, &ours_cat, &keys), expect, "catgets on the host C library over libniji's catalog");

        let reference = Path::new("/usr/bin/gencat");
        if !reference.exists() {
            skip_note::skipped("no /usr/bin/gencat: the byte comparison with the reference's catalog is skipped");
            return;
        }
        let ref_cat = d.join("ref.cat");
        gencat(reference, &msg, &ref_cat);
        assert_eq!(lookup(&niji_probe, &ref_cat, &keys), expect, "catgets on libniji over the reference's catalog");
        let (a, b) = (std::fs::read(&ours_cat).unwrap(), std::fs::read(&ref_cat).unwrap());
        let first = a.iter().zip(&b).position(|(x, y)| x != y);
        assert!(a == b, "libniji's catalog ({} bytes) differs from the reference's ({} bytes), first at byte {first:?}", a.len(), b.len());
    }
}
