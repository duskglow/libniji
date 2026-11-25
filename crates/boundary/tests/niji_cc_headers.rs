//! The compiler drivers compile against libniji's own C headers (plan/headers.md slice 20):
//! `scripts/niji-cc` with the headers `cargo xtask so` staged, and the packaged `tools/deb/niji-gcc`
//! (and `niji-g++`) with the sysroot's `usr/include`. A C++ compile finds the C++ library's headers
//! first and libniji's C headers under them. `NIJI_SYSTEM_HEADERS=1` restores the system's headers,
//! and a program's own `-nostdinc` gets no directory added.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../core/tests/support/skip_note.rs"]
mod skip_note;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn products() -> PathBuf {
    let p = std::env::var_os("NIJI_PRODUCTS").map_or_else(|| root().join("target/niji"), PathBuf::from);
    assert!(
        p.join("libc.a").exists() && p.join("include/features.h").exists() && p.join("uapi").is_dir(),
        "the products or the staged headers are missing: run `cargo xtask so`"
    );
    p
}

/// `__NIJI__` is defined by libniji's headers only: the program fails to compile on any other set.
const C_PROGRAM: &str = "#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <pty.h>\n\
#ifndef __NIJI__\n#error not libniji's headers\n#endif\n\
int main(void) { char b[16]; strcpy(b, \"niji\"); printf(\"%s %zu\\n\", b, strlen(b)); return EXIT_SUCCESS; }\n";

/// The C++ library's headers include libniji's C headers below them (`<cstdlib>` reaches `stdlib.h`).
/// `<locale.h>` comes first and names `__locale_t` on its own (D-X.169: the C++ library's locale support needs it).
const CXX_PROGRAM: &str = "#include <locale.h>\ntypedef __locale_t lp_locale_t;\n\
#include <cstdio>\n#include <cstdlib>\n#include <cstring>\n#include <cmath>\n\
#include <string>\n#include <vector>\n#include <iostream>\n\
#ifndef __NIJI__\n#error not libniji's headers\n#endif\n\
int main() { std::vector<std::string> v{\"a\"}; std::printf(\"%zu %d\\n\", v.size(), std::abs(-2)); return 0; }\n";

/// Without libniji's headers: compiles only where `__NIJI__` is not defined.
const SYSTEM_PROGRAM: &str = "#include <stdio.h>\n#ifdef __NIJI__\n#error libniji's headers\n#endif\nint main(void) { return 0; }\n";

fn scratch(name: &str) -> PathBuf {
    let dir = root().join("target/test-scratch/niji_cc_headers").join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("p.c"), C_PROGRAM).unwrap();
    std::fs::write(dir.join("p.cc"), CXX_PROGRAM).unwrap();
    std::fs::write(dir.join("s.c"), SYSTEM_PROGRAM).unwrap();
    dir
}

fn run(cc: &Path, dir: &Path, env: &[(&str, &str)], args: &[&str]) -> Output {
    let mut cmd = Command::new(cc);
    cmd.current_dir(dir).env("NIJI_PRODUCTS", products()).env_remove("NIJI_SYSTEM_HEADERS").env_remove("NIJI_CC_EXTRA");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.args(args).output().unwrap()
}

fn ok(o: &Output, what: &str) {
    assert!(o.status.success(), "{what} failed:\n{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn niji_cc_compiles_and_links_c_against_libniji_headers() {
    // Trace: no spec clause (validation driver scripts/niji-cc; plan/headers.md slice 20).
    let dir = scratch("c");
    let cc = root().join("scripts/niji-cc");
    ok(&run(&cc, &dir, &[], &["-O1", "-c", "p.c", "-o", "p.o"]), "niji-cc -c p.c");
    ok(&run(&cc, &dir, &[], &["-O1", "p.c", "-o", "p"]), "niji-cc p.c -o p");
    let out = Command::new(dir.join("p")).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "niji 4\n");
}

#[test]
fn niji_cc_compiles_cxx_with_the_cxx_library_above_libniji_headers() {
    // Trace: no spec clause (plan/headers.md P4: libstdc++'s <cstdlib> and friends over libniji's C headers).
    let dir = scratch("cxx");
    let cc = root().join("scripts/niji-cc");
    ok(&run(&cc, &dir, &[], &["-O1", "-c", "p.cc", "-o", "a.o"]), "niji-cc -c p.cc");
    ok(&run(&cc, &dir, &[], &["-O1", "-x", "c++", "-c", "p.cc", "-o", "b.o"]), "niji-cc -x c++ -c p.cc");
    ok(&run(&cc, &dir, &[("NIJI_REAL_CC", "g++")], &["-O1", "-c", "p.cc", "-o", "c.o"]), "NIJI_REAL_CC=g++ niji-cc -c p.cc");
}

