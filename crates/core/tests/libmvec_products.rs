//! `libmvec.so.1`, `libmvec.a` and `libmvec.so` (SYS-LIBMVEC, ledger M-0/M-1): every export of
//! spec Appendix C's `libmvec.so.1` rows is defined at its version, and every vector variant,
//! called through the x86-64 vector function ABI, gives in each lane the scalar libm function's
//! result bit for bit (`c/libmvec_lanes.c`; ISA classes the CPU lacks are skipped and named).
//! The same program run against the reference library within 4 ulps checks the program's own
//! register layout (lane order, pointer vectors) black-box.
//! No spec clause names these files: spec Appendix C lists the library and its symbols.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

fn products() -> PathBuf {
    let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/niji");
    for p in ["libc.a", "libc.so.6", "libm.so.6", "libm.a", "libmvec.so.1", "libmvec.a", "libmvec.so", "ld-linux-x86-64.so.2", "niji-crt1.o"] {
        assert!(lib.join(p).exists(), "{p} is missing: run `cargo xtask so`");
    }
    lib.canonicalize().unwrap()
}

fn source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/c/libmvec_lanes.c")
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("niji-libmvec-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn compile(args: &[OsString]) {
    let o = Command::new("cc").args(args).output().unwrap();
    assert!(o.status.success(), "cc {args:?}: {}", String::from_utf8_lossy(&o.stderr));
}

/// Run `exe` with `args`; its stdout, failing on a non-zero exit with the program's lines.
fn run(exe: &Path, args: &[&str], lib: Option<&Path>) -> String {
    let mut c = Command::new(exe);
    c.args(args);
    if let Some(l) = lib {
        c.env("LD_LIBRARY_PATH", l);
    }
    let o = c.output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(o.status.success(), "{}: {out}{}", exe.display(), String::from_utf8_lossy(&o.stderr));
    assert!(out.contains("lanes: checked "), "{out}");
    print!("{out}");
    out
}

const CFLAGS: [&str; 4] = ["-fno-builtin", "-O1", "-Wno-psabi", "-Wno-builtin-declaration-mismatch"];

/// SYS-LIBMVEC (no spec clause): each lane of every variant equals the scalar function, through
/// the shared library (`-lmvec` finds the `libmvec.so` link) and through the static archive.
#[test]
fn libmvec_lanes_equal_the_scalar_functions() {
    let lib = products();
    let dir = scratch("lanes");
    let dynamic = dir.join("lanes-dynamic");
    let mut a: Vec<OsString> = vec!["-nostdlib".into(), "-o".into(), dynamic.clone().into(), lib.join("niji-crt1.o").into(), source().into()];
    a.extend(CFLAGS.iter().map(OsString::from));
    a.extend([format!("-L{}", lib.display()).into(), format!("-Wl,--dynamic-linker={}", lib.join("ld-linux-x86-64.so.2").display()).into(), "-lmvec".into()]);
    a.extend([lib.join("libm.so.6").into(), lib.join("libc.so.6").into(), "-lgcc".into()]);
    compile(&a);
    let out = run(&dynamic, &["exact"], Some(&lib));
    assert!(out.contains(", 0 differ (bit-exact)"), "{out}");
    let fixed = dir.join("lanes-static");
    let mut a: Vec<OsString> = vec!["-static".into(), "-nostdlib".into(), "-o".into(), fixed.clone().into(), lib.join("niji-crt1.o").into(), source().into()];
    a.extend(CFLAGS.iter().map(OsString::from));
    a.extend([lib.join("libmvec.a").into(), lib.join("libm.a").into(), lib.join("libc.a").into(), "-lgcc".into()]);
    compile(&a);
    let s = run(&fixed, &["exact"], None);
    assert_eq!(out, s, "the shared and static forms check the same lanes");
    let _ = std::fs::remove_dir_all(&dir);
}

/// SYS-LIBMVEC (no spec clause): the reference library passes the same program within 4 ulps, so
/// the program's register layout is the ABI's and not only libniji's. Skipped without one.
#[test]
fn libmvec_lane_layout_matches_the_reference() {
    let reference = Path::new("/usr/lib/x86_64-linux-gnu/libmvec.so");
    if !reference.exists() {
        skip_note::skipped("libmvec_lane_layout_matches_the_reference: the host has no reference libmvec.so; the layout is checked against libniji only");
        return;
    }
    let dir = scratch("ref");
    let exe = dir.join("lanes-ref");
    let mut a: Vec<OsString> = vec!["-o".into(), exe.clone().into(), source().into()];
    a.extend(CFLAGS.iter().map(OsString::from));
    a.extend(["-lmvec".into(), "-lm".into()]);
    compile(&a);
    let out = run(&exe, &["ulps", "4"], None);
    assert!(out.contains(", 0 differ (ulps bound)"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// SYS-LIBMVEC (no spec clause): `libmvec.so.1` defines exactly its versions.tsv rows, read back
/// with readelf independently of the build's own check, and `libmvec.a` defines every name.
#[test]
fn libmvec_exports_are_the_tables() {
    let lib = products();
    let table = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/abi/versions.tsv")).unwrap();
    let want: BTreeSet<String> = table.lines().filter_map(|l| match l.split('\t').collect::<Vec<_>>().as_slice() {
        ["libmvec.so.1", sym, ver, "default", ..] => Some(format!("{sym}@@{ver}")),
        _ => None,
    }).collect();
    assert_eq!(want.len(), 216, "versions.tsv: libmvec.so.1 rows");
    let o = Command::new("readelf").args(["--dyn-syms", "-W"]).arg(lib.join("libmvec.so.1")).output().unwrap();
    let text = String::from_utf8_lossy(&o.stdout);
    let got: BTreeSet<String> = text.lines().filter_map(|l| {
        let f: Vec<&str> = l.split_whitespace().collect();
        (f.len() >= 8 && f[6] != "UND" && f[0].ends_with(':') && f[7].starts_with("_ZGV")).then(|| f[7].to_string())
    }).collect();
    assert_eq!(got, want);
    let o = Command::new("nm").args(["--defined-only"]).arg(lib.join("libmvec.a")).output().unwrap();
    let names: BTreeSet<String> = String::from_utf8_lossy(&o.stdout).lines().filter_map(|l| l.split_whitespace().nth(2).map(str::to_string)).collect();
    for w in &want {
        let n = w.split('@').next().unwrap();
        assert!(names.contains(n), "libmvec.a does not define {n}");
    }
}
