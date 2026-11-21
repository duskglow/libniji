//! TZ-TOOLS: libniji's `zic` and `zdump` (`cargo xtask so` test links under target/niji-test) against the
//! reference programs and the reference tzdata source, run black-box on the host.
//!
//! * The corpus: every zone of the installed `tzdata.zi` compiled by both programs (fat, the default, and
//!   `-b slim`) decodes to the same transitions, indicators, first type, leap records and footer, except the
//!   zones listed in `KNOWN_*` (reasons there), which must still read the same through `zdump`.
//! * zdump: libniji's zdump over libniji's localtime prints what the reference zdump prints, for every zone.
//! * Malformed source: the same exit status and message; hostile source: never a crash, never a file outside
//!   the output directory.
//!
//! Trace: s31_9_c_1, s31_9_i_1, s31_9_i_2, s31_9_g_1, s31_9_g_2, s31_9_g_3, s31_9_f_1, s31_10_c_1,
//! s31_10_i_1, s31_10_g_1, s31_10_g_2, s31_10_g_3, s31_10_g_4.

#![allow(clippy::disallowed_methods)] // a host tool's test: S8 governs the library crates only

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const TZDATA: &str = "/usr/share/zoneinfo/tzdata.zi";
const REF_ZIC: &str = "/usr/sbin/zic";
const REF_ZDUMP: &str = "/usr/bin/zdump";

/// Fat output: zones whose decoded content differs from the reference's while reading the same. Tbilisi and
/// Lisbon: the reference keeps one transition that changes nothing a reader sees (a line boundary inside +05
/// DST in 1997; LMT to the same LMT in 1884); Warsaw: the 1918 change to EET carries the standard-time
/// indicator of the coinciding rule. Links share their target's file.
const KNOWN_FAT: &[&str] = &["Asia/Tbilisi", "Europe/Lisbon", "Portugal", "Europe/Warsaw", "Poland"];
/// Slim output: the same, plus zones where the two hand over from the listed transitions to the footer at a
/// different transition (libniji keeps one the footer also gives, or leaves one to it).
const KNOWN_SLIM_EXTRA: &[&str] = &["America/Ciudad_Juarez", "America/Indiana/Winamac", "America/Ojinaga", "Asia/Gaza", "Asia/Hebron"];
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root").to_path_buf()
}

/// libniji's test link of `name`, or None (with the reason printed) when it or the reference is missing.
fn tools(name: &str, reference: &str) -> Option<PathBuf> {
    let ours = root().join("target/niji-test").join(name);
    if !ours.is_file() {
        println!("SKIPPED: {} is not built (run `cargo xtask so` first)", ours.display());
        return None;
    }
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

fn corpus(extra: &[&str], known: &[&str]) {
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
    let diffs: Vec<String> = parallel(&known_checked, |n| (reading(&zdump, &r.join(n)) != reading(&zdump, &o.join(n))).then(|| n.clone()))
        .into_iter()
        .flatten()
        .collect();
    assert!(diffs.is_empty(), "listed zones that read differently: {diffs:?}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn zic_compiles_every_zone_as_the_reference_does() {
    corpus(&[], KNOWN_FAT);
}

#[test]
fn zic_slim_compiles_every_zone_as_the_reference_does() {
    let known: Vec<&str> = KNOWN_FAT.iter().chain(KNOWN_SLIM_EXTRA).copied().collect();
    corpus(&["-b", "slim"], &known);
}

#[test]
fn zic_with_leap_seconds_matches_the_reference() {
    // Trace: s31_9_i_1 (-L).
    let leap = "/usr/share/zoneinfo/leapseconds";
    if !Path::new(leap).is_file() {
        println!("SKIPPED: {leap} is not installed");
        return;
    }
    corpus(&["-L", leap], KNOWN_FAT);
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
        &["-Q", "UTC"],
        &["--help"],
        &[],
    ];
    for c in cases {
        let a = run(Command::new(REF_ZDUMP).args(*c));
        let b = run(Command::new(&zdump).args(*c));
        assert_eq!((a.status.code(), String::from_utf8_lossy(&a.stdout), String::from_utf8_lossy(&a.stderr)), (b.status.code(), String::from_utf8_lossy(&b.stdout), String::from_utf8_lossy(&b.stderr)), "zdump {c:?}");
    }
    // An abbreviation under three characters is warned about once; other unusual ones are not (TZDIR names the
    // zone directory).
    let dir = scratch("zdump-abbr");
    fs::write(dir.join("a.zi"), "Zone T/Short 1:00 - AB 2000\n\t2:00 - ABCDEFGHI 2001\n\t3:00 - A_B 2002\n\t4:00 - AB 2003\n\t5:00 - <x>\n").unwrap();
    assert!(run(Command::new(&zic).arg("-d").arg(dir.join("z")).arg(dir.join("a.zi"))).status.success());
    let args = ["-v", "-c", "1999,2004", "T/Short"];
    let a = run(Command::new(REF_ZDUMP).args(args).env("TZDIR", dir.join("z")));
    let b = run(Command::new(&zdump).args(args).env("TZDIR", dir.join("z")));
    assert_eq!((a.stdout, a.stderr), (b.stdout, b.stderr));
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
    assert_eq!((a.status.code(), a.stderr), (b.status.code(), b.stderr), "a source that cannot be opened");
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
