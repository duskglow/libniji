//! The debugger rendezvous under libniji's `ld.so` (phase 14 DL-23, handoff/DL-23.md; plan/ph14/design.md
//! §13; spec s27_4_c_0). One C program reads `_r_debug` through `DT_DEBUG`, through `dlsym` and
//! through its own copy (a `COPY` relocation, checked with `readelf -r`), walks `r_map` and compares it
//! with `dl_iterate_phdr` at start, after `dlopen` and after `dlclose`; it exits with the number of
//! the first failed check. Built as PIE and as `-no-pie`. A gdb batch run (when gdb is installed) stops
//! at a breakpoint in a `dlopen`ed library, which needs `r_brk` called with the list consistent.
//! Needs `cargo xtask so` and a C compiler (else SKIP).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects)]

#[path = "../../core/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;
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
/* NOLD: the program does not name ld.so (no link-time reference to `_r_debug`): the start adds
   ld.so's link map last, its l_ld from ld.so's own headers. Nothing on the link line names ld.so
   (libniji's libc.so.6 has no DT_NEEDED on it), so ld.so's symbols are looked up in its handle. */
#ifdef NOLD
static void *ldsym(const char *s) {
    void *ld = dlopen("ld-linux-x86-64.so.2", RTLD_NOW | RTLD_NOLOAD);
    return ld ? dlsym(ld, s) : 0;
}
#define RDBG (*(struct r_debug *)ldsym("_r_debug"))
#define GLOBAL(s) ldsym(s)
#else
#define RDBG _r_debug
#define GLOBAL(s) dlsym(RTLD_DEFAULT, s)
#endif
/* In a library the program names by path, which names its own dependency by path. */
extern int rdv_mid(void);
static struct { unsigned long addr; const char *name; unsigned long dyn; } ph[64];
static int nph;
static int cb(struct dl_phdr_info *i, size_t s, void *d) {
    (void)s; (void)d;
    if (nph >= 64) return 0;
    ph[nph].addr = i->dlpi_addr; ph[nph].name = i->dlpi_name; ph[nph].dyn = 0;
    for (int j = 0; j < i->dlpi_phnum; j++)
        if (i->dlpi_phdr[j].p_type == PT_DYNAMIC) ph[nph].dyn = i->dlpi_addr + i->dlpi_phdr[j].p_vaddr;
    nph++;
    return 0;
}
static struct r_debug *dt(void) {
    for (ElfW(Dyn) *d = _DYNAMIC; d->d_tag != DT_NULL; d++)
        if (d->d_tag == DT_DEBUG) return (struct r_debug *)d->d_un.d_ptr;
    return 0;
}
/* 0, or the first failed check (base + k). */
static int check(const char *when, int base, int want, const char *last, const char *interp, const char *need) {
    /* ld.so's own definition (the program's copy, when it has one, comes first in the global scope). */
    void *ld = dlopen("ld-linux-x86-64.so.2", RTLD_NOW | RTLD_NOLOAD);
    struct r_debug *own = ld ? dlsym(ld, "_r_debug") : 0;
    void *brk = GLOBAL("_dl_debug_state");
    if (!own || !brk || !GLOBAL("_r_debug") || GLOBAL("_r_debug") != (void *)&RDBG) {
        fprintf(stderr, "%s: ld.so handle %p, own %p, brk %p, _r_debug %p\n", when, ld, (void *)own, brk, GLOBAL("_r_debug"));
        return base + 1;
    }
    if (dt() != own) return base + 2;
    if (own->r_version != 1 || RDBG.r_version != 1) return base + 3;
    if (own->r_state != RT_CONSISTENT || RDBG.r_state != RT_CONSISTENT) return base + 4;
    if ((void *)own->r_brk != brk || RDBG.r_brk != own->r_brk) return base + 5;
    if (own->r_map != RDBG.r_map || own->r_ldbase != RDBG.r_ldbase || !own->r_ldbase) return base + 6;
    nph = 0;
    dl_iterate_phdr(cb, 0);
    int k = 0, ldso = 0, needed = 0;
    struct link_map *prev = 0;
    for (struct link_map *m = own->r_map; m; prev = m, m = m->l_next, k++) {
        if (k >= nph || m->l_addr != ph[k].addr || strcmp(m->l_name, ph[k].name)) {
            fprintf(stderr, "%s: entry %d '%s' vs '%s'\n", when, k, m->l_name, k < nph ? ph[k].name : "-");
            return base + 7;
        }
        /* l_ld: the object's dynamic array, where its PT_DYNAMIC puts it (ld.so's included). */
        if (m->l_prev != prev || !m->l_ld || (unsigned long)m->l_ld != ph[k].dyn) {
            fprintf(stderr, "%s: entry %d '%s' l_ld %p vs PT_DYNAMIC %#lx\n", when, k, m->l_name, (void *)m->l_ld, ph[k].dyn);
            return base + 8;
        }
        if (k > 0 && m->l_name[0] != '/' && strcmp(m->l_name, "linux-vdso.so.1")) { fprintf(stderr, "%s: '%s' is not a path\n", when, m->l_name); return base + 9; }
        ldso += m->l_addr == own->r_ldbase && !strcmp(m->l_name, interp);
        /* A dependency named by path in a library's DT_NEEDED: l_name is that path (its string
           outlives the library's file view). */
        needed += !strcmp(m->l_name, need);
    }
    if (needed != 1) { fprintf(stderr, "%s: %d link maps named %s\n", when, needed, need); return base + 13; }
    if (k != nph || (want >= 0 && k != want)) { fprintf(stderr, "%s: %d maps, %d phdr, want %d\n", when, k, nph, want); return base + 10; }
    if (strcmp(own->r_map->l_name, "") || ldso != 1) return base + 11;
    if (last && strcmp(prev->l_name, last)) return base + 12;
    return 0;
}
int main(int argc, char **argv) {
    if (argc < 4) return 99;
    if (rdv_mid() != 1) return 98;
    int r = check("start", 0, -1, 0, argv[2], argv[3]);
    int n0 = nph;
    if (r) return r;
    void *h = dlopen(argv[1], RTLD_NOW);
    if (!h) return 50;
    struct link_map *lm = 0;
    if (dlinfo(h, RTLD_DI_LINKMAP, &lm) || (void *)lm != h) return 51;
    if ((r = check("dlopen", 20, n0 + 1, argv[1], argv[2], argv[3]))) return r;
    if (dlopen(0, RTLD_NOW) != (void *)RDBG.r_map) return 52;
    ((void (*)(void))dlsym(h, "rdv_target"))();
    dlclose(h);
    if ((r = check("dlclose", 40, n0, 0, argv[2], argv[3]))) return r;
    puts("rendezvous ok");
    return 0;
}
"#;

