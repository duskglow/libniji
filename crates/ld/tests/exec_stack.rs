//! Objects asking for an executable stack under libniji's `ld.so` (COMPAT-19, R14-13; threat model
//! loader rule 10): refused by default with a message naming the object, the program or the
//! `dlopen` caller and the ways in; accepted under `GLIBC_TUNABLES=niji.compat.execstack=1` for a
//! program, a start-up dependency and a `dlopen`, after which the main stack and the stacks the
//! library made for threads, earlier and later, are executable and a caller-supplied stack is not
//! (the COMPAT19-TUNABLE black-box probe, tools/probe/execstack); ignored in a secure process (a
//! set-group-ID program: no root); and a C nested function taken by address (a trampoline on the
//! stack) runs only when accepted.
//!
//! Skips (with the reason) when `cc` is absent, the products are not built, the compiler is not
//! GCC (smoke test: nested functions), or the user has no supplementary group or neither the
//! corpus directory nor the temporary directory honours the set-group-ID bit (secure test).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use niji_ld::arch::{Arch, Native};
use niji_ld::elf::Elf;

const TUNABLE: (&str, &str) = ("GLIBC_TUNABLES", "niji.compat.execstack=1");

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
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

/// The program (the probe's logic): starts a thread on a library-made stack and one on a
/// caller-supplied stack, `dlopen`s `argv[1]` when given (printing `dlopen ok` or the `dlerror`
/// text), starts a third thread, then prints the `/proc/self/maps` permissions of the main stack,
/// the earlier thread's stack, the caller-supplied stack and the later thread's stack.
const PROG: &str = "#define _GNU_SOURCE
#include <dlfcn.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>
static void perms_of(uintptr_t a, char out[5]) {
    FILE *f = fopen(\"/proc/self/maps\", \"r\");
    char line[512];
    strcpy(out, \"????\");
    while (f && fgets(line, sizeof line, f)) {
        unsigned long lo, hi;
        char p[5];
        if (sscanf(line, \"%lx-%lx %4s\", &lo, &hi, p) == 3 && a >= lo && a < hi) { memcpy(out, p, 5); break; }
    }
    if (f) fclose(f);
}
static volatile int go;
struct slot { volatile uintptr_t at; };
static void *thr(void *arg) {
    volatile char here;
    ((struct slot *)arg)->at = (uintptr_t)&here;
    while (!go) usleep(1000);
    return 0;
}
static pthread_t start(struct slot *s, void *stack, size_t len) {
    pthread_t th;
    pthread_attr_t a;
    pthread_attr_init(&a);
    if (stack) pthread_attr_setstack(&a, stack, len);
    if (pthread_create(&th, &a, thr, s) != 0) { puts(\"pthread_create failed\"); _exit(3); }
    while (!s->at) usleep(1000);
    return th;
}
int main(int argc, char **argv) {
    volatile char here;
    char m[5], a[5], u[5], c[5];
    struct slot sa = {0}, su = {0}, sc = {0};
    size_t len = 1 << 20;
    void *own = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    pthread_t ta = start(&sa, 0, 0), tu = start(&su, own, len), tc;
    if (argc > 1) {
        void *h = dlopen(argv[1], RTLD_NOW);
        if (h) puts(\"dlopen ok\"); else printf(\"dlopen: %s\\n\", dlerror());
    }
    tc = start(&sc, 0, 0);
    perms_of((uintptr_t)&here, m);
    perms_of(sa.at, a);
    perms_of(su.at, u);
    perms_of(sc.at, c);
    printf(\"main %s before %s own-stack %s after %s\\n\", m, a, u, c);
    go = 1;
    pthread_join(ta, 0);
    pthread_join(tu, 0);
    pthread_join(tc, 0);
    return 0;
}
";

/// The stack permissions once an executable stack is accepted (the probe): every stack the library
/// made is executable, the caller-supplied one is not.
const EXEC: &str = "main rwxp before rwxp own-stack rw-p after rwxp\n";
/// The stack permissions when nothing asked (or the request was refused).
const PLAIN: &str = "main rw-p before rw-p own-stack rw-p after rw-p\n";

/// A corpus: its scratch directory, `ld.so`, and the link flags of a program (the last three: the
/// C library, `ld.so` and `-lgcc`).
struct Corpus {
    d: PathBuf,
    ld: PathBuf,
    flags: Vec<String>,
}

/// The corpus in a new scratch directory holding `libx.so` (asks for an executable stack) and libc
/// named by path; `None` with the reason printed.
fn corpus(name: &str) -> Option<Corpus> {
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
    let d = target().join(format!("ld-exec-stack-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    // libc named by path (its DT_SONAME tag made inert), as the other loader corpora name it.
    let mut so = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = Elf::parse::<Native>(&so).unwrap();
    let dynph = elf.find(niji_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (dynph.offset as usize, dynph.filesz as usize);
    let at = (off..off + len).step_by(16).find(|&o| so[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    so[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-dl.so");
    std::fs::write(&libc, &so).unwrap();
    let libc_s = libc.to_str().unwrap().to_owned();
    cc(&d, "libx.so", "int x_fn(void) { return 1; }\n", &["-nostdlib", "-fPIC", "-shared", "-Wl,-z,execstack", "-Wl,-soname,libx.so", &libc_s]);
    let flags = vec![
        "-nostdlib".to_owned(),
        "-fPIE".to_owned(),
        "-pie".to_owned(),
        "-Wl,--no-as-needed".to_owned(),
        format!("-Wl,--dynamic-linker={}", ld.display()),
        lib.join("niji-crt1.o").to_str().unwrap().to_owned(),
        format!("-L{}", d.display()),
        format!("-Wl,-rpath,{}", d.display()),
    ];
    Some(Corpus { d, ld, flags: [flags, vec![libc_s, lib.join(Native::INTERP).to_str().unwrap().to_owned(), "-lgcc".to_owned()]].concat() })
}

impl Corpus {
    /// Build program `out` from `src`, with `extra` link flags before the C library.
    fn prog(&self, out: &str, src: &str, extra: &[&str]) -> PathBuf {
        let (head, tail) = self.flags.split_at(self.flags.len() - 3);
        let flags: Vec<&str> = head.iter().map(String::as_str).chain(extra.iter().copied()).chain(tail.iter().map(String::as_str)).collect();
        cc(&self.d, out, src, &flags)
    }

    fn lib(&self) -> String {
        self.d.join("libx.so").to_str().unwrap().to_owned()
    }
}

/// Run `cmd` with only `env` in its environment.
fn run(mut cmd: Command, env: &[(&str, &str)]) -> Output {
    cmd.env_clear();
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

/// (exit status, standard output, standard error).
fn result(o: &Output) -> (Option<i32>, String, String) {
    (o.status.code(), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned())
}

/// The sentence every refusal ends with: the tunable and the planned allowlist.
const WAYS: &str = "to accept it, start the process with GLIBC_TUNABLES=niji.compat.execstack=1 (COMPAT-19; not honoured in a secure process); a system allowlist (/etc/niji/execstack.d) is planned\n";

/// The refusal of a program asking for an executable stack: status 127, the program named.
fn refused_program(o: &Output, prog: &Path, what: &str) {
    let want = format!("ld.so: {}: the program asks for an executable stack (PT_GNU_STACK with PF_X); refused; {WAYS}", prog.display());
    assert_eq!(result(o), (Some(127), String::new(), want), "{what}");
}

/// no spec clause (COMPAT-19, R14-13). A program linked with `-z execstack`: refused by default
/// (run directly and as `ld.so`'s argument) naming the program and both ways in; with the tunable
/// it runs and every library-made stack is executable, whether the kernel set up the stack for it
/// (run directly) or `ld.so` was the program the kernel started (the loader changes the main stack).
#[test]
fn exec_stack_program() {
    let Some(c) = corpus("program") else { return };
    let p = c.prog("prog-x", PROG, &["-Wl,-z,execstack"]);
    refused_program(&run(Command::new(&p), &[]), &p, "direct, default");
    let mut via = Command::new(&c.ld);
    via.arg(&p);
    refused_program(&run(via, &[]), &p, "through ld.so, default");
    refused_program(&run(Command::new(&p), &[("GLIBC_TUNABLES", "niji.compat.execstack=0")]), &p, "direct, tunable 0");
    assert_eq!(result(&run(Command::new(&p), &[TUNABLE])), (Some(0), EXEC.to_owned(), String::new()), "direct, tunable");
    let mut via = Command::new(&c.ld);
    via.arg(&p);
    assert_eq!(result(&run(via, &[TUNABLE])), (Some(0), EXEC.to_owned(), String::new()), "through ld.so, tunable");
    let _ = std::fs::remove_dir_all(&c.d);
}

/// no spec clause (COMPAT-19, R14-13). A start-up dependency linked with `-z execstack`: refused by
/// default naming the dependency and the program needing it; with the tunable the program runs
/// and every library-made stack is executable (the kernel made the program's own stack
/// non-executable: the loader changed it). A program asking for nothing is unchanged.
#[test]
fn exec_stack_dependency() {
    let Some(c) = corpus("dependency") else { return };
    let p = c.prog("prog-dep", PROG, &["-l:libx.so"]);
    let want = format!("{}: error while loading shared libraries: {}: the object asks for an executable stack (PT_GNU_STACK with PF_X); refused; {WAYS}", p.display(), c.lib());
    assert_eq!(result(&run(Command::new(&p), &[])), (Some(127), String::new(), want), "default");
    assert_eq!(result(&run(Command::new(&p), &[TUNABLE])), (Some(0), EXEC.to_owned(), String::new()), "tunable");
    let plain = c.prog("prog", PROG, &[]);
    assert_eq!(result(&run(Command::new(&plain), &[TUNABLE])), (Some(0), PLAIN.to_owned(), String::new()), "nothing asks, tunable");
    let _ = std::fs::remove_dir_all(&c.d);
}

/// no spec clause (COMPAT-19, R14-13). `dlopen` of a library asking for an executable stack after
/// threads exist: by default it fails with the `dlerror` text naming the object, the caller and
/// the ways in, and no stack changes; with the tunable it opens, and the main stack, the earlier
/// thread's stack and the later thread's stack are executable, the caller-supplied stack is not.
#[test]
fn exec_stack_dlopen() {
    let Some(c) = corpus("dlopen") else { return };
    let p = c.prog("prog", PROG, &[]);
    let x = c.lib();
    let mut cmd = Command::new(&p);
    cmd.arg(&x);
    let want = format!("dlopen: {x}: the object asks for an executable stack (PT_GNU_STACK with PF_X); refused (dlopen called from {}); {WAYS}{PLAIN}", p.display());
    assert_eq!(result(&run(cmd, &[])), (Some(0), want, String::new()), "default");
    let mut cmd = Command::new(&p);
    cmd.arg(&x);
    assert_eq!(result(&run(cmd, &[TUNABLE])), (Some(0), format!("dlopen ok\n{EXEC}"), String::new()), "tunable");
    let _ = std::fs::remove_dir_all(&c.d);
}

/// no spec clause (COMPAT-19, R14-13; the tunable is not honoured in a secure process, as
/// COMPAT-12). A set-group-ID copy of the program asking for an executable stack, and of the
/// program `dlopen`ing such a library, are refused with the tunable set.
#[test]
fn exec_stack_secure_ignores_tunable() {
    let Some(c) = corpus("secure") else { return };
    let id = |a: &str| String::from_utf8(Command::new("id").arg(a).output().unwrap().stdout).unwrap();
    let primary = id("-g").trim().to_owned();
    let Some(group) = id("-G").split_whitespace().find(|g| *g != primary).map(str::to_owned) else {
        let _ = std::fs::remove_dir_all(&c.d);
        return eprintln!("SKIP: the user has no supplementary group (a set-group-ID file needs one without root)");
    };
    let px = c.prog("prog-x", PROG, &["-Wl,-z,execstack"]);
    let pd = c.prog("prog", PROG, &[]);
    // The kernel ignores the bit on a `nosuid` file system and under no_new_privs. The programs
    // name their libraries and interpreter by absolute path, so copies in another directory run the
    // same: try the corpus directory, then one under the temporary directory, and keep the first
    // where a set-group-ID copy of a small program reports AT_SECURE.
    let tmp = std::env::temp_dir().join(format!("niji-ld-exec-stack-secure-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let secure = "#include <stdio.h>\n#include <sys/auxv.h>\nint main(void) { printf(\"%lu\\n\", getauxval(AT_SECURE)); return 0; }\n";
    let probe = c.prog("secure", secure, &[]);
    let mut found = None;
    for dir in [c.d.clone(), tmp.clone()] {
        if std::fs::create_dir_all(&dir).is_err() {
            continue;
        }
        let copies: Vec<PathBuf> = [(&probe, "secure-sgid"), (&px, "prog-x-sgid"), (&pd, "prog-sgid")]
            .into_iter()
            .map(|(from, to)| {
                let p = dir.join(to);
                std::fs::copy(from, &p).unwrap();
                assert!(Command::new("chgrp").arg(&group).arg(&p).status().unwrap().success());
                assert!(Command::new("chmod").arg("g+s").arg(&p).status().unwrap().success());
                p
            })
            .collect();
        if result(&run(Command::new(&copies[0]), &[])) == (Some(0), "1\n".to_owned(), String::new()) {
            found = Some(copies);
            break;
        }
    }
    let Some(copies) = found else {
        let _ = (std::fs::remove_dir_all(&c.d), std::fs::remove_dir_all(&tmp));
        return eprintln!("SKIP: the kernel did not set AT_SECURE for a set-group-ID program in {} or {} (nosuid file system or no_new_privs)", c.d.display(), tmp.display());
    };
    refused_program(&run(Command::new(&copies[1]), &[TUNABLE]), &copies[1], "secure program, tunable");
    let x = c.lib();
    let mut cmd = Command::new(&copies[2]);
    cmd.arg(&x);
    let want = format!("dlopen: {x}: the object asks for an executable stack (PT_GNU_STACK with PF_X); refused (dlopen called from {}); {WAYS}{PLAIN}", copies[2].display());
    assert_eq!(result(&run(cmd, &[TUNABLE])), (Some(0), want, String::new()), "secure dlopen, tunable");
    let _ = (std::fs::remove_dir_all(&c.d), std::fs::remove_dir_all(&tmp));
}

/// no spec clause (COMPAT-19; the gcc execution tests behind the request). A C nested function
/// taken by address calls through a trampoline GCC writes on the stack, and GCC marks the object
/// as asking for an executable stack: refused by default, and with the tunable the call through
/// the trampoline returns, on the main thread and on a library-made thread's stack.
#[test]
fn exec_stack_nested_function_smoke() {
    let gcc = Command::new("cc").arg("-v").output().is_ok_and(|o| String::from_utf8_lossy(&o.stderr).contains("gcc version"));
    if !gcc {
        return eprintln!("SKIP: the system C compiler is not GCC (C nested functions are a GCC extension)");
    }
    let Some(c) = corpus("nested") else { return };
    let src = "#include <pthread.h>
#include <stdio.h>
static int through(int k) {
    int add(int x) { return x + k; }
    int (*volatile fp)(int) = add;
    return fp(40);
}
static void *thr(void *arg) { (void)arg; return (void *)(long)through(3); }
int main(int argc, char **argv) {
    (void)argv;
    pthread_t t;
    void *r = 0;
    if (pthread_create(&t, 0, thr, 0) != 0 || pthread_join(t, &r) != 0) return 3;
    printf(\"main %d thread %ld\\n\", through(argc + 1), (long)r);
    return 0;
}
";
    let p = c.prog("nested", src, &[]);
    refused_program(&run(Command::new(&p), &[]), &p, "default");
    assert_eq!(result(&run(Command::new(&p), &[TUNABLE])), (Some(0), "main 42 thread 43\n".to_owned(), String::new()), "tunable");
    let _ = std::fs::remove_dir_all(&c.d);
}
