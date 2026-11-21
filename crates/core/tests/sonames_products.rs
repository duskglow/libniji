//! The sonames and development files a C library package ships (SL-SONAMES, plan/system-libc.md
//! §6 item 5): `libresolv.so.2` forwarding to `libc.so.6` (D-16.6), `libBrokenLocale.so.1`, and
//! the archives `-l<x>` finds in a static link. Programs are built against `target/niji` with
//! the system compiler and run directly (dynamic ones name libniji's loader and run with
//! `LD_LIBRARY_PATH` naming its directory).
//! No spec clause names these files: spec Appendix C lists the libraries and their symbols.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

#[path = "support/oracle.rs"]
mod oracle;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

fn products() -> PathBuf {
    let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/niji");
    for p in ["libc.a", "libc.so.6", "ld-linux-x86-64.so.2", "libresolv.so.2", "libBrokenLocale.so.1", "libBrokenLocale.a", "niji-crt1.o"] {
        assert!(lib.join(p).exists(), "{p} is missing: run `cargo xtask so`");
    }
    lib.canonicalize().unwrap()
}

/// Build `src` with `libs` statically (`static_link`) or dynamically, run it with `args` (and
/// `LOCPATH=locpath`), return its stdout. A dynamic program runs with `LD_LIBRARY_PATH` naming
/// `target/niji`: `libc.so.6` is a pinned soname, found only in the build's own library
/// directory or through `LD_LIBRARY_PATH`, never through RPATH/RUNPATH (D-27.16, R14-3 (a)), so
/// `LD_LIBRARY_PATH` is the route for running programs against a development build (ruling #5909).
fn build_run_with(dir: &Path, name: &str, src: &str, libs: &[&str], static_link: bool, args: &[&str], locpath: Option<&Path>) -> String {
    let lib = products();
    let c = dir.join(format!("{name}.c"));
    std::fs::write(&c, src).unwrap();
    let exe = dir.join(name);
    let mut a: Vec<OsString> = vec!["-nostdlib".into(), "-fno-builtin".into(), "-O1".into(), "-o".into(), exe.clone().into(), lib.join("niji-crt1.o").into(), c.into(), format!("-L{}", lib.display()).into()];
    if static_link {
        a.insert(0, "-static".into());
        a.extend(libs.iter().map(OsString::from));
        a.extend([lib.join("libc.a").into(), "-lgcc".into()]);
    } else {
        a.push(format!("-Wl,--dynamic-linker={}", lib.join("ld-linux-x86-64.so.2").display()).into());
        a.extend(libs.iter().map(OsString::from));
        a.extend([lib.join("libc.so.6").into(), "-lgcc".into()]);
    }
    let o = Command::new("cc").args(&a).output().unwrap();
    assert!(o.status.success(), "cc {a:?}: {}", String::from_utf8_lossy(&o.stderr));
    let mut run = Command::new(&exe);
    run.args(args);
    if let Some(l) = locpath {
        run.env("LOCPATH", l);
    }
    if !static_link {
        run.env("LD_LIBRARY_PATH", &lib);
    }
    let o = run.output().unwrap();
    assert!(o.status.success(), "{name}: {o:?}");
    String::from_utf8(o.stdout).unwrap()
}

