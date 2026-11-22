//! Which zone a `TZ` value selects (§23.6) through `niji::time::load`, on a scratch zone
//! directory this test builds (no dependence on the host's zone files). Written before the
//! COLD-REWRITE2-cold-02 rewrite and passing on the old code.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, missing_docs)]

use niji::time::{self, Loaded, Zone};
use niji_boundary::sys::CSys;
use std::path::PathBuf;

/// A TZif version 1 file with no transitions and one type: `offset`, abbreviation `abbr`.
fn tzif(offset: i32, abbr: &[u8]) -> Vec<u8> {
    let mut f = b"TZif".to_vec();
    f.extend_from_slice(&[0; 16]);
    // isutcnt, isstdcnt, leapcnt, timecnt, typecnt, charcnt
    for n in [0u32, 0, 0, 0, 1, abbr.len() as u32 + 1] {
        f.extend_from_slice(&n.to_be_bytes());
    }
    f.extend_from_slice(&offset.to_be_bytes());
    f.extend_from_slice(&[0, 0]);
    f.extend_from_slice(abbr);
    f.push(0);
    f
}

fn scratch() -> PathBuf {
    let d = std::env::temp_dir().join(format!("niji-time-load-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("Area")).unwrap();
    std::fs::write(d.join("Area/Nine"), tzif(32_400, b"NST")).unwrap();
    std::fs::write(d.join("junk"), b"not a zone file").unwrap();
    d
}

fn z(p: &std::path::Path) -> Vec<u8> {
    let mut v = p.to_str().unwrap().as_bytes().to_vec();
    v.push(0);
    v
}

fn zone_of(l: &Loaded) -> &Zone {
    l.zone.as_ref().unwrap_or(l.fixed)
}

fn offset(l: &Loaded) -> i64 {
    zone_of(l).at(0).unwrap().offset
}

fn abbr(l: &Loaded) -> Vec<u8> {
    let zn = zone_of(l);
    zn.name(zn.at(0).unwrap().name).to_vec()
}

#[test]
fn s23_6_c_0() {
    // Clauses: s23_6_c_0, s23_6_c_1, s23_6_c_2, s23_6_c_3 (the selection order of §23.6).
    let d = scratch();
    let dir = d.to_str().unwrap().as_bytes();
    let missing = z(&d.join("absent"));
    let load = |tz: Option<&[u8]>, default: &[u8]| time::load(&CSys, tz, Some(dir), default, false).unwrap();
    // Unset: the default file, kept with its identity; unreadable: UTC, still watched.
    let l = load(None, &z(&d.join("Area/Nine")));
    assert_eq!((offset(&l), abbr(&l)), (32_400, b"NST".to_vec()));
    let (path, id) = l.file.as_ref().unwrap();
    assert_eq!(path.as_slice(), z(&d.join("Area/Nine")).as_slice());
    assert_eq!(*id, time::file_id(&CSys, path.as_slice()));
    assert!(id.is_some());
    let l = load(None, &missing);
    assert!(l.zone.is_none() && core::ptr::eq(l.fixed, &time::UTC), "unset, no file: UTC");
    assert_eq!(l.file.as_ref().map(|(p, id)| (p.as_slice().to_vec(), *id)), Some((missing.clone(), None)));
    // Empty, or only `:`: UTC named Universal.
    for tz in [&b""[..], b":"] {
        let l = load(Some(tz), &missing);
        assert!(l.zone.is_none() && core::ptr::eq(l.fixed, &time::UNIVERSAL), "{tz:?}");
        assert!(l.file.is_none());
    }
    // A name under TZDIR, with or without the colon; an absolute path.
    for tz in [&b"Area/Nine"[..], b":Area/Nine"] {
        let l = load(Some(tz), &missing);
        assert_eq!(offset(&l), 32_400, "{tz:?}");
        assert_eq!(l.file.as_ref().unwrap().0.as_slice(), z(&d.join("Area/Nine")).as_slice());
    }
    let abs = d.join("Area/Nine");
    let l = load(Some(abs.to_str().unwrap().as_bytes()), &missing);
    assert_eq!(offset(&l), 32_400);
    // Not a TZif file (a directory, text, nothing): the value as a rule, else UTC named by
    // its leading name.
    let l = load(Some(b"EST5EDT,M3.2.0,M11.1.0"), &missing);
    assert_eq!((offset(&l), abbr(&l)), (-18_000, b"EST".to_vec()), "a rule");
    assert!(l.file.is_none(), "a rule is not watched");
    for (tz, name) in [(&b"Area"[..], &b"Area"[..]), (b"junk", b"junk"), (b"Nowhere/Else", b"Nowhere"), (b"/abs/absent", b""), (b"12", b"")] {
        let l = load(Some(tz), &missing);
        assert_eq!((offset(&l), abbr(&l)), (0, name.to_vec()), "{tz:?}");
        assert!(l.file.is_none(), "{tz:?}: not watched");
    }
    // TZDIR unset: the system directory (a name that cannot be there).
    let l = time::load(&CSys, Some(b"libniji/No/Such/Zone"), None, &missing, false).unwrap();
    assert_eq!((offset(&l), abbr(&l)), (0, b"libniji".to_vec()));
    assert_eq!(time::ZONEINFO, b"/usr/share/zoneinfo");
    assert_eq!(time::LOCALTIME, b"/etc/localtime\0");
    assert_eq!(time::file_id(&CSys, &missing), None);
    // The size limit (4 MiB, a libniji choice): a zone file padded to the limit is read; one byte
    // more and it is not a zone (the value is then a rule: UTC named by its leading name).
    let mut big = tzif(32_400, b"NST");
    big.resize(4 << 20, 0);
    std::fs::write(d.join("Big"), &big).unwrap();
    assert_eq!(offset(&load(Some(b"Big"), &missing)), 32_400, "a file of the limit is read");
    big.push(0);
    std::fs::write(d.join("Big"), &big).unwrap();
    let l = load(Some(b"Big"), &missing);
    assert_eq!((offset(&l), abbr(&l)), (0, b"Big".to_vec()), "a file over the limit is not a zone");
    let _ = std::fs::remove_dir_all(&d);
}

/// The secure selection (`spec/security/threat-model.md`, the `TZ` row): with `trusted` the
/// scratch directory standing in for the system one, a value names a file only inside it.
#[test]
fn s23_6_c_2_secure_names_files_only_inside_the_zone_directory() {
    // Clauses: s23_6_c_0, s23_6_c_2 (secure mode: the threat model's TZ row; spec/behaviour/dynlink-secure.md).
    let base = std::env::temp_dir().join(format!("niji-time-secure-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (inside, out, sibling) = (base.join("zi"), base.join("out"), base.join("zi2"));
    for dir in [inside.join("Area"), out.clone(), sibling.clone()] {
        std::fs::create_dir_all(dir).unwrap();
    }
    for f in [inside.join("Area/Nine"), out.join("Nine"), sibling.join("Nine")] {
        std::fs::write(f, tzif(32_400, b"NST")).unwrap();
    }
    let s = |p: &std::path::Path| p.to_str().unwrap().as_bytes().to_vec();
    let dir = s(&inside);
    let missing = z(&base.join("absent"));
    let load_in = |tz: &[u8], tzdir: Option<&[u8]>, trusted: Option<&[u8]>| time::load_in(&CSys, Some(tz), tzdir, &missing, trusted).unwrap();
    let cat = |a: &[u8], b: &[u8]| [a, b].concat();
    let abs_in = s(&inside.join("Area/Nine"));
    let abs_out = s(&out.join("Nine"));
    // Read: a name, a rule, an absolute path inside (also with `//` and `.` components).
    let read = [b"Area/Nine".to_vec(), b":Area/Nine".to_vec(), abs_in.clone(), cat(b":", &abs_in), cat(&dir, b"//Area/./Nine")];
    // Refused: outside (also a sibling sharing the prefix), through `..`, the value then a rule.
    let up = cat(b"../../../../../../../..", &abs_out);
    let dotdot_abs = cat(&dir, &cat(b"/../zi/", b"Area/Nine"));
    let refused: [(Vec<u8>, &[u8]); 6] = [
        (abs_out.clone(), b""),
        (cat(b":", &abs_out), b""),
        (s(&sibling.join("Nine")), b""),
        (up, b""),
        (b"Area/../Area/Nine".to_vec(), b"Area"),
        (dotdot_abs, b""),
    ];
    for tz in &read {
        let l = load_in(tz, None, Some(&dir));
        assert_eq!((offset(&l), abbr(&l)), (32_400, b"NST".to_vec()), "secure {:?}", String::from_utf8_lossy(tz));
        assert!(l.file.is_some(), "a file zone is watched");
    }
    for (tz, name) in &refused {
        let l = load_in(tz, None, Some(&dir));
        assert_eq!((offset(&l), abbr(&l)), (0, name.to_vec()), "secure {:?} is refused", String::from_utf8_lossy(tz));
        assert!(l.file.is_none(), "refused: no file");
        // Outside secure mode the same value names the same file (`..` resolved under TZDIR).
        let l = load_in(tz, Some(&dir), None);
        assert_eq!(offset(&l), 32_400, "plain {:?} is read", String::from_utf8_lossy(tz));
    }
    // A rule is still a rule, also when a `..` component follows it (no file read).
    assert_eq!((offset(&load_in(b"JST-9", None, Some(&dir))), abbr(&load_in(b"JST-9", None, Some(&dir)))), (32_400, b"JST".to_vec()));
    let l = load_in(b"JST-9/../x", None, Some(&dir));
    assert_eq!((offset(&l), abbr(&l), l.file.is_none()), (32_400, b"JST".to_vec(), true));
    // TZDIR is ignored in secure mode: a relative name resolves under the trusted directory only.
    let l = load_in(b"Nine", Some(&s(&out)), Some(&dir));
    assert_eq!((offset(&l), abbr(&l)), (0, b"Nine".to_vec()), "secure: TZDIR ignored");
    assert_eq!(offset(&load_in(b"Nine", Some(&s(&out)), None)), 32_400, "plain: TZDIR honoured");
    // `load` with the process's flag: secure trusts only the system directory, which the scratch
    // directory is not; plain reads the same values.
    for secure in [true, false] {
        let l = time::load(&CSys, Some(&abs_out), None, &missing, secure).unwrap();
        assert_eq!(offset(&l), if secure { 0 } else { 32_400 }, "load secure={secure}: absolute path outside");
        let l = time::load(&CSys, Some(b"Nine"), Some(&s(&out)), &missing, secure).unwrap();
        assert_eq!(offset(&l), if secure { 0 } else { 32_400 }, "load secure={secure}: TZDIR");
    }
    let _ = std::fs::remove_dir_all(&base);
}
