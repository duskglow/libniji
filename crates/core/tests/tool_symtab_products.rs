//! The names tools read in `libc.so.6`'s `.symtab` (VALGRIND-LOADER-VIEW, GitHub issue #13, D-X.153). No
//! spec clause: `.symtab` is not part of the ABI; valgrind's tools (3.24) match its names literally and
//! wrap some functions only under a versioned name (helgrind wraps `pthread_create@*`), so a default
//! that has a compat version beside it must also be named `name@@VERSION` there, at the default's
//! address with its size (valgrind takes symbols with one address and one size as one function, and
//! passes over a size-0 one). Needs the products (`cargo xtask so`) and `readelf`; without `readelf`
//! the test says so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

fn readelf(args: &[&str]) -> String {
    let so = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/niji/libc.so.6");
    assert!(so.exists(), "libc.so.6 is missing: run `cargo xtask so`");
    let o = Command::new("readelf").args(args).arg(&so).output().unwrap();
    assert!(o.status.success(), "readelf {args:?} libc.so.6: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// (table, name, value, size, type) of every defined symbol row of `readelf -sW` text.
fn rows(text: &str) -> Vec<(String, String, String, String, String)> {
    let mut table = String::new();
    let mut v = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Symbol table '") {
            table = rest.split('\'').next().unwrap().to_string();
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 8 && f[0].ends_with(':') && f[6] != "UND" {
            v.push((table.clone(), f[7].to_string(), f[1].to_string(), f[2].to_string(), f[3].to_string()));
        }
    }
    v
}

/// VALGRIND-LOADER-VIEW (D-X.153); no spec clause (a tool's view of `.symtab`). Every function
/// libc.so.6 exports at a default version and at a compat version is also named
/// `name@@VERSION` in `.symtab`, with the plain name's value, size (not 0) and type; `pthread_create`
/// (the one helgrind needs) among them.
#[test]
fn compat_defaults_are_named_with_their_version_in_symtab() {
    if skip_note::missing("readelf") {
        skip_note::skipped("skipped: no readelf");
        return;
    }
    let all = rows(&readelf(&["-sW"]));
    let mut defaults = BTreeMap::new();
    let mut compat = BTreeSet::new();
    for (table, name, value, size, ty) in &all {
        if table != ".dynsym" || !matches!(ty.as_str(), "FUNC" | "IFUNC") {
            continue;
        }
        let Some((n, v)) = name.split_once('@') else { continue };
        match v.strip_prefix('@') {
            Some(v) => {
                defaults.insert(n.to_string(), (v.to_string(), value.clone(), size.clone(), ty.clone()));
            }
            None => {
                compat.insert(n.to_string());
            }
        }
    }
    let symtab: BTreeSet<(&str, &str, &str, &str)> = all.iter().filter(|r| r.0 == ".symtab").map(|r| (r.1.as_str(), r.2.as_str(), r.3.as_str(), r.4.as_str())).collect();
    let mut missing = Vec::new();
    let mut checked = 0;
    for (n, (v, value, size, ty)) in defaults.iter().filter(|(n, _)| compat.contains(*n)) {
        checked += 1;
        let alias = format!("{n}@@{v}");
        if size == "0" || !symtab.contains(&(alias.as_str(), value.as_str(), size.as_str(), ty.as_str())) || !symtab.contains(&(n.as_str(), value.as_str(), size.as_str(), ty.as_str())) {
            missing.push(alias);
        }
    }
    assert!(defaults.get("pthread_create").is_some_and(|d| d.0 == "GLIBC_2.34") && compat.contains("pthread_create"), "libc.so.6 does not export pthread_create at GLIBC_2.34 and a compat version");
    assert!(missing.is_empty(), "{} of {checked} defaults with a compat version are not named name@@VERSION in .symtab beside the plain name at its value and size: {:?}", missing.len(), &missing[..missing.len().min(8)]);
}
