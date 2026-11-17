//! The start files of a plain compiler-driver link (SL-CRT): `gcc`, `g++` and `gcc -pg` with no
//! `-nostartfiles`, each finding libniji's start files under their installed names (the
//! development package's `crt1.o`, `Scrt1.o`, `rcrt1.o`, `gcrt1.o`, `grcrt1.o`, `crti.o`,
//! `crtn.o`, `Mcrt1.o`, tools/deb/build.sh) through `-B`, which the driver searches first. Each link
//! is traced (`-Wl,-t`): every start file and C library file it read must be libniji's, so a file
//! missing from the products cannot pass by being taken from the host. The objects themselves are
//! checked against the x86-64 psABI's `.init`/`.fini` layout. Needs the products
//! (`cargo xtask so`), `cc`, `c++`, `gprof`, `readelf`, `objcopy` and `nm`; without the tools the test
//! says so and passes. The same plain link also builds a program that binds `libm.so.6`'s old
//! versions (`fmod@GLIBC_2.2.5`, SYS-COMPAT-VERSIONS; `log`, `log2`) and one that binds `libc.so.6`'s
//! `pthread_sigmask@GLIBC_2.2.5` (SYS-COMPAT-VERSIONS-REST), and compares each with the reference.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;
#[path = "support/timeout.rs"]
mod timeout;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn products() -> PathBuf {
    root().join("target/niji")
}

/// Installed name -> product (tools/deb/build.sh, niji6-dev).
const START: [(&str, &str); 8] = [
    ("crt1.o", "niji-crt1.o"),
    ("Scrt1.o", "niji-crt1.o"),
    ("rcrt1.o", "niji-rcrt1.o"),
    ("gcrt1.o", "niji-gcrt1.o"),
    ("grcrt1.o", "niji-grcrt1.o"),
    ("crti.o", "niji-crti.o"),
    ("crtn.o", "niji-crtn.o"),
    ("Mcrt1.o", "niji-Mcrt1.o"),
];

/// The archives a link reads by name (`-lc`, `-lm`, the `libc.so` script).
const ARCHIVES: [&str; 3] = ["libc_nonshared.a", "libc.a", "libm.a"];

/// The C library files a link may read; each must be libniji's.
const LIBS: [&str; 10] = ["libc.so", "libm.so", "libc.so.6", "libm.so.6", "libc-by-path.so", "libm-by-path.so", "ld-linux-x86-64.so.2", "libc_nonshared.a", "libc.a", "libm.a"];

/// A copy of `so` with its DT_SONAME tag renamed to an unused tag, so that a link names it by its
/// path: libniji's loader does not search sonames at start yet (DL-8, plan/system-libc.md §6 item
/// 1); conformance/run.sh's `bypath` does the same.
fn by_path(so: &Path, out: &Path) {
    let o = Command::new("readelf").arg("-Wd").arg(so).output().unwrap();
    let t = String::from_utf8_lossy(&o.stdout);
    let off = t.lines().find_map(|l| l.strip_prefix("Dynamic section at offset ")).and_then(|r| r.split_whitespace().next()).unwrap();
    let off = usize::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
    let idx = t.lines().filter(|l| l.trim_start().starts_with("0x")).position(|l| l.contains("(SONAME)")).unwrap();
    let mut b = std::fs::read(so).unwrap();
    b[off + 16 * idx..off + 16 * idx + 8].copy_from_slice(&0x6fff_fdf8_u64.to_le_bytes());
    std::fs::write(out, b).unwrap();
}

fn tools_missing() -> bool {
    let missing = ["cc", "c++", "gprof", "readelf", "objcopy", "nm"].into_iter().find(|t| skip_note::missing(t));
    if let Some(t) = missing {
        skip_note::skipped(&format!("skipped: no {t}"));
    }
    missing.is_some()
}

