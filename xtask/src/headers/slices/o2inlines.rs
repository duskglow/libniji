//! Header-inline bodies and C++ overloads of the call-site names (D-X.158, HEADERS-O2-INLINES). Listed by
//! `xtask/build.rs`.
//!
//! The slice adds no marker: the bodies are hand-written template text under `__NIJI_INLINES` (`headers/features.h.in`)
//! and the C++ overloads hand-written lines with `@@attrs`. This module holds the tests: what an object compiled
//! against libniji's headers references, at which optimization levels, and that no declaration lets the compiler drop
//! an errno test after a call that may set errno.
use super::super::Slice;

pub(super) const SLICE: Slice = Slice { markers: &[], inline: &[], linked_elsewhere: &[] };

#[cfg(test)]
mod tests {
    use super::super::super::generate;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// The generated headers in a scratch directory, and the compiler's own include directory; None (after saying why)
    /// where a tool or the build host's kernel headers are absent.
    struct Headers {
        dir: PathBuf,
        gccinc: String,
    }

    impl Headers {
        fn new(tag: &str, tools: &[&str]) -> Option<Headers> {
            let ok = tools.iter().all(|t| Command::new(t).arg("--version").output().is_ok_and(|o| o.status.success()));
            if !ok || !Path::new("/usr/include/x86_64-linux-gnu").is_dir() {
                eprintln!("skipped: needs {tools:?} and the build host's kernel headers (the generated headers include them)");
                return None;
            }
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
            let (files, errors) = generate(&root).expect("generate");
            assert!(errors.is_empty(), "{errors:?}");
            let dir = std::env::temp_dir().join(format!("niji-o2inl-{tag}.{}", std::process::id()));
            for (h, text) in &files {
                let p = dir.join("inc").join(h);
                fs::create_dir_all(p.parent().unwrap()).unwrap();
                fs::write(&p, text).unwrap();
            }
            let out = Command::new("gcc").arg("-print-file-name=include").output().expect("gcc");
            let gccinc = String::from_utf8(out.stdout).unwrap().trim().to_string();
            Some(Headers { dir, gccinc })
        }

        /// Compiles `src` with `cc` (gcc or g++) and `flags` against the generated headers: the object's undefined
        /// symbols, or the compiler's diagnostics.
        fn compile(&self, k: usize, cc: &str, flags: &str, src: &str) -> Result<Vec<String>, String> {
            let ext = if cc == "g++" { "cc" } else { "c" };
            let file = self.dir.join(format!("u{k}.{ext}"));
            let obj = self.dir.join(format!("u{k}.o"));
            fs::write(&file, src).unwrap();
            let out = Command::new(cc)
                .args(["-c", "-nostdinc", "-isystem"])
                .arg(self.dir.join("inc"))
                .args(["-isystem", &self.gccinc, "-isystem", "/usr/include/x86_64-linux-gnu", "-isystem", "/usr/include"])
                .args(flags.split_whitespace())
                .arg("-o")
                .arg(&obj)
                .arg(&file)
                .output()
                .expect(cc);
            if !out.status.success() {
                return Err(String::from_utf8_lossy(&out.stderr).into_owned());
            }
            let nm = Command::new("nm").arg("-u").arg(&obj).output().expect("nm");
            Ok(String::from_utf8_lossy(&nm.stdout).split_whitespace().filter(|s| *s != "U").map(String::from).collect())
        }
    }

