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
            Kind::Locale => std::fs::read(format!("{I18N}/locales/{name}")).map_err(|_| missing)?,
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

/// Compile `locale` for `charmap` with the reference into a fresh directory; `None` (reason
/// printed) when it cannot run.
fn reference(locale: &str, charmap: &str) -> Option<PathBuf> {
    if !Path::new(&format!("{I18N}/locales/{locale}")).exists() {
        println!("skip {locale}.{charmap}: no source {I18N}/locales/{locale}");
        return None;
    }
    let dir = std::env::temp_dir().join(format!("niji-locale-ref-{}-{locale}.{charmap}", std::process::id()));
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
    use niji::locale::collate::{compare, key_bytes, Collate, Collation};
    let (Some(Collation::Rules(t)), Some(Collation::Rules(o))) = (Collate::parse(theirs), Collate::parse(ours)) else {
        out.push("LC_COLLATE: the reader does not take both as rules".into());
        return;
    };
    let words = corpus(utf8);
    let mb = |s: &Vec<char>| -> Vec<u8> { if utf8 { s.iter().collect::<String>().into_bytes() } else { s.iter().map(|&c| c as u8).collect() } };
    let bytes: Vec<Vec<u8>> = words.iter().map(mb).collect();
    for d in order_diffs(&bytes, |a, b| compare(&t, a.as_slice(), b.as_slice()), |a, b| compare(&o, a.as_slice(), b.as_slice())) {
        out.push(format!("LC_COLLATE strcoll: {d}"));
    }
    let wide: Vec<Vec<u32>> = words.iter().map(|s| s.iter().map(|&c| c as u32).collect()).collect();
    for d in order_diffs(&wide, |a, b| compare(&t, a.as_slice(), b.as_slice()), |a, b| compare(&o, a.as_slice(), b.as_slice())) {
        out.push(format!("LC_COLLATE wcscoll: {d}"));
    }
    let cps: Vec<u32> = alphabet(utf8).iter().map(|&c| c as u32).collect();
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

fn compare(locale: &str, charmap: &str) {
    let Some(dir) = reference(locale, charmap) else { return };
    let ours = niji_locale::compile(locale.as_bytes(), charmap.as_bytes(), &mut Disk).unwrap_or_else(|e| panic!("{locale}.{charmap}: {e:?}"));
    let mut out = Vec::new();
    for (n, file) in FILES.iter().enumerate() {
        let Some(file) = file else { continue };
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

#[test]
fn en_us_utf8() {
    compare("en_US", "UTF-8");
}

#[test]
fn de_de_utf8() {
    compare("de_DE", "UTF-8");
}

#[test]
fn fr_fr_utf8() {
    compare("fr_FR", "UTF-8");
}

#[test]
fn ja_jp_utf8() {
    compare("ja_JP", "UTF-8");
}

#[test]
fn tr_tr_utf8() {
    compare("tr_TR", "UTF-8");
}

#[test]
fn en_us_iso_8859_1() {
    compare("en_US", "ISO-8859-1");
}