/// A fresh work directory with `lib/`: the link-time files under their installed names.
fn work(name: &str) -> (PathBuf, PathBuf) {
    let p = products();
    assert!(p.join("niji-crti.o").exists(), "the products are missing: run `cargo xtask so`");
    let w = root().join("target/crt-products").join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&w);
    let lib = w.join("lib");
    std::fs::create_dir_all(&lib).unwrap();
    for (installed, product) in START {
        std::os::unix::fs::symlink(p.join(product), lib.join(installed)).unwrap();
    }
    for f in ARCHIVES {
        std::os::unix::fs::symlink(p.join(f), lib.join(f)).unwrap();
    }
    for name in ["libc", "libm"] {
        let copy = lib.join(format!("{name}-by-path.so"));
        by_path(&p.join(format!("{name}.so.6")), &copy);
        let extra = if name == "libc" { format!(" \"{}\" AS_NEEDED ( \"{}\" )", lib.join("libc_nonshared.a").display(), p.join("ld-linux-x86-64.so.2").display()) } else { String::new() };
        std::fs::write(lib.join(format!("{name}.so")), format!("GROUP ( \"{}\"{extra} )\n", copy.display())).unwrap();
    }
    (w, lib)
}

fn ldso() -> String {
    format!("-Wl,--dynamic-linker={}", products().join("ld-linux-x86-64.so.2").display())
}

/// Runs a compiler driver (`cc` or `c++`); with `lib`, links against it with `-B` and returns
/// the link trace. Panics with the driver's messages when it fails.
fn drive(driver: &str, lib: Option<&Path>, args: &[String]) -> String {
    let mut cmd = Command::new(driver);
    if let Some(l) = lib {
        cmd.arg(format!("-B{}/", l.display())).arg("-Wl,-t");
    }
    let o = cmd.args(args).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success(), "{driver} {args:?} failed:\n{text}");
    text
}

/// The start files `trace` shows the link read (installed names); every start file and C library
/// file in it must lie in `lib`.
fn start_files(trace: &str, lib: &Path) -> BTreeSet<String> {
    let lib = lib.display().to_string();
    let prod = s(&products());
    let names: BTreeSet<&str> = START.iter().map(|s| s.0).chain(LIBS).collect();
    let mut seen = BTreeSet::new();
    for word in trace.split(|c: char| c.is_whitespace() || c == '(' || c == ')') {
        let Some(name) = word.rsplit('/').next() else { continue };
        if word.starts_with('/') && names.contains(name) {
            assert!(word.starts_with(&lib) || word.starts_with(&prod), "the link read {word}, not libniji's (from {lib}):\n{trace}");
            if START.iter().any(|s| s.0 == name) {
                seen.insert(name.to_string());
            }
        }
    }
    seen
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

/// Runs `exe` (in `dir`, environment cleared); its standard output.
fn run(exe: &Path, dir: &Path) -> String {
    let mut cmd = Command::new("timeout");
    cmd.arg(timeout::secs(30)).arg(exe).current_dir(dir).env_clear();
    let o = cmd.output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(o.status.success(), "{}: {:?}\nstdout:\n{out}stderr:\n{}", exe.display(), o.status, String::from_utf8_lossy(&o.stderr));
    out
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

/// `readelf -SW` of `obj`: (name, type, flags, alignment, index, size) per section.
fn sections(obj: &Path) -> Vec<(String, String, String, u64, String, u64)> {
    let o = Command::new("readelf").arg("-SW").arg(obj).output().unwrap();
    assert!(o.status.success());
    let mut out = Vec::new();
    for line in String::from_utf8_lossy(&o.stdout).lines() {
        let Some((head, rest)) = line.split_once("] ") else { continue };
        let f: Vec<&str> = rest.split_whitespace().collect();
        // Name Type Address Off Size ES [Flg] Lk Inf Al
        if f.len() < 9 || f[0] == "Name" {
            continue;
        }
        let flags = if f.len() >= 10 { f[6].to_string() } else { String::new() };
        let ndx = head.rsplit('[').next().unwrap_or("").trim().to_string();
        let size = u64::from_str_radix(f[4], 16).unwrap();
        out.push((f[0].to_string(), f[1].to_string(), flags, f[f.len() - 1].parse().unwrap(), ndx, size));
    }
    out
}

/// `readelf -sW` of `obj`: (type, bind, visibility, section index, name) of every named symbol.
fn symbols(obj: &Path) -> Vec<[String; 5]> {
    let o = Command::new("readelf").arg("-sW").arg(obj).output().unwrap();
    assert!(o.status.success());
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() == 8 && f[0].ends_with(':') && f[0] != "Num:").then(|| [f[3], f[4], f[5], f[6], f[7]].map(String::from))
        })
        .collect()
}

