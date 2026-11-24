//! The compiler drivers compile against libniji's own C headers (plan/headers.md slice 20):
//! `scripts/niji-cc` with the headers `cargo xtask so` staged, and the packaged `tools/deb/niji-gcc`
//! (and `niji-g++`) with the sysroot's `usr/include`. A C++ compile finds the C++ library's headers
//! first and libniji's C headers under them. `NIJI_SYSTEM_HEADERS=1` restores the system's headers,
//! and a program's own `-nostdinc` gets no directory added.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn products() -> PathBuf {
    let p = std::env::var_os("NIJI_PRODUCTS").map_or_else(|| root().join("target/niji"), PathBuf::from);
    assert!(
        p.join("libc.a").exists() && p.join("include/features.h").exists() && p.join("uapi").is_dir(),
        "the products or the staged headers are missing: run `cargo xtask so`"
    );
    p
}

/// `__NIJI__` is defined by libniji's headers only: the program fails to compile on any other set.
const C_PROGRAM: &str = "#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <pty.h>\n\
#ifndef __NIJI__\n#error not libniji's headers\n#endif\n\
int main(void) { char b[16]; strcpy(b, \"niji\"); printf(\"%s %zu\\n\", b, strlen(b)); return EXIT_SUCCESS; }\n";

/// The C++ library's headers include libniji's C headers below them (`<cstdlib>` reaches `stdlib.h`).
const CXX_PROGRAM: &str = "#include <cstdio>\n#include <cstdlib>\n#include <cstring>\n#include <cmath>\n\
#include <string>\n#include <vector>\n#include <iostream>\n\
#ifndef __NIJI__\n#error not libniji's headers\n#endif\n\
int main() { std::vector<std::string> v{\"a\"}; std::printf(\"%zu %d\\n\", v.size(), std::abs(-2)); return 0; }\n";

/// Without libniji's headers: compiles only where `__NIJI__` is not defined.
const SYSTEM_PROGRAM: &str = "#include <stdio.h>\n#ifdef __NIJI__\n#error libniji's headers\n#endif\nint main(void) { return 0; }\n";

fn scratch(name: &str) -> PathBuf {
    let dir = root().join("target/test-scratch/niji_cc_headers").join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("p.c"), C_PROGRAM).unwrap();
    std::fs::write(dir.join("p.cc"), CXX_PROGRAM).unwrap();
    std::fs::write(dir.join("s.c"), SYSTEM_PROGRAM).unwrap();
    dir
}

fn run(cc: &Path, dir: &Path, env: &[(&str, &str)], args: &[&str]) -> Output {
    let mut cmd = Command::new(cc);
    cmd.current_dir(dir).env("NIJI_PRODUCTS", products()).env_remove("NIJI_SYSTEM_HEADERS").env_remove("NIJI_CC_EXTRA");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.args(args).output().unwrap()
}

fn ok(o: &Output, what: &str) {
    assert!(o.status.success(), "{what} failed:\n{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn niji_cc_compiles_and_links_c_against_libniji_headers() {
    // Trace: no spec clause (validation driver scripts/niji-cc; plan/headers.md slice 20).
    let dir = scratch("c");
    let cc = root().join("scripts/niji-cc");
    ok(&run(&cc, &dir, &[], &["-O1", "-c", "p.c", "-o", "p.o"]), "niji-cc -c p.c");
    ok(&run(&cc, &dir, &[], &["-O1", "p.c", "-o", "p"]), "niji-cc p.c -o p");
    let out = Command::new(dir.join("p")).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "niji 4\n");
}

#[test]
fn niji_cc_compiles_cxx_with_the_cxx_library_above_libniji_headers() {
    // Trace: no spec clause (plan/headers.md P4: libstdc++'s <cstdlib> and friends over libniji's C headers).
    let dir = scratch("cxx");
    let cc = root().join("scripts/niji-cc");
    ok(&run(&cc, &dir, &[], &["-O1", "-c", "p.cc", "-o", "a.o"]), "niji-cc -c p.cc");
    ok(&run(&cc, &dir, &[], &["-O1", "-x", "c++", "-c", "p.cc", "-o", "b.o"]), "niji-cc -x c++ -c p.cc");
    ok(&run(&cc, &dir, &[("NIJI_REAL_CC", "g++")], &["-O1", "-c", "p.cc", "-o", "c.o"]), "NIJI_REAL_CC=g++ niji-cc -c p.cc");
}

#[test]
fn niji_cc_system_headers_and_nostdinc_add_no_libniji_directory() {
    // Trace: no spec clause (scripts/niji-cc: NIJI_SYSTEM_HEADERS=1 and a program's own -nostdinc).
    let dir = scratch("sys");
    let cc = root().join("scripts/niji-cc");
    ok(&run(&cc, &dir, &[("NIJI_SYSTEM_HEADERS", "1")], &["-c", "s.c", "-o", "s.o"]), "NIJI_SYSTEM_HEADERS=1 niji-cc -c s.c");
    // p.c compiles on libniji's headers only, so a driver that added them under -nostdinc compiles it.
    let o = run(&cc, &dir, &[], &["-nostdinc", "-c", "p.c", "-o", "n.o"]);
    assert!(!o.status.success(), "niji-cc -nostdinc found <stdio.h>: the driver added a header directory");
    let o = run(&cc, &dir, &[], &["-c", "s.c", "-o", "d.o"]);
    assert!(!o.status.success(), "niji-cc -c s.c compiled without __NIJI__: not libniji's headers");
}

/// A sysroot laid out as niji6-dev installs it: `usr/include` holds libniji's headers and the
/// kernel UAPI directories (links to the staged copies here).
fn sysroot(dir: &Path) -> PathBuf {
    let p = products();
    let inc = dir.join("sysroot/usr/include");
    std::fs::create_dir_all(inc.join("x86_64-linux-gnu")).unwrap();
    for tree in ["include", "uapi"] {
        let mut names: Vec<_> = std::fs::read_dir(p.join(tree)).unwrap().map(|e| e.unwrap().file_name()).collect();
        names.sort();
        for n in names {
            std::os::unix::fs::symlink(p.join(tree).join(&n), inc.join(&n)).unwrap();
        }
    }
    dir.join("sysroot")
}

#[test]
fn packaged_niji_gcc_compiles_against_the_sysroot_headers() {
    // Trace: no spec clause (tools/deb/niji-gcc, package niji6-dev; plan/headers.md slice 20).
    let dir = scratch("deb");
    let root_dir = sysroot(&dir);
    let sr = root_dir.to_str().unwrap();
    let gcc = root().join("tools/deb/niji-gcc");
    let gxx = dir.join("niji-g++");
    std::os::unix::fs::symlink(&gcc, &gxx).unwrap();
    ok(&run(&gcc, &dir, &[("NIJI_SYSROOT", sr)], &["-O1", "-c", "p.c", "-o", "p.o"]), "niji-gcc -c p.c");
    ok(&run(&gxx, &dir, &[("NIJI_SYSROOT", sr)], &["-O1", "-c", "p.cc", "-o", "q.o"]), "niji-g++ -c p.cc");
    ok(&run(&gcc, &dir, &[("NIJI_SYSROOT", sr), ("NIJI_SYSTEM_HEADERS", "1")], &["-c", "s.c", "-o", "s.o"]), "NIJI_SYSTEM_HEADERS=1 niji-gcc -c s.c");
    let o = run(&gcc, &dir, &[("NIJI_SYSROOT", sr)], &["-c", "s.c", "-o", "d.o"]);
    assert!(!o.status.success(), "niji-gcc -c s.c compiled without __NIJI__: not libniji's headers");
}
