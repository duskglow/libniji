//! niji-locale's output against the reference localedef's, for the locales of issue #14 (en_US,
//! de_DE, fr_FR, ja_JP, tr_TR in UTF-8) and one single-byte charmap (en_US ISO-8859-1).
//!
//! s31_8_c_1, s31_8_g_1: the same source and charmap compiled by both. Every item of the string,
//! number and byte categories is compared byte for byte; `LC_CTYPE`'s fixed items byte for byte
//! and its tables by what the C library reads from them (`niji::locale`): every class bit, map
//! value and width of every code point below 0x110000; `LC_COLLATE` by the order `strcoll`,
//! `wcscoll` and bracket ranges give over a corpus (s31_8_c_1: the key bytes are libniji's own,
//! the order is the reference's). Both use the installed sources
//! (`/usr/share/i18n`); a case whose source, charmap or reference program is absent is skipped
//! with the reason printed.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, missing_docs)]

use std::path::{Path, PathBuf};
use std::process::Command;

use niji::locale::ctype::CtypeFile;
use niji::locale::file::Category;
use niji::locale::table3::Table3;
use niji_locale::{Files, Kind, TryVec};

const I18N: &str = "/usr/share/i18n";
/// The decompressed size a charmap may have (the largest installed is under 4 MiB).
const CHARMAP_LIMIT: usize = 64 << 20;

struct Disk;

impl Files for Disk {
    fn read(&mut self, kind: Kind, name: &[u8]) -> niji_locale::Result<TryVec<u8>> {
        let name = String::from_utf8_lossy(name).into_owned();
        let missing = niji_locale::Error::missing("no such file");
        let data = match kind {
            Kind::Locale => std::fs::read(source_path(&name)).map_err(|_| missing)?,
            Kind::Charmap => match std::fs::read(format!("{I18N}/charmaps/{name}")) {
                Ok(d) => d,
                Err(_) => {
                    let gz = std::fs::read(format!("{I18N}/charmaps/{name}.gz")).map_err(|_| missing)?;
                    return niji::charmap::inflate::gunzip(&gz, CHARMAP_LIMIT).map_err(|_| niji_locale::Error::missing("bad gzip"));
                }
            },
        };
        TryVec::from_slice(&data).map_err(Into::into)
    }
}

/// The category files' names, by category number (`None`: no file).
const FILES: [Option<&str>; 13] = [
    Some("LC_CTYPE"),
    Some("LC_NUMERIC"),
    Some("LC_TIME"),
    Some("LC_COLLATE"),
    Some("LC_MONETARY"),
    Some("LC_MESSAGES/SYS_LC_MESSAGES"),
    None,
    Some("LC_PAPER"),
    Some("LC_NAME"),
    Some("LC_ADDRESS"),
    Some("LC_TELEPHONE"),
    Some("LC_MEASUREMENT"),
    Some("LC_IDENTIFICATION"),
];

fn leak(v: Vec<u8>) -> &'static [u8] {
    Box::leak(v.into_boxed_slice())
}

/// The file of the locale source `locale`: an absolute path as given, else an installed source.
fn source_path(locale: &str) -> PathBuf {
    if locale.starts_with('/') { PathBuf::from(locale) } else { Path::new(I18N).join("locales").join(locale) }
}

/// Compile `locale` for `charmap` with the reference into a fresh directory; `None` (reason
/// printed) when it cannot run.
fn reference(locale: &str, charmap: &str) -> Option<PathBuf> {
    if !source_path(locale).exists() {
        println!("skip {locale}.{charmap}: no source {}", source_path(locale).display());
        return None;
    }
    // One directory per call: two tests may compile the same locale at once (they did share one,
    // and one removed it under the other).
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("niji-locale-ref-{}-{call}-{}.{charmap}", std::process::id(), locale.replace('/', "_")));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).ok()?;
    let out = dir.join("out");
    let run = Command::new("localedef").args(["-i", locale, "-c", "-f", charmap]).arg(&out).env("LC_ALL", "C").output();
    match run {
        Err(e) => {
            println!("skip {locale}.{charmap}: the reference localedef cannot run: {e}");
            None
        }
        // -c: warnings give exit status 1 and still write the locale.
        Ok(o) if out.join("LC_CTYPE").exists() => {
            let _ = o;
            Some(dir)
        }
        Ok(o) => {
            println!("skip {locale}.{charmap}: the reference wrote no locale (status {:?}): {}", o.status.code(), String::from_utf8_lossy(&o.stderr));
            None
        }
    }
}

/// Item-by-item differences of one category; at most `limit` lines.
fn diff_items(what: &str, theirs: Category, ours: Category, skip: &[usize], out: &mut Vec<String>) {
    if theirs.magic() != ours.magic() || theirs.len() != ours.len() {
        out.push(format!("{what}: magic/count {:#x}/{} vs ours {:#x}/{}", theirs.magic(), theirs.len(), ours.magic(), ours.len()));
        return;
    }
    for i in (0..theirs.len()).filter(|i| !skip.contains(i)) {
        let (t, o) = (theirs.item(i), ours.item(i));
        if t != o {
            let show = |b: &[u8]| {
                let s: Vec<String> = b.iter().take(48).map(|x| format!("{x:02x}")).collect();
                format!("{}{}", s.join(""), if b.len() > 48 { "…" } else { "" })
            };
            out.push(format!("{what} item {i}: len {} vs ours {}\n  ref  {}\n  ours {}", t.len(), o.len(), show(t), show(o)));
        }
    }
}

fn words(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect()
}

/// The zero-ended run of `tbl` from `at`.
fn run(tbl: &[u32], at: usize) -> &[u32] {
    let rest = tbl.get(at..).unwrap_or(&[]);
    &rest[..rest.iter().position(|&c| c == 0).unwrap_or(rest.len())]
}

