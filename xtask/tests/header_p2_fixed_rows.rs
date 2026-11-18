//! SL-HEADERS-VIS-FIX, plan/headers.md section 3, P2: the headers as the templates make them NOW (`headers-gen --out`,
//! not the committed `libc-include/`) show the names of this story's rows in exactly the configurations of
//! `spec/abi/visibility.tsv`: `WIFCONTINUED` (stdlib.h), no feature-test macro from `features-time64.h` or
//! `gnu/stubs.h` included alone, and `struct timespec` beside `struct stat` (sys/pidfd.h) only where it is the
//! implementation's. A plant on a template or on the generator is seen here without any other build.
//! Runs `spec/tools/header_probe.py` and `spec/tools/probe_compare.py`; needs cc and python3.
//!
//! Trace: no spec clause (header visibility proof P2, plan/headers.md).

use std::path::{Path, PathBuf};
use std::process::Command;

const HEADERS: &str = "stdlib.h,gnu/stubs.h,features-time64.h,sys/pidfd.h";

fn have(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok_and(|o| o.status.success())
}

#[test]
fn p2_visibility_of_the_fixed_rows() {
    if !have("cc") || !have("python3") {
        eprintln!("p2_visibility_of_the_fixed_rows: skipped, no cc or python3 (P2 compiles its probes with them)");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("workspace root").to_path_buf();
    let base: PathBuf = std::env::temp_dir().join(format!("header-p2-fixed-rows.{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (inc, got) = (base.join("include"), base.join("got"));
    std::fs::create_dir_all(&got).expect("make the scratch directory");

    let gen = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("headers-gen")
        .arg("--out")
        .arg(&inc)
        .current_dir(&root)
        .output()
        .expect("run xtask headers-gen");
    assert!(gen.status.success(), "headers-gen --out: {}", String::from_utf8_lossy(&gen.stderr));

    let probe = Command::new("python3")
        .arg(root.join("spec/tools/header_probe.py"))
        .arg("--include")
        .arg(&inc)
        .args(["--headers", HEADERS, "--jobs", "8", "--out"])
        .arg(&got)
        .current_dir(&root)
        .output()
        .expect("run header_probe.py");
    assert!(probe.status.success(), "header_probe.py: {}", String::from_utf8_lossy(&probe.stderr));

    let cmp = Command::new("python3")
        .arg(root.join("spec/tools/probe_compare.py"))
        .args(["visibility", "--headers", HEADERS, "--got"])
        .arg(&got)
        .current_dir(&root)
        .output()
        .expect("run probe_compare.py");
    let out = String::from_utf8_lossy(&cmp.stdout).into_owned();
    let err = String::from_utf8_lossy(&cmp.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&base);
    assert!(cmp.status.success(), "P2 differences in this story's rows:\n{out}{err}");
}