#[test]
fn niji_cc_system_headers_and_nostdinc_add_no_libniji_directory() {
    // Trace: no spec clause (scripts/niji-cc: NIJI_SYSTEM_HEADERS=1 and a program's own -nostdinc).
    let dir = scratch("sys");
    let cc = root().join("scripts/niji-cc");
    ok(&run(&cc, &dir, &[("NIJI_SYSTEM_HEADERS", "1")], &["-c", "s.c", "-o", "s.o"]), "NIJI_SYSTEM_HEADERS=1 niji-cc -c s.c");
    // p.c compiles on libniji's headers only, so a driver that added them under -nostdinc compiles it.
    let o = run(&cc, &dir, &[], &["-nostdinc", "-c", "p.c", "-o", "n.o"]);
    assert!(!o.status.success(), "niji-cc -nostdinc found <stdio.h>: the driver added a header directory");
    let o = run(&cc, &dir, &[], &["-c", "s.c", "-o", "d.o"]);
    assert!(!o.status.success(), "niji-cc -c s.c compiled without __NIJI__: not libniji's headers");
}

/// A sysroot laid out as niji6-dev installs it: `usr/include` holds libniji's headers and the
/// kernel UAPI directories (links to the staged copies here).
fn sysroot(dir: &Path) -> PathBuf {
    let p = products();
    let inc = dir.join("sysroot/usr/include");
    std::fs::create_dir_all(inc.join("x86_64-linux-gnu")).unwrap();
    for tree in ["include", "uapi"] {
        let mut names: Vec<_> = std::fs::read_dir(p.join(tree)).unwrap().map(|e| e.unwrap().file_name()).collect();
        names.sort();
        for n in names {
            std::os::unix::fs::symlink(p.join(tree).join(&n), inc.join(&n)).unwrap();
        }
    }
    dir.join("sysroot")
}

#[test]
fn packaged_niji_gcc_compiles_against_the_sysroot_headers() {
    // Trace: no spec clause (tools/deb/niji-gcc, package niji6-dev; plan/headers.md slice 20).
    let dir = scratch("deb");
    let root_dir = sysroot(&dir);
    let sr = root_dir.to_str().unwrap();
    let gcc = root().join("tools/deb/niji-gcc");
    let gxx = dir.join("niji-g++");
    std::os::unix::fs::symlink(&gcc, &gxx).unwrap();
    ok(&run(&gcc, &dir, &[("NIJI_SYSROOT", sr)], &["-O1", "-c", "p.c", "-o", "p.o"]), "niji-gcc -c p.c");
    ok(&run(&gxx, &dir, &[("NIJI_SYSROOT", sr)], &["-O1", "-c", "p.cc", "-o", "q.o"]), "niji-g++ -c p.cc");
    ok(&run(&gcc, &dir, &[("NIJI_SYSROOT", sr), ("NIJI_SYSTEM_HEADERS", "1")], &["-c", "s.c", "-o", "s.o"]), "NIJI_SYSTEM_HEADERS=1 niji-gcc -c s.c");
    let o = run(&gcc, &dir, &[("NIJI_SYSROOT", sr)], &["-c", "s.c", "-o", "d.o"]);
    assert!(!o.status.success(), "niji-gcc -c s.c compiled without __NIJI__: not libniji's headers");
}

