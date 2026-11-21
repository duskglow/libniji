//! `_dl_find_object` and `dladdr` through the loader's address index (DL-FIND-OBJECT-FAST,
//! crates/boundary/src/ld/index.rs) under libniji's `ld.so` and `libc.so.6`. One C program checks
//! itself against the program headers `dl_iterate_phdr` reports and exits with the number of the
//! first check that failed (its text on standard error); `alarm` is only a hang guard, loops have
//! iteration budgets. Needs `cargo xtask so` and a C compiler (else SKIP).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use niji_ld::arch::{Arch, Native};
use niji_ld::elf::Elf;

/// Libraries in the corpus: the first half stays loaded, the second half is opened and closed.
const N: usize = 40;

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// The corpus directory and the program, or `None` with the reason printed (as dlclose.rs).
fn setup(name: &str) -> Option<(PathBuf, PathBuf)> {
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
    let d = target().join(format!("ld-find-index-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    // `libc.so.6` named by path with its DT_SONAME made inert (as dlclose.rs).
    let mut so = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = Elf::parse::<Native>(&so).unwrap();
    let dynph = elf.find(niji_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (dynph.offset as usize, dynph.filesz as usize);
    let at = (off..off + len).step_by(16).find(|&o| so[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    so[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-dl.so");
    std::fs::write(&libc, &so).unwrap();
    for k in 1..=N {
        let name = format!("libq{k}.so");
        let soname = format!("-Wl,-soname,{name}");
        build(&d, &name, &format!("int v{k} = {k};\nint f{k}(void) {{ return v{k}; }}\n"), &["-nostdlib", "-fPIC", "-shared", "-Wl,--eh-frame-hdr", &soname]);
    }
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("niji-crt1.o");
    let prog = build(&d, "prog", PROG, &["-nostdlib", "-fPIE", "-pie", "-Wl,--export-dynamic", "-Wl,--eh-frame-hdr", &interp, crt1.to_str().unwrap(), libc.to_str().unwrap(), ld.to_str().unwrap(), "-lgcc"]);
    Some((d, prog))
}

fn build(d: &Path, out: &str, src: &str, flags: &[&str]) -> PathBuf {
    let c = d.join(format!("{out}.c"));
    std::fs::write(&c, src).unwrap();
    let o = d.join(out);
    let r = Command::new("cc").args(["-O1", "-fno-stack-protector", "-fcf-protection=none"]).arg(&c).args(flags).arg("-o").arg(&o).output().unwrap();
    assert!(r.status.success(), "{out}: {}", String::from_utf8_lossy(&r.stderr));
    o
}

/// The program: `argv[1]` the corpus directory, `argv[2]` `map` or `churn`.
const PROG: &str = r#"#define _GNU_SOURCE
#include <dlfcn.h>
#include <link.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#define N 40
#define HALF (N / 2)
static const char *dir;
static void *h[N + 1];
struct obj { unsigned long lo, hi, eh; int nseg, vdso; unsigned long s[16][2]; };
static struct obj objs[256];
static int nobj;
static unsigned long page;
static int cb(struct dl_phdr_info *i, size_t sz, void *d) {
    (void)sz; (void)d;
    if (nobj >= 256) return 1;
    struct obj *o = &objs[nobj++];
    memset(o, 0, sizeof *o);
    o->lo = ~0UL;
    o->vdso = strstr(i->dlpi_name, "linux-vdso") != NULL;
    for (int k = 0; k < i->dlpi_phnum; k++) {
        const ElfW(Phdr) *p = &i->dlpi_phdr[k];
        unsigned long a = i->dlpi_addr + p->p_vaddr;
        if (p->p_type == PT_LOAD) {
            if (a < o->lo) o->lo = a;
            if (a + p->p_memsz > o->hi) o->hi = a + p->p_memsz;
            if (o->nseg < 16) { o->s[o->nseg][0] = a; o->s[o->nseg][1] = a + p->p_memsz; o->nseg++; }
        }
        if (p->p_type == PT_GNU_EH_FRAME) o->eh = a;
    }
    return 0;
}
static void snapshot(void) { nobj = 0; dl_iterate_phdr(cb, 0); }
static void fail(int n, const char *what, unsigned long a) { fprintf(stderr, "check %d: %s at %#lx\n", n, what, a); exit(n); }
/* The object whose range (lowest to highest PT_LOAD address, gaps between segments included: probe R-5)
 * holds a (-1: none), and the first non-vDSO object whose range holds it. */
static int span_owner(unsigned long a) {
    for (int j = 0; j < nobj; j++)
        if (objs[j].lo <= a && a < objs[j].hi) return j;
    return -1;
}
static int range_owner(unsigned long a) {
    for (int j = 0; j < nobj; j++)
        if (!objs[j].vdso && objs[j].lo <= a && a < objs[j].hi) return j;
    return -1;
}
static void probe(int base, unsigned long a) {
    struct dl_find_object f;
    memset(&f, 0, sizeof f);
    int j = span_owner(a), r = _dl_find_object((void *)a, &f);
    if (j < 0 && r != -1) fail(base + 1, "_dl_find_object found an object where none is", a);
    if (j >= 0 && r != 0) fail(base + 2, "_dl_find_object found nothing in a loaded object's range", a);
    if (j >= 0 && ((unsigned long)f.dlfo_map_start != objs[j].lo || (unsigned long)f.dlfo_map_end != objs[j].hi || (unsigned long)f.dlfo_eh_frame != objs[j].eh))
        fail(base + 3, "_dl_find_object: wrong range or PT_GNU_EH_FRAME", a);
    Dl_info info;
    memset(&info, 0, sizeof info);
    int o = range_owner(a), d = dladdr((void *)a, &info);
    if (o < 0 && d != 0) fail(base + 4, "dladdr found an object where none is", a);
    if (o >= 0 && (d == 0 || (unsigned long)info.dli_fbase != (objs[o].lo & -page))) fail(base + 5, "dladdr: wrong or no object", a);
}
/* Every segment's first and last byte, the byte past it, the byte below each object. */
static void verify(int base) {
    snapshot();
    for (int j = 0; j < nobj; j++) {
        probe(base, objs[j].lo - 1);
        for (int k = 0; k < objs[j].nseg; k++) {
            unsigned long lo = objs[j].s[k][0], hi = objs[j].s[k][1];
            probe(base, lo);
            if (hi > lo) probe(base, hi - 1);
            probe(base, hi);
        }
    }
}
static void *open_lib(int k) {
    char p[4200];
    snprintf(p, sizeof p, "%s/libq%d.so", dir, k);
    void *x = dlopen(p, RTLD_NOW);
    if (!x) { fprintf(stderr, "dlopen %s: %s\n", p, dlerror()); exit(90); }
    return x;
}
static unsigned long fn(int k) { char s[16]; snprintf(s, sizeof s, "f%d", k); return (unsigned long)dlsym(h[k], s); }

static void map(void) {
    /* The start-time objects only (the index built by the first reader). */
    verify(10);
    for (int k = 1; k <= N; k++) h[k] = open_lib(k);
    verify(20);
    /* Every link map is the handle. */
    for (int k = 1; k <= N; k++) {
        struct dl_find_object f;
        if (_dl_find_object((void *)fn(k), &f) != 0 || f.dlfo_link_map != h[k]) fail(31, "_dl_find_object: link map is not the handle", fn(k));
    }
    /* Closed objects are gone at once (their pages may stay mapped for a grace period). */
    unsigned long gone[N + 1] = {0};
    for (int k = 2; k <= N; k += 2) { gone[k] = fn(k); dlclose(h[k]); h[k] = 0; }
    for (int k = 2; k <= N; k += 2) {
        struct dl_find_object f;
        Dl_info info;
        if (_dl_find_object((void *)gone[k], &f) != -1) fail(41, "_dl_find_object finds a closed object", gone[k]);
        if (dladdr((void *)gone[k], &info) != 0) fail(42, "dladdr finds a closed object", gone[k]);
    }
    verify(50);
    for (int k = 2; k <= N; k += 2) h[k] = open_lib(k);
    verify(60);
}

/* Readers look up the objects that stay loaded while a writer opens and closes others. */
#define CYCLES 3000
#define READERS 3
struct want { unsigned long a, lo, hi, eh; void *map; };
static struct want want[N * 8];
static int nwant;
static volatile int done;
static void *reader(void *arg) {
    long bad = 0, i = 0;
    (void)arg;
    while (!done) {
        struct want *w = &want[i++ % nwant];
        struct dl_find_object f;
        Dl_info info;
        if (_dl_find_object((void *)w->a, &f) != 0 || (unsigned long)f.dlfo_map_start != w->lo || (unsigned long)f.dlfo_map_end != w->hi
            || (unsigned long)f.dlfo_eh_frame != w->eh || f.dlfo_link_map != w->map) bad++;
        if (dladdr((void *)w->a, &info) == 0 || (unsigned long)info.dli_fbase != (w->lo & -page)) bad++;
    }
    return (void *)bad;
}
static void churn(void) {
    for (int k = 1; k <= HALF; k++) h[k] = open_lib(k);
    for (int k = 1; k <= HALF; k++) {
        struct dl_find_object f;
        if (_dl_find_object((void *)fn(k), &f) != 0) fail(70, "no object for a loaded function", fn(k));
        snapshot();
        for (int j = 0; j < nobj; j++)
            if (objs[j].lo == (unsigned long)f.dlfo_map_start)
                for (int s = 0; s < objs[j].nseg && nwant < N * 8; s++)
                    if (objs[j].s[s][1] > objs[j].s[s][0])
                        want[nwant++] = (struct want){ objs[j].s[s][0], objs[j].lo, objs[j].hi, objs[j].eh, h[k] };
    }
    pthread_t t[READERS];
    for (int r = 0; r < READERS; r++) pthread_create(&t[r], 0, reader, 0);
    for (int c = 0; c < CYCLES; c++) {
        int k = HALF + 1 + (c * 7) % HALF;
        if (h[k]) { dlclose(h[k]); h[k] = 0; } else h[k] = open_lib(k);
    }
    done = 1;
    long bad = 0;
    for (int r = 0; r < READERS; r++) { void *b; pthread_join(t[r], &b); bad += (long)b; }
    if (bad) { fprintf(stderr, "check 71: %ld lookups of loaded objects were wrong during the churn\n", bad); exit(71); }
    verify(80);
}

int main(int argc, char **argv) {
    if (argc < 3) return 99;
    alarm(600);
    dir = argv[1];
    page = (unsigned long)sysconf(_SC_PAGESIZE);
    if (!strcmp(argv[2], "map")) map(); else churn();
    puts("ok");
    return 0;
}
"#;

fn run(prog: &Path, d: &Path, mode: &str) {
    let o = Command::new(prog).arg(d).arg(mode).output().unwrap();
    assert!(o.status.success() && o.stdout == b"ok\n", "{mode}: status {:?}\n{}{}", o.status.code(), String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
}

/// s27_1_c_9, s27_1_c_6: `_dl_find_object` and `dladdr` find the object whose range (lowest to
/// highest `PT_LOAD` address) holds an address, at every segment's edges and in the gaps between
/// segments (`_dl_find_object` too: probe R-5), with only the start-time objects (the index's
/// first build), after `dlopen`, after `dlclose` (a closed object is never found again) and after
/// reopening.
#[test]
fn s27_1_c_9_find_object_index_matches_the_program_headers() {
    let Some((d, prog)) = setup("map") else { return };
    run(&prog, &d, "map");
    let _ = std::fs::remove_dir_all(&d);
}

/// s27_1_c_9 (lock-free readers, D-27.1): three threads look up the objects that stay loaded while
/// another opens and closes others 3000 times; every lookup must give the object's exact range,
/// `PT_GNU_EH_FRAME` and link map (a reader that used a copy the writer was rewriting would miss or
/// mix entries).
#[test]
fn s27_1_c_9_find_object_index_during_dlopen_dlclose_churn() {
    let Some((d, prog)) = setup("churn") else { return };
    run(&prog, &d, "churn");
    let _ = std::fs::remove_dir_all(&d);
}
