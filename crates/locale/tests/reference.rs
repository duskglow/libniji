//! niji-locale's output against the reference localedef's, for the locales of issue #14 (en_US,
//! de_DE, fr_FR, ja_JP, tr_TR in UTF-8) and one single-byte charmap (en_US ISO-8859-1).
//!
//! s31_8_c_1, s31_8_g_1: the same source and charmap compiled by both. Every item of the string,
//! number and byte categories is compared byte for byte; `LC_CTYPE`'s fixed items byte for byte
//! and its tables by what the C library reads from them (`niji::locale`): every class bit, map
//! value and width of every code point below 0x110000. Both use the installed sources
//! (`/usr/share/i18n`); a case whose source, charmap or reference program is absent is skipped
//! with the reason printed.

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

fn names(item: &[u8]) -> Vec<Vec<u8>> {
    item.split(|&b| b == 0).take_while(|s| !s.is_empty()).map(<[u8]>::to_vec).collect()
}

/// `LC_CTYPE` compared by what the reader sees: the fixed items, then every class, map and width.
fn diff_ctype(theirs: Category, ours: Category, out: &mut Vec<String>) {
    let class_offset = theirs.word(17) as usize;
    let map_offset = theirs.word(18) as usize;
    diff_items("LC_CTYPE", theirs, ours, &(12..13).chain(class_offset..theirs.len()).collect::<Vec<_>>(), out);
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
    let mut report = |what: String, firsts: &mut Vec<String>| {
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
                if a.value_u8(wc) != b.value_u8(wc) {
                    count += 1;
                    report(format!("{wc:#06x} ref {:?} ours {:?}", a.value_u8(wc), b.value_u8(wc)), &mut firsts);
                }
            }
            if count > 0 {
                out.push(format!("LC_CTYPE width: {count} differ: {}", firsts.join(", ")));
            }
        }
        _ => out.push("LC_CTYPE width: table missing".into()),
    }
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
            // LC_COLLATE: not written yet.
            if n != 3 {
                out.push(format!("{file}: ours is missing"));
            }
            continue;
        };
        let (Some(t), Some(o)) = (Category::parse(leak(theirs)), Category::parse(leak(mine.as_slice().to_vec()))) else {
            out.push(format!("{file}: a header the reader refuses"));
            continue;
        };
        if n == 0 {
            diff_ctype(t, o, &mut out);
        } else {
            diff_items(file, t, o, &[], &mut out);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
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