/// `LC_CTYPE` items 61-65 decoded: each from-string with its alternatives, in table order.
fn translit(c: Category) -> Vec<(Vec<u32>, Vec<Vec<u32>>)> {
    let n = c.word(61) as usize;
    let (fi, ft, ti, tt) = (words(c.item(62)), words(c.item(63)), words(c.item(64)), words(c.item(65)));
    (0..n)
        .map(|k| {
            let from = run(&ft, fi.get(k).copied().unwrap_or(u32::MAX) as usize).to_vec();
            let mut at = ti.get(k).copied().unwrap_or(u32::MAX) as usize;
            let mut alts = Vec::new();
            while tt.get(at).is_some_and(|&c| c != 0) {
                let alt = run(&tt, at).to_vec();
                at += alt.len() + 1;
                alts.push(alt);
            }
            (from, alts)
        })
        .collect()
}

fn names(item: &[u8]) -> Vec<Vec<u8>> {
    item.split(|&b| b == 0).take_while(|s| !s.is_empty()).map(<[u8]>::to_vec).collect()
}

/// `LC_CTYPE` compared by what the reader sees: the fixed items, then every class, map and width.
fn diff_ctype(theirs: Category, ours: Category, out: &mut Vec<String>) {
    let class_offset = theirs.word(17) as usize;
    let map_offset = theirs.word(18) as usize;
    // 12 (WIDTH) and the class and map tables are compared by lookups; 63-65 (the
    // transliteration strings) decoded, since libniji stores each distinct alternative list once.
    diff_items("LC_CTYPE", theirs, ours, &[12, 64, 65].into_iter().chain(class_offset..theirs.len()).collect::<Vec<_>>(), out);
    if translit(theirs) != translit(ours) {
        let (t, o) = (translit(theirs), translit(ours));
        let first = t.iter().zip(o.iter()).find(|(a, b)| a != b);
        out.push(format!("LC_CTYPE transliteration: {} entries vs ours {}; first difference {first:x?}", t.len(), o.len()));
    }
    if CtypeFile::parse(theirs).is_none() || CtypeFile::parse(ours).is_none() {
        out.push("LC_CTYPE: a file the reader refuses".into());
        return;
    }
    let tables = |c: Category, first: usize, n: usize| -> Vec<Option<Table3>> { (first..first + n).map(|i| Table3::parse(c.item(i))).collect() };
    let classes = names(theirs.item(10));
    let maps = names(theirs.item(11));
    let (tc, oc) = (tables(theirs, class_offset, classes.len()), tables(ours, ours.word(17) as usize, classes.len()));
    let (tm, om) = (tables(theirs, map_offset, maps.len()), tables(ours, ours.word(18) as usize, maps.len()));
    let (tw, ow) = (Table3::parse(theirs.item(12)), Table3::parse(ours.item(12)));
    let report = |what: String, firsts: &mut Vec<String>| {
        if firsts.len() < 10 {
            firsts.push(what);
        }
    };
    for (k, name) in classes.iter().enumerate() {
        let name = String::from_utf8_lossy(name);
        let (Some(Some(a)), Some(Some(b))) = (tc.get(k), oc.get(k)) else {
            out.push(format!("LC_CTYPE class {name}: table missing"));
            continue;
        };
        let (mut firsts, mut count) = (Vec::new(), 0u32);
        for wc in 0..0x11_0000u32 {
            if a.bit(wc) != b.bit(wc) {
                count += 1;
                report(format!("{wc:#06x} ref {}", a.bit(wc)), &mut firsts);
            }
        }
        if count > 0 {
            out.push(format!("LC_CTYPE class {name}: {count} differ: {}", firsts.join(", ")));
        }
    }
    for (k, name) in maps.iter().enumerate() {
        let name = String::from_utf8_lossy(name);
        let (Some(Some(a)), Some(Some(b))) = (tm.get(k), om.get(k)) else {
            out.push(format!("LC_CTYPE map {name}: table missing"));
            continue;
        };
        let (mut firsts, mut count) = (Vec::new(), 0u32);
        for wc in 0..0x11_0000u32 {
            if a.value_i32(wc) != b.value_i32(wc) {
                count += 1;
                report(format!("{wc:#06x} ref {:+} ours {:+}", a.value_i32(wc), b.value_i32(wc)), &mut firsts);
            }
        }
        if count > 0 {
            out.push(format!("LC_CTYPE map {name}: {count} differ: {}", firsts.join(", ")));
        }
    }
    match (tw, ow) {
        (Some(a), Some(b)) => {
            let (mut firsts, mut count) = (Vec::new(), 0u32);
            for wc in 0..0x11_0000u32 {
                // What wcwidth reads: an absent entry is 0xff (−1), as a stored 0xff is.
                let (x, y) = (a.value_u8(wc).unwrap_or(0xff), b.value_u8(wc).unwrap_or(0xff));
                if x != y {
                    count += 1;
                    report(format!("{wc:#06x} ref {x} ours {y}"), &mut firsts);
                }
            }
            if count > 0 {
                out.push(format!("LC_CTYPE width: {count} differ: {}", firsts.join(", ")));
            }
        }
        _ => out.push("LC_CTYPE width: table missing".into()),
    }
}

/// The characters the collation corpus is made of: ASCII, Latin-1, Latin Extended-A, the
/// contraction parts of the sources (L with middle dot), Turkish letters, Greek, Cyrillic,
/// kana, Han inside and outside the sources' `..` range, and code points no source orders.
fn alphabet(utf8: bool) -> Vec<char> {
    let mut v: Vec<char> = (0x20..0x7f).chain(0xa0..0x180).filter(|&c| utf8 || c < 0x100).filter_map(char::from_u32).collect();
    if utf8 {
        for cp in [0x387, 0x391, 0x3b1, 0x3ac, 0x410, 0x430, 0x419, 0x439, 0x306, 0x301, 0x3042, 0x30a2, 0x4e00, 0x4e01, 0x9fa5, 0x3400, 0x378, 0x1f600, 0x10fffd, 0xe000, 0x2014, 0x20ac] {
            v.extend(char::from_u32(cp));
        }
    }
    v
}

