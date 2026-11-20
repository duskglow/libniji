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
