//! Canonical PLT entries under libniji's `ld.so` and `libc.so.6` (DL-CANONICAL-PLT, GitHub issue #10;
//! spec/behaviour/dynlink.md "Symbol binding"). When a program takes the address of a function a
//! shared library defines without going through the GOT (non-PIC code, or PC-relative data such as
//! `.long f - .`), the link gives the program an undefined symbol for the function whose value is its
//! PLT entry, and that entry is the function's address in the process (gABI "Symbol Values"): every
//! reference to the address, in any object, at start or in an object `dlopen`ed later, and
//! `dlsym(RTLD_DEFAULT)`, binds the entry; only the entry's own `JUMP_SLOT` binds the definition.
//! The corpus reproduces the issue's programs from their described behaviour: the issue's own
//! (binutils `indirect-extern-access-2a`), `pr19031` (the program's `f@GOTPCREL` against its `.long f
//! - .`) and `pr22842` (a PIE built with `-Wa,-mx86-used-note=yes` whose library compares the canonical
//! address with its own). A definition that is protected is the library's own address in that library;
//! the program's entry then gets a warning, or a refused start when the library is marked
//! `GNU_PROPERTY_1_NEEDED_INDIRECT_EXTERN_ACCESS`. Each program checks itself and exits with the number
//! of the first check that failed. The reference gives the same answers (probe-settled; the dynlink
//! notes). Needs `cargo xtask so` and a C compiler (else SKIP).
//!
//! Trace: s27_1_g_0 (symbol resolution follows ELF rules), s27_1_c_3 (`dlsym(RTLD_DEFAULT)`).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use niji_ld::arch::{Arch, Native};
use niji_ld::elf::Elf;

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// The products a corpus program links against.
struct Kit {
    d: PathBuf,
    ld: PathBuf,
    libc: PathBuf,
    crt1: PathBuf,
}

