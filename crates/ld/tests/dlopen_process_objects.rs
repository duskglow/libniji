//! No fixed room for the paths and dependency lists of the objects a process holds
//! (DL-PROCESS-OBJECTS part 1, audit HL-5; dlopen(3) states no limit on either). One `dlopen` of
//! `libroot.so` adds `N` + 1 objects found in a directory whose path is over 1000 bytes long, so
//! their paths take well over the 128 KiB the path arena once held (its spans land in later
//! generations); each library `k` needs every library after it, so the dependency lists hold about
//! `N^2 / 2` entries, well over the 16 Ki the arena once held. `dladdr` (`dli_fname`),
//! `dl_iterate_phdr` (`dlpi_name`) and the link-map chain (`l_name`) must give each object's full
//! path; `dlclose` and a second load of the same set (spans written again) must do the same.
//! Part 2 (the object tables): 2000 objects at once, past the tables' first generation of 512
//! (`many_corpus`), found, listed, closed, reused and finalised at exit.
//!
//! Skips (with the reason) when `cc` is absent or the products are not built.
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::expect_used, clippy::panic)]

#[path = "../../core/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;
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

/// The built products (`target/niji`, its `ld.so`), or `None` with the reason printed.
fn products() -> Option<(PathBuf, PathBuf)> {
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
    Some((lib, ld))
}

