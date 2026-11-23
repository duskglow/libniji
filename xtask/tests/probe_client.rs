//! PROBE-NIJI-TREE: the probe client (`tools/probe/probe`) has no default libniji tree. Running a probe or its
//! self-test without `--niji` must exit 2 with a message naming the option, before it reads the source, builds or
//! runs anything; with `--niji` the option is taken (an absent tree then fails on the tree, not on the option). HOME
//! is a fresh scratch directory, so the client can reach no tree, cache or `~/bin/shared` of the host running the test.
//!
//! Trace: no spec clause (probe tooling).

use std::path::PathBuf;
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("probe-client-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("make a scratch HOME");
    d
}

fn probe(home: &PathBuf, args: &[&str]) -> (Option<i32>, String, String) {
    let client = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tools/probe/probe");
    let out = Command::new("python3")
        .arg(&client)
        .args(args)
        .current_dir(home)
        .env("HOME", home)
        .env("PROBE_CACHE", home.join("cache"))
        .env("PROBE_SLOT", "1")
        .output()
        .expect("run python3 on the probe client");
    (out.status.code(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn a_probe_without_niji_exits_2_naming_the_option() {
    // Trace: no spec clause (probe tooling; PROBE-NIJI-TREE).
    let home = scratch("required");
    // The source does not exist: a client that fell back to a default tree would fail on it, or on the tree.
    for args in [&["no-such-probe.c"][..], &["no-such-probe.c", "--json"][..], &["--selftest"][..]] {
        let (code, stdout, stderr) = probe(&home, args);
        assert_eq!(code, Some(2), "probe {args:?}: stdout {stdout:?} stderr {stderr:?}");
        assert!(stderr.contains("--niji <tree> is required") && stderr.contains("~/w/<role>"), "probe {args:?}: {stderr:?}");
        assert!(stdout.is_empty(), "probe {args:?}: {stdout:?}");
    }
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_probe_with_niji_takes_that_tree() {
    // Trace: no spec clause (probe tooling; PROBE-NIJI-TREE).
    let home = scratch("given");
    let src = home.join("p.c");
    std::fs::write(&src, "// probe client test\nint main(void){ return 0; }\n").expect("write the probe source");
    let tree = home.join("no-such-tree");
    let (code, stdout, stderr) = probe(&home, &[src.to_str().unwrap(), "--niji", tree.to_str().unwrap()]);
    assert_eq!(code, Some(2), "stdout {stdout:?} stderr {stderr:?}");
    assert!(stderr.contains(&format!("no libniji tree at {}", tree.display())), "{stderr:?}");
    assert!(!stderr.contains("is required"), "{stderr:?}");
    let _ = std::fs::remove_dir_all(&home);
}
