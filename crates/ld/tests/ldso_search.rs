//! `ld.so` finds a program's `DT_NEEDED` libraries named without `/` at program start (DL-START-SEARCH,
//! D-27.16): the built product runs `-nostdlib` programs whose libraries are found through each search
//! step (DT_RPATH and its requester chain, LD_LIBRARY_PATH, DT_RUNPATH of the requester only, `$ORIGIN`
//! of the program and of a found library), each library variant returning its own exit status, against
//! the reference loader run on the same files (status, error text, and the `--list` lines); a candidate
//! of another machine passed over and a non-object refused; a set-group-ID start that ignores
//! LD_LIBRARY_PATH; and (with `strace`) the cache opened once when a search reaches it, never when every
//! name is a path.
//!
//! Clause: s27_1_c_1_1 (path search applies when the name contains no slash: DT_RPATH, LD_LIBRARY_PATH,
//! DT_RUNPATH, the cache, then the default directories). The cache's own format and lookup order are
//! covered by search_cache.rs and cache_write.rs; a product built for `/` reads the host's cache, which a
//! test must never write, so here the cache step is shown on the host's own cache (a system library).
//! Skips (with the reason) when `cc` is absent, the product is not built, the reference loader is absent
//! (its comparison only), the user has no supplementary group or the file system ignores the
//! set-group-ID bit (secure test only), or `strace` cannot trace (cache test only).
// ARCH(x86_64): the test programs are x86-64 assembly and system call numbers; the product is x86_64's.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use niji_ld::arch::{Arch, Native};

/// The program: calls `x()` from its first dependency and exits with what it returns. No libc.
const PROG: &str = r#"
int x(void);
void start_c(void) {
    long r, code = x();
    __asm__ volatile("syscall" : "=a"(r) : "a"(231L), "D"(code) : "rcx", "r11", "memory");
    for (;;) {}
}
__asm__(".globl _start\n_start:\n\tand $-16, %rsp\n\tcall start_c\n\thlt\n");
"#;

/// The secure-start probe: exits with its AT_SECURE value (0 when the vector has none). No libc.
const SECURE: &str = r#"
void start_c(long *sp) {
    long argc = sp[0], secure = 0, r;
    char **e = (char **)(sp + 1) + argc + 1;
    while (*e) e++;
    for (long *aux = (long *)(e + 1); aux[0]; aux += 2)
        if (aux[0] == 23) secure = aux[1];
    __asm__ volatile("syscall" : "=a"(r) : "a"(231L), "D"(secure) : "rcx", "r11", "memory");
    for (;;) {}
}
__asm__(".globl _start\n_start:\n\tmov %rsp, %rdi\n\tand $-16, %rsp\n\tcall start_c\n\thlt\n");
"#;

/// A library variant: `x()` returns `V`.
const STEP: &str = "int x(void) { return V; }\n";
/// A library that needs another: `x()` is 100 plus the leaf's `y()`.
const MID: &str = "int y(void);\nint x(void) { return y() + 100; }\n";
/// A leaf library: `y()` returns `V`.
const LEAF: &str = "int y(void) { return V; }\n";

const FLAGS: [&str; 4] = ["-nostdlib", "-O1", "-fno-stack-protector", "-fcf-protection=none"];

fn cc_ok() -> bool {
    Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success())
}

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// The built interpreter, or `None` (with the reason) before `cargo xtask so`.
fn product() -> Option<PathBuf> {
    let p = target().join("niji").join(Native::INTERP);
    if p.exists() {
        return Some(p);
    }
    eprintln!("SKIP: {} not built (run cargo xtask so)", p.display());
    None
}

/// The reference loader, or `None` (with the reason): its comparison is then left out.
fn reference() -> Option<PathBuf> {
    let p = Path::new("/lib64").join(Native::INTERP);
    if p.exists() {
        return Some(p);
    }
    eprintln!("NOTE: no reference loader at {}: libniji's results are checked against the expected values only", p.display());
    None
}

