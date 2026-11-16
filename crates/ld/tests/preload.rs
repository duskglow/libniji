//! Preloaded objects under libniji's `ld.so` (DL-LD-PRELOAD; ld.so(8) "LD_PRELOAD" and
//! "/etc/ld.so.preload"; plan/ph14/threat-model-draft.md rule 1; spec/behaviour/dynlink-preload.md,
//! the reference probe): `LD_PRELOAD` objects interpose on the program's dependencies and on libc,
//! `dlsym(RTLD_NEXT, ...)` from a preload reaches libc's definition, the constructor order, an object
//! that cannot be preloaded (the reference's line; the program runs), the secure-mode rule (a
//! set-group-ID program: no root), and the preload file (a user and mount namespace binds a private
//! `/etc` over the real one: no root, nothing on the host changes).
//!
//! Skips (with the reason) when `cc` is absent, the products are not built, the user has no
//! supplementary group or neither the corpus directory nor the temporary directory honours the
//! set-group-ID bit (secure test: `nosuid`, no_new_privs), or
//! unprivileged user namespaces are unavailable (file test).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use niji_ld::arch::{Arch, Native};
use niji_ld::elf::Elf;

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

/// A library `lib<name>.so`: a constructor that prints `ctor <name>`, `f` returning `f`, then `extra`.
fn lib_src(name: &str, f: u32, extra: &str) -> String {
    format!("#include <unistd.h>\n__attribute__((constructor)) static void c(void) {{ write(1, \"ctor {name}\\n\", {}); }}\nint f(void) {{ return {f}; }}\n{extra}\n", 6 + name.len())
}

