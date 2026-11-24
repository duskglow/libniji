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
#[path = "../../core/tests/support/scratch.rs"]
mod scratch;
#[path = "../../core/tests/support/contain.rs"]
mod contain;
use scratch::Scratch;
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
    dir: Scratch,
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
    // Canonical base, so the directory's own path is canonical.
    let d = Scratch::dir_in(&std::fs::canonicalize(target()).unwrap(), &format!("ld-names-{tag}"));
    for sub in ["bin/lib", "bin/sub", "alias"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
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
    // contain: probe(one mount and a test, then exits)
    let probe = Command::new("unshare").args(["-r", "-m", "sh", "-c", "mount -t tmpfs none /proc && test ! -e /proc/self"]).output();
    if !probe.as_ref().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no unprivileged user and mount namespace to hide /proc ({probe:?})");
        return;
    }
    let script = "mount -t tmpfs none /proc && cd \"$0\" && exec \"$@\"";
    let hidden = |env: &[&str], argv: &[&str]| {
        // Contained (support/contain.rs).
        let o = contain::user("unshare").args(["-r", "-m", "env"]).args(env).args(["sh", "-c", script]).arg(&*c.dir).args(argv).output().unwrap();
        (o.status.code().unwrap_or(-1), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
    };
    let stand_in = format!("LD_ORIGIN_PATH={}", c.path("bin/sub/.."));
    let (rc, out) = hidden(&[&stand_in], &["./link/runpath", &c.path("bin/sub/../lib/libnames-a.so")]);
    assert!(rc == 0 && out.contains("names ok"), "LD_ORIGIN_PATH: rc {rc}\n{out}");
    let (rc, out) = hidden(&[], &["./link/runpath", "-"]);
    assert!(rc == 127 && out.contains("libnames-a.so"), "no origin: rc {rc}\n{out}");
}

/// Every object of the process, one line each: `object|dlpi_name|dli_fname|_r_debug's
/// l_name|dlinfo chain's l_name|map` (`map`: `dladdr1`'s `RTLD_DL_LINKMAP` is `_r_debug`'s node),
/// after `dlopen` of each argument; then the vDSO's symbol check (DL-OBJECT-PATHS).
const ALL: &str = r#"
#define _GNU_SOURCE
#include <dlfcn.h>
#include <elf.h>
#include <link.h>
#include <stdio.h>
#include <string.h>
#include <sys/auxv.h>
extern int all_search(void), all_slash(void);
struct row { unsigned long addr, first; const char *name; };
static struct row rows[64];
static int nrows;
static int cb(struct dl_phdr_info *i, size_t s, void *d) {
    (void)s; (void)d;
    if (nrows == 64) return 1;
    rows[nrows].addr = i->dlpi_addr; rows[nrows].name = i->dlpi_name; rows[nrows].first = 0;
    for (int k = 0; k < i->dlpi_phnum; k++)
        if (i->dlpi_phdr[k].p_type == PT_LOAD) { rows[nrows].first = i->dlpi_addr + i->dlpi_phdr[k].p_vaddr; break; }
    nrows++;
    return 0;
}
static struct link_map *by_addr(struct link_map *m, unsigned long a) {
    for (; m; m = m->l_next) if (m->l_addr == a) return m;
    return 0;
}
static int usable(const ElfW(Sym) *s) {
    return s->st_shndx && s->st_shndx != SHN_ABS && ELF64_ST_TYPE(s->st_info) != STT_TLS;
}
/* An address inside a vDSO function: its symbol is the usable one holding it with the highest
   start, at equal starts the lowest index. 0, or the failed check. */