/// The link-time address of symbol `name` in `exe` (`nm`).
fn link_address(exe: &Path, name: &str) -> u64 {
    let o = Command::new("nm").arg(exe).output().unwrap();
    let text = String::from_utf8_lossy(&o.stdout);
    let line = text.lines().find(|l| l.split_whitespace().nth(2) == Some(name)).unwrap_or_else(|| panic!("{} defines no {name}", exe.display()));
    u64::from_str_radix(line.split_whitespace().next().unwrap(), 16).unwrap()
}

/// The bytes of section `name` of `obj`.
fn bytes(obj: &Path, name: &str, scratch: &Path) -> Vec<u8> {
    let out = scratch.join(format!("{}{name}.bin", obj.file_name().unwrap().to_string_lossy()));
    let st = Command::new("objcopy").args(["-O", "binary", "-j", name]).arg(obj).arg(&out).status().unwrap();
    assert!(st.success(), "objcopy {name} {}", obj.display());
    std::fs::read(&out).unwrap()
}

/// endbr64; sub $8,%rsp: the prologue, with the stack aligned for the contributions' calls.
const PROLOGUE: [u8; 8] = [0xf3, 0x0f, 0x1e, 0xfa, 0x48, 0x83, 0xec, 0x08];
/// add $8,%rsp; ret: the epilogue.
const EPILOGUE: [u8; 5] = [0x48, 0x83, 0xc4, 0x08, 0xc3];

#[test]
fn start_files_follow_the_psabi() {
    // Clauses: s1_1_c_5, s1_1_c_6 (DT_INIT/DT_FINI are the .init/.fini functions; SL-CRT).
    if skip_note::missing("readelf") || skip_note::missing("objcopy") {
        skip_note::skipped("skipped: no readelf or objcopy");
        return;
    }
    let (w, _) = work("psabi");
    let p = products();
    for name in ["crti", "crtn"] {
        let obj = p.join(format!("niji-{name}.o"));
        let secs = sections(&obj);
        for sec in [".init", ".fini"] {
            let (_, ty, flags, align, _, _) = secs.iter().find(|x| x.0 == sec).unwrap_or_else(|| panic!("{name}.o has no {sec}: {secs:?}"));
            assert_eq!((ty.as_str(), flags.as_str()), ("PROGBITS", "AX"), "{name}.o {sec}: code, allocated, never writable");
            let b = bytes(&obj, sec, &w);
            if name == "crti" {
                assert_eq!(b, PROLOGUE, "crti.o {sec}: the prologue");
            } else {
                // Alignment 1: no fill between the last contribution and the epilogue.
                assert_eq!(*align, 1, "crtn.o {sec} alignment");
                assert_eq!(b, EPILOGUE, "crtn.o {sec}: the epilogue");
            }
        }
        let syms = symbols(&obj);
        let globals: Vec<&[String; 5]> = syms.iter().filter(|x| x[1] != "LOCAL").collect();
        if name == "crti" {
            for (sym, sec) in [("_init", ".init"), ("_fini", ".fini")] {
                let ndx = &secs.iter().find(|x| x.0 == sec).unwrap().4;
                let want = ["FUNC", "GLOBAL", "HIDDEN"].map(String::from);
                assert!(globals.iter().any(|x| x[4] == sym && x[..3] == want && x[3] == *ndx), "crti.o: {sym} must be a hidden global function at the start of {sec}: {syms:?}");
            }
            assert_eq!(globals.len(), 2, "crti.o defines only _init and _fini: {syms:?}");
        } else {
            assert!(globals.is_empty(), "crtn.o defines no global: {syms:?}");
        }
    }
    // Every start file marks the stack non-executable and has no writable code.
    for (installed, product) in START {
        let secs = sections(&p.join(product));
        assert!(secs.iter().any(|x| x.0 == ".note.GNU-stack" && !x.2.contains('X')), "{installed}: no .note.GNU-stack: {secs:?}");
        assert!(!secs.iter().any(|x| x.2.contains('W') && x.2.contains('X')), "{installed}: a writable code section: {secs:?}");
    }
    // Mcrt1.o: no driver names it; nothing in it reaches a program (the assembler's empty
    // .text/.data/.bss are allocated but hold no byte).
    let m = p.join("niji-Mcrt1.o");
    let msecs = sections(&m);
    assert!(!msecs.iter().any(|x| x.2.contains('A') && x.5 != 0), "Mcrt1.o has allocated content: {msecs:?}");
    assert!(symbols(&m).iter().all(|x| x[1] == "LOCAL"), "Mcrt1.o defines a symbol");
    // grcrt1.o: the static-PIE start plus the profiling hook in .preinit_array.
    let g = p.join("niji-grcrt1.o");
    let syms = symbols(&g);
    for sym in ["_start", "__gmon_start__"] {
        assert!(syms.iter().any(|x| x[4] == sym && x[1] == "GLOBAL" && x[3] != "UND"), "grcrt1.o does not define {sym}");
    }
    assert!(syms.iter().any(|x| x[4] == "niji_start_main_static_pie" && x[3] == "UND"), "grcrt1.o does not start through the relocating start");
    assert!(sections(&g).iter().any(|x| x.0 == ".preinit_array"), "grcrt1.o has no .preinit_array");
}

