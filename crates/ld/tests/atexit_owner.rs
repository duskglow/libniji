//! The opted-in registrations (DL-ATEXIT-OWNER, D-27.29; owner rulings #4860-#4862) under Liella's
//! `ld.so` and `libc.so.6`: modules linked against the link-time interface stub call
//! `atexit`, `at_quick_exit` and `pthread_atfork` at `LIELLA_OWNER_1`. One C program checks every
//! behaviour itself and exits with the number of the first check that failed (its text on standard
//! error); `alarm` is only a hang guard (a registration that deadlocks under `dlopen`'s lock), never
//! a verdict on speed. Needs `cargo xtask so` and a C compiler (else SKIP).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use liella_ld::arch::{Arch, Native};
use liella_ld::elf::Elf;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn target() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root().join("target"), PathBuf::from)
}

/// Build `out` from the C source `src` with `flags` in `d`.
fn build(d: &Path, out: &str, src: &str, flags: &[&str]) -> PathBuf {
    let c = d.join(format!("{out}.c"));
    std::fs::write(&c, src).unwrap();
    let o = d.join(out);
    let r = Command::new("cc").args(["-O1", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(&o).output().unwrap();
    assert!(r.status.success(), "{out}: {}", String::from_utf8_lossy(&r.stderr));
    o
}

/// The helpers. A (opted in): its constructor registers a handler of the program while `dlopen`
/// holds the loader's lock; `a_register` registers its own handler, one of the program's, one of
/// another module and quick-exit and atfork handlers. B and H only hold handlers. S registers
/// through Liella's `libc_nonshared.a` (the archive's stub: `__cxa_atexit` with S's `__dso_handle`).
const LIB: &str = r#"#include <pthread.h>
#include <stdlib.h>
extern void ev(const char *);
#ifdef KIND_A
extern void main_handler(void);
extern void main_ctor_handler(void);
static void a_own(void) { ev("aX"); }
static void a_quick(void) { ev("aQ"); }
static void a_prep(void) { ev("aP"); }
__attribute__((constructor)) static void ctor(void) { if (atexit(main_ctor_handler) != 0) ev("ctor-failed"); }
__attribute__((destructor)) static void dtor(void) { ev("dA"); }
int a_register(void (*other)(void)) {
    return atexit(a_own) | atexit(main_handler) | atexit(other) | at_quick_exit(a_quick) | pthread_atfork(a_prep, 0, 0);
}
#endif
#ifdef KIND_B
__attribute__((destructor)) static void dtor(void) { ev("dB"); }
void b_handler(void) { ev("bH"); }
#endif
#ifdef KIND_H
__attribute__((destructor)) static void dtor(void) { ev("dH"); }
void h_handler(void) { ev("hH"); }
#endif
#ifdef KIND_S
void *__dso_handle __attribute__((visibility("hidden"))) = &__dso_handle;
int s_register(void (*f)(void)) { return atexit(f); }
#endif
"#;

/// The program: `prog DIR net` checks the unload steps, `prog DIR exit` the exit path.
const PROG: &str = r#"#include <dlfcn.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>
static char got[1024];
static size_t n;
void ev(const char *s) {
    size_t l = strlen(s);
    if (n + l + 2 < sizeof got) { if (n) got[n++] = ' '; memcpy(got + n, s, l); n += l; got[n] = 0; }
    (void)!write(1, s, l);
    (void)!write(1, " ", 1);
}
void main_handler(void) { ev("mainH"); }
void main_ctor_handler(void) { ev("mainC"); }
static char path[4200];
static const char *P(const char *dir, const char *name) { strcpy(path, dir); strcat(path, "/"); strcat(path, name); return path; }
static int step;
static void say(const char *s) { (void)!write(2, s, strlen(s)); }
/* Check the events since the last check, then forget them. */
static void expect(const char *want) {
    step++;
    if (strcmp(got, want) != 0) { say("check failed: got '"); say(got); say("', want '"); say(want); say("'\n"); _exit(step); }
    n = 0;
    got[0] = 0;
}
static void forked(void) {
    pid_t p = fork();
    if (p == 0) _exit(0);
    if (p < 0 || waitpid(p, 0, 0) != p) _exit(93);
}
int main(int argc, char **argv) {
    if (argc < 3) return 90;
    alarm(120);
    const char *dir = argv[1];
    void *a = dlopen(P(dir, "libA.so"), RTLD_NOW);
    if (!a) { say(dlerror()); return 91; }
    void *b = dlopen(P(dir, "libB.so"), RTLD_NOW);
    if (!b) { say(dlerror()); return 91; }
    int (*reg)(void (*)(void)) = (int (*)(void (*)(void)))dlsym(a, "a_register");
    void (*bh)(void) = (void (*)(void))dlsym(b, "b_handler");
    if (!reg || !bh || reg(bh) != 0) return 92;
    expect("");                     /* 1: nothing ran at registration */
    if (strcmp(argv[2], "exit") == 0) {
        forked();
        expect("aP");               /* 2: the opted-in prepare handler runs while A is loaded */
        exit(0);                    /* the opted-in exit handlers run once, most recent first */
    }
    void *s = dlopen(P(dir, "libS.so"), RTLD_NOW);
    void *h = dlopen(P(dir, "libH.so"), RTLD_NOW);
    if (!s || !h) { say(dlerror()); return 94; }
    int (*sreg)(void (*)(void)) = (int (*)(void (*)(void)))dlsym(s, "s_register");
    void (*hh)(void) = (void (*)(void))dlsym(h, "h_handler");
    if (!sreg || !hh || sreg(hh) != 0) return 95;
    if (dlclose(b) != 0) return 96;
    expect("dB bH");                /* 2: the net: A's entry whose handler B holds runs at B's unload */
    if (dlclose(h) != 0) return 96;
    expect("dH");                   /* 3: a stub-registered entry keeps owner-only behaviour (#4862) */
    if (dlclose(a) != 0) return 96;
    expect("dA mainH aX mainC");    /* 4: A's entries (the constructor's too) at A's unload, most recent first */
    forked();
    expect("");                     /* 5: A's prepare handler is gone with A */
    quick_exit(0);                  /* A's quick-exit handler is gone with A; S's entry stays unrun */
}
"#;

/// The scratch directory with the corpus built: `libc.so.6` without `DT_SONAME` (Liella's `ld.so`
/// has no library search yet, #4537), the interface stub built from `csys/owner/stub.S` with that
/// copy's path as its soname (so an opted-in module's version need names the C library the program
/// loads), the helpers and the program. `None` with the reason printed when a tool is missing.
fn corpus(name: &str) -> Option<(PathBuf, PathBuf)> {
    if !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no system C compiler to build the corpus");
        return None;
    }
    let lib = target().join("liella");
    let ld = lib.join(Native::INTERP);
    if !ld.exists() || !lib.join("libc.so.6").exists() || !lib.join("liella-crt1.o").exists() || !lib.join("libc_nonshared.a").exists() {
        eprintln!("SKIP: ld.so, libc.so.6, libc_nonshared.a or liella-crt1.o not built (run cargo xtask so)");
        return None;
    }
    let d = target().join(format!("ld-atexit-owner-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    let mut so = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = Elf::parse::<Native>(&so).unwrap();
    let dynph = elf.find(liella_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (dynph.offset as usize, dynph.filesz as usize);
    let at = (off..off + len).step_by(16).find(|&o| so[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    so[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-owner-dl.so");
    std::fs::write(&libc, &so).unwrap();
    let map = d.join("owner.map");
    std::fs::write(&map, "LIELLA_OWNER_1 {\n  global:\n    at_quick_exit;\n    atexit;\n    pthread_atfork;\n  local:\n    *;\n};\n").unwrap();
    let stub = d.join("libc-owner-stub.so");
    let r = Command::new("cc")
        .args(["-shared", "-nostdlib"])
        .arg(format!("-Wl,-soname,{}", libc.display()))
        .arg(format!("-Wl,--version-script={}", map.display()))
        .arg(root().join("csys/owner/stub.S"))
        .arg("-o")
        .arg(&stub)
        .output()
        .unwrap();
    assert!(r.status.success(), "stub: {}", String::from_utf8_lossy(&r.stderr));
    let (stub, libc_s) = (stub.to_str().unwrap().to_string(), libc.to_str().unwrap().to_string());
    build(&d, "libA.so", LIB, &["-nostdlib", "-fPIC", "-shared", "-DKIND_A", &stub]);
    build(&d, "libB.so", LIB, &["-nostdlib", "-fPIC", "-shared", "-DKIND_B"]);
    build(&d, "libH.so", LIB, &["-nostdlib", "-fPIC", "-shared", "-DKIND_H"]);
    let nonshared = lib.join("libc_nonshared.a");
    build(&d, "libS.so", LIB, &["-nostdlib", "-fPIC", "-shared", "-DKIND_S", nonshared.to_str().unwrap(), &libc_s]);
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("liella-crt1.o");
    let prog = build(&d, "prog", PROG, &["-nostdlib", "-fPIE", "-pie", "-Wl,--export-dynamic", &interp, crt1.to_str().unwrap(), &libc_s, ld.to_str().unwrap(), "-lgcc"]);
    Some((d, prog))
}

/// Run `prog DIR MODE` kernel-started: exit 0 and no signal; its standard output (the events).
fn run(d: &Path, prog: &Path, mode: &str) -> String {
    use std::os::unix::process::ExitStatusExt;
    let o = Command::new(prog).arg(d).arg(mode).output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!((o.status.code(), o.status.signal()), (Some(0), None), "{mode}: check {:?} failed; stderr: {err}", o.status.code());
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// s1_2_c_5, s27_1_c_4 (D-27.29, rulings #4860 and #4862): an opted-in module's registrations are
/// attributed to it at registration (also from its constructor, under `dlopen`'s lock) and run at its
/// `dlclose`, after its destructor and most recent first, while its code is mapped; an opted-in
/// entry whose handler another module holds runs at that module's unload (the net); the module's
/// quick-exit and atfork handlers are never called after the unload (a call would fault); an entry
/// registered through the archive's stub is not run at its handler module's unload.
#[test]
fn s1_2_c_5_opted_handlers_run_before_unmap_during_dlclose() {
    let Some((d, prog)) = corpus("net") else { return };
    let out = run(&d, &prog, "net");
    assert_eq!(out, "dB bH dH dA mainH aX mainC ", "the events, in order");
    std::fs::remove_dir_all(&d).ok();
}

/// s1_2_c_5 (D-27.29): without `dlclose`, `exit` runs every opted-in exit handler once, most recent
/// first, before the destructors of the loaded modules; the opted-in prepare handler runs at `fork`.
#[test]
fn s1_2_c_5_opted_handlers_run_once_during_exit() {
    let Some((d, prog)) = corpus("exit") else { return };
    let out = run(&d, &prog, "exit");
    let words: Vec<&str> = out.split_whitespace().collect();
    assert_eq!(words.get(..5), Some(&["aP", "bH", "mainH", "aX", "mainC"][..]), "the exit handlers, most recent first: {out:?}");
    let mut rest: Vec<&str> = words[5..].to_vec();
    rest.sort_unstable();
    assert_eq!(rest, ["dA", "dB"], "each destructor once, after the handlers: {out:?}");
    std::fs::remove_dir_all(&d).ok();
}