static int vdso_symbol(struct link_map *map) {
    unsigned long base = map->l_addr, n = 0;
    ElfW(Sym) *sym = 0; const char *str = 0;
    for (ElfW(Dyn) *d = map->l_ld; d->d_tag != DT_NULL; d++) {
        if (d->d_tag == DT_SYMTAB) sym = (void *)(base + d->d_un.d_ptr);
        if (d->d_tag == DT_STRTAB) str = (void *)(base + d->d_un.d_ptr);
        if (d->d_tag == DT_HASH) n = ((unsigned int *)(base + d->d_un.d_ptr))[1];
    }
    for (unsigned long k = 1; sym && str && k < n; k++) {
        if (!usable(&sym[k]) || sym[k].st_size < 2) continue;
        unsigned long a = base + sym[k].st_value + 1, best = 0, bk = 0;
        for (unsigned long j = 1; j < n; j++) {
            unsigned long lo = base + sym[j].st_value;
            if (usable(&sym[j]) && lo <= a && a < lo + sym[j].st_size && (!bk || lo > best)) { best = lo; bk = j; }
        }
        Dl_info di; const ElfW(Sym) *se = 0;
        if (!dladdr1((void *)a, &di, (void **)&se, RTLD_DL_SYMENT)) return 60;
        printf("vdso %s+1: [%s] [%s]\n", str + sym[k].st_name, di.dli_fname, di.dli_sname ? di.dli_sname : "(null)");
        if (strcmp(di.dli_fname, "linux-vdso.so.1") || (unsigned long)di.dli_fbase != base) return 61;
        if (!di.dli_sname || strcmp(di.dli_sname, str + sym[bk].st_name) || (unsigned long)di.dli_saddr != best || se != &sym[bk]) return 62;
        return 0;
    }
    puts("vdso: no DT_HASH function symbol (symbol not checked)");
    return 0;
}
int main(int argc, char **argv) {
    if (all_search() + all_slash() != 3) return 99;
    for (int k = 1; k < argc; k++)
        if (!dlopen(argv[k], RTLD_NOW)) { printf("dlopen %s: %s\n", argv[k], dlerror()); return 30; }
    struct link_map *head = 0, *vdso = 0;
    if (dlinfo(dlopen(0, RTLD_NOW), RTLD_DI_LINKMAP, &head)) return 31;
    dl_iterate_phdr(cb, 0);
    for (int k = 0; k < nrows; k++) {
        Dl_info di; struct link_map *lm = 0;
        const char *fn = "-";
        if (rows[k].first && dladdr1((void *)rows[k].first, &di, (void **)&lm, RTLD_DL_LINKMAP)) fn = di.dli_fname;
        struct link_map *r = by_addr(_r_debug.r_map, rows[k].addr), *l = by_addr(head, rows[k].addr);
        printf("object|%s|%s|%s|%s|%s\n", rows[k].name, fn, r ? r->l_name : "-", l ? l->l_name : "-", r && lm == r ? "map" : "other");
        if (!strcmp(rows[k].name, "linux-vdso.so.1")) vdso = r;
    }
    if (getauxval(AT_SYSINFO_EHDR)) {
        if (!vdso) return 50;
        int rc = vdso_symbol(vdso);
        if (rc) return rc;
    }
    puts("names ok");
    return 0;
}
"#;