    impl Drop for Headers {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    /// (flags, header, body of the test function, symbols it must reference, symbols it must not)
    type Shape = (&'static str, &'static str, &'static str, &'static [&'static str], &'static [&'static str]);

    // Spec 12.5 (s12_5_c_1 getchar, s12_5_c_5 getline/getdelim, s12_5_c_9 the _unlocked forms), 6.2 (s6_2_c_3 mbrlen
    // is mbrtowc), C 7.22.1.1-2 (atof/atoi/atol/atoll are the strtod/strtol/strtoll calls) and C 7.22.5.1 (bsearch);
    // callsites.tsv's rows for these names. Compiled against libniji's headers, a call references what the
    // reference's objects reference (black-box probe, D-X.158): when optimizing for speed the inline body's callee
    // or nothing at all, and the function itself at -O0, -Os and -fno-inline and through a taken address. A defect
    // here (a body dropped, the wrong level condition, a GNU body outside _GNU_SOURCE) gives a program link needs
    // the reference's headers do not.
    #[test]
    fn a_call_references_what_the_inline_body_calls_when_optimizing_for_speed() {
        let Some(h) = Headers::new("shape", &["gcc", "nm"]) else { return };
        // (flags, headers, body of `long f (FILE *fp, const char *s, char **lp, size_t *np, mbstate_t *ps)`,
        //  symbols it must reference, symbols it must not)
        let cases: &[Shape] = &[
            ("-O2", "stdio.h", "return getchar ();", &["getc", "stdin"], &["getchar"]),
            ("-O0", "stdio.h", "return getchar ();", &["getchar"], &["getc"]),
            ("-Os", "stdio.h", "return getchar ();", &["getchar"], &["getc"]),
            ("-O2 -fno-inline", "stdio.h", "return getchar ();", &["getchar"], &["getc"]),
            ("-O2", "stdio.h", "int (*volatile g) (void) = getchar; return g ();", &["getchar"], &[]),
            ("-O2", "stdio.h", "return putchar ('x');", &["putc", "stdout"], &["putchar"]),
            ("-O2", "stdio.h", "return getc_unlocked (fp) + putc_unlocked ('x', fp);", &["__uflow", "__overflow"], &["getc_unlocked", "putc_unlocked"]),
            ("-O2 -D_GNU_SOURCE", "stdio.h", "return feof_unlocked (fp) + ferror_unlocked (fp);", &[], &["feof_unlocked", "ferror_unlocked"]),
            ("-O2 -D_GNU_SOURCE", "stdio.h", "return getline (lp, np, fp);", &["__getdelim"], &["getline"]),
            ("-O2 -D_POSIX_C_SOURCE=200809L", "stdio.h", "return getline (lp, np, fp);", &["getline"], &["__getdelim"]),
            ("-O2 -std=c17", "stdlib.h", "return atoi (s) + atol (s);", &["strtol"], &["atoi", "atol"]),
            ("-O2 -std=c2x", "stdlib.h", "return atoi (s);", &["__isoc23_strtol"], &["atoi", "strtol"]),
            ("-O2", "stdlib.h", "return atoll (s);", &["strtoll"], &["atoll"]),
            ("-O2", "stdlib.h", "return (long) atof (s);", &["strtod"], &["atof"]),
            ("-O0", "stdlib.h", "return (long) atof (s) + atoi (s);", &["atof", "atoi"], &["strtod", "strtol"]),
            ("-Os", "stdlib.h", "return (long) atof (s);", &["atof"], &["strtod"]),
            ("-O2", "stdlib.h", "return bsearch (s, *lp, *np, 1, (int (*) (const void *, const void *)) fp) != 0;", &[], &["bsearch"]),
            ("-O2", "wchar.h", "return (long) mbrlen (s, *np, ps);", &["mbrtowc", "__mbrlen"], &["mbrlen"]),
            ("-O2", "wchar.h", "return (long) mbrlen (s, *np, 0);", &["__mbrlen"], &["mbrlen", "mbrtowc"]),
        ];
        let mut failures = Vec::new();
        for (k, (flags, hdr, body, want, not)) in cases.iter().enumerate() {
            let src = format!(
                "#include <stdio.h>\n#include <wchar.h>\n#include <{hdr}>\n\
                 long f (FILE *fp, const char *s, char **lp, size_t *np, mbstate_t *ps) {{ (void) fp; (void) s; (void) lp; (void) np; (void) ps; {body} }}\n"
            );
            match h.compile(k, "gcc", flags, &src) {
                Err(e) => failures.push(format!("{flags} {body}: {e}")),
                Ok(syms) => {
                    let missing: Vec<_> = want.iter().filter(|w| !syms.iter().any(|s| s == *w)).collect();
                    let extra: Vec<_> = not.iter().filter(|n| syms.iter().any(|s| s == *n)).collect();
                    if !missing.is_empty() || !extra.is_empty() {
                        failures.push(format!("{flags} {body}: references {syms:?}; missing {missing:?}, unwanted {extra:?}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    // Spec 4.5 (s4_5_c_5 strchrnul) and the GNU and BSD search functions' C++ forms (callsites.tsv's C++ rows; the
    // reference's objects, black-box probe, D-X.158): in C++ each has a const and a non-const overload, so a const
    // argument gives a const result (assigning it to `char *` is an error), a non-const one a `char *`, a call
    // references the C symbol, and `&F` alone is ambiguous. A defect here (one C declaration only, or a wrong asm
    // label) lets a const string be written through, or makes a call reference a mangled name.
    #[test]
    fn the_search_functions_have_const_correct_cxx_overloads() {
        let Some(h) = Headers::new("cxx", &["g++", "nm"]) else { return };
        let pre = "#include <string.h>\n#include <strings.h>\n";
        // (body, Some(symbol a call must reference) or None for a compile-time error)
        let cases: &[(&str, Option<&str>)] = &[
            ("const char *f (const char *s) { return strchrnul (s, 'a'); }", Some("strchrnul")),
            ("char *f (char *s) { return strchrnul (s, 'a'); }", Some("strchrnul")),
            ("char *f (const char *s) { return strchrnul (s, 'a'); }", None),
            ("char *f (char *s) { return strcasestr (s, \"a\"); }", Some("strcasestr")),
            ("char *f (const char *s) { return strcasestr (s, \"a\"); }", None),
            ("void *f (void *s) { return memrchr (s, 'a', 4); }", Some("memrchr")),
            ("void *f (const void *s) { return memrchr (s, 'a', 4); }", None),
            ("const void *f (const void *s) { return rawmemchr (s, 'a'); }", Some("rawmemchr")),
            ("char *f (char *s) { return basename (s); }", Some("basename")),
            ("char *f (const char *s) { return basename (s); }", None),
            ("char *f (char *s) { return index (s, 'a'); }", Some("index")),
            ("char *f (const char *s) { return rindex (s, 'a'); }", None),
            ("void *f () { auto p = &strchrnul; return (void *) p; }", None),
        ];
        let mut failures = Vec::new();
        for (k, (body, want)) in cases.iter().enumerate() {
            for flags in ["-O0 -D_GNU_SOURCE", "-O2 -D_GNU_SOURCE"] {
                match (h.compile(k, "g++", flags, &format!("{pre}{body}\n")), want) {
                    (Ok(_), None) => failures.push(format!("{flags} {body}: compiled, wants a compile-time error")),
                    (Err(_), None) => {}
                    (Err(e), Some(_)) => failures.push(format!("{flags} {body}: {e}")),
                    (Ok(syms), Some(w)) => {
                        if !syms.iter().any(|s| s == w) {
                            failures.push(format!("{flags} {body}: references {syms:?}, wants {w}"));
                        }
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    // Spec 0.6 (s0_6_c0_1_2: a call may change errno) with C 7.22.1.1 / 7.22.1.4 (atof is strtod, which sets ERANGE)
    // and C 7.12.4.1 (acos sets EDOM under math_errhandling & MATH_ERRNO); owed #145, D-X.158. A caller's errno test
    // after the call is kept at every level: with a pure or const declaration the compiler may assume the call leaves
    // errno alone and drop the test (conformance s08_strto_fenv_flags missed ERANGE for atof that way). Opaque
    // arguments, so only the declaration decides.
    #[test]
    fn an_errno_test_after_a_call_that_may_set_errno_is_kept() {
        let Some(h) = Headers::new("errno", &["gcc", "nm"]) else { return };
        let calls = [
            ("stdlib.h", "double", "atof (s)"),
            ("stdlib.h", "int", "atoi (s)"),
            ("stdlib.h", "long", "atol (s)"),
            ("stdlib.h", "long long", "atoll (s)"),
            ("stdlib.h", "double", "strtod (s, 0)"),
            ("math.h", "double", "acos (x)"),
        ];
        let mut failures = Vec::new();
        let mut k = 0;
        for (hdr, ty, call) in calls {
            for level in ["-O2", "-Os"] {
                let src = format!(
                    "#include <{hdr}>\n#include <errno.h>\nextern void mk (void);\nextern double x;\n\
                     {ty} volatile r;\nvoid f (const char *s) {{ (void) s; errno = 0; r = {call}; if (errno != 0) mk (); }}\n"
                );
                k += 1;
                match h.compile(k, "gcc", level, &src) {
                    Err(e) => failures.push(format!("{level} {call}: {e}")),
                    Ok(syms) if !syms.iter().any(|s| s == "mk") => {
                        failures.push(format!("{level} {call}: the errno test was dropped (references {syms:?})"))
                    }
                    Ok(_) => {}
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }
}
