//! The system packages' symbols step on the gate's products (SL-SYMBOLS-NODES). No spec clause:
//! release packaging (plan/system-libc.md section 3.2; deb-symbols(5) is the specification).
//!
//! `tools/deb/build.sh --flavour system --symbols-only` stages libc6's libraries from
//! `target/liella` and runs `tools/deb/symbols.py`, which stops on a version node without a
//! dependency rule and on exports the specification does not list. Run here, a change that adds a
//! node (the version script in `xtask/src/so.rs`, `spec/abi`, csys) or edits `tools/deb/` goes red
//! on its own branch, not on the next system build. The version nodes and exports do not depend on
//! the flavour's prefix, so the gate's products stand in for the system build's. Then each symbol
//! of a Liella-only node `LIELLA_<KIND>_<n>` must name the template `| liella-<kind>-abi (>= <n>)`,
//! and libc6's `Provides:` must offer that package at that level or newer. Needs the products
//! (`cargo xtask so`), `sh`, `python3`, `readelf` and `strip`; without the tools it says so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn every_version_node_has_a_dependency_rule() {
    // no spec clause (packaging; plan/system-libc.md section 3.2, SL-SYMBOLS-NODES).
    if let Some(t) = ["sh", "python3", "readelf", "strip"].into_iter().find(|t| skip_note::missing(t)) {
        skip_note::skipped(&format!("skipped: no {t}"));
        return;
    }
    let products = root().join("target/liella");
    assert!(products.join("libc.so.6").exists(), "the products are missing: run `cargo xtask so`");
    let work = root().join("target/deb-symbols-products").join(std::process::id().to_string());
    let _ = std::fs::remove_dir_all(&work);
    let o = Command::new("sh")
        .arg(root().join("tools/deb/build.sh"))
        .args(["--flavour", "system", "--symbols-only", "--version", "2.39+liella19700101.1-1"])
        .arg("--products")
        .arg(&products)
        .arg("--work")
        .arg(&work)
        .arg("--out")
        .arg(work.join("out"))
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "the symbols step failed: {err}");
    let debian = work.join("stage/libc6/DEBIAN");
    let symbols = std::fs::read_to_string(debian.join("symbols")).unwrap();
    let control = std::fs::read_to_string(debian.join("control")).unwrap();

    // Templates per library block, then each Liella-only node's symbols against them.
    let mut templates: Vec<String> = Vec::new();
    let mut named: BTreeMap<String, u32> = BTreeMap::new();
    let mut lines = 0;
    for line in symbols.lines() {
        if !line.starts_with([' ', '|', '*']) {
            templates.clear();
        } else if let Some(t) = line.strip_prefix("| ") {
            templates.push(t.to_string());
        } else if let Some(rest) = line.strip_prefix(' ') {
            let f: Vec<&str> = rest.split_whitespace().collect();
            let Some((_, node)) = f[0].split_once('@') else { continue };
            let Some(kl) = node.strip_prefix("LIELLA_").filter(|k| *k != "PRIVATE") else { continue };
            let (kind, level) = kl.rsplit_once('_').unwrap_or_else(|| panic!("{line}: not LIELLA_<KIND>_<n>"));
            let pkg = format!("liella-{}-abi", kind.to_lowercase());
            let want = format!("{pkg} (>= {level})");
            let id: usize = f.get(2).and_then(|i| i.parse().ok()).unwrap_or_else(|| panic!("{line}: no template"));
            assert_eq!(templates.get(id - 1), Some(&want), "{line}: template {id} of {templates:?}");
            let level: u32 = level.parse().unwrap();
            let top = named.entry(pkg).or_insert(level);
            *top = (*top).max(level);
            lines += 1;
        }
    }
    assert!(lines > 0 && named.contains_key("liella-native-abi"), "no Liella-only node in the symbols file: {named:?}");
    let provides = control.lines().find_map(|l| l.strip_prefix("Provides: ")).unwrap_or_default();
    for (pkg, level) in &named {
        let offered = provides.split(", ").find_map(|p| p.strip_prefix(&format!("{pkg} (= "))?.strip_suffix(')')?.parse::<u32>().ok());
        assert!(offered.is_some_and(|o| o >= *level), "libc6 Provides '{provides}': no {pkg} (= {level} or newer)");
    }
    let _ = std::fs::remove_dir_all(&work);
}
