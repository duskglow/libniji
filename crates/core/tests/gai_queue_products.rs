//! `getaddrinfo_a`'s queue and notifications under volume, end to end (spec §16.2;
//! getaddrinfo_a(3); D-16.8): `tests/oracle/gai_queue.c`, built statically against libniji's
//! products. Numeric requests only (no DNS, no root, nothing leaves the process): calls that
//! start part-way round the queue and outgrow it, then threads making one-request calls at once,
//! each notified by a queued signal or a `SIGEV_THREAD` call. Every request must complete with
//! exactly its own answer and every notification come once: the expected lines follow from the
//! man page alone. A healthy run takes well under a second; a lost request is reported
//! after 30 s in which nothing completes (a hang guard, not a time limit on the result).
//! Needs the products (`cargo xtask so`) and a C compiler; without the compiler it says so and
//! passes.
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

fn build(out: &Path) -> bool {
    let r = root();
    let mut cmd = Command::new("cc");
    cmd.args(["-std=gnu17", "-O1", "-w", "-static", "-nostdlib", "-fno-builtin"]).arg(r.join("target/niji/niji-crt1.o")).arg(r.join("crates/core/tests/oracle/gai_queue.c")).arg(r.join("target/niji/libc.a")).arg("-lgcc");
    oracle::build(cmd, out)
}

#[test]
fn every_request_and_notification_arrives_as_the_reference() {
    // Clauses: s16_2_c_5, s16_2_c_1, s16_2_g_0 (D-16.8).
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler");
        return;
    }
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let ours = dir.join("gai_queue_niji");
    assert!(build(&ours),"gai_queue.c does not link against the products");
    let o = Command::new("timeout").arg(timeout::secs(600)).arg(&ours).env_clear().env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin").output().unwrap();
    assert!(o.status.success(), "{} failed: {:?}\n{}", ours.display(), o.status, String::from_utf8_lossy(&o.stderr));
    let got = String::from_utf8_lossy(&o.stdout).into_owned();
    // What getaddrinfo_a(3) promises: every call accepted, every request answered with its own
    // address and port, none lost, every notification once.
    let mut want = String::new();
    for (r, (lead, mass)) in [(1, 100), (37, 3000), (63, 3000), (500, 8000), (3001, 20000), (777, 40000)].into_iter().enumerate() {
        let n = lead + mass;
        want.push_str(&format!("round {r}: {lead} then {mass} requests: calls 0 0, exact {n} of {n}, lost 0\n"));
    }
    want.push_str("concurrent: 4 threads x 2000 calls: refused 0, exact 8000 of 8000, lost 0\n");
    want.push_str("notifications: missing 0, twice 0, bad 0\n");
    assert_eq!(got, want, "getaddrinfo_a lost, mixed up or refused requests, or lost or repeated notifications");
}
