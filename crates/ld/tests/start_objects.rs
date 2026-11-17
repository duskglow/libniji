//! No fixed limit on the objects `ld.so` loads at program start (DL-STARTUP-OBJECTS, GitHub issue
//! #9; ld.so(8): the program's dependencies are loaded "recursively", with no count stated): a
//! program linked against `N` trivial libraries, `N` past every bound the loader once had (64
//! objects, 128 inline dependency links, 255 as an 8-bit index, 256 objects in a `--list`), starts,
//! runs every constructor and destructor once (the exit in the reverse of the start's order), sees
//! every object through `dl_iterate_phdr`, can `dlopen` more, and is listed whole by `--list`; under
//! address-space limits the start either runs or ends with the loader's message and status 127,
//! never a signal.
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

/// Libraries the program needs: past 64, 128, 255 and 256.
const N: usize = 300;

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

fn built(mut cmd: Command, what: &str) {
    let r = cmd.output().unwrap();
    assert!(r.status.success(), "{what}: {}", String::from_utf8_lossy(&r.stderr));
}

/// `libl<k>.so`: `f<k>` returns `k` (`fx`: 120); its constructor writes `c<k>`, its destructor `d<k>`.
fn lib_src(k: &str) -> String {
    let v = if k == "x" { "120" } else { k };
    let line = |p: &str| format!("write(1, \"{p}{k}\\n\", {})", k.len() + 2);
    format!("#include <unistd.h>\n__attribute__((constructor)) static void c(void) {{ {}; }}\n__attribute__((destructor)) static void d(void) {{ {}; }}\nint f{k}(void) {{ return {v}; }}\n", line("c"), line("d"))
}

/// The program: the sum of every `f<k>`, the objects `dl_iterate_phdr` reports, and `fx` of
/// `libx.so` opened with `dlopen` after the start (-1: not opened).
fn prog_src() -> String {
    let mut s = String::from("#include <dlfcn.h>\n#include <link.h>\n#include <stdio.h>\n#include <unistd.h>\n");
    for k in 1..=N {
        writeln!(s, "int f{k}(void);").unwrap();
    }
    s.push_str("static int count(struct dl_phdr_info *i, size_t z, void *d) { (void)i; (void)z; ++*(int *)d; return 0; }\nint main(void) {\n    long sum = 0;\n");
    for k in 1..=N {
        writeln!(s, "    sum += f{k}();").unwrap();
    }
    s.push_str("    int n = 0;\n    dl_iterate_phdr(count, &n);\n    void *h = dlopen(\"libx.so\", RTLD_NOW);\n    int (*fx)(void) = h ? (int (*)(void))dlsym(h, \"fx\") : 0;\n    char b[96];\n    int len = snprintf(b, sizeof b, \"sum=%ld objects=%d x=%d\\n\", sum, n, fx ? fx() : -1);\n    write(1, b, len);\n    return 0;\n}\n");
    s
}

/// The corpus in a new scratch directory: (directory, `ld.so`, program), or `None` with the reason printed.
fn corpus() -> Option<(PathBuf, PathBuf, PathBuf)> {
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
    let d = target().join(format!("ld-start-objects-{}", std::process::id()));
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
    let names: Vec<String> = (1..=N).map(|k| k.to_string()).chain(["x".to_string()]).collect();
    for chunk in names.chunks(32) {
        let jobs: Vec<_> = chunk
            .iter()
            .map(|k| {
                let name = if k == "x" { "libx.so".to_string() } else { format!("libl{k}.so") };
                let soname = format!("-Wl,-soname,{name}");
                (name.clone(), cc(&d, &name, &lib_src(k), &["-nostdlib", "-fPIC", "-shared", "-Wl,--no-as-needed", &soname, libc_s]).spawn().unwrap())
            })
            .collect();
        for (name, job) in jobs {
            let r = job.wait_with_output().unwrap();
            assert!(r.status.success(), "{name} did not build");
        }
    }
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let (ldir, rpath) = (format!("-L{}", d.display()), format!("-Wl,-rpath,{}", d.display()));
    let crt1 = lib.join("niji-crt1.o");
    let mut flags: Vec<String> = ["-nostdlib", "-fPIE", "-pie", "-Wl,--no-as-needed", &interp, crt1.to_str().unwrap(), &ldir, &rpath].map(String::from).to_vec();
    flags.extend((1..=N).map(|k| format!("-l:libl{k}.so")));
    flags.extend([libc_s.to_string(), ld.to_str().unwrap().to_string(), "-lgcc".to_string()]);
    let flags: Vec<&str> = flags.iter().map(String::as_str).collect();
    built(cc(&d, "prog", &prog_src(), &flags), "prog");
    let prog = d.join("prog");
    Some((d, ld, prog))
}

