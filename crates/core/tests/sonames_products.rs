//! The sonames and development files a C library package ships (SL-SONAMES, plan/system-libc.md
//! §6 item 5): `libresolv.so.2` forwarding to `libc.so.6` (D-16.6), `libBrokenLocale.so.1`, and
//! the archives `-l<x>` finds in a static link. Programs are built against `target/liella` with
//! the system compiler and run directly (dynamic ones name Liella's loader and its directory).
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
    let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/liella");
    for p in ["libc.a", "libc.so.6", "ld-linux-x86-64.so.2", "libresolv.so.2", "libBrokenLocale.so.1", "libBrokenLocale.a", "liella-crt1.o"] {
        assert!(lib.join(p).exists(), "{p} is missing: run `cargo xtask so`");
    }
    lib.canonicalize().unwrap()
}

/// The loader's refusal of a `DT_NEEDED` soname while its start-time library search is absent
/// (DL-8, plan/system-libc.md §6 item 1).
const NO_SEARCH: &str = "the library search is not available in this build";

/// Build `src` with `libs` statically (`static_link`) or dynamically, run it with `args` (and
/// `LOCPATH=locpath`), return its stdout; `None` (and the reason printed) for a dynamic program the
/// loader cannot start for want of its soname search.
fn build_run_with(dir: &Path, name: &str, src: &str, libs: &[&str], static_link: bool, args: &[&str], locpath: Option<&Path>) -> Option<String> {
    let lib = products();
    let c = dir.join(format!("{name}.c"));
    std::fs::write(&c, src).unwrap();
    let exe = dir.join(name);
    let mut a: Vec<OsString> = vec!["-nostdlib".into(), "-fno-builtin".into(), "-O1".into(), "-o".into(), exe.clone().into(), lib.join("liella-crt1.o").into(), c.into(), format!("-L{}", lib.display()).into()];
    if static_link {
        a.insert(0, "-static".into());
        a.extend(libs.iter().map(OsString::from));
        a.extend([lib.join("libc.a").into(), "-lgcc".into()]);
    } else {
        a.push(format!("-Wl,--dynamic-linker={}", lib.join("ld-linux-x86-64.so.2").display()).into());
        a.push(format!("-Wl,-rpath,{}", lib.display()).into());
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
    let o = run.output().unwrap();
    if !static_link && String::from_utf8_lossy(&o.stderr).contains(NO_SEARCH) {
        eprintln!("{name}: dynamic run skipped: Liella's loader has no start-time soname search yet (DL-8); linked and inspected only");
        return None;
    }
    assert!(o.status.success(), "{name}: {o:?}");
    Some(String::from_utf8(o.stdout).unwrap())
}

/// [`build_run_with`] without arguments or `LOCPATH`.
fn build_run(dir: &Path, name: &str, src: &str, libs: &[&str], static_link: bool) -> Option<String> {
    build_run_with(dir, name, src, libs, static_link, &[], None)
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("liella-sonames-{tag}-{}", std::process::id()));
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
    let d = build_run(&dir, "dyn", RESOLV, &libs, false);
    if let Some(d) = d {
        assert_eq!(d, want, "dynamic");
    }
    let needed = readelf(&["-d"], &dir.join("dyn"));
    assert!(needed.contains("[libresolv.so.2]") && needed.contains("[libc.so.6]"), "the dynamic program needs libresolv.so.2 and libc.so.6:\n{needed}");
    // Its references bind libresolv.so.2's versions (functions and data), none a private alias.
    let syms = readelf(&["--dyn-syms", "-W"], &dir.join("dyn"));
    for s in ["inet_net_pton@GLIBC_2.2.5", "ns_get16@GLIBC_2.9", "__p_class_syms@GLIBC_2.2.5", "_res_opcodes@GLIBC_2.2.5"] {
        assert!(syms.contains(s), "the dynamic program binds {s}:\n{syms}");
    }
    assert!(!syms.contains("LIELLA_PRIVATE"), "no private name reaches the program:\n{syms}");
    assert_eq!(build_run(&dir, "stat", RESOLV, &libs, true).as_deref(), Some(want), "static");
    let _ = std::fs::remove_dir_all(&dir);
}

/// D-16.6 (no spec clause): `libresolv.so.2` defines its versions.tsv exports and nothing of
/// `libc.so.6`'s; each forwarded function is a jump through `libc.so.6`'s `LIELLA_PRIVATE` alias,
/// which `libc.so.6` defines and nothing else names.
#[test]
fn sonames_resolv_forwards_through_private_aliases() {
    let lib = products();
    let r = readelf(&["--dyn-syms", "-W"], &lib.join("libresolv.so.2"));
    assert!(r.contains(" ns_get16@@GLIBC_2.9"), "ns_get16 export:\n{r}");
    assert!(r.contains(" res_gethostbyname@GLIBC_2.2.5"), "compat export (non-default):\n{r}");
    assert!(r.contains("UND __liella_fwd_ns_get16@LIELLA_PRIVATE"), "forwarded through the alias:\n{r}");
    assert!(!r.contains("GLIBC_PRIVATE"), "no GLIBC_PRIVATE:\n{r}");
    let c = readelf(&["--dyn-syms", "-W"], &lib.join("libc.so.6"));
    assert!(c.contains(" __liella_fwd_ns_get16@@LIELLA_PRIVATE"), "libc.so.6 alias:\n");
    assert!(!c.contains(" ns_get16@"), "libc.so.6 must not export ns_get16 under its own name");
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
        if let Some(out) = build_run_with(&dir, name, MBMAX, libs, false, &utf8, None) {
            assert_eq!(out, want, "{name}");
        }
    }
    let needed = readelf(&["-d"], &dir.join("dyn"));
    assert!(needed.contains("[libBrokenLocale.so.1]"), "-lBrokenLocale links libBrokenLocale.so.1:\n{needed}");
    let syms = readelf(&["--dyn-syms", "-W"], &products().join("libBrokenLocale.so.1"));
    assert!(syms.contains(" __ctype_get_mb_cur_max@@GLIBC_2.2.5"), "libBrokenLocale.so.1 defines it:\n{syms}");
    assert_eq!(build_run_with(&dir, "stat", MBMAX, &[], true, &utf8, None).as_deref(), Some("1 6\n"), "static without -lBrokenLocale");
    assert_eq!(build_run_with(&dir, "stat_broken", MBMAX, &["-lBrokenLocale"], true, &utf8, None).as_deref(), Some("1 4\n"), "static -lBrokenLocale");
    // The widths between: the first program shows the code sets' own widths, the second the bound.
    let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/test-locales");
    let wide = [("ko_KR", "EUC-KR"), ("ja_JP", "EUC-JP"), ("zh_CN", "GB18030")];
    let names: Vec<String> = wide.iter().map(|(l, c)| format!("{l}.{c}")).collect();
    if wide.iter().zip(&names).all(|((l, c), n)| oracle::locale(&locales.join(n), l, c, "LC_CTYPE")) {
        let args: Vec<&str> = names.iter().map(String::as_str).collect();
        for (name, libs, want) in [("stat_wide", &[][..], "1 2 3 4\n"), ("stat_broken_wide", &["-lBrokenLocale"][..], "1 1 2 2\n")] {
            assert_eq!(build_run_with(&dir, name, MBMAX, libs, true, &args, Some(&locales)).as_deref(), Some(want), "{name} ({names:?})");
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