/// The output every build of crt_init_fini.c prints: `_init`'s contribution, main, `_fini`'s,
/// with the stack aligned at each contribution's call.
const HELLO: &str = "prog init sp%16=0\nhello, world\nprog fini sp%16=0\n";

#[test]
fn plain_cc_links_and_runs_hello() {
    // Clauses: s1_1_c_5, s1_1_c_6 (DT_INIT runs before main, DT_FINI at exit; SL-CRT).
    if tools_missing() {
        return;
    }
    let (w, lib) = work("hello");
    let src = s(&root().join("crates/core/tests/oracle/crt_init_fini.c"));
    // (driver options, the start object the driver must pick, dynamic)
    for (opts, start, dynamic) in [(&[][..], "Scrt1.o", true), (&["-no-pie"][..], "crt1.o", true), (&["-static"][..], "crt1.o", false), (&["-static-pie"][..], "rcrt1.o", false)] {
        let tag = opts.first().copied().unwrap_or("pie");
        let mut cflags: Vec<String> = vec!["-std=gnu17".into(), "-O1".into()];
        cflags.push(if tag == "-no-pie" { "-fno-pie".into() } else { "-fPIE".into() });
        let obj = w.join(format!("hello{tag}.o"));
        drive("cc", None, &[cflags.clone(), vec!["-c".into(), src.clone(), "-o".into(), s(&obj)]].concat());
        let exe = w.join(format!("hello{tag}"));
        let mut args: Vec<String> = opts.iter().map(|x| (*x).to_string()).collect();
        if dynamic {
            args.push(ldso());
        }
        args.extend([s(&obj), "-o".into(), s(&exe)]);
        let trace = drive("cc", Some(&lib), &args);
        assert_eq!(start_files(&trace, &lib), set(&[start, "crti.o", "crtn.o"]), "cc {tag}: start files");
        assert_eq!(run(&exe, &w), HELLO, "cc {tag}");
        // The same program on the host's C library prints the same.
        let host = w.join(format!("host{tag}"));
        drive("cc", None, &[opts.iter().map(|x| (*x).to_string()).collect(), vec![s(&obj), "-o".into(), s(&host)]].concat());
        assert_eq!(run(&host, &w), HELLO, "the reference, cc {tag}");
    }
    // A shared object's own _init and _fini, run by the loader around the program's (named by
    // its path: the object has no soname).
    let so = w.join("libcrtlib.so");
    drive("cc", None, &["-std=gnu17".into(), "-O1".into(), "-fPIC".into(), "-DCRT_LIB".into(), "-c".into(), src.clone(), "-o".into(), s(&w.join("lib.o"))]);
    let trace = drive("cc", Some(&lib), &["-shared".into(), s(&w.join("lib.o")), "-o".into(), s(&so)]);
    assert_eq!(start_files(&trace, &lib), set(&["crti.o", "crtn.o"]), "cc -shared: start files");
    drive("cc", None, &["-std=gnu17".into(), "-O1".into(), "-DCRT_USE_LIB".into(), "-c".into(), src, "-o".into(), s(&w.join("uselib.o"))]);
    let exe = w.join("uselib");
    drive("cc", Some(&lib), &[ldso(), s(&w.join("uselib.o")), s(&so), "-o".into(), s(&exe)]);
    let host = w.join("host-uselib");
    let host_so = w.join("host").join("libcrtlib.so");
    std::fs::create_dir_all(host_so.parent().unwrap()).unwrap();
    drive("cc", None, &["-shared".into(), s(&w.join("lib.o")), "-o".into(), s(&host_so)]);
    drive("cc", None, &[s(&w.join("uselib.o")), s(&host_so), "-o".into(), s(&host)]);
    let want = run(&host, &w);
    assert!(want.contains("lib init sp%16=0\n") && want.contains("lib fini sp%16=0\n"), "the reference: {want}");
    assert_eq!(run(&exe, &w), want, "a shared object's _init and _fini");
}