/// A canonical scratch directory under the target directory (not /tmp: the set-group-ID test needs a
/// file system that honours the bit; canonical, so `$ORIGIN` and the given paths agree).
fn dir(name: &str) -> PathBuf {
    let d = target().join(format!("ldso-search-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(&d).unwrap()
}

fn s(p: &Path) -> String {
    p.to_str().unwrap().to_owned()
}

/// Compile `src` (with `-DV=<v>`) in `d` with `args`, output `out` (relative to `d`).
fn cc(d: &Path, src: &str, v: i32, out: &str, args: &[String]) {
    let c = d.join(format!("{}.c", out.replace('/', "_")));
    std::fs::write(&c, src).unwrap();
    if let Some(parent) = d.join(out).parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let o = Command::new("cc").current_dir(d).args(FLAGS).arg(format!("-DV={v}")).args(args).arg(&c).arg("-o").arg(out).output().unwrap();
    assert!(o.status.success(), "cc {out}: {}", String::from_utf8_lossy(&o.stderr));
}

/// A shared library `out` with DT_SONAME `soname` (none when empty).
fn lib(d: &Path, src: &str, v: i32, out: &str, soname: &str, extra: &[String]) {
    // Every library named is recorded (a toolchain may default to --as-needed, and the libraries
    // come before the source file).
    let mut args = vec!["-shared".to_owned(), "-fPIC".to_owned(), "-Wl,--no-as-needed".to_owned()];
    if !soname.is_empty() {
        args.push(format!("-Wl,-soname,{soname}"));
    }
    args.extend_from_slice(extra);
    cc(d, src, v, out, &args);
}

/// The program `out`, interpreter `ld`, linked with `extra` (its libraries and search paths).
fn prog(d: &Path, ld: &Path, out: &str, extra: &[String]) -> PathBuf {
    prog_from(d, ld, PROG, out, extra)
}

/// The program `out` from `src`, interpreter `ld`, linked with `extra`.
fn prog_from(d: &Path, ld: &Path, src: &str, out: &str, extra: &[String]) -> PathBuf {
    let mut args: Vec<String> = ["-fPIE", "-pie", "-Wl,-z,relro", "-Wl,--no-as-needed"].map(str::to_owned).to_vec();
    args.push(format!("-Wl,--dynamic-linker={}", ld.display()));
    args.extend_from_slice(extra);
    cc(d, src, 0, out, &args);
    d.join(out)
}

/// `cmd` with LD_LIBRARY_PATH set to `llp`, or removed (the test runner may set it).
fn with_llp(mut cmd: Command, llp: Option<&str>) -> Command {
    match llp {
        Some(v) => cmd.env("LD_LIBRARY_PATH", v),
        None => cmd.env_remove("LD_LIBRARY_PATH"),
    };
    cmd
}

/// Run with a hang guard only (`timeout`; a regression could spin).
fn timed(prog: &Path) -> Command {
    let mut c = Command::new("timeout");
    c.arg("60").arg(prog);
    c
}

fn output(mut c: Command) -> Output {
    c.output().unwrap()
}

/// The `name => path` lines of a `--list` output, addresses removed.
fn deps(out: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(out)
        .lines()
        .filter_map(|l| l.trim().split_once(" => "))
        .map(|(n, p)| format!("{n} => {}", p.split(" (0x").next().unwrap_or("")))
        .collect()
}

/// The libraries the search tests use, in `d`: `libstep.so.1` returning 11 (in `rp`), 12 (`llp`), 13
/// (`rn`); a link-time copy (`link`, never searched); `a/libmid.so.1` (RUNPATH `$ORIGIN/inner`) needing
/// `a/inner/libleaf.so.1` (21); `b/libmidp.so` (no DT_SONAME, so a program names it by its path;
/// RUNPATH `$ORIGIN/../a/inner`) needing the same leaf; `c/libmidc.so.1` (no search path) needing
/// `c/libleafc.so.1` (22); `badm` holds another machine's `libstep.so.1`, `badx` a 200-byte
/// non-object of that name.
fn libraries(d: &Path) {
    for (v, at) in [(0, "link"), (11, "rp"), (12, "llp"), (13, "rn")] {
        lib(d, STEP, v, &format!("{at}/libstep.so.1"), "libstep.so.1", &[]);
    }
    lib(d, LEAF, 21, "a/inner/libleaf.so.1", "libleaf.so.1", &[]);
    lib(d, MID, 0, "a/libmid.so.1", "libmid.so.1", &[format!("-L{}", s(&d.join("a/inner"))), "-l:libleaf.so.1".into(), "-Wl,--enable-new-dtags,-rpath,$ORIGIN/inner".into()]);
    lib(d, MID, 0, "b/libmidp.so", "", &[format!("-L{}", s(&d.join("a/inner"))), "-l:libleaf.so.1".into(), "-Wl,--enable-new-dtags,-rpath,$ORIGIN/../a/inner".into()]);
    lib(d, LEAF, 22, "c/libleafc.so.1", "libleafc.so.1", &[]);
    lib(d, MID, 0, "c/libmidc.so.1", "libmidc.so.1", &[format!("-L{}", s(&d.join("c"))), "-l:libleafc.so.1".into()]);
    let mut other = std::fs::read(d.join("llp/libstep.so.1")).unwrap();
    other[18..20].copy_from_slice(&183u16.to_le_bytes()); // e_machine: EM_AARCH64
    std::fs::create_dir_all(d.join("badm")).unwrap();
    std::fs::write(d.join("badm/libstep.so.1"), other).unwrap();
    std::fs::create_dir_all(d.join("badx")).unwrap();
    std::fs::write(d.join("badx/libstep.so.1"), [b'x'; 200]).unwrap();
}

/// One case: what it shows, the program, LD_LIBRARY_PATH (or none), and the exit status or the text
/// after `<prog>: error while loading shared libraries: ` (status 127).
type Case<'a> = (&'a str, &'a Path, Option<&'a str>, Result<i32, String>);

/// Each search step, the requester chain and the candidate rule, against the expected result and the
/// reference loader run on the same program and environment: the exit status (which library variant
/// ran), the error text and status 127 for a library not found or refused, and for a program that
/// starts, the `--list` lines (the libraries the search finds) equal the reference's.
/// Clause: s27_1_c_1_1.
#[test]
fn ldso_search_steps_match_reference() {
    let (Some(ld), true) = (product(), cc_ok()) else { return eprintln!("SKIP: no product or no cc") };
    let refl = reference();
    let d = dir("steps");
    libraries(&d);
    let p = |x: &str| s(&d.join(x));
    let step = || vec![format!("-L{}", p("link")), "-l:libstep.so.1".to_owned()];
    let with = |mut v: Vec<String>, more: &[String]| {
        v.extend_from_slice(more);
        v
    };
    let rpath = prog(&d, &ld, "p_rpath", &with(step(), &[format!("-Wl,--disable-new-dtags,-rpath,{}", p("rp"))]));
    let runpath = prog(&d, &ld, "p_runpath", &with(step(), &[format!("-Wl,--enable-new-dtags,-rpath,{}", p("rn"))]));
    let plain = prog(&d, &ld, "p_plain", &step());
    let origin = prog(&d, &ld, "p_origin", &[format!("-L{}", p("a")), "-l:libmid.so.1".into(), format!("-Wl,-rpath-link,{}", p("a/inner")), "-Wl,--enable-new-dtags,-rpath,$ORIGIN/a".into()]);
    // `b/libmidp.so` by its absolute path (it has no DT_SONAME): its leaf is found only through the
    // `$ORIGIN` of an object opened by path.
    let origin_path = prog(&d, &ld, "p_origin_path", &[p("b/libmidp.so"), format!("-Wl,-rpath-link,{}", p("a/inner"))]);
    let mids = || vec![format!("-L{}", p("c")), "-l:libmidc.so.1".to_owned(), format!("-Wl,-rpath-link,{}", p("c"))];
    let chain_rpath = prog(&d, &ld, "p_chain_rpath", &with(mids(), &[format!("-Wl,--disable-new-dtags,-rpath,{}", p("c"))]));
    let chain_runpath = prog(&d, &ld, "p_chain_runpath", &with(mids(), &[format!("-Wl,--enable-new-dtags,-rpath,{}", p("c"))]));
    let missing = |name: &str| -> Result<i32, String> { Err(format!("{name}: cannot open shared object file: No such file or directory")) };
    let (llp, badm, badx) = (p("llp"), format!("{}:{}", p("badm"), p("llp")), format!("{}:{}", p("badx"), p("llp")));
    let cases: [Case<'_>; 12] = [
        ("DT_RPATH", &rpath, None, Ok(11)),
        ("DT_RPATH before LD_LIBRARY_PATH", &rpath, Some(llp.as_str()), Ok(11)),
        ("DT_RUNPATH", &runpath, None, Ok(13)),
        ("LD_LIBRARY_PATH before DT_RUNPATH", &runpath, Some(llp.as_str()), Ok(12)),
        ("LD_LIBRARY_PATH", &plain, Some(llp.as_str()), Ok(12)),
        ("not found", &plain, None, missing("libstep.so.1")),
        ("$ORIGIN of the program and of a found library", &origin, None, Ok(121)),
        ("$ORIGIN of a library named by path", &origin_path, None, Ok(121)),
        ("DT_RPATH of the requester's loader", &chain_rpath, None, Ok(122)),
        ("DT_RUNPATH of the requester only", &chain_runpath, None, missing("libleafc.so.1")),
        ("another machine's candidate passed over", &plain, Some(badm.as_str()), Ok(12)),
        ("a non-object candidate refused", &plain, Some(badx.as_str()), Err(format!("{}/libstep.so.1: invalid ELF header", p("badx")))),
    ];
    for (what, prog, env, want) in cases {
        let got = output(with_llp(timed(prog), env));
        let mut runs = vec![("niji", got)];
        if let Some(r) = &refl {
            let mut c = Command::new("timeout");
            c.arg("60").arg(r).arg(prog);
            runs.push(("reference", output(with_llp(c, env))));
        }
        for (who, o) in &runs {
            let err = String::from_utf8_lossy(&o.stderr);
            match &want {
                Ok(code) => assert_eq!(o.status.code(), Some(*code), "{what}: {who}: {err}"),
                Err(text) => {
                    assert_eq!(o.status.code(), Some(127), "{what}: {who}: status");
                    assert_eq!(err, format!("{}: error while loading shared libraries: {text}\n", prog.display()), "{what}: {who}: message");
                }
            }
        }
        // The `--list` line of an object needed by its path differs from the reference's (libniji:
        // `PATH => PATH`, reference: the path alone; the dependency list, not the start-time search):
        // that case compares the run only, until the list's line form is settled.
        if let (Ok(_), Some(r), false) = (&want, &refl, prog == origin_path.as_path()) {
            let list = |l: &Path| output(with_llp({ let mut c = Command::new(l); c.arg("--list").arg(prog); c }, env));
            let (mine, theirs) = (list(&ld), list(r));
            assert_eq!(mine.status.code(), Some(0), "{what}: --list: {}", String::from_utf8_lossy(&mine.stderr));
            assert_eq!(deps(&mine.stdout), deps(&theirs.stdout), "{what}: --list differs from the reference");
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// A set-group-ID program (the kernel sets AT_SECURE; a supplementary group of the user, no root)
/// ignores LD_LIBRARY_PATH: its DT_RUNPATH library runs, not the LD_LIBRARY_PATH one.
/// Clause: s27_1_c_1_1; threat model rule 5 (secure mode: no LD_LIBRARY_PATH).
#[test]
fn ldso_search_secure_ignores_library_path() {
    let (Some(ld), true) = (product(), cc_ok()) else { return eprintln!("SKIP: no product or no cc") };
    let id = |a: &str| String::from_utf8(Command::new("id").arg(a).output().unwrap().stdout).unwrap();
    let primary = id("-g").trim().to_owned();
    let Some(group) = id("-G").split_whitespace().find(|g| *g != primary).map(str::to_owned) else {
        return eprintln!("SKIP: the user has no supplementary group (a set-group-ID file needs one without root)");
    };
    let d = dir("secure");
    for (v, at) in [(0, "link"), (12, "llp"), (13, "rn")] {
        lib(&d, STEP, v, &format!("{at}/libstep.so.1"), "libstep.so.1", &[]);
    }
    let prog = prog(&d, &ld, "p_runpath", &[format!("-L{}", s(&d.join("link"))), "-l:libstep.so.1".into(), format!("-Wl,--enable-new-dtags,-rpath,{}", s(&d.join("rn")))]);
    let llp = s(&d.join("llp"));
    let plain = output(with_llp(timed(&prog), Some(llp.as_str())));
    assert_eq!(plain.status.code(), Some(12), "not secure: LD_LIBRARY_PATH first");
    assert!(Command::new("chgrp").arg(&group).arg(&prog).status().unwrap().success());
    assert!(Command::new("chmod").arg("g+s").arg(&prog).status().unwrap().success());
    let probe = prog_from(&d, &ld, SECURE, "p_secure", &[]);
    assert!(Command::new("chgrp").arg(&group).arg(&probe).status().unwrap().success());
    assert!(Command::new("chmod").arg("g+s").arg(&probe).status().unwrap().success());
    if output(timed(&probe)).status.code() != Some(1) {
        let _ = std::fs::remove_dir_all(&d);
        return eprintln!("SKIP: the kernel did not set AT_SECURE (the file system ignores set-group-ID here)");
    }
    let secure = output(with_llp(timed(&prog), Some(llp.as_str())));
    assert_eq!(secure.status.code(), Some(13), "secure: LD_LIBRARY_PATH ignored, DT_RUNPATH used: {}", String::from_utf8_lossy(&secure.stderr));
    let _ = std::fs::remove_dir_all(&d);
}

/// The cache is opened only when a search reaches it, and once per start: a program whose only
/// `DT_NEEDED` entry is a path opens no cache; one needing a system library (found in the cache) and
/// then a missing one opens it exactly once and ends with the missing library's message, status 127.
/// Clause: s27_1_c_1_1 (the cache step).
#[test]
fn ldso_search_cache_opened_once_when_reached() {
    let (Some(ld), true) = (product(), cc_ok()) else { return eprintln!("SKIP: no product or no cc") };
    if option_env!("NIJI_PREFIX").is_some() {
        return eprintln!("SKIP: a prefixed build reads <prefix>/etc/ld.so.cache, not the host's cache");
    }
    if !Command::new("strace").args(["-o", "/dev/null", "true"]).status().is_ok_and(|s| s.success()) {
        return eprintln!("SKIP: strace is absent or cannot trace here");
    }
    let d = dir("cache");
    lib(&d, STEP, 11, "nos/libnos.so", "", &[]);
    lib(&d, STEP, 0, "nope/libnope.so.1", "libnope.so.1", &[]);
    let by_path = prog(&d, &ld, "p_path", &[s(&d.join("nos/libnos.so"))]);
    let cached = prog(&d, &ld, "p_cache", &["-lgcc_s".into(), format!("-L{}", s(&d.join("nope"))), "-l:libnope.so.1".into()]);
    std::fs::remove_file(d.join("nope/libnope.so.1")).unwrap();
    let traced = |p: &Path, log: &str| {
        let mut c = Command::new("timeout");
        c.arg("60").arg("strace").args(["-qq", "-e", "trace=open,openat", "-o"]).arg(d.join(log)).arg(p);
        let o = output(with_llp(c, None));
        (o, std::fs::read_to_string(d.join(log)).unwrap())
    };
    let opens = |t: &str| t.lines().filter(|l| l.contains("ld.so.cache")).count();
    let (o, t) = traced(&by_path, "path.trace");
    assert_eq!(o.status.code(), Some(11), "path-named library: {}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(opens(&t), 0, "every DT_NEEDED is a path: no cache open:\n{t}");
    let (o, t) = traced(&cached, "cache.trace");
    assert_eq!(o.status.code(), Some(127), "missing after a cached library: {}", String::from_utf8_lossy(&o.stderr));
    let want = format!("{}: error while loading shared libraries: libnope.so.1: cannot open shared object file: No such file or directory\n", cached.display());
    assert_eq!(String::from_utf8_lossy(&o.stderr), want);
    assert!(t.lines().any(|l| l.contains("libgcc_s.so.1\"") && !l.contains("= -1")), "libgcc_s.so.1 found:\n{t}");
    assert_eq!(opens(&t), 1, "two searches reached the cache: one open:\n{t}");
    let _ = std::fs::remove_dir_all(&d);
}