/// A deterministic corpus: every character alone, then strings of 2 to 6 characters drawn from
/// a weighted mix (letters most, then accents, spaces, punctuation and the rest).
fn corpus(utf8: bool) -> Vec<Vec<char>> {
    let a = alphabet(utf8);
    let common: Vec<char> = "aAbBcCeEiIlLoOsSuUzZ  -'.0129éÉèçÇßæøåäöüÄÖÜıİğĞşŞ·".chars().filter(|c| utf8 || (*c as u32) < 0x100).collect();
    let mut out: Vec<Vec<char>> = a.iter().map(|&c| vec![c]).collect();
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for _ in 0..4000 {
        let len = 2 + (next() % 5) as usize;
        let s = (0..len).map(|_| if next() % 4 == 0 { a[(next() % a.len() as u64) as usize] } else { common[(next() % common.len() as u64) as usize] }).collect();
        out.push(s);
    }
    out
}

/// Whether two collations order `corpus` alike (sorted by the first, every adjacent pair has
/// the same order under the second; both are total preorders, so they then agree on every
/// pair); the first disagreements, at most 5.
fn order_diffs<S: Clone + std::fmt::Debug>(corpus: &[S], theirs: impl Fn(&S, &S) -> std::cmp::Ordering, ours: impl Fn(&S, &S) -> std::cmp::Ordering) -> Vec<String> {
    let mut v = corpus.to_vec();
    v.sort_by(&theirs);
    v.windows(2).filter(|w| theirs(&w[0], &w[1]) != ours(&w[0], &w[1])).take(5).map(|w| format!("{:?} {:?}: ref {:?}, ours {:?}", w[0], w[1], theirs(&w[0], &w[1]), ours(&w[0], &w[1]))).collect()
}

/// `LC_COLLATE` by what the C library does with it: `strcoll` (multibyte) and `wcscoll` order
/// over [`corpus`], and the collation-sequence order of the alphabet's characters
/// (bracket-expression ranges). Keys equal byte for byte are counted, not required.
fn diff_collate(theirs: Category, ours: Category, utf8: bool, out: &mut Vec<String>) {
    diff_collate_of(theirs, ours, utf8, |_| true, out);
}

/// [`diff_collate`] over the strings of the corpus and the characters of the alphabet that
/// `keep` accepts.
fn diff_collate_of(theirs: Category, ours: Category, utf8: bool, keep: fn(char) -> bool, out: &mut Vec<String>) {
    use niji::locale::collate::{compare, key_bytes, Collate, Collation};
    let (t, o) = match (Collate::parse(theirs), Collate::parse(ours)) {
        (Some(Collation::Rules(t)), Some(Collation::Rules(o))) => (t, o),
        // Both without rules: both order by code point.
        (Some(Collation::CodePoint), Some(Collation::CodePoint)) => return,
        (t, o) => {
            let form = |c: &Option<Collation>| match c {
                Some(Collation::Rules(_)) => "rules",
                Some(Collation::CodePoint) => "code point",
                None => "refused",
            };
            out.push(format!("LC_COLLATE: the reference's is {}, ours {}", form(&t), form(&o)));
            return;
        }
    };
    let words: Vec<Vec<char>> = corpus(utf8).into_iter().filter(|s| s.iter().all(|&c| keep(c))).collect();
    let mb = |s: &Vec<char>| -> Vec<u8> { if utf8 { s.iter().collect::<String>().into_bytes() } else { s.iter().map(|&c| c as u8).collect() } };
    let bytes: Vec<Vec<u8>> = words.iter().map(mb).collect();
    for d in order_diffs(&bytes, |a, b| compare(&t, a.as_slice(), b.as_slice()), |a, b| compare(&o, a.as_slice(), b.as_slice())) {
        out.push(format!("LC_COLLATE strcoll: {d}"));
    }
    let wide: Vec<Vec<u32>> = words.iter().map(|s| s.iter().map(|&c| c as u32).collect()).collect();
    for d in order_diffs(&wide, |a, b| compare(&t, a.as_slice(), b.as_slice()), |a, b| compare(&o, a.as_slice(), b.as_slice())) {
        out.push(format!("LC_COLLATE wcscoll: {d}"));
    }
    let cps: Vec<u32> = alphabet(utf8).into_iter().filter(|&c| keep(c)).map(|c| c as u32).collect();
    let seq = |c: &Collate, cp: u32| if utf8 { c.collseq_wide(cp) } else { c.collseq_byte(cp as u8) };
    for d in order_diffs(&cps, |a, b| seq(&t, *a).cmp(&seq(&t, *b)), |a, b| seq(&o, *a).cmp(&seq(&o, *b))) {
        out.push(format!("LC_COLLATE collseq: {d}"));
    }
    let key = |c: &Collate, s: &[u8]| {
        let mut k = vec![0u8; 4096];
        let n = key_bytes(c, s, &mut k).unwrap_or(0);
        k.truncate(n);
        k
    };
    let same = bytes.iter().filter(|s| key(&t, s) == key(&o, s)).count();
    println!("LC_COLLATE: {} strings, {same} strxfrm keys byte-equal", bytes.len());
}

/// The installed sources with libniji's generated charmaps (`niji_charmaps`, LOCALE-CHARMAPS-GEN)
/// in place of the installed ones: what localedef reads on a system with libniji's `locales`.
struct Generated;

impl Files for Generated {
    fn read(&mut self, kind: Kind, name: &[u8]) -> niji_locale::Result<TryVec<u8>> {
        match kind {
            Kind::Locale => Disk.read(kind, name),
            Kind::Charmap => match niji_charmaps::CHARSETS.iter().find(|c| c.0.as_bytes() == name) {
                Some(&(n, codec)) => TryVec::from_slice(niji_charmaps::charmap(n, codec).as_bytes()).map_err(Into::into),
                None => Err(niji_locale::Error::missing("no generated charmap")),
            },
        }
    }
}

fn compare(locale: &str, charmap: &str) {
    compare_with(locale, charmap, &mut Disk, false);
}