/// libc named by path in `d` (its DT_SONAME tag made inert), as the other loader corpora name it.
fn named_libc(lib: &Path, d: &Path) -> String {
    let mut so = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = Elf::parse::<Native>(&so).unwrap();
    let dynph = elf.find(niji_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (dynph.offset as usize, dynph.filesz as usize);
    let at = (off..off + len).step_by(16).find(|&o| so[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    so[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-dl.so");
    std::fs::write(&libc, &so).unwrap();
    libc.to_str().unwrap().to_string()
}

/// The flags that link a program of the corpus in `d` against the products.
fn prog_flags(lib: &Path, ld: &Path, libc: &str, extra: &[&str]) -> Vec<String> {
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("niji-crt1.o").to_str().unwrap().to_string();
    let mut f: Vec<String> = ["-nostdlib", "-fPIE", "-pie"].map(String::from).to_vec();
    f.extend(extra.iter().map(|s| s.to_string()));
    f.extend([interp, crt1, libc.to_string(), ld.to_str().unwrap().to_string(), "-lgcc".to_string()]);
    f
}

/// The corpus in a new scratch directory of its own (`tag`: each test builds its own): (the
/// long-named library directory, the program), or `None` with the reason printed. The scratch
/// guard is removed when the test passes and kept (path on stderr) when it fails.
fn corpus(tag: &str) -> Option<(Scratch, PathBuf, PathBuf)> {
    let (lib, ld) = products()?;
    let top = Scratch::dir_in(&std::fs::canonicalize(target()).unwrap(), &format!("ld-process-objects-{tag}"));
    // Five components of 200 bytes: every library path is over 1000 bytes.
    let d = ["a", "b", "c", "d", "e"].iter().fold(top.to_path_buf(), |p, c| p.join(c.repeat(200)));
    std::fs::create_dir_all(d.join("stub")).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    let stub = d.join("stub");
    let libc_s = named_libc(&lib, &d);
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
    let flags = prog_flags(&lib, &ld, &libc_s, &[&format!("-DN={N}")]);
    build_all(vec![("prog".to_string(), cc(&top, "prog", PROG, &flags))]);
    let prog = top.join("prog");
    Some((top, d, prog))
}

/// Run `cmd` with an empty environment.
fn run(cmd: &mut Command) -> Output {
    cmd.env_clear();
    if let Some(p) = std::env::var_os("PATH") {
        cmd.env("PATH", p);
    }
    cmd.output().unwrap()
}

/// s27_1_c_1 (`dlopen` loads an object and its dependencies): one call adds `N` + 1 objects whose
/// paths and dependency lists pass the arenas' old room several times over; every object reports
/// its full path (`dladdr`, `dl_iterate_phdr`, `l_name`), and closing and opening the set again
/// works the same.
#[test]
fn dlopen_process_objects_paths_and_dependencies_past_the_old_arenas() {
    let Some((_top, d, prog)) = corpus("arenas") else { return };
    let path_bytes: usize = (1..=N).map(|k| d.join(format!("libp{k}.so")).as_os_str().len() + 1).sum();
    assert!(path_bytes > 2 * 128 * 1024 && N * (N - 1) / 2 > 2 * 16 * 1024, "the corpus passes both old arenas twice");
    let o = run(Command::new(&prog).arg(&d));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success(), "the program failed: {:?}\n{out}{}", o.status, String::from_utf8_lossy(&o.stderr));
    // Listed in the directory: the `N` libraries, the root, and the C library the corpus names by path
    // (`libc-dl.so`, in the same directory).
    let want: String = ["first", "again"].iter().map(|r| format!("{r} sum {SUM}\n{r} dladdr {N}\n{r} phdr {}\n{r} lmap {}\n", N + 2, N + 2)).collect();
    assert_eq!(out, want, "every object's full path, in both loads");
}

/// no spec clause (the arenas grow, so the only limit is memory): under a sweep of address-space
/// limits, from the first one at which the program runs at all, each run either completes or fails
/// cleanly: `dlopen` returns NULL with a `dlerror` text (status 1) or the start ends with the
/// loader's message (127); never a signal. Every limit refused is swept; the sweep ends once two
/// runs in a row complete (a larger limit only adds room no growth path lacks).
#[test]
fn dlopen_process_objects_out_of_memory_is_clean() {
    let Some((_top, d, prog)) = corpus("oom") else { return };
    let (mut ran, mut refused, mut in_row) = (0, 0, 0);
    let mut reasons = std::collections::BTreeSet::new();
    for kb in (2048..=96 * 1024).step_by(2048) {
        if in_row == 2 {
            break;
        }
        let o = run(Command::new("sh").arg("-c").arg(format!("ulimit -v {kb}; exec \"$0\" \"$1\"")).arg(&prog).arg(&d));
        let (out, err) = (String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned());
        let started = out.lines().next().is_some() || err.starts_with("ld.so:") || err.contains("error while loading shared libraries");
        if ran + refused == 0 && !o.status.success() && !started {
            continue; // below the kernel's own need: the loader never ran
        }
        if o.status.success() {
            ran += 1;
            in_row += 1;
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
    eprintln!("dlopen under limits: {refused} refused ({reasons:?}), {ran} ran");
}

/// Each library of the object-table corpus (DL-PROCESS-OBJECTS part 2): `KIND` `t` (a top, needing
/// `libbase.so`), `b` (a base), `q` (one of the hub's dependencies) or `h` (the hub). Its
/// constructor registers its own marker with the program, its destructor reports it.
const MANY_LIB: &str = r#"
void reg(const void *, int);
void note(const void *);
static char m;
#ifdef BASE
const void *fb(void) { return &m; }
#endif
#ifdef TOP
const void *fb(void);
const void *ft(void) { return fb(); }
#endif
__attribute__((constructor)) static void ini(void) { reg(&m, KIND); }
__attribute__((destructor)) static void fin(void) { note(&m); }
"#;

/// The object-table program: `argv[1]` holds `p<k>/` (1500 copies of a top and its base, each
/// its own pair of files) and `h/` (the hub and its 600 dependencies). Every line it prints is the
/// same under any C library; `reused` (on stderr) is libniji's slot reuse. Status 1 with
/// `open: <dlerror>` or `close: <dlerror>` when a call fails.
const MANY: &str = r#"#define _GNU_SOURCE
#include <dlfcn.h>
#include <link.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#define PAIRS 1000
#define MORE 500
#define Q 600
#define DIRS (PAIRS + MORE)
#define MAXO (2 * DIRS + Q + 1)
static char line[512];
#define SAY(fd, ...) do { int n_ = snprintf(line, sizeof line, __VA_ARGS__); if (write(fd, line, (size_t)n_) != n_) _exit(3); } while (0)
static const char *dir;
static size_t dir_len;
/* Every object a constructor registered: its marker, kind, whether it is still loaded. */
static const void *who[MAXO];
static char kind[MAXO];
static int live[MAXO], lab[MAXO], nwho, at_exit;
static int top_id[DIRS], base_id[DIRS], q_id[Q + 1], hub_id = -1;
static void *handle[DIRS];
/* The finalisation sequence (object ids). */
static int seq[MAXO], nseq;
void reg(const void *m, int k) {
    if (nwho == MAXO) _exit(4);
    who[nwho] = m; kind[nwho] = (char)k; live[nwho] = 1; nwho++;
}
void note(const void *m) {
    int id = nwho - 1;
    while (id >= 0 && !(live[id] && who[id] == m)) id--;
    if (id < 0) { SAY(1, "fini of an unknown object\n"); return; }
    live[id] = 0;
    seq[nseq++] = id;
    if (at_exit) SAY(1, "x %c %d\n", kind[id], lab[id]);
}
/* The object id a path names; -2 for one outside the corpus, -1 for no corpus object. */
static int parse(const char *p) {
    char *e;
    if (!p || strncmp(p, dir, dir_len) != 0 || p[dir_len] != '/') return -2;
    p += dir_len + 1;
    if (p[0] == 'p') {
        long k = strtol(p + 1, &e, 10);
        if (e == p + 1 || k < 0 || k >= DIRS) return -1;
        if (!strcmp(e, "/libtop.so")) return top_id[k];
        if (!strncmp(e, "/libbase", 8)) return base_id[k];
        return -1;
    }
    if (!strcmp(p, "h/libhub.so")) return hub_id;
    if (!strncmp(p, "h/libq", 6)) {
        long j = strtol(p + 6, &e, 10);
        if (e != p + 6 && j >= 1 && j <= Q && !strcmp(e, ".so")) return q_id[j];
    }
    return -1;
}
static int seen[MAXO];
static int walk(struct dl_phdr_info *i, size_t size, void *data) {
    (void)size;
    int id = parse(i->dlpi_name);
    if (id != -2 && (id < 0 || !live[id] || seen[id]++)) ++*(int *)data;
    return 0;
}
/* Every loaded corpus object found by its marker (dladdr, _dl_find_object), listed once by
   dl_iterate_phdr and once on the link-map chain, by its own path; nothing unloaded is. */
static void check(const char *round) {
    int nlive = 0, a = 0, f = 0, ph = 0, lm = 0, bad = 0;
    for (int i = 0; i < nwho; i++) {
        if (!live[i]) continue;
        nlive++;
        Dl_info info;
        if (dladdr(who[i], &info) && parse(info.dli_fname) == i) a++;
        struct dl_find_object o;
        if (_dl_find_object((void *)who[i], &o) == 0 && o.dlfo_map_start <= who[i] && who[i] < o.dlfo_map_end && o.dlfo_link_map && parse(o.dlfo_link_map->l_name) == i) f++;
    }
    memset(seen, 0, sizeof seen);
    dl_iterate_phdr(walk, &bad);
    for (int i = 0; i < nwho; i++) ph += live[i] && seen[i] == 1;
    memset(seen, 0, sizeof seen);
    struct dl_find_object o;
    if (_dl_find_object((void *)check, &o) != 0 || !o.dlfo_link_map) { SAY(1, "%s: the program is in no object\n", round); return; }
    struct link_map *m = o.dlfo_link_map;
    while (m->l_prev) m = m->l_prev;
    for (; m; m = m->l_next) {
        int id = parse(m->l_name);
        if (id != -2 && (id < 0 || !live[id] || seen[id]++)) bad++;
    }
    for (int i = 0; i < nwho; i++) lm += live[i] && seen[i] == 1;
    SAY(1, "%s: live %d dladdr %d find %d phdr %d lmap %d stray %d\n", round, nlive, a, f, ph, lm, bad);
}
static int open_pair(int k, int *bound, int *ordered) {
    static char path[4096];
    int s = nwho;
    snprintf(path, sizeof path, "%s/p%d/libtop.so", dir, k);
    void *h = dlopen(path, RTLD_NOW);
    if (!h) { SAY(1, "open: %s\n", dlerror()); return 1; }
    handle[k] = h;
    if (nwho - s == 2 && kind[s] == 'b' && kind[s + 1] == 't') {
        base_id[k] = s; top_id[k] = s + 1; lab[s] = lab[s + 1] = k; ++*ordered;
    }
    const void *(*ft)(void) = (const void *(*)(void))dlsym(h, "ft");
    *bound += ft && base_id[k] >= 0 && ft() == who[base_id[k]];
    return 0;
}
static void *map_of(const void *m) {
    struct dl_find_object o;
    return _dl_find_object((void *)m, &o) == 0 ? (void *)o.dlfo_link_map : NULL;
}
int main(int argc, char **argv) {
    static char path[4096];
    static void *old[PAIRS];
    if (argc < 2) return 2;
    dir = argv[1];
    dir_len = strlen(dir);
    for (int k = 0; k < DIRS; k++) top_id[k] = base_id[k] = -1;
    for (int j = 0; j <= Q; j++) q_id[j] = -1;
    int bound = 0, ordered = 0;
    for (int k = 0; k < PAIRS; k++) if (open_pair(k, &bound, &ordered)) return 1;
    SAY(1, "opened %d: bound %d dependency first %d\n", PAIRS, bound, ordered);
    check("first");
    /* Close every odd pair: the top's finaliser, then its base's. */
    int fin = 0, dep_first = 0, nold = 0;
    for (int k = 1; k < PAIRS; k += 2) {
        if (top_id[k] >= 0) { old[nold++] = handle[k]; old[nold++] = map_of(who[base_id[k]]); }
        int s = nseq;
        if (dlclose(handle[k])) { SAY(1, "close: %s\n", dlerror()); return 1; }
        fin += nseq - s;
        dep_first += nseq - s == 2 && seq[s] == top_id[k] && seq[s + 1] == base_id[k];
    }
    SAY(1, "closed %d: finalised %d dependent first %d\n", PAIRS / 2, fin, dep_first);
    check("closed");
    bound = ordered = 0;
    for (int k = PAIRS; k < DIRS; k++) if (open_pair(k, &bound, &ordered)) return 1;
    SAY(1, "opened %d more: bound %d dependency first %d\n", MORE, bound, ordered);
    int reused = 0;
    for (int k = PAIRS; k < DIRS; k++) {
        void *mine[2] = { handle[k], base_id[k] >= 0 ? map_of(who[base_id[k]]) : NULL };
        for (int x = 0; x < 2; x++) for (int i = 0; i < nold; i++) if (mine[x] && mine[x] == old[i]) { reused++; break; }
    }
    SAY(2, "reused %d of %d\n", reused, nold);
    check("reopened");
    /* The hub: one call adds 601 objects, one dlclose removes them. */
    int s0 = nwho;
    snprintf(path, sizeof path, "%s/h/libhub.so", dir);
    void *hub = dlopen(path, RTLD_NOW);
    if (!hub) { SAY(1, "open: %s\n", dlerror()); return 1; }
    int hub_last = nwho - s0 == Q + 1 && kind[nwho - 1] == 'h', named = 0;
    if (hub_last) hub_id = nwho - 1;
    for (int i = s0; i < nwho; i++) {
        Dl_info info;
        char *e;
        if (kind[i] != 'q' || !dladdr(who[i], &info) || !info.dli_fname) continue;
        const char *p = info.dli_fname;
        if (strncmp(p, dir, dir_len) != 0 || strncmp(p + dir_len, "/h/libq", 7) != 0) continue;
        long j = strtol(p + dir_len + 7, &e, 10);
        if (j >= 1 && j <= Q && !strcmp(e, ".so") && q_id[j] < 0) { q_id[j] = i; lab[i] = (int)j; named++; }
    }
    SAY(1, "hub: objects %d named %d hub last %d\n", nwho - s0, named, hub_last);
    check("hub");
    int s = nseq;
    if (dlclose(hub)) { SAY(1, "close: %s\n", dlerror()); return 1; }
    int order = nseq - s == Q + 1;
    for (int k = 0; order && k <= Q; k++) order = seq[s + k] == nwho - 1 - k;
    SAY(1, "hub closed: finalised %d dependents first, in reverse %d\n", nseq - s, order);
    check("hub closed");
    SAY(1, "exit\n");
    at_exit = 1;
    return 0;
}
"#;

/// The object-table corpus in a new scratch directory: (the object directory, the program), or
/// `None` with the reason printed. One top, base and hub dependency are built, then copied (each
/// copy its own file, so its own object). The scratch guard is removed when the test passes and
/// kept (path on stderr) when it fails.
fn many_corpus() -> Option<(Scratch, PathBuf, PathBuf)> {
    let (lib, ld) = products()?;
    let guard = Scratch::dir_in(&std::fs::canonicalize(target()).unwrap(), "ld-process-objects-many");
    std::fs::create_dir_all(guard.join("o/h")).unwrap();
    std::fs::create_dir_all(guard.join("build")).unwrap();
    let top = guard.to_path_buf();
    let (d, b, h) = (top.join("o"), top.join("build"), top.join("o/h"));
    let libc = named_libc(&lib, &top);
    let flags = |more: &[&str]| -> Vec<String> { ["-nostdlib", "-fPIC", "-shared"].iter().chain(more).map(|s| s.to_string()).collect() };
    build_all(vec![
        ("libbase0000.so".into(), cc(&b, "libbase0000.so", MANY_LIB, &flags(&["-DBASE", "-DKIND='b'", "-Wl,-soname,libbase0000.so"]))),
        ("libq.so".into(), cc(&b, "libq.so", MANY_LIB, &flags(&["-DKIND='q'"]))),
    ]);
    let base = b.join("libbase0000.so");
    build_all(vec![("libtop.so".into(), cc(&b, "libtop.so", MANY_LIB, &flags(&["-DTOP", "-DKIND='t'", "-Wl,--no-as-needed", "-Wl,-rpath,$ORIGIN", base.to_str().unwrap()])))]);
    // Pair k's base is `libbase<k>.so` (4 digits), named so in its soname and its top's DT_NEEDED: a
    // needed name matching a loaded object's soname is that object, so one shared name would make
    // every top share the first base.
    let (top_so, base_so) = (std::fs::read(b.join("libtop.so")).unwrap(), std::fs::read(&base).unwrap());
    let named = |so: &[u8], k: usize| -> Vec<u8> {
        let (from, to) = (b"libbase0000.so", format!("libbase{k:04}.so"));
        let mut out = so.to_vec();
        let at: Vec<usize> = so.windows(from.len()).enumerate().filter(|(_, w)| w == from).map(|(i, _)| i).collect();
        assert!(!at.is_empty(), "the name to patch is in the object");
        for i in at {
            out[i..i + from.len()].copy_from_slice(to.as_bytes());
        }
        out
    };
    for k in 0..1500 {
        let p = d.join(format!("p{k}"));
        std::fs::create_dir(&p).unwrap();
        std::fs::write(p.join("libtop.so"), named(&top_so, k)).unwrap();
        std::fs::write(p.join(format!("libbase{k:04}.so")), named(&base_so, k)).unwrap();
    }
    for j in 1..=600 {
        std::fs::copy(b.join("libq.so"), h.join(format!("libq{j}.so"))).unwrap();
    }
    let mut hub = flags(&["-DKIND='h'", "-Wl,--no-as-needed", "-Wl,-rpath,$ORIGIN", &format!("-L{}", h.display())]);
    hub.extend((1..=600).map(|j| format!("-l:libq{j}.so")));
    build_all(vec![("libhub.so".into(), cc(&b, "libhub.so", MANY_LIB, &hub))]);
    std::fs::copy(b.join("libhub.so"), h.join("libhub.so")).unwrap();
    let flags = prog_flags(&lib, &ld, &libc, &["-Wl,--export-dynamic"]);
    build_all(vec![("prog".into(), cc(&top, "prog", MANY, &flags))]);
    Some((guard, d, top.join("prog")))
}

/// s27_1_c_1, s27_1_c_4, s27_1_c_6, s27_1_c_8, s27_1_c_9 (dlopen(3) states no limit on the objects
/// a process holds): 2000 objects held at once, four times the tables' first generation of 512.
/// Each is found by an address inside it (`dladdr`, `_dl_find_object`), listed once by
/// `dl_iterate_phdr` and once on the link-map chain, each by its own path, and binds to its own
/// dependency; its constructor runs after its dependency's. Closing every other pair runs the
/// top's finaliser before its base's and leaves no closed object listed or found; 500 more pairs
/// load, the first in the freed slots at the top (libniji). One `dlopen` of a hub adds 601 objects and its `dlclose`
/// finalises them all, the hub first, then in reverse constructor order. At exit the 2000 objects
/// left are finalised once each, in load order with each top before its base (DL-EXIT-ORDER-PROBE),
/// as under the reference (probe of this program, DL-PROCESS-OBJECTS-2 handoff).
#[test]
fn dlopen_process_objects_past_the_old_object_tables() {
    let Some((_top, d, prog)) = many_corpus() else { return };
    let o = run(Command::new(&prog).arg(&d));
    let (out, err) = (String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success(), "the program failed: {:?}\n{out}{err}", o.status);
    let all = |r: &str, n: usize| format!("{r}: live {n} dladdr {n} find {n} phdr {n} lmap {n} stray 0\n");
    let want = [
        "opened 1000: bound 1000 dependency first 1000\n".to_string(),
        all("first", 2000),
        "closed 500: finalised 1000 dependent first 500\n".to_string(),
        all("closed", 1000),
        "opened 500 more: bound 500 dependency first 500\n".to_string(),
        all("reopened", 2000),
        "hub: objects 601 named 600 hub last 1\n".to_string(),
        all("hub", 2601),
        "hub closed: finalised 601 dependents first, in reverse 1\n".to_string(),
        all("hub closed", 2000),
    ]
    .concat();
    let (head, exit) = out.split_once("exit\n").unwrap_or((&out, ""));
    assert_eq!(head, want, "the object tables past 512 objects");
    // The pairs left in load order (the even pairs below 1000, then 1000..1500; DL-EXIT-ORDER-PROBE),
    // each top before the base it needs, whatever slot each took.
    let want_exit: Vec<String> = (0..1000).step_by(2).chain(1000..1500).flat_map(|k| [format!("x t {k}"), format!("x b {k}")]).collect();
    let got: Vec<&str> = exit.lines().collect();
    let first = got.iter().zip(&want_exit).position(|(g, w)| g != w);
    assert!(got.len() == want_exit.len() && first.is_none(), "exit finalisers: {} of {}, first difference at {first:?}: {:?}", got.len(), want_exit.len(), first.map(|i| (got[i], &want_exit[i])));
    // libniji reuses the freed slots at the top of the tables (the D-27 dlclose record: a new set
    // starts at the top): pair 999, closed last, held the top two, and pair 1000 takes them. The
    // holes below the top stay unused (DL-PROCESS-OBJECTS-2 handoff, a later story's).
    let reused: usize = err.split("reused ").nth(1).and_then(|s| s.split(' ').next()).and_then(|s| s.parse().ok()).unwrap_or(0);
    assert!(reused >= 2, "slot reuse at the top: {err}");
}