/// `libpre`'s `strlen`: libc's (found with `RTLD_NEXT`) plus 1000.
const PRE_STRLEN: &str = "#define _GNU_SOURCE
#include <dlfcn.h>
#include <string.h>
size_t strlen(const char *s) { size_t (*n)(const char *) = (size_t (*)(const char *))dlsym(RTLD_NEXT, \"strlen\"); return n(s) + 1000; }";

/// The program: needs `libdep.so` (its `f`, 2); prints `ctor prog` from its constructor, then
/// `f=<f()> strlen=<strlen("abcd")>`, then `secure` when AT_SECURE is set.
const PROG: &str = "#include <stdio.h>
#include <string.h>
#include <sys/auxv.h>
#include <unistd.h>
int f(void);
__attribute__((constructor)) static void c(void) { write(1, \"ctor prog\\n\", 10); }
int main(void) {
    char b[64];
    int n = snprintf(b, sizeof b, \"f=%d strlen=%zu\\n\", f(), strlen(\"abcd\"));
    write(1, b, n);
    if (getauxval(AT_SECURE)) write(1, \"secure\\n\", 7);
    return 0;
}
";

/// The corpus in a new scratch directory: the program, `libdep.so` (needed by it, `f` = 2),
/// `libpre.so` (`f` = 1 and the `strlen` interposer), `libpre2.so` (`f` = 5), `libepre.so` (`f` =
/// 3), each printing its constructor; returns (directory, `ld.so`, program), or `None` with the
/// reason printed.
fn corpus(name: &str) -> Option<(PathBuf, PathBuf, PathBuf)> {
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
    let d = target().join(format!("ld-preload-{name}-{}", std::process::id()));
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
    let libc_s = libc.to_str().unwrap();
    for (n, f, extra) in [("dep", 2, ""), ("pre", 1, PRE_STRLEN), ("pre2", 5, ""), ("epre", 3, "")] {
        let soname = format!("-Wl,-soname,lib{n}.so");
        cc(&d, &format!("lib{n}.so"), &lib_src(n, f, extra), &["-nostdlib", "-fPIC", "-shared", "-Wl,--no-as-needed", &soname, libc_s]);
    }
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let (ldir, rpath) = (format!("-L{}", d.display()), format!("-Wl,-rpath,{}", d.display()));
    let crt1 = lib.join("niji-crt1.o");
    let flags = ["-nostdlib", "-fPIE", "-pie", "-Wl,--no-as-needed", &interp, crt1.to_str().unwrap(), &ldir, &rpath, "-l:libdep.so", libc_s, ld.to_str().unwrap(), "-lgcc"];
    let prog = cc(&d, "prog", PROG, &flags);
    Some((d, ld, prog))
}

/// Run `cmd` with only `env` in its environment.
fn run(mut cmd: Command, env: &[(&str, &str)]) -> Output {
    cmd.env_clear();
    for (k, v) in env {
        cmd.env(k, v);
    }
    // The host's `unshare` and `sh` (the file test) need a search path.
    if let Some(p) = std::env::var_os("PATH") {
        cmd.env("PATH", p);
    }
    cmd.output().unwrap()
}

/// Standard output and standard error of a run that exited 0.
fn ran(o: &Output, what: &str) -> (String, String) {
    let (out, err) = (String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned());
    assert_eq!(o.status.code(), Some(0), "{what}: {:?}; stdout: {out} stderr: {err}", o.status);
    (out, err)
}

/// The program's output with no preload.
const PLAIN: &str = "ctor dep\nctor prog\nf=2 strlen=4\n";
/// The program's output with `libpre.so` preloaded: its `f` and `strlen` interpose, its constructor
/// runs after the program's dependency's (no edge from the program to a preload).
const PRE: &str = "ctor dep\nctor pre\nctor prog\nf=1 strlen=1004\n";

/// The reference's line for a preload that cannot be loaded (DL-LD-PRELOAD probe).
fn ignored(name: &str, from: &str, reason: &str) -> String {
    format!("ERROR: ld.so: object '{name}' from {from} cannot be preloaded ({reason}): ignored.\n")
}

/// s27_1_g_0, s27_1_c_3 (global scope order; RTLD_NEXT), ld.so(8) LD_PRELOAD; the DL-LD-PRELOAD probe's
/// order and scope facts (spec/behaviour/dynlink-preload.md). A preload interposes on the program's dependency and
/// on libc, its `dlsym(RTLD_NEXT, "strlen")` reaches libc's, constructors run dependencies first
/// with later preloads after earlier ones' dependencies; names by path, bare (the program's search),
/// with `$ORIGIN`, separated by colons and spaces, repeated, or naming a dependency: loaded once.
#[test]
fn preload_interposes_and_reaches_next() {
    let Some((d, ld, prog)) = corpus("order") else { return };
    let ds = d.to_str().unwrap();
    let (pre, pre2, dep) = (format!("{ds}/libpre.so"), format!("{ds}/libpre2.so"), format!("{ds}/libdep.so"));
    let both = format!("{pre2}:  {pre}");
    let twice = format!("{pre} {pre}");
    let cases: [(&str, &str); 8] = [
        ("", PLAIN),
        (&pre, PRE),
        ("libpre.so", PRE),
        ("$ORIGIN/libpre.so", PRE),
        (&twice, PRE),
        (&both, "ctor dep\nctor pre\nctor pre2\nctor prog\nf=5 strlen=1004\n"),
        (&dep, PLAIN),
        (" : ", PLAIN),
    ];
    for (value, want) in cases {
        let o = run(Command::new(&prog), &[("LD_PRELOAD", value)]);
        let (out, err) = ran(&o, value);
        assert_eq!((out.as_str(), err.as_str()), (want, ""), "LD_PRELOAD={value:?}");
    }
    // `ld.so PROGRAM` preloads the same way.
    let mut c = Command::new(&ld);
    c.arg(&prog);
    let (out, err) = ran(&run(c, &[("LD_PRELOAD", &pre)]), "ld.so PROGRAM");
    assert_eq!((out.as_str(), err.as_str()), (PRE, ""), "ld.so PROGRAM, LD_PRELOAD={pre}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (no spec clause: ld.so(8) LD_PRELOAD; DL-LD-PRELOAD probe of the reference's texts.) An object
/// that cannot be preloaded (missing, a short non-ELF file, a directory, a truncated ELF file) gets
/// the reference's line, in order, and the program runs without it.
#[test]
fn preload_failures_are_ignored_with_their_reason() {
    let Some((d, _, prog)) = corpus("ignored") else { return };
    let ds = d.to_str().unwrap();
    std::fs::write(d.join("short"), "hello\n").unwrap();
    std::fs::create_dir(d.join("sub")).unwrap();
    std::fs::write(d.join("trunc.so"), &std::fs::read(d.join("libpre.so")).unwrap()[..100]).unwrap();
    let names = [("absent.so", "cannot open shared object file"), ("short", "file too short"), ("sub", "cannot read file data"), ("trunc.so", "cannot read file data")];
    let value = names.iter().map(|(n, _)| format!("{ds}/{n}")).collect::<Vec<_>>().join(" ");
    let (out, err) = ran(&run(Command::new(&prog), &[("LD_PRELOAD", &value)]), "ignored");
    let want: String = names.iter().map(|(n, r)| ignored(&format!("{ds}/{n}"), "LD_PRELOAD", r)).collect();
    assert_eq!((out.as_str(), err.as_str()), (PLAIN, want.as_str()));
    let _ = std::fs::remove_dir_all(&d);
}

/// s1_1_g_4 (threat model rule 1, spec/behaviour/dynlink-secure.md's table): a set-group-ID program
/// (AT_SECURE; a supplementary group, no root) skips an `LD_PRELOAD` name with `/` silently, looks
/// for a bare one only in the trusted directories (not the program's RPATH) and takes it only when
/// set-user-ID: a trusted-directory library without the bit gets the "cannot open" line.
#[test]
fn secure_preload_only_trusted_set_user_id() {
    let Some((d, _, prog)) = corpus("secure") else { return };
    let id = |a: &str| String::from_utf8(Command::new("id").arg(a).output().unwrap().stdout).unwrap();
    let primary = id("-g").trim().to_owned();
    let Some(group) = id("-G").split_whitespace().find(|g| *g != primary).map(str::to_owned) else {
        let _ = std::fs::remove_dir_all(&d);
        return eprintln!("SKIP: the user has no supplementary group (a set-group-ID file needs one without root)");
    };
    // The kernel ignores the bit on a `nosuid` file system (a tree copied under /dev/shm) and under
    // no_new_privs. The program names its libraries and interpreter by absolute path, so a copy in
    // another directory runs the same: try the corpus directory, then one under the temporary
    // directory, and keep the first where a plain run reports AT_SECURE.
    let tmp = std::env::temp_dir().join(format!("niji-ld-preload-secure-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let mut secure_prog = None;
    for dir in [d.clone(), tmp.clone()] {
        if std::fs::create_dir_all(&dir).is_err() {
            continue;
        }
        let p = dir.join("prog-sgid");
        std::fs::copy(&prog, &p).unwrap();
        assert!(Command::new("chgrp").arg(&group).arg(&p).status().unwrap().success());
        assert!(Command::new("chmod").arg("g+s").arg(&p).status().unwrap().success());
        let (out, err) = ran(&run(Command::new(&p), &[]), "secure, no preload");
        if out.strip_suffix("secure\n").is_some() {
            assert_eq!((out.as_str(), err.as_str()), (format!("{PLAIN}secure\n").as_str(), ""), "secure, no preload");
            secure_prog = Some(p);
            break;
        }
    }
    let Some(prog) = secure_prog else {
        let _ = (std::fs::remove_dir_all(&d), std::fs::remove_dir_all(&tmp));
        return eprintln!("SKIP: the kernel did not set AT_SECURE for a set-group-ID program in {} or {} (nosuid file system or no_new_privs)", d.display(), tmp.display());
    };
    // A library every trusted-directory system has, not set-user-ID: found there, refused for the bit.
    let system = ["libz.so.1", "liblzma.so.5", "libbz2.so.1.0"].into_iter().find(|n| Native::SYSTEM_DIRS.iter().any(|s| std::fs::metadata(Path::new(s).join(n)).is_ok_and(|m| m.is_file())));
    let ds = d.to_str().unwrap();
    let mut value = format!("{ds}/libpre.so libpre.so");
    let mut want = ignored("libpre.so", "LD_PRELOAD", "cannot open shared object file");
    if let Some(n) = system {
        value.push(' ');
        value.push_str(n);
        want.push_str(&ignored(n, "LD_PRELOAD", "cannot open shared object file"));
    } else {
        eprintln!("NOTE: no libz.so.1, liblzma.so.5 or libbz2.so.1.0 in a trusted directory: the set-user-ID check is not exercised here");
    }
    let o = run(Command::new(&prog), &[("LD_PRELOAD", &value)]);
    let (out, err) = ran(&o, "secure");
    assert_eq!((out, err), (format!("{PLAIN}secure\n"), want), "secure, LD_PRELOAD={value}");
    let _ = (std::fs::remove_dir_all(&d), std::fs::remove_dir_all(&tmp));
}

/// (no spec clause: ld.so(8) "/etc/ld.so.preload"; DL-LD-PRELOAD probe of the file's syntax and
/// order.) The preload file's names load after the `LD_PRELOAD` ones and before the program's
/// dependencies; spaces, tabs, newlines and colons separate, `#` comments to the end of its line
/// (glued to a name too), no final newline needed; a missing one gets the line naming the file.
#[test]
fn preload_file_after_environment_names() {
    let Some((d, _, prog)) = corpus("file") else { return };
    if !Command::new("unshare").args(["-r", "-m", "true"]).status().is_ok_and(|s| s.success()) {
        let _ = std::fs::remove_dir_all(&d);
        return eprintln!("SKIP: unprivileged user and mount namespaces are not available (unshare -r -m)");
    }
    let ds = d.to_str().unwrap();
    let etc = d.join("etc");
    std::fs::create_dir(&etc).unwrap();
    let text = format!("# {ds}/libdep.so\n{ds}/libepre.so\t\n\n/nonexistent/y.so:{ds}/libpre.so#{ds}/libpre2.so");
    std::fs::write(etc.join("ld.so.preload"), text).unwrap();
    // `exec` is the shell's own: no program of the host system starts under the private /etc. Only
    // the program gets LD_PRELOAD: `unshare`, `sh` and `mount` are the host's programs, and the
    // corpus libraries need libniji's libc.
    let pre2 = format!("{ds}/libpre2.so");
    let mut c = Command::new("unshare");
    let script = "mount --bind \"$0\" /etc && LD_PRELOAD=\"$2\" && export LD_PRELOAD && exec \"$1\"";
    c.args(["-r", "-m", "sh", "-c", script, etc.to_str().unwrap(), prog.to_str().unwrap(), &pre2]);
    let (out, err) = ran(&run(c, &[]), "preload file");
    let want = "ctor dep\nctor pre\nctor epre\nctor pre2\nctor prog\nf=5 strlen=1004\n";
    assert_eq!((out.as_str(), err.as_str()), (want, ignored("/nonexistent/y.so", "/etc/ld.so.preload", "cannot open shared object file").as_str()));
    let _ = std::fs::remove_dir_all(&d);
}
