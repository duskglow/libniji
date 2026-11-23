//! No fixed limit on the new objects one `dlopen` call loads (DL-DLOPEN-NEW-OBJECTS, audit HL-4;
//! dlopen(3): the named object is loaded and "any of its dependencies ... are also automatically
//! loaded", with no count stated). One `dlopen` of `libroot.so` adds `N` + 1 objects at once, `N`
//! well past every bound the call once had (64 new objects, 1024 dependency links among them, 8-bit
//! indices): each library `k` needs `k+1 ..= k+6`, so the init order is forced (`N` first, the root
//! last); every library has a TLS variable (`N` new TLS modules in one call); library 150 calls an
//! IFUNC (pass 2 of relocation, late in the set); library `N` binds to `t` in `libt.so`, which it does
//! not need, so `libt.so` stays loaded while library `N` does (the binding is recorded for an object
//! far past the 64th). Then `dlclose` (and a thread created after it, which must not set up the
//! unloaded TLS modules), a second load of the same set, and the exit's destructors.
//!
//! Skips (with the reason) when `cc` is absent or the products are not built.
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::expect_used, clippy::panic)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use niji_ld::arch::{Arch, Native};
use niji_ld::elf::Elf;

/// Libraries one `dlopen` adds besides the root: past 64 objects, 1024 links (about `6 N`) and an
/// 8-bit index (more than 256 new objects).
const N: usize = 300;
/// The library that calls an IFUNC of its own.
const IFUNC_LIB: usize = 150;
/// What `froot` returns: every `f<k>` returns `k` from its TLS variable, library `N` adds `t()`
/// (1000) and library [`IFUNC_LIB`] the IFUNC's 5000.
const SUM: usize = N * (N + 1) / 2 + 1000 + 5000;

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// The command that builds `out` from `src` with `flags` in `d`.
fn cc(d: &Path, out: &str, src: &str, flags: &[&str]) -> Command {
    let c = d.join(format!("{out}.c"));
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("cc");
    cmd.args(["-O1", "-fno-builtin", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(d.join(out));
    cmd
}

/// Run every command (32 at a time), each must succeed.
fn build_all(jobs: Vec<(String, Command)>) {
    let mut jobs = jobs.into_iter().peekable();
    while jobs.peek().is_some() {
        let running: Vec<_> = jobs.by_ref().take(32).map(|(name, mut c)| (name, c.stderr(std::process::Stdio::piped()).spawn().unwrap())).collect();
        for (name, job) in running {
            let r = job.wait_with_output().unwrap();
            assert!(r.status.success(), "{name} did not build: {}", String::from_utf8_lossy(&r.stderr));
        }
    }
}

/// A constructor writing `c<tag>` and a destructor writing `d<tag>`.
fn ctor_dtor(tag: &str) -> String {
    let line = |p: &str| format!("write(1, \"{p}{tag}\\n\", {})", tag.len() + 2);
    format!("#include <unistd.h>\n__attribute__((constructor)) static void c(void) {{ {}; }}\n__attribute__((destructor)) static void d(void) {{ {}; }}\n", line("c"), line("d"))
}

/// `libl<k>.so`: `f<k>` returns its TLS variable (`k`), plus `t()` for `N` and the IFUNC for [`IFUNC_LIB`].
fn lib_src(k: usize) -> String {
    let mut s = ctor_dtor(&k.to_string());
    // Exported (so not folded into a constant): each library has a PT_TLS segment, a TLS module.
    writeln!(s, "__thread int tv{k} = {k};\n#define tv tv{k}").unwrap();
    let extra = if k == N {
        s.push_str("int t(void);\n");
        " + t()"
    } else if k == IFUNC_LIB {
        s.push_str("static int fi_impl(void) { return 5000; }\nstatic void *fi_pick(void) { return (void *)fi_impl; }\nint fi(void) __attribute__((ifunc(\"fi_pick\")));\n");
        " + fi()"
    } else {
        ""
    };
    writeln!(s, "int f{k}(void) {{ return tv{extra}; }}").unwrap();
    s
}

/// `libroot.so` (constructor `c0`): `froot` sums every `f<k>`.
fn root_src() -> String {
    let mut s = ctor_dtor("0");
    for k in 1..=N {
        writeln!(s, "int f{k}(void);").unwrap();
    }
    s.push_str("long froot(void) {\n    long sum = 0;\n");
    for k in 1..=N {
        writeln!(s, "    sum += f{k}();").unwrap();
    }
    s.push_str("    return sum;\n}\n");
    s
}

/// The program: `libt.so` (`RTLD_GLOBAL`), then the root's set; closes both, opens both again,
/// and exits. Exit status 1 with `open: <dlerror>` when a `dlopen` fails.
const PROG: &str = r#"#include <dlfcn.h>
#include <pthread.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
static void say(const char *s) { write(1, s, strlen(s)); }
static void *thread(void *a) { say("thread\n"); return a; }
static int load(void **ht, void **h) {
    *ht = dlopen("libt.so", RTLD_NOW | RTLD_GLOBAL);
    *h = *ht ? dlopen("libroot.so", RTLD_NOW) : 0;
    if (!*h) {
        char b[512];
        int n = snprintf(b, sizeof b, "open: %s\n", dlerror());
        write(1, b, n);
        return 0;
    }
    long (*fr)(void) = (long (*)(void))dlsym(*h, "froot");
    char b[64];
    int n = snprintf(b, sizeof b, "sum=%ld\n", fr ? fr() : -1L);
    write(1, b, n);
    return 1;
}
int main(void) {
    void *ht, *h;
    if (!load(&ht, &h)) return 1;
    say("close-t\n");
    dlclose(ht);
    say("close-root\n");
    dlclose(h);
    pthread_t th;
    if (pthread_create(&th, 0, thread, 0) == 0) pthread_join(th, 0);
    say("reopen\n");
    if (!load(&ht, &h)) return 1;
    say("end\n");
    return 0;
}
"#;

/// The corpus in a new scratch directory of its own (`tag`: each test builds its own, as tests run
/// at once): (directory, program), or `None` with the reason printed.
fn corpus(tag: &str) -> Option<(PathBuf, PathBuf)> {
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
    let d = target().join(format!("ld-dlopen-new-objects-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("stub")).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    let stub = d.join("stub");
    // libc named by path (its DT_SONAME tag made inert), as the other loader corpora name it.
    let mut so = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = Elf::parse::<Native>(&so).unwrap();
    let dynph = elf.find(niji_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (dynph.offset as usize, dynph.filesz as usize);
    let at = (off..off + len).step_by(16).find(|&o| so[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    so[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-dl.so");
    std::fs::write(&libc, &so).unwrap();
    let (libc_s, ld_s) = (libc.to_str().unwrap().to_string(), ld.to_str().unwrap().to_string());
    let (stub_l, rpath) = (format!("-L{}", stub.display()), format!("-Wl,-rpath,{}", d.display()));
    let shared = |name: &str| -> Vec<String> { ["-nostdlib", "-fPIC", "-shared", "-Wl,--no-as-needed", &format!("-Wl,-soname,{name}"), &rpath, &stub_l].map(String::from).to_vec() };
    // Stubs first (empty, the right soname), so each library links its dependencies by name
    // whatever order the libraries are built in.
    let empty = "int stub_unused;\n";
    let stubs = (1..=N).map(|k| format!("libl{k}.so")).map(|name| (name.clone(), cc(&stub, &name, empty, &["-nostdlib", "-fPIC", "-shared", &format!("-Wl,-soname,{name}")])));
    build_all(stubs.collect());
    let mut jobs = Vec::new();
    for k in 1..=N {
        let name = format!("libl{k}.so");
        let mut flags = shared(&name);
        flags.extend((k + 1..=(k + 6).min(N)).map(|j| format!("-l:libl{j}.so")));
        flags.extend([libc_s.clone(), ld_s.clone()]);
        let flags: Vec<&str> = flags.iter().map(String::as_str).collect();
        jobs.push((name.clone(), cc(&d, &name, &lib_src(k), &flags)));
    }
    let mut flags = shared("libroot.so");
    flags.extend((1..=N).map(|k| format!("-l:libl{k}.so")));
    flags.push(libc_s.clone());
    let flags: Vec<&str> = flags.iter().map(String::as_str).collect();
    jobs.push(("libroot.so".to_string(), cc(&d, "libroot.so", &root_src(), &flags)));
    let flags = shared("libt.so");
    let mut flags: Vec<&str> = flags.iter().map(String::as_str).collect();
    flags.push(&libc_s);
    jobs.push(("libt.so".to_string(), cc(&d, "libt.so", &format!("{}int t(void) {{ return 1000; }}\n", ctor_dtor("T")), &flags)));
    build_all(jobs);
    let one = std::fs::read(d.join(format!("libl{N}.so"))).unwrap();
    assert!(Elf::parse::<Native>(&one).unwrap().find(niji_ld::elf::pt::TLS).is_some(), "each library has a TLS module (PT_TLS)");
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("niji-crt1.o");
    let flags = ["-nostdlib", "-fPIE", "-pie", "-Wl,--no-as-needed", &interp, crt1.to_str().unwrap(), &rpath, &libc_s, &ld_s, "-lgcc"];
    build_all(vec![("prog".to_string(), cc(&d, "prog", PROG, &flags))]);
    let prog = d.join("prog");
    Some((d, prog))
}

/// Run `cmd` with an empty environment.
fn run(cmd: &mut Command) -> Output {
    cmd.env_clear();
    if let Some(p) = std::env::var_os("PATH") {
        cmd.env("PATH", p);
    }
    cmd.output().unwrap()
}

/// The `<tag><k>` lines of `lines` in order, as `k` (`T` left out).
fn seq(lines: &[&str], tag: char) -> Vec<usize> {
    lines.iter().filter_map(|l| l.strip_prefix(tag)).filter_map(|k| k.parse().ok()).collect()
}

/// The lines of `out` from the line after `from` (the start: `None`) to the line before `to`.
fn between<'a>(out: &'a str, from: Option<&str>, to: &str) -> Vec<&'a str> {
    let lines = out.lines();
    let after: Vec<&str> = match from {
        None => lines.collect(),
        Some(f) => lines.skip_while(|l| *l != f).skip(1).collect(),
    };
    after.into_iter().take_while(|l| *l != to).collect()
}

/// The constructors of one load: `libt.so`'s, then the set's in the only order its links allow.
fn check_load(seg: &[&str], what: &str) {
    assert_eq!(seg.first(), Some(&"cT"), "{what}: libt.so's constructor first: {seg:?}");
    let want: Vec<usize> = (0..=N).rev().collect();
    assert_eq!(seq(seg, 'c'), want, "{what}: every constructor once, each after its dependencies'");
    assert_eq!(seg.last().copied(), Some(format!("sum={SUM}").as_str()), "{what}: every library is relocated and bound (TLS, IFUNC, t)");
}

/// The destructors of the whole set and then `libt.so`'s: the root first, each library before the
/// libraries it needs, `libt.so` after library `N`, which binds to it.
fn check_unload(seg: &[&str], what: &str) {
    assert_eq!(seq(seg, 'd'), (0..=N).collect::<Vec<_>>(), "{what}: every destructor once, each before its dependencies'");
    assert_eq!(seg.iter().filter(|l| **l == "dT").count(), 1, "{what}: libt.so's destructor once: {seg:?}");
    assert_eq!(seg.iter().position(|l| *l == "dT"), seg.iter().position(|l| *l == format!("d{N}")).map(|p| p + 1), "{what}: libt.so after library {N}, which binds to it");
}

/// s27_1_c_1 (`dlopen` loads an object and its dependencies and runs their constructors): one call
/// adds `N` + 1 objects; closing and opening the set again works the same.
#[test]
fn dlopen_new_objects_past_every_old_bound() {
    let Some((d, prog)) = corpus("run") else { return };
    let o = run(&mut Command::new(&prog));
    let out = String::from_utf8_lossy(&o.stdout);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "the program failed: {:?}\n{err}\n{}", o.status, out.lines().filter(|l| l.starts_with("open:")).collect::<Vec<_>>().join("\n"));
    check_load(&between(&out, None, "close-t"), "first load");
    let kept = between(&out, Some("close-t"), "close-root");
    assert!(kept.is_empty(), "closing libt.so unloads nothing while library {N} is bound to it: {kept:?}");
    let closed = between(&out, Some("close-root"), "reopen");
    assert_eq!(closed.last(), Some(&"thread"), "a thread created after the dlclose starts (it sets up no unloaded TLS module): {:?}", closed.last());
    check_unload(&closed[..closed.len() - 1], "dlclose");
    check_load(&between(&out, Some("reopen"), "end"), "second load");
    check_unload(&between(&out, Some("end"), "\u{0}"), "exit");
    std::fs::remove_dir_all(&d).ok();
}

/// no spec clause (the call's tables are allocated, so the only limit is memory): under a sweep of
/// address-space limits, from the first one at which the program runs at all, each run either
/// completes or fails cleanly: `dlopen` returns NULL with a `dlerror` text (status 1) or the start
/// ends with the loader's message (127); never a signal. Every limit refused is swept; the sweep
/// ends once two runs in a row complete (a larger limit only adds room no growth path lacks; on the
/// Zen 1 host the full sweep to 48 MiB ran 84 of its 189 limits above that point and none of them
/// was refused).
#[test]
fn dlopen_new_objects_out_of_memory_is_clean() {
    let Some((d, prog)) = corpus("oom") else { return };
    let (mut ran, mut refused, mut in_row, mut first_ran) = (0, 0, 0, None);
    let mut reasons = std::collections::BTreeSet::new();
    for kb in (1024..=48 * 1024).step_by(256) {
        if in_row == 2 {
            break;
        }
        let o = run(Command::new("sh").arg("-c").arg(format!("ulimit -v {kb}; exec \"$0\"")).arg(&prog));
        let (out, err) = (String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned());
        let started = out.lines().next().is_some() || err.starts_with("ld.so:") || err.contains("error while loading shared libraries");
        if ran + refused == 0 && !o.status.success() && !started {
            continue; // below the kernel's own need: the loader never ran
        }
        if o.status.success() {
            ran += 1;
            in_row += 1;
            first_ran.get_or_insert(kb);
            continue;
        }
        in_row = 0;
        refused += 1;
        let code = o.status.code();
        assert!(code == Some(1) || code == Some(127), "limit {kb} KiB: not a clean failure: {:?}\n{err}", o.status);
        let opens: Vec<&str> = out.lines().filter(|l| l.starts_with("open: ")).collect();
        if code == Some(1) {
            assert_eq!(opens.len(), 1, "limit {kb} KiB: one dlopen failure with its text:\n{out}");
            reasons.insert(opens[0].rsplit(": ").next().unwrap_or_default().to_string());
        } else {
            assert!(err.lines().count() == 1, "limit {kb} KiB: one loader line expected:\n{err}");
        }
    }
    assert!(ran > 0, "the program never ran within the sweep's largest limit");
    eprintln!("dlopen under limits: {refused} refused ({reasons:?}), {ran} ran, the first at {first_ran:?} KiB");
    std::fs::remove_dir_all(&d).ok();
}
