//! Threads with TLS in dynamically linked programs (phase 14 DL-15 `DL-TLS-THREADS`,
//! plan/ph14/design.md §7, R14-8; closes D-22.10's dynamic `EAGAIN`).
//!
//! Two layers: (1) the TLS descriptor functions of `csys/src/machine/tlsdesc.S`, called directly
//! (each variant forced by the caller) by `oracle/tlsdesc_abi.c` through `oracle/tlsdesc_harness.S`:
//! the offset in `%rax` and every other register, the vector registers and the floating-point
//! controls unchanged, the stack aligned and misaligned; (2) a program and a shared library with TLS
//! in every model a library gets (GD, LD, IE, TLSDESC) plus the program's own (LE), threads created
//! by the program and by the library, three rounds so that later threads reuse exited threads'
//! mappings: each new thread sees every variable's initial value and zeros, its own `errno`, and
//! keeps its own values while the others write theirs. Run on the host's glibc (the reference)
//! and on libniji's `libc.so.6` under libniji's `ld.so`.
// ARCH(x86_64): the oracle and the descriptor convention are x86-64's.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../core/tests/support/skip_note.rs"]
mod skip_note;
mod products;

#[path = "../../core/tests/support/scratch.rs"]
mod scratch;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch directory removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(what: &str) -> Scratch {
        let d = crate::scratch::base().join(format!("{what}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        Scratch(d)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        crate::scratch::release(std::path::Path::new(&self.0));
    }
}

fn cc(dir: &Path, args: &[&str]) {
    let o = Command::new("cc").current_dir(dir).args(args).output().expect("cc");
    assert!(o.status.success(), "cc {args:?}: {}", String::from_utf8_lossy(&o.stderr));
}

/// no spec clause (R14-8; A. Oliva, "Thread-Local Storage Descriptors for IA32 and AMD64/EM64T";
/// the convention in tlsdesc.S's header): the static and the dynamic descriptor function return the
/// offset from the thread pointer in `%rax` and change no other general register, no xmm register,
/// neither MXCSR nor the x87 control word, with the stack 16-byte aligned or off by 8 at the call;
/// the dynamic one under a resolver that changes all of them.
#[test]
fn tlsdesc_functions_keep_every_register_but_rax() {
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler; the TLS descriptor functions not checked");
        return;
    }
    let d = Scratch::new("niji-tlsdesc-abi");
    let oracle = root().join("crates/boundary/tests/oracle");
    let exe = d.0.join("tlsdesc_abi");
    let (abi, harness, prim) = (oracle.join("tlsdesc_abi.c"), oracle.join("tlsdesc_harness.S"), root().join("csys/src/machine/tlsdesc.S"));
    cc(&d.0, &["-O2", "-std=gnu11", "-Wall", "-Wextra", "-Werror", abi.to_str().unwrap(), harness.to_str().unwrap(), prim.to_str().unwrap(), "-o", exe.to_str().unwrap()]);
    let o = Command::new(&exe).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success() && out.contains("tlsdesc: ok"), "TLS descriptor convention broken: {:?}\n{out}", o.status);
}

