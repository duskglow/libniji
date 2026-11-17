//! Name lookups when no DNS server answers, end to end (spec §16.1, §16.2, §16.4; GitHub issue
//! #8): `tests/oracle/gai_down.c`, built against the host C library and statically against
//! libniji's products, run by `gai_down_run.sh` in private user, mount and network namespaces
//! (no root: `unshare -r`) where only `lo` is up. The outputs must be identical line by line: the
//! network unreachable, the connection refused, timeouts, SERVFAIL, a truncated reply with TCP
//! refused, an empty or missing `resolv.conf`, a missing `/etc/hosts` or `nsswitch.conf`
//! (`getaddrinfo` `EAI_AGAIN`, `gethostbyname` `TRY_AGAIN`), and a server that comes up between
//! two rounds of lookups (libniji's name-service cache, D-16.10, on: nothing found while the
//! server was down is served afterwards). The record is `spec/behaviour/gai-down.md`.
//! Needs the products (`cargo xtask so`), a C compiler, `unshare` and `ip`; without them, or
//! where unprivileged user namespaces are off, the test says so and passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

#[path = "support/timeout.rs"]
mod timeout;
#[path = "support/oracle.rs"]
mod oracle;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build(out: &Path, niji: bool) -> bool {
    let r = root();
    let src = r.join("crates/core/tests/oracle/gai_down.c");
    let mut cmd = Command::new("cc");
    if niji {
        cmd.args(["-std=gnu17", "-O1", "-w", "-static", "-nostdlib", "-fno-builtin"]).arg(r.join("target/niji/niji-crt1.o")).arg(&src).arg(r.join("target/niji/libc.a")).arg("-lgcc");
    } else {
        cmd.args(["-std=gnu17", "-O1", "-w"]).arg(&src);
    }
    oracle::build(cmd, out)
}

fn namespaces() -> bool {
    Command::new("unshare").args(["-r", "-m", "-n", "sh", "-c", "ip link set lo up && mount -t tmpfs tmpfs /dev/shm"]).env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin").status().is_ok_and(|s| s.success())
}

#[test]
fn lookups_without_a_server_equal_the_reference() {
    // Clauses: s16_2_f_0, s16_2_c_1, s16_2_c_4, s16_1_c_0, s16_1_c_2, s16_4_c_0 (D-16.10).
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler");
        return;
    }
    if !namespaces() {
        skip_note::skipped("skipped: no unprivileged user, mount and network namespaces (`unshare -r -m -n`, `ip`)");
        return;
    }
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let (host, ours) = (dir.join("gai_down_host"), dir.join("gai_down_niji"));
    assert!(build(&host, false), "gai_down.c does not build against the host C library");
    assert!(build(&ours, true), "gai_down.c does not link against the products");
    let script = r.join("crates/core/tests/oracle/gai_down_run.sh");
    let run = |exe: &Path, scratch: &str| -> String {
        let o = Command::new("timeout")
            .arg(timeout::secs(600))
            .arg("sh")
            .arg(&script)
            .arg(exe)
            .arg(dir.join(scratch))
            .env_clear()
            .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
            .output()
            .unwrap();
        assert!(o.status.success(), "{} failed: {:?}\n{}", exe.display(), o.status, String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).into_owned()
    };
    let want = run(&host, "gai_down-host");
    let got = run(&ours, "gai_down-niji");
    let mut scenario = "";
    let mut bad = Vec::new();
    for (w, g) in want.lines().zip(got.lines()) {
        if w.starts_with("== ") || w.starts_with("-- ") {
            scenario = w;
        }
        if w != g {
            bad.push(format!("{scenario}\n  reference: {w}\n  libniji:   {g}"));
        }
    }
    assert_eq!(want.lines().count(), got.lines().count(), "line counts differ:\n{want}\n----\n{got}");
    assert!(bad.is_empty(), "{} lines differ:\n{}", bad.len(), bad.join("\n"));
}