#[test]
fn plain_cxx_runs_static_constructors_and_destructors() {
    // Clauses: s1_1_c_5, s1_1_c_6 (SL-CRT: plain g++).
    if tools_missing() {
        return;
    }
    let (w, lib) = work("cxx");
    let src = s(&root().join("crates/core/tests/oracle/crt_cxx.cc"));
    let want = "ctor first\nctor second\nmain\nctor local\ndtor local\ndtor second\ndtor first\n";
    for (opts, start, dynamic) in [(&[][..], "Scrt1.o", true), (&["-static"][..], "crt1.o", false)] {
        let tag = opts.first().copied().unwrap_or("pie");
        let obj = w.join(format!("cxx{tag}.o"));
        drive("c++", None, &["-O1".into(), "-c".into(), src.clone(), "-o".into(), s(&obj)]);
        let exe = w.join(format!("cxx{tag}"));
        let mut args: Vec<String> = opts.iter().map(|x| (*x).to_string()).collect();
        if dynamic {
            // The C++ run-time libraries in the program, so the run needs no library of the host.
            args.extend(["-static-libstdc++".into(), "-static-libgcc".into(), ldso()]);
        }
        args.extend([s(&obj), "-o".into(), s(&exe)]);
        let trace = drive("c++", Some(&lib), &args);
        assert_eq!(start_files(&trace, &lib), set(&[start, "crti.o", "crtn.o"]), "c++ {tag}: start files");
        assert_eq!(run(&exe, &w), want, "c++ {tag}");
        let host = w.join(format!("host{tag}"));
        drive("c++", None, &[opts.iter().map(|x| (*x).to_string()).collect(), vec![s(&obj), "-o".into(), s(&host)]].concat());
        assert_eq!(run(&host, &w), want, "the reference, c++ {tag}");
    }
}

