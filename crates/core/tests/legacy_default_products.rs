//! `bindresvport` and `__libc_sa_len` at their default versions (`@@GLIBC_2.2.5`; spec §15.8
//! s15_8_c_2 for `bindresvport`, no spec clause for `__libc_sa_len`, which the spec only lists):
//! `tests/oracle/legacy_default_diff.c` calls both by name, with no compat alias or `-D` rename.
//! It is built against the host C library, against libniji's `libc.so.6` and statically against
//! `libc.a`; all three must print the same text, and the dynamic program must bind both names at
//! `GLIBC_2.2.5`. Needs the products (`cargo xtask so`) and a C compiler; without the compiler
//! the test says so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

#[path = "support/oracle.rs"]
mod oracle;
use oracle::root;
#[path = "support/timeout.rs"]
mod timeout;
use std::path::{Path, PathBuf};
use std::process::Command;

fn products() -> PathBuf {
    let lib = root().join("target/niji");
    for p in ["libc.a", "libc.so.6", "ld-linux-x86-64.so.2", "niji-crt1.o"] {
        assert!(lib.join(p).exists(), "{p} is missing: run `cargo xtask so`");
    }
    lib.canonicalize().unwrap()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Link {
    Reference,
    Dynamic,
    Static,
}

fn build(link: Link, dir: &Path) -> PathBuf {
    let src = root().join("crates/core/tests/oracle/legacy_default_diff.c");
    let out = dir.join(format!("legacy_default_diff-{link:?}").to_lowercase());
    let mut cmd = Command::new("cc");
    cmd.args(["-std=gnu17", "-O1", "-w"]);
    match link {
        Link::Reference => {
            cmd.arg(&src);
        }
        Link::Dynamic => {
            let lib = products();
            cmd.args(["-nostdlib", "-fno-builtin"]).arg(lib.join("niji-crt1.o")).arg(&src);
            cmd.arg(format!("-Wl,--dynamic-linker={}", lib.join("ld-linux-x86-64.so.2").display()));
            cmd.arg(lib.join("libc.so.6")).arg("-lgcc");
        }
        Link::Static => {
            let lib = products();
            cmd.args(["-static", "-nostdlib", "-fno-builtin"]).arg(lib.join("niji-crt1.o")).arg(&src);
            cmd.arg(lib.join("libc.a")).arg("-lgcc");
        }
    }
    assert!(oracle::build(cmd, &out), "legacy_default_diff.c does not link ({link:?})");
    out
}

/// Run the program (a 60 s hang guard, scaled by the machine's load); its stdout.
fn run(link: Link, exe: &Path) -> String {
    let mut cmd = Command::new("timeout");
    cmd.arg(timeout::secs(60));
    if link == Link::Dynamic {
        // libc.so.6 is a pinned soname: a development build is reached through LD_LIBRARY_PATH
        // (D-27.16), set by `env` for the program alone.
        cmd.arg("env").arg(format!("LD_LIBRARY_PATH={}", products().display()));
    }
    cmd.arg(exe);
    let o = cmd.output().unwrap();
    assert!(o.status.success(), "legacy_default_diff ({link:?}) exits {:?}: {}", o.status, String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}

/// Spec: s15_8_c_2 (`bindresvport`); `__libc_sa_len`: no spec clause (listed by name only). Both
/// link by name against `libc.so.6` and `libc.a` and give the reference's results.
#[test]
fn bindresvport_and_libc_sa_len_link_by_name_as_the_reference() {
    if skip_note::missing("cc") {
        skip_note::skipped("no C compiler (cc) to build tests/oracle/legacy_default_diff.c");
        return;
    }
    let dir = root().join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let want = run(Link::Reference, &build(Link::Reference, &dir));
    assert!(want.contains("families known") && want.contains("bindresvport bad descriptor"), "the reference program printed too little:\n{want}");
    for link in [Link::Dynamic, Link::Static] {
        let exe = build(link, &dir);
        if link == Link::Dynamic {
            let d = String::from_utf8(Command::new("readelf").args(["-W", "--dyn-syms"]).arg(&exe).output().unwrap().stdout).unwrap();
            for name in ["bindresvport@GLIBC_2.2.5", "__libc_sa_len@GLIBC_2.2.5"] {
                assert!(d.contains(name), "the dynamic program does not bind {name}:\n{d}");
            }
        }
        let got = run(link, &exe);
        assert_eq!(got, want, "legacy_default_diff ({link:?}) differs from the reference");
    }
}