// Spec s27_1_c_6, s27_1_c_7, s27_1_c_8, s27_4_c_0, s27_1_c_1 (probed, handoff/DL-OBJECT-PATHS.md):
// every object is named alike by `dl_iterate_phdr`, `dladdr`, `_r_debug` and `dlinfo`, and
// `dladdr1`'s link map is its node. The name is the path it was opened by: a bare name the search
// found (DT_NEEDED, LD_PRELOAD, `dlopen`) as the directory searched plus the name, a name with `/`
// (DT_NEEDED, LD_PRELOAD, `dlopen`) as given, `ld.so` as the program's PT_INTERP, the vDSO
// `linux-vdso.so.1` (also for `dladdr` of an address in it, with its symbol); the program `""`,
// `dladdr`'s `argv[0]`.
#[test]
fn every_object_is_named_by_the_path_it_was_opened_by() {
    if !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: no system C compiler to build the corpus");
        return;
    }
    let lib = target().join("niji");
    if !lib.join(Native::INTERP).exists() || !lib.join("libc.so.6").exists() || !lib.join("niji-crt1.o").exists() {
        eprintln!("SKIP: ld.so, libc.so.6 or niji-crt1.o not built (run cargo xtask so)");
        return;
    }
    let lib = std::fs::canonicalize(lib).unwrap();
    let ld = lib.join(Native::INTERP);
    let d = Scratch::dir_in(&std::fs::canonicalize(target()).unwrap(), "ld-names-all");
    let p = |rel: &str| format!("{}/{rel}", d.display());
    for sub in ["libs", "slash", "pre", "dl", "bin"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
    let cc = |out: &str, src: &str, flags: &[&str]| {
        let c = d.join(format!("{}.c", out.replace('/', "_")));
        std::fs::write(&c, src).unwrap();
        let r = Command::new("cc").current_dir(&d).args(["-O1", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(out).output().unwrap();
        assert!(r.status.success(), "{out}: {}", String::from_utf8_lossy(&r.stderr));
    };
    let shared = ["-nostdlib", "-fPIC", "-shared"];
    cc("libs/liball-search.so", "int all_search(void) { return 1; }\n", &shared);
    cc("slash/liball-slash.so", "int all_slash(void) { return 2; }\n", &shared);
    for so in ["pre/libpa.so", "pre/libpb.so", "libs/libpc.so", "dl/libdl1.so", "dl/libdl2.so", "dl/libdl3.so"] {
        cc(so, "int all_other(void) { return 3; }\n", &shared);
    }
    // DT_NEEDED: `liball-search.so` (bare), `slash/liball-slash.so` (no DT_SONAME: the path as
    // linked), `libc.so.6` and `ld.so`'s DT_SONAME (bare); PT_INTERP the loader's path.
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("niji-crt1.o");
    let libdir = lib.to_str().unwrap();
    cc("bin/all", ALL, &["-nostdlib", "-fPIC", "-pie", "-Wl,--no-as-needed", &interp, crt1.to_str().unwrap(), "-L", "libs", "-lall-search", "slash/liball-slash.so", "-L", libdir, "-l:libc.so.6", ld.to_str().unwrap(), "-lgcc"]);
    // The variables are set inside the shell, which would load libniji's libc.so.6 itself.
    let script = "export LD_LIBRARY_PATH=\"$1\" LD_PRELOAD=\"$2\"; shift 2; exec \"$@\"";
    let (lp, pre) = (format!("{}:{}:{libdir}", p("libs"), p("dl")), format!("{} pre/libpb.so libpc.so", p("pre/libpa.so")));
    let o = Command::new("sh").current_dir(&d).args(["-c", script, "sh", &lp, &pre, "./bin/all", "libdl1.so", "./dl/libdl2.so", &p("dl/libdl3.so")]).output().unwrap();
    let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.code() == Some(0) && out.contains("names ok"), "rc {:?}\n{out}", o.status.code());
    let rows: Vec<Vec<&str>> = out.lines().filter_map(|l| l.strip_prefix("object|")).map(|l| l.split('|').collect()).collect();
    // The program: `""` in the lists, `argv[0]` from `dladdr`.
    assert!(rows.first().is_some_and(|r| r[..] == ["", "./bin/all", "", "", "map"]), "program row\n{out}");
    for r in &rows[1..] {
        assert!(r.len() == 5 && r[1..4].iter().all(|n| *n == r[0]) && r[4] == "map", "an object named differently by the interfaces: {r:?}\n{out}");
    }
    let mut want = vec![p("pre/libpa.so"), "pre/libpb.so".into(), p("libs/libpc.so"), p("libs/liball-search.so"), "slash/liball-slash.so".into()];
    want.extend([format!("{libdir}/libc.so.6"), ld.display().to_string(), p("dl/libdl1.so"), "./dl/libdl2.so".into(), p("dl/libdl3.so")]);
    // The vDSO's row (the program fails without one when the kernel gave one) agrees like any other.
    for w in &want {
        assert!(rows.iter().any(|r| r[0] == w.as_str()), "no object named {w}\n{out}");
    }
}
