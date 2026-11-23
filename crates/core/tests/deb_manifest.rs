//! The Debian packages' contents as data (PKG-MANIFEST). No spec clause: release packaging.
//!
//! `tools/deb/manifest.tsv` (one row per shipped file) and `tools/deb/control.tsv` (the control
//! fields) are what `tools/deb/build.sh` installs and what the checks expect, through
//! `tools/deb/manifest.py`. The tests:
//!
//! * the tables are well formed and sorted (so two stories adding rows merge cleanly), every
//!   text source and description exists, every product a file row names is one the build
//!   writes (`target/niji`), and every product the build writes is shipped by a file row or
//!   listed in manifest.py's `UNSHIPPED`: a bad or missing row fails here, not in a package build;
//!   the charmap rows are exactly `spec/encoding/charmaps.tsv`'s;
//! * a file in a product directory that file rows read from (`charmaps/`) with no row of its own
//!   fails the lint;
//! * the coexisting flavour (niji6, niji6-dev) built from the products with `build.sh --products`
//!   is exactly the manifest (`manifest.py check coexist`: every row with its type, mode, link
//!   target, contents and strip state, nothing unlisted, the control text): the build's shared
//!   helpers and the check agree;
//! * `-lm` (issue #28): each flavour's `libm.so` is a linker script naming `libm.so.6` and
//!   `libmvec.so.1` AS_NEEDED (never a link), and with the coexist packages a vectorised loop
//!   linked with plain `-lm` needs and runs with `libmvec.so.1`, a scalar program does not need it.
//!
//! Needs `sh`, `python3`, `readelf`, `strip`, `dpkg-deb` (and `gcc`, `timeout` for the `-lm`
//! links); without them it says so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;
#[path = "support/timeout.rs"]
mod timeout;

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The data and the code that read it, named here as paths so that `cargo xtask affected` selects
/// this test for a change to any of them.
const FILES: [&str; 7] = [
    "tools/deb/manifest.tsv",
    "tools/deb/control.tsv",
    "tools/deb/manifest.py",
    "tools/deb/build.sh",
    "tools/deb/check.sh",
    "tools/deb/check-system.sh",
    // The charmaps libc6 ships: lint checks the manifest's charmap rows against it.
    "spec/encoding/charmaps.tsv",
];

/// The `-lm` development file (issue #28): both flavours' linker scripts and the programs linked
/// against them, named so that `cargo xtask affected` selects these tests for a change to any.
const LIBM_SO: [&str; 4] = ["tools/deb/coexist/libm-so.ld", "tools/deb/system/libm-so.ld", "tools/deb/smoke/mvec.c", "tools/deb/smoke/nomvec.c"];

