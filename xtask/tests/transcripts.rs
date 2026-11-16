//! UTIL-HARNESS: `cargo xtask transcript check` over the committed transcripts (validation/transcripts/), run as a
//! gate runs it: libniji's side (the `ldconfig -p` stub until UTIL-3) against the reference where it is installed and
//! against the record, and a transcript whose record was changed by one byte must fail with a MISMATCH.
//!
//! Trace: s31_2_g_6 (`ldconfig -p` listing), s31_2_f_1 (a cache that cannot be opened); the harness itself has no
//! spec clause (test-harness tooling).

use std::path::Path;
use std::process::Command;

fn check(operands: &[&str]) -> (Option<i32>, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root");
    let out = Command::new(env!("CARGO_BIN_EXE_xtask")).arg("transcript").arg("check").args(operands).current_dir(root).output().expect("run the xtask binary");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.code(), text)
}

#[test]
fn the_committed_ldconfig_transcripts_hold() {
    // Trace: s31_2_g_6, s31_2_f_1.
    let (code, text) = check(&["validation/transcripts/ldconfig"]);
    println!("{text}");
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains(" 0 mismatched, ") && text.contains(" 0 errors "), "{text}");
    for line in text.lines().filter(|l| l.starts_with("SKIPPED")) {
        // Only a host without unprivileged user namespaces may skip, and the line says why.
        assert!(line.contains("needs user and mount namespaces: "), "{line}");
    }
}

#[test]
fn the_shipped_ldconfig_passes_the_transcripts() {
    // Trace: s31_2_g_6, s31_2_f_1. The freestanding program `cargo xtask so` links (D-31.2), run over the same
    // transcripts as the engine's hosted front end: its system-call file system (`-r` through openat2) must agree.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root");
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), std::path::PathBuf::from);
    let program = target.join("niji/ldconfig");
    if !program.exists() {
        eprintln!("SKIP: {} not built (run cargo xtask so)", program.display());
        return;
    }
    let niji = format!("ldconfig={}", program.display());
    let (code, text) = check(&["validation/transcripts/ldconfig", "--niji", &niji]);
    println!("{text}");
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains(" 0 mismatched, ") && text.contains(" 0 errors "), "{text}");
}

#[test]
fn a_record_changed_by_one_byte_is_a_mismatch() {
    // Trace: s31_2_g_6.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root");
    let text = std::fs::read_to_string(root.join("validation/transcripts/ldconfig/p-flags.transcript")).expect("read");
    let changed = text.replacen("(libc6,x86-64) => /usr/lib/libg.so.1", "(libc6,x86-64) => /usr/lib/libG.so.1", 1);
    assert_ne!(changed, text, "the transcript lists libg.so.1");
    let dir = std::env::temp_dir().join(format!("xtask-transcripts-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("changed.transcript");
    std::fs::write(&file, changed).expect("write");
    let (code, out) = check(&[file.to_str().expect("utf-8 path")]);
    std::fs::remove_dir_all(&dir).expect("cleanup");
    println!("{out}");
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("MISMATCH stdout") && out.contains("libG.so.1"), "{out}");
}
