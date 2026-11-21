//! The name every interface reports for an object, by how it was found (DL-NAME-PATH-2,
//! duskglow/libniji#24; spec s27_1_c_6 `dladdr`, s27_1_c_7 `dlinfo`, s27_1_c_8 `dl_iterate_phdr`,
//! s27_4_c_0 `_r_debug`). One C program finds an object by an address in it and compares the name
//! `dladdr`, `dl_iterate_phdr`, the `dlinfo(RTLD_DI_LINKMAP)` chain and `_r_debug`'s chain give
//! with the expected one, for a start-time library and for a `dlopen`ed one. The expected names
//! are the reference's, probed black box (handoff/DL-NAME-PATH-2.md): through a `$ORIGIN` list the
//! program's directory is the file the kernel ran (absolute, links resolved), under `ld.so
//! PROGRAM` the path as given after the working directory; a relative `LD_LIBRARY_PATH` or
//! `dlopen` path is kept as given. Needs `cargo xtask so` and a C compiler (else SKIP).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects)]

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
#include <link.h>
#include <stdio.h>
#include <string.h>
extern int names_a(void);
static unsigned long base;
static const char *ph;
static int cb(struct dl_phdr_info *i, size_t s, void *d) {
    (void)s; (void)d;
    if (i->dlpi_addr == base) ph = i->dlpi_name;
    return 0;
}
/* 0, or the first check (code + k) where the object holding `sym` is not named `want`. */
static int names(const char *what, void *sym, const char *want, int code) {
    Dl_info di;
    if (!sym || !dladdr(sym, &di)) return code + 1;
    base = (unsigned long)di.dli_fbase;
    ph = 0;
    dl_iterate_phdr(cb, 0);
    struct link_map *lm = 0, *m, *r;
    if (dlinfo(dlopen(0, RTLD_NOW), RTLD_DI_LINKMAP, &lm)) return code + 2;
    for (m = lm; m && m->l_addr != base; m = m->l_next) {}
    for (r = _r_debug.r_map; r && r->l_addr != base; r = r->l_next) {}
    printf("%s: dladdr [%s] phdr [%s] dlinfo [%s] r_debug [%s] want [%s]\n", what, di.dli_fname, ph ? ph : "-",
           m ? m->l_name : "-", r ? r->l_name : "-", want);
    if (strcmp(di.dli_fname, want)) return code + 3;
    if (!ph || strcmp(ph, want)) return code + 4;
    if (!m || strcmp(m->l_name, want)) return code + 5;
    if (!r || strcmp(r->l_name, want)) return code + 6;
    return 0;
}
/* argv[1]: the start-time library's expected name; argv[2], argv[3]: a name to dlopen and its expected name. */
int main(int argc, char **argv) {
    if (argc != 2 && argc != 4) return 99;
    int r = names("start", (void *)names_a, argv[1], 10);
    if (r) return r;
    if (argc == 4) {
        void *h = dlopen(argv[2], RTLD_NOW);
        if (!h) { printf("dlopen %s: %s\n", argv[2], dlerror()); return 30; }
        struct link_map *lm = 0;
        if (dlinfo(h, RTLD_DI_LINKMAP, &lm) || strcmp(lm->l_name, argv[3])) return 31;
        if ((r = names("dlopen", dlsym(h, "names_b"), argv[3], 40))) return r;
    }
    puts("names ok");
    return 0;
}
"#;

/// The corpus in a fresh directory `dir` (canonical): `bin/{runpath,rpath,plain}` (the first two
/// with `$ORIGIN/lib`), `bin/lib/libnames-a.so` they need by bare name, `bin/sub/libnames-b.so`,
/// `link` a symbolic link to `bin`, `alias/prog` one to `bin/runpath`; `ld` libniji's loader.
struct Corpus {
    dir: PathBuf,
    ld: PathBuf,
}

