//! `_dl_get_tls_static_info` under libniji's `ld.so` (story LDSO-TLS-STATIC-INFO, GitHub issue #26;
//! decision record of the story; probes P-TLSINFO-1..3 in handoff/LDSO-TLS-STATIC-INFO.md). Sanitizer
//! runtimes find it with `dlsym(RTLD_NEXT, ...)` from a library that names only the C library and
//! call it unchecked. Programs are built with the host compiler and run under libniji's `ld.so` as a
//! program, its directory first on `LD_LIBRARY_PATH`. Needs `cargo xtask so` and a C compiler (else
//! SKIP); the sanitizer test also needs the compiler's `libasan`/`liblsan` (else SKIP).
// ARCH(x86_64): the corpus reads the thread pointer at %fs:0 (variant II, x86-64 psABI §3.4.6).
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use niji_ld::arch::{Arch, Native};

/// A library that names only the C library: its `RTLD_DEFAULT` and `RTLD_NEXT` lookups.
const LOOK: &str = r#"
#define _GNU_SOURCE
#include <dlfcn.h>
void *look(int next) { return dlsym(next ? RTLD_NEXT : RTLD_DEFAULT, "_dl_get_tls_static_info"); }
"#;

/// A library with a 4096-byte static TLS block aligned to 128.
const LIBT: &str = "__thread char lib_tls[4096] __attribute__((aligned(128))) = {2};\nchar *lib_tls_addr(void) { return lib_tls; }\n";

/// Prints `<who> size align` per thread, then `ok` when every check holds, else `bad: <why>`.
const PROG: &str = r#"
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#ifdef EXE_TLS
__thread char exe_tls[1000] __attribute__((aligned(64))) = {1};
#endif
#ifdef LIB_TLS
extern char *lib_tls_addr(void);
#endif
extern void *look(int next);
static size_t first_size, first_align;
static void bad(const char *why) { printf("bad: %s\n", why); exit(1); }
static void block(uintptr_t tp, size_t size, uintptr_t at, size_t len) {
    if (at < tp - size || at + len > tp) bad("a static block lies outside [tp - size, tp)");
}
static void report(const char *who) {
    void (*f)(size_t *, size_t *) = (void (*)(size_t *, size_t *))look(0);
    if (!f || (void *)f != look(1)) bad("RTLD_DEFAULT and RTLD_NEXT disagree or find nothing");
    size_t size = 1, align = 1;
    f(&size, &align);
    uintptr_t tp;
    __asm__ volatile("mov %%fs:0, %0" : "=r"(tp));
    printf("%s %zu %zu\n", who, size, align);
    if (!align || (align & (align - 1)) || tp % align) bad("the alignment is not the thread pointer's");
    if (first_size && (size != first_size || align != first_align)) bad("another thread reports other values");
    first_size = size, first_align = align;
#ifdef EXE_TLS
    block(tp, size, (uintptr_t)exe_tls, 1000);
#endif
#ifdef LIB_TLS
    block(tp, size, (uintptr_t)lib_tls_addr(), 4096);
#endif
    (void)block;
}
static void *thr(void *a) { report("thread"); return a; }
int main(void) {
    report("main");
    pthread_t th;
    if (pthread_create(&th, 0, thr, 0) || pthread_join(th, 0)) bad("pthread");
    printf("ok\n");
    return 0;
}
"#;

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// libniji's products directory and a fresh scratch directory, or `None` (with the reason).
fn setup(tag: &str) -> Option<(PathBuf, PathBuf)> {
    if !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no system C compiler to build the corpus");
        return None;
    }
    let lib = target().join("niji");
    if !lib.join(Native::INTERP).exists() || !lib.join("libc.so.6").exists() {
        eprintln!("SKIP: ld.so or libc.so.6 not built (run cargo xtask so)");
        return None;
    }
    let d = target().join(format!("ld-tls-static-info-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    Some((std::fs::canonicalize(lib).unwrap(), std::fs::canonicalize(d).unwrap()))
}

/// Compile `src` (written next to `out`) with `flags`; the compiler's output on failure.
fn cc(out: &Path, src: &str, flags: &[&str]) -> Result<(), String> {
    let c = out.with_extension("c");
    std::fs::write(&c, src).unwrap();
    let r = Command::new("cc").arg("-O1").arg(&c).args(flags).arg("-o").arg(out).output().unwrap();
    if r.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&r.stderr).into_owned()) }
}

/// Run `prog` under libniji's `ld.so` with `path` as `LD_LIBRARY_PATH`.
fn run(lib: &Path, path: &str, prog: &Path) -> Output {
    Command::new(lib.join(Native::INTERP)).arg(prog).env("LD_LIBRARY_PATH", path).output().unwrap()
}

