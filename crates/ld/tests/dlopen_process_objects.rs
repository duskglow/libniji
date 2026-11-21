//! No fixed room for the paths and dependency lists of the objects a process holds
//! (DL-PROCESS-OBJECTS part 1, audit HL-5; dlopen(3) states no limit on either). One `dlopen` of
//! `libroot.so` adds `N` + 1 objects found in a directory whose path is over 1000 bytes long, so
//! their paths take well over the 128 KiB the path arena once held (its spans land in later
//! generations); each library `k` needs every library after it, so the dependency lists hold about
//! `N^2 / 2` entries, well over the 16 Ki the arena once held. `dladdr` (`dli_fname`),
//! `dl_iterate_phdr` (`dlpi_name`) and the link-map chain (`l_name`) must give each object's full
//! path; `dlclose` and a second load of the same set (spans written again) must do the same.
//!
//! Skips (with the reason) when `cc` is absent or the products are not built.
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use niji_ld::arch::{Arch, Native};
use niji_ld::elf::Elf;

/// Libraries the root needs: with paths of over 1000 bytes they take more than twice the old
/// 128 KiB, and `N (N - 1) / 2` dependency entries are more than twice the old 16 Ki.
const N: usize = 300;
/// What `froot` returns: every `f<k>` returns `k`.
const SUM: usize = N * (N + 1) / 2;

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// The command that builds `out` from `src` with `flags` in `d`.
fn cc(d: &Path, out: &str, src: &str, flags: &[String]) -> Command {
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

/// The program: `argv[1]` is the corpus directory. Two rounds of `dlopen(libroot.so)`, the checks,
/// and `dlclose`; status 1 with `open: <dlerror>` when a `dlopen` fails.
const PROG: &str = r#"#define _GNU_SOURCE
#include <dlfcn.h>
#include <link.h>
#include <stdio.h>
#include <string.h>
static const char *dir;
static size_t dir_len;
static int listed;
static int in_dir(const char *p) { return p && strncmp(p, dir, dir_len) == 0 && p[dir_len] == '/'; }
static int count(struct dl_phdr_info *i, size_t size, void *data) {
    (void)size; (void)data;
    listed += in_dir(i->dlpi_name);
    return 0;
}
static int check(void *h, const char *round) {
    static char path[8192];
    int (*root)(void) = (int (*)(void))dlsym(h, "froot");
    if (!root) { printf("%s: no froot: %s\n", round, dlerror()); return 1; }
    printf("%s sum %d\n", round, root());
    int named = 0;
    for (int k = 1; k <= N; k++) {
        char name[32];
        snprintf(name, sizeof name, "f%d", k);
        snprintf(path, sizeof path, "%s/libp%d.so", dir, k);
        Dl_info info;
        void *f = dlsym(h, name);
        if (f && dladdr(f, &info) && info.dli_fname && strcmp(info.dli_fname, path) == 0) named++;
    }
    printf("%s dladdr %d\n", round, named);
    listed = 0;
    dl_iterate_phdr(count, 0);
    printf("%s phdr %d\n", round, listed);
    int maps = 0;
    struct link_map *m = (struct link_map *)h;
    while (m->l_prev) m = m->l_prev;
    for (; m; m = m->l_next) maps += in_dir(m->l_name);
    printf("%s lmap %d\n", round, maps);
    return 0;
}
int main(int argc, char **argv) {
    static char root[8192];
    if (argc < 2) return 2;
    dir = argv[1];
    dir_len = strlen(dir);
    snprintf(root, sizeof root, "%s/libroot.so", dir);
    for (int round = 0; round < 2; round++) {
        void *h = dlopen(root, RTLD_NOW);
        if (!h) { printf("open: %s\n", dlerror()); return 1; }
        if (check(h, round ? "again" : "first")) return 1;
        if (dlclose(h)) { printf("close: %s\n", dlerror()); return 1; }
    }
    return 0;
}
"#;

/// The corpus in a new scratch directory of its own (`tag`: each test builds its own): (the
/// long-named library directory, the program), or `None` with the reason printed.
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
    let top = target().join(format!("ld-process-objects-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&top);
    // Five components of 200 bytes: every library path is over 1000 bytes.
    let d = ["a", "b", "c", "d", "e"].iter().fold(top.clone(), |p, c| p.join(c.repeat(200)));
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
    let libc_s = libc.to_str().unwrap().to_string();
    let shared = |name: &str| -> Vec<String> {
        let mut f: Vec<String> = ["-nostdlib", "-fPIC", "-shared", "-Wl,--no-as-needed"].map(String::from).to_vec();
        f.extend([format!("-Wl,-soname,{name}"), format!("-Wl,-rpath,{}", d.display()), format!("-L{}", stub.display())]);
        f
    };
    // Stubs first (empty, the right soname), so each library links its dependencies by name
    // whatever order the libraries are built in.
    let stubs = (1..=N).map(|k| format!("libp{k}.so")).map(|name| {
        let flags = ["-nostdlib", "-fPIC", "-shared"].map(String::from).into_iter().chain([format!("-Wl,-soname,{name}")]).collect::<Vec<_>>();
        (name.clone(), cc(&stub, &name, "int stub_unused;\n", &flags))
    });
    build_all(stubs.collect());
    let mut jobs = Vec::new();
    for k in 1..=N {
        let name = format!("libp{k}.so");
        let mut flags = shared(&name);
        flags.extend((k + 1..=N).map(|j| format!("-l:libp{j}.so")));
        jobs.push((name.clone(), cc(&d, &name, &format!("int f{k}(void) {{ return {k}; }}\n"), &flags)));
    }
    let mut flags = shared("libroot.so");
    flags.extend((1..=N).map(|k| format!("-l:libp{k}.so")));
    flags.push(libc_s.clone());
    let decls: String = (1..=N).map(|k| format!("int f{k}(void);\n")).collect();
    let body: String = (1..=N).map(|k| format!(" + f{k}()")).collect();
    jobs.push(("libroot.so".to_string(), cc(&d, "libroot.so", &format!("{decls}int froot(void) {{ return 0{body}; }}\n"), &flags)));
    build_all(jobs);
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("niji-crt1.o").to_str().unwrap().to_string();
    let flags = ["-nostdlib", "-fPIE", "-pie", &format!("-DN={N}"), &interp, &crt1, &libc_s, ld.to_str().unwrap(), "-lgcc"].map(String::from).to_vec();
    build_all(vec![("prog".to_string(), cc(&top, "prog", PROG, &flags))]);
    Some((d, top.join("prog")))
}

/// Run `cmd` with an empty environment.
fn run(cmd: &mut Command) -> Output {
    cmd.env_clear();
    if let Some(p) = std::env::var_os("PATH") {
        cmd.env("PATH", p);
    }
    cmd.output().unwrap()
}

fn top_of(d: &Path) -> PathBuf {
    d.ancestors().nth(5).unwrap().to_path_buf()
}

/// s27_1_c_1 (`dlopen` loads an object and its dependencies): one call adds `N` + 1 objects whose
/// paths and dependency lists pass the arenas' old room several times over; every object reports
/// its full path (`dladdr`, `dl_iterate_phdr`, `l_name`), and closing and opening the set again
/// works the same.
#[test]
fn dlopen_process_objects_paths_and_dependencies_past_the_old_arenas() {
    let Some((d, prog)) = corpus("arenas") else { return };
    let path_bytes: usize = (1..=N).map(|k| d.join(format!("libp{k}.so")).as_os_str().len() + 1).sum();
    assert!(path_bytes > 2 * 128 * 1024 && N * (N - 1) / 2 > 2 * 16 * 1024, "the corpus passes both old arenas twice");
    let o = run(Command::new(&prog).arg(&d));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success(), "the program failed: {:?}\n{out}{}", o.status, String::from_utf8_lossy(&o.stderr));
    // Listed in the directory: the `N` libraries, the root, and the C library the corpus names by path
    // (`libc-dl.so`, in the same directory).
    let want: String = ["first", "again"].iter().map(|r| format!("{r} sum {SUM}\n{r} dladdr {N}\n{r} phdr {}\n{r} lmap {}\n", N + 2, N + 2)).collect();
    assert_eq!(out, want, "every object's full path, in both loads");
    std::fs::remove_dir_all(top_of(&d)).ok();
}

/// no spec clause (the arenas grow, so the only limit is memory): under a sweep of address-space
/// limits, from the first one at which the program runs at all, each run either completes or fails
/// cleanly: `dlopen` returns NULL with a `dlerror` text (status 1) or the start ends with the
/// loader's message (127); never a signal.
#[test]
fn dlopen_process_objects_out_of_memory_is_clean() {
    let Some((d, prog)) = corpus("oom") else { return };
    let (mut ran, mut refused) = (0, 0);
    let mut reasons = std::collections::BTreeSet::new();
    for kb in (2048..=96 * 1024).step_by(2048) {
        let o = run(Command::new("sh").arg("-c").arg(format!("ulimit -v {kb}; exec \"$0\" \"$1\"")).arg(&prog).arg(&d));
        let (out, err) = (String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned());
        let started = out.lines().next().is_some() || err.starts_with("ld.so:") || err.contains("error while loading shared libraries");
        if ran + refused == 0 && !o.status.success() && !started {
            continue; // below the kernel's own need: the loader never ran
        }
        if o.status.success() {
            ran += 1;
            continue;
        }
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
    eprintln!("dlopen under limits: {refused} refused ({reasons:?}), {ran} ran");
    std::fs::remove_dir_all(top_of(&d)).ok();
}
