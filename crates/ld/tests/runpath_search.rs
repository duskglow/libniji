//! `dlopen` searches with the program's and the caller's `DT_RUNPATH` / `DT_RPATH` `$ORIGIN`, and
//! `RTLD_NOLOAD` by the names objects were loaded under (DL-RUNPATH; spec s27_1_c_1_1 search order,
//! s27_1_c_7 `dlinfo`). The expected results are the reference's, probed black box on the same corpus
//! (handoff/DL-RUNPATH.md): the program's `$ORIGIN` lists and a `$ORIGIN` name reach `dlopen`; a
//! start-time library's own `DT_RUNPATH` `$ORIGIN` serves its `dlopen` calls; a library caller
//! without lists uses the program's `DT_RPATH` and never its `DT_RUNPATH`; `DT_RUNPATH` is not used
//! for a dependency's own needs; `RTLD_NOLOAD` finds a start-time object without `DT_SONAME` by its
//! `DT_NEEDED` name, `ld.so` by its `PT_INTERP` path (another path to its file is not it). In a
//! secure process `dlopen` expands the program's `$ORIGIN` only in a trusted directory (rulings
//! #7094, #7190; no reference probe: a set-user-ID program needs root).
//! Needs `cargo xtask so` and a C compiler (else SKIP).
// ARCH(x86_64): the corpus is built with the host compiler for x86_64 and run under the x86_64 loader.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects)]

#[path = "../../core/tests/support/contain.rs"]
mod contain;
use std::path::{Path, PathBuf};
use std::process::Command;

use niji_ld::arch::{Arch, Native};

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// One `dlopen` and what it gives: the handle's link-map name, the error text, or "(null)".
const OPEN: &str = r#"
#define _GNU_SOURCE
#include <dlfcn.h>
#include <link.h>
#include <stdio.h>
void OPFN(const char *name, int flags) {
    void *h = dlopen(name, flags);
    const char *e = h ? 0 : dlerror();
    struct link_map *lm = 0;
    if (h) dlinfo(h, RTLD_DI_LINKMAP, &lm);
    printf(WHO " %s: %s\n", name, h ? (lm ? lm->l_name : "?") : (e ? e : "(null)"));
}
"#;

/// argv: `o:NAME` the program opens, `n:NAME` with RTLD_NOLOAD, `lo:NAME` libcall.so opens,
/// `po:NAME` libplain.so opens, `si` the program's search directories, `s:SYM` the RTLD_DEFAULT
/// definer.
const PROG: &str = r#"
#include <stdlib.h>
#include <string.h>
extern void call_op(const char *, int);
extern void plain_op(const char *, int);
extern int liba(void);
int main(int argc, char **argv) {
    (void)liba();
    for (int i = 1; i < argc; i++) {
        char *a = argv[i];
        if (!strncmp(a, "o:", 2)) prog_op(a + 2, RTLD_NOW);
        else if (!strncmp(a, "n:", 2)) prog_op(a + 2, RTLD_NOW | RTLD_NOLOAD);
        else if (!strncmp(a, "lo:", 3)) call_op(a + 3, RTLD_NOW);
        else if (!strncmp(a, "po:", 3)) plain_op(a + 3, RTLD_NOW);
        else if (!strcmp(a, "si")) {
            Dl_serinfo sz, *si;
            void *h = dlopen(0, RTLD_NOW);
            if (dlinfo(h, RTLD_DI_SERINFOSIZE, &sz)) return 2;
            si = malloc(sz.dls_size);
            *si = sz;
            if (dlinfo(h, RTLD_DI_SERINFO, si)) return 3;
            for (unsigned k = 0; k < si->dls_cnt; k++) printf("dir %s\n", si->dls_serpath[k].dls_name);
        } else if (!strncmp(a, "s:", 2)) {
            void *p = dlsym(RTLD_DEFAULT, a + 2);
            Dl_info di;
            printf("sym %s: %s\n", a + 2, p && dladdr(p, &di) ? di.dli_fname : "NULL");
        }
    }
    return 0;
}
"#;

