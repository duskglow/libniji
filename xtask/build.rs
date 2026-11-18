//! Lists the header slices' generator modules, `src/headers/slices/<slice>.rs`, for `headers.rs`
//! (SL-HEADERS-REGISTRY; plan/headers.md section 4, "Adding a slice"). A slice adds itself by adding its file: no
//! shared list of modules to edit, so two slices developed in parallel touch no common line.
//!
//! Writes `$OUT_DIR/header_slices.rs`: one `#[path] mod <slice>;` per file (sorted by name, not directory order) and
//! `ALL`, each module's `SLICE` with its name.
#![allow(clippy::disallowed_methods)] // a host tool: S8 governs the library crates only
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("src/headers/slices");
    // A directory: cargo reruns this script when any file below it is added, removed or changed.
    println!("cargo:rerun-if-changed={}", dir.display());
    println!("cargo:rerun-if-changed=build.rs");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .flatten()
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".rs")).map(String::from))
        .collect();
    names.sort();
    let mut out = String::from("// Written by xtask/build.rs from the file names of xtask/src/headers/slices/.\n");
    for n in &names {
        let ident = n.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        assert!(ident, "xtask/src/headers/slices/{n}.rs: a slice module is named [a-z][a-z0-9_]*.rs");
        out.push_str(&format!("#[path = {:?}]\nmod {n};\n", dir.join(format!("{n}.rs"))));
    }
    out.push_str("/// Every slice module's additions to the generator, with the module's name, in name order.\n");
    out.push_str("pub(super) const ALL: &[(&str, &super::Slice)] = &[\n");
    for n in &names {
        out.push_str(&format!("    ({n:?}, &{n}::SLICE),\n"));
    }
    out.push_str("];\n");
    let dst = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("header_slices.rs");
    if fs::read_to_string(&dst).ok().as_deref() != Some(out.as_str()) {
        fs::write(&dst, out).unwrap_or_else(|e| panic!("{}: {e}", dst.display()));
    }
}