/// `ctype_only`: compare `LC_CTYPE` alone (what a charmap decides: classes, maps, widths).
fn compare_with(locale: &str, charmap: &str, files: &mut dyn Files, ctype_only: bool) {
    let Some(dir) = reference(locale, charmap) else { return };
    // A source given by path is a test's own, compiled as the program does (defaults included).
    let ours = if locale.starts_with('/') {
        niji_locale::compile_output(locale.as_bytes(), charmap.as_bytes(), files).map(|o| o.files)
    } else {
        niji_locale::compile(locale.as_bytes(), charmap.as_bytes(), files)
    };
    let ours = ours.unwrap_or_else(|e| panic!("{locale}.{charmap}: {e:?}"));
    let mut out = Vec::new();
    for (n, file) in FILES.iter().enumerate() {
        let Some(file) = file.filter(|_| n == 0 || !ctype_only) else { continue };
        let Ok(theirs) = std::fs::read(dir.join("out").join(file)) else {
            out.push(format!("{file}: the reference wrote none"));
            continue;
        };
        let Some(mine) = ours.get(n).and_then(Option::as_ref) else {
            out.push(format!("{file}: ours is missing"));
            continue;
        };
        let (Some(t), Some(o)) = (Category::parse(leak(theirs)), Category::parse(leak(mine.as_slice().to_vec()))) else {
            out.push(format!("{file}: a header the reader refuses"));
            continue;
        };
        if n == 0 {
            diff_ctype(t, o, &mut out);
        } else if n == 3 {
            diff_collate(t, o, charmap == "UTF-8", &mut out);
        } else {
            diff_items(file, t, o, &[], &mut out);
        }
    }
    if std::env::var_os("NIJI_LOCALE_KEEP").is_some() {
        // Inspection: keep the reference's files and write ours beside them.
        for (n, file) in FILES.iter().enumerate() {
            if let (Some(file), Some(Some(mine))) = (file, ours.get(n)) {
                let p = dir.join("ours").join(file);
                let _ = std::fs::create_dir_all(p.parent().unwrap_or(&dir));
                let _ = std::fs::write(&p, mine.as_slice());
            }
        }
        println!("kept {}", dir.display());
    } else {
        let _ = std::fs::remove_dir_all(&dir);
    }
    assert!(out.is_empty(), "{locale}.{charmap}: {} differences from the reference:\n{}", out.len(), out.join("\n"));
}

/// s31_8_c_1, s31_8_g_1: en_US UTF-8 compiled by both, compared as the module comment says.
#[test]
fn en_us_utf8() {
    compare("en_US", "UTF-8");
}

/// s31_8_c_1, s31_8_g_1: de_DE UTF-8 compiled by both, compared as the module comment says.
#[test]
fn de_de_utf8() {
    compare("de_DE", "UTF-8");
}

/// s31_8_c_1, s31_8_g_1: fr_FR UTF-8 compiled by both, compared as the module comment says.
#[test]
fn fr_fr_utf8() {
    compare("fr_FR", "UTF-8");
}

/// s31_8_c_1, s31_8_g_1: ja_JP UTF-8 compiled by both, compared as the module comment says.
#[test]
fn ja_jp_utf8() {
    compare("ja_JP", "UTF-8");
}

/// s31_8_c_1, s31_8_g_1: tr_TR UTF-8 compiled by both, compared as the module comment says.
#[test]
fn tr_tr_utf8() {
    compare("tr_TR", "UTF-8");
}

/// s31_8_c_1, s31_8_g_1: en_US ISO-8859-1 compiled by both, compared as the module comment says.
#[test]
fn en_us_iso_8859_1() {
    compare("en_US", "ISO-8859-1");
}

