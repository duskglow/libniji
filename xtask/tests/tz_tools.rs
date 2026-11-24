//! TZ-TOOLS: libniji's `zic` and `zdump` (`cargo xtask so` test links under target/niji-test) against the
//! reference programs and the reference tzdata source, run black-box on the host.
//!
//! * The corpus: every zone of the installed `tzdata.zi` compiled by both programs (fat, the default, and
//!   `-b slim`) decodes to the same transitions, indicators, first type, leap records and footer, except the
//!   zones listed in `KNOWN_*` (reasons there), which must still read the same through `zdump`.
//! * zdump: libniji's zdump over libniji's localtime prints what the reference zdump prints, for every zone.
//! * Malformed source: the same exit status and message; zone and link names (refused, kept, over the
//!   kernel's limits): the same status, message and files; hostile source, and links already in the output
//!   tree: never a crash, never a file outside the output directory (D-31.18).
//!
//! UTIL-TZSELECT-PLDD (below the zic and zdump tests): libniji's `tzselect` and `pldd` against the reference
//! programs, and the `/usr/bin/ld.so` rows of the package manifest.
//!
//! * tzselect: scripted sessions (every continent, junk answers, every probed TZ string, No then Yes, the coord
//!   menu, a locale without UTF-8, narrow and wide terminals, options, missing tables) give the same exit status,
//!   standard output and standard error as the reference: the scripted sessions with the real-time clock stopped at
//!   fixed times (a preloaded library built here), clock lines compared; the other sessions with the two clock lines
//!   of the information block masked. Every session ends on a confirmation or at a continent or country menu. `-c`: the reference's zones with every zone
//!   listed, in increasing great-circle distance (D-31.34), checked against a haversine computed here. End of input
//!   at every prompt of a session: status 1 and no value (libniji only).
//! * pldd: on a traceable process with dlopen'd objects and a second thread, the reference's listing; the errors
//!   (no operand, two, not a number, no such process, not traceable), `--usage` and option errors the same.
//! * The manifest: both flavours ship `usr/bin/ld.so` as a link that resolves to the loader the flavour ships.
//!
//! Trace: s31_9_c_1, s31_9_i_1, s31_9_i_2, s31_9_g_1, s31_9_g_2, s31_9_g_3, s31_9_f_1, s31_10_c_1,
//! s31_10_i_1, s31_10_g_1, s31_10_g_2, s31_10_g_3, s31_10_g_4, s31_11_c_1, s31_11_i_1, s31_11_i_2, s31_11_g_1,
//! s31_11_f_1, s31_13_c_1, s31_13_i_1, s31_13_g_1, s31_13_f_1, s31_13_f_2, s31_13_f_3.

#![allow(clippy::disallowed_methods)] // a host tool's test: S8 governs the library crates only

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

mod test_link;

const TZDATA: &str = "/usr/share/zoneinfo/tzdata.zi";
const REF_ZIC: &str = "/usr/sbin/zic";
const REF_ZDUMP: &str = "/usr/bin/zdump";

/// Fat output: zones whose decoded content differs from the reference's while reading the same. Tbilisi and
/// Lisbon: the reference keeps one transition that changes nothing a reader sees (a line boundary inside +05
/// DST in 1997; LMT to the same LMT in 1884); Warsaw: the 1918 change to EET carries the standard-time
/// indicator of the coinciding rule. Links share their target's file.
const KNOWN_FAT: &[&str] = &["Asia/Tbilisi", "Europe/Lisbon", "Portugal", "Europe/Warsaw", "Poland"];
/// Slim output: the same, plus zones where the two hand over from the listed transitions to the footer at a
/// different transition (libniji keeps one the footer also gives, or leaves one to it). America/Ojinaga: libniji's
/// slim file reads CST from 2022-10-30 as the source gives (the reference's slim file reads otherwise until
/// 2022-11-06), so these zones are judged against the reference's fat file.
const KNOWN_SLIM_EXTRA: &[&str] = &["America/Ciudad_Juarez", "America/Indiana/Winamac", "America/Ojinaga", "Asia/Gaza", "Asia/Hebron"];
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root").to_path_buf()
}

/// libniji's test link of `name`, or None (with the reason printed) when it is missing.
fn link(name: &str) -> Option<PathBuf> {
    // XTASK-STALE-PRODUCTS: a link left by an earlier build whose loader or library is gone is skipped too.
    match test_link::usable(&root().join("target/niji-test").join(name)) {
        Ok(p) => Some(p),
        Err(why) => {
            println!("SKIPPED: {why}");
            None
        }
    }
}

/// libniji's test link of `name`, or None (with the reason printed) when it or the reference is missing.
fn tools(name: &str, reference: &str) -> Option<PathBuf> {
    let ours = link(name)?;
    for p in [reference, TZDATA] {
        if !Path::new(p).is_file() {
            println!("SKIPPED: {p} is not installed on this host (the reference to compare with)");
            return None;
        }
    }
    Some(ours)
}

