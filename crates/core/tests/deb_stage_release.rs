//! `tools/deb/stage-release.sh` stages release assets under the names GitHub publishes. No spec clause:
//! release packaging (issue #3: GitHub does not allow `~` in asset names and publishes it as `.`, so a
//! SHA256SUMS naming the `~` file failed `sha256sum -c` on the downloads).
//!
//! The test builds a small .deb whose version has `~` (like `niji6`'s `0.0~snapshot...`), stages it with
//! a second file whose version has none, and checks: no staged name has `~`; SHA256SUMS names exactly the
//! staged files and `sha256sum -c` passes in the staging directory; the version inside the staged .deb
//! still has `~`; MANIFEST maps the staged name to the package and the Debian version and is no asset;
//! a name with a character GitHub rewrites is refused and creates nothing. Needs `sh`, `dpkg-deb` and
//! `sha256sum`; without them it says so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Build `<dir>/<pkg>_<version>_all.deb` with `dpkg-deb --build`.
fn make_deb(dir: &Path, pkg: &str, version: &str, file: &str) -> PathBuf {
    let tree = dir.join(format!("tree-{pkg}"));
    std::fs::create_dir_all(tree.join("DEBIAN")).unwrap();
    let control = format!("Package: {pkg}\nVersion: {version}\nArchitecture: all\nMaintainer: t <t@example.org>\nDescription: t\n x\n");
    std::fs::write(tree.join("DEBIAN/control"), control).unwrap();
    let deb = dir.join(file);
    let o = Command::new("dpkg-deb").arg("--build").arg(&tree).arg(&deb).output().unwrap();
    assert!(o.status.success(), "dpkg-deb: {}", String::from_utf8_lossy(&o.stderr));
    deb
}

fn stage(out: &Path, files: &[&Path]) -> Output {
    Command::new("sh")
        .arg(root().join("tools/deb/stage-release.sh"))
        .arg("--out")
        .arg(out)
        .args(["--head", "0123456789abcdef"])
        .args(files)
        .output()
        .unwrap()
}

#[test]
fn tilde_assets_are_staged_under_published_names() {
    // no spec clause (release packaging; GitHub issue #3).
    if let Some(t) = ["sh", "dpkg-deb", "sha256sum"].into_iter().find(|t| skip_note::missing(t)) {
        skip_note::skipped(&format!("skipped: no {t}"));
        return;
    }
    let work = root().join("target/deb-stage-release").join(std::process::id().to_string());
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let tilde = make_deb(&work, "fakepkg", "0.0~snapshot20261115+abc", "fakepkg_0.0~snapshot20261115+abc_amd64.deb");
    let plain = make_deb(&work, "otherpkg", "2.39+x.2-1", "otherpkg_2.39+x.2-1_amd64.deb");

    let out = work.join("staged");
    let o = stage(&out, &[&tilde, &plain]);
    assert!(o.status.success(), "stage-release failed: {}", String::from_utf8_lossy(&o.stderr));

    let mut names: Vec<String> =
        std::fs::read_dir(&out).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    assert!(names.iter().all(|n| !n.contains('~')), "a staged name still has '~': {names:?}");
    let published = "fakepkg_0.0.snapshot20261115+abc_amd64.deb";
    assert!(names.iter().any(|n| n == published), "{published} is not staged: {names:?}");
    assert!(names.iter().any(|n| n == "otherpkg_2.39+x.2-1_amd64.deb"), "an unchanged name was altered: {names:?}");

    // SHA256SUMS names exactly the staged .debs, and verifies on them.
    let sums = std::fs::read_to_string(out.join("SHA256SUMS")).unwrap();
    let mut listed: Vec<&str> = sums.lines().filter_map(|l| l.split_whitespace().nth(1)).map(|n| n.trim_start_matches('*')).collect();
    listed.sort_unstable();
    let debs: Vec<&str> = names.iter().map(String::as_str).filter(|n| n.ends_with(".deb")).collect();
    assert_eq!(listed, debs, "SHA256SUMS does not name exactly the staged files");
    let c = Command::new("sha256sum").arg("-c").arg("SHA256SUMS").current_dir(&out).output().unwrap();
    assert!(c.status.success(), "sha256sum -c SHA256SUMS failed: {}", String::from_utf8_lossy(&c.stdout));

    // The version inside the package is unchanged.
    let v = Command::new("dpkg-deb").arg("-f").arg(out.join(published)).arg("Version").output().unwrap();
    assert_eq!(String::from_utf8_lossy(&v.stdout).trim(), "0.0~snapshot20261115+abc");

    // MANIFEST maps staged name -> package and Debian version; it is not listed as an asset.
    let manifest = std::fs::read_to_string(out.join("MANIFEST")).unwrap();
    let line = manifest.lines().find(|l| l.ends_with(published)).expect("the manifest has no line for the staged name");
    assert!(line.contains("fakepkg") && line.contains("0.0~snapshot20261115+abc"), "manifest line: {line}");
    assert!(!sums.contains("MANIFEST"), "MANIFEST is listed in SHA256SUMS");

    // A name GitHub would rewrite differently is refused, and nothing is created.
    let odd = work.join("odd name_1_all.deb");
    std::fs::copy(&plain, &odd).unwrap();
    let out2 = work.join("staged2");
    let o = stage(&out2, &[&odd]);
    assert!(!o.status.success(), "a name with a space was staged");
    assert!(!out2.exists(), "a refused run created the directory");
    let _ = std::fs::remove_dir_all(&work);
}
