//! `ld.so [OPTION]... PROGRAM` (LDSO-RUN-PROGRAM, ld.so(8); the reference probed on a Zen 1 host,
//! spec/behaviour/dynlink.md "Program mode options"): the product interpreter runs a position-independent
//! program with a shared-object dependency under each option it implements, refuses the ones it
//! does not, and gives the reference's statuses and messages for a missing, short, directory,
//! static and shared-object file. Skips (with the reason) before `cargo xtask so` or without `cc`.

// ARCH(x86_64): the test programs are x86-64 assembly and system call numbers; the product is x86_64's.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]

#[path = "../../core/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use niji_ld::arch::{Arch, Native};
use niji_ld::program;

/// System calls and output, shared by the program and the objects (no libc anywhere).
const RT: &str = r#"
static long sys3(long n, long a, long b, long c) {
    long r;
    __asm__ volatile("syscall" : "=a"(r) : "a"(n), "D"(a), "S"(b), "d"(c) : "rcx", "r11", "memory");
    return r;
}
static unsigned long len(const char *s) { unsigned long n = 0; while (s[n]) n++; return n; }
static void put(const char *p, const char *s) { sys3(1, 1, (long)p, len(p)); sys3(1, 1, (long)s, len(s)); sys3(1, 1, (long)"\n", 1); }
"#;

/// The program: `dep:<n>` from `dep_value` in `libdep.so` (a PLT call the loader must bind), then one
/// line per argument and per `LD_*` environment entry; exits 0.
const PROG: &str = r#"
int dep_value(void);
void start_c(long *sp) {
    long argc = sp[0];
    char **argv = (char **)(sp + 1), **e = argv + argc + 1;
    char d[2] = { (char)('0' + dep_value()), 0 };
    put("dep:", d);
    for (long i = 0; i < argc; i++) put("arg:", argv[i]);
    for (; *e; e++) if ((*e)[0] == 'L' && (*e)[1] == 'D' && (*e)[2] == '_') put("env:", *e);
    sys3(231, 0, 0, 0);
}
__asm__(".globl _start\n_start:\n\tmov %rsp, %rdi\n\tand $-16, %rsp\n\tcall start_c\n\thlt\n");
"#;

/// A static program (and, linked as one, a static PIE): exits 3 with nothing of the loader's start.
const STATIC: &str = "__asm__(\".globl _start\\n_start:\\n\\tmov $231, %eax\\n\\tmov $3, %edi\\n\\tsyscall\\n\");\n";

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

fn cc(d: &Path, out: &str, src: &str, flags: &[&str]) {
    let c = d.join(format!("{}.c", out.replace('/', "_")));
    std::fs::write(&c, src).unwrap();
    let o = Command::new("cc").args(["-O1", "-fno-builtin", "-fno-stack-protector", "-fcf-protection=none", "-nostdlib"]).args(flags).arg(&c).arg("-o").arg(d.join(out)).output().unwrap();
    assert!(o.status.success(), "cc {out}: {}", String::from_utf8_lossy(&o.stderr));
}

/// The scratch directory with `prog`, `lib/libdep.so` (1), `lib2/libdep.so` (2), `libpre.so` and
/// `libpre2.so` (`dep_value` 9; constructors print `preload:a` / `preload:b`), `libaud.so`,
/// `static`, `staticpie` and `notelf`; with the interpreter. `None` (and why) when unavailable.
fn fixture(name: &str) -> Option<(Scratch, PathBuf)> {
    let ld = target().join("niji").join(Native::INTERP);
    if !ld.exists() {
        eprintln!("SKIP: {} not built (run cargo xtask so)", ld.display());
        return None;
    }
    if !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no cc");
        return None;
    }
    let s = Scratch::dir_in(&target(), &format!("ldso-options-{name}"));
    let d = s.to_path_buf();
    for sub in ["lib", "lib2"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
    let shared = ["-fPIC", "-shared", "-Wl,-soname,libdep.so"];
    cc(&d, "lib/libdep.so", "int dep_value(void) { return 1; }\n", &shared);
    cc(&d, "lib2/libdep.so", "int dep_value(void) { return 2; }\n", &shared);
    for (f, tag) in [("libpre.so", "a"), ("libpre2.so", "b")] {
        let src = format!("{RT}int dep_value(void) {{ return 9; }}\n__attribute__((constructor)) static void hi(void) {{ put(\"preload:\", \"{tag}\"); }}\n");
        cc(&d, f, &src, &["-fPIC", "-shared"]);
    }
    cc(&d, "libaud.so", &format!("{RT}unsigned int la_version(unsigned int v) {{ put(\"audit:\", \"loaded\"); return v; }}\n"), &["-fPIC", "-shared"]);
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    cc(&d, "prog", &format!("{RT}{PROG}"), &["-fPIE", "-pie", "-Wl,--no-as-needed", &interp, &format!("-L{}", d.join("lib").display()), "-ldep"]);
    cc(&d, "static", STATIC, &["-static", "-no-pie"]);
    cc(&d, "staticpie", STATIC, &["-static-pie", "-fPIE"]);
    std::fs::write(d.join("notelf"), "not an elf file\n").unwrap();
    std::fs::write(d.join("notelf_long"), "not an elf file, but longer than an ELF header: ".repeat(3)).unwrap();
    Some((s, ld))
}

