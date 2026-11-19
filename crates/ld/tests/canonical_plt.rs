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
//! Trace: s27_1_g_0 (symbol resolution follows ELF rules), s27_1_c_3 (`dlsym(RTLD_DEFAULT)`,
//! `RTLD_NEXT`, handles), s27_1_c_6 (`dladdr`).
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
/// reaches the library's `f`. The program is linked once with a GNU and once with a SysV hash table
/// only: the loader keeps "the program has canonical entries" with the table it walks, one path per
/// style.
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
    for style in ["gnu", "sysv"] {
        let hash = format!("-Wl,--hash-style={style}");
        let p = cc(
            &k.d,
            &format!("prog-{style}"),
            prog,
            &["-nostdlib", "-fno-pic", "-no-pie", &hash, "-Wl,--no-as-needed", &interp, k.crt1.to_str().unwrap(), libf.to_str().unwrap(), k.libc.to_str().unwrap(), k.ld.to_str().unwrap(), "-lgcc"],
        );
        for (how, o) in run_both(&k, &p, &[k.d.to_str().unwrap()], &[]) {
            assert_ok(how, &format!("non-PIE canonical PLT, {style} hash"), &o);
        }
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

/// s27_1_g_0: the canonical PLT entry of a function the library defines as an IFUNC. The program's
/// `JUMP_SLOT` for it is bound after the other relocations (its resolver runs then), and must still
/// skip the entry and reach the function the resolver picks; the library's own reference binds the
/// entry, which is not the implementation's address. Probe-settled: the reference passes every check,
/// lazily and with `LD_BIND_NOW`.
#[test]
fn s27_1_g_0_canonical_plt_entry_for_ifunc_definition() {
    let Some(k) = setup("ifunc") else { return };
    let libf = cc(
        &k.d,
        "libf.so",
        "int f_calls;\nstatic void f_impl(void) { f_calls++; }\nstatic void *f_resolve(void) { return (void *)f_impl; }\nvoid f(void) __attribute__((ifunc(\"f_resolve\")));\nvoid *f_p(void) { return (void *)f; }\nvoid *f_impl_p(void) { return (void *)f_impl; }\n",
        LIB_FLAGS,
    );
    let prog = r"extern void f(void);
extern void *f_p(void);
extern void *f_impl_p(void);
extern int f_calls;
int main(void) {
    void *canon = (void *)&f;
    if (canon != f_p()) return 1;
    if (canon == f_impl_p()) return 2;
    f();
    ((void (*)(void))canon)();
    return f_calls == 2 ? 0 : 3;
}
";
    let interp = format!("-Wl,--dynamic-linker={}", k.ld.display());
    let p = cc(
        &k.d,
        "prog",
        prog,
        &["-nostdlib", "-fno-pic", "-no-pie", "-Wl,--no-as-needed", &interp, k.crt1.to_str().unwrap(), libf.to_str().unwrap(), k.libc.to_str().unwrap(), k.ld.to_str().unwrap(), "-lgcc"],
    );
    for (how, o) in run_both(&k, &p, &[], &[]) {
        assert_ok(how, "IFUNC canonical PLT", &o);
    }
    std::fs::remove_dir_all(&k.d).ok();
}

/// The corpus of the scoped-lookups test below: a non-PIE
/// program with canonical PLT entries for `f` (libf.so) and `v` (libv.so's default `v@@V1`) and a COPY
/// of libf.so's `data_x`; one observation per line. `canon` is the program's entry, `real` libf.so's own
/// address of `f`.
const SCOPED_PROG: &str = r#"#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
extern void f(void);
extern void *f_p(void);
extern void *f_real(void);
extern int f_calls;
extern int data_x;
extern int *x_p(void);
extern void v(void);
extern void *use_v(void);
extern void *v1_real(void);
extern void *v0_real(void);
extern int v_calls;
static void *canon, *real;
static const char *cls(void *p) { return !p ? "null" : p == canon ? "canon" : p == real ? "real" : "other"; }
static const char *base(const char *s) {
    if (!s) return "(null)";
    const char *b = strrchr(s, '/');
    b = b ? b + 1 : s;
    return *b ? b : "(empty)";
}
static void info(const char *what, void *at) {
    Dl_info di;
    memset(&di, 0, sizeof di);
    int ok = dladdr(at, &di);
    printf("%s %d %s %s %s\n", what, ok != 0, base(di.dli_fname), di.dli_sname ? di.dli_sname : "(null)", cls(di.dli_saddr));
}
static void *fbase(void *at) {
    Dl_info di;
    memset(&di, 0, sizeof di);
    return dladdr(at, &di) ? di.dli_fbase : 0;
}
static void *open_in(const char *dir, const char *name, int mode) {
    char path[4096];
    snprintf(path, sizeof path, "%s/%s", dir, name);
    return dlopen(path, mode);
}
int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : ".";
    canon = (void *)&f;
    real = f_real();
    printf("canon_is_real %d\n", canon == real);
    printf("lib_ref %s\n", cls(f_p()));
    f();
    ((void (*)(void))canon)();
    printf("calls %d\n", f_calls);
    printf("default %s\n", cls(dlsym(RTLD_DEFAULT, "f")));
    printf("next %s\n", cls(dlsym(RTLD_NEXT, "f")));
    void *hf = open_in(dir, "libf.so", RTLD_NOW | RTLD_NOLOAD);
    printf("handle_libf %s\n", hf ? cls(dlsym(hf, "f")) : "noload-failed");
    void *hd = open_in(dir, "libdeep.so", RTLD_NOW | RTLD_DEEPBIND);
    void *(*deep_f)(void) = hd ? (void *(*)(void))dlsym(hd, "deep_f") : 0;
    printf("deepbind_ref %s\n", deep_f ? cls(deep_f()) : "open-failed");
    printf("deepbind_handle %s\n", hd ? cls(dlsym(hd, "f")) : "open-failed");
    void *hl = open_in(dir, "libloc.so", RTLD_NOW | RTLD_LOCAL);
    void *(*loc_f)(void) = hl ? (void *(*)(void))dlsym(hl, "loc_f") : 0;
    printf("local_ref %s\n", loc_f ? cls(loc_f()) : "open-failed");
    printf("local_handle %s\n", hl ? cls(dlsym(hl, "f")) : "open-failed");
    info("dladdr_canon", canon);
    info("dladdr_canon_plus1", (char *)canon + 1);
    info("dladdr_real", real);
    info("dladdr_prog_base", fbase(canon));
    info("dladdr_libdeep_base", fbase((void *)deep_f));
    void *cv = (void *)&v;
    printf("v_canon_is_v1 %d\n", cv == v1_real());
    printf("v_lib_ref %s\n", use_v() == cv ? "canon" : use_v() == v1_real() ? "v1" : "other");
    void *d1 = dlvsym(RTLD_DEFAULT, "v", "V1");
    void *d0 = dlvsym(RTLD_DEFAULT, "v", "V0");
    printf("dlvsym_V1 %s\n", d1 == cv ? "canon" : d1 == v1_real() ? "v1" : d1 ? "other" : "null");
    printf("dlvsym_V0 %s\n", d0 == cv ? "canon" : d0 == v0_real() ? "v0" : d0 ? "other" : "null");
    v();
    ((void (*)(void))cv)();
    printf("v_calls %d\n", v_calls);
    printf("copy %d %d\n", x_p() == &data_x, data_x);
    return 0;
}
"#;