/// The library's traditional-dialect part: GD on its globals, LD on a static, IE.
const LIB_TRAD: &str = "#include <pthread.h>
__thread long t_gd_v = 0x1111;
static __thread long t_ld_v = 0x2222;
__attribute__((tls_model(\"initial-exec\"))) __thread long t_ie_v = 0x3333;
__thread char t_zero_v[256];
__thread char t_al_v[8] __attribute__((aligned(64))) = {7};
long *t_gd(void) { return &t_gd_v; }
long *t_ld(void) { return &t_ld_v; }
long *t_ie(void) { return &t_ie_v; }
char *t_zero(void) { return t_zero_v; }
char *t_al(void) { return t_al_v; }
int lib_spawn(pthread_t *t, void *(*f)(void *), void *arg) { return pthread_create(t, 0, f, arg); }
";

/// The library's descriptor-dialect part (`-mtls-dialect=gnu2`): TLSDESC on a global and a static.
const LIB_DESC: &str = "__thread long d_gd_v = 0x4444;
static __thread long d_ld_v = 0x5555;
long *t_dgd(void) { return &d_gd_v; }
long *t_dld(void) { return &d_ld_v; }
";

/// The program: exits 0 when every check holds, else a number naming the first that failed.
const PROG: &str = "#include <errno.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
extern long *t_gd(void), *t_ld(void), *t_ie(void), *t_dgd(void), *t_dld(void);
extern char *t_zero(void), *t_al(void);
extern int lib_spawn(pthread_t *, void *(*)(void *), void *);
static __thread int p_le = 5;
static __thread char p_zero[64];
#define THREADS 6
static pthread_barrier_t bar;
static int fresh(int with_errno) {
  if (p_le != 5) return 1;
  for (int i = 0; i < 64; i++) if (p_zero[i]) return 2;
  if (*t_gd() != 0x1111) return 3;
  if (*t_ld() != 0x2222) return 4;
  if (*t_ie() != 0x3333) return 5;
  if (*t_dgd() != 0x4444) return 6;
  if (*t_dld() != 0x5555) return 7;
  char *z = t_zero();
  for (int i = 0; i < 256; i++) if (z[i]) return 8;
  char *a = t_al();
  if ((uintptr_t)a % 64 || a[0] != 7) return 9;
  if (with_errno && errno != 0) return 10;
  return 0;
}
static void mark(long id) {
  p_le = (int)id; memset(p_zero, (int)id, 64);
  *t_gd() = id + 1; *t_ld() = id + 2; *t_ie() = id + 3; *t_dgd() = id + 4; *t_dld() = id + 5;
  memset(t_zero(), (int)id, 256); t_al()[0] = (char)id; errno = (int)id;
}
static int same(long id) {
  char c = (char)id;
  if (p_le != id || *t_gd() != id + 1 || *t_ld() != id + 2 || *t_ie() != id + 3 || *t_dgd() != id + 4 || *t_dld() != id + 5) return 11;
  char *z = t_zero();
  for (int i = 0; i < 64; i++) if (p_zero[i] != c) return 12;
  for (int i = 0; i < 256; i++) if (z[i] != c) return 12;
  if (t_al()[0] != c) return 13;
  if (errno != id) return 14;
  return 0;
}
static void *worker(void *arg) {
  long id = (long)arg;
  int r = fresh(1);
  if (r == 0) mark(id);
  pthread_barrier_wait(&bar);
  if (r == 0) r = same(id);
  return (void *)(intptr_t)r;
}
int main(void) {
  int r = fresh(0);
  if (r) return 20 + r;
  mark(1000);
  for (int round = 0; round < 3; round++) {
    pthread_t t[THREADS];
    pthread_barrier_init(&bar, 0, THREADS);
    for (int k = 0; k < THREADS; k++) {
      long id = 100 * (round + 1) + k + 1;
      int e = (k % 2) ? lib_spawn(&t[k], worker, (void *)id) : pthread_create(&t[k], 0, worker, (void *)id);
      if (e) { printf(\"round %d: create %d failed: %d\\n\", round, k, e); return 40; }
    }
    for (int k = 0; k < THREADS; k++) {
      void *res;
      if (pthread_join(t[k], &res)) return 41;
      if (res) { printf(\"round %d thread %d: check %ld\\n\", round, k, (long)(intptr_t)res); return 50 + (int)(intptr_t)res; }
    }
    pthread_barrier_destroy(&bar);
  }
  r = same(1000);
  if (r) return 70 + r;
  puts(\"tls threads: ok\");
  return 0;
}
";

/// Build the library and the program in `dir`: `link` are the extra link arguments (libniji's
/// `libc.so` script and its loader), `pie` the program's. Returns the program.
fn build(dir: &Path, link: &[&str], prog_link: &[&str]) -> PathBuf {
    for (name, src, dialect) in [("trad", LIB_TRAD, "-mtls-dialect=gnu"), ("desc", LIB_DESC, "-mtls-dialect=gnu2")] {
        std::fs::write(dir.join(format!("{name}.c")), src).unwrap();
        cc(dir, &["-c", "-O1", "-fPIC", dialect, &format!("{name}.c"), "-o", &format!("{name}.o")]);
    }
    // No soname: the program names the library by path (libniji's loader has no search yet).
    let lib = dir.join("libtt.so");
    let mut args = vec!["-shared", "trad.o", "desc.o", "-o", lib.to_str().unwrap()];
    args.extend(link);
    cc(dir, &args);
    let r = String::from_utf8_lossy(&Command::new("readelf").arg("-rW").arg(&lib).output().expect("readelf").stdout).into_owned();
    for kind in ["R_X86_64_DTPMOD64", "R_X86_64_DTPOFF64", "R_X86_64_TPOFF64", "R_X86_64_TLSDESC"] {
        assert!(r.contains(kind), "the library has no {kind}: the models would not all be covered\n{r}");
    }
    std::fs::write(dir.join("prog.c"), PROG).unwrap();
    let exe = dir.join("prog");
    let mut args = vec!["-O1", "prog.c", lib.to_str().unwrap(), "-o", exe.to_str().unwrap()];
    args.extend(prog_link);
    cc(dir, &args);
    exe
}

/// s22_6_c_4 (each thread its own instance of a TLS variable), s22_1_c_1 (thread creation), s27_3_c_0
/// (`__tls_get_addr`), errno per thread (§2): threads created by a dynamically linked program and
/// by its library each get every initially loaded module's TLS (GD, LD, IE, TLSDESC in the library,
/// LE in the program) initialised, zeros where the image ends, the library's 64-byte alignment, and
/// their own `errno`, in fresh and in reused thread mappings; on the reference and on libniji.
#[test]
fn threads_get_static_tls_of_every_initial_module() {
    if skip_note::missing("cc") || skip_note::missing("readelf") {
        skip_note::skipped("skipped: no C compiler or readelf; threads with TLS in dynamic programs not checked");
        return;
    }
    let host = Scratch::new("niji-tls-threads-host");
    let exe = build(&host.0, &["-pthread"], &["-pthread"]);
    let o = Command::new(&exe).output().unwrap();
    assert_eq!(o.status.code(), Some(0), "on the reference: {}", String::from_utf8_lossy(&o.stdout));

    let own = Scratch::new("niji-tls-threads");
    let (ld, libc) = products::niji_link_script(&own.0);
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let libc = libc.to_str().unwrap();
    let exe = build(&own.0, &["-nodefaultlibs", libc, "-lgcc"], &["-nodefaultlibs", &interp, libc, "-lgcc"]);
    for how in ["PT_INTERP", "ld.so PROGRAM"] {
        let o = if how == "PT_INTERP" { Command::new(&exe).output() } else { Command::new(&ld).arg(&exe).output() }.unwrap();
        assert_eq!(o.status.code(), Some(0), "on libniji ({how}): {} {}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    }
}

/// A `dlopen`ed library with initial-exec TLS (a static-surplus block, DL-16d).
const LIB_STATIC: &str = "__attribute__((tls_model(\"initial-exec\"))) __thread long a_var[8];
long *a_get(void) { return a_var; }
";

/// A `dlopen`ed library with global-dynamic TLS only (a block made at first access, DL-19).
const LIB_ON_DEMAND: &str = "__thread long b_var[8] = { 0x4444, 0x4445, 0x4446, 0x4447, 0x4448, 0x4449, 0x444a, 0x444b };
long *b_get(void) { return b_var; }
";

/// Threads mark the static library's block, end, and the library is closed; the on-demand library
/// opened next takes its module ID (DL-17), and the threads created then (in the ended threads'
/// mappings) must each see that library's own image. Exits 0, else a number naming the step.
const PROG_REUSE: &str = "#define _GNU_SOURCE
#include <dlfcn.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#define THREADS 4
static long *(*get)(void);
static void *touch(void *arg) {
  long *v = get();
  for (int i = 0; i < 8; i++) v[i] = (long)(intptr_t)arg;
  return 0;
}
static void *check(void *arg) {
  (void)arg;
  long *v = get();
  for (int i = 0; i < 8; i++) if (v[i] != 0x4444 + i) return (void *)(intptr_t)(1 + i);
  return 0;
}
static int round_of(void *(*fn)(void *)) {
  pthread_t t[THREADS];
  for (int k = 0; k < THREADS; k++)
    if (pthread_create(&t[k], 0, fn, (void *)(intptr_t)(0x5a5a0000L + k))) return 1;
  for (int k = 0; k < THREADS; k++) {
    void *r;
    if (pthread_join(t[k], &r)) return 2;
    if (r) { printf(\"thread %d: word %ld is not the image\\n\", k, (long)(intptr_t)r - 1); return 3; }
  }
  return 0;
}
int main(int argc, char **argv) {
  if (argc != 3) return 9;
  void *a = dlopen(argv[1], RTLD_NOW);
  if (!a) { printf(\"dlopen: %s\\n\", dlerror()); return 10; }
  size_t ida = 0, idb = 0;
  if (dlinfo(a, RTLD_DI_TLS_MODID, &ida)) return 11;
  get = (long *(*)(void))dlsym(a, \"a_get\");
  if (!get || round_of(touch)) return 12;
  if (dlclose(a)) return 13;
  void *b = dlopen(argv[2], RTLD_NOW);
  if (!b) { printf(\"dlopen: %s\\n\", dlerror()); return 14; }
  if (dlinfo(b, RTLD_DI_TLS_MODID, &idb)) return 15;
  get = (long *(*)(void))dlsym(b, \"b_get\");
  if (!get) return 16;
  int r = round_of(check);
  if (r) return 20 + r;
  printf(\"module ids: %zu %zu\\n\", ida, idb);
  return 0;
}
";

/// s27_3_c_0 (the block of a module loaded by `dlopen` is allocated on first access), s22_6_c_4
/// (each thread its own instance): a module made per thread at first access (DL-19) that takes the
/// ID of an unloaded static-surplus module (DL-17) gives threads created in ended threads' mappings
/// its own image, not the old module's block (the reused mapping's area below the thread pointer
/// is not cleared, D-22.25); on the reference and on libniji, where the ID is checked to be reused.
#[test]
fn a_reused_mapping_gets_its_own_block_of_a_module_on_a_reused_id() {
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler; on-demand TLS in reused thread mappings not checked");
        return;
    }
    let build = |dir: &Path, lib_link: &[&str], prog_link: &[&str]| -> PathBuf {
        for (name, src, model) in [("static", LIB_STATIC, "-ftls-model=initial-exec"), ("demand", LIB_ON_DEMAND, "-ftls-model=global-dynamic")] {
            std::fs::write(dir.join(format!("{name}.c")), src).unwrap();
            let (src, so) = (format!("{name}.c"), format!("lib{name}.so"));
            let mut args = vec!["-O1", "-fPIC", "-shared", model, &src, "-o", &so];
            args.extend(lib_link);
            cc(dir, &args);
        }
        std::fs::write(dir.join("reuse.c"), PROG_REUSE).unwrap();
        let mut args = vec!["-O1", "reuse.c", "-o", "reuse"];
        args.extend(prog_link);
        cc(dir, &args);
        dir.join("reuse")
    };
    let host = Scratch::new("niji-tls-reuse-host");
    let exe = build(&host.0, &[], &["-pthread", "-ldl"]);
    let o = Command::new(&exe).arg(host.0.join("libstatic.so")).arg(host.0.join("libdemand.so")).output().unwrap();
    assert_eq!(o.status.code(), Some(0), "on the reference: {}", String::from_utf8_lossy(&o.stdout));

    let own = Scratch::new("niji-tls-reuse");
    let (ld, libc) = products::niji_link_script(&own.0);
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let libc = libc.to_str().unwrap();
    let exe = build(&own.0, &["-nodefaultlibs", libc, "-lgcc"], &["-nodefaultlibs", &interp, libc, "-lgcc"]);
    let o = Command::new(&exe).arg(own.0.join("libstatic.so")).arg(own.0.join("libdemand.so")).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert_eq!(o.status.code(), Some(0), "on libniji: {out} {}", String::from_utf8_lossy(&o.stderr));
    let ids: Vec<&str> = out.lines().find_map(|l| l.strip_prefix("module ids: ")).expect("the ids line").split(' ').collect();
    assert_eq!(ids[0], ids[1], "the on-demand module did not take the closed module's ID: the case is not reached");
}