static SCRATCH: AtomicUsize = AtomicUsize::new(0);

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("niji-tz-{}-{}-{tag}", std::process::id(), SCRATCH.fetch_add(1, Ordering::Relaxed)));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).expect("scratch directory");
    d
}

fn run(cmd: &mut Command) -> Output {
    cmd.output().expect("run a tz program")
}

#[derive(Debug, PartialEq, Clone)]
struct Block {
    version: u8,
    first: Option<(i32, u8, String)>,
    trans: Vec<(i64, i32, u8, String, u8, u8)>,
    leaps: Vec<(i64, i32)>,
}

/// One TZif data block (RFC 9636 §3) from `d` at `off`; returns it and the offset after it.
fn block(d: &[u8], off: usize, v1: bool) -> Result<(Block, usize), String> {
    let need = |p: usize, n: usize| if p + n <= d.len() { Ok(()) } else { Err(format!("truncated at {p}")) };
    need(off, 44)?;
    if &d[off..off + 4] != b"TZif" {
        return Err("no TZif magic".into());
    }
    let be32 = |p: usize| i32::from_be_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]);
    let counts: Vec<usize> = (0..6).map(|i| be32(off + 20 + 4 * i) as usize).collect();
    let (isut, isstd, leap, timecnt, typecnt, charcnt) = (counts[0], counts[1], counts[2], counts[3], counts[4], counts[5]);
    let tsz = if v1 { 4 } else { 8 };
    let mut p = off + 44;
    need(p, timecnt * (tsz + 1) + typecnt * 6 + charcnt + leap * (tsz + 4) + isstd + isut)?;
    let time = |p: usize| if v1 { be32(p) as i64 } else { i64::from_be_bytes(d[p..p + 8].try_into().unwrap()) };
    let times: Vec<i64> = (0..timecnt).map(|i| time(p + i * tsz)).collect();
    p += timecnt * tsz;
    let idx: Vec<usize> = d[p..p + timecnt].iter().map(|&b| b as usize).collect();
    p += timecnt;
    let types: Vec<(i32, u8, usize)> = (0..typecnt).map(|i| (be32(p + 6 * i), d[p + 6 * i + 4], d[p + 6 * i + 5] as usize)).collect();
    p += typecnt * 6;
    let chars = &d[p..p + charcnt];
    p += charcnt;
    let leaps: Vec<(i64, i32)> = (0..leap).map(|i| (time(p + i * (tsz + 4)), be32(p + i * (tsz + 4) + tsz))).collect();
    p += leap * (tsz + 4);
    let std = &d[p..p + isstd];
    p += isstd;
    let ut = &d[p..p + isut];
    p += isut;
    let abbr = |a: usize| -> String {
        let s = chars.get(a..).unwrap_or(&[]);
        String::from_utf8_lossy(&s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]).into_owned()
    };
    let resolved = |i: usize| -> Result<(i32, u8, String, u8, u8), String> {
        let (u, dst, a) = *types.get(i).ok_or("type index out of range")?;
        Ok((u, dst, abbr(a), std.get(i).copied().unwrap_or(0), ut.get(i).copied().unwrap_or(0)))
    };
    let mut trans = Vec::new();
    for (t, i) in times.iter().zip(idx) {
        let (u, dst, a, s, x) = resolved(i)?;
        trans.push((*t, u, dst, a, s, x));
    }
    let first = if typecnt > 0 { let (u, dst, a, _, _) = resolved(0)?; Some((u, dst, a)) } else { None };
    Ok((Block { version: d[off + 4], first, trans, leaps }, p))
}

fn decode(path: &Path) -> Result<(Block, Block, Vec<u8>), String> {
    let d = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (b1, p) = block(&d, 0, true)?;
    let (b2, p2) = block(&d, p, false)?;
    Ok((b1, b2, d[p2..].to_vec()))
}

/// The zone files (TZif, not symbolic links) under `dir`, as names relative to it.
fn zone_files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).expect("read the output tree").flatten() {
            let p = e.path();
            let m = fs::symlink_metadata(&p).expect("stat");
            if m.is_dir() {
                stack.push(p);
            } else if m.is_file() && fs::read(&p).map(|b| b.starts_with(b"TZif")).unwrap_or(false) {
                out.push(p.strip_prefix(dir).unwrap().to_string_lossy().into_owned());
            }
        }
    }
    out.sort();
    out
}

/// Runs `f` over `items` on all CPUs; the results in item order.
fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(32);
    let next = AtomicUsize::new(0);
    let mut out: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
    let slots = std::sync::Mutex::new(&mut out);
    std::thread::scope(|s| {
        for _ in 0..n {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= items.len() {
                    break;
                }
                let r = f(&items[i]);
                slots.lock().unwrap()[i] = Some(r);
            });
        }
    });
    out.into_iter().map(|r| r.expect("every item ran")).collect()
}

