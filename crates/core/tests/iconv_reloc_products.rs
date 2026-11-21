//! The built-in iconv tables cost a process nothing at load (ICONV-CJK-STARTUP): the CJK
//! descriptors (`niji::iconv::cjk_tables`) and the built-in name table (`niji::iconv::NAME_ROWS`,
//! `NAME_BYTES`) hold offsets, never addresses, so `libc.so.6` has no dynamic relocation inside
//! them, and no relocated word (e.g. a compiler lookup table of descriptor addresses) points into
//! them. A pointer in one of them is one `R_X86_64_RELATIVE` per row that every dynamic program
//! applies at start, whether it converts or not (snapshot 2026-11-21.6: 71 of them, +1,408
//! instructions per start). Needs `cargo xtask so` and `readelf`; without either it skips.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

use std::path::PathBuf;
use std::process::Command;

fn readelf(flag: &str, so: &PathBuf) -> String {
    let o = Command::new("readelf").arg(flag).arg(so).output().unwrap();
    assert!(o.status.success(), "readelf {flag} {}", so.display());
    String::from_utf8(o.stdout).unwrap()
}

/// The table symbols checked: (mangled-name part, required).
const TABLES: [(&str, bool); 4] = [("5iconv10cjk_tables", false), ("5iconv10cjk_tables4SETS", true), ("5iconv9NAME_ROWS", true), ("5iconv10NAME_BYTES", true)];

#[test]
fn iconv_tables_have_no_load_relocations() {
    // no spec clause: a start-up cost bound (load-time relocations), not behaviour.
    let so = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/niji/libc.so.6");
    if !so.exists() || skip_note::missing("readelf") {
        skip_note::skipped("skipped: no target/niji/libc.so.6 (cargo xtask so) or no readelf");
        return;
    }
    // (name, start, end) of every object symbol of the tables.
    let mut objs = Vec::new();
    for line in readelf("-sW", &so).lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 8 || f[3] != "OBJECT" || !TABLES.iter().any(|(t, _)| f[7].contains(t)) {
            continue;
        }
        let (Ok(at), Ok(size)) = (u64::from_str_radix(f[1], 16), f[2].parse::<u64>()) else { continue };
        if size > 0 {
            objs.push((f[7].to_owned(), at, at + size));
        }
    }
    for (t, required) in TABLES {
        assert!(!required || objs.iter().any(|(n, _, _)| n.contains(t)), "no symbol `*{t}*` in {}: the check would see nothing", so.display());
    }
    let inside = |a: u64| objs.iter().find(|(_, s, e)| (*s..*e).contains(&a)).map(|(n, _, _)| n.as_str());
    // A `.got` slot holds one table's address once, whatever its row count: the non-LTO (dev)
    // profile keeps the crate's statics preemptible-looking and reads them through
    // `add sym@GOTPCREL(%rip)`, which the linker cannot rewrite in a shared object (ICONV-CJK-STARTUP:
    // `builtin` on `NAME_BYTES`, and the same for about 20 other statics of that build, e.g.
    // `locale::classes::C_TABLE`). The release profile (fat LTO) has no such slot. Not a per-row
    // cost and not what this check bounds, so `.got` words are left out of "points into".
    let mut got = 0..0;
    for line in readelf("-SW", &so).lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if let Some(i) = f.iter().position(|w| *w == ".got") {
            if let (Some(Ok(at)), Some(Ok(size))) = (f.get(i + 2).map(|w| u64::from_str_radix(w, 16)), f.get(i + 4).map(|w| u64::from_str_radix(w, 16))) {
                got = at..at + size;
            }
        }
    }
    let mut bad = Vec::new();
    for line in readelf("-rW", &so).lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 4 || f[2] != "R_X86_64_RELATIVE" {
            continue;
        }
        let (Ok(at), Ok(to)) = (u64::from_str_radix(f[0], 16), u64::from_str_radix(f[3], 16)) else { continue };
        if let Some(n) = inside(at) {
            bad.push(format!("relocated word at {at:#x} inside {n}"));
        } else if let Some(n) = inside(to).filter(|_| !got.contains(&at)) {
            bad.push(format!("relocated word at {at:#x} points into {n}"));
        }
    }
    assert!(bad.is_empty(), "iconv table relocations in {} ({}):\n{}", so.display(), bad.len(), bad.join("\n"));
}
