//! Address lookups when another object lies in a hole of the program's address range (probe R-6 in
//! `spec/behaviour/dynlink.md`, LD-TABLES-LAYOUT), under libniji's `ld.so` and `libc.so.6`. The
//! test program is the probe itself, `tools/probe/dl/holes/probe.c`: it fills the free address
//! space above the program's highest hole so the next mapping lands in that hole, maps an anonymous
//! page at the hole's bottom, `dlopen`s L and checks that `_dl_find_object` (link map, range,
//! `PT_GNU_EH_FRAME`), `dladdr` and `dladdr1` name L for L's code and the program for `main`, and
//! that a handler of the program L registers with `atexit` (attributed to L, the opted-in owner
//! lookup) runs at L's `dlclose`. The program is linked in several layouts (holes of 2 MiB, the
//! default, headers inside the text segment) and run kernel-started and as `ld.so PROGRAM`, so
//! which object owns an address never depends on where the kernel or the loader put the objects.
//! Needs `cargo xtask so` and a C compiler (else SKIP).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use niji_ld::arch::{Arch, Native};
use niji_ld::elf::Elf;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn target() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root().join("target"), PathBuf::from)
}

/// The probe, also the test program.
const PROG: &str = include_str!("../../../tools/probe/dl/holes/probe.c");

/// L: a function and an `atexit` registration made from L's code (so attributed to L).
const LIB: &str = "#include <stdlib.h>\nvoid l_func(void) {}\nint l_register(void (*f)(void)) { return atexit(f); }\n";

/// Build `out` from the C source `src` with `flags` in `d`.
fn build(d: &Path, out: &str, src: &str, flags: &[&str]) -> PathBuf {
    let c = d.join(format!("{out}.c"));
    std::fs::write(&c, src).unwrap();
    let o = d.join(out);
    let r = Command::new("cc").args(["-O1", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(&o).output().unwrap();
    assert!(r.status.success(), "{out}: {}", String::from_utf8_lossy(&r.stderr));
    o
}

/// The scratch directory, the program (linked with `layout` added) and `ld.so`. The program is
/// linked against a copy of `libc.so.6` without `DT_SONAME`, so it names it by its path; the copy is
/// then replaced by the unmodified `libc.so.6` (as `atexit_owner.rs`). L is linked against the
/// link-time interface `niji-owner/libc-owner.so`, so its `atexit` registrations are attributed to
/// it. `None` with the reason printed when a tool is missing.
fn corpus(name: &str, layout: &[&str]) -> Option<(PathBuf, PathBuf, PathBuf)> {
    if !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no system C compiler to build the corpus");
        return None;
    }
    let lib = target().join("niji");
    let ld = lib.join(Native::INTERP);
    let iface = lib.join("niji-owner/libc-owner.so");
    if !ld.exists() || !lib.join("libc.so.6").exists() || !lib.join("niji-crt1.o").exists() || !iface.exists() {
        eprintln!("SKIP: ld.so, libc.so.6, niji-owner/libc-owner.so or niji-crt1.o not built (run cargo xtask so)");
        return None;
    }
    let d = target().join(format!("ld-address-holes-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    let original = std::fs::read(lib.join("libc.so.6")).unwrap();
    let mut so = original.clone();
    let elf = Elf::parse::<Native>(&so).unwrap();
    let dynph = elf.find(niji_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (dynph.offset as usize, dynph.filesz as usize);
    let at = (off..off + len).step_by(16).find(|&o| so[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    so[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-holes-dl.so");
    std::fs::write(&libc, &so).unwrap();
    build(&d, "libL.so", LIB, &["-nostdlib", "-fPIC", "-shared", "-Wl,--eh-frame-hdr", iface.to_str().unwrap()]);
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("niji-crt1.o");
    let mut flags = vec!["-nostdlib", "-fPIE", "-pie", "-Wl,--eh-frame-hdr", &interp, crt1.to_str().unwrap(), libc.to_str().unwrap(), ld.to_str().unwrap(), "-lgcc"];
    flags.extend_from_slice(layout);
    let prog = build(&d, "prog", PROG, &flags);
    std::fs::write(&libc, &original).unwrap();
    Some((d, prog, ld))
}

/// Run the program (as `ld.so PROGRAM` when `via_ld`) with L and `mode`: exit 0, "PASSED"; its
/// standard output.
fn run(d: &Path, prog: &Path, ld: &Path, via_ld: bool, mode: &str) -> String {
    use std::os::unix::process::ExitStatusExt;
    let lib = d.join("libL.so");
    let o = if via_ld { Command::new(ld).arg(prog).arg(&lib).arg(mode).output() } else { Command::new(prog).arg(&lib).arg(mode).output() }.unwrap();
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert_eq!((o.status.code(), o.status.signal()), (Some(0), None), "{mode}: {out}stderr: {}", String::from_utf8_lossy(&o.stderr));
    assert!(out.ends_with("PASSED\n"), "{out}");
    out
}

/// s27_1_c_9, s27_1_c_6, s1_2_c_5 (probe R-6): a program the kernel started, linked with 2 MiB
/// holes between its segments; L lands in its highest hole. `_dl_find_object`, `dladdr` and
/// `dladdr1` name L for L's code and the program (its run of segments) for `main`; an anonymous
/// page in the hole belongs to no object; L's `atexit` handler runs at L's `dlclose`.
#[test]
fn s27_1_c_9_object_in_the_program_hole_is_found_as_itself() {
    let Some((d, prog, ld)) = corpus("hole", &["-Wl,-z,max-page-size=0x200000"]) else { return };
    let out = run(&d, &prog, &ld, false, "hole");
    let anon = out.lines().find(|l| l.starts_with("anon page")).unwrap_or_else(|| panic!("no anonymous page in the hole: {out}"));
    assert!(anon.contains("_dl_find_object rc -1 ") && anon.contains("dladdr rc 0 "), "an address in the hole is no object's: {anon}");
    std::fs::remove_dir_all(&d).ok();
}

/// s27_1_c_9, s27_1_c_6, s1_2_c_5: the same program as `ld.so PROGRAM` (the loader maps the program
/// and keeps its holes reserved), the default layout and the layout with the program headers in
/// the text segment (`-z noseparate-code`): every lookup gives the same answers wherever L lands.
#[test]
fn s27_1_c_9_address_lookups_hold_in_every_layout() {
    let variants: [(&str, &[&str], bool); 3] =
        [("ldso", &["-Wl,-z,max-page-size=0x200000"], true), ("default", &[], false), ("noseparate", &["-Wl,-z,noseparate-code"], false)];
    for (name, layout, via_ld) in variants {
        let Some((d, prog, ld)) = corpus(name, layout) else { return };
        run(&d, &prog, &ld, via_ld, "any");
        std::fs::remove_dir_all(&d).ok();
    }
}
