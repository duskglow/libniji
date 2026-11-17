//! Heap metadata page faults (HEAP-META-DENSE, D-3.15): `tests/oracle/heap_meta_faults.c`, linked
//! statically against the products, allocates the first block of 14 size classes after a warm-up
//! allocation, frees them again and reports the minor faults this took (getrusage ru_minflt). The
//! blocks are never written, so only heap metadata is counted. A same-thread free must not read
//! the remote bitmaps (their pages stay untouched while the segment had no remote free: 14 more). With one 4144-byte record per slab every
//! new class touched a record page of its own (14+); with the slab heads in one page and bitmaps
//! written only as far as their class uses them, two classes share a bitmap page (7). Faults are
//! deterministic counters (WL gate rule), not timing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Upper bound on the lowest of [`RUNS`] counts: 7 bitmap pages for 14 classes plus headroom.
const BOUND: u64 = 9;
const RUNS: usize = 5;
/// Upper bound on the lowest of [`RUNS`] counts of the process's first `malloc(16)` (D-3.19): 11
/// measured (one registry page, the first cache, the segment's head and bitmap pages, the
/// block's slab page, the first `getcpu`'s vDSO page, and the text and stack pages of the path),
/// plus one. A regression guard only: a heap that maps the three registries separately again
/// stayed within it (plant run, HEAP-FIRST-USE); `the_first_allocation_reserves_one_mapping` (rt)
/// holds the one mapping.
const FIRST_BOUND: u64 = 12;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build(src: &str, out: &Path) {
    let r = root();
    let st = Command::new("cc")
        .args(["-std=gnu17", "-O1", "-static", "-nostdlib", "-fno-builtin"])
        .arg(r.join("target/niji/niji-crt1.o"))
        .arg(r.join("crates/core/tests/oracle").join(src))
        .arg(r.join("target/niji/libc.a"))
        .arg("-lgcc")
        .arg("-o")
        .arg(out)
        .status()
        .expect("cc");
    assert!(st.success(), "{src} does not link against the products");
}

#[test]
fn init_heap_metadata_of_new_size_classes_stays_dense() {
    // Trace: D-3.15
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("heap_meta_faults_niji");
    build("heap_meta_faults.c", &exe);
    let mut best = u64::MAX;
    for _ in 0..RUNS {
        let o = Command::new(&exe).env_clear().stdin(Stdio::null()).output().unwrap();
        assert!(o.status.success(), "{} failed: {:?}", exe.display(), o.status);
        let err = String::from_utf8_lossy(&o.stderr);
        let n: u64 = err.trim().strip_prefix("minflt ").and_then(|v| v.parse().ok()).unwrap_or_else(|| panic!("unexpected output {err:?}"));
        best = best.min(n);
    }
    eprintln!("heap metadata faults for 14 new classes (malloc + free): {best} (bound {BOUND})");
    assert!(best <= BOUND, "14 new size classes took {best} page faults > {BOUND} (dense slab metadata lost?)");
}

#[test]
fn the_first_allocation_touches_few_pages() {
    // Trace: D-3.19 (no spec clause: the heap's first reservation is internal).
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("heap_first_faults_niji");
    build("heap_first_faults.c", &exe);
    let mut best = u64::MAX;
    for _ in 0..RUNS {
        let o = Command::new(&exe).env_clear().stdin(Stdio::null()).output().unwrap();
        assert!(o.status.success(), "{} failed: {:?}", exe.display(), o.status);
        let err = String::from_utf8_lossy(&o.stderr);
        let n: u64 = err.trim().strip_prefix("minflt ").and_then(|v| v.parse().ok()).unwrap_or_else(|| panic!("unexpected output {err:?}"));
        best = best.min(n);
    }
    eprintln!("page faults of the first malloc(16): {best} (bound {FIRST_BOUND})");
    assert!(best <= FIRST_BOUND, "the first malloc(16) took {best} page faults > {FIRST_BOUND} (first reservation split again?)");
}

#[test]
fn the_vdso_lookup_takes_no_folding_decision() {
    // Trace: D-3.20 (no spec clause: system calls of an internal lookup). The vDSO lookup runs on
    // the heap's first slow path when several nodes are allowed (D-3.20) and at the first clock
    // call on every host, which this oracle uses so the test does not depend on the host's
    // topology. Its name compares must not go through the exported compare entry, whose first
    // small call reads vm.mmap_min_addr (#4170, ruling #4197): the first clock_gettime makes no
    // read system call. The guard itself is untouched; the program's own first copy, fill or
    // compare takes the decision.
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("vdso_lookup_reads_niji");
    build("vdso_lookup_reads.c", &exe);
    let o = Command::new(&exe).env_clear().stdin(Stdio::null()).output().unwrap();
    assert!(o.status.success(), "{} failed: {:?}", exe.display(), o.status);
    let err = String::from_utf8_lossy(&o.stderr);
    if let Some(why) = err.trim().strip_prefix("skip ") {
        eprintln!("skipped: {why}");
        return;
    }
    let n: i64 = err.trim().strip_prefix("reads ").and_then(|v| v.parse().ok()).unwrap_or_else(|| panic!("unexpected output {err:?}"));
    assert!(n == 0, "the first clock_gettime made {n} read system calls (vDSO name compare through the exported compare entry?)");
}