fn setup(tag: &str) -> Option<Corpus> {
    if !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no system C compiler to build the corpus");
        return None;
    }
    let lib = target().join("niji");
    if !lib.join(Native::INTERP).exists() || !lib.join("libc.so.6").exists() || !lib.join("niji-crt1.o").exists() {
        eprintln!("SKIP: ld.so, libc.so.6 or niji-crt1.o not built (run cargo xtask so)");
        return None;
    }
    let lib = std::fs::canonicalize(lib).unwrap();
    let ld = lib.join(Native::INTERP);
    let d = target().join(format!("ld-names-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for sub in ["bin/lib", "bin/sub", "alias"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
    let d = std::fs::canonicalize(d).unwrap();
    let cc = |out: &Path, src: &str, flags: &[&str]| {
        let c = d.join(format!("{}.c", out.file_name().unwrap().to_str().unwrap()));
        std::fs::write(&c, src).unwrap();
        let r = Command::new("cc").args(["-O1", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(out).output().unwrap();
        assert!(r.status.success(), "{}: {}", out.display(), String::from_utf8_lossy(&r.stderr));
    };
    // libc.so.6 without DT_SONAME, linked by its path (as rendezvous.rs does): the start opens it by
    // that path, with no installed prefix and no LD_LIBRARY_PATH.
    let mut bytes = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = niji_ld::elf::Elf::parse::<Native>(&bytes).unwrap();
    let dynph = elf.find(niji_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (usize::try_from(dynph.offset).unwrap(), usize::try_from(dynph.filesz).unwrap());
    let at = (off..off + len).step_by(16).find(|&o| bytes[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    bytes[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-names.so");
    std::fs::write(&libc, &bytes).unwrap();
    // No DT_SONAME: the program names the library `libnames-a.so`, a bare name the start searches.
    cc(&d.join("bin/lib/libnames-a.so"), "int names_a(void) { return 1; }\n", &["-nostdlib", "-fPIC", "-shared"]);
    cc(&d.join("bin/sub/libnames-b.so"), "int names_b(void) { return 2; }\n", &["-nostdlib", "-fPIC", "-shared"]);
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("niji-crt1.o");
    let libdir = d.join("bin/lib");
    for (prog, list) in [("runpath", &["-Wl,--enable-new-dtags", "-Wl,-rpath,$ORIGIN/lib"][..]), ("rpath", &["-Wl,--disable-new-dtags", "-Wl,-rpath,$ORIGIN/lib"][..]), ("plain", &[][..])] {
        let mut flags = vec!["-nostdlib", "-fPIC", "-pie", &interp, crt1.to_str().unwrap(), "-L", libdir.to_str().unwrap(), "-lnames-a"];
        flags.extend_from_slice(list);
        flags.extend_from_slice(&[libc.to_str().unwrap(), ld.to_str().unwrap(), "-lgcc"]);
        cc(&d.join("bin").join(prog), PROG, &flags);
    }
    std::os::unix::fs::symlink("bin", d.join("link")).unwrap();
    std::os::unix::fs::symlink("../bin/runpath", d.join("alias/prog")).unwrap();
    Some(Corpus { dir: d, ld })
}

impl Corpus {
    /// Run `argv` (`argv[0]` exactly as given, relative to the corpus) from the corpus directory
    /// with `env` added; its exit status and output.
    fn run(&self, env: &[(&str, &str)], argv: &[&str]) -> (i32, String) {
        let mut c = Command::new("sh");
        c.current_dir(&self.dir).arg("-c").arg("exec \"$@\"").arg("sh").args(argv);
        for (k, v) in env {
            c.env(k, v);
        }
        let o = c.output().unwrap();
        (o.status.code().unwrap_or(-1), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
    }

    fn path(&self, rel: &str) -> String {
        format!("{}/{rel}", self.dir.display())
    }

    fn expect_ok(&self, env: &[(&str, &str)], argv: &[&str]) {
        let (rc, out) = self.run(env, argv);
        assert!(rc == 0 && out.contains("names ok"), "{env:?} {argv:?}: rc {rc}\n{out}");
    }
}

// Spec s27_1_c_6, s27_1_c_7, s27_1_c_8, s27_4_c_0: a library found through the program's
// DT_RUNPATH or DT_RPATH `$ORIGIN` is named by its full path in all four interfaces, however the
// program was started: by a relative path, an absolute one, through a link to its directory or to
// its file (the directory is the file the kernel ran, links resolved).
#[test]
fn origin_library_is_named_by_its_full_path() {
    let Some(c) = setup("origin") else { return };
    let want = c.path("bin/lib/libnames-a.so");
    for prog in ["runpath", "rpath"] {
        c.expect_ok(&[], &[&format!("./bin/{prog}"), &want]);
        c.expect_ok(&[], &[&c.path(&format!("bin/{prog}")), &want]);
        c.expect_ok(&[], &[&format!("./link/{prog}"), &want]);
    }
    c.expect_ok(&[], &["./alias/prog", &want]);
    // A `$ORIGIN` in LD_LIBRARY_PATH is the program's too.
    c.expect_ok(&[("LD_LIBRARY_PATH", "$ORIGIN/lib")], &["./link/plain", &want]);
}

// Spec s27_1_c_8 (no clause on `ld.so PROGRAM`'s origin; probed): run as `ld.so PROGRAM` with a
// relative path, the program's `$ORIGIN` is the working directory, `/`, then the path's directory
// as given (no `.` or link removed).
#[test]
fn ld_so_program_origin_is_the_given_path_after_the_working_directory() {
    let Some(c) = setup("ldso") else { return };
    let ld = c.ld.to_str().unwrap().to_owned();
    c.expect_ok(&[], &[&ld, "./bin/runpath", &c.path("./bin/lib/libnames-a.so")]);
    c.expect_ok(&[], &[&ld, "./link/runpath", &c.path("./link/lib/libnames-a.so")]);
}

// Spec s27_1_c_1, s27_1_c_8: a relative LD_LIBRARY_PATH element and a relative `dlopen` path are
// kept as given in the name (probed); an absolute `dlopen` path likewise; a bare `dlopen` name found
// through LD_LIBRARY_PATH is named by the element it was found in.
#[test]
fn relative_search_and_dlopen_names_are_kept_as_given() {
    let Some(c) = setup("relative") else { return };
    c.expect_ok(&[("LD_LIBRARY_PATH", "bin/lib")], &["./bin/plain", "bin/lib/libnames-a.so"]);
    c.expect_ok(&[("LD_LIBRARY_PATH", "bin/lib/../lib")], &["./bin/plain", "bin/lib/../lib/libnames-a.so"]);
    c.expect_ok(&[("LD_LIBRARY_PATH", "bin/sub:bin/lib")], &["./bin/plain", "bin/lib/libnames-a.so", "libnames-b.so", "bin/sub/libnames-b.so"]);
    let want = c.path("bin/lib/libnames-a.so");
    c.expect_ok(&[], &["./bin/runpath", &want, "bin/sub/../sub/libnames-b.so", "bin/sub/../sub/libnames-b.so"]);
    let abs = c.path("link/sub/libnames-b.so");
    c.expect_ok(&[], &["./bin/runpath", &want, &abs, &abs]);
}

// Spec s27_1_c_8 and threat model rule 2 (spec/security/threat-model.md; probed): when the file the kernel ran cannot
// be read (`/proc` hidden), the program's `$ORIGIN` is unknown: `LD_ORIGIN_PATH` stands in, as
// given, and without it the `$ORIGIN` element is skipped.
#[test]
fn origin_without_proc_is_ld_origin_path() {
    let Some(c) = setup("noproc") else { return };
    let probe = Command::new("unshare").args(["-r", "-m", "sh", "-c", "mount -t tmpfs none /proc && test ! -e /proc/self"]).output();
    if !probe.as_ref().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no unprivileged user and mount namespace to hide /proc ({probe:?})");
        return;
    }
    let script = "mount -t tmpfs none /proc && cd \"$0\" && exec \"$@\"";
    let hidden = |env: &[&str], argv: &[&str]| {
        let o = Command::new("unshare").args(["-r", "-m", "env"]).args(env).args(["sh", "-c", script]).arg(&c.dir).args(argv).output().unwrap();
        (o.status.code().unwrap_or(-1), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
    };
    let stand_in = format!("LD_ORIGIN_PATH={}", c.path("bin/sub/.."));
    let (rc, out) = hidden(&[&stand_in], &["./link/runpath", &c.path("bin/sub/../lib/libnames-a.so")]);
    assert!(rc == 0 && out.contains("names ok"), "LD_ORIGIN_PATH: rc {rc}\n{out}");
    let (rc, out) = hidden(&[], &["./link/runpath", "-"]);
    assert!(rc == 127 && out.contains("libnames-a.so"), "no origin: rc {rc}\n{out}");
}