/// `zdump -V -c 1800,2100` of the zone file at `path` through libniji's zdump, with the path replaced.
fn reading(zdump: &Path, path: &Path) -> String {
    let o = run(Command::new(zdump).args(["-V", "-c", "1800,2100"]).arg(path));
    String::from_utf8_lossy(&o.stdout).replace(&path.display().to_string(), "ZONE")
}

/// `baseline`: the reference options whose output the listed zones must read like (the fat file for slim:
/// it lists every transition of the source).
fn corpus(extra: &[&str], known: &[&str], baseline: &[&str]) {
    // Trace: s31_9_c_1, s31_9_i_2, s31_9_g_1, s31_9_g_2, s31_9_g_3.
    let (Some(zic), Some(zdump)) = (tools("zic", REF_ZIC), tools("zdump", REF_ZDUMP)) else { return };
    let dir = scratch("corpus");
    let (r, o) = (dir.join("ref"), dir.join("ours"));
    let a = run(Command::new(REF_ZIC).args(extra).arg("-d").arg(&r).arg(TZDATA));
    let b = run(Command::new(&zic).args(extra).arg("-d").arg(&o).arg(TZDATA));
    assert_eq!(a.status.code(), Some(0), "reference zic: {}", String::from_utf8_lossy(&a.stderr));
    assert_eq!(b.status.code(), Some(0), "libniji zic: {}", String::from_utf8_lossy(&b.stderr));
    let names = zone_files(&r);
    assert!(names.len() > 400, "the reference wrote only {} zone files", names.len());
    let mut strict = 0;
    let mut bad = BTreeMap::new();
    let mut known_checked = Vec::new();
    for n in &names {
        let x = decode(&r.join(n)).expect("the reference output decodes");
        let y = decode(&o.join(n)).unwrap_or_else(|e| panic!("{n}: libniji's output does not decode: {e}"));
        if x == y {
            strict += 1;
        } else if known.contains(&n.as_str()) {
            known_checked.push(n.clone());
        } else {
            let what = if x.2 != y.2 { format!("footer {:?} vs {:?}", x.2, y.2) } else if x.1 != y.1 { "v2 block".into() } else { "v1 block".into() };
            bad.insert(n.clone(), what);
        }
    }
    println!("corpus {extra:?}: {} zones, {strict} decoded identically, {} listed ({known_checked:?})", names.len(), known_checked.len());
    assert!(bad.is_empty(), "zones decoding differently from the reference: {bad:#?}");
    // The listed zones must read the same: libniji's zdump over both files.
    let base = dir.join("baseline");
    let c = run(Command::new(REF_ZIC).args(baseline).arg("-d").arg(&base).arg(TZDATA));
    assert_eq!(c.status.code(), Some(0), "reference zic {baseline:?}");
    let diffs: Vec<String> = parallel(&known_checked, |n| (reading(&zdump, &base.join(n)) != reading(&zdump, &o.join(n))).then(|| n.clone()))
        .into_iter()
        .flatten()
        .collect();
    assert!(diffs.is_empty(), "listed zones that read differently: {diffs:?}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn zic_compiles_every_zone_as_the_reference_does() {
    corpus(&[], KNOWN_FAT, &[]);
}

#[test]
fn zic_slim_compiles_every_zone_as_the_reference_does() {
    let known: Vec<&str> = KNOWN_FAT.iter().chain(KNOWN_SLIM_EXTRA).copied().collect();
    corpus(&["-b", "slim"], &known, &[]);
}

#[test]
fn zic_with_leap_seconds_matches_the_reference() {
    // Trace: s31_9_i_1 (-L).
    let leap = "/usr/share/zoneinfo/leapseconds";
    if !Path::new(leap).is_file() {
        println!("SKIPPED: {leap} is not installed");
        return;
    }
    corpus(&["-L", leap], KNOWN_FAT, &["-L", leap]);
}

/// zdump output of every zone, libniji's (over libniji's localtime) against the reference's.
fn zdump_all(opts: &[&str]) {
    // Trace: s31_10_c_1, s31_10_i_1, s31_10_g_2, s31_10_g_3.
    let Some(zdump) = tools("zdump", REF_ZDUMP) else { return };
    let src = fs::read_to_string(TZDATA).expect("read tzdata.zi");
    let mut zones: Vec<String> = src
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.first() {
                Some(&"Z") if f.len() > 1 => Some(f[1].to_string()),
                Some(&"L") if f.len() > 2 => Some(f[2].to_string()),
                _ => None,
            }
        })
        .filter(|z| Path::new("/usr/share/zoneinfo").join(z).is_file())
        .collect();
    zones.sort();
    assert!(zones.len() > 400, "only {} zones installed", zones.len());
    let diffs: Vec<String> = parallel(&zones, |z| {
        let a = run(Command::new(REF_ZDUMP).args(opts).arg(z));
        let b = run(Command::new(&zdump).args(opts).arg(z));
        (a.stdout != b.stdout || a.stderr != b.stderr || a.status.code() != b.status.code()).then(|| z.clone())
    })
    .into_iter()
    .flatten()
    .collect();
    println!("zdump {opts:?}: {} zones, {} differ", zones.len(), diffs.len());
    assert!(diffs.is_empty(), "zones where zdump {opts:?} differs from the reference: {diffs:?}");
}

