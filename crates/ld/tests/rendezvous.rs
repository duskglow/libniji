//! The debugger rendezvous under libniji's `ld.so` (phase 14 DL-23, handoff/DL-23.md; plan/ph14/design.md
//! §13; spec s27_4_c_0). One C program reads `_r_debug` through `DT_DEBUG`, through `dlsym` and
//! through its own copy (a `COPY` relocation, checked with `readelf -r`), walks `r_map` and compares it
//! with `dl_iterate_phdr` at start, after `dlopen` and after `dlclose`; it exits with the number of
//! the first failed check. Built as PIE and as `-no-pie`. A gdb batch run (when gdb is installed) stops
//! at a breakpoint in a `dlopen`ed library, which needs `r_brk` called with the list consistent.
//! Needs `cargo xtask so` and a C compiler (else SKIP).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use niji_ld::arch::{Arch, Native};

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

const PROG: &str = r#"
#define _GNU_SOURCE
#include <dlfcn.h>
#include <elf.h>
#include <link.h>
#include <stdio.h>
#include <string.h>
extern ElfW(Dyn) _DYNAMIC[];
static struct { unsigned long addr; const char *name; } ph[64];
static int nph;
static int cb(struct dl_phdr_info *i, size_t s, void *d) {
    (void)s; (void)d;
    if (nph < 64) { ph[nph].addr = i->dlpi_addr; ph[nph].name = i->dlpi_name; nph++; }
    return 0;
}
static struct r_debug *dt(void) {
    for (ElfW(Dyn) *d = _DYNAMIC; d->d_tag != DT_NULL; d++)
        if (d->d_tag == DT_DEBUG) return (struct r_debug *)d->d_un.d_ptr;
    return 0;
}
/* 0, or the first failed check (base + k). */
static int check(const char *when, int base, int want, const char *last, const char *interp) {
    struct r_debug *own = dlsym(RTLD_DEFAULT, "_r_debug");
    void *brk = dlsym(RTLD_DEFAULT, "_dl_debug_state");
    if (!own || !brk) return base + 1;
    if (dt() != own) return base + 2;
    if (own->r_version != 1 || _r_debug.r_version != 1) return base + 3;
    if (own->r_state != RT_CONSISTENT || _r_debug.r_state != RT_CONSISTENT) return base + 4;
    if ((void *)own->r_brk != brk || _r_debug.r_brk != own->r_brk) return base + 5;
    if (own->r_map != _r_debug.r_map || own->r_ldbase != _r_debug.r_ldbase || !own->r_ldbase) return base + 6;
    nph = 0;
    dl_iterate_phdr(cb, 0);
    int k = 0, ldso = 0;
    struct link_map *prev = 0;
    for (struct link_map *m = own->r_map; m; prev = m, m = m->l_next, k++) {
        if (k >= nph || m->l_addr != ph[k].addr || strcmp(m->l_name, ph[k].name)) {
            fprintf(stderr, "%s: entry %d '%s' vs '%s'\n", when, k, m->l_name, k < nph ? ph[k].name : "-");
            return base + 7;
        }
        if (m->l_prev != prev || !m->l_ld) return base + 8;
        if (k > 0 && m->l_name[0] != '/') { fprintf(stderr, "%s: '%s' is not a path\n", when, m->l_name); return base + 9; }
        ldso += m->l_addr == own->r_ldbase && !strcmp(m->l_name, interp);
    }
    if (k != nph || (want >= 0 && k != want)) { fprintf(stderr, "%s: %d maps, %d phdr, want %d\n", when, k, nph, want); return base + 10; }
    if (strcmp(own->r_map->l_name, "") || ldso != 1) return base + 11;
    if (last && strcmp(prev->l_name, last)) return base + 12;
    return 0;
}
int main(int argc, char **argv) {
    if (argc < 3) return 99;
    int r = check("start", 0, -1, 0, argv[2]);
    int n0 = nph;
    if (r) return r;
    void *h = dlopen(argv[1], RTLD_NOW);
    if (!h) return 50;
    struct link_map *lm = 0;
    if (dlinfo(h, RTLD_DI_LINKMAP, &lm) || (void *)lm != h) return 51;
    if ((r = check("dlopen", 20, n0 + 1, argv[1], argv[2]))) return r;
    if (dlopen(0, RTLD_NOW) != (void *)_r_debug.r_map) return 52;
    ((void (*)(void))dlsym(h, "rdv_target"))();
    dlclose(h);
    if ((r = check("dlclose", 40, n0, 0, argv[2]))) return r;
    puts("rendezvous ok");
    return 0;
}
"#;

const LIB: &str = "volatile int rdv_hits;\nvoid rdv_target(void) { rdv_hits++; }\n";