/// What the reference prints for [`SCOPED_PROG`] (probe-settled, lazily and with `LD_BIND_NOW`).
const SCOPED_EXPECTED: &str = "canon_is_real 0
lib_ref canon
calls 2
default canon
next real
handle_libf real
deepbind_ref real
deepbind_handle real
local_ref canon
local_handle null
dladdr_canon 1 prog f canon
dladdr_canon_plus1 1 prog (null) null
dladdr_real 1 libf.so f real
dladdr_prog_base 1 prog (null) null
dladdr_libdeep_base 1 libdeep.so (null) null
v_canon_is_v1 0
v_lib_ref canon
dlvsym_V1 canon
dlvsym_V0 v0
v_calls 2
copy 1 42
";

/// s27_1_g_0, s27_1_c_3, s27_1_c_6: the canonical PLT entry in the lookups that do not start at the
/// program, and next to versions and a COPY. The entry is the address for the global scope only:
/// `dlsym(RTLD_NEXT)` from the program, `dlsym` on the defining library's handle, and a library opened
/// `RTLD_DEEPBIND` (its own scope first: its reference and its handle) give libf.so's own `f`; a library
/// opened `RTLD_LOCAL` still binds the entry, and its handle (no definition of `f`) gives NULL.
/// `dladdr` names `f` at the entry's own address only (the symbol's size is 0), and nothing at the
/// program's or a library's load base (`dli_fbase`, where an import's value 0 would sit); a versioned entry
/// (`v@V1`) is found by `dlvsym(.., "V1")` and by a library's `v@V1` reference while `V0` gives libv.so's
/// old definition; a COPY of the defining library's data is unaffected. Probe-settled: the reference
/// prints [`SCOPED_EXPECTED`].
#[test]
fn s27_1_g_0_canonical_plt_entry_for_scoped_lookups_dladdr_versions_and_copy() {
    let Some(k) = setup("scoped") else { return };
    let libf = cc(
        &k.d,
        "libf.so",
        "int f_calls;\nint data_x = 42;\nvoid f(void) { f_calls++; }\nextern void f_own(void) __attribute__((alias(\"f\"), visibility(\"hidden\")));\nvoid *f_p(void) { return (void *)f; }\nvoid *f_real(void) { return (void *)f_own; }\nint *x_p(void) { return &data_x; }\n",
        LIB_FLAGS,
    );
    let map = k.d.join("libv.map");
    std::fs::write(&map, "V0 { };\nV1 { } V0;\n").unwrap();
    let script = format!("-Wl,--version-script={}", map.display());
    let libv = cc(
        &k.d,
        "libv.so",
        "int v_calls;\nvoid v0_impl(void) { v_calls += 100; }\nvoid v1_impl(void) { v_calls++; }\n__asm__(\".symver v0_impl,v@V0\");\n__asm__(\".symver v1_impl,v@@V1\");\nextern void v0_own(void) __attribute__((alias(\"v0_impl\"), visibility(\"hidden\")));\nextern void v1_own(void) __attribute__((alias(\"v1_impl\"), visibility(\"hidden\")));\nvoid *v0_real(void) { return (void *)v0_own; }\nvoid *v1_real(void) { return (void *)v1_own; }\n",
        &[LIB_FLAGS, &[script.as_str()]].concat(),
    );
    let libuse = cc(&k.d, "libuse.so", "extern void v(void);\nvoid *use_v(void) { return (void *)v; }\n", &[LIB_FLAGS, &[libv.to_str().unwrap()]].concat());
    cc(&k.d, "libdeep.so", "extern void f(void);\nvoid *deep_f(void) { return (void *)f; }\n", &[LIB_FLAGS, &[libf.to_str().unwrap()]].concat());
    cc(&k.d, "libloc.so", "extern void f(void);\nvoid *loc_f(void) { return (void *)f; }\n", LIB_FLAGS);
    let interp = format!("-Wl,--dynamic-linker={}", k.ld.display());
    let p = cc(
        &k.d,
        "prog",
        SCOPED_PROG,
        &[
            "-nostdlib",
            "-fno-pic",
            "-no-pie",
            "-Wl,--no-as-needed",
            &interp,
            k.crt1.to_str().unwrap(),
            libf.to_str().unwrap(),
            libv.to_str().unwrap(),
            libuse.to_str().unwrap(),
            k.libc.to_str().unwrap(),
            k.ld.to_str().unwrap(),
            "-lgcc",
        ],
    );
    for (how, o) in run_both(&k, &p, &[k.d.to_str().unwrap()], &[]) {
        assert_ok(how, "scoped canonical PLT", &o);
        assert_eq!(String::from_utf8_lossy(&o.stdout), SCOPED_EXPECTED, "scoped canonical PLT ({how}): observations differ from the reference's");
    }
    std::fs::remove_dir_all(&k.d).ok();
}