#[test]
fn plain_cc_pg_writes_gmon_out() {
    // Clauses: s28_1_c_1, s28_1_c_2 (SL-CRT: gcrt1.o and grcrt1.o in a plain -pg link).
    if tools_missing() {
        return;
    }
    let (w, lib) = work("pg");
    let src = s(&root().join("crates/core/tests/oracle/gmon_pg.c"));
    let obj = w.join("pg.o");
    drive("cc", None, &["-std=gnu17".into(), "-O1".into(), "-pg".into(), "-fPIE".into(), "-c".into(), src, "-o".into(), s(&obj)]);
    for (opts, start, dynamic) in [(&[][..], "gcrt1.o", true), (&["-static-pie"][..], "grcrt1.o", false)] {
        let tag = opts.first().copied().unwrap_or("pie");
        let exe = w.join(format!("pg{tag}"));
        let mut args: Vec<String> = vec!["-pg".into()];
        args.extend(opts.iter().map(|x| (*x).to_string()));
        if dynamic {
            args.push(ldso());
        }
        args.extend([s(&obj), "-o".into(), s(&exe)]);
        let trace = drive("cc", Some(&lib), &args);
        assert_eq!(start_files(&trace, &lib), set(&[start, "crti.o", "crtn.o"]), "cc -pg {tag}: start files");
        let dir = w.join(format!("run{tag}"));
        std::fs::create_dir_all(&dir).unwrap();
        run(&exe, &dir);
        let gmon = dir.join("gmon.out");
        assert!(gmon.exists(), "cc -pg {tag}: no gmon.out");
        // gmon.out holds link-time pcs (the load bias subtracted, as the reference writes for
        // both kinds of PIE): the histogram starts at __executable_start's link-time address.
        let b = std::fs::read(&gmon).unwrap();
        assert!(b.len() > 37 && &b[..5] == b"gmon\x01" && b[20] == 0, "cc -pg {tag}: not a gmon.out with a histogram first");
        let lo = u64::from_le_bytes(b[21..29].try_into().unwrap());
        assert_eq!(lo, link_address(&exe, "__executable_start"), "cc -pg {tag}: gmon.out pcs are not link-time pcs");
        let o =Command::new("gprof").args(["-b", "-q"]).arg(&exe).arg(&gmon).output().unwrap();
        let text = String::from_utf8_lossy(&o.stdout);
        assert!(o.status.success() && text.contains("main"), "cc -pg {tag}: gprof cannot read gmon.out:\n{text}{}", String::from_utf8_lossy(&o.stderr));
    }
}

/// The `name@VERSION` pairs `readelf --dyn-syms` shows `exe` binding (undefined).
fn bound_versions(exe: &Path) -> BTreeSet<String> {
    let o = Command::new("readelf").args(["--dyn-syms", "-W"]).arg(exe).output().unwrap();
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() >= 8 && f[6] == "UND").then(|| f[7].split(" (").next().unwrap_or(f[7]).to_string())
        })
        .collect()
}

