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

/// The test link of iconv (`cargo xtask so`, UTIL-7), if built.
fn iconv_test_link() -> Option<std::path::PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root").join("target/niji-test/iconv");
    if p.is_file() {
        return Some(p);
    }
    println!("SKIPPED: target/niji-test/iconv is not built (run `cargo xtask so` first)");
    None
}

#[test]
fn the_committed_iconv_transcripts_hold() {
    // Trace: s31_6_c_1, s31_6_i_1, s31_6_g_1, s31_6_g_2, s31_6_f_1, s31_6_f_2, s31_6_f_3, s31_1_f_1.
    if iconv_test_link().is_none() {
        return;
    }
    let (code, text) = check(&["validation/transcripts/iconv"]);
    println!("{text}");
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains(" 0 mismatched, ") && text.contains(" 0 errors "), "{text}");
}

#[test]
fn every_name_iconv_lists_converts() {
    // Trace: s31_6_g_3 (`-l`: exactly the names libniji can convert; this checks every listed name converts).
    let Some(iconv) = iconv_test_link() else { return };
    let out = Command::new(&iconv).arg("-l").env_clear().output().expect("run iconv -l");
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).expect("UTF-8 listing");
    let names: Vec<&str> = text.lines().map(|l| l.strip_suffix("//").or_else(|| l.strip_suffix('/')).expect("NAME// form")).collect();
    assert!(names.len() >= 63, "only {} names", names.len());
    for name in names {
        let r = Command::new(&iconv).args(["-f", name, "-t", "UTF-8", "/dev/null"]).env_clear().output().expect("run iconv");
        assert_eq!(r.status.code(), Some(0), "{name} is listed but does not convert: {}", String::from_utf8_lossy(&r.stderr));
    }
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

/// The freestanding `ldconfig` (D-31.2), LDCONFIG-LIMITS (owner ruling: no artificial caps; running out of memory is
/// a clean error): 5002 arguments, 4999 directories (old cap 4096, refused with `E2BIG`'s text) are all read, and a configuration that
/// does not fit the address space an `RLIMIT_AS` allows ends in `out of memory`, exit 1, never a crash.
#[test]
fn the_shipped_ldconfig_reads_every_argument_and_reports_exhaustion() {
    // Trace: s31_2_i_1, s31_2_i_2.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root");
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), std::path::PathBuf::from);
    let program = target.join("niji/ldconfig");
    if !program.exists() {
        eprintln!("SKIP: {} not built (run cargo xtask so)", program.display());
        return;
    }
    let scratch = std::env::temp_dir().join(format!("ldconfig-limits-{}", std::process::id()));
    let dirs: Vec<String> = (0..4999)
        .map(|i| {
            let dir = scratch.join(i.to_string());
            std::fs::create_dir_all(&dir).expect("scratch directory");
            dir.display().to_string()
        })
        .collect();
    let out = Command::new(&program).args(["-n", "-X", "-v"]).args(&dirs).output().expect("run ldconfig");
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "every argument is read: {}", err.lines().next().unwrap_or_default());
    assert_eq!(String::from_utf8_lossy(&out.stdout).matches(": (from <cmdline>:0)\n").count(), 4999, "each operand is scanned");
    // A sparse 256 MiB configuration (no disk blocks) under a 64 MiB address-space limit.
    let conf = scratch.join("ld.so.conf");
    std::fs::File::create(&conf).and_then(|f| f.set_len(256 << 20)).expect("sparse configuration");
    let out = Command::new("/bin/sh")
        .args(["-c", "ulimit -v 65536 && exec \"$0\" -N -X -f \"$1\""])
        .arg(&program)
        .arg(&conf)
        .output()
        .expect("run ldconfig under a limit");
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "a clean exit, not a signal: {err}");
    assert!(err.ends_with("ldconfig: out of memory\n"), "the out-of-memory report: {err}");
    std::fs::remove_dir_all(&scratch).expect("remove the scratch directory");
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
