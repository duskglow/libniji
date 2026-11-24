//! The compiler drivers compile against libniji's own C headers (plan/headers.md slice 20):
//! `scripts/niji-cc` with the headers `cargo xtask so` staged, and the packaged `tools/deb/niji-gcc`
//! (and `niji-g++`) with the sysroot's `usr/include`. A C++ compile finds the C++ library's headers
//! first and libniji's C headers under them. `NIJI_SYSTEM_HEADERS=1` restores the system's headers,
//! and a program's own `-nostdinc` gets no directory added.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../core/tests/support/skip_note.rs"]
mod skip_note;

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
/// `<locale.h>` comes first and names `__locale_t` on its own (D-X.169: the C++ library's locale support needs it).
const CXX_PROGRAM: &str = "#include <locale.h>\ntypedef __locale_t lp_locale_t;\n\
#include <cstdio>\n#include <cstdlib>\n#include <cstring>\n#include <cmath>\n\
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

/// The scalar functions libmvec.so.1 has vector variants of (spec/abi/versions.tsv), each with its
/// parameter count (`v`, `vv`, or `vvv` for the sincos shape).
fn libmvec_scalars() -> Vec<(String, usize)> {
    let text = std::fs::read_to_string(root().join("spec/abi/versions.tsv")).unwrap();
    let mut out: Vec<(String, usize)> = text
        .lines()
        .filter_map(|l| l.strip_prefix("libmvec.so.1\t_ZGVbN"))
        .filter_map(|l| {
            let name = l.split('\t').next()?.trim_start_matches(|c: char| c.is_ascii_digit());
            let (params, f) = name.split_once('_')?;
            Some((f.to_string(), params.len()))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The `_ZGV` names (sorted, space-separated; empty for none) in the assembly `cc` makes of a loop
/// over `f` in `c[i] = f(a[i]...)` form, compiled `-O3` with `flags`. `omp`: `#pragma omp simd` on the loop.
fn vector_calls(cc: &Path, dir: &Path, f: &str, params: usize, float: bool, omp: bool, flags: &[&str]) -> String {
    let t = if float { "float" } else { "double" };
    let call = match params {
        1 => format!("c[i] = {f}(a[i]);"),
        2 => format!("c[i] = {f}(a[i], b[i]);"),
        _ => format!("{f}(a[i], &b[i], &c[i]);"),
    };
    let pragma = if omp { "#pragma omp simd\n" } else { "" };
    let src = format!("#define _GNU_SOURCE 1\n#include <math.h>\n{t} a[1024], b[1024], c[1024];\nvoid k(void) {{\n  int i;\n{pragma}  for (i = 0; i < 1024; i++) {call}\n}}\n");
    std::fs::write(dir.join("v.c"), src).unwrap();
    let o = run(cc, dir, &[], &[&["-O3", "-S", "-o", "-"], flags, &["v.c"]].concat());
    ok(&o, &format!("{} -O3 {flags:?} on a loop over {f}", cc.display()));
    let asm = String::from_utf8_lossy(&o.stdout);
    let mut names: Vec<&str> = asm.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).filter(|w| w.starts_with("_ZGV")).collect();
    names.sort_unstable();
    names.dedup();
    names.join(" ")
}

#[test]
fn niji_cc_loops_call_the_libmvec_variants_the_reference_calls() {
    // Trace: no spec clause (D-X.170; issue #28: vector math through libmvec.so.1 for programs built on
    // libniji's headers). For every function with libmvec variants: the variants gcc calls in a loop
    // with libniji's math.h are those it calls with the reference's, under -ffast-math (SSE and
    // AVX-512 widths, C and C++), none without it, and the strict-standard simd loop of -std=c89.
    let dir = scratch("simd");
    let niji = root().join("scripts/niji-cc");
    let reference = PathBuf::from("gcc");
    let scalars = libmvec_scalars();
    assert!(scalars.len() >= 50, "spec/abi/versions.tsv lists {} libmvec scalar functions", scalars.len());
    let configs: [(&str, bool, &[&str]); 5] = [
        ("fast", false, &["-ffast-math"]),
        ("fast-avx512", false, &["-ffast-math", "-mavx512f"]),
        ("not-fast", false, &[]),
        ("fast-c++", false, &["-x", "c++", "-ffast-math"]),
        ("c89-omp", true, &["-std=c89", "-ffast-math", "-fopenmp-simd"]),
    ];
    let mut differ = Vec::new();
    let mut reference_calls = 0;
    for (f, params) in &scalars {
        // `sinf` is the float form of `sin`; `erf` is a double function that ends in f.
        let float = f.strip_suffix('f').is_some_and(|d| scalars.iter().any(|(s, _)| s == d));
        for (name, omp, flags) in &configs {
            let want = vector_calls(&reference, &dir, f, *params, float, *omp, flags);
            let got = vector_calls(&niji, &dir, f, *params, float, *omp, flags);
            reference_calls += usize::from(!want.is_empty());
            if got != want {
                differ.push(format!("{f} {name}: libniji [{got}], reference [{want}]"));
            }
        }
    }
    if reference_calls == 0 {
        skip_note::skipped("skipped: the reference's headers make gcc call no vector variant here");
        return;
    }
    assert!(differ.is_empty(), "{} differences:\n{}", differ.len(), differ.join("\n"));
}
