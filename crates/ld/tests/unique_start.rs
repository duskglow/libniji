//! `STB_GNU_UNIQUE` definitions at start and after it, under Liella's `ld.so` and `libc.so.6`
//! (DL-LIBSTDCXX; spec/behaviour/dynlink.md "Symbol binding"). Two libraries the program needs each
//! define the same 200 unique objects (more unique names than a fixed start-up table would hold: a
//! C++ runtime has about a hundred), and a third, opened with `dlopen` after start, defines them
//! again. Every reference, from either start-time library, the program or the late library, and
//! `dlsym` through the late library's handle, must reach the first library's objects with its
//! values. The program checks this itself and exits with the number of the first check that failed.
//! Needs `cargo xtask so` and a C compiler (else SKIP).
//!
//! Trace: s27_1_g_0 (global scope order, `STB_GNU_UNIQUE`).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::expect_used, clippy::panic)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use liella_ld::arch::{Arch, Native};
use liella_ld::elf::Elf;

/// Unique names per library: past 64, the size of the start-up table this test replaced.
const N: usize = 200;

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// The scratch directory, `ld.so`, `libc.so.6` without its `DT_SONAME` (the program names it by
/// path) and `liella-crt1.o`, or `None` with the reason printed.
fn setup() -> Option<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    if !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no system C compiler to build the corpus");
        return None;
    }
    let lib = target().join("liella");
    let ld = lib.join(Native::INTERP);
    if !ld.exists() || !lib.join("libc.so.6").exists() || !lib.join("liella-crt1.o").exists() {
        eprintln!("SKIP: ld.so, libc.so.6 or liella-crt1.o not built (run cargo xtask so)");
        return None;
    }
    let d = target().join(format!("ld-unique-start-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    let mut so = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = Elf::parse::<Native>(&so).unwrap();
    let dynph = elf.find(liella_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (dynph.offset as usize, dynph.filesz as usize);
    let at = (off..off + len).step_by(16).find(|&o| so[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    so[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-dl.so");
    std::fs::write(&libc, &so).unwrap();
    Some((d, ld, libc, lib.join("liella-crt1.o")))
}

/// Build `out` from `src` with `flags` in `d`.
fn cc(d: &Path, out: &str, src: &str, flags: &[&str]) -> PathBuf {
    let c = d.join(format!("{out}.c"));
    std::fs::write(&c, src).unwrap();
    let o = d.join(out);
    let r = Command::new("cc").args(["-O1", "-fno-builtin", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(&o).output().unwrap();
    assert!(r.status.success(), "{out}: {}", String::from_utf8_lossy(&r.stderr));
    o
}

/// A library defining unique objects `u0`..`u{N-1}` with values `base + k`, and `{tag}_table()`: the
/// addresses its own references to them bind (data relocations, looked up like any other).
fn lib_src(tag: &str, base: usize) -> String {
    let mut s = String::new();
    for k in 0..N {
        writeln!(s, "__asm__(\".section .data.u{k},\\\"aw\\\"\\n.globl u{k}\\n.type u{k}, @gnu_unique_object\\n.size u{k}, 4\\n.p2align 2\\nu{k}: .long {}\\n.text\\n\");", base + k).unwrap();
        writeln!(s, "extern int u{k};").unwrap();
    }
    write!(s, "int *{tag}_tab[{N}] = {{").unwrap();
    for k in 0..N {
        write!(s, "&u{k},").unwrap();
    }
    writeln!(s, "}};\nint **{tag}_table(void) {{ return {tag}_tab; }}").unwrap();
    // `uw`: unique too, but no relocation names it, so only a lookup can reach it.
    writeln!(s, "__asm__(\".section .data.uw,\\\"aw\\\"\\n.globl uw\\n.type uw, @gnu_unique_object\\n.size uw, 4\\n.p2align 2\\nuw: .long {}\\n.text\\n\");", base + N).unwrap();
    s
}

/// The program: its own references to `u7` and `u{N-1}`, both start-time libraries' tables, then the
/// late library's table and `dlsym` through its handle.
fn prog_src() -> String {
    let last = N - 1;
    format!(
        r#"#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
extern int u7, u{last};
int **ua_table(void);
int **ub_table(void);
static int fail(int n, const char *what) {{ fprintf(stderr, "check %d: %s\n", n, what); return n; }}
int main(int argc, char **argv) {{
    int **a = ua_table(), **b = ub_table();
    for (int k = 0; k < {N}; k++) {{
        if (*a[k] != 1000 + k) return fail(1, "the first library's references reach its own definitions");
        if (b[k] != a[k]) return fail(2, "the second library's references reach the first library's unique objects");
    }}
    if (&u7 != a[7] || &u{last} != a[{last}] || u7 != 1007) return fail(3, "the program's references reach the first library's unique objects");
    char path[4096];
    snprintf(path, sizeof path, "%s/libuc.so", argc > 1 ? argv[1] : ".");
    void *h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!h) return fail(4, dlerror());
    int **(*uc_table)(void) = (int **(*)(void))dlsym(h, "uc_table");
    if (!uc_table) return fail(5, dlerror());
    int **c = uc_table();
    for (int k = 0; k < {N}; k++)
        if (c[k] != a[k]) return fail(6, "a library opened after start binds the start's unique objects");
    if (dlsym(h, "u100") != a[100]) return fail(7, "dlsym through the late handle gives the start's unique object");
    int *w = (int *)dlsym(RTLD_DEFAULT, "uw");
    if (!w || *w != 1000 + {N}) return fail(10, "dlsym in the global scope gives the first library's unique object");
    if (dlsym(h, "uw") != w) return fail(11, "dlsym through the late handle gives the start's unique object no relocation named");
    if (dlclose(h) != 0) return fail(8, dlerror());
    if (*a[{last}] != 1000 + {last}) return fail(9, "the start's unique objects keep their values");
    return 0;
}}
"#
    )
}

/// s27_1_g_0: one definition per unique name for the process, the first in the start's global scope,
/// with more unique names than any fixed table would hold, kept for an object opened later.
#[test]
fn s27_1_g_0_unique_definitions_at_start_and_after() {
    let Some((d, ld, libc, crt1)) = setup() else { return };
    let dir = d.to_str().unwrap();
    let start_lib = |name: &str, src: &str| cc(&d, name, src, &["-nostdlib", "-fPIC", "-shared", "-Wl,--no-as-needed"]);
    let ua = start_lib("libua.so", &lib_src("ua", 1000));
    let ub = start_lib("libub.so", &lib_src("ub", 2000));
    start_lib("libuc.so", &lib_src("uc", 3000));
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let p = cc(
        &d,
        "prog",
        &prog_src(),
        &["-nostdlib", "-fPIE", "-pie", "-Wl,--no-as-needed", &interp, crt1.to_str().unwrap(), ua.to_str().unwrap(), ub.to_str().unwrap(), libc.to_str().unwrap(), ld.to_str().unwrap(), "-lgcc"],
    );
    for (how, mut cmd) in [("kernel-started", Command::new(&p)), ("ld.so PROGRAM", {
        let mut c = Command::new(&ld);
        c.arg(&p);
        c
    })] {
        use std::os::unix::process::ExitStatusExt;
        let o = cmd.arg(dir).output().unwrap();
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!((o.status.code(), o.status.signal()), (Some(0), None), "{how}: unique check {:?} failed; stderr: {err}", o.status.code());
    }
    std::fs::remove_dir_all(&d).ok();
}