/// The size and alignment `_dl_get_tls_static_info` reports, the same through `RTLD_DEFAULT` and
/// `RTLD_NEXT` from a library that does not name `ld.so`, in the main thread and a created one; every
/// static block lies in `[tp - size, tp)`, the alignment is the thread pointer's and covers each
/// module's, and the size grows by each initially loaded module's block (probe P-TLSINFO-1: no TLS, a
/// block in the program, a block in a library, both). (no spec clause: a `GLIBC_PRIVATE` interface of
/// `ld.so`, spec/abi/versions.tsv; probes P-TLSINFO-1..3, the story's decision record.)
#[test]
fn tls_static_info_reports_every_threads_static_block() {
    let Some((lib, d)) = setup("layout") else { return };
    let look = d.join("liblook.so");
    let libt = d.join("libtsi.so");
    cc(&look, LOOK, &["-shared", "-fPIC"]).unwrap();
    cc(&libt, LIBT, &["-shared", "-fPIC"]).unwrap();
    let rpath = format!("-Wl,-rpath,{}", d.display());
    let mut sizes = Vec::new();
    for (name, defs, libs) in [("none", &[][..], &[][..]), ("exe", &["-DEXE_TLS"][..], &[][..]), ("lib", &["-DLIB_TLS"][..], &[libt.to_str().unwrap()][..]), ("both", &["-DEXE_TLS", "-DLIB_TLS"][..], &[libt.to_str().unwrap()][..])] {
        let prog = d.join(name);
        let mut flags: Vec<&str> = defs.to_vec();
        flags.extend_from_slice(&["-pthread", &rpath, look.to_str().unwrap()]);
        flags.extend_from_slice(libs);
        cc(&prog, PROG, &flags).unwrap();
        let o = run(&lib, lib.to_str().unwrap(), &prog);
        let out = String::from_utf8_lossy(&o.stdout).into_owned();
        assert!(o.status.success() && out.ends_with("ok\n"), "{name}: {o:?}");
        let v: Vec<u64> = out.lines().next().unwrap().split(' ').skip(1).map(|x| x.parse().unwrap()).collect();
        let want_align = if libs.is_empty() { 64 } else { 128 };
        assert!(v[1] >= want_align, "{name}: alignment {} below a module's {want_align}", v[1]);
        sizes.push(v[0]);
    }
    // The size without TLS: the surplus and the thread block, no more than a page of the latter.
    assert!(sizes[0] > niji_ld::tls::SURPLUS && sizes[0] <= niji_ld::tls::SURPLUS + 4096, "size without TLS {}", sizes[0]);
    // Each module's block adds its bytes, rounded to its alignment at most.
    for (k, (min, max)) in [(1000, 1000 + 64), (4096, 4096 + 128), (5096, 5096 + 64 + 128)].into_iter().enumerate() {
        let grew = sizes[k + 1] - sizes[0];
        assert!((min..max).contains(&grew), "variant {}: the size grew by {grew}, want [{min}, {max})", k + 1);
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// A library whose constructor sees the program's `.preinit_array` `errno` and gives the thread a C++
/// `thread_local` destructor (as a C++ library's static initialisation does).
const LIBW: &str = r#"
#include <errno.h>
#include <stdio.h>
extern void *__dso_handle;
extern int __cxa_thread_atexit_impl(void (*)(void *), void *, void *);
static int seen;
static void dtor(void *p) { printf("thread_local destructor %s\n", (const char *)p); fflush(stdout); }
__attribute__((constructor)) static void ctor(void) { seen = errno; __cxa_thread_atexit_impl(dtor, "ran", &__dso_handle); }
int lib_seen(void) { return seen; }
"#;

/// The program's `.preinit_array` function sets `errno`, a key's value and a `uselocale` choice; `main`
/// prints what it sees, then the library's `thread_local` destructor runs at `exit`.
const PREW: &str = r#"
#define _GNU_SOURCE
#include <errno.h>
#include <locale.h>
#include <pthread.h>
#include <stdio.h>
extern int lib_seen(void);
static pthread_key_t key;
static locale_t loc;
static void pre(int argc, char **argv, char **envp) {
    (void)argc, (void)argv, (void)envp;
    if (pthread_key_create(&key, 0) || pthread_setspecific(key, (void *)0x1234)) return;
    loc = newlocale(LC_ALL_MASK, "C", (locale_t)0);
    if (loc) uselocale(loc);
    errno = 77;
}
__attribute__((section(".preinit_array"), used)) static void (*pre_fn)(int, char **, char **) = pre;
int main(void) {
    int e = errno;
    printf("errno %d lib %d key %p locale %d\n", e, lib_seen(), pthread_getspecific(key), loc && uselocale((locale_t)0) == loc);
    return 0;
}
"#;

/// The initial thread keeps what the program's `.preinit_array` and a library's constructor gave it,
/// which libniji's `ld.so` runs before `__libc_start_main`: `errno` (the library's constructor and
/// `main` see 77), a key's value (a sanitizer runtime's thread record, GitHub issue #26), the
/// `uselocale` choice, and a `thread_local` destructor that runs at `exit` (probe-settled, the story's
/// decision record D-22.20). (spec s1_1_c_5 constructors before `main`; s2_1_c_0 the thread's
/// `errno`; s22_6_c_1 the thread's key values; s7_3_c_4 the thread's locale; s1_2_c_6 its
/// `thread_local` destructors.)
#[test]
fn initial_thread_keeps_words_set_before_main() {
    let Some((lib, d)) = setup("words") else { return };
    let libw = d.join("libw.so");
    cc(&libw, LIBW, &["-shared", "-fPIC"]).unwrap();
    let prog = d.join("prew");
    cc(&prog, PREW, &["-pthread", &format!("-Wl,-rpath,{}", d.display()), libw.to_str().unwrap()]).unwrap();
    let o = run(&lib, lib.to_str().unwrap(), &prog);
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success(), "{o:?}");
    assert_eq!(out, "errno 77 lib 77 key 0x1234 locale 1\nthread_local destructor ran\n", "{o:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// The issue's reproducer: a program built with `-fsanitize=address` reports its heap overflow, and one
/// built with `-fsanitize=leak` its leak, instead of the runtime calling a null lookup result before
/// `main` (GitHub issue #26). ASan's reports are whole: the runtime finds the main thread it recorded
/// from `.preinit_array` (`thread T0`), prints the allocation stack (and the free stack for a use
/// after free) and ends with `SUMMARY` and its exit code 1 (D-22.20). (no spec clause: a
/// `GLIBC_PRIVATE` interface of `ld.so` and the initial thread's words, s22_6_c_1; the story's
/// decision records.)
#[test]
fn tls_static_info_sanitizer_runtimes_start() {
    let Some((lib, d)) = setup("san") else { return };
    let mut ran = 0;
    // ASan: one byte past an 8-byte block; a byte of a freed block. LSan: the only pointer to a
    // block, in a global, cleared. Volatile: at -O1 the store before `free` is otherwise removed as
    // dead.
    let overflow = "#include <stdlib.h>\nint main(void) { volatile char *p = malloc(8); p[8] = 1; free((void *)p); return 0; }\n";
    let freed = "#include <stdlib.h>\nint main(void) { volatile char *p = malloc(8); free((void *)p); return p[1]; }\n";
    let leak = "#include <stdlib.h>\nvoid *volatile g;\nint main(void) { g = malloc(64); g = 0; return 0; }\n";
    let asan_overflow = &["ERROR: AddressSanitizer: heap-buffer-overflow", "WRITE of size 1 at ", " thread T0\n", "allocated by thread T0 here:\n", "SUMMARY: AddressSanitizer: heap-buffer-overflow "][..];
    let asan_freed = &["ERROR: AddressSanitizer: heap-use-after-free", "READ of size 1 at ", " thread T0\n", "freed by thread T0 here:\n", "previously allocated by thread T0 here:\n", "SUMMARY: AddressSanitizer: heap-use-after-free "][..];
    let lsan = &["ERROR: LeakSanitizer: detected memory leaks"][..];
    for (name, flag, rt, src, want) in [("address", "-fsanitize=address", "libasan.so", overflow, asan_overflow), ("freed", "-fsanitize=address", "libasan.so", freed, asan_freed), ("leak", "-fsanitize=leak", "liblsan.so", leak, lsan)] {
        let prog = d.join(name);
        if let Err(e) = cc(&prog, src, &["-g", flag]) {
            eprintln!("SKIP {flag}: the compiler cannot build it: {}", e.lines().next().unwrap_or(""));
            continue;
        }
        // The runtime's directory and libgcc_s's, after libniji's.
        let dir_of = |f: &str| {
            let p = Command::new("cc").arg(format!("-print-file-name={f}")).output().unwrap();
            let p = std::fs::canonicalize(String::from_utf8_lossy(&p.stdout).trim()).unwrap();
            p.parent().unwrap().display().to_string()
        };
        let path = format!("{}:{}:{}", lib.display(), dir_of(rt), dir_of("libgcc_s.so.1"));
        let o = run(&lib, &path, &prog);
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(want.iter().all(|w| err.contains(w)), "{name}: {o:?}");
        assert!(!err.contains("CHECK failed"), "{name}: the runtime stopped on its own check: {o:?}");
        if flag == "-fsanitize=address" {
            // The report ends with its summary (no further error) and the runtime's exit code.
            assert_eq!(o.status.code(), Some(1), "{name}: {o:?}");
        } else {
            assert!(!o.status.success(), "{name}: {o:?}");
        }
        ran += 1;
    }
    if ran == 0 {
        eprintln!("SKIP: no sanitizer runtime in this toolchain");
    }
    let _ = std::fs::remove_dir_all(&d);
}
