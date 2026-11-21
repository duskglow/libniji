//! `tools/deb/version.sh` gives the coexist packages (`niji6`, `niji6-dev`) versions that apt orders by
//! snapshot: no spec clause (release packaging; RELEASE-SNAPSHOT-1120B: snapshot-2026-11-20.2's niji6, with
//! the commit hash 3f2a84520118, sorted below 20.1's, 50abc44ed51f, because the version had no serial).
//!
//! Checks with `dpkg --compare-versions`: two serials of one day sort in serial order whatever their commit
//! hashes (both directions of hash order), a later day sorts above an earlier day's every serial, every
//! serial version sorts above the serial-less versions already published, the system flavour keeps its
//! serial, and a malformed serial is refused. Needs `sh`, `dpkg` and `date`; without them it says so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

use std::path::PathBuf;
use std::process::Command;

const DAY_1115: &str = "1794744000";
const DAY_1120: &str = "1795176000";
const DAY_1121: &str = "1795262400";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn version(flavour: &str, epoch: &str, commit: &str, serial: &str) -> String {
    let o = Command::new("sh")
        .arg(root().join("tools/deb/version.sh"))
        .args([flavour, epoch, commit])
        .env("NIJI_DEB_SERIAL", serial)
        .output()
        .unwrap();
    assert!(o.status.success(), "version.sh: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap().trim().to_string()
}

fn lt(a: &str, b: &str) {
    let ok = Command::new("dpkg").args(["--compare-versions", a, "lt", b]).status().unwrap().success();
    assert!(ok, "{a} must sort below {b}");
}

#[test]
fn same_day_snapshots_sort_by_serial_not_commit() {
    // no spec clause (release packaging; RELEASE-SNAPSHOT-1120B).
    if let Some(t) = ["sh", "dpkg", "date"].into_iter().find(|t| skip_note::missing(t)) {
        skip_note::skipped(&format!("skipped: no {t}"));
        return;
    }
    let (low, high) = ("3f2a84520118", "50abc44ed51f");
    // The later serial has the smaller hash, then the larger one.
    lt(&version("coexist", DAY_1120, high, "1"), &version("coexist", DAY_1120, low, "2"));
    lt(&version("coexist", DAY_1120, low, "1"), &version("coexist", DAY_1120, high, "2"));
    // A second digit in the serial is a number, not a string.
    lt(&version("coexist", DAY_1120, high, "9"), &version("coexist", DAY_1120, low, "10"));
    // The next day sorts above every serial of the day before.
    lt(&version("coexist", DAY_1120, high, "12"), &version("coexist", DAY_1121, low, "1"));
    assert_eq!(version("coexist", DAY_1120, "3f2a84520118ffff", "2"), "0.0~snapshot20261120.2+3f2a84520118");
}

#[test]
fn serial_versions_sort_above_published_serialless_versions() {
    // no spec clause (release packaging): apt upgrades from every snapshot published before the serial.
    if let Some(t) = ["sh", "dpkg", "date"].into_iter().find(|t| skip_note::missing(t)) {
        skip_note::skipped(&format!("skipped: no {t}"));
        return;
    }
    for published in ["0.0~snapshot20261115+aaaaaaaaaaaa", "0.0~snapshot20261120+3f2a84520118", "0.0~snapshot20261120+ffffffffffff"] {
        lt(published, &version("coexist", DAY_1120, "000000000000", "1"));
    }
    lt("0.0~snapshot20261115+ffffffffffff", &version("coexist", DAY_1115, "000000000000", "1"));
}

#[test]
fn system_flavour_keeps_its_serial_and_bad_serials_are_refused() {
    // no spec clause (release packaging; owner ruling R2 for the system version).
    if let Some(t) = ["sh", "date"].into_iter().find(|t| skip_note::missing(t)) {
        skip_note::skipped(&format!("skipped: no {t}"));
        return;
    }
    assert_eq!(version("system", DAY_1120, "3f2a84520118", "2"), "2.39+niji20261120.2-1");
    for bad in ["0", "01", "x", "1.2", "-1"] {
        let o = Command::new("sh")
            .arg(root().join("tools/deb/version.sh"))
            .args(["coexist", DAY_1120, "3f2a84520118"])
            .env("NIJI_DEB_SERIAL", bad)
            .output()
            .unwrap();
        assert!(!o.status.success(), "serial {bad:?} must be refused");
    }
}