const LIB: &str = "volatile int rdv_hits;\nvoid rdv_target(void) { rdv_hits++; }\n";
const LEAF: &str = "int rdv_leaf(void) { return 1; }\n";
const MID: &str = "extern int rdv_leaf(void);\nint rdv_mid(void) { return rdv_leaf(); }\n";

/// What [`setup`] built in its fresh directory: the `dlopen`ed library, the program, and the
/// start-time library a library names by path (`libmid.so` names `librdv-leaf.so` by its path).
struct Built {
    dir: Scratch,
    so: PathBuf,
    prog: PathBuf,
    leaf: PathBuf,
}

/// Build the libraries and the program (`pie`: PIE, else `-no-pie`; `name_ld`: `ld.so` on its link
/// line, so it refers to `_r_debug` by name, else built with `NOLD`) in a fresh directory.
fn setup(tag: &str, pie: bool, name_ld: bool) -> Option<Built> {
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
    let d = Scratch::dir_in(&std::fs::canonicalize(target()).unwrap(), &format!("ld-rendezvous-{tag}"));
    let cc = |out: &Path, src: &str, flags: &[&str]| {
        let c = out.with_extension("c");
        std::fs::write(&c, src).unwrap();
        let r = Command::new("cc").args(["-O1", "-g", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(out).output().unwrap();
        assert!(r.status.success(), "{}: {}", out.display(), String::from_utf8_lossy(&r.stderr));
    };
    // libc.so.6 without DT_SONAME, linked by its path (as crates/ld/tests/dlclose.rs does): the
    // program names it by path, the start opens it by that path.
    let mut bytes = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = niji_ld::elf::Elf::parse::<Native>(&bytes).unwrap();
    let dynph = elf.find(niji_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (usize::try_from(dynph.offset).unwrap(), usize::try_from(dynph.filesz).unwrap());
    let at = (off..off + len).step_by(16).find(|&o| bytes[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    bytes[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-rdv.so");
    std::fs::write(&libc, &bytes).unwrap();
    let so = d.join("librdv.so");
    cc(&so, LIB, &["-nostdlib", "-fPIC", "-shared"]);
    // No DT_SONAME: each is named by the path it was linked by. The leaf's DT_NEEDED string is in
    // libmid's file, which the start reads through a view it releases.
    let leaf = d.join("librdv-leaf.so");
    cc(&leaf, LEAF, &["-nostdlib", "-fPIC", "-shared"]);
    let mid = d.join("librdv-mid.so");
    cc(&mid, MID, &["-nostdlib", "-fPIC", "-shared", leaf.to_str().unwrap()]);
    let prog = d.join("prog");
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("niji-crt1.o");
    // PIE built with -fPIC reaches `_r_debug` through the GOT (no copy); -no-pie gets a COPY.
    let mode: &[&str] = if pie { &["-fPIC", "-pie"] } else { &["-fno-pic", "-no-pie"] };
    let mut flags = vec!["-nostdlib"];
    flags.extend_from_slice(mode);
    flags.extend_from_slice(&[&interp, crt1.to_str().unwrap(), mid.to_str().unwrap(), libc.to_str().unwrap()]);
    if name_ld {
        flags.push(ld.to_str().unwrap());
    } else {
        flags.push("-DNOLD");
    }
    flags.push("-lgcc");
    cc(&prog, PROG, &flags);
    Some(Built { dir: d, so, prog, leaf })
}

fn run(b: &Built) -> (i32, String) {
    let ld = std::fs::canonicalize(target().join("niji")).unwrap().join(Native::INTERP);
    let o = Command::new(&b.prog).arg(&b.so).arg(&ld).arg(&b.leaf).output().unwrap();
    (o.status.code().unwrap_or(-1), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

/// Whether `prog` has an `R_X86_64_COPY` of `_r_debug` (`readelf -r`); `None` without readelf.
fn copied(prog: &Path) -> Option<bool> {
    match Command::new("readelf").args(["-rW"]).arg(prog).output() {
        Ok(r) if r.status.success() => Some(String::from_utf8_lossy(&r.stdout).lines().any(|l| l.contains("R_X86_64_COPY") && l.contains("_r_debug"))),
        _ => {
            eprintln!("note: no readelf, the COPY relocation is not checked by name");
            None
        }
    }
}

// Spec s27_4_c_0: `_r_debug` (via DT_DEBUG and the symbol) lists the objects dl_iterate_phdr lists,
// in its order, at start, after dlopen and after dlclose; r_state RT_CONSISTENT; r_brk _dl_debug_state.
#[test]
fn rendezvous_pie_matches_dl_iterate_phdr() {
    let Some(b) = setup("pie", true, true) else { return };
    assert_ne!(copied(&b.prog), Some(true), "the PIE program has a COPY of _r_debug: it no longer tests ld.so's own alone");
    let (code, out) = run(&b);
    assert_eq!(code, 0, "failed check {code}: {out}");
}

// Spec s27_4_c_0: a program that does not name ld.so still lists it (last, by PT_INTERP), with its
// l_addr and l_ld where its headers put them, as dl_iterate_phdr reports it.
#[test]
fn rendezvous_lists_ld_so_the_program_does_not_name() {
    let Some(b) = setup("nold", true, false) else { return };
    let (code, out) = run(&b);
    assert_eq!(code, 0, "failed check {code}: {out}");
}

// Spec s27_4_c_0: a -no-pie program's COPY of `_r_debug` reads what ld.so's own does (r_state
// included) at every point, and DT_DEBUG names ld.so's own.
#[test]
fn rendezvous_copy_relocation_is_kept_current() {
    let Some(b) = setup("copy", false, true) else { return };
    assert_ne!(copied(&b.prog), Some(false), "the -no-pie program has no COPY relocation of _r_debug");
    let (code, out) = run(&b);
    assert_eq!(code, 0, "failed check {code}: {out}");
}

// Spec s27_4_c_0: a debugger that breaks on r_brk sees every object: gdb lists libc by path and stops
// at a pending breakpoint in a library the program dlopens.
#[test]
fn rendezvous_gdb_breaks_in_a_dlopened_library() {
    if !Command::new("gdb").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no gdb");
        return;
    }
    let Some(b) = setup("gdb", true, true) else { return };
    let (d, so) = (&b.dir, &b.so);
    let ld = std::fs::canonicalize(target().join("niji")).unwrap().join(Native::INTERP);
    let o = Command::new("gdb")
        .args(["-nx", "-batch", "-ex", "set breakpoint pending on", "-ex", "set debuginfod enabled off", "-ex", "break rdv_target", "-ex", "run", "-ex", "info sharedlibrary", "-ex", "kill", "--args"])
        .arg(&b.prog)
        .arg(so)
        .arg(&ld)
        .arg(&b.leaf)
        .output()
        .unwrap();
    let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if out.contains("ptrace: Operation not permitted") || out.contains("Couldn't get registers") {
        eprintln!("SKIP: ptrace is not permitted here");
        return;
    }
    assert!(out.contains("Breakpoint 1, ") && out.contains("rdv_target"), "gdb did not stop in the dlopened library:\n{out}");
    let listed = |p: &Path| out.lines().any(|l| l.contains(p.to_str().unwrap_or("-")));
    assert!(listed(so) && listed(&d.join("libc-rdv.so")) && listed(&ld), "info sharedlibrary misses an object:\n{out}");
}