#[test]
fn old_libm_versions_bind_and_match_the_reference() {
    // Clauses: s0_6_c0_6_0 (a name's old version stays exported for old binaries), s10_2_t_r12
    // (fmod). The program binds each name's GLIBC_2.2.5 version with `.symver` (Debian's gawk binds
    // fmod's) and prints every edge case's result bits, errno and flags; libniji's libm.so.6 must
    // give the reference's text, case for case: the default version's result, but for the old
    // logarithms' NaN sign on a negative argument (spec/behaviour/libm-compat.md).
    if tools_missing() {
        return;
    }
    let (w, lib) = work("libm-compat");
    let src = s(&root().join("crates/core/tests/oracle/libm_compat_versions.c"));
    let obj = w.join("libm_compat_versions.o");
    drive("cc", None, &["-std=gnu17".into(), "-O1".into(), "-fPIE".into(), "-fno-builtin".into(), "-c".into(), src, "-o".into(), s(&obj)]);
    let exe = w.join("libm_compat_versions");
    // libm.so.6 needs libc.so.6 by name, so this program names both by soname (a by-path libc beside
    // it would load a second C library): `soname/` holds link scripts naming the products, the link
    // checks libm's needs against libniji's files (-rpath-link) and the loader finds them through
    // LD_LIBRARY_PATH.
    let p = products();
    let soname = w.join("soname");
    std::fs::create_dir_all(&soname).unwrap();
    let libc_script = format!("GROUP ( \"{}\" \"{}\" AS_NEEDED ( \"{}\" ) )\n", s(&p.join("libc.so.6")), s(&lib.join("libc_nonshared.a")), s(&p.join("ld-linux-x86-64.so.2")));
    std::fs::write(soname.join("libc.so"), libc_script).unwrap();
    std::fs::write(soname.join("libm.so"), format!("GROUP ( \"{}\" )\n", s(&p.join("libm.so.6")))).unwrap();
    drive("cc", Some(&lib), &[ldso(), format!("-L{}", s(&soname)), format!("-Wl,-rpath-link={}", s(&p)), s(&obj), "-lm".into(), "-o".into(), s(&exe)]);
    let bound = bound_versions(&exe);
    // (name, default version, cases, cases where the old version differs from the default): the old
    // versions Debian trixie's programs bind. The old logarithms give a NaN with the sign bit clear for
    // a negative argument (8 cases; a NaN argument, of either sign, is not one of them).
    let names = [
        ("fmod", "GLIBC_2.38", 484, 0),
        ("fmodf", "GLIBC_2.38", 484, 0),
        ("exp", "GLIBC_2.29", 22, 0),
        ("exp10", "GLIBC_2.39", 22, 0),
        ("pow", "GLIBC_2.29", 484, 0),
        ("hypot", "GLIBC_2.35", 484, 0),
        ("hypotf", "GLIBC_2.35", 484, 0),
        ("log", "GLIBC_2.29", 24, 8),
        ("log2", "GLIBC_2.29", 24, 8),
    ];
    for (name, default, ..) in names {
        for pair in [format!("{name}@GLIBC_2.2.5"), format!("{name}@{default}")] {
            assert!(bound.contains(&pair), "the program binds {pair}: {bound:?}");
        }
    }
    let host = w.join("host-libm_compat_versions");
    drive("cc", None, &[s(&obj), "-lm".into(), "-o".into(), s(&host)]);
    let run_v = |e: &Path, libs: Option<&Path>| {
        // The path is set by `env` after `timeout` starts, so only the program sees it (the host's
        // `timeout` must not load libniji's libc.so.6 under the host's loader).
        let mut cmd = Command::new("timeout");
        cmd.arg(timeout::secs(30)).arg("env");
        if let Some(l) = libs {
            cmd.arg(format!("LD_LIBRARY_PATH={}", s(l)));
        }
        let o = cmd.arg(e).arg("-v").current_dir(&w).env_clear().output().unwrap();
        assert!(o.status.success(), "{}: {:?} {}", e.display(), o.status, String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).into_owned()
    };
    let want = run_v(&host, None);
    for (name, _, cases, diffs) in names {
        assert!(want.contains(&format!("\n{name}@GLIBC_2.2.5: {cases} cases, {diffs} differ from the default\n")), "the reference, {name}:\n{want}");
    }
    let got = run_v(&exe, Some(&p));
    // Cases where libniji's DEFAULT pow differs from the reference's (both libraries print "same as
    // default" for them, so the old version is still the default's alias): an exact subnormal
    // result raises FE_UNDERFLOW in the reference only (IEEE 754 7.5: underflow needs an inexact
    // tiny result). pow(-1, +-1e308) = 1 and the signalling-NaN cases (sNaN^+-0, 1^sNaN: a quiet
    // NaN with FE_INVALID) are compared like every other case (MATH-POW-EDGES).
    const DEFAULT_POW: [&str; 3] = [
        "pow@GLIBC_2.2.5(0x1p-1, 0x1p+10)",
        "pow@GLIBC_2.2.5(0x0.012688b70e62bp-1022, 0x1p+0)",
        "pow@GLIBC_2.2.5(-0x0.012688b70e62bp-1022, 0x1p+0)",
    ];
    for g in got.lines().filter(|g| DEFAULT_POW.iter().any(|k| g.starts_with(&format!("{k} = ")))) {
        assert!(g.ends_with(": same as default"), "libniji's old pow is its default's alias: {g}");
    }
    let differ: Vec<String> = got
        .lines()
        .zip(want.lines())
        .filter(|(g, r)| g != r && !DEFAULT_POW.iter().any(|k| g.starts_with(&format!("{k} = "))))
        .map(|(g, r)| format!("libniji:   {g}\nreference: {r}"))
        .collect();
    assert!(differ.is_empty(), "libniji's old version against the reference's, {} cases differ:\n{}", differ.len(), differ.iter().take(12).cloned().collect::<Vec<_>>().join("\n"));
    assert_eq!(got.lines().count(), want.lines().count(), "the same cases");
}