/// The scalar functions libmvec.so.1 has vector variants of (spec/abi/versions.tsv), each with its
/// parameter count (`v`, `vv`, or `vvv` for the sincos shape).
fn libmvec_scalars() -> Vec<(String, usize)> {
    let text = std::fs::read_to_string(root().join("spec/abi/versions.tsv")).unwrap();
    let mut out: Vec<(String, usize)> = text
        .lines()
        .filter_map(|l| l.strip_prefix("libmvec.so.1\t_ZGVbN"))
        .filter_map(|l| {
            let name = l.split('\t').next()?.trim_start_matches(|c: char| c.is_ascii_digit());
            let (params, f) = name.split_once('_')?;
            Some((f.to_string(), params.len()))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The `_ZGV` names (sorted, space-separated; empty for none) in the assembly `cc` makes of a loop
/// over `f` in `c[i] = f(a[i]...)` form, compiled `-O3` with `flags`. `omp`: `#pragma omp simd` on the loop.
fn vector_calls(cc: &Path, dir: &Path, f: &str, params: usize, float: bool, omp: bool, flags: &[&str]) -> String {
    let t = if float { "float" } else { "double" };
    let call = match params {
        1 => format!("c[i] = {f}(a[i]);"),
        2 => format!("c[i] = {f}(a[i], b[i]);"),
        _ => format!("{f}(a[i], &b[i], &c[i]);"),
    };
    let pragma = if omp { "#pragma omp simd\n" } else { "" };
    let src = format!("#define _GNU_SOURCE 1\n#include <math.h>\n{t} a[1024], b[1024], c[1024];\nvoid k(void) {{\n  int i;\n{pragma}  for (i = 0; i < 1024; i++) {call}\n}}\n");
    std::fs::write(dir.join("v.c"), src).unwrap();
    let o = run(cc, dir, &[], &[&["-O3", "-S", "-o", "-"], flags, &["v.c"]].concat());
    ok(&o, &format!("{} -O3 {flags:?} on a loop over {f}", cc.display()));
    let asm = String::from_utf8_lossy(&o.stdout);
    let mut names: Vec<&str> = asm.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).filter(|w| w.starts_with("_ZGV")).collect();
    names.sort_unstable();
    names.dedup();
    names.join(" ")
}

#[test]
fn niji_cc_loops_call_the_libmvec_variants_the_reference_calls() {
    // Trace: no spec clause (D-X.170; issue #28: vector math through libmvec.so.1 for programs built on
    // libniji's headers). For every function with libmvec variants: the variants gcc calls in a loop
    // with libniji's math.h are those it calls with the reference's, under -ffast-math (SSE and
    // AVX-512 widths, C and C++), none without it, and the strict-standard simd loop of -std=c89.
    let dir = scratch("simd");
    let niji = root().join("scripts/niji-cc");
    let reference = PathBuf::from("gcc");
    let scalars = libmvec_scalars();
    assert!(scalars.len() >= 50, "spec/abi/versions.tsv lists {} libmvec scalar functions", scalars.len());
    let configs: [(&str, bool, &[&str]); 5] = [
        ("fast", false, &["-ffast-math"]),
        ("fast-avx512", false, &["-ffast-math", "-mavx512f"]),
        ("not-fast", false, &[]),
        ("fast-c++", false, &["-x", "c++", "-ffast-math"]),
        ("c89-omp", true, &["-std=c89", "-ffast-math", "-fopenmp-simd"]),
    ];
    let mut differ = Vec::new();
    let mut reference_calls = 0;
    for (f, params) in &scalars {
        // `sinf` is the float form of `sin`; `erf` is a double function that ends in f.
        let float = f.strip_suffix('f').is_some_and(|d| scalars.iter().any(|(s, _)| s == d));
        for (name, omp, flags) in &configs {
            let want = vector_calls(&reference, &dir, f, *params, float, *omp, flags);
            let got = vector_calls(&niji, &dir, f, *params, float, *omp, flags);
            reference_calls += usize::from(!want.is_empty());
            if got != want {
                differ.push(format!("{f} {name}: libniji [{got}], reference [{want}]"));
            }
        }
    }
    if reference_calls == 0 {
        skip_note::skipped("skipped: the reference's headers make gcc call no vector variant here");
        return;
    }
    assert!(differ.is_empty(), "{} differences:\n{}", differ.len(), differ.join("\n"));
}

/// Programs on `<complex.h>` and `<tgmath.h>` (SL-HEADERS-COMPLEX-TGMATH): (name, C++, compiler flags,
/// source, expected output). Shapes of the sources that include them: a configure run test of csqrt,
/// a C99 detection program, the header-coexistence compile test of a TLS library, a tgmath user, the
/// double-complex get/set of a multiprecision library, a binding generator's complex types, and a
/// Python-extension compiler's C and C++ complex code. Every exact result is exact in any correct
/// implementation (square roots of squares, products of small integers).
type Program = (&'static str, bool, &'static [&'static str], &'static str, &'static str);

const COMPLEX_PROGRAMS: [Program; 10] = [
    (
        "complex-c11",
        false,
        &["-std=c11"],
        "#include <complex.h>\n#include <stdio.h>\n\
static const double complex k = CMPLX (1.5, -2.0);\n\
int main (void) { volatile double complex z = -4.0; double complex r = csqrt (z); float complex f = CMPLXF (3.0f, 4.0f);\n\
double complex p = cpow (CMPLX (2.0, 0.0), CMPLX (10.0, 0.0));\n\
printf (\"%g %g|%g %g|%g|%g %g|%d %d|%.6g\\n\", creal (r), cimag (r), creal (k), cimag (k), cabsf (f), creal (conj (k)),\n\
cimag (conj (k)), (int) sizeof (I), (int) sizeof (CMPLXL (1, 2)), creal (p)); return 0; }\n",
        "0 2|1.5 -2|5|1.5 2|8 32|1024\n",
    ),
    (
        "tgmath-gnu17",
        false,
        &[],
        "#include <tgmath.h>\n#include <stdio.h>\n\
#define T(e) _Generic ((e), float: \"f\", double: \"d\", long double: \"l\", float complex: \"cf\", double complex: \"cd\", \\\n\
long double complex: \"cl\", default: \"?\")\n\
int main (void) { volatile float x = 2.0f; volatile long double y = 2.0L; volatile int n = 2; volatile double complex z = 3.0 + 4.0 * I;\n\
printf (\"%s %s %s %s %s %s %s\\n\", T (sqrt (x)), T (sqrt (y)), T (sqrt (n)), T (sqrt (z)), T (fabs (z)), T (pow (x, y)), T (pow (x, z)));\n\
printf (\"%g %Lg %g %g %g\\n\", (double) sqrt (x * 2), sqrt (y * 8), fabs (z), cimag (sqrt (z * 0 - 4)), (double) ldexp (x, 3)); return 0; }\n",
        "f l d cd d l cd\n2 4 5 2 16\n",
    ),
    (
        "tgmath-user",
        false,
        &["-std=gnu11", "-Wall", "-Werror"],
        "#include <stdint.h>\n#include <stdio.h>\n#include <tgmath.h>\n\
int main (void) { volatile uint64_t sz = 4097; printf (\"%d\\n\", (int) ceil (log2 ((double) sz))); return 0; }\n",
        "13\n",
    ),
    (
        "coexist-c99",
        false,
        &["-std=c99", "-pedantic-errors", "-Wall", "-Werror"],
        "#if defined (__STDC_VERSION__) && __STDC_VERSION__ >= 199901L && !defined (__STDC_NO_COMPLEX__)\n\
#include <complex.h>\n#include <tgmath.h>\n#endif\n#include <stdio.h>\n\
int main (void) { puts (\"ok\"); return 0; }\n",
        "ok\n",
    ),
    (
        "c99-detect",
        false,
        &["-std=c99"],
        "#include <complex.h>\n#include <tgmath.h>\n#include <stdbool.h>\n#include <stdio.h>\n\
int main (void) { _Bool b = true; double complex c = 1.0 + 1.0 * I; c = c * c; for (int i = 0; i < 1; i++) b = !b;\n\
printf (\"%g %g %d\\n\", creal (c), cimag (c), b); return 0; }\n",
        "0 2 0\n",
    ),
    (
        "binding-types",
        false,
        &[],
        "#include <complex.h>\n#include <stdio.h>\n\
int main (void) { float complex f = 1.0f + 2.0f * I; long double complex l = conjl (3.0L + 4.0L * I);\n\
printf (\"%g %g %Lg %Lg %Lg\\n\", crealf (f), cimagf (f), creall (l), cimagl (l), cabsl (l)); return 0; }\n",
        "1 2 3 -4 5\n",
    ),
    (
        "get-set-dc",
        false,
        &[],
        "#include <complex.h>\n#include <stdio.h>\n\
static double _Complex get_dc (double re, double im) { return CMPLX (re, im); }\n\
static void set_dc (double *re, double *im, double _Complex z) { *re = creal (z); *im = cimag (z); }\n\
int main (void) { double re, im; set_dc (&re, &im, get_dc (0.5, -0.25) * I); printf (\"%g %g\\n\", re, im); return 0; }\n",
        "0.25 0.5\n",
    ),
    (
        "csqrt-run-test",
        false,
        &[],
        "#include <complex.h>\n#include <stdio.h>\n\
int main (void) { volatile double complex z = -1.0; double complex r = csqrt (z);\n\
if (!(creal (r) == 0.0 && cimag (r) == 1.0)) return 1; puts (\"ok\"); return 0; }\n",
        "ok\n",
    ),
    (
        "python-ext-c",
        false,
        &[],
        "#include <complex.h>\n#include <stdio.h>\ntypedef double _Complex pyx_dc;\n\
int main (void) { pyx_dc a = 1.0 + 2.0 * _Complex_I, b = a * conj (a);\n\
printf (\"%g %g %g\\n\", creal (b), cimag (b), cabs (a * 0 + 3 + 4 * I)); return 0; }\n",
        "5 0 5\n",
    ),
    (
        "cxx98-complex.h",
        true,
        &["-std=c++98"],
        "#include <complex.h>\n#include <stdio.h>\n\
int main () { double complex z = -4.0; double complex r = csqrt (z);\n\
printf (\"%g %g %d\\n\", __real__ r, __imag__ r, (int) sizeof (I)); return 0; }\n",
        "0 2 8\n",
    ),
];

/// The Python-extension compiler's C++ form: `<complex>` (the C++ library's `<complex.h>` and
/// `<tgmath.h>` take it in C++11 and later) over libniji's C headers.
const COMPLEX_CXX17: &str = "#include <complex>\n#include <complex.h>\n#include <tgmath.h>\n#include <cstdio>\n\
int main () { std::complex<double> a (1.0, 2.0); std::complex<double> b = a * std::conj (a);\n\
std::complex<double> s = std::sqrt (std::complex<double> (-4.0, 0.0));\n\
std::printf (\"%g %g %g %g %g\\n\", b.real (), b.imag (), s.real (), s.imag (), std::abs (std::complex<double> (3, 4))); return 0; }\n";

#[test]
fn complex_and_tgmath_programs_match_the_reference() {
    // Trace: no spec clause (plan/headers.md slice 11's complex.h and tgmath.h; C 7.3 and 7.25 results,
    // the results of the same sources built on the reference's headers and library). C programs are
    // linked with libniji (niji-cc) and run; the C++ ones compile with niji-cc and the packaged
    // niji-g++ (the C++ library's run time is the system's), and run on the reference.
    let dir = scratch("complex");
    let niji = root().join("scripts/niji-cc");
    let root_dir = sysroot(&dir);
    let sr = root_dir.to_str().unwrap();
    let gcc = root().join("tools/deb/niji-gcc");
    let gxx = dir.join("niji-g++");
    std::os::unix::fs::symlink(&gcc, &gxx).unwrap();
    let run_prog = |exe: &str| {
        let o = Command::new(dir.join(exe)).output().unwrap();
        assert!(o.status.success(), "{exe} exited with {:?}", o.status);
        String::from_utf8_lossy(&o.stdout).into_owned()
    };
    let mut programs: Vec<Program> = COMPLEX_PROGRAMS.to_vec();
    programs.push(("python-ext-cxx17", true, &["-std=c++17"][..], COMPLEX_CXX17, "5 0 0 2 5\n"));
    for (name, cxx, flags, src, want) in programs {
        let file = format!("{name}.{}", if cxx { "cc" } else { "c" });
        std::fs::write(dir.join(&file), src).unwrap();
        let reference = if cxx { "g++" } else { "gcc" };
        let mut args: Vec<&str> = flags.to_vec();
        args.extend([file.as_str(), "-o", "ref", "-lm"]);
        let o = Command::new(reference).current_dir(&dir).args(&args).output().unwrap();
        ok(&o, &format!("{reference} {name} (the reference)"));
        assert_eq!(run_prog("ref"), want, "{name} on the reference");
        let mut args: Vec<&str> = flags.to_vec();
        args.extend(["-c", file.as_str(), "-o", "p.o"]);
        ok(&run(&niji, &dir, &[], &args), &format!("niji-cc -c {name}"));
        ok(&run(if cxx { &gxx } else { &gcc }, &dir, &[("NIJI_SYSROOT", sr)], &args), &format!("packaged niji-gcc -c {name}"));
        if !cxx {
            let mut args: Vec<&str> = flags.to_vec();
            args.extend([file.as_str(), "-o", "niji", "-lm"]);
            ok(&run(&niji, &dir, &[], &args), &format!("niji-cc {name}"));
            assert_eq!(run_prog("niji"), want, "{name} with libniji");
        }
    }
}