/// Run `ld args` in `d` with only `env` (plus PATH) in the environment, under a hang guard. `env`
/// reaches the interpreter through `env(1)`, so the host's `timeout` never sees an `LD_PRELOAD` meant
/// for the loader under test.
fn run(ld: &Path, d: &Path, env: &[(&str, &str)], args: &[&str]) -> Output {
    let mut c = Command::new("timeout");
    c.args(["20", "env"]).args(env.iter().map(|(k, v)| format!("{k}={v}"))).arg(ld).args(args).current_dir(d).env_clear();
    if let Some(p) = std::env::var_os("PATH") {
        c.env("PATH", p);
    }
    c.output().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// Standard output of a run that exited 0.
fn ran(o: &Output, what: &str) -> String {
    assert_eq!(o.status.code(), Some(0), "{what}: {o:?}");
    out(o)
}

/// (no spec clause: ld.so(8) program-mode options; the reference probed.)
#[test]
fn ldso_program_option_parsing() {
    let a = |v: &[&'static [u8]]| program::action(v.iter().copied());
    let p = |mode, name, at, options| program::Action::Program { mode, name, at, options };
    let (run, list, verify) = (program::Mode::Run, program::Mode::List, program::Mode::Verify);
    let o = program::Options::default();
    assert_eq!(a(&[b"./p", b"--argv0", b"x"]), p(run, b"./p", 0, o), "options after the program are its arguments");
    let lp = program::Options { library_path: Some(b"lib"), ..o };
    assert_eq!(a(&[b"--library-path", b"lib2", b"--library-path", b"lib", b"./p"]), p(run, b"./p", 4, lp), "the last copy wins");
    let all = program::Options { library_path: Some(b"l"), preload: Some(b"a:b"), argv0: Some(b"N"), audit: Some(b"x.so"), inhibit_cache: true, ..o };
    let args: [&[u8]; 11] = [b"--inhibit-cache", b"--library-path", b"l", b"--preload", b"a:b", b"--argv0", b"N", b"--audit", b"x.so", b"--list", b"./p"];
    assert_eq!(a(&args), p(list, b"./p", 10, all));
    assert_eq!(a(&[b"--verify", b"./p"]), p(verify, b"./p", 1, o));
    assert_eq!(a(&[b"--list"]), program::Action::Missing);
    for opt in [&b"--library-path"[..], b"--preload", b"--argv0", b"--audit"] {
        assert_eq!(a(&[opt]), program::Action::Unknown(opt), "a value is required");
    }
    assert_eq!(a(&[b"--library-path=lib", b"./p"]), program::Action::Unknown(b"--library-path=lib"));
    let search = program::Options { inhibit_rpath: Some(b""), hwcaps_prepend: Some(b"a:b"), hwcaps_mask: Some(b"x86-64-v2"), ..o };
    assert_eq!(a(&[b"--inhibit-rpath", b"", b"--glibc-hwcaps-prepend", b"a:b", b"--glibc-hwcaps-mask", b"x86-64-v2", b"./p"]), p(run, b"./p", 6, search));
    for opt in [&b"--inhibit-rpath"[..], b"--glibc-hwcaps-prepend", b"--glibc-hwcaps-mask"] {
        assert_eq!(a(&[opt]), program::Action::Unknown(opt), "a value is required");
    }
    // The two lists need no program and ignore one; the last mode option wins (probed).
    assert_eq!(a(&[b"--list-tunables"]), program::Action::Tunables);
    assert_eq!(a(&[b"--list-tunables", b"./nosuch", b"x"]), program::Action::Tunables);
    assert_eq!(a(&[b"--list", b"--list-diagnostics", b"./p"]), program::Action::Diagnostics);
    assert_eq!(a(&[b"--list-tunables", b"--list"]), program::Action::Missing);
    assert_eq!(a(&[b"--list-diagnostics", b"--frob"]), program::Action::Unknown(b"--frob"));
    // --inhibit-rpath: the path an object was loaded by, exactly; an empty first entry is the program.
    for (list, path, named) in [("", "", true), (":", "", true), ("x:", "", false), ("x::y", "", false), ("x:/a/l.so", "/a/l.so", true), ("/a/l.so:", "/a/l.so", true), ("l.so", "/a/l.so", false), ("x /a/l.so", "/a/l.so", false), ("", "/a/l.so", false), ("/a/../a/l.so", "/a/l.so", false)] {
        assert_eq!(program::rpath_inhibited(list.as_bytes(), path.as_bytes()), named, "{list:?} names {path:?}");
    }
    let sub = |pre: Option<&'static [u8]>, mask: Option<&'static [u8]>| program::hwcaps_subdirs(&[&b"v3"[..], b"v2"], pre, mask).map(|h| String::from_utf8_lossy(h).into_owned()).collect::<Vec<_>>();
    assert_eq!(sub(None, None), ["v3", "v2"]);
    assert_eq!(sub(Some(b"p1::p2"), None), ["p1", "p2", "v3", "v2"], "prepended first, empty names dropped");
    assert_eq!(sub(Some(b""), Some(b"v2:x")), ["v2"], "the mask keeps the supported names it lists");
    assert_eq!(sub(Some(b"p1"), Some(b"")), ["p1"], "an empty mask keeps none; the prepended are not masked");
    let mut b = [0u8; 24];
    assert_eq!(program::number(0, true, &mut b), b"0x0");
    assert_eq!(program::number(u64::MAX, true, &mut b), b"0xffffffffffffffff");
    assert_eq!(program::number(32767, false, &mut b), b"32767");
    let mut q = [0u8; 4];
    for (byte, text) in [(b'"', &br#"\""#[..]), (b'\\', br"\\"), (b'a', b"a"), (b'\t', br"\011"), (0xff, br"\377"), (0x7f, br"\177")] {
        assert_eq!(program::quoted_byte(byte, &mut q), text, "{byte:#x}");
    }
    assert_eq!(program::audit_names(b"a.so: b.so::c.so").collect::<Vec<_>>(), [&b"a.so"[..], b"b.so", b"c.so"]);
}

/// (no spec clause: ld.so(8) `--library-path`, `--list` and `--inhibit-cache`; the reference probed.)
#[test]
fn ldso_program_library_path() {
    let Some((s, ld)) = fixture("libpath") else { return };
    let d: &Path = &s;
    let o = ran(&run(&ld, d, &[], &["--library-path", "lib", "./prog", "a", "--argv0"]), "--library-path");
    assert_eq!(o, "dep:1\narg:./prog\narg:a\narg:--argv0\n", "the dependency from the option's directory; later words are the program's");
    let o = ran(&run(&ld, d, &[("LD_LIBRARY_PATH", "lib2")], &["--library-path", "lib", "./prog"]), "over the environment");
    assert_eq!(o, "dep:1\narg:./prog\nenv:LD_LIBRARY_PATH=lib2\n", "the option replaces LD_LIBRARY_PATH, which stays in the environment");
    let o = ran(&run(&ld, d, &[("LD_LIBRARY_PATH", "lib2")], &["./prog"]), "environment only");
    assert!(o.starts_with("dep:2\n"), "{o}");
    let o = ran(&run(&ld, d, &[], &["--library-path", "lib2", "--library-path", "nolib:lib", "./prog"]), "twice");
    assert!(o.starts_with("dep:1\n"), "the last copy, every element searched: {o}");
    let o = run(&ld, d, &[], &["./prog"]);
    assert_eq!(o.status.code(), Some(program::FAILURE), "{o:?}");
    assert!(err(&o).contains("./prog: error while loading shared libraries: libdep.so: cannot open shared object file"), "{o:?}");
    let o = ran(&run(&ld, d, &[], &["--inhibit-cache", "--library-path", "lib", "--list", "./prog"]), "--list");
    assert!(o.contains("libdep.so => lib/libdep.so"), "{o}");
    let o = ran(&run(&ld, d, &[], &["--inhibit-cache", "--library-path", "lib", "./prog"]), "--inhibit-cache");
    assert!(o.starts_with("dep:1\n"), "{o}");
}

/// `--inhibit-cache`: a search that would reach the loader cache never opens it (a dependency found
/// nowhere: without the option the search opens the cache once, which shows the trace sees it).
/// (no spec clause: ld.so(8) `--inhibit-cache`; the reference probed.)
#[test]
fn ldso_program_inhibit_cache_opens_no_cache() {
    let Some((s, ld)) = fixture("inhibit") else { return };
    let d: &Path = &s;
    if !Command::new("strace").args(["-o", "/dev/null", "true"]).status().is_ok_and(|s| s.success()) {
        return eprintln!("SKIP: strace is absent or cannot trace here");
    }
    let traced = |args: &[&str], log: &str| {
        let mut c = Command::new("timeout");
        c.args(["20", "strace", "-qq", "-e", "trace=open,openat", "-o"]).arg(d.join(log)).arg(&ld).args(args).current_dir(d).env_clear();
        let o = c.output().unwrap();
        assert_eq!(o.status.code(), Some(program::FAILURE), "libdep.so is found nowhere: {o:?}");
        let t = std::fs::read_to_string(d.join(log)).unwrap();
        let opens = t.lines().filter(|l| l.contains("ld.so.cache")).count();
        (opens, t)
    };
    let (opens, t) = traced(&["./prog"], "plain.trace");
    assert_eq!(opens, 1, "the search reaches the cache:\n{t}");
    let (opens, t) = traced(&["--inhibit-cache", "./prog"], "inhibit.trace");
    assert_eq!(opens, 0, "--inhibit-cache: no cache opened:\n{t}");
}

/// (no spec clause: ld.so(8) `--preload` and `--argv0`; the reference probed.)
#[test]
fn ldso_program_preload_and_argv0() {
    let Some((s, ld)) = fixture("preload") else { return };
    let d: &Path = &s;
    let lp = ["--library-path", "lib"];
    let with = |env: &[(&str, &str)], more: &[&str]| run(&ld, d, env, &[&lp[..], more, &["./prog"]].concat());
    let o = ran(&with(&[], &["--preload", "./libpre.so"]), "--preload");
    assert_eq!(o, "preload:a\ndep:9\narg:./prog\n", "the preload's definition comes first; LD_PRELOAD is not set");
    let o = ran(&with(&[], &["--preload", "./libpre.so:./libpre2.so"]), "a list");
    assert!(o.starts_with("preload:b\npreload:a\ndep:9\n"), "loaded in order, initialised in reverse: {o}");
    let o = ran(&with(&[], &["--preload", "./libpre.so", "--preload", "./libpre2.so"]), "twice");
    assert!(o.starts_with("preload:b\ndep:9\n"), "the last copy only: {o}");
    let o = ran(&with(&[("LD_PRELOAD", "./libpre2.so")], &["--preload", "./libpre.so"]), "with LD_PRELOAD");
    assert!(o.starts_with("preload:a\npreload:b\ndep:9\n") && o.contains("env:LD_PRELOAD=./libpre2.so"), "LD_PRELOAD's first, then the option's: {o}");
    let o = with(&[], &["--preload", "./nosuch.so"]);
    assert_eq!(out(&o), "dep:1\narg:./prog\n", "{o:?}");
    assert!(err(&o).contains("ERROR: ld.so: object './nosuch.so' from --preload cannot be preloaded (cannot open shared object file): ignored."), "{o:?}");
    // With LD_PRELOAD too, each missing name still names its own source.
    let o = with(&[("LD_PRELOAD", "./nosuch2.so")], &["--preload", "./nosuch.so"]);
    assert!(err(&o).contains("object './nosuch.so' from --preload cannot be preloaded"), "{o:?}");
    assert!(err(&o).contains("object './nosuch2.so' from LD_PRELOAD cannot be preloaded"), "{o:?}");
    let o = ran(&with(&[], &["--argv0", "NAME"]), "--argv0");
    assert_eq!(o, "dep:1\narg:NAME\n");
    let o = ran(&run(&ld, d, &[], &["--argv0", "", "--library-path", "lib", "./prog", "x"]), "empty --argv0");
    assert_eq!(o, "dep:1\narg:\narg:x\n");
}

/// (no spec clause: ld.so(8) `--verify` and `--audit`, the options libniji refuses and the usage
/// errors; the reference probed.)
#[test]
fn ldso_program_verify_audit_and_refusals() {
    let Some((s, ld)) = fixture("verify") else { return };
    let d: &Path = &s;
    for (file, status) in [("./prog", 0), ("lib/libdep.so", 2), ("./staticpie", 2), ("./static", 1), ("./notelf", 1), ("./nosuch", 1)] {
        let o = run(&ld, d, &[], &["--verify", file]);
        assert_eq!(o.status.code(), Some(status), "--verify {file}: {o:?}");
        assert!(o.stdout.is_empty() && o.stderr.is_empty(), "--verify writes nothing: {o:?}");
    }
    let o = run(&ld, d, &[], &["--library-path", "lib", "--audit", "./libaud.so", "./prog"]);
    assert_eq!(out(&o), "dep:1\narg:./prog\n", "the program runs, the auditor is not loaded: {o:?}");
    assert!(err(&o).contains("ERROR: ld.so: object './libaud.so' cannot be loaded as audit interface: "), "never silently ignored: {o:?}");
    let me = ld.to_str().unwrap();
    for (args, text) in [(&["--frob", "./prog"][..], "unrecognized option '--frob'"), (&["--library-path"], "unrecognized option '--library-path'"), (&[], "missing program name"), (&["--list"], "missing program name")] {
        let o = run(&ld, d, &[], args);
        assert_eq!(o.status.code(), Some(program::USAGE), "{args:?}: {o:?}");
        assert!(err(&o).starts_with(&format!("{me}: {text}")) && err(&o).contains(&format!("Try '{me} --help'")), "{args:?}: {o:?}");
    }
}

/// `--inhibit-rpath`, the `glibc-hwcaps` options and the two lists, end to end.
/// (no spec clause: ld.so(8) `--inhibit-rpath`, `--glibc-hwcaps-prepend`, `--glibc-hwcaps-mask`,
/// `--list-tunables`, `--list-diagnostics`; the reference probed.)
#[test]
fn ldso_program_inhibit_rpath_hwcaps_and_lists() {
    let Some((s, ld)) = fixture("search") else { return };
    let d: &Path = &s;
    // A program whose RUNPATH finds libdep.so in lib/ (the plain one has none).
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    cc(d, "prog_rp", &format!("{RT}{PROG}"), &["-fPIE", "-pie", "-Wl,--no-as-needed", &interp, &format!("-L{}", d.join("lib").display()), "-ldep", "-Wl,--enable-new-dtags,-rpath,lib"]);
    assert!(ran(&run(&ld, d, &[], &["./prog_rp"]), "RUNPATH").starts_with("dep:1\n"));
    for list in ["", ":"] {
        let o = run(&ld, d, &[], &["--inhibit-rpath", list, "./prog_rp"]);
        assert_eq!(o.status.code(), Some(program::FAILURE), "{list:?} names the program: {o:?}");
        assert!(err(&o).contains("libdep.so: cannot open shared object file"), "{o:?}");
    }
    for list in ["x:", "prog_rp", "./prog_rp"] {
        assert!(ran(&run(&ld, d, &[], &["--inhibit-rpath", list, "./prog_rp"]), list).starts_with("dep:1\n"), "{list:?} names no object");
    }
    let o = run(&ld, d, &[], &["--inhibit-rpath", "", "--list", "./prog_rp"]);
    assert_ne!(o.status.code(), Some(0), "--list searches as the start does: {o:?}");
    // glibc-hwcaps: hw/ holds libdep.so (1), and in glibc-hwcaps/ x86-64-v2 (6) and myhw (5).
    for (sub, v) in [("hw", 1), ("hw/glibc-hwcaps/x86-64-v2", 6), ("hw/glibc-hwcaps/myhw", 5)] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
        cc(d, &format!("{sub}/libdep.so"), &format!("int dep_value(void) {{ return {v}; }}\n"), &["-fPIC", "-shared", "-Wl,-soname,libdep.so"]);
    }
    let dep = |more: &[&str]| ran(&run(&ld, d, &[], &[&["--library-path", "hw"][..], more, &["./prog"]].concat()), "hwcaps").lines().next().unwrap_or("").to_string();
    if dep(&[]) != "dep:6" {
        return eprintln!("SKIP: this CPU does not support x86-64-v2, the glibc-hwcaps subdirectory the test needs");
    }
    assert_eq!(dep(&["--glibc-hwcaps-mask", ""]), "dep:1", "an empty mask: no subdirectory");
    assert_eq!(dep(&["--glibc-hwcaps-mask", "x:x86-64-v2"]), "dep:6");
    assert_eq!(dep(&["--glibc-hwcaps-prepend", "myhw"]), "dep:5", "prepended first");
    assert_eq!(dep(&["--glibc-hwcaps-prepend", "myhw", "--glibc-hwcaps-prepend", ""]), "dep:6", "the last copy wins");
    assert_eq!(dep(&["--glibc-hwcaps-prepend", "myhw", "--glibc-hwcaps-mask", ""]), "dep:5", "the prepended are not masked");
    // --list-tunables: libniji's tunables in the reference's form; a program after it is not opened.
    let o = ran(&run(&ld, d, &[("GLIBC_TUNABLES", "niji.pthread.rseq=0:glibc.cpu.x86_rep_movsb_threshold=0x1000:glibc.cpu.hwcaps=-AVX2")], &["--list-tunables", "./nosuch"]), "--list-tunables");
    for line in ["niji.pthread.rseq: 0 (min: 0, max: 1)", "niji.cpu.rep_movsb_threshold: 0x1000 (min: 0x100, max: 0xffffffffffffffff)", "niji.cpu.hwcaps: -AVX2", "niji.malloc.perturb: 0 (min: 0, max: 255)"] {
        assert!(o.lines().any(|l| l == line), "{line:?} in\n{o}");
    }
    let o = ran(&run(&ld, d, &[], &["--list-tunables"]), "defaults");
    assert!(o.lines().any(|l| l == "niji.pthread.rseq: 1 (min: 0, max: 1)") && o.lines().any(|l| l == "niji.cpu.hwcaps:"), "{o}");
    // --list-diagnostics: key=value lines, strings quoted and escaped, numbers in hexadecimal.
    let o = ran(&run(&ld, d, &[("ZZ_SECRET", "hidden"), ("LD_X", "a\"b\\c\td")], &["--list-diagnostics", "./nosuch"]), "--list-diagnostics");
    assert!(o.lines().all(|l| l.split_once('=').is_some_and(|(k, _)| !k.is_empty())), "every line is key=value:\n{o}");
    assert!(o.lines().any(|l| l.starts_with("env_filtered[0x") && l.ends_with("]=\"ZZ_SECRET\"")) && !o.contains("hidden"), "{o}");
    assert!(o.lines().any(|l| l.starts_with("env[0x") && l.ends_with(r#"]="LD_X=a\"b\\c\011d""#)), "{o}");
    assert!(o.contains(&format!("path.rtld=\"{}\"\n", Native::INTERP)) && o.contains("dl_pagesize=0x") && o.contains("auxv[0x0].a_type=0x"), "{o}");
}

/// (no spec clause: ld.so(8) run as a program on files that are not dynamic programs; the reference
/// probed.)
#[test]
fn ldso_program_files_that_are_not_dynamic_programs() {
    let Some((s, ld)) = fixture("files") else { return };
    let d: &Path = &s;
    for prog in ["./static", "./staticpie"] {
        let o = run(&ld, d, &[], &[prog]);
        assert_eq!(o.status.code(), Some(3), "{prog} starts itself, as from the kernel: {o:?}");
    }
    for (file, reason) in [
        ("./nosuch", "cannot open shared object file: No such file or directory"),
        ("./notelf", "file too short"),
        ("./notelf_long", "invalid ELF header"),
        ("./lib", "cannot read file data: Error 21"),
        ("./lib/libdep.so", "the object has no entry point"),
    ] {
        let o = run(&ld, d, &[], &[file]);
        assert_eq!(o.status.code(), Some(program::FAILURE), "{file}: {o:?}");
        assert!(err(&o).starts_with(&format!("{file}: error while loading shared libraries: {file}: {reason}")), "{file}: {o:?}");
    }
}

/// `--list` of a file it cannot list ends as running it does: the loading-error form, status 127,
/// nothing on standard output.
/// (no spec clause: ld.so(8) `--list`; the reference probed, LDSO-PROGRAM-OPTIONS-REST.)
#[test]
fn ldso_program_list_of_files_that_cannot_be_listed() {
    let Some((s, ld)) = fixture("listfiles") else { return };
    let d: &Path = &s;
    for (file, reason) in [
        ("./nosuch", "cannot open shared object file: No such file or directory"),
        ("./notelf", "file too short"),
        ("./notelf_long", "invalid ELF header"),
        ("./lib", "cannot read file data: Error 21"),
    ] {
        let o = run(&ld, d, &[], &["--list", file]);
        assert_eq!(o.status.code(), Some(program::FAILURE), "--list {file}: {o:?}");
        assert_eq!(err(&o), format!("{file}: error while loading shared libraries: {file}: {reason}\n"), "--list {file}");
        assert!(o.stdout.is_empty(), "--list {file}: {o:?}");
    }
}

/// A program linked with libniji's C library that `dlopen`s `libdep.so` and prints its
/// `dep_value` (`dlopen:<n>`), or `dlerror:<text>` and status 1.
const DLOPEN_PROG: &str = r#"
#include <dlfcn.h>
#include <stdio.h>
int main(void) {
    void *h = dlopen("libdep.so", RTLD_NOW);
    if (!h) { printf("dlerror:%s\n", dlerror()); return 1; }
    int (*f)(void) = (int (*)(void))dlsym(h, "dep_value");
    printf("dlopen:%d\n", f ? f() : -1);
    return 0;
}
"#;

/// `dlopen` after `ld.so --library-path` searches the option's directories instead of
/// `LD_LIBRARY_PATH`'s (the environment keeps its value), with or without `--inhibit-cache`, as the
/// start's own search does.
/// (no spec clause: ld.so(8) `--library-path` and `--inhibit-cache`; the reference probed: dlopen uses
/// the option, LDSO-RUN-PROGRAM.)
#[test]
fn ldso_program_dlopen_after_library_path_and_inhibit_cache() {
    let Some((s, ld)) = fixture("dlopen") else { return };
    let d: &Path = &s;
    let niji = std::fs::canonicalize(target().join("niji")).unwrap();
    let (libc, crt1) = (niji.join("libc.so.6"), niji.join("niji-crt1.o"));
    if !libc.exists() || !crt1.exists() {
        eprintln!("SKIP: libc.so.6 or niji-crt1.o not built (run cargo xtask so)");
        return;
    }
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let src = d.join("dprog.c");
    std::fs::write(&src, DLOPEN_PROG).unwrap();
    let o = Command::new("cc")
        .args(["-O1", "-fno-stack-protector", "-fcf-protection=none", "-nostdlib", "-fPIE", "-pie", &interp])
        .arg(&src)
        .arg(&crt1)
        .arg(&libc)
        .arg(&ld)
        .args(["-lgcc", "-o"])
        .arg(d.join("dprog"))
        .output()
        .unwrap();
    assert!(o.status.success(), "cc dprog: {}", String::from_utf8_lossy(&o.stderr));
    let path = |dirs: &str| format!("{dirs}:{}", niji.display());
    let env = [("LD_LIBRARY_PATH", "lib2")];
    let o = ran(&run(&ld, d, &env, &["--library-path", &path("lib"), "./dprog"]), "dlopen after --library-path");
    assert_eq!(o, "dlopen:1\n", "dlopen searches the option's directory, not LD_LIBRARY_PATH's");
    let o = ran(&run(&ld, d, &env, &["--inhibit-cache", "--library-path", &path("lib"), "./dprog"]), "dlopen after --inhibit-cache");
    assert_eq!(o, "dlopen:1\n", "--inhibit-cache keeps the option's directories for dlopen");
    let o = ran(&run(&ld, d, &[("LD_LIBRARY_PATH", &path("lib2"))], &["./dprog"]), "dlopen without the option");
    assert_eq!(o, "dlopen:2\n", "without the option, dlopen searches LD_LIBRARY_PATH");
    let o = run(&ld, d, &env, &["--library-path", &path("nolib"), "./dprog"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(out(&o).starts_with("dlerror:libdep.so: cannot open shared object file"), "the option's directories only: {o:?}");
}