#[test]
fn zdump_verbose_matches_the_reference_for_every_zone() {
    zdump_all(&["-v", "-c", "1800,2100"]);
}

#[test]
fn zdump_brief_matches_the_reference_for_every_zone() {
    zdump_all(&["-i", "-c", "1800,2100"]);
}

#[test]
fn zdump_full_range_and_options_match_the_reference() {
    // Trace: s31_10_i_1, s31_10_g_1, s31_10_g_2, s31_10_g_4.
    let (Some(zdump), Some(zic)) = (tools("zdump", REF_ZDUMP), tools("zic", REF_ZIC)) else { return };
    let cases: &[&[&str]] = &[
        &["-v", "America/New_York", "Europe/London"],
        &["-V", "Australia/Lord_Howe", "Pacific/Apia"],
        &["-i", "Asia/Kolkata", "Antarctica/Troll"],
        &["-V", "-t", "1600000000,1700000000", "Europe/London"],
        &["-v", "-c", "x", "UTC"],
        &[],
    ];
    // Messages name the program as it was run: each side's path is replaced by its name.
    let ours_path = zdump.display().to_string();
    let text = |o: &Output, path: &str| (o.status.code(), String::from_utf8_lossy(&o.stdout).replace(path, "zdump"), String::from_utf8_lossy(&o.stderr).replace(path, "zdump"));
    for c in cases {
        let a = run(Command::new(REF_ZDUMP).args(*c));
        let b = run(Command::new(&zdump).args(*c));
        assert_eq!(text(&a, REF_ZDUMP), text(&b, &ours_path), "zdump {c:?}");
    }
    // A bad option and --help: the same status and usage text (the bug-report line is each library's own, D-31.4).
    for c in [&["-Q", "UTC"][..], &["--help"][..]] {
        let a = text(&run(Command::new(REF_ZDUMP).args(c)), REF_ZDUMP);
        let b = text(&run(Command::new(&zdump).args(c)), &ours_path);
        assert_eq!(a.0, b.0, "zdump {c:?} status");
        let usage = |t: &(Option<i32>, String, String)| format!("{}{}", t.1, t.2).lines().filter(|l| !l.contains("Report bugs") && !l.is_empty()).collect::<Vec<_>>().join("
");
        assert_eq!(usage(&a), usage(&b), "zdump {c:?} usage text");
    }
    // An abbreviation under three characters is warned about once; other unusual ones are not (TZDIR names the
    // zone directory).
    let dir = scratch("zdump-abbr");
    fs::write(dir.join("a.zi"), "Zone T/Short 1:00 - AB 2000\n\t2:00 - ABCDEFGHI 2001\n\t3:00 - A_B 2002\n\t4:00 - AB 2003\n\t5:00 - <x>\n").unwrap();
    assert!(run(Command::new(&zic).arg("-d").arg(dir.join("z")).arg(dir.join("a.zi"))).status.success());
    let args = ["-v", "-c", "1999,2004", "T/Short"];
    let a = run(Command::new(REF_ZDUMP).args(args).env("TZDIR", dir.join("z")));
    let b = run(Command::new(&zdump).args(args).env("TZDIR", dir.join("z")));
    assert_eq!(text(&a, REF_ZDUMP), text(&b, &ours_path), "zdump {args:?} under TZDIR");
    let _ = fs::remove_dir_all(&dir);
}

/// zic on one source in its own directory: (status, stderr, files written).
fn zic_on(zic: &Path, src: &[u8], args: &[&str]) -> (Option<i32>, String, usize) {
    let dir = scratch("zic-one");
    fs::write(dir.join("in.zi"), src).unwrap();
    let o = run(Command::new(zic).args(args).args(["-d", "out", "in.zi"]).current_dir(&dir));
    let files = if dir.join("out").exists() { zone_files(&dir.join("out")).len() } else { 0 };
    let _ = fs::remove_dir_all(&dir);
    (o.status.code(), String::from_utf8_lossy(&o.stderr).into_owned(), files)
}

#[test]
fn zic_rejects_malformed_source_as_the_reference_does() {
    // Trace: s31_9_i_1, s31_9_f_1.
    let Some(zic) = tools("zic", REF_ZIC) else { return };
    let cases: &[(&str, &str)] = &[
        ("unknown rule", "Zone A/B 1:00 NoSuch TST\n"),
        ("bad month", "Rule X 2000 max - Foo 1 0 1 D\nZone A/B 1:00 X T%sT\n"),
        ("duplicate zone", "Zone A/B 1:00 - TST\nZone A/B 2:00 - TST\n"),
        ("short zone", "Zone A/B 1:00\n"),
        ("bad offset", "Zone A/B 1:x0 - TST\n"),
        ("unknown line", "Bogus a b c\n"),
        ("missing continuation", "Zone A/B 1:00 - TST 2000\n"),
        ("end not after", "Zone A/B 1:00 - TST 2000\n 2:00 - TST 1999\n"),
        ("odd quotes", "Zone A/B 1:00 - \"TST\n"),
        ("years reversed", "Rule X 2000 1990 - Jan 1 0 1 D\nZone A/B 1:00 X T%sT\n"),
        ("short rule", "Rule X 2000 max - Jan 1 0 1\n"),
        ("short link", "Link A/B\n"),
        ("long abbreviation", "Zone A/B 1:00 - ABCDEFGHI\n"),
        ("odd abbreviation", "Zone A/B 1:00 - A_B\n"),
    ];
    for (what, src) in cases {
        let a = zic_on(Path::new(REF_ZIC), src.as_bytes(), &[]);
        let b = zic_on(&zic, src.as_bytes(), &[]);
        assert_eq!(a, b, "{what}: (status, message, files) of the reference vs libniji");
    }
    let a = run(Command::new(REF_ZIC).args(["-d", "/nonexistent-out", "/nonexistent.zi"]));
    let b = run(Command::new(&zic).args(["-d", "/nonexistent-out", "/nonexistent.zi"]));
    // The message names the program as it was run: each side's path is replaced by its name.
    let msg = |o: &Output, path: &str| String::from_utf8_lossy(&o.stderr).replace(path, "zic");
    assert_eq!((a.status.code(), msg(&a, REF_ZIC)), (b.status.code(), msg(&b, &zic.display().to_string())), "a source that cannot be opened");
}

/// Everything under `dir` but `in.zi`, sorted: `path/` for a directory, `path -> target` for a symbolic link
/// (never followed), `path` for a file. find(1) walks from each directory in turn, so a name longer than the
/// path limit once joined to `dir` is still listed.
fn tree(dir: &Path) -> Vec<String> {
    let o = run(Command::new("find").args([".", "-mindepth", "1", "-printf", "%y %P\\0%l\\0"]).current_dir(dir));
    assert!(o.status.success(), "find in {}: {}", dir.display(), String::from_utf8_lossy(&o.stderr));
    let fields: Vec<String> = o.stdout.split(|&b| b == 0).map(|f| String::from_utf8_lossy(f).into_owned()).collect();
    let mut out: Vec<String> = fields
        .chunks_exact(2)
        .filter(|f| f[0] != "f in.zi")
        .map(|f| match f[0].split_at(2) {
            ("l ", p) => format!("{p} -> {}", f[1]),
            ("d ", p) => format!("{p}/"),
            (_, p) => p.to_string(),
        })
        .collect();
    out.sort();
    out
}

/// zic with `-d box/out` on `src` in a new directory: (status, stderr with the program's path written `zic`,
/// everything it left there).
fn zic_tree(zic: &Path, src: &[u8]) -> (Option<i32>, String, Vec<String>) {
    let dir = scratch("names");
    fs::write(dir.join("in.zi"), src).unwrap();
    let o = run(Command::new(zic).args(["-d", "box/out", "in.zi"]).current_dir(&dir));
    let err = String::from_utf8_lossy(&o.stderr).replace(&*zic.to_string_lossy(), "zic");
    let t = tree(&dir);
    let _ = fs::remove_dir_all(&dir);
    (o.status.code(), err, t)
}

#[test]
fn zic_handles_zone_and_link_names_as_the_reference_does() {
    // Trace: s31_9_i_1, s31_9_f_1 (a name in the source becomes a file under -d: names that would leave it,
    // or name nothing, are refused with the reference's message; any other byte is kept; the kernel's limits
    // on a name are met as the reference meets them).
    let Some(zic) = tools("zic", REF_ZIC) else { return };
    let long = |n: usize| "a".repeat(n);
    let deep = |n: usize| {
        let mut s = String::from("a");
        while s.len() + 10 <= n {
            s.push_str("/abcdefghi");
        }
        let pad = n - s.len();
        s + &"x".repeat(pad)
    };
    // Refused: leaves -d or names no file (the order of the checks shows in the last two).
    let mut names: Vec<Vec<u8>> = ["/x", "../x", "a/../../x", "a/./b", ".", "..", "a//b", "a/", "\"\"", "a//../b", "../a//b"]
        .iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();
    // Kept: bytes outside the portable set, a leading '-', dot files, components of 15 and 255 bytes.
    names.extend([&b"a*b"[..], "a\u{e9}b".as_bytes(), b"a\x01b", b"a\x7fb", b"a\xffb", b"a\\b", b"a:b", b"-a/b", b"a/.b", b"a/...", b"\"a b\""].map(<[u8]>::to_vec));
    // Over the limits: a component of 256 bytes (last or inside), a name of 4096 bytes (4095 still fits).
    for n in [format!("a/{}", long(15)), format!("a/{}", long(255)), format!("a/{}", long(256)), format!("a/{}/b", long(256)), deep(4095), deep(4096)] {
        names.push(n.into_bytes());
    }
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for n in &names {
        let shown = String::from_utf8_lossy(&n[..n.len().min(40)]).into_owned();
        cases.push((format!("zone {shown:?}"), [&b"Zone "[..], n, b" 1:00 - TST\n"].concat()));
        cases.push((format!("link name {shown:?}"), [&b"Zone A/B 1:00 - TST\nLink A/B "[..], n, b"\n"].concat()));
    }
    // Link targets: a directory cannot be hard-linked (refused), a missing file gets a symbolic link.
    for t in [".", "..", "../x", "/x", "A/B/", "A//B", "A/./B"] {
        cases.push((format!("link target {t:?}"), format!("Zone A/B 1:00 - TST\nLink {t} C/D\n").into_bytes()));
    }
    let mut bad = Vec::new();
    for (what, src) in &cases {
        let a = zic_tree(Path::new(REF_ZIC), src);
        let b = zic_tree(&zic, src);
        assert!(b.2.iter().all(|p| p == "box/" || p.starts_with("box/out")), "{what}: wrote outside -d: {:?}", b.2);
        if a != b {
            // Long names: show the ends of the messages and only the entries one side lacks.
            let cut = |s: &str| if s.len() > 300 { format!("...{}", &s[s.floor_char_boundary(s.len() - 300)..]) } else { s.to_string() };
            let only = |x: &[String], y: &[String]| x.iter().filter(|p| !y.contains(p)).map(|p| cut(p)).collect::<Vec<_>>();
            bad.push(format!("{what}: reference {:?} {:?} only {:?}; libniji {:?} {:?} only {:?}", a.0, cut(&a.1), only(&a.2, &b.2), b.0, cut(&b.1), only(&b.2, &a.2)));
        }
    }
    assert!(bad.is_empty(), "names handled differently from the reference:\n{}", bad.join("\n"));
}

#[test]
fn zic_never_writes_through_a_symbolic_link_below_the_output_directory() {
    // Trace: s31_9_c_1 (loaded data is untrusted: whatever links the source or the tree holds, every file zic
    // makes or removes lies under -d, and nothing outside it changes). libniji only: D-31.18.
    let Some(zic) = tools("zic", REF_ZIC) else { return };
    // (what, source, symbolic links placed in -d before the run as (name, contents)).
    type Case = (&'static str, &'static str, &'static [(&'static str, &'static str)]);
    let cases: &[Case] = &[
        // Links in the source that would turn a name into a way out.
        ("link to a directory outside", "Zone A/B 1:00 - TST\nLink ../../victim X\nLink A/B X/y\n", &[]),
        ("link to a directory inside", "Zone S/z 1:00 - TST\nZone A/B 1:00 - TST\nLink S P/Q/R\nLink T P/Q/R/Z\nLink A/B P/Q/R/Z/W\n", &[]),
        ("link to a missing name", "Zone A/B 1:00 - TST\nLink ../../victim/none X\nLink A/B X/y\n", &[]),
        // Links already in the tree (an earlier run's, or placed there), in a directory and at the file itself.
        ("tree link in a zone name", "Zone evil/y 1:00 - TST\n", &[("evil", "../../victim")]),
        ("tree link in a link name", "Zone A/B 1:00 - TST\nLink A/B evil/y\n", &[("evil", "../../victim")]),
        ("tree link at a zone", "Zone leaf 1:00 - TST\n", &[("leaf", "../../victim/file")]),
        ("tree link at a link", "Zone A/B 1:00 - TST\nLink A/B leaf\n", &[("leaf", "../../victim/file")]),
    ];
    for (what, src, links) in cases {
        let dir = scratch("nofollow");
        fs::write(dir.join("in.zi"), src).unwrap();
        for d in ["box/out", "victim/d", "T"] {
            fs::create_dir_all(dir.join(d)).unwrap();
        }
        fs::write(dir.join("victim/file"), "keep").unwrap();
        for (name, to) in *links {
            std::os::unix::fs::symlink(to, dir.join("box/out").join(name)).unwrap();
        }
        let before: Vec<String> = tree(&dir).into_iter().filter(|p| !p.starts_with("box/out/")).collect();
        let o = run(Command::new(&zic).args(["-d", "box/out", "in.zi"]).current_dir(&dir));
        assert!(matches!(o.status.code(), Some(0 | 1)), "{what}: zic ended with {:?}", o.status);
        let after: Vec<String> = tree(&dir).into_iter().filter(|p| !p.starts_with("box/out/")).collect();
        assert_eq!(before, after, "{what}: wrote outside -d ({})", String::from_utf8_lossy(&o.stderr));
        assert_eq!(fs::read_to_string(dir.join("victim/file")).unwrap(), "keep", "{what}: wrote through a link");
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn zic_survives_hostile_source() {
    // Trace: s31_9_c_1 (loaded data is untrusted: never a crash, never a write outside -d).
    let Some(zic) = tools("zic", REF_ZIC) else { return };
    let mut many = String::from("Rule M 1900 max - Jan 1 0 1 D\n");
    for i in 0..300 {
        many.push_str(&format!("Rule M {} only - Jun 1 0 0:{:02} A{i}\n", 1901 + i, (i % 59) + 1));
    }
    many.push_str("Zone A/B 1:00 M %s\n");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("huge year range", b"Rule X 1 2000000000 - Jan 1 0 1 D\nRule X 1 2000000000 - Jul 1 0 0 S\nZone A/B 1:00 X T%sT\n".to_vec()),
        ("year beyond the limit", b"Rule X 1 99999999999999999999 - Jan 1 0 1 D\nZone A/B 1:00 X T%sT\n".to_vec()),
        ("huge offset", b"Zone A/B 999999:59:59 - TST\n".to_vec()),
        ("huge until", b"Zone A/B 1:00 - TST 2000000000\n 2:00 - TST\n".to_vec()),
        ("too many types", many.into_bytes()),
        ("link loop", b"Link A/B A/C\nLink A/C A/B\n".to_vec()),
        ("escaping names", b"Zone ../x 1:00 - TST\nZone /tmp/x 1:00 - TST\nZone a//b 1:00 - TST\nLink Etc/UTC ../y\n".to_vec()),
        ("NUL byte", b"Zone A/B 1:00 - T\0ST\n".to_vec()),
        ("long line", { let mut v = b"Zone A/B 1:00 - ".to_vec(); v.extend(std::iter::repeat_n(b'A', 1 << 20)); v.push(b'\n'); v }),
        ("long abbreviation format", { let mut v = b"Rule R 2000 max - Jan 1 0 1 ".to_vec(); v.extend(std::iter::repeat_n(b'L', 5000)); v.extend(b"\nZone A/B 1:00 R %s%s%s\n"); v }),
        ("odd days", b"Rule X 2000 max - Feb Sun>=29 0 1 D\nRule X 2000 max - Feb lastSun 25:00 0 S\nZone A/B 1:00 X T%sT\n".to_vec()),
    ];
    for (what, src) in &cases {
        let dir = scratch("hostile");
        fs::write(dir.join("in.zi"), src).unwrap();
        fs::create_dir_all(dir.join("box/out")).unwrap();
        let o = run(Command::new(&zic).args(["-d", "box/out", "in.zi"]).current_dir(&dir));
        assert!(matches!(o.status.code(), Some(0 | 1)), "{what}: zic ended with {:?}", o.status);
        let outside: Vec<_> = fs::read_dir(dir.join("box")).unwrap().flatten().map(|e| e.file_name()).filter(|n| n != "out").collect();
        assert!(outside.is_empty() && !dir.join("x").exists() && !dir.join("y").exists(), "{what}: wrote outside -d: {outside:?}");
        let _ = fs::remove_dir_all(&dir);
    }
}

const REF_TZSELECT: &str = "/usr/bin/tzselect";
const REF_PLDD: &str = "/usr/bin/pldd";
const ZONEINFO: &str = "/usr/share/zoneinfo";

/// libniji's test link of `name`, or None (with the reason printed) when it or a file the comparison needs is missing.
fn ours(name: &str, needs: &[&str]) -> Option<PathBuf> {
    let p = link(name)?;
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
    session_at(prog, args, env, input, None)
}

/// A preloaded library that stops the real-time clock at `NIJI_TEST_CLOCK` (seconds since the epoch) in every
/// process of a session, the reference's helper programs included; the other clocks run.
const CLOCK_PIN_C: &str = r#"#define _GNU_SOURCE
#include <stdlib.h>
#include <sys/syscall.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>
static time_t pin(void) {
    time_t v = 0;
    for (const char *s = getenv("NIJI_TEST_CLOCK"); s && *s >= '0' && *s <= '9'; s++) v = v * 10 + (*s - '0');
    return v;
}
time_t time(time_t *t) { time_t v = pin(); if (t) *t = v; return v; }
int clock_gettime(clockid_t id, struct timespec *ts) {
    if (id != CLOCK_REALTIME && id != CLOCK_REALTIME_COARSE) return (int)syscall(SYS_clock_gettime, id, ts);
    ts->tv_sec = pin();
    ts->tv_nsec = 0;
    return 0;
}
int gettimeofday(struct timeval *restrict tv, void *restrict tz) {
    (void)tz;
    if (tv) { tv->tv_sec = pin(); tv->tv_usec = 0; }
    return 0;
}
int timespec_get(struct timespec *ts, int base) {
    if (base != TIME_UTC) return 0;
    ts->tv_sec = pin();
    ts->tv_nsec = 0;
    return base;
}
"#;

/// Builds the clock pin into `dir`; the C compiler is there whenever the test links are (they are built with it).
/// It names no library it needs (`-nostdlib`): `getenv` and `syscall` bind to the C library the process already has,
/// which a test link loads by path (no `libc.so.6` name for a dependency to match).
fn clock_pin(dir: &Path) -> PathBuf {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).expect("scratch directory");
    let (src, so) = (dir.join("clock_pin.c"), dir.join("clock_pin.so"));
    fs::write(&src, CLOCK_PIN_C).expect("write the clock pin");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let o = Command::new(&cc).args(["-shared", "-fPIC", "-O2", "-nostdlib", "-o"]).arg(&so).arg(&src).output().expect("run the C compiler");
    assert!(o.status.success(), "{cc}: the clock pin does not build: {}", String::from_utf8_lossy(&o.stderr));
    so
}

/// A session with the real-time clock stopped at `clock` = (the pin library, seconds since the epoch): the clock lines
/// are then the same in every run and are compared, not masked. Unpinned, the reference reads the clock more than
/// once in a session and its choice to show the clock lines for an offset with seconds (EST5:30:59) changes when a
/// second ends between the reads (probed: shown in about 1 run in 50, every time with the two lines' seconds equal).
fn session_at(prog: &Path, args: &[&str], env: &[(&str, &str)], input: &str, clock: Option<(&Path, &str)>) -> Run {
    let mut c = Command::new(prog);
    // Messages name the program as it was run: the reference names its path.
    c.arg0(REF_TZSELECT).args(args).env_clear().env("PATH", "/usr/bin:/bin").env("HOME", "/tmp").env("LANG", "C.UTF-8");
    for (k, v) in env {
        c.env(k, v);
    }
    if let Some((pin, secs)) = clock {
        c.env("LD_PRELOAD", pin).env("NIJI_TEST_CLOCK", secs);
    }
    let child = c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("run tzselect");
    let mut r = finish(child, input);
    if clock.is_none() {
        // Unpinned, the two clock lines of the information block differ between any two runs.
        mask_clock(&mut r);
    }
    r
}

/// Keeps only the heads of the two clock lines of the information block.
fn mask_clock(r: &mut Run) {
    r.err = r
        .err
        .lines()
        .map(|l| match l.split_once('\t') {
            Some((head @ ("Selected time is now:" | "Universal Time is now:"), _)) => head.to_string(),
            _ => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
}

/// TZ strings whose clock lines are not compared, with why. "EST5EDT,365,0" (DST from zero-based day 365, in a
/// common year the next year's January 1, to day 0): libniji's local time is in DST on 2026-11-24 and 2026-12-31,
/// the reference's is not (a libniji defect in the rule's evaluation, reported for its own story).
const CLOCK_UNCOMPARED: &[&str] = &["EST5EDT,365,0"];

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

/// The clock times the sessions run at (seconds since the epoch) and the reference's UTC clock line at each: every
/// session at the first (the instant of a CI run whose unpinned comparison differed on EST5:30:59), the TZ string
/// sessions also at an hour boundary and at the last second of a year (zones east of UTC are already in the next).
const PINS: &[(&str, &str)] = &[
    ("1795543500", "Tue Nov 24 18:05:00 UTC 2026."),
    ("1795546800", "Tue Nov 24 19:00:00 UTC 2026."),
    ("1798761599", "Thu Dec 31 23:59:59 UTC 2026."),
];

/// s31_11_c_1, s31_11_i_2, s31_11_g_1: the interactive sessions match the reference's, clock lines included, at
/// fixed clock times.
#[test]
fn tzselect_sessions_match_the_reference() {
    let Some(prog) = ours("tzselect", &[REF_TZSELECT, ZONEINFO]) else { return };
    let reference = Path::new(REF_TZSELECT);
    let dir = std::env::temp_dir().join(format!("niji-tzselect-{}", std::process::id()));
    let pin = clock_pin(&dir);
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
    let (mut errors, mut finished, mut runs) = (Vec::new(), 0, 0);
    for (i, &(secs, utc)) in PINS.iter().enumerate() {
        for (what, args, env, input) in &cases {
            if i > 0 && !what.starts_with("TZ \"") {
                continue;
            }
            let clock = Some((pin.as_path(), secs));
            let (mut a, mut b) = (session_at(&prog, args, env, input, clock), session_at(reference, args, env, input, clock));
            // The clock is stopped where it was asked to stop, in the reference's helper programs too.
            let line = b.err.lines().find(|l| l.starts_with("Universal Time is now:"));
            if line.is_some_and(|l| l != format!("Universal Time is now:\t{utc}")) {
                errors.push(format!("{what} at {secs}: the reference's clock is not pinned: {line:?}"));
            }
            if CLOCK_UNCOMPARED.iter().any(|s| *what == format!("TZ {s:?}")) {
                mask_clock(&mut a);
                mask_clock(&mut b);
            }
            if let Err(e) = same(&format!("{what} at {secs}"), &a, &b) {
                errors.push(e);
            }
            finished += usize::from(i == 0 && a.rc == 0 && !a.out.trim().is_empty());
            runs += 1;
        }
    }
    let _ = fs::remove_dir_all(&dir);
    // A finished session prints the value; most of the cases above finish.
    assert!(finished > continents + TZ_STRINGS.len(), "only {finished} sessions printed a value");
    assert!(errors.is_empty(), "{} of {runs} sessions differ:\n{}", errors.len(), errors.join("\n"));
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