/// s31_8_c_1, s31_8_g_3: a source that defines one category gets every other category's
/// default, equal to what the reference writes for a category a source does not define (#7534):
/// once for a multi-byte charmap (UTF-8) and once for a single-byte one (ISO-8859-1). Every item
/// of every file is compared, none excepted: XBD 7.3 leaves a category the source omits
/// unspecified, so libniji writes the reference's defaults (D-31.24).
#[test]
fn undefined_categories_get_the_references_defaults() {
    let dir = std::env::temp_dir().join(format!("niji-locale-defaults-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("only_measurement");
    std::fs::write(&src, "LC_MEASUREMENT\nmeasurement 2\nEND LC_MEASUREMENT\n").unwrap();
    let src = src.to_str().unwrap();
    for charmap in ["UTF-8", "ISO-8859-1"] {
        let Some(rdir) = reference(src, charmap) else { continue };
        let ours = niji_locale::compile_output(src.as_bytes(), charmap.as_bytes(), &mut Disk).unwrap().files;
        let mut out = Vec::new();
        for (n, file) in FILES.iter().enumerate() {
            let Some(file) = file else { continue };
            let theirs = std::fs::read(rdir.join("out").join(file)).unwrap_or_else(|_| panic!("{charmap}: the reference wrote no {file}"));
            let mine = ours[n].as_ref().unwrap_or_else(|| panic!("{charmap}: ours has no {file}"));
            let (t, o) = (Category::parse(leak(theirs)).unwrap(), Category::parse(leak(mine.as_slice().to_vec())).unwrap());
            match n {
                0 => diff_ctype(t, o, &mut out),
                3 => diff_collate(t, o, charmap == "UTF-8", &mut out),
                _ => diff_items(file, t, o, &[], &mut out),
            }
        }
        let _ = std::fs::remove_dir_all(&rdir);
        assert!(out.is_empty(), "{charmap}: {} differences:\n{}", out.len(), out.join("\n"));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The category files written for a source that defines only `LC_MEASUREMENT`, compiled with
/// libniji's generated `charmap` (no installed data or reference needed), read as the C library
/// reads them (`niji::locale`).
fn undefined(charmap: &str) -> Vec<Option<Category>> {
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("niji-locale-undef-{}-{}", std::process::id(), RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("only_measurement");
    std::fs::write(&src, "LC_MEASUREMENT\nmeasurement 2\nEND LC_MEASUREMENT\n").unwrap();
    let files = niji_locale::compile_output(src.to_str().unwrap().as_bytes(), charmap.as_bytes(), &mut Generated).unwrap().files;
    let _ = std::fs::remove_dir_all(&dir);
    FILES.iter().enumerate().map(|(n, f)| f.map(|f| Category::parse(leak(files[n].as_ref().unwrap_or_else(|| panic!("no {f}")).as_slice().to_vec())).unwrap())).collect()
}

/// s31_8_g_3 (D-31.24): an undefined `LC_CTYPE` classes the ASCII letters and digits (`alpha`,
/// `upper`, `lower`, `digit`, `xdigit`, `print`, `graph`) and the white space (`space`, `blank`,
/// space also `print`), and nothing else: punctuation and control characters are in no class and
/// have no width; `toupper` maps the ASCII letters; no wide input digits.
#[test]
fn undefined_ctype_has_the_references_defaults() {
    for charmap in ["UTF-8", "ISO-8859-1"] {
        let c = CtypeFile::parse(undefined(charmap)[0].unwrap()).unwrap();
        let k = |n: &str| c.class_index(n.as_bytes()).unwrap();
        for (wc, classes, width) in [('A', &["alpha", "upper", "xdigit", "print", "graph"][..], 1), ('z', &["alpha", "lower", "print", "graph"], 1), ('7', &["digit", "xdigit", "print", "graph"], 1), (' ', &["space", "blank", "print"], 1), ('!', &[], -1), ('~', &[], -1), ('\u{1}', &[], -1), ('\u{7f}', &[], -1), ('\t', &["space", "blank"], -1)] {
            for name in ["alpha", "upper", "lower", "digit", "xdigit", "space", "blank", "print", "graph", "punct", "cntrl", "alnum"] {
                let want = classes.contains(&name) || (name == "alnum" && (classes.contains(&"alpha") || classes.contains(&"digit")));
                assert_eq!(c.in_class(k(name), wc as u32), want, "{charmap}: {wc:?} in {name}");
            }
            assert_eq!(c.width(wc as u32), width, "{charmap}: width of {wc:?}");
        }
        assert_eq!(c.map(c.map_index(b"toupper").unwrap(), 'q' as u32), 'Q' as u32, "{charmap}");
        assert_eq!(c.category().word(30), 0, "{charmap}: INDIGITS_WC_LEN");
        assert!((31..=40).all(|i| c.category().item(i).is_empty()), "{charmap}: wide input digits");
    }
}

/// s31_8_g_3 (D-31.24): an undefined `LC_TIME` has the English names and formats as multibyte
/// strings, their wide forms empty up to `t_fmt` (`t_fmt_ampm`'s and `date_fmt`'s kept), and empty
/// alternative month names.
#[test]
fn undefined_time_has_the_references_defaults() {
    let t = undefined("UTF-8")[2].unwrap();
    assert_eq!((t.string(0), t.string(7 + 7 + 12 + 11), t.string(43)), (&b"Sun"[..], &b"December"[..], &b"%I:%M:%S %p"[..]));
    assert!((52..=94).all(|i| t.item(i) == [0u8; 4]), "wide names and formats up to t_fmt are empty");
    let wide: Vec<u8> = "%I:%M:%S %p\0".chars().flat_map(|c| (c as u32).to_le_bytes()).collect();
    assert_eq!(t.item(95), wide.as_slice(), "wide t_fmt_ampm");
    assert!((111..=158).all(|i| t.item(i).iter().all(|&b| b == 0)), "alternative month names are empty");
}

/// s31_8_g_3 (D-31.24): an undefined `LC_MONETARY` has empty symbols, `.` as the radix, every
/// "not available" number 0xff, `mon_grouping` the one byte 0x7f and CRNCYSTR `-`.
#[test]
fn undefined_monetary_has_the_references_defaults() {
    let m = undefined("UTF-8")[4].unwrap();
    assert_eq!((m.item(0), m.item(1), m.item(2)), (&b"\0"[..], &b"\0"[..], &b".\0"[..]));
    assert_eq!(m.item(4), [0x7f], "mon_grouping");
    assert!((7..=14).chain(16..=21).all(|i| m.item(i) == [0xff]), "not-available numbers");
    assert_eq!(m.item(15), b"-\0", "CRNCYSTR");
}

/// s31_8_g_3 (D-31.24): an undefined `LC_ADDRESS` has the default `postal_fmt` and country codes
/// of spaces (two and three).
#[test]
fn undefined_address_has_the_references_defaults() {
    let a = undefined("UTF-8")[9].unwrap();
    assert_eq!(a.string(0), b"%a%N%f%N%d%N%b%N%s %h %e %r%N%C-%z %T%N%c%N");
    assert_eq!((a.item(3), a.item(4)), (&b"  \0"[..], &b"   \0"[..]));
}

/// s31_8_g_3 (D-31.24): the other undefined categories: `LC_NUMERIC` `.` without grouping,
/// `LC_MESSAGES` `^[yY]`/`^[nN]`, `LC_PAPER` 297 x 210, `LC_NAME`'s and `LC_TELEPHONE`'s formats.
#[test]
fn undefined_other_categories_have_the_references_defaults() {
    let all = undefined("ISO-8859-1");
    let c = |n: usize| all[n].unwrap();
    assert_eq!((c(1).string(0), c(1).string(1), c(1).string(2)), (&b"."[..], &b""[..], &b""[..]), "LC_NUMERIC");
    assert_eq!((c(5).string(0), c(5).string(1)), (&b"^[yY]"[..], &b"^[nN]"[..]), "LC_MESSAGES");
    assert_eq!((c(7).word(0), c(7).word(1)), (297, 210), "LC_PAPER");
    assert_eq!(c(8).string(0), b"%p%t%g%t%m%t%f", "LC_NAME");
    assert_eq!(c(10).string(0), b"+%c %a%t%l", "LC_TELEPHONE");
}

/// s31_8_c_1, s31_8_i_2: the #14 locales compiled with libniji's generated charmaps
/// (LOCALE-CHARMAPS-GEN) equal the reference's compile with its own, compared as the module
/// comment says.
#[test]
fn generated_charmaps_compile_like_the_reference() {
    for (locale, charmap) in [("en_US", "UTF-8"), ("de_DE", "UTF-8"), ("fr_FR", "UTF-8"), ("ja_JP", "UTF-8"), ("tr_TR", "UTF-8"), ("en_US", "ISO-8859-1")] {
        compare_with(locale, charmap, &mut Generated, false);
    }
}

/// The generated locale sources (LOCALE-SOURCES-GEN, D-31.27: `cargo xtask so` writes them to
/// `target/niji/locale-sources`, or `$NIJI_PRODUCTS/locale-sources`) with the generated charmaps:
/// what localedef reads on a system with libniji's `locales` and no file of the reference's.
struct Sources(PathBuf);

impl Files for Sources {
    fn read(&mut self, kind: Kind, name: &[u8]) -> niji_locale::Result<TryVec<u8>> {
        match kind {
            Kind::Locale => {
                let data = std::fs::read(self.0.join(String::from_utf8_lossy(name).as_ref())).map_err(|_| niji_locale::Error::missing("no such file"))?;
                TryVec::from_slice(&data).map_err(Into::into)
            }
            Kind::Charmap => Generated.read(kind, name),
        }
    }
}

fn sources() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let products = std::env::var_os("NIJI_PRODUCTS").map_or_else(|| root.join("target/niji"), PathBuf::from);
    let dir = products.join("locale-sources");
    assert!(dir.join("SUPPORTED").exists(), "the generated locale sources are missing: run `cargo xtask so`");
    dir
}

/// s31_8_c_1: the #14 locales compiled from libniji's generated sources (Unicode CLDR, D-31.27)
/// keep what the comparison with the reference found equal: every `LC_CTYPE` class bit, case
/// map value and width of every code point, the `LC_NUMERIC` items (en_US, de_DE, fr_FR: the
/// decimal point, grouping and thousands separator printf uses) and, for the locales the
/// reference does not tailor (en_US, de_DE, fr_FR), the collation order of the corpus
/// (CLDR's root collation with UTS #10 shifted variable weighting). The parts that differ by
/// design (transliteration, dates, the tailored orders) are docs/differences-from-glibc.md rows.
#[test]
fn generated_sources_keep_the_reference_ctype_numbers_and_root_order() {
    let src = sources();
    for locale in ["en_US", "de_DE", "fr_FR", "ja_JP", "tr_TR"] {
        let Some(dir) = reference(locale, "UTF-8") else { continue };
        let ours = niji_locale::compile(locale.as_bytes(), b"UTF-8", &mut Sources(src.clone())).unwrap_or_else(|e| panic!("{locale}: the generated source does not compile: {e:?}"));
        let cat = |n: usize, theirs: bool| -> Category {
            let bytes = if theirs { std::fs::read(dir.join("out").join(FILES[n].unwrap())).unwrap() } else { ours[n].as_ref().unwrap().as_slice().to_vec() };
            Category::parse(leak(bytes)).unwrap_or_else(|| panic!("{locale}: category {n}: a header the reader refuses"))
        };
        let mut out = Vec::new();
        diff_ctype(cat(0, true), cat(0, false), &mut out);
        // The POSIX classes and libniji's two combining classes, and the case maps; ja_JP's own
        // classes and maps of the reference (jkanji, tojhira, ...) are a differences-row item.
        let standard = |l: &str| {
            l.strip_prefix("LC_CTYPE class ").and_then(|r| r.split(':').next()).is_some_and(|c| {
                ["upper", "lower", "alpha", "digit", "xdigit", "space", "print", "graph", "blank", "cntrl", "punct", "alnum", "combining", "combining_level3"].contains(&c.trim())
            })
        };
        let case_map = |l: &str| ["toupper:", "tolower:", "totitle:"].iter().any(|m| l.strip_prefix("LC_CTYPE map ").is_some_and(|r| r.starts_with(m)));
        out.retain(|l| standard(l) || case_map(l) || l.starts_with("LC_CTYPE width") || l.contains("refuses"));
        if matches!(locale, "en_US" | "de_DE" | "fr_FR") {
            diff_items("LC_NUMERIC", cat(1, true), cat(1, false), &[], &mut out);
            // Left out, each a differences-row item: UCA's implicit weights order every unified
            // ideograph, where the reference leaves CJK Extension A and the supplementary-plane
            // ideographs undefined; the reference ties U+013F/U+0140 with L/l and MIDDLE DOT at
            // every level. The collation-sequence order (bracket ranges) is compared for ASCII
            // only: beyond it the reference's sequence is its table's line order (Greek, U+0149
            // and U+00FE before Latin A), which a weight order need not follow.
            diff_collate_of(cat(3, true), cat(3, false), true, |c| !matches!(c as u32, 0x3400..=0x4DBF | 0x9FA6..=0x9FFF | 0x20000..=0x323AF | 0x13F | 0x140), &mut out);
            out.retain(|l| {
                let Some(rest) = l.strip_prefix("LC_COLLATE collseq: ") else { return true };
                rest.split([' ', ':']).take(2).all(|n| n.parse::<u32>().is_ok_and(|n| n < 0x80))
            });
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.is_empty(), "{locale}: {} differences from the reference:\n{}", out.len(), out.join("\n"));
    }
}

/// s31_8_c_1, s31_8_i_2: CJK locales compiled with the generated multi-byte charmaps give the
/// reference's `LC_CTYPE` (classes, maps, every width: the double-width rule of D-31.22). The other
/// categories are not compared here: they differ with the installed charmaps too (handoff).
#[test]
fn generated_cjk_charmaps_give_the_reference_ctype() {
    for (locale, charmap) in [("ja_JP", "EUC-JP"), ("ja_JP", "WINDOWS-31J"), ("ko_KR", "EUC-KR"), ("ko_KR", "CP949")] {
        compare_with(locale, charmap, &mut Generated, true);
    }
}

/// s31_8_g_1, s31_8_g_4: the products' `localedef` (the freestanding program, its tool arena
/// reusing and unmapping freed blocks, D-31.23) writes exactly the library's compile of en_US and
/// de_DE for UTF-8, as directories and into an archive under `--prefix`. Needs the products
/// (`cargo xtask so`) and the locale sources; without them it says so and skips.
#[test]
fn products_localedef_writes_the_library_compile() {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/niji/localedef");
    if !bin.exists() || !source_path("en_US").exists() {
        println!("skip: needs {} (cargo xtask so) and {}", bin.display(), source_path("en_US").display());
        return;
    }
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("niji-locale-products-{}-{}", std::process::id(), RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("p/usr/lib/locale")).unwrap();
    for locale in ["en_US", "de_DE"] {
        let out = dir.join(format!("{locale}.UTF-8"));
        let run = Command::new(&bin).args(["-i", locale, "-f", "UTF-8"]).arg(&out).env_clear().output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{locale}: {}", String::from_utf8_lossy(&run.stderr));
        let lib = niji_locale::compile_output(locale.as_bytes(), b"UTF-8", &mut Disk).unwrap();
        for (n, file) in FILES.iter().enumerate() {
            let (Some(file), Some(Some(want))) = (file, lib.files.get(n)) else { continue };
            let got = std::fs::read(out.join(file)).unwrap_or_default();
            assert!(got == want.as_slice(), "{locale} {file}: the program's file differs from the library's");
        }
        let prefix = format!("--prefix={}", dir.join("p").display());
        let run = Command::new(&bin).args([prefix.as_str(), "-i", locale, "-f", "UTF-8", &format!("{locale}.UTF-8")]).env_clear().output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{locale} archive: {}", String::from_utf8_lossy(&run.stderr));
    }
    let archive = std::fs::read(dir.join("p/usr/lib/locale/locale-archive")).unwrap();
    let a = niji_locale::archive::Archive::parse(&archive).unwrap();
    assert_eq!(a.names().unwrap().iter().map(|n| n.to_vec()).collect::<Vec<_>>(), [b"de_DE.utf8".to_vec(), b"en_US.utf8".to_vec()]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// s31_8_i_4, s31_8_i_5: the products' `localedef` reads `I18NPATH` from its environment and the working directory
/// first: a `-i` / `-f` name found in an `I18NPATH` entry's `locales/` / `charmaps/`, and the same `-i` name in the
/// working directory taking its place; the charmap names characters by POSIX names only. Files the test writes;
/// needs only the products (`cargo xtask so`), else it says so and skips.
#[test]
fn products_localedef_searches_i18npath_and_the_working_directory() {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/niji/localedef");
    if !bin.exists() {
        println!("skip: needs {} (cargo xtask so)", bin.display());
        return;
    }
    let dir = std::env::temp_dir().join(format!("niji-locale-i18npath-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for d in ["e/locales", "e/charmaps", "cwd"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    let mut charmap = String::from("<code_set_name> TCM\n<mb_cur_max> 1\n<mb_cur_min> 1\nCHARMAP\n<NUL> /x00\n");
    for c in b'A'..=b'Z' {
        charmap.push_str(&format!("<{}> /x{c:02x}\n", c as char));
    }
    charmap.push_str("END CHARMAP\n");
    std::fs::write(dir.join("e/charmaps/TCM"), charmap).unwrap();
    let source = |c: char| format!("LC_NUMERIC\ndecimal_point \"<{c}>\"\nthousands_sep \"\"\ngrouping -1\nEND LC_NUMERIC\n");
    std::fs::write(dir.join("e/locales/tsrc"), source('Q')).unwrap();
    let path = format!(":{}", dir.join("e").display());
    let point = |out: &str| -> Option<u8> {
        let f = std::fs::read(dir.join(out).join("LC_NUMERIC")).ok()?;
        b"QW".iter().copied().find(|&c| f.windows(2).any(|w| w == [c, 0]))
    };
    let run = |out: &str| {
        let mut c = Command::new(&bin);
        c.args(["-c", "--quiet", "-i", "tsrc", "-f", "TCM"]).arg(dir.join(out)).current_dir(dir.join("cwd"));
        c.env_clear().env("I18NPATH", &path).output().unwrap()
    };
    let o = run("o1");
    assert_eq!((o.status.code(), point("o1")), (Some(0), Some(b'Q')), "{}", String::from_utf8_lossy(&o.stderr));
    std::fs::write(dir.join("cwd/tsrc"), source('W')).unwrap();
    let o = run("o2");
    assert_eq!((o.status.code(), point("o2")), (Some(0), Some(b'W')), "{}", String::from_utf8_lossy(&o.stderr));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Sources by name from one directory (an absolute name as given), charmaps installed.
struct Under(PathBuf);

impl Files for Under {
    fn read(&mut self, kind: Kind, name: &[u8]) -> niji_locale::Result<TryVec<u8>> {
        if kind == Kind::Charmap {
            return Disk.read(kind, name);
        }
        let name = String::from_utf8_lossy(name).into_owned();
        let p = if name.starts_with('/') { PathBuf::from(&name) } else { self.0.join(&name) };
        TryVec::from_slice(&std::fs::read(p).map_err(|_| niji_locale::Error::missing("no such file"))?).map_err(Into::into)
    }
}

/// s31_8_g_5: strings of every string category with characters ISO-8859-1 lacks, a test's own
/// source with an `include` chain (found through `I18NPATH`), compiled by both: every item of
/// every category file that holds such strings equal. Transliterated (own entry before an include's, the first include
/// before the second, the first alternative that fits whole), and not (no entry, an alternative
/// that does not fit, a string's single character): the string empty.
#[test]
fn strings_the_charmap_lacks_are_transliterated_as_the_reference_does() {
    let u = |s: &str| s.chars().map(|c| format!("<U{:04X}>", c as u32)).collect::<String>();
    let q = |s: &str| format!("\"{}\"", u(s));
    let tl = |e: &[(&str, &[&str])]| e.iter().map(|(f, to)| format!("{} {}\n", u(f), to.iter().map(|t| q(t)).collect::<Vec<_>>().join(";"))).collect::<String>();
    let ctype = |body: String| format!("LC_CTYPE\ntranslit_start\n{body}translit_end\nEND LC_CTYPE\n");
    let dir = std::env::temp_dir().join(format!("niji-locale-translit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("tra"), ctype(format!("include \"trc\";\"\"\n{}", tl(&[("\u{20ac}", &["A1"]), ("\u{4e01}", &["Z"])])))).unwrap();
    std::fs::write(dir.join("trb"), ctype(tl(&[("\u{4e01}", &["Y"]), ("\u{4e02}", &["W"])]))).unwrap();
    std::fs::write(dir.join("trc"), ctype(tl(&[("\u{4e03}", &["V"]), ("\u{4e01}", &["U"])]))).unwrap();
    let own = tl(&[("\u{20ac}", &["EUR"]), ("\u{202f}", &[" "]), ("\u{2030}", &["\u{2031}", "%0"]), ("\u{2032}", &["\u{2033}"]), ("\u{2033}", &["\""]), ("\u{2034}", &["", "A\u{2035}", "B"])]);
    let list = |v: &[&str]| v.iter().map(|s| q(s)).collect::<Vec<_>>().join(";");
    let mon: Vec<&str> = ["\u{20ac}", "a\u{4e00}b", "\u{4e01}", "\u{4e02}", "\u{4e03}", "x\u{2030}", "\u{2032}", "\u{2034}", "\u{e9}", "m", "m", "m"].into();
    let src = format!(
        "comment_char %\nescape_char /\n{}\
LC_MONETARY\nint_curr_symbol {}\ncurrency_symbol {}\nmon_decimal_point {}\nmon_thousands_sep {}\nmon_grouping 3\npositive_sign \"\"\nnegative_sign {}\n\
int_frac_digits 2\nfrac_digits 2\np_cs_precedes 1\np_sep_by_space 1\nn_cs_precedes 1\nn_sep_by_space 1\np_sign_posn 1\nn_sign_posn 1\nEND LC_MONETARY\n\
LC_NUMERIC\ndecimal_point {}\nthousands_sep {}\ngrouping 3\nEND LC_NUMERIC\n\
LC_MESSAGES\nyesexpr {}\nnoexpr {}\nyesstr {}\nnostr {}\nEND LC_MESSAGES\n\
LC_ADDRESS\npostal_fmt {}\ncountry_name {}\nlang_name {}\ncountry_ab2 \"DE\"\ncountry_ab3 \"DEU\"\ncountry_num 276\nEND LC_ADDRESS\n\
LC_TELEPHONE\ntel_int_fmt {}\nint_prefix {}\nEND LC_TELEPHONE\n\
LC_NAME\nname_fmt {}\nname_mr {}\nEND LC_NAME\n\
LC_IDENTIFICATION\ntitle {}\nsource {}\nEND LC_IDENTIFICATION\n\
LC_TIME\nabday {}\nday {}\nabmon {}\nmon {}\nd_t_fmt {}\nd_fmt \"%x\"\nt_fmt \"%X\"\nam_pm {}\nt_fmt_ampm \"%r\"\nweek 7;19971130;4\ndate_fmt \"%a\"\nEND LC_TIME\n",
        ctype(format!("include \"tra\";\"\"\ninclude \"trb\";\"\"\n{own}")),
        q("EUR "), q("\u{20ac}"), q("\u{2030}"), q("\u{202f}"), q("-\u{4e00}"),
        q("\u{4e01}"), q("\u{4e02}"),
        q("^[yY\u{20ac}]"), q("^[nN\u{4e00}]"), q("y\u{20ac}s"), q("n\u{2032}"),
        q("%a%N\u{20ac}"), q("\u{4e00}"), q("x\u{202f}y"),
        q("+%c \u{20ac}"), q("49"),
        q("%d%t\u{20ac}"), q("M\u{4e03}"),
        q("T \u{20ac}"), q("S\u{4e00}"),
        list(&["a"; 7]), list(&["d"; 7]), list(&["m"; 12]), list(&mon), q("%c\u{20ac}"), list(&["AM\u{4e00}", "PM\u{20ac}"]),
    );
    let path = dir.join("trsrc");
    std::fs::write(&path, src).unwrap();
    let out = dir.join("out");
    let run = Command::new("localedef").args(["-c", "-f", "ISO-8859-1", "-i"]).arg(&path).arg(&out).env("LC_ALL", "C").env("I18NPATH", &dir).current_dir(&dir).output();
    if !(run.is_ok() && out.join("LC_TIME").exists()) {
        println!("skip: the reference localedef wrote nothing: {run:?}");
        return;
    }
    let ours = niji_locale::compile_output(path.to_str().unwrap().as_bytes(), b"ISO-8859-1", &mut Under(dir.clone())).unwrap_or_else(|e| panic!("{e:?}"));
    let mut diffs = Vec::new();
    // The categories whose strings are converted (LC_CTYPE, which converts none, is the table's
    // own; LC_COLLATE converts none).
    for (n, file) in FILES.iter().enumerate() {
        let (Some(file), false) = (file, n == 0 || n == 3) else { continue };
        let theirs = Category::parse(leak(std::fs::read(out.join(file)).unwrap())).unwrap();
        let mine = Category::parse(leak(ours.files[n].as_ref().unwrap().as_slice().to_vec())).unwrap();
        diff_items(file, theirs, mine, &[], &mut diffs);
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(diffs.is_empty(), "{} differences from the reference:\n{}", diffs.len(), diffs.join("\n"));
}