/// [`build_run_with`] without arguments or `LOCPATH`.
fn build_run(dir: &Path, name: &str, src: &str, libs: &[&str], static_link: bool) -> String {
    build_run_with(dir, name, src, libs, static_link, &[], None)
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("niji-sonames-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `readelf -d` and `--dyn-syms` text of `so`.
fn readelf(args: &[&str], so: &Path) -> String {
    String::from_utf8(Command::new("readelf").args(args).arg(so).output().unwrap().stdout).unwrap()
}

const RESOLV: &str = "#include <arpa/inet.h>\n#include <arpa/nameser.h>\n#include <resolv.h>\n#include <math.h>\n#include <stdio.h>\n\
extern const struct res_sym __p_class_syms[];\nextern const char *_res_opcodes[];\n\
int main(void) {\n  unsigned char b[4] = {0}, w[2] = {0x12, 0x34};\n  int bits = inet_net_pton(AF_INET, \"10.1.0.0/16\", b, sizeof b);\n\
  const char *t = \"-\";\n  for (int i = 0; i < 7 && __p_class_syms[i].name; i++) if (__p_class_syms[i].number == 1) t = __p_class_syms[i].name;\n\
  printf(\"%d %u.%u %x %s %s %s %.0f\\n\", bits, b[0], b[1], (unsigned) ns_get16(w), p_class(1), t, _res_opcodes[0], sqrt(16.0));\n  return 0;\n}\n";

/// D-16.6, D-27.9 (no spec clause): a program linked with `-lresolv -lpthread -ldl -lrt -lm` runs
/// dynamically (needing `libresolv.so.2`, whose functions jump to `libc.so.6` and whose data
/// objects are filled from it) and statically, with the same output.
#[test]
fn sonames_resolv_dynamic_and_static() {
    let dir = scratch("resolv");
    let libs = ["-lresolv", "-lpthread", "-ldl", "-lrt", "-lm"];
    let want = "16 10.1 1234 IN IN QUERY 4\n";
    assert_eq!(build_run(&dir, "dyn", RESOLV, &libs, false), want, "dynamic");
    let needed = readelf(&["-d"], &dir.join("dyn"));
    assert!(needed.contains("[libresolv.so.2]") && needed.contains("[libc.so.6]"), "the dynamic program needs libresolv.so.2 and libc.so.6:\n{needed}");
    // Its references bind libresolv.so.2's versions (functions and data), none a private alias.
    let syms = readelf(&["--dyn-syms", "-W"], &dir.join("dyn"));
    for s in ["inet_net_pton@GLIBC_2.2.5", "ns_get16@GLIBC_2.9", "__p_class_syms@GLIBC_2.2.5", "_res_opcodes@GLIBC_2.2.5"] {
        assert!(syms.contains(s), "the dynamic program binds {s}:\n{syms}");
    }
    assert!(!syms.contains("NIJI_PRIVATE"), "no private name reaches the program:\n{syms}");
    assert_eq!(build_run(&dir, "stat", RESOLV, &libs, true), want, "static");
    let _ = std::fs::remove_dir_all(&dir);
}

/// D-16.6, LIBRESOLV-OWN-CODE (no spec clause): `libresolv.so.2` defines its versions.tsv exports
/// and nothing of `libc.so.6`'s. Its own code (`ns_get16`, the printers, the tables) is defined in
/// it, so `libc.so.6` carries no alias for it; each name that uses the C library's resolver state
/// (`__res_close`, the hosts-file walkers) is a jump through `libc.so.6`'s `NIJI_PRIVATE` alias.
#[test]
fn sonames_resolv_own_code_and_private_aliases() {
    let lib = products();
    let r = readelf(&["--dyn-syms", "-W"], &lib.join("libresolv.so.2"));
    let defined = |name: &str| r.lines().any(|l| l.ends_with(&format!(" {name}")) && !l.contains(" UND "));
    for s in ["ns_get16@@GLIBC_2.9", "__p_query@@GLIBC_2.2.5", "__p_class_syms@@GLIBC_2.2.5", "inet_net_pton@@GLIBC_2.2.5"] {
        assert!(defined(s), "{s}: libresolv.so.2's own definition:\n{r}");
    }
    assert!(r.contains(" res_gethostbyname@GLIBC_2.2.5"), "compat export (non-default):\n{r}");
    assert!(r.contains("UND __niji_fwd___res_close@NIJI_PRIVATE"), "forwarded through the alias:\n{r}");
    assert!(!r.contains("__niji_fwd_ns_get16"), "own code is not forwarded:\n{r}");
    assert!(!r.contains("GLIBC_PRIVATE"), "no GLIBC_PRIVATE:\n{r}");
    let c = readelf(&["--dyn-syms", "-W"], &lib.join("libc.so.6"));
    assert!(c.contains(" __niji_fwd___res_close@@NIJI_PRIVATE"), "libc.so.6 alias:\n");
    assert!(!c.contains("__niji_fwd_ns_get16") && !c.contains("__niji_fwd___p_type_syms"), "libc.so.6 carries no alias of libresolv.so.2's own code");
    assert!(!c.contains(" ns_get16@"), "libc.so.6 must not export ns_get16 under its own name");
}

const RESOLV_DLOPEN: &str = "#include <dlfcn.h>\n#include <stdio.h>\n#include <string.h>\n\
int main(void) {\n  void *h = dlopen(\"libresolv.so.2\", RTLD_NOW);\n  if (!h) { printf(\"dlopen: %s\\n\", dlerror()); return 1; }\n\
  const char *(*p_type)(int) = (const char *(*)(int)) dlsym(h, \"__p_type\");\n\
  const char *(*p_time)(unsigned) = (const char *(*)(unsigned)) dlsym(h, \"__p_time\");\n\
  void (*p_query)(const unsigned char *) = (void (*)(const unsigned char *)) dlsym(h, \"__p_query\");\n\
  void (*fp_nquery)(const unsigned char *, int, FILE *) = (void (*)(const unsigned char *, int, FILE *)) dlsym(h, \"__fp_nquery\");\n\
  int (*queriesmatch)(const unsigned char *, const unsigned char *, const unsigned char *, const unsigned char *) =\n\
    (int (*)(const unsigned char *, const unsigned char *, const unsigned char *, const unsigned char *)) dlsym(h, \"__res_queriesmatch\");\n\
  void (*res_close)(void) = (void (*)(void)) dlsym(h, \"__res_close\");\n\
  if (!p_type || !p_time || !p_query || !fp_nquery || !queriesmatch || !res_close) { printf(\"dlsym\\n\"); return 2; }\n\
  static unsigned char q[512] = {0x12,0x34,1,0,0,1,0,0,0,0,0,0,7,'e','x','a','m','p','l','e',3,'c','o','m',0,0,1,0,1};\n\
  printf(\"%s %s %s %d\\n\", p_type(28), p_type(4242), p_time(3661), queriesmatch(q, q + 29, q, q + 29));\n\
  fflush(stdout);\n  p_query(q);\n  fp_nquery(q, 29, stdout);\n  fflush(stdout);\n  res_close();\n  return dlclose(h);\n}\n";

/// What [`RESOLV_DLOPEN`] prints, settled by a probe of glibc 2.39 (2026-11-20): `p_query` parses
/// a whole 512-byte message, so the zero bytes after the question are an over-long message;
/// `fp_nquery` with the message's own length prints the header and the question.
const RESOLV_DLOPEN_OUT: &str = "AAAA 4242 1h1m1s 1\n;; ns_initparse: Message too long\n\
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 4660\n\
;; flags: rd; QUERY: 1, ANSWER: 0, AUTHORITY: 0, ADDITIONAL: 0\n\
;;\texample.com, type = A, class = IN\n";

/// D-16.6, LIBRESOLV-OWN-CODE (no spec clause): a program that never names libresolv.so.2 loads it
/// with `dlopen` and calls its own code (a static name, a per-thread result buffer for an unknown
/// type and for `p_time`, the message printers writing to `stdout` through the C library, an
/// error text, a message comparison) and a name forwarded to `libc.so.6` (`__res_close`), and
/// prints what the same program prints on glibc 2.39 ([`RESOLV_DLOPEN_OUT`]).
#[test]
fn sonames_resolv_dlopen_own_code() {
    let dir = scratch("resolv-dlopen");
    let out = build_run(&dir, "dl", RESOLV_DLOPEN, &[], false);
    let needed = readelf(&["-d"], &dir.join("dl"));
    assert!(!needed.contains("[libresolv.so.2]"), "the program does not need libresolv.so.2 at start:\n{needed}");
    assert_eq!(out, RESOLV_DLOPEN_OUT, "libresolv.so.2's own code through dlopen");
    let _ = std::fs::remove_dir_all(&dir);
}

const MBMAX: &str = "#include <locale.h>\n#include <stdio.h>\n#include <stdlib.h>\n\
int main(int argc, char **argv) {\n  printf(\"%zu\", (size_t) MB_CUR_MAX);\n\
  for (int i = 1; i < argc; i++) {\n    if (!setlocale(LC_ALL, argv[i])) return 2;\n    printf(\" %zu\", (size_t) MB_CUR_MAX);\n  }\n\
  printf(\"\\n\");\n  return 0;\n}\n";

/// `MB_CUR_MAX` (`__ctype_get_mb_cur_max`; no spec clause: libBrokenLocale's values are probe-settled, SL-SONAMES record):
/// linked with `-lBrokenLocale`, dynamically or statically, MB_CUR_MAX is 1 in "C" and 4 in
/// "C.UTF-8"; without it, 1 and 6. Statically, code sets of width 2, 3 and 4 (EUC-KR, EUC-JP,
/// GB18030, compiled by the host's `localedef`; without it the test says so) give 1, 2 and 2.
#[test]
fn sonames_brokenlocale_mb_cur_max() {
    let dir = scratch("broken");
    let utf8 = ["C.UTF-8"];
    for (name, libs, want) in [("plain", &[][..], "1 6\n"), ("dyn", &["-lBrokenLocale"][..], "1 4\n")] {
        assert_eq!(build_run_with(&dir, name, MBMAX, libs, false, &utf8, None), want, "{name}");
    }
    let needed = readelf(&["-d"], &dir.join("dyn"));
    assert!(needed.contains("[libBrokenLocale.so.1]"), "-lBrokenLocale links libBrokenLocale.so.1:\n{needed}");
    let syms = readelf(&["--dyn-syms", "-W"], &products().join("libBrokenLocale.so.1"));
    assert!(syms.contains(" __ctype_get_mb_cur_max@@GLIBC_2.2.5"), "libBrokenLocale.so.1 defines it:\n{syms}");
    assert_eq!(build_run_with(&dir, "stat", MBMAX, &[], true, &utf8, None), "1 6\n", "static without -lBrokenLocale");
    assert_eq!(build_run_with(&dir, "stat_broken", MBMAX, &["-lBrokenLocale"], true, &utf8, None), "1 4\n", "static -lBrokenLocale");
    // The widths between: the first program shows the code sets' own widths, the second the bound.
    let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/test-locales");
    let wide = [("ko_KR", "EUC-KR"), ("ja_JP", "EUC-JP"), ("zh_CN", "GB18030")];
    let names: Vec<String> = wide.iter().map(|(l, c)| format!("{l}.{c}")).collect();
    if wide.iter().zip(&names).all(|((l, c), n)| oracle::locale(&locales.join(n), l, c, "LC_CTYPE")) {
        let args: Vec<&str> = names.iter().map(String::as_str).collect();
        for (name, libs, want) in [("stat_wide", &[][..], "1 2 3 4\n"), ("stat_broken_wide", &["-lBrokenLocale"][..], "1 1 2 2\n")] {
            assert_eq!(build_run_with(&dir, name, MBMAX, libs, true, &args, Some(&locales)), want, "{name} ({names:?})");
        }
    } else {
        skip_note::skipped("sonames_brokenlocale_mb_cur_max: no localedef; the 2-, 3- and 4-byte code sets not checked");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The development archives (no spec clause; plan/system-libc.md §2): the archives a static `-l<x>` needs exist, empty.
#[test]
fn sonames_empty_archives() {
    let lib = products();
    for a in ["libresolv.a", "libg.a", "libmcheck.a", "libpthread_nonshared.a"] {
        let o = Command::new("ar").arg("t").arg(lib.join(a)).output().unwrap();
        assert!(o.status.success() && o.stdout.is_empty(), "{a} is an empty archive: {o:?}");
    }
}