/// The scratch directory `tag`, `ld.so`, `libc.so.6` without its `DT_SONAME` (the program names it by
/// path) and `niji-crt1.o`, or `None` with the reason printed.
fn setup(tag: &str) -> Option<Kit> {
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
    let d = target().join(format!("ld-canonical-plt-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    let mut so = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = Elf::parse::<Native>(&so).unwrap();
    let dynph = elf.find(niji_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (usize::try_from(dynph.offset).unwrap(), usize::try_from(dynph.filesz).unwrap());
    let at = (off..off + len).step_by(16).find(|&o| so[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    so[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-dl.so");
    std::fs::write(&libc, &so).unwrap();
    Some(Kit { d, ld, libc, crt1: lib.join("niji-crt1.o") })
}

/// Build `out` in `dir` from `src` with `flags`; the compiler's error when it fails.
fn try_cc(dir: &Path, out: &str, src: &str, flags: &[&str]) -> Result<PathBuf, String> {
    let c = dir.join(format!("{out}.c"));
    std::fs::write(&c, src).unwrap();
    let o = dir.join(out);
    let r = Command::new("cc").args(["-O1", "-fno-builtin", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(&o).output().unwrap();
    if r.status.success() { Ok(o) } else { Err(String::from_utf8_lossy(&r.stderr).into_owned()) }
}

fn cc(dir: &Path, out: &str, src: &str, flags: &[&str]) -> PathBuf {
    try_cc(dir, out, src, flags).unwrap_or_else(|e| panic!("{out}: {e}"))
}

const LIB_FLAGS: &[&str] = &["-nostdlib", "-fPIC", "-shared", "-Wl,--no-as-needed"];

/// `f` and its library's own reference to it (`GLOB_DAT`); `f_calls` counts the calls that reach it.
const LIBF: &str = "int f_calls;\nvoid f(void) { f_calls++; }\nvoid *f_p(void) { return (void *)f; }\n";
/// The same with `f` protected: the library's references bind its own `f`.
const LIBF_PROTECTED: &str = "int f_calls;\n__attribute__((visibility(\"protected\"))) void f(void) { f_calls++; }\nvoid *f_p(void) { return (void *)f; }\n";

/// Run `prog` with `args`, kernel-started and as `ld.so PROGRAM`, with `env`; each run's output. The
/// runs go through `timeout` as a hang guard only: a `JUMP_SLOT` bound to its own PLT entry would
/// call itself for ever.
fn run_both(k: &Kit, prog: &Path, args: &[&str], env: &[(&str, &Path)]) -> Vec<(&'static str, Output)> {
    [("kernel-started", vec![prog.as_os_str()]), ("ld.so PROGRAM", vec![k.ld.as_os_str(), prog.as_os_str()])]
        .into_iter()
        .map(|(how, argv)| {
            let mut c = Command::new("timeout");
            c.arg("120").args(argv).args(args);
            for (key, v) in env {
                c.env(key, v);
            }
            (how, c.output().unwrap())
        })
        .collect()
}

fn assert_ok(how: &str, what: &str, o: &Output) {
    use std::os::unix::process::ExitStatusExt;
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!((o.status.code(), o.status.signal()), (Some(0), None), "{what} ({how}): check {:?} failed; stderr: {err}", o.status.code());
}

/// s27_1_g_0, s27_1_c_3: the issue's program and `pr19031`, a non-PIE program built without PIC: its
/// `&f`, its `.long f - .`, its own `f@GOTPCREL`, the library's `return f;`, a library opened later and
/// `dlsym(RTLD_DEFAULT)` give one address, the program's PLT entry, and a call through that entry
/// reaches the library's `f`.
#[test]
fn s27_1_g_0_nonpie_program_canonical_plt_entry_binds_every_address_reference() {
    let Some(k) = setup("nonpie") else { return };
    let libf = cc(&k.d, "libf.so", LIBF, LIB_FLAGS);
    cc(&k.d, "libd.so", "extern void f(void);\nvoid *d_f(void) { return (void *)f; }\n", LIB_FLAGS);
    let prog = r#"#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
extern void f(void);
extern void *f_p(void);
extern int f_calls;
extern const int frel;
__asm__(".section .rodata\n.globl frel\n.p2align 2\nfrel: .long f - .\n.text\n");
static void *gotp(void) { void *p; __asm__("movq f@GOTPCREL(%%rip), %0" : "=r"(p)); return p; }
int main(int argc, char **argv) {
    void *canon = (void *)&f;
    if (canon != f_p()) return 1;
    if ((char *)&frel + frel != (char *)canon) return 2;
    if (gotp() != canon) return 3;
    f();
    ((void (*)(void))canon)();
    if (f_calls != 2) return 4;
    char path[4096];
    snprintf(path, sizeof path, "%s/libd.so", argc > 1 ? argv[1] : ".");
    void *h = dlopen(path, RTLD_NOW);
    if (!h) return 5;
    void *(*d_f)(void) = (void *(*)(void))dlsym(h, "d_f");
    if (!d_f || d_f() != canon) return 6;
    if (dlsym(RTLD_DEFAULT, "f") != canon) return 7;
    return 0;
}
"#;
    let interp = format!("-Wl,--dynamic-linker={}", k.ld.display());
    let p = cc(
        &k.d,
        "prog",
        prog,
        &["-nostdlib", "-fno-pic", "-no-pie", "-Wl,--no-as-needed", &interp, k.crt1.to_str().unwrap(), libf.to_str().unwrap(), k.libc.to_str().unwrap(), k.ld.to_str().unwrap(), "-lgcc"],
    );
    for (how, o) in run_both(&k, &p, &[k.d.to_str().unwrap()], &[]) {
        assert_ok(how, "non-PIE canonical PLT", &o);
    }
    std::fs::remove_dir_all(&k.d).ok();
}

/// s27_1_g_0: `pr22842`, a PIE whose data holds `.long test - .` (a canonical PLT entry in a PIE, at
/// its load bias) and that carries the x86 used-ISA notes: the library's `foo` gets the entry's
/// address and compares it with its own reference to `test`.
#[test]
fn s27_1_g_0_pie_canonical_plt_entry_binds_library_reference() {
    let Some(k) = setup("pie") else { return };
    let libt = cc(&k.d, "libt.so", "int test_calls;\nvoid test(void) { test_calls++; }\nint foo(void (*bar)(void)) { return bar == test; }\n", LIB_FLAGS);
    let prog = r#"extern void test(void);
extern int foo(void (*)(void));
extern int test_calls;
extern int trel;
__asm__(".data\n.globl trel\n.p2align 2\ntrel: .long test - .\n.text\n");
int main(void) {
    void (*canon)(void) = (void (*)(void))((char *)&trel + trel);
    if (!foo(canon)) return 1;
    test();
    canon();
    return test_calls == 2 ? 0 : 2;
}
"#;
    let interp = format!("-Wl,--dynamic-linker={}", k.ld.display());
    let p = cc(
        &k.d,
        "prog",
        prog,
        &["-nostdlib", "-fPIE", "-pie", "-Wa,-mx86-used-note=yes", "-Wl,--no-as-needed", &interp, k.crt1.to_str().unwrap(), libt.to_str().unwrap(), k.libc.to_str().unwrap(), k.ld.to_str().unwrap(), "-lgcc"],
    );
    for (how, o) in run_both(&k, &p, &[], &[]) {
        assert_ok(how, "PIE canonical PLT", &o);
    }
    std::fs::remove_dir_all(&k.d).ok();
}

/// s27_1_g_0: a program that only calls `g` (no address taken) has an undefined `g` with value 0, no
/// canonical PLT entry. Linked with a SysV hash table only, that symbol sits in the program's hash
/// chains (a GNU hash table leaves undefined value-0 symbols out), so the loader meets it while
/// binding the library's `return g;` and `dlsym(RTLD_DEFAULT)`; both must reach the library's own `g`,
/// never an address made from the program's 0.
#[test]
fn s27_1_g_0_sysv_hash_program_call_only_reference_binds_library_definition() {
    let Some(k) = setup("sysv") else { return };
    let libg = cc(
        &k.d,
        "libg.so",
        "int g_calls;\nvoid g(void) { g_calls++; }\nvoid *g_p(void) { return (void *)g; }\n\
         extern void g_own_alias(void) __attribute__((alias(\"g\"), visibility(\"hidden\")));\n\
         void *g_own(void) { return (void *)g_own_alias; }\n",
        LIB_FLAGS,
    );
    let prog = r#"#define _GNU_SOURCE
#include <dlfcn.h>
extern void g(void);
extern void *g_p(void);
extern void *g_own(void);
extern int g_calls;
int main(void) {
    g();
    void *p = g_p();
    if (p != g_own()) return 1;
    if (dlsym(RTLD_DEFAULT, "g") != p) return 2;
    ((void (*)(void))p)();
    return g_calls == 2 ? 0 : 3;
}
"#;
    let interp = format!("-Wl,--dynamic-linker={}", k.ld.display());
    let p = cc(
        &k.d,
        "prog",
        prog,
        &["-nostdlib", "-fPIE", "-pie", "-Wl,--hash-style=sysv", "-Wl,--no-as-needed", &interp, k.crt1.to_str().unwrap(), libg.to_str().unwrap(), k.libc.to_str().unwrap(), k.ld.to_str().unwrap(), "-lgcc"],
    );
    for (how, o) in run_both(&k, &p, &[], &[]) {
        assert_ok(how, "SysV-hash call-only reference", &o);
    }
    std::fs::remove_dir_all(&k.d).ok();
}

/// s27_1_g_0: the program's canonical PLT entry for a function the library (swapped in after the
/// link, which the linker refuses) defines as protected. The library keeps its own address and the
/// start goes on with a warning naming the function; when that library is also marked
/// `GNU_PROPERTY_1_NEEDED_INDIRECT_EXTERN_ACCESS` the start is refused (status 127, a message naming
/// the function). Without such a compiler option the marked half is skipped (printed).
#[test]
fn s27_1_g_0_canonical_plt_entry_for_protected_definition() {
    let Some(k) = setup("protected") else { return };
    let soname = ["-Wl,-soname,libcp.so"];
    let plain = k.d.join("plain");
    let prot = k.d.join("prot");
    let marked = k.d.join("marked");
    for dir in [&plain, &prot, &marked] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let flags = |extra: &[&'static str]| [LIB_FLAGS, &soname[..], extra].concat();
    cc(&plain, "libcp.so", LIBF, &flags(&[]));
    cc(&prot, "libcp.so", LIBF_PROTECTED, &flags(&[]));
    let marked_built = try_cc(&marked, "libcp.so", LIBF_PROTECTED, &flags(&["-mno-direct-extern-access"]));
    let prog = r"extern void f(void);
extern void *f_p(void);
extern int f_calls;
int main(void) {
    void *canon = (void *)&f;
    f();
    ((void (*)(void))canon)();
    if (f_calls != 2) return 1;
    return f_p() != canon ? 0 : 2;
}
";
    let interp = format!("-Wl,--dynamic-linker={}", k.ld.display());
    let lpath = format!("-L{}", plain.display());
    let p = cc(
        &k.d,
        "prog",
        prog,
        &["-nostdlib", "-fno-pic", "-no-pie", "-Wl,--no-as-needed", &interp, k.crt1.to_str().unwrap(), &lpath, "-lcp", k.libc.to_str().unwrap(), k.ld.to_str().unwrap(), "-lgcc"],
    );
    for (how, o) in run_both(&k, &p, &[], &[("LD_LIBRARY_PATH", &prot)]) {
        assert_ok(how, "protected definition", &o);
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(err.contains("warning") && err.contains("protected function `f'"), "protected definition ({how}): no warning naming f; stderr: {err}");
    }
    match marked_built {
        Err(e) => eprintln!("SKIP (marked half): the compiler has no -mno-direct-extern-access: {e}"),
        Ok(_) => {
            for (how, o) in run_both(&k, &p, &[], &[("LD_LIBRARY_PATH", &marked)]) {
                let err = String::from_utf8_lossy(&o.stderr);
                assert_eq!(o.status.code(), Some(127), "marked library ({how}): the start goes on; stderr: {err}");
                assert!(err.contains("indirect extern access: f"), "marked library ({how}): no message naming f; stderr: {err}");
            }
        }
    }
    std::fs::remove_dir_all(&k.d).ok();
}