#[test]
fn old_pthread_sigmask_binds_and_matches_the_reference() {
    // Clauses: s0_6_c0_6_0 (a name's old version stays exported for old binaries), s20_3_c_1
    // (pthread_sigmask). Debian trixie's btscanner, pbzip2 and wavemon bind
    // pthread_sigmask@GLIBC_2.2.5; the reference's old version equals its default on every probed case
    // (spec/behaviour/pthread-sigmask-compat.md), so libniji's is an alias of its default. The program
    // binds both versions and prints, per case, the return value, errno, the old set and the kernel
    // mask afterwards; libniji must print the reference's text, case for case.
    if tools_missing() {
        return;
    }
    let (w, lib) = work("sigmask-compat");
    let src = s(&root().join("crates/core/tests/oracle/pthread_sigmask_compat.c"));
    let obj = w.join("pthread_sigmask_compat.o");
    drive("cc", None, &["-std=gnu17".into(), "-O1".into(), "-fPIE".into(), "-c".into(), src, "-o".into(), s(&obj)]);
    let exe = w.join("pthread_sigmask_compat");
    drive("cc", Some(&lib), &[ldso(), s(&obj), "-o".into(), s(&exe)]);
    let bound = bound_versions(&exe);
    for pair in ["pthread_sigmask@GLIBC_2.2.5", "pthread_sigmask@GLIBC_2.32"] {
        assert!(bound.contains(pair), "the program binds {pair}: {bound:?}");
    }
    let host = w.join("host-pthread_sigmask_compat");
    drive("cc", None, &[s(&obj), "-o".into(), s(&host)]);
    let want = run(&host, &w);
    assert!(want.ends_with("\npthread_sigmask@GLIBC_2.2.5: 140 cases, 0 differ from the default\n"), "the reference:\n{want}");
    let got = run(&exe, &w);
    assert!(got.ends_with("\npthread_sigmask@GLIBC_2.2.5: 140 cases, 0 differ from the default\n"), "libniji's old version is its default's alias:\n{got}");
    // Differences of libniji's DEFAULT version from the reference's, outside this compat version
    // (both libraries print "same as default" for every case; reported by SYS-COMPAT-VERSIONS-REST
    // for the default's own story): libniji writes the whole 128-byte old set (zeros after the
    // kernel's 8 bytes, "+120"; the reference "+0"); its masks treat signals 32 and 33 otherwise (a set
    // can block 33; the old set hides 32); and with a null set it still rejects an invalid `how`
    // (EINVAL; the reference ignores `how` then, as POSIX says). Exactly these are masked or skipped.
    let default_only = |text: &str| -> String {
        let reserved = !((1u64 << 31) | (1u64 << 32));
        let field = |l: &str, key: &str| -> String {
            match l.split_once(key) {
                Some((head, rest)) if rest.len() >= 16 && rest.as_bytes()[..16].iter().all(u8::is_ascii_hexdigit) => {
                    let v = u64::from_str_radix(&rest[..16], 16).unwrap() & reserved;
                    format!("{head}{key}{v:016x}{}", &rest[16..])
                }
                _ => l.to_string(),
            }
        };
        text.lines()
            .filter(|l| !l.contains(", how 3, set 0000000000000000 (null)") && !l.contains(", how -1, set 0000000000000000 (null)"))
            .map(|l| field(&field(&l.replace(" +120 after ", " +0 after "), " old "), " after ") + "\n")
            .collect()
    };
    let (got, want) = (default_only(&got), default_only(&want));
    let differ: Vec<String> = got.lines().zip(want.lines()).filter(|(g, r)| g != r).map(|(g, r)| format!("libniji:   {g}\nreference: {r}")).collect();
    assert!(differ.is_empty(), "libniji against the reference, {} cases differ:\n{}", differ.len(), differ.iter().take(12).cloned().collect::<Vec<_>>().join("\n"));
    assert_eq!(got.lines().count(), want.lines().count(), "the same cases");
}
