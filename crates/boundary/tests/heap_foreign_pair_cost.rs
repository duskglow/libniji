//! The cost of libniji's `malloc`/`free` pair on the foreign path (PASS2-NATIVE-RUNNER): in a
//! process another C library started (this test process, under the host's loader or with the
//! exports interposed), every C entry of the heap takes its general path, which asks whose block it
//! is (`heap::DEBUG` starts `FOREIGN`, D-X.60). The gate's second pass now runs as libniji
//! processes, so this bound keeps the foreign path's cost in view: a slower ownership check shows
//! here as a count, not as a slower gate.
//!
//! How: a small C launcher forks, attaches a user-mode instruction counter to the child
//! (`perf_event_open`, `inherit` so the test thread counts, `enable_on_exec` so the launcher's own
//! code does not) and lets it execute this binary again on this test, which then makes `n` pairs
//! of `posix::s03_memory::{malloc, free}` (64 bytes) and returns. Two runs, `FEW` and `MANY` pairs:
//! the difference of their counts over the difference of `n` is the steady pair's cost, the process
//! start and exit cancelling out. Instruction counts, no wall clock. `NIJI_TEST_MALLOC_PERTURB` is
//! removed for the child: the bound is the plain pair's, not the poisoning fill's.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::as_conversions)]

use std::path::PathBuf;
use std::process::Command;

use niji_boundary::posix::s03_memory as m;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The child's switch: the number of pairs to make.
const CHILD: &str = "NIJI_PAIR_COST_CHILD";

/// Pairs in the short and the long run.
const FEW: u64 = 1_000;
const MANY: u64 = 201_000;

/// The most user-mode instructions one foreign `malloc(64)`/`free` pair may cost (with the loop's
/// own few), in the test profile (unoptimised: the count is the test build's, not the products').
/// Measured on a Zen 1 host (60 vCPU), 2026-11-20: 1911 in the first pass's build, 2063 with the
/// exports interposed; the bound is the higher plus 37 (under 2%).
const PAIR_BOUND: u64 = 2100;

/// The launcher: `launcher <program> <args>...` runs the program in a counted child and prints its
/// count as `count <n>` after the child's own output, or `noperf <errno>` when the counter is refused.
const LAUNCHER: &str = r#"
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
long syscall(long, ...);
int main(int argc, char **argv) {
    int go[2];
    if (argc < 2 || pipe(go) != 0) return 2;
    fflush(stdout);
    pid_t pid = fork();
    if (pid < 0) return 2;
    if (pid == 0) {
        char c;
        close(go[1]);
        if (read(go[0], &c, 1) != 1) _exit(3);
        close(go[0]);
        execv(argv[1], argv + 1);
        _exit(4);
    }
    close(go[0]);
    /* perf_event_attr: PERF_TYPE_HARDWARE and size 64 in word 0, PERF_COUNT_HW_INSTRUCTIONS in
       word 1; word 5: disabled | inherit | exclude_kernel | exclude_hv | enable_on_exec.
       298 = SYS_perf_event_open on x86-64. */
    uint64_t attr[16] = {0};
    attr[0] = (uint64_t)64 << 32;
    attr[1] = 1;
    attr[5] = 1 | 1 << 1 | 1 << 5 | 1 << 6 | 1 << 12;
    long fd = syscall(298, attr, (long)pid, -1L, -1L, 0L);
    int e = errno;
    if (write(go[1], "g", 1) != 1) return 2;
    close(go[1]);
    int st;
    if (waitpid(pid, &st, 0) != pid) return 2;
    if (!WIFEXITED(st) || WEXITSTATUS(st) != 0) { printf("child failed, status %d\n", st); return 1; }
    if (fd < 0) { printf("noperf %d\n", e); return 0; }
    uint64_t n = 0;
    if (read(fd, &n, sizeof n) != sizeof n) { printf("noperf read %d\n", errno); return 0; }
    printf("count %llu\n", (unsigned long long)n);
    return 0;
}
"#;

/// `n` pairs of libniji's `malloc(64)` and `free`, through the Rust entries (the foreign path).
// opaque-ok: the bound counts the entries themselves, without a wrapper's frame; every pointer escapes
// through `sink` and the size through `black_box`, and the count (about 1,900 per pair) shows no pair elided.
fn pairs(n: u64) {
    let mut sink = 0usize;
    for _ in 0..n {
        let p = m::malloc(std::hint::black_box(64));
        assert!(!p.is_null(), "malloc(64) failed");
        sink ^= p as usize;
        m::free(p);
    }
    std::hint::black_box(sink);
}

/// The launcher's output for `n` pairs: the child's and its own, or `None` when no counter.
fn counted(launcher: &std::path::Path, n: u64) -> Option<u64> {
    let me = std::env::current_exe().unwrap();
    let out = Command::new(launcher)
        .arg(&me)
        .args(["--exact", "foreign_malloc_free_pair_cost", "--include-ignored", "--nocapture", "--test-threads", "1"])
        .env(CHILD, n.to_string())
        .env_remove("NIJI_TEST_MALLOC_PERTURB")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "launcher failed ({}): {text}{}", out.status, String::from_utf8_lossy(&out.stderr));
    let last = text.lines().last().unwrap_or_default();
    if last.starts_with("noperf") {
        println!("SKIP: no user-mode instruction counter for this process ({last}): perf_event_open refused");
        return None;
    }
    assert!(text.contains("running 1 test"), "the child ran no test: {text}");
    Some(last.strip_prefix("count ").and_then(|v| v.parse().ok()).unwrap_or_else(|| panic!("unexpected launcher output {text:?}")))
}

#[test]
fn foreign_malloc_free_pair_cost() {
    // Trace: no spec clause (a cost bound on libniji's foreign heap path, D-X.60; PASS2-NATIVE-RUNNER).
    if let Ok(n) = std::env::var(CHILD) {
        pairs(n.parse().unwrap());
        return;
    }
    let dir = root().join("target/test-scratch/heap_foreign_pair_cost").join(std::process::id().to_string());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("launcher.c"), LAUNCHER).unwrap();
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let out = Command::new(&cc).current_dir(&dir).args(["-O1", "-o", "launcher", "launcher.c"]).output().unwrap();
    assert!(out.status.success(), "{cc} failed: {}", String::from_utf8_lossy(&out.stderr));
    let launcher = dir.join("launcher");
    let Some(few) = counted(&launcher, FEW) else { return };
    let Some(many) = counted(&launcher, MANY) else { return };
    let _ = std::fs::remove_dir_all(&dir);
    let per = many.saturating_sub(few) / (MANY - FEW);
    println!("foreign malloc(64)/free pair: {per} user instructions ({FEW} pairs: {few}, {MANY} pairs: {many}); bound {PAIR_BOUND}");
    assert!(per > 0, "no instructions counted for {} pairs: the counter did not follow the test thread", MANY - FEW);
    assert!(per <= PAIR_BOUND, "foreign malloc(64)/free pair costs {per} user instructions, more than the bound {PAIR_BOUND}");
}