#[test]
fn several_nodes_read_the_node_from_rseq() {
    // Trace: D-3.21 (no spec clause: the heap's internal node source). On a host with several
    // allowed nodes and a kernel that fills the rseq area's node_id (Linux 6.3+), a thread pinned
    // to a CPU of node n > 0 takes its first block from node n with the vDSO unmapped: a node read
    // through the vDSO getcpu would fault, so a clean run shows the node came from rseq, and the
    // page's node shows it was the right field. One node or an older kernel: skipped with why.
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("rseq_node_niji");
    build("rseq_node.c", &exe);
    let o = Command::new(&exe).env_clear().stdin(Stdio::null()).output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{} failed: {:?} {err:?} (a fault: the node read went through the vDSO)", exe.display(), o.status);
    if let Some(why) = err.trim().strip_prefix("skip ") {
        eprintln!("skipped: {why}");
        return;
    }
    let (node, page) = err.trim().strip_prefix("node ").and_then(|v| v.split_once(" page ")).unwrap_or_else(|| panic!("unexpected output {err:?}"));
    eprintln!("rseq node read: node {node}, page on node {page}");
    assert_eq!(node, page, "the thread on node {node} took a slab placed on node {page} (node_id read from the wrong field?)");
}

/// The address space (VmSize) the steps of GitHub issue #21 add over the process's start, in
/// KiB: `tests/oracle/heap_address_space.c` (one `malloc(16)`; 200000 blocks of 96..159 bytes;
/// every other one freed and refilled with `malloc(120)`; all freed; a pointer array grown to
/// 200000 by doubling `realloc`). Per step: the reference's growth (glibc 2.39, measured on the
/// Zen 1 build host, identical over 5 runs) and libniji's bound, its measured growth (identical
/// over 5 runs) plus 256 KiB (one slab of a segment's growth). Before HEAP-ADDRESS-SPACE libniji
/// grew by 16448, 32836, 49220 and 67652 KiB (whole 16 MiB segment windows, a 16 MiB large-block
/// table): every step but the first fails.
const AS_STEPS: [(&str, u64, u64); 4] =
    [("malloc1", 132, 836 + 256), ("alloc", 27984, 28996 + 256), ("refill", 29568, 32836 + 256), ("realloc", 28052, 34888 + 256)];
/// RLIMIT_AS over the VmSize at start under which the whole program must still succeed: the
/// least the reference needs is 29568 KiB, libniji 34896 KiB (bisected to 16 KiB); this is
/// libniji's need plus 256 KiB, inside the issue's 35000 KiB headroom (findutils'
/// many-dir-entries-vs-OOM test). Before HEAP-ADDRESS-SPACE libniji needed 67652 KiB.
const AS_HEADROOM: u64 = 34896 + 256;

#[test]
fn the_heap_keeps_address_space_near_its_use() {
    // Clauses: s3_1_c_1 (T14: address space under a limit); GitHub issue #21, HEAP-ADDRESS-SPACE.
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("heap_address_space_niji");
    build("heap_address_space.c", &exe);
    let o = Command::new(&exe).env_clear().stdin(Stdio::null()).output().unwrap();
    assert!(o.status.success(), "{} failed: {:?}", exe.display(), o.status);
    let out = String::from_utf8_lossy(&o.stdout);
    let kib = |step: &str| -> u64 {
        out.lines()
            .find_map(|l| l.strip_prefix(step).and_then(|v| v.strip_prefix(' ')).and_then(|v| v.parse().ok()))
            .unwrap_or_else(|| panic!("no step {step} in {out:?}"))
    };
    let start = kib("start");
    for (step, reference, bound) in AS_STEPS {
        let grew = kib(step).saturating_sub(start);
        eprintln!("{step}: VmSize +{grew} KiB over start (reference +{reference}, bound {bound})");
        assert!(grew <= bound, "{step}: the heap added {grew} KiB of address space > {bound} (reference {reference}): whole segments or tables mapped up front, or freed space not reused?");
    }
    let o = Command::new(&exe).arg(AS_HEADROOM.to_string()).env_clear().stdin(Stdio::null()).output().unwrap();
    assert!(
        o.status.success(),
        "under RLIMIT_AS = start + {AS_HEADROOM} KiB the issue's steps failed: {:?} (exit 9..12: the malloc1, alloc, refill or realloc step)",
        o.status
    );
}