/// The corpus (canonical directory `P`): `bin/{runpath,rpath}` (`$ORIGIN/lib:$ORIGIN/sub`), needing
/// `bin/lib/{libA.so (no DT_SONAME), libcall.so, libplain.so}`; libcall.so has `DT_RUNPATH`
/// `$ORIGIN/cdep` and needs `cdep/libC.so` (no DT_SONAME); `cdep/libD.so`; `bin/sub/{libB.so,
/// libE.so (needs libF.so), libF.so}`.
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
    let d = target().join(format!("ld-runpath-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for sub in ["bin/lib/cdep", "bin/sub"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
    let d = std::fs::canonicalize(d).unwrap();
    // libc.so.6 without DT_SONAME, linked by its path (as object_names.rs does).
    let mut bytes = std::fs::read(lib.join("libc.so.6")).unwrap();
    let elf = niji_ld::elf::Elf::parse::<Native>(&bytes).unwrap();
    let dynph = elf.find(niji_ld::elf::pt::DYNAMIC).unwrap();
    let (off, len) = (usize::try_from(dynph.offset).unwrap(), usize::try_from(dynph.filesz).unwrap());
    let at = (off..off + len).step_by(16).find(|&o| bytes[o..o + 8] == 14u64.to_le_bytes()).expect("libc.so.6 has a DT_SONAME");
    bytes[at..at + 8].copy_from_slice(&0x6fff_fdf8u64.to_le_bytes());
    let libc = d.join("libc-runpath.so");
    std::fs::write(&libc, &bytes).unwrap();
    let libc = libc.to_str().unwrap().to_owned();
    let cc = |out: &str, src: &str, flags: &[&str]| {
        let c = d.join(format!("{}.c", out.replace('/', "_")));
        std::fs::write(&c, src).unwrap();
        let r = Command::new("cc").args(["-O1", "-fno-stack-protector", "-fcf-protection=none", "-nostdlib", "-fPIC"]).arg(&c).args(flags).arg("-o").arg(d.join(out)).output().unwrap();
        assert!(r.status.success(), "{out}: {}", String::from_utf8_lossy(&r.stderr));
    };
    let (l, s, cd) = (d.join("bin/lib"), d.join("bin/sub"), d.join("bin/lib/cdep"));
    let (l, s, cd) = (l.to_str().unwrap(), s.to_str().unwrap(), cd.to_str().unwrap());
    let so = |out: &str, src: &str, extra: &[&str]| {
        let mut f = vec!["-shared"];
        f.extend_from_slice(extra);
        f.push(&libc);
        cc(out, src, &f);
    };
    so("bin/lib/libA.so", "int liba(void) { return 1; }\n", &[]);
    so("bin/lib/cdep/libC.so", "int libc_dep(void) { return 3; }\n", &[]);
    so("bin/lib/cdep/libD.so", "int libd(void) { return 4; }\n", &[]);
    so("bin/sub/libB.so", "int libb(void) { return 2; }\n", &[]);
    so("bin/sub/libF.so", "int libf(void) { return 6; }\n", &[]);
    so("bin/sub/libE.so", "extern int libf(void); int libe(void) { return libf(); }\n", &["-L", s, "-lF"]);
    let call = format!("#define WHO \"call\"\n#define OPFN call_op\n{OPEN}extern int libc_dep(void);\nint use_c(void) {{ return libc_dep(); }}\n");
    so("bin/lib/libcall.so", &call, &["-Wl,-soname,libcall.so", "-L", cd, "-lC", "-Wl,--enable-new-dtags", "-Wl,-rpath,$ORIGIN/cdep"]);
    so("bin/lib/libplain.so", &format!("#define WHO \"plain\"\n#define OPFN plain_op\n{OPEN}"), &["-Wl,-soname,libplain.so"]);
    let interp = format!("-Wl,--dynamic-linker={}", ld.display());
    let crt1 = lib.join("niji-crt1.o");
    let rpl = format!("-Wl,-rpath-link,{cd}");
    let src = format!("#define WHO \"prog\"\n#define OPFN prog_op\n{OPEN}{PROG}");
    for (prog, tags) in [("runpath", &["-Wl,--enable-new-dtags", "-Wl,-rpath,$ORIGIN/lib:$ORIGIN/sub"][..]), ("rpath", &["-Wl,--disable-new-dtags", "-Wl,-rpath,$ORIGIN/lib:$ORIGIN/sub"][..]), ("plain", &[][..])] {
        let mut flags = vec!["-pie", &interp, crt1.to_str().unwrap(), "-L", l, &rpl, "-lA", "-lcall", "-lplain"];
        flags.extend_from_slice(tags);
        flags.extend_from_slice(&[&libc, "-lgcc"]);
        cc(&format!("bin/{prog}"), &src, &flags);
    }
    Some(Corpus { dir: d, ld })
}

impl Corpus {
    /// Run `./bin/<prog> args...` from the corpus directory: its output with the directory as `P`.
    fn run(&self, prog: &str, args: &[&str]) -> String {
        self.run_env(prog, args, None)
    }

    /// [`Corpus::run`] with `LD_LIBRARY_PATH` set to `path`.
    fn run_env(&self, prog: &str, args: &[&str], path: Option<&str>) -> String {
        let mut c = Command::new(format!("./bin/{prog}"));
        c.current_dir(&self.dir).args(args);
        // Never the test runner's own LD_LIBRARY_PATH: it would come first in every search.
        match path {
            Some(p) => c.env("LD_LIBRARY_PATH", p),
            None => c.env_remove("LD_LIBRARY_PATH"),
        };
        let o = c.output().unwrap();
        assert!(o.status.success(), "{prog} {args:?}: {:?}\n{}", o.status, String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).replace(self.dir.to_str().unwrap(), "P")
    }

    fn expect(&self, prog: &str, args: &[&str], want: &[&str]) {
        let out = self.run(prog, args);
        let got: Vec<&str> = out.lines().collect();
        assert_eq!(got, want, "{prog} {args:?}");
    }
}

const MISSING: &str = "cannot open shared object file: No such file or directory";

// Spec s27_1_c_1_1, s27_1_c_7: the program's DT_RUNPATH or DT_RPATH `$ORIGIN` list serves its
// bare-name `dlopen`, a `$ORIGIN` name is the program's directory, and RTLD_DI_SERINFO lists the
// expanded directories; a DT_RUNPATH serves only the program's own needs (libE.so's libF.so is not
// found through it), a DT_RPATH also its dependencies'.
#[test]
fn program_origin_lists_serve_dlopen() {
    let Some(c) = setup("prog") else { return };
    for prog in ["runpath", "rpath"] {
        c.expect(prog, &["o:libB.so"], &["prog libB.so: P/bin/sub/libB.so"]);
        c.expect(prog, &["o:$ORIGIN/sub/libB.so"], &["prog $ORIGIN/sub/libB.so: P/bin/sub/libB.so"]);
        let dirs = c.run(prog, &["si"]);
        assert!(dirs.starts_with("dir P/bin/lib\ndir P/bin/sub\n"), "{prog} si:\n{dirs}");
    }
    c.expect("runpath", &["o:libE.so"], &[&format!("prog libE.so: libF.so: {MISSING}")]);
    c.expect("rpath", &["o:libE.so"], &["prog libE.so: P/bin/sub/libE.so"]);
}

// Spec s27_1_c_1_1: a program without lists: its `$ORIGIN` name is its directory, looked up at the
// `dlopen` (the start searched no token); an LD_LIBRARY_PATH `$ORIGIN` list serves `dlopen` with
// the directory the start looked up.
#[test]
fn program_origin_without_lists() {
    let Some(c) = setup("plain") else { return };
    let abs = format!("{0}/bin/lib:{0}/bin/lib/cdep", c.dir.display());
    let out = c.run_env("plain", &["o:$ORIGIN/sub/libB.so"], Some(&abs));
    assert_eq!(out, "prog $ORIGIN/sub/libB.so: P/bin/sub/libB.so\n");
    let out = c.run_env("plain", &["o:libB.so"], Some("$ORIGIN/lib:$ORIGIN/lib/cdep:$ORIGIN/sub"));
    assert_eq!(out, "prog libB.so: P/bin/sub/libB.so\n");
}

// Spec s27_1_c_1_1: a start-time library's DT_RUNPATH `$ORIGIN` serves its own `dlopen` (bare and
// `$ORIGIN` names), never the program's lists; a library caller without lists searches the
// program's DT_RPATH but not its DT_RUNPATH.
#[test]
fn caller_runpath_origin_serves_its_dlopen() {
    let Some(c) = setup("caller") else { return };
    c.expect("runpath", &["lo:libD.so"], &["call libD.so: P/bin/lib/cdep/libD.so"]);
    c.expect("runpath", &["lo:$ORIGIN/cdep/libD.so"], &["call $ORIGIN/cdep/libD.so: P/bin/lib/cdep/libD.so"]);
    for prog in ["runpath", "rpath"] {
        c.expect(prog, &["lo:libB.so"], &[&format!("call libB.so: libB.so: {MISSING}")]);
    }
    c.expect("runpath", &["po:libB.so"], &[&format!("plain libB.so: libB.so: {MISSING}")]);
    c.expect("rpath", &["po:libB.so"], &["plain libB.so: P/bin/sub/libB.so"]);
}

/// The secure-mode program (`bin/secure`, `DT_RUNPATH` `$ORIGIN/sub`, every start-time need by
/// absolute path): argv[1] `force` sets the `AT_SECURE` entry of its own auxiliary vector to 1
/// (where `dlopen` reads the flag; no privilege needed), `keep` leaves it; it prints the flag, then
/// what each further argument's `dlopen` gives (the link-map name or `NULL`).
const SECURE: &str = r#"
#define _GNU_SOURCE
#include <dlfcn.h>
#include <elf.h>
#include <link.h>
#include <stdio.h>
#include <string.h>
#include <sys/auxv.h>
int main(int argc, char **argv, char **envp) {
    unsigned long secure = getauxval(AT_SECURE);
    if (argc > 1 && !strcmp(argv[1], "force")) {
        char **e = envp;
        while (*e) e++;
        for (Elf64_auxv_t *a = (Elf64_auxv_t *)(e + 1); a->a_type != AT_NULL; a++)
            if (a->a_type == AT_SECURE) secure = a->a_un.a_val = 1;
    }
    printf("secure %lu\n", secure);
    for (int i = 2; i < argc; i++) {
        void *h = dlopen(argv[i], RTLD_NOW);
        struct link_map *lm = 0;
        if (h) dlinfo(h, RTLD_DI_LINKMAP, &lm);
        printf("%s: %s\n", argv[i], h ? (lm ? lm->l_name : "?") : "NULL");
    }
    return 0;
}
"#;

/// Run `c` (`secure mode libB.so $ORIGIN/sub/libB.so`, its environment set); its stdout.
fn secure_run(c: &mut Command) -> String {
    let o = c.output().unwrap();
    assert!(o.status.success(), "{c:?}: {:?}\n{}", o.status, String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

// Spec s27_1_c_1_1; threat model rule 2 (rulings #7094, #7190): in a secure process `dlopen` expands
// the program's `$ORIGIN` only when it is a trusted directory, as the start does. The flag is set in
// the process's own auxiliary vector (no root); the trusted directory is a default one with the
// corpus bound over it in a user and mount namespace (`unshare -r -m`). A set-group-ID copy (the
// kernel's AT_SECURE; a supplementary group of the user) must refuse the untrusted one too.
#[test]
fn secure_dlopen_origin_only_trusted() {
    let Some(c) = setup("secure") else { return };
    let bin = c.dir.join("bin");
    let (libc, crt1) = (c.dir.join("libc-runpath.so"), c.ld.with_file_name("niji-crt1.o"));
    let src = c.dir.join("secure.c");
    std::fs::write(&src, SECURE).unwrap();
    let r = Command::new("cc")
        .args(["-O1", "-fno-stack-protector", "-fcf-protection=none", "-nostdlib", "-fPIC", "-pie"])
        .arg(format!("-Wl,--dynamic-linker={}", c.ld.display()))
        .arg(&crt1)
        .arg(&src)
        .args(["-Wl,--enable-new-dtags", "-Wl,-rpath,$ORIGIN/sub"])
        .arg(&libc)
        .arg("-lgcc")
        .arg("-o")
        .arg(bin.join("secure"))
        .output()
        .unwrap();
    assert!(r.status.success(), "secure: {}", String::from_utf8_lossy(&r.stderr));
    let names = ["libB.so", "$ORIGIN/sub/libB.so"];
    let want = |flag: u32, found: &str| format!("secure {flag}\n{0}: {found}\n{1}: {found}\n", names[0], names[1]);
    let p = c.dir.display().to_string();
    let run = |prog: &Path, mode: &str| {
        secure_run(Command::new(prog).env_clear().arg(mode).args(names))
    };
    assert_eq!(run(&bin.join("secure"), "keep"), want(0, &format!("{p}/bin/sub/libB.so")), "not secure: $ORIGIN/sub used");
    assert_eq!(run(&bin.join("secure"), "force"), want(1, "NULL"), "secure: $ORIGIN {p}/bin is not trusted");

    // Trusted: a default directory that is no symbolic link, the corpus's bin/ bound over it.
    match Native::SYSTEM_DIRS.iter().find(|t| std::fs::canonicalize(t).is_ok_and(|r| r == Path::new(t))) {
        Some(t) if Command::new("unshare").args(["-r", "-m", "true"]).status().is_ok_and(|s| s.success()) => {
            let script = "mount --bind \"$1\" \"$2\" && exec \"$2/secure\" force \"$3\" \"$4\"";
            // Contained (support/contain.rs).
            let mut cmd = contain::user("unshare");
            cmd.env_clear().env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin").args(["-r", "-m", "sh", "-c", script, "sh"]).arg(&bin).args([*t, names[0], names[1]]);
            let out = secure_run(&mut cmd);
            assert_eq!(out, want(1, &format!("{t}/sub/libB.so")), "secure: $ORIGIN {t} is trusted");
        }
        Some(_) => eprintln!("SKIP (trusted part): unprivileged user and mount namespaces are not available (unshare -r -m)"),
        None => eprintln!("SKIP (trusted part): no default directory that is a real directory"),
    }

    // The kernel's flag: a set-group-ID copy beside the program (same $ORIGIN).
    let id = |a: &str| String::from_utf8(Command::new("id").arg(a).output().unwrap().stdout).unwrap();
    let primary = id("-g").trim().to_owned();
    let Some(group) = id("-G").split_whitespace().find(|g| *g != primary).map(str::to_owned) else {
        let _ = std::fs::remove_dir_all(&c.dir);
        return eprintln!("SKIP (set-group-ID part): the user has no supplementary group (a set-group-ID file needs one without root)");
    };
    let sgid = bin.join("secure-sgid");
    for (cmd, args) in [("cp", vec![bin.join("secure"), sgid.clone()]), ("chgrp", vec![PathBuf::from(&group), sgid.clone()]), ("chmod", vec![PathBuf::from("g+s"), sgid.clone()])] {
        assert!(Command::new(cmd).args(&args).status().unwrap().success(), "{cmd} {args:?}");
    }
    let out = run(&sgid, "keep");
    if out.starts_with("secure 1\n") {
        assert_eq!(out, want(1, "NULL"), "set-group-ID: $ORIGIN {p}/bin is not trusted");
    } else {
        eprintln!("SKIP (set-group-ID part): the kernel did not set AT_SECURE in {} (nosuid file system or no_new_privs)", bin.display());
    }
    let _ = std::fs::remove_dir_all(&c.dir);
}

// Spec s27_1_c_1 (RTLD_NOLOAD), s27_4_c_0: a start-time object without DT_SONAME is found by its
// DT_NEEDED name, even one only its requester's DT_RUNPATH reaches (libC.so); `ld.so` by its
// PT_INTERP path and soname, not by another path to its file; and `ld.so` is in the global scope.
#[test]
fn noload_finds_objects_by_their_loaded_names() {
    let Some(c) = setup("noload") else { return };
    let ld = c.ld.to_str().unwrap().to_owned();
    let other = format!("{}/./{}", c.ld.parent().unwrap().display(), Native::INTERP);
    let args = ["n:libA.so".to_owned(), "n:libC.so".to_owned(), format!("n:{ld}"), format!("n:{}", Native::INTERP), format!("n:{other}"), "s:_r_debug".to_owned()];
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let want = [
        "prog libA.so: P/bin/lib/libA.so".to_owned(),
        "prog libC.so: P/bin/lib/cdep/libC.so".to_owned(),
        format!("prog {ld}: {ld}"),
        format!("prog {}: {ld}", Native::INTERP),
        format!("prog {other}: (null)"),
        format!("sym _r_debug: {ld}"),
    ];
    let want: Vec<&str> = want.iter().map(String::as_str).collect();
    c.expect("runpath", &args, &want);
}