fn lint(products: &std::path::Path) -> (bool, String) {
    let o = Command::new("python3").arg(root().join(FILES[2])).arg("lint").arg("--products").arg(products).output().unwrap();
    (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

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
    let (ok, out) = lint(&products());
    assert!(ok, "manifest lint failed:\n{out}");
    assert!(out.starts_with("manifest: OK"), "manifest lint printed: {out}");
}

// FS-ORDER: each listed entry is linked into the fixture once; the order of the links is never observed.
#[test]
fn an_unshipped_file_in_a_shipped_product_directory_fails_lint() {
    // no spec clause (packaging; PKG-MANIFEST-4): lint --products also looks inside each product
    // directory a file row reads from (charmaps/), so a file the build adds there without a row
    // cannot be left out of the packages unnoticed. The fixture is the real products (linked) plus
    // one extra regular file in charmaps/.
    if tools_missing(&["python3"]) {
        return;
    }
    let src = products();
    let dir = root().join("target/deb-manifest-dirs").join(std::process::id().to_string());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("charmaps")).unwrap();
    for e in std::fs::read_dir(&src).unwrap() {
        let e = e.unwrap();
        if e.file_name() != "charmaps" {
            std::os::unix::fs::symlink(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    for e in std::fs::read_dir(src.join("charmaps")).expect("the products have no charmaps/ (cargo xtask so writes it)") {
        let e = e.unwrap();
        std::os::unix::fs::symlink(e.path(), dir.join("charmaps").join(e.file_name())).unwrap();
    }
    std::fs::write(dir.join("charmaps/NOT-A-ROW"), "<code_set_name> NOT-A-ROW\n").unwrap();
    let (ok, out) = lint(&dir);
    let fails: Vec<&str> = out.lines().filter(|l| l.contains("FAIL")).collect();
    assert!(
        !ok && fails.len() == 1 && fails[0].contains("product charmaps/NOT-A-ROW is in"),
        "lint should refuse exactly the unshipped charmaps/NOT-A-ROW:\n{out}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The coexisting flavour's packages built from `products` with `build.sh --products` into `work`;
/// returns the directory holding the `.deb` files.
fn build_coexist(products: &std::path::Path, work: &std::path::Path) -> PathBuf {
    let _ = std::fs::remove_dir_all(work);
    let out = work.join("out");
    let o = Command::new("sh")
        .arg(root().join(FILES[3]))
        .args(["--version", "0.0~snapshot19700101+000000000000"])
        .arg("--products")
        .arg(products)
        .arg("--work")
        .arg(work.join("work"))
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(o.status.success(), "build.sh --products failed: {}", String::from_utf8_lossy(&o.stderr));
    out
}

#[test]
fn coexist_packages_are_the_manifest() {
    // no spec clause (packaging; PKG-MANIFEST).
    if tools_missing(&["sh", "python3", "readelf", "strip", "dpkg-deb"]) {
        return;
    }
    let work = root().join("target/deb-manifest").join(std::process::id().to_string());
    let out = build_coexist(&products(), &work);
    let o = Command::new("python3").arg(root().join(FILES[2])).args(["check", "coexist"]).arg(&out).output().unwrap();
    let report = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success() && report.is_empty(), "the packages differ from the manifest:\n{report}{}", String::from_utf8_lossy(&o.stderr));
    let _ = std::fs::remove_dir_all(&work);
}

/// A linker script's commands: the text after its last comment, whitespace collapsed.
fn script_commands(text: &str) -> String {
    text.rsplit("*/").next().unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn libm_so_is_a_script_adding_libmvec_as_needed_in_both_flavours() {
    // no spec clause (packaging; issue #28; GNU ld manual, "File Commands": GROUP, AS_NEEDED). A
    // link libm.so -> libm.so.6 leaves a vectorised loop's _ZGV* calls undefined with plain -lm.
    // Every manifest row for a libm.so is a text row (never a link) whose source is one of the two
    // scripts, one row per flavour, and the two scripts give the linker the same commands: the
    // coexist one is linked and run below, so the system one (built only at a snapshot, proved by
    // tools/deb/check-system.sh and root-system.sh) cannot drift from it in the gate.
    let manifest = std::fs::read_to_string(root().join(FILES[0])).unwrap();
    let rows: Vec<Vec<&str>> =
        manifest.lines().filter(|l| !l.starts_with('#')).map(|l| l.split('\t').collect::<Vec<_>>()).filter(|f| f.len() > 4 && f[2].ends_with("/libm.so")).collect();
    let got: Vec<String> = rows.iter().map(|f| format!("{} {} {} {}", f[0], f[2], f[3], f[4])).collect();
    assert_eq!(
        got,
        [
            "coexist usr/lib/niji/usr/lib/x86_64-linux-gnu/libm.so text coexist/libm-so.ld",
            "system usr/lib/x86_64-linux-gnu/libm.so text system/libm-so.ld",
        ],
        "the manifest's libm.so rows (a link cannot add libmvec.so.1)"
    );
    let coexist = script_commands(&std::fs::read_to_string(root().join(LIBM_SO[0])).unwrap());
    let system = script_commands(&std::fs::read_to_string(root().join(LIBM_SO[1])).unwrap());
    assert_eq!(coexist, system, "the two flavours' libm.so scripts differ in their commands");
    assert_eq!(
        coexist,
        "OUTPUT_FORMAT(elf64-x86-64) GROUP ( /lib/x86_64-linux-gnu/libm.so.6 AS_NEEDED ( /lib/x86_64-linux-gnu/libmvec.so.1 ) )",
        "libm.so's commands"
    );
}

/// The dynamic sections' NEEDED names of `exe`, in order.
fn needed(exe: &std::path::Path) -> Vec<String> {
    let o = Command::new("readelf").arg("-dW").arg(exe).output().unwrap();
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter(|l| l.contains("(NEEDED)"))
        .filter_map(|l| l.split('[').nth(1).map(|s| s.trim_end_matches(']').to_string()))
        .collect()
}

#[test]
fn coexist_lm_brings_in_libmvec_only_for_a_vector_call() {
    // no spec clause (packaging; issue #28). With the coexist packages unpacked as a sysroot and
    // niji-gcc: the issue's program, built -O3 -ffast-math and linked with plain -lm, needs
    // libmvec.so.1 and prints 0.8415 through the packaged loader; a program with one scalar call,
    // linked -lm under --no-as-needed, needs libm.so.6 and not libmvec.so.1.
    if tools_missing(&["sh", "python3", "readelf", "strip", "dpkg-deb", "gcc", "timeout"]) {
        return;
    }
    let mvec_src = root().join(LIBM_SO[2]);
    // The vector call exists only where the compiler's headers declare sinf's simd variants.
    let o = Command::new("gcc").args(["-O3", "-ffast-math", "-S", "-o", "-"]).arg(&mvec_src).output().unwrap();
    assert!(o.status.success(), "gcc -S mvec.c: {}", String::from_utf8_lossy(&o.stderr));
    if !String::from_utf8_lossy(&o.stdout).contains("_ZGVbN4v_sinf") {
        skip_note::skipped("skipped: gcc -O3 -ffast-math emits no _ZGVbN4v_sinf call here (its headers declare no simd variant of sinf)");
        return;
    }
    let work = root().join("target/deb-libm-so").join(std::process::id().to_string());
    let out = build_coexist(&products(), &work);
    let sysroot_parent = work.join("root");
    let mut debs: Vec<PathBuf> = std::fs::read_dir(&out).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "deb")).collect();
    debs.sort();
    assert!(!debs.is_empty(), "build.sh wrote no .deb into {}", out.display());
    for deb in &debs {
        let o = Command::new("dpkg-deb").arg("-x").arg(deb).arg(&sysroot_parent).output().unwrap();
        assert!(o.status.success(), "dpkg-deb -x {}: {}", deb.display(), String::from_utf8_lossy(&o.stderr));
    }
    let sysroot = sysroot_parent.join("usr/lib/niji");
    let lib = sysroot.join("usr/lib/x86_64-linux-gnu");
    let script = lib.join("libm.so");
    assert!(std::fs::symlink_metadata(&script).unwrap().file_type().is_file(), "{} is not a regular file", script.display());
    let cc = sysroot_parent.join("usr/bin/niji-gcc");
    let loader = lib.join("ld-linux-x86-64.so.2");
    // build(src, exe, extra link options): links with niji-gcc against the unpacked sysroot.
    let build = |src: &std::path::Path, exe: &std::path::Path, link: &[&str]| {
        let o = Command::new(&cc).env("NIJI_SYSROOT", &sysroot).args(["-O3", "-ffast-math"]).arg(src).args(link).arg("-o").arg(exe).output().unwrap();
        assert!(o.status.success(), "niji-gcc {} {link:?} does not link:\n{}", src.display(), String::from_utf8_lossy(&o.stderr));
    };
    // run(exe): its standard output, started by the packaged loader with the packaged libraries.
    // `env` inside the watchdog sets LD_LIBRARY_PATH, so the host's `timeout` never loads them.
    let run = |exe: &std::path::Path| {
        let mut path = std::ffi::OsString::from("LD_LIBRARY_PATH=");
        path.push(&lib);
        let o = Command::new("timeout").arg(timeout::secs(60)).arg("env").arg(path).arg(&loader).arg(exe).output().unwrap();
        assert!(o.status.success(), "{} exits {:?}: {}", exe.display(), o.status, String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).into_owned()
    };
    let mvec = work.join("mvec");
    build(&mvec_src, &mvec, &["-lm"]);
    let n = needed(&mvec);
    assert!(n.iter().any(|s| s == "libmvec.so.1"), "mvec (-lm) needs {n:?}, without libmvec.so.1");
    assert_eq!(run(&mvec), "0.8415\n", "mvec's output");
    let nomvec = work.join("nomvec");
    build(&root().join(LIBM_SO[3]), &nomvec, &["-Wl,--no-as-needed", "-lm"]);
    let n = needed(&nomvec);
    assert!(n.iter().any(|s| s == "libm.so.6") && !n.iter().any(|s| s == "libmvec.so.1"), "nomvec (--no-as-needed -lm) needs {n:?}: want libm.so.6 and not libmvec.so.1");
    assert_eq!(run(&nomvec), "nomvec: sin(1) 0.8415\n", "nomvec's output");
    let _ = std::fs::remove_dir_all(&work);
}
