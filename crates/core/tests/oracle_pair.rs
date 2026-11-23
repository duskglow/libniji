//! The shared harness of the products tests in `support/oracle.rs`: the link pair
//! ([`oracle::build_pair`]: one C source against the host C library and statically against
//! libniji's products) and the known-differences filter ([`oracle::with_headers`],
//! [`oracle::without`]). Every `*_products.rs` test that compares libniji with the reference
//! goes through them, so a defect here (a library left off the link line, a filter that takes a
//! line no entry records) would weaken all of them at once; these tests pin both down without
//! a compiler or the products.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]
#[path = "support/oracle.rs"]
mod oracle;
use oracle::{build_pair, root, with_headers, without};

/// The command line of one side, the repository root written as `<root>`.
fn argv(pair: oracle::Pair<'_>, niji: bool) -> Vec<String> {
    let r = root().display().to_string();
    let cmd = pair.command(niji);
    assert_eq!(cmd.get_program(), "cc");
    cmd.get_args().map(|a| a.to_str().unwrap().replace(&r, "<root>")).collect()
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

const SRC: &str = "crates/core/tests/oracle/t.c";

#[test]
fn build_pair_links_the_same_source_against_each_library() {
    // Trace: no spec clause (test-harness behaviour: the products link line, PRODUCTS-HARNESS-SHARED).
    let p = build_pair(SRC);
    assert_eq!(argv(p, false), strs(&["-std=gnu17", "-O1", "<root>/crates/core/tests/oracle/t.c"]));
    assert_eq!(
        argv(p, true),
        strs(&["-std=gnu17", "-O1", "-static", "-nostdlib", "-fno-builtin", "<root>/target/niji/niji-crt1.o", "<root>/crates/core/tests/oracle/t.c", "<root>/target/niji/libc.a", "-lgcc"])
    );
}

#[test]
fn build_pair_keeps_each_tests_flags_and_libraries() {
    // Trace: no spec clause (test-harness behaviour: per-test flags and libraries, PRODUCTS-HARNESS-SHARED).
    let math = build_pair(SRC).libm();
    assert_eq!(argv(math, false), strs(&["-std=gnu17", "-O1", "-fno-builtin", "<root>/crates/core/tests/oracle/t.c", "-lm"]));
    assert_eq!(
        argv(math, true),
        strs(&["-std=gnu17", "-O1", "-static", "-nostdlib", "-fno-builtin", "<root>/target/niji/niji-crt1.o", "<root>/crates/core/tests/oracle/t.c", "<root>/target/niji/libm.a", "<root>/target/niji/libc.a", "-lgcc"])
    );
    let resolv = build_pair(SRC).flags(&["-w"]).host_libs(&["-lresolv"]).niji_libs(&["-lresolv"]).niji_flags(&["-DNIJI_PRODUCTS"]);
    assert_eq!(argv(resolv, false), strs(&["-std=gnu17", "-O1", "-w", "<root>/crates/core/tests/oracle/t.c", "-lresolv"]));
    assert_eq!(
        argv(resolv, true),
        strs(&[
            "-std=gnu17",
            "-O1",
            "-w",
            "-static",
            "-nostdlib",
            "-fno-builtin",
            "-DNIJI_PRODUCTS",
            "<root>/target/niji/niji-crt1.o",
            "<root>/crates/core/tests/oracle/t.c",
            "<root>/target/niji/libc.a",
            "-L",
            "<root>/target/niji",
            "-lresolv",
            "-lgcc",
        ])
    );
    let profiled = build_pair(SRC).flags(&["-pg"]).host_flags(&["-static"]).crt("niji-gcrt1.o");
    assert_eq!(argv(profiled, false), strs(&["-std=gnu17", "-O1", "-pg", "-static", "<root>/crates/core/tests/oracle/t.c"]));
    assert_eq!(
        argv(profiled, true),
        strs(&["-std=gnu17", "-O1", "-pg", "-static", "-nostdlib", "-fno-builtin", "<root>/target/niji/niji-gcrt1.o", "<root>/crates/core/tests/oracle/t.c", "<root>/target/niji/libc.a", "-lgcc"])
    );
}

#[test]
fn build_pair_quiet_reports_a_failed_build_as_false() {
    // Trace: no spec clause (test-harness behaviour: a build that may fail by design, PRODUCTS-HARNESS-SHARED).
    // A source that does not exist: the compiler starts and fails. Quiet, that is `false` for the
    // caller to judge; not quiet, it is a test failure (TEST-SILENT-SKIPS).
    let out = root().join("target/test-oracles/oracle-pair-missing");
    let missing = build_pair("crates/core/tests/oracle/oracle-pair-no-such-source.c");
    assert!(!missing.quiet().build(&out, false));
    let loud = std::panic::catch_unwind(|| missing.build(&out, false));
    if std::process::Command::new("cc").arg("--version").output().is_ok() {
        assert!(loud.is_err(), "a failed build that is not quiet must fail the test");
    }
}

const OUT: &str ="== open\nok 1\n-- twice\nsame\nsame\n== close\nsame\nok 2\n";

#[test]
fn with_headers_gives_each_line_the_last_header_above_it() {
    // Trace: no spec clause (test-harness behaviour: the case a recorded difference names, PRODUCTS-HARNESS-SHARED).
    let both: Vec<(String, String)> = with_headers(OUT, &["== ", "-- "]);
    let heads: Vec<&str> = both.iter().map(|(h, _)| h.as_str()).collect();
    assert_eq!(heads, ["== open\n", "== open\n", "-- twice\n", "-- twice\n", "-- twice\n", "== close\n", "== close\n", "== close\n"]);
    let sections = with_headers(OUT, &["== "]);
    assert_eq!(sections[4], ("== open\n".to_string(), "same".to_string()));
    assert!(with_headers("before\n== a\n", &["== "])[0].0.is_empty());
}

#[test]
fn without_takes_one_line_per_entry_and_only_in_its_case() {
    // Trace: no spec clause (test-harness behaviour: the known-differences filter, PRODUCTS-HARNESS-SHARED).
    let lines = with_headers(OUT, &["== ", "-- "]);
    // One entry takes the first `same` of its case only: the second, and the one in `== close`, stay.
    let left = without(lines.clone(), &[("-- twice", "same", "one")], "libniji");
    assert_eq!(text(&left), ["== open", "ok 1", "-- twice", "same", "== close", "same", "ok 2"]);
    // Two entries take both lines of the case; still none of another case.
    let left = without(lines.clone(), &[("-- twice", "same", "one"), ("-- twice", "same", "two")], "libniji");
    assert_eq!(text(&left), ["== open", "ok 1", "-- twice", "== close", "same", "ok 2"]);
    // An entry matches by the start of the line and of the header.
    let left = without(lines, &[("== cl", "ok", "prefix")], "libniji");
    assert_eq!(text(&left), ["== open", "ok 1", "-- twice", "same", "same", "== close", "same"]);
}

#[test]
fn without_fails_on_a_recorded_line_no_longer_printed() {
    // Trace: no spec clause (test-harness behaviour: a stale record fails, PRODUCTS-HARNESS-SHARED).
    let lines = with_headers(OUT, &["== ", "-- "]);
    let stale = std::panic::catch_unwind(|| without(lines.clone(), &[("== open", "same", "not in this case")], "libniji"));
    let msg = stale.expect_err("an entry that takes no line must fail").downcast::<String>().unwrap();
    assert!(msg.contains("recorded line no longer printed by libniji"), "{msg}");
    let extra = std::panic::catch_unwind(|| without(lines.clone(), &[("-- twice", "same", "a"), ("-- twice", "same", "b"), ("-- twice", "same", "c")], "libniji"));
    assert!(extra.is_err(), "three entries for two lines must fail");
}