/// Run `cmd` with an empty environment.
fn run(mut cmd: Command) -> Output {
    cmd.env_clear();
    if let Some(p) = std::env::var_os("PATH") {
        cmd.env("PATH", p);
    }
    cmd.output().unwrap()
}

/// The `<tag><k>` lines of `out` in order, as `k` (the library lines only: `x` left out).
fn seq(out: &str, tag: char) -> Vec<usize> {
    out.lines().filter_map(|l| l.strip_prefix(tag)).filter_map(|k| k.parse().ok()).collect()
}

/// s27_1_c_1_1 (no count of start-time objects is refused): a start of `N` libraries runs whole.
#[test]
fn start_objects_past_every_old_bound() {
    let Some((d, ld, prog)) = corpus() else { return };
    let o = run(Command::new(&prog));
    let out = String::from_utf8_lossy(&o.stdout);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "a start of {N} libraries failed: {:?}\n{err}", o.status);
    let ctors = seq(&out, 'c');
    let mut sorted = ctors.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (1..=N).collect::<Vec<_>>(), "every constructor runs once");
    let mut dtors = seq(&out, 'd');
    dtors.reverse();
    assert_eq!(dtors, ctors, "the destructors run once each, in the reverse of the constructors' order");
    assert_eq!(out.lines().filter(|l| *l == "cx" || *l == "dx").count(), 2, "the dlopened library's constructor and destructor run once\n{out}");
    let line = out.lines().find(|l| l.starts_with("sum=")).unwrap_or_default();
    let fields: Vec<&str> = line.split([' ', '=']).collect();
    assert_eq!(fields.get(1), Some(&(N * (N + 1) / 2).to_string().as_str()), "every library's function is bound: {line}");
    let seen: usize = fields.get(3).and_then(|v| v.parse().ok()).unwrap_or(0);
    assert!(seen > N + 2, "dl_iterate_phdr reports the program, its {N} libraries, libc and ld.so: {line}");
    assert_eq!(fields.get(5), Some(&"120"), "dlopen after a start of {N} objects works: {line}");
    // `--list`: every library on its own line.
    let o = run(Command::new(&ld).arg("--list").arg(&prog));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success(), "--list of {N} libraries failed: {}", String::from_utf8_lossy(&o.stderr));
    for k in 1..=N {
        let want = format!("\tlibl{k}.so => {}/libl{k}.so (0x", d.display());
        assert!(out.lines().any(|l| l.starts_with(&want)), "--list leaves out libl{k}.so");
    }
    std::fs::remove_dir_all(&d).ok();
}

/// no spec clause (the loader's tables are allocated, so the only start-time limit is memory): under
/// a sweep of address-space limits, from the first one at which the loader runs at all, the start
/// either runs or ends with one `ld.so:` line and status 127; never a signal.
#[test]
fn start_objects_out_of_memory_is_clean() {
    let Some((d, _, prog)) = corpus() else { return };
    let (mut ran, mut refused) = (0, 0);
    let mut reasons = std::collections::BTreeSet::new();
    for kb in (1024..=48 * 1024).step_by(256) {
        let o = run(Command::new("sh").arg("-c").arg(format!("ulimit -v {kb}; exec \"$0\"")).arg(&prog));
        let err = String::from_utf8_lossy(&o.stderr).into_owned();
        let loader = err.starts_with("ld.so:") || err.contains("error while loading shared libraries");
        if ran + refused == 0 && !o.status.success() && !loader {
            continue; // below the kernel's own need: the loader never ran
        }
        if o.status.success() {
            ran += 1;
            continue;
        }
        refused += 1;
        assert_eq!(o.status.code(), Some(127), "limit {kb} KiB: not the loader's clean failure: {:?}\n{err}", o.status);
        assert!(loader && err.lines().count() == 1, "limit {kb} KiB: one loader line expected:\n{err}");
        reasons.insert(err.split(": ").last().unwrap_or_default().trim().to_string());
    }
    assert!(ran > 0, "the program never ran within the sweep's largest limit");
    eprintln!("start under limits: {refused} refused ({reasons:?}), {ran} ran");
    std::fs::remove_dir_all(&d).ok();
}
