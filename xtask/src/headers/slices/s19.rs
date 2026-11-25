//! Slice 19 (plan/headers.md section 4: diagnostic attributes). Listed by `xtask/build.rs`.
//!
//! The slice adds no marker: `spec/abi/diagattrs.tsv` is read by the generator core (each declaration `@@emit`
//! writes, and the inline `@@attrs(NAME)` of a hand-written line, carry the row's ` __attribute__ ((...))`), and this
//! module holds the differential test against the reference's public headers.
use super::super::Slice;

pub(super) const SLICE: Slice = Slice { markers: &[], inline: &[], linked_elsewhere: &[] };

#[cfg(test)]
mod tests {
    use super::super::super::generate;
    use std::fs;
    use std::path::Path;

    // plan/headers.md slice 19 (no spec clause: the standards give the argument rules, the reference's
    // headers give which function carries which attribute). The headers are regenerated in memory from
    // the tables, written to a scratch directory and compared with the reference's headers by
    // spec/tools/attr_probe.py check: the same warnings (option and line) on a corpus of its own.
    // Skipped, and says why, where the compiler or the reference's headers are absent.
    #[test]
    fn diagnostic_attributes_match_the_reference() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        if !Path::new("/usr/include/stdio.h").exists() || std::process::Command::new("gcc").arg("--version").output().is_err() {
            eprintln!("skipped: needs gcc and the system's public headers (the reference of the differential)");
            return;
        }
        let (files, errors) = generate(&root).expect("generate");
        assert!(errors.is_empty(), "{errors:?}");
        let dir = std::env::temp_dir().join(format!("niji-attr-test.{}", std::process::id()));
        for (h, text) in &files {
            let p = dir.join(h);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, text).unwrap();
        }
        let out = std::process::Command::new("python3")
            .arg(root.join("spec/tools/attr_probe.py"))
            .args(["check", "--include"])
            .arg(&dir)
            .arg("--keep")
            .arg(dir.join("probe-work"))
            .output()
            .expect("python3");
        let _ = fs::remove_dir_all(&dir);
        let text = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "attr_probe check failed:\n{text}");
    }

    // plan/headers.md slice 19 (no spec clause; HEADERS-NONNULL-MATCH). The nonnull columns of diagattrs.tsv are what
    // the probe derives from the reference now: `attr_probe table --columns nonnull,nonnull_nofortify,nonnull_fortify` (the nonnull
    // tests alone) rewrites a copy of the table, which must come out byte for byte the same. `check` above compares
    // the two sides through the same probe, so a probe defect that misreads both alike (a null callback on every
    // line, a size that wakes an `access` attribute, a builtin's own nonnull) passes it; this test sees it.
    // Skipped, and says why, where the compiler or the reference's headers are absent.
    #[test]
    fn nonnull_columns_are_the_probes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        if !Path::new("/usr/include/stdio.h").exists() || std::process::Command::new("gcc").arg("--version").output().is_err() {
            eprintln!("skipped: needs gcc and the system's public headers (the reference of the probe)");
            return;
        }
        let table = root.join("spec/abi/diagattrs.tsv");
        let want = fs::read_to_string(&table).expect("diagattrs.tsv");
        let dir = std::env::temp_dir().join(format!("niji-nonnull-test.{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let copy = dir.join("diagattrs.tsv");
        fs::write(&copy, &want).unwrap();
        let out = std::process::Command::new("python3")
            .arg(root.join("spec/tools/attr_probe.py"))
            .args(["table", "--columns", "nonnull,nonnull_nofortify,nonnull_fortify", "--out"])
            .arg(&copy)
            .arg("--keep")
            .arg(dir.join("probe-work"))
            .output()
            .expect("python3");
        let got = fs::read_to_string(&copy).unwrap_or_default();
        let _ = fs::remove_dir_all(&dir);
        let text = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "attr_probe table failed:\n{text}");
        let differ: Vec<_> = want.lines().zip(got.lines()).filter(|(a, b)| a != b).take(20).collect();
        assert!(want == got, "the nonnull columns differ from the probe's ({} vs {} lines); first rows (table, probe):\n{differ:#?}", want.lines().count(), got.lines().count());
    }
}
