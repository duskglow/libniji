//! The DNS message parsers of `libresolv.so.2` against the reference (spec §16.4 s16_4_c_3;
//! RFC 1035 sections 4.1 and 4.1.4, RFC 2181, RFC 3596; resolver(3)): `tests/oracle/ns_parse_diff.c`
//! runs `ns_initparse`, `ns_parserr` (in order, out of order, out of range), `ns_skiprr`,
//! `ns_msg_getflag`, `dn_expand`, `dn_skipname`, `ns_name_unpack`, `ns_name_uncompress`,
//! `ns_name_skip` and `ns_get16`/`ns_get32`/`ns_put16`/`ns_put32` over a fixed corpus of
//! well-formed responses and malformed messages (cut headers, questions and records, RDLENGTH past
//! the end, record counts above what is there, compression loops, pointers out of the message,
//! extended and reserved label types, names around the 255-byte limit). It is built against the
//! host C library with `-lresolv`, against libniji dynamically with `-lresolv` (so every call goes
//! through `libresolv.so.2`, as in a Debian program that needs it) and statically against
//! `libc.a`; all three must print the same text. Needs the products (`cargo xtask so`) and a C
//! compiler; without the compiler the test says so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/timeout.rs"]
mod timeout;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn products() -> PathBuf {
    let lib = root().join("target/niji");
    for p in ["libc.a", "libc.so.6", "ld-linux-x86-64.so.2", "libresolv.so.2", "libresolv.so", "niji-crt1.o"] {
        assert!(lib.join(p).exists(), "{p} is missing: run `cargo xtask so`");
    }
    lib.canonicalize().unwrap()
}

/// How the probe is linked.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Link {
    Reference,
    Dynamic,
    Static,
}

fn build(link: Link, dir: &Path) -> PathBuf {
    let src = root().join("crates/core/tests/oracle/ns_parse_diff.c");
    let out = dir.join(format!("ns_parse_diff-{link:?}").to_lowercase());
    let mut cmd = Command::new("cc");
    cmd.args(["-std=gnu17", "-O1", "-w"]);
    match link {
        Link::Reference => {
            cmd.arg(&src).arg("-lresolv");
        }
        Link::Dynamic => {
            let lib = products();
            cmd.args(["-nostdlib", "-fno-builtin"]).arg(lib.join("niji-crt1.o")).arg(&src).arg(format!("-L{}", lib.display()));
            cmd.arg(format!("-Wl,--dynamic-linker={}", lib.join("ld-linux-x86-64.so.2").display()));
            cmd.arg("-lresolv").arg(lib.join("libc.so.6")).arg("-lgcc");
        }
        Link::Static => {
            let lib = products();
            cmd.args(["-static", "-nostdlib", "-fno-builtin"]).arg(lib.join("niji-crt1.o")).arg(&src).arg(format!("-L{}", lib.display()));
            cmd.arg("-lresolv").arg(lib.join("libc.a")).arg("-lgcc");
        }
    }
    assert!(oracle::build(cmd, &out), "ns_parse_diff.c does not build ({link:?})");
    out
}

/// Run the probe (a 60 s hang guard, scaled by the machine's load); its stdout.
fn run(link: Link, exe: &Path) -> String {
    let mut cmd = Command::new("timeout");
    cmd.arg(timeout::secs(60)).arg(exe);
    if link == Link::Dynamic {
        // libc.so.6 is a pinned soname: a development build is reached through LD_LIBRARY_PATH
        // (D-27.16, ruling #5909), and libresolv.so.2 with it.
        cmd.env("LD_LIBRARY_PATH", products());
    }
    let o = cmd.output().unwrap();
    assert!(o.status.success(), "ns_parse_diff ({link:?}) exits {:?}: {}", o.status, String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}

/// The first line where `got` differs from `want`, with the section heading above it.
fn first_difference(want: &str, got: &str) -> Option<String> {
    let (w, g): (Vec<&str>, Vec<&str>) = (want.lines().collect(), got.lines().collect());
    let mut section = "";
    for i in 0..w.len().max(g.len()) {
        let (a, b) = (w.get(i).copied(), g.get(i).copied());
        if let Some(s) = a.filter(|l| l.starts_with("== ")) {
            section = s;
        }
        if a != b {
            return Some(format!("{section}\n  line {}:\n  reference {}\n  niji      {}", i + 1, a.unwrap_or("<none>"), b.unwrap_or("<none>")));
        }
    }
    None
}

/// Spec: s16_4_c_3. The ns_* parsers and name expanders give the reference's results and errors on
/// every message of the corpus, called through `libresolv.so.2` and through `libc.a`.
#[test]
fn ns_parse_equals_the_reference() {
    if skip_note::missing("cc") {
        skip_note::skipped("no C compiler (cc) to build tests/oracle/ns_parse_diff.c");
        return;
    }
    let dir = root().join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let want = run(Link::Reference, &build(Link::Reference, &dir));
    assert!(want.matches("\n== ").count() >= 30, "the reference probe printed too little:\n{want}");
    for link in [Link::Dynamic, Link::Static] {
        let exe = build(link, &dir);
        if link == Link::Dynamic {
            let d = String::from_utf8(Command::new("readelf").arg("-dW").arg(&exe).output().unwrap().stdout).unwrap();
            assert!(d.contains("[libresolv.so.2]"), "the dynamic probe does not need libresolv.so.2:\n{d}");
        }
        let got = run(link, &exe);
        if let Some(d) = first_difference(&want, &got) {
            panic!("ns_parse_diff ({link:?}) differs from the reference at\n{d}");
        }
    }
}