/// Build the library and the program (`pie`: PIE, else `-no-pie`) in a fresh directory: (dir, lib, prog).
fn setup(tag: &str, pie: bool) -> Option<(PathBuf, PathBuf, PathBuf)> {
    if !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no system C compiler to build the corpus");
        return None;
    }
    let lib = target().join("niji");
    let ld = lib.join(Native::INTERP);
    if !ld.exists() || !lib.join("libc.so.6").exists() || !lib.join("niji-crt1.o").exists() {
        eprintln!("SKIP: ld.so, libc.so.6 or niji-crt1.o not built (run cargo xtask so)");
        return None;
    }
    let lib = std::fs::canonicalize(lib).unwrap();
    let ld = lib.join(Native::INTERP);
    let d = target().join(format!("ld-rendezvous-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    let cc = |out: &Path, src: &str, flags: &[&str]| {
        let c = out.with_extension("c");
        std::fs::write(&c, src).unwrap();
        let r = Command::new("cc").args(["-O1", "-g", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(out).output().unwrap();
        assert!(r.status.success(), "{}: {}", out.display(), String::from_utf8_lossy(&r.stderr));
    };
    let so = d.join("librdv.so");
    cc(&so, LIB, &["-nostdlib", "-fPIC", "-shared"]);
    let prog = d.join("prog");
    let (interp, rpath) = (format!("-Wl,--dynamic-linker={}", ld.display()), format!("-Wl,-rpath,{}", lib.display()));
    let crt1 = lib.join("niji-crt1.o");
    let mode: &[&str] = if pie { &["-fPIE", "-pie"] } else { &["-fno-pic", "-no-pie"] };
    let mut flags = vec!["-nostdlib"];
    flags.extend_from_slice(mode);
    flags.extend_from_slice(&[&interp, &rpath, crt1.to_str().unwrap(), "-L", lib.to_str().unwrap(), "-l:libc.so.6", "-l:ld-linux-x86-64.so.2", "-lgcc"]);
    cc(&prog, PROG, &flags);
    Some((d, so, prog))
}

fn run(prog: &Path, so: &Path) -> (i32, String) {
    let ld = std::fs::canonicalize(target().join("niji")).unwrap().join(Native::INTERP);
    let o = Command::new(prog).arg(so).arg(&ld).output().unwrap();
    (o.status.code().unwrap_or(-1), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

// Spec s27_4_c_0: `_r_debug` (via DT_DEBUG and the symbol) lists the objects dl_iterate_phdr lists,
// in its order, at start, after dlopen and after dlclose; r_state RT_CONSISTENT; r_brk _dl_debug_state.
#[test]
fn rendezvous_pie_matches_dl_iterate_phdr() {
    let Some((d, so, prog)) = setup("pie", true) else { return };
    let (code, out) = run(&prog, &so);
    assert_eq!(code, 0, "failed check {code}: {out}");
    let _ = std::fs::remove_dir_all(d);
}

// Spec s27_4_c_0: a -no-pie program's COPY of `_r_debug` reads what ld.so's own does (r_state
// included) at every point, and DT_DEBUG names ld.so's own.
#[test]
fn rendezvous_copy_relocation_is_kept_current() {
    let Some((d, so, prog)) = setup("copy", false) else { return };
    let r = Command::new("readelf").args(["-rW"]).arg(&prog).output();
    match r {
        Ok(r) if r.status.success() => {
            let text = String::from_utf8_lossy(&r.stdout);
            assert!(text.lines().any(|l| l.contains("R_X86_64_COPY") && l.contains("_r_debug")), "no COPY relocation of _r_debug:\n{text}");
        }
        _ => eprintln!("note: no readelf, the COPY relocation is not checked by name"),
    }
    let (code, out) = run(&prog, &so);
    assert_eq!(code, 0, "failed check {code}: {out}");
    let _ = std::fs::remove_dir_all(d);
}

// Spec s27_4_c_0: a debugger that breaks on r_brk sees every object: gdb lists libc by path and stops
// at a pending breakpoint in a library the program dlopens.
#[test]
fn rendezvous_gdb_breaks_in_a_dlopened_library() {
    if !Command::new("gdb").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no gdb");
        return;
    }
    let Some((d, so, prog)) = setup("gdb", true) else { return };
    let ld = std::fs::canonicalize(target().join("niji")).unwrap().join(Native::INTERP);
    let o = Command::new("gdb")
        .args(["-nx", "-batch", "-ex", "set breakpoint pending on", "-ex", "set debuginfod enabled off", "-ex", "break rdv_target", "-ex", "run", "-ex", "info sharedlibrary", "-ex", "kill", "--args"])
        .arg(&prog)
        .arg(&so)
        .arg(&ld)
        .output()
        .unwrap();
    let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if out.contains("ptrace: Operation not permitted") || out.contains("Couldn't get registers") {
        eprintln!("SKIP: ptrace is not permitted here");
        return;
    }
    assert!(out.contains("Breakpoint 1, ") && out.contains("rdv_target"), "gdb did not stop in the dlopened library:\n{out}");
    let libc = target().join("niji").join("libc.so.6");
    let libc = std::fs::canonicalize(&libc).unwrap_or(libc);
    let listed = |p: &Path| out.lines().any(|l| l.contains(p.to_str().unwrap_or("-")));
    assert!(listed(&so) && (listed(&libc) || out.contains("/libc.so.6")), "info sharedlibrary misses an object:\n{out}");
    let _ = std::fs::remove_dir_all(d);
}
