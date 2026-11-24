//! UTIL-HARNESS: `cargo xtask transcript check` over the committed transcripts (validation/transcripts/), run as a
//! gate runs it: libniji's side (the `ldconfig -p` stub until UTIL-3) against the reference where it is installed and
//! against the record, and a transcript whose record was changed by one byte must fail with a MISMATCH.
//!
//! Trace: s31_2_g_6 (`ldconfig -p` listing), s31_2_f_1 (a cache that cannot be opened); the harness itself has no
//! spec clause (test-harness tooling).
#![allow(clippy::disallowed_methods)] // a host tool's test: S8 governs the library crates only

use std::path::Path;
use std::process::Command;

fn check(operands: &[&str]) -> (Option<i32>, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root");
    let out = Command::new(env!("CARGO_BIN_EXE_xtask")).arg("transcript").arg("check").args(operands).current_dir(root).output().expect("run the xtask binary");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.code(), text)
}

mod test_link;

/// The test link of iconv (`cargo xtask so`, UTIL-7), if built and its loader and library are there.
fn iconv_test_link() -> Option<std::path::PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root").join("target/niji-test/iconv");
    test_link::usable(&p).map_err(|why| println!("SKIPPED: {why}")).ok()
}

#[test]
fn a_leftover_iconv_link_whose_loader_is_gone_is_skipped() {
    // No spec clause (test-harness tooling; XTASK-STALE-PRODUCTS): a copy of the iconv test link whose interpreter
    // names a loader that is not there (as a link left after its products were removed, or its role directory
    // moved) is skipped with that loader's path, never run (running it fails with ENOENT).
    let Some(iconv) = iconv_test_link() else { return };
    let mut elf = std::fs::read(&iconv).expect("read the iconv link");
    let interp = test_link::loads(&elf).expect("the iconv link is ELF").into_iter().next().expect("the iconv link has an interpreter");
    let name = interp.file_name().expect("interpreter file name").to_string_lossy().into_owned();
    let gone = name.replacen("ld-", "no-", 1);
    assert_ne!(gone, name, "the interpreter {} is an ld-* loader", interp.display());
    let at = elf.windows(name.len() + 1).position(|w| w[..name.len()] == *name.as_bytes() && w[name.len()] == 0).expect("the interpreter path in the file");
    elf[at..at + gone.len()].copy_from_slice(gone.as_bytes());
    let gone_path = interp.with_file_name(&gone);
    assert!(!gone_path.exists(), "{} exists", gone_path.display());
    let dir = std::env::temp_dir().join(format!("xtask-leftover-link-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let stale = dir.join("iconv");
    std::fs::write(&stale, &elf).expect("write the leftover link");
    std::fs::set_permissions(&stale, std::fs::metadata(&iconv).expect("stat").permissions()).expect("chmod");
    let r = test_link::usable(&stale);
    if let Ok(p) = &r {
        // What a test that took the link as built would do.
        let run = Command::new(p).arg("-l").env_clear().output();
        let _ = std::fs::remove_dir_all(&dir);
        run.expect("run the leftover iconv link");
        panic!("the leftover link ran although its loader {} is gone", gone_path.display());
    }
    let _ = std::fs::remove_dir_all(&dir);
    let why = r.expect_err("checked above");
    println!("SKIPPED (as wanted): {why}");
    assert!(why.contains(&gone_path.display().to_string()) && why.contains("is gone"), "the reason names the missing loader: {why}");
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
fn the_committed_gencat_transcripts_hold() {
    // Trace: s31_12_c_1, s31_12_i_1, s31_12_g_1, s31_12_f_1, s31_1_c_2. libniji's gencat (the test link `cargo
    // xtask so` builds, UTIL-14) against the reference where installed and the record: options, messages, statuses,
    // the -H header and the written catalogs byte for byte.
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root").join("target/niji-test/gencat");
    if !p.is_file() {
        println!("SKIPPED: target/niji-test/gencat is not built (run `cargo xtask so` first)");
        return;
    }
    let (code, text) = check(&["validation/transcripts/gencat"]);
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
    // Each name converts both ways, each step in a fresh process (the names of the charmaps' headers included: a
    // fresh process finds them by its own header scan): one character from UTF-8 to it and back. The character is
    // `A`, or for a set without `A` (Katakana, Cyrillic, OCR and add-on sets) the first character the set itself
    // gives for its bytes 0x21-0x7e and 0xa1-0xfe, decoded with `-c`.
    let convert = |args: &[&str], input: &[u8]| -> std::process::Output {
        use std::io::Write;
        let mut child = Command::new(&iconv).args(args).env_clear().stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().expect("run iconv");
        child.stdin.take().expect("stdin").write_all(input).expect("write the input");
        child.wait_with_output().expect("wait for iconv")
    };
    let bytes: Vec<u8> = (0x21..=0x7e).chain(0xa1..=0xfe).collect();
    for name in names {
        let mut ch = String::from("A");
        if convert(&["-f", "UTF-8", "-t", name], b"A").status.code() != Some(0) {
            let own = convert(&["-c", "-f", name, "-t", "UTF-8"], &bytes);
            let own = String::from_utf8(own.stdout).unwrap_or_default();
            ch = own.chars().next().map(String::from).unwrap_or_else(|| panic!("{name} is listed but decodes none of its bytes"));
        }
        let to = convert(&["-f", "UTF-8", "-t", name], ch.as_bytes());
        assert_eq!(to.status.code(), Some(0), "{name} is listed but UTF-8 to it fails for {ch:?}: {}", String::from_utf8_lossy(&to.stderr));
        let back = convert(&["-f", name, "-t", "UTF-8"], &to.stdout);
        assert_eq!(back.status.code(), Some(0), "{name} is listed but it to UTF-8 fails: {}", String::from_utf8_lossy(&back.stderr));
        assert_eq!(back.stdout, ch.as_bytes(), "{name} is listed but does not round-trip {ch:?}");
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

#[test]
fn the_shipped_localedef_passes_the_transcripts() {
    // Trace: s31_8_i_1, s31_8_f_1, s31_1_c_2. The freestanding program `cargo xtask so` links (D-31.19), run over
    // the committed localedef transcripts (options, messages, statuses, the directory form's failures).
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root");
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), std::path::PathBuf::from);
    let program = target.join("niji/localedef");
    if !program.exists() {
        eprintln!("SKIP: {} not built (run cargo xtask so)", program.display());
        return;
    }
    let niji = format!("localedef={}", program.display());
    let (code, text) = check(&["validation/transcripts/localedef", "--niji", &niji]);
    println!("{text}");
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains(" 0 mismatched, ") && text.contains(" 0 errors "), "{text}");
    for line in text.lines().filter(|l| l.starts_with("SKIPPED")) {
        // Only a host without unprivileged user namespaces may skip, and the line says why.
        assert!(line.contains("needs user and mount namespaces: "), "{line}");
    }
}

#[test]
fn the_shipped_locales_commands_pass_the_transcripts() {
    // Trace: s31_14_c_2. update-locale and validlocale are the shipped localedef run under their names (D-31.36,
    // D-31.37): the committed transcripts (edits of the locale file, --reset, LANGUAGE, options; validlocale's
    // outputs). locale-gen has none: the hosts' reference locale-gen is a different interface from the Debian one
    // libniji follows (its probes: handoff/UTIL-LOCALE-GEN-probe.txt); tests/prog.rs and tools/deb/root-system.sh
    // cover it.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tree root");
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), std::path::PathBuf::from);
    let program = target.join("niji/localedef");
    if !program.exists() {
        eprintln!("SKIP: {} not built (run cargo xtask so)", program.display());
        return;
    }
    let (u, v) = (format!("update-locale={}", program.display()), format!("validlocale={}", program.display()));
    let (code, text) = check(&["validation/transcripts/update-locale", "validation/transcripts/validlocale", "--niji", &u, "--niji", &v]);
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
