//! The gate's second pass runs its test processes as libniji processes (PASS2-NATIVE-RUNNER,
//! D-X.135): `cargo xtask test` and `cargo xtask affected --run` set `NIJI_TEST_PASS=native` on that
//! pass and run each test binary's copy through the runner, so the process maps the prefixed loader
//! (`/tmp/niji2-<hex>/ld.so`) and libniji's `libc.so.6` from that root's `lib/`. Without the variable
//! (the first pass, the interposed suite, a plain `cargo test`) the process maps neither. A pass that
//! lost its runner would run the host's processes again and still pass every other test.
#![allow(clippy::unwrap_used, clippy::panic)]

#[test]
fn the_native_pass_runs_libniji_processes() {
    // Trace: no spec clause (the gate's second pass as libniji processes, D-X.135).
    let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
    let files: Vec<&str> = maps.lines().filter_map(|l| l.split_whitespace().nth(5)).collect();
    let loader = files.iter().find(|f| f.starts_with("/tmp/niji2-") && f.ends_with("/ld.so"));
    let native = std::env::var("NIJI_TEST_PASS").as_deref() == Ok("native");
    if native {
        let root = loader.unwrap_or_else(|| panic!("the native pass's process maps no prefixed loader: {files:?}")).trim_end_matches("/ld.so");
        let libc = format!("{root}/lib/libc.so.6");
        assert!(files.iter().any(|f| *f == libc || f.ends_with("/target/niji/libc.so.6")), "the native pass's process maps no libniji libc.so.6 from {root}/lib: {files:?}");
        let exe = std::env::current_exe().unwrap();
        assert!(exe.parent().is_some_and(|d| d.ends_with("niji-deps")), "the native pass runs {} instead of its copy", exe.display());
    } else {
        assert!(loader.is_none(), "a process outside the native pass maps the prefixed loader {loader:?}");
    }
}

#[test]
fn every_pass_libniji_serves_the_c_family_in_poisons_its_fresh_blocks() {
    // Trace: no spec clause (PASS2-NATIVE-POISON, D-X.137: the test processes poison C-family blocks
    // wherever libniji's heap serves them, so a read of a fresh or freed block fails in both passes).
    // Natively the products' heap serves the C family (`MALLOC_PERTURB_` from the runner); with the
    // exports interposed, the test binary's own heap (`NIJI_TEST_MALLOC_PERTURB`). In the first pass
    // the host's heap serves it: nothing of libniji's to check.
    let native = std::env::var("NIJI_TEST_PASS").as_deref() == Ok("native");
    if !native && !cfg!(feature = "export") {
        return;
    }
    let byte = std::env::var("NIJI_TEST_MALLOC_PERTURB").ok().and_then(|v| v.parse::<u32>().ok()).map(|v| v.to_le_bytes()[0]);
    let byte = byte.unwrap_or_else(|| panic!("the second pass runs without NIJI_TEST_MALLOC_PERTURB: its C-family blocks are not poisoned"));
    for n in [1, 24, 200, 5000] {
        assert_eq!(niji_boundary::selftest::c_family_fresh_fill(n), Some(!byte), "a fresh {n}-byte C-family block does not hold the complement of the poison byte {byte}");
    }
}
