//! The Debian packages' contents as data (PKG-MANIFEST). No spec clause: release packaging.
//!
//! `tools/deb/manifest.tsv` (one row per shipped file) and `tools/deb/control.tsv` (the control
//! fields) are what `tools/deb/build.sh` installs and what the checks expect, through
//! `tools/deb/manifest.py`. Two tests:
//!
//! * the tables are well formed and sorted (so two stories adding rows merge cleanly), every
//!   text source and description exists, and every product a file row names is one the build
//!   writes (`target/niji`): a bad row fails here, not in a package build;
//! * the coexisting flavour (niji6, niji6-dev) built from the products with `build.sh --products`
//!   is exactly the manifest (`manifest.py check coexist`: every row with its type, mode, link
//!   target, contents and strip state, nothing unlisted, the control text): the build's shared
//!   helpers and the check agree.
//!
//! Needs `sh`, `python3`, `readelf`, `strip`, `dpkg-deb`; without them it says so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The data and the code that read it, named here as paths so that `cargo xtask affected` selects
/// this test for a change to any of them.
const FILES: [&str; 6] = [
    "tools/deb/manifest.tsv",
    "tools/deb/control.tsv",
    "tools/deb/manifest.py",
    "tools/deb/build.sh",
    "tools/deb/check.sh",
    "tools/deb/check-system.sh",
];

fn tools_missing(tools: &[&str]) -> bool {
    if let Some(t) = tools.iter().find(|t| skip_note::missing(t)) {
        skip_note::skipped(&format!("skipped: no {t}"));
        return true;
    }
    false
}

fn products() -> PathBuf {
    let p = root().join("target/niji");
    assert!(p.join("libc.so.6").exists(), "the products are missing: run `cargo xtask so`");
    p
}

#[test]
fn tables_are_sorted_well_formed_and_name_built_products() {
    // no spec clause (packaging; PKG-MANIFEST).
    if tools_missing(&["python3"]) {
        return;
    }
    for f in FILES {
        assert!(root().join(f).is_file(), "{f} is missing");
    }
    let o = Command::new("python3").arg(root().join(FILES[2])).arg("lint").arg("--products").arg(products()).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success(), "manifest lint failed:\n{out}{}", String::from_utf8_lossy(&o.stderr));
    assert!(out.starts_with("manifest: OK"), "manifest lint printed: {out}");
}

#[test]
fn coexist_packages_are_the_manifest() {
    // no spec clause (packaging; PKG-MANIFEST).
    if tools_missing(&["sh", "python3", "readelf", "strip", "dpkg-deb"]) {
        return;
    }
    let products = products();
    let work = root().join("target/deb-manifest").join(std::process::id().to_string());
    let _ = std::fs::remove_dir_all(&work);
    let out = work.join("out");
    let o = Command::new("sh")
        .arg(root().join(FILES[3]))
        .args(["--version", "0.0~snapshot19700101+000000000000"])
        .arg("--products")
        .arg(&products)
        .arg("--work")
        .arg(work.join("work"))
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(o.status.success(), "build.sh --products failed: {}", String::from_utf8_lossy(&o.stderr));
    let o = Command::new("python3").arg(root().join(FILES[2])).args(["check", "coexist"]).arg(&out).output().unwrap();
    let report = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success() && report.is_empty(), "the packages differ from the manifest:\n{report}{}", String::from_utf8_lossy(&o.stderr));
    let _ = std::fs::remove_dir_all(&work);
}
