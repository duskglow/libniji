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
use std::process::{Command, Output, Stdio};

use niji_boundary::sys::CSys;
use niji_kernel::sys::Sys;
use niji_kernel::uapi::{ADDR_NO_RANDOMIZE, PERSONALITY_QUERY};

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
/// Zen 1 build host, identical over 5 runs) and libniji's bound. malloc1 and alloc: libniji's
/// measured growth (identical over 5 runs) plus 256 KiB (one slab of a segment's growth). Before
/// HEAP-ADDRESS-SPACE libniji grew by 16448, 32836, 49220 and 67652 KiB (whole 16 MiB segment
/// windows, a 16 MiB large-block table): every step but the first fails. malloc1: a new
/// segment's first 16 slabs (4 MiB of address space, HEAP-ADDRESS-SPACE-5).
/// refill and realloc: the measured level without the reuse of a nearby class's freed blocks
/// (ruling #6513, HEAP-ADDRESS-SPACE-6). They do not yet meet their targets, 33092 (refill) and
/// 35144 KiB (realloc), measured with that reuse; HEAP-SPILL-COLD closes the gap and tightens
/// these bounds to the targets.
const AS_STEPS: [(&str, u64, u64); 4] = [("malloc1", 132, 4420 + 256), ("alloc", 27984, 28996 + 256), ("refill", 29568, 39236), ("realloc", 28052, 41288)];
/// RLIMIT_AS over the VmSize at start under which the whole program must still succeed, run
/// with a fixed layout (`ADDR_NO_RANDOMIZE`; ruling #6566): the least the reference needs is
/// 29568 KiB; this is libniji's measured need without the reuse of a nearby class's freed blocks
/// (bisected to 16 KiB, Zen 1 host: 41296 in 10 of 10 fixed-layout runs; ruling #6513). With
/// randomisation the need depends on where the kernel places the heap's segments next to the
/// other mappings: 41296..43088 KiB over 40 runs (HEAP-ADDRESS-SPACE-6), 41296..43552 over 20
/// (HEAP-ADDRESS-SPACE-7), so a randomised check at one value would be intermittent;
/// HEAP-SPILL-COLD makes the need placement-independent and adds that check. It does not yet
/// meet its target, 35152 KiB (the need with that reuse plus 256 KiB, inside the issue's 35000
/// KiB headroom for findutils' many-dir-entries-vs-OOM test); HEAP-SPILL-COLD closes the gap.
/// Before HEAP-ADDRESS-SPACE libniji needed 67652 KiB.
const AS_HEADROOM: u64 = 41296;

#[test]
fn the_heap_keeps_address_space_near_its_use() {
    // Clauses: s3_1_c_1 (T14: address space under a limit); GitHub issue #21, HEAP-ADDRESS-SPACE.
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("heap_address_space_niji");
    build("heap_address_space.c", &exe);
    // A single-node measurement (ruling #7441): both runs below, every step of them, are pinned to
    // the CPUs of one NUMA node (the lowest with allowed CPUs). On the two-node Skylake-SP host a
    // process moved to the other node mid-run needs up to 4360 KiB more address space (jobs
    // test-scratch-cleanup-4-heaprep2 and -heapmig: pinned to node 0, node 1 or unpinned 0 of 20
    // runs failed, with its affinity flipped between the nodes 15 of 20), and the bounds hold the
    // single-node level with no margin for that. The node-change case is measured and tested on
    // its own (HEAP-SPILL-COLD-2). One node: nothing is pinned.
    let nodes = numa_nodes();
    let node: &[usize] = match nodes.as_slice() {
        [first, _, ..] => first,
        _ => &[],
    };
    let o = with_cpus(node, || {
        assert_on_one_node();
        Command::new(&exe).env_clear().stdin(Stdio::null()).output().unwrap()
    });
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
    // The rule form (HEAP-WALK-ADDRESS-SPACE, as the find walk's rule, HEAP-FIND-DEPTH-NEED-3): in
    // the same layout, the least headroom over start the steps succeed under (page-exact) against
    // the most address space the same steps hold without a limit. Start-up and the steps' real use
    // are in both and cancel; what is left is address space the heap needs under a limit and does
    // not keep without one (a segment not placed, freed space not reused there). Measured (Zen 1
    // build host): need 41288, held 41288. The allowance is the find walk's.
    let run = |arg: String| with_cpus(node, || with_fixed_layout(|| Command::new(&exe).arg(arg).env_clear().stdin(Stdio::null()).output().unwrap()));
    let o = run("-".into());
    assert!(o.status.success(), "{} - failed: {:?}", exe.display(), o.status);
    let out = String::from_utf8_lossy(&o.stdout);
    let at = |step: &str| -> u64 { out.lines().find_map(|l| l.strip_prefix(step).and_then(|v| v.strip_prefix(' ')).and_then(|v| v.parse().ok())).unwrap_or_else(|| panic!("no step {step} in {out:?}")) };
    let held = AS_STEPS.iter().map(|&(step, ..)| at(step).saturating_sub(at("start"))).max().unwrap_or(0);
    let (mut lo, mut hi): (u64, u64) = (0, 1 << 17);
    while hi.saturating_sub(lo) > 4 {
        let mid = lo.midpoint(hi) / 4 * 4;
        if run(mid.to_string()).status.success() {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    eprintln!("issue #21 steps (ADDR_NO_RANDOMIZE): need {hi} KiB over start under RLIMIT_AS, hold {held} without a limit (allowance {FIND_PLACEMENT_SLACK})");
    assert!(
        hi <= held.saturating_add(FIND_PLACEMENT_SLACK),
        "the issue's steps need {hi} KiB over start under RLIMIT_AS but hold at most {held} without a limit: more than {FIND_PLACEMENT_SLACK} over (a segment not placed, or freed space not reused, under the limit?)"
    );
    let o = with_cpus(node, || with_fixed_layout(|| Command::new(&exe).arg(AS_HEADROOM.to_string()).env_clear().stdin(Stdio::null()).output().unwrap()));
    assert!(
        o.status.success(),
        "under RLIMIT_AS = start + {AS_HEADROOM} KiB (ADDR_NO_RANDOMIZE) the issue's steps failed: {:?} (exit 9..12: the malloc1, alloc, refill or realloc step)",
        o.status
    );
}

/// GitHub issue #21's real workload: findutils' `find` over one directory of 200000 files, run by
/// the products' dynamic loader and libc (the system's `find`), fixed layout, one node, in the
/// `en_US.UTF-8` locale. The issue's formula: the least `ulimit -v` (KiB) the walk needs over the
/// least `find` needs for 3 files is at most 35000 ([`ISSUE_21_ALLOWANCE`]). The walk fills several segments
/// of one shard; a segment after the first reservation's (metadata and 8 slabs, 2304 KiB) needs 9
/// blockers to be placed aligned once the over-mapping does not fit under the limit. With 8
/// (before HEAP-FIND-DEPTH-NEED, D-3.25) none was placed and the heap mapped per request: 37344
/// KiB with snapshot 22.4 against 34784 with 22.3 (Zen 1 build host, find 4.9.0, tmpfs). The
/// locale matters: its mappings take the aligned neighbours of a later segment; with an empty
/// environment the same walk needed 30080 (22.4) and 31792 (22.3), and 8 blockers were enough.
/// The rule (HEAP-FIND-DEPTH-NEED-3): what the test protects is that under a limit the heap still
/// places its later segments, so the walk needs about what it maps without a limit. Both are
/// measured in the same run, layout and phase: the least `ulimit -v` the 200000-entry walk
/// succeeds under, and the address space it leaves mapped at exit without a limit (the heap keeps
/// its segments, and the over-mapping it aligns them with is returned on the way). Start-up, the
/// library's own allocations and the walk's real use are in both and cancel; a KiB constant over
/// the 3-entry need was moved past its bound by unrelated changes (start-up layout, the library's
/// allocations routed through the heap, the 16 MiB alignment phase), three reds in three trains.
/// The allowance: under a limit a later segment may go to an aligned slot where another mapping
/// stops its growth after one step (the over-mapping finds room for growth instead), and the walk
/// then places one more segment, its metadata 256 KiB each; two of them, plus one placement
/// round's blocker pages held at the peak (9 pages). Measured (Haswell-EP build host, 8 phases,
/// page-exact): 0 KiB over at 4 phases, 256 at 3, 264 at 1. With the cause put back (8 blockers,
/// no later segment placed: the heap maps per request) the walk needed 34340 to 34852 KiB over 3
/// entries against 33048 to 33576 (Skylake-SP build host).
const FIND_PLACEMENT_SLACK: u64 = 2 * 256 + 9 * 4;

/// GitHub issue #21's formula: `find` over 200000 entries succeeds under a `ulimit -v` of the
/// 3-entry need plus 35000 KiB.
const ISSUE_21_ALLOWANCE: u64 = 35000;

/// Layout shifts (KiB) the rule is checked at: one 16 MiB period (the heap's segment alignment),
/// 2 MiB apart. Where the mappings before the heap end within 16 MiB decides which aligned slots
/// are free; the need over 3 entries took values in bands 256 KiB apart over the period (33048 to
/// 33576 KiB, Skylake-SP build host; 33232 to 33752, Haswell-EP with the preload), and a change in
/// any library's size moves the phase.
const FIND_PHASES: [u64; 8] = [0, 2048, 4096, 6144, 8192, 10240, 12288, 14336];

/// `find dir` run by the products (the loader's `--library-path`; LD_LIBRARY_PATH would reach the
/// shell too) with `vm_size_at_exit.so` preloaded (in every run, so that every run has the same
/// layout), the mappings shifted down by `phase` KiB, under `ulimit -v kib` if given. The shift:
/// a stack limit above 128 MiB moves the kernel's mmap base down by the excess
/// (ADDR_NO_RANDOMIZE), page for page.
fn find_run(find: &Path, dir: &Path, preload: &Path, phase: u64, kib: Option<u64>) -> Output {
    let r = root().join("target/niji");
    let ld = ["ld-linux-x86-64.so.2", "ld.so"].iter().map(|n| r.join(n)).find(|p| p.exists()).expect("the products' dynamic loader");
    let stack = 131_072_u64.saturating_add(phase);
    let limit = kib.map_or(String::new(), |k| format!("ulimit -v {k}; "));
    // Descriptor 3 is the preload's (on the standard error pipe), open in every run alike.
    let cmd = format!("ulimit -s {stack} || exit 99; {limit}exec \"$0\" --preload \"$1\" --library-path \"$2\" \"$3\" \"$4\" > /dev/null 3>&2");
    let o = Command::new("sh").arg("-c").arg(cmd).arg(&ld).arg(preload).arg(&r).arg(find).arg(dir).env_clear().env("LANG", "en_US.UTF-8").stdin(Stdio::null()).output().unwrap();
    assert_ne!(o.status.code(), Some(99), "ulimit -s {stack} refused: the stack hard limit was checked before the run");
    o
}

/// Least `ulimit -v` (KiB, to the 4 KiB page) under which [`find_run`] succeeds: on a coarser
/// grid a need is rounded up, and a fixed cost one page apart moves it by a whole step.
fn find_need(find: &Path, dir: &Path, preload: &Path, phase: u64) -> u64 {
    let ok = |kib: u64| find_run(find, dir, preload, phase, Some(kib)).status.success();
    let (mut lo, mut hi) = (0, 1 << 17);
    assert!(ok(hi), "find {} fails under ulimit -v {hi} with the products", dir.display());
    while hi.saturating_sub(lo) > 4 {
        let mid = lo.midpoint(hi);
        if ok(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

/// The address space (KiB) `find dir` leaves mapped at exit without a limit: the `VmSize:` line
/// `vm_size_at_exit.so` writes to descriptor 3 (opened on the standard error pipe).
fn find_mapped(find: &Path, dir: &Path, preload: &Path, phase: u64) -> u64 {
    let o = find_run(find, dir, preload, phase, None);
    assert!(o.status.success(), "find {} fails without a limit with the products: {:?}", dir.display(), o.status);
    let err = String::from_utf8_lossy(&o.stderr);
    let kib = err.lines().find_map(|l| l.strip_prefix("VmSize:")).and_then(|v| v.trim().strip_suffix("kB")).and_then(|v| v.trim().parse().ok());
    kib.unwrap_or_else(|| panic!("no VmSize line from the preloaded vm_size_at_exit.so: {err}"))
}

/// A directory tree removed when dropped (also when the test fails).
struct Dirs(PathBuf);

impl Drop for Dirs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn find_over_200000_entries_bounds_its_address_space() {
    // Clauses: s3_1_c_1 (T14: address space under a limit); GitHub issue #21, D-3.25.
    let find = Path::new("/usr/bin/find");
    if !find.exists() {
        eprintln!("skipped: no /usr/bin/find on this host (findutils not installed)");
        return;
    }
    let r = root();
    assert!(r.join("target/niji/libc.so.6").exists(), "the products are missing: run `cargo xtask so`");
    // The directories on tmpfs, where the bound was measured: find's need depends on the file
    // system it reads (the same walk on ext4 needed 3247 KiB less), so not under target/.
    let shm = Path::new("/dev/shm");
    if !shm.is_dir() {
        eprintln!("skipped: no /dev/shm (tmpfs) on this host");
        return;
    }
    // The workload's locale (the bound was measured in it): without the archive `find` runs in
    // the C locale, where the case does not arise.
    if !Path::new("/usr/lib/locale/locale-archive").exists() {
        eprintln!("skipped: no /usr/lib/locale/locale-archive (the en_US.UTF-8 locale) on this host");
        return;
    }
    // The phases are set through the stack limit (see `find_need`).
    let top = 131_072_u64.saturating_add(FIND_PHASES.iter().copied().max().unwrap_or(0));
    if !Command::new("sh").arg("-c").arg(format!("ulimit -s {top}")).status().is_ok_and(|s| s.success()) {
        eprintln!("skipped: the stack hard limit is below {top} KiB, so the layout phase cannot be set");
        return;
    }
    let oracles = r.join("target/test-oracles");
    std::fs::create_dir_all(&oracles).unwrap();
    let preload = oracles.join("vm_size_at_exit.so");
    let st = Command::new("cc")
        .args(["-O1", "-shared", "-fPIC", "-nostdlib", "-fno-builtin"])
        .arg(r.join("crates/core/tests/oracle/vm_size_at_exit.c"))
        .arg("-o")
        .arg(&preload)
        .status()
        .expect("cc");
    assert!(st.success(), "vm_size_at_exit.c does not build");
    let base = Dirs(shm.join(format!("niji-find-entries-{}", std::process::id())));
    let (d3, d200k) = (base.0.join("d3"), base.0.join("d200k"));
    for (d, n) in [(&d3, 3), (&d200k, 200_000)] {
        std::fs::create_dir_all(d).unwrap();
        for i in 0..n {
            std::fs::File::create(d.join(i.to_string())).unwrap();
        }
    }
    let nodes = numa_nodes();
    let node: &[usize] = match nodes.as_slice() {
        [first, _, ..] => first,
        _ => &[],
    };
    let mut rows = Vec::new();
    with_cpus(node, || {
        with_fixed_layout(|| {
            for phase in FIND_PHASES {
                let n3 = find_need(find, &d3, &preload, phase);
                rows.push((phase, n3, find_need(find, &d200k, &preload, phase), find_mapped(find, &d200k, &preload, phase)));
            }
            Command::new("true").output().unwrap()
        })
    });
    for &(phase, n3, n200k, mapped) in &rows {
        eprintln!(
            "phase {phase} KiB: find over 200000 entries needs {n200k} KiB, maps {mapped} without a limit: {} over (allowance {FIND_PLACEMENT_SLACK}); {} over 3 entries ({n3}; issue #21 allows {ISSUE_21_ALLOWANCE})",
            n200k.saturating_sub(mapped),
            n200k.saturating_sub(n3)
        );
    }
    for (phase, n3, n200k, mapped) in rows {
        let over = n200k.saturating_sub(mapped);
        assert!(
            over <= FIND_PLACEMENT_SLACK,
            "find over 200000 entries needs {over} KiB more under a limit than it maps without one at layout phase {phase} KiB (needs {n200k}, maps {mapped}) > {FIND_PLACEMENT_SLACK}: a later heap segment not placed under the limit?"
        );
        let over3 = n200k.saturating_sub(n3);
        assert!(over3 <= ISSUE_21_ALLOWANCE, "find over 200000 entries needs {over3} KiB over 3 entries at layout phase {phase} KiB > issue #21's {ISSUE_21_ALLOWANCE}");
    }
}

/// What a UTF-8 locale may cost over the reference's, past the same-run C.UTF-8 baseline (KiB):
/// a quarter of one heap slab (256 KiB) and a sixty-fourth of the heap's 2 MiB segment growth
/// step, so that one locale object that opens a heap size class of its own, or grows the heap,
/// is seen. Measured with the permanent arena (LOCALE-ADDRESS-SPACE; Zen 1 build host, find
/// 4.9.0): 4 KiB over. With the view, items, `Tables` and name leaked from the heap (before) the
/// en_US.UTF-8 run left 2032 KiB more mapped: its four new size classes grew the heap a step.
const LOCALE_MARGIN: u64 = 64;

/// `find dir` with `vm_size_at_exit.so` preloaded and `LANG=lang`, run by the products (`ld` the
/// products' loader, with `--library-path`) or by the system's loader and libc (the reference,
/// `lib_path` `None`): the address space (KiB) it leaves mapped at exit.
fn locale_mapped(ld: &Path, lib_path: Option<&Path>, find: &Path, dir: &Path, preload: &Path, lang: &str) -> u64 {
    let lp = lib_path.map_or(String::new(), |p| format!("--library-path '{}' ", p.display()));
    // Descriptor 3 is the preload's (on the standard error pipe), as in `find_run`.
    let cmd = format!("exec \"$0\" --preload \"$1\" {lp}\"$2\" \"$3\" > /dev/null 3>&2");
    let o = with_fixed_layout(|| Command::new("sh").arg("-c").arg(cmd).arg(ld).arg(preload).arg(find).arg(dir).env_clear().env("LANG", lang).stdin(Stdio::null()).output().unwrap());
    assert!(o.status.success(), "find {} with LANG={lang} by {} fails: {:?}", dir.display(), ld.display(), o.status);
    let err = String::from_utf8_lossy(&o.stderr);
    let kib = err.lines().find_map(|l| l.strip_prefix("VmSize:")).and_then(|v| v.trim().strip_suffix("kB")).and_then(|v| v.trim().parse().ok());
    kib.unwrap_or_else(|| panic!("no VmSize line from the preloaded vm_size_at_exit.so: {err}"))
}

#[test]
fn a_utf8_locale_costs_no_more_address_space_than_the_references() {
    // Clauses: s7_2_c_2 (`setlocale(LC_ALL, "")` loads LANG's locale), s3_1_c_1 (T14: address space); the
    // Yohane report on GitHub issue #21's `find` (LOCALE-ADDRESS-SPACE).
    // `find` over 3 entries in en_US.UTF-8 and in C, by the products and by the reference, in
    // one run and one fixed layout: the address space the UTF-8 locale adds at exit (both map the
    // locale archive) is at most the reference's, plus what the products add for C.UTF-8, whose
    // data is built in (no archive, no locale data: the loader state a non-C `LC_CTYPE` sets up
    // and the first arena chunk, LOCALE-ADDRESS-SPACE), plus [`LOCALE_MARGIN`].
    let find = Path::new("/usr/bin/find");
    let system_ld = Path::new("/lib64/ld-linux-x86-64.so.2");
    let shm = Path::new("/dev/shm");
    for (need, why) in [
        (find, "no /usr/bin/find on this host (findutils not installed)"),
        (system_ld, "no system loader /lib64/ld-linux-x86-64.so.2 (the reference) on this host"),
        (Path::new("/usr/lib/locale/locale-archive"), "no /usr/lib/locale/locale-archive on this host"),
        (shm, "no /dev/shm (tmpfs) on this host"),
    ] {
        if !need.exists() {
            eprintln!("skipped: {why}");
            return;
        }
    }
    let listed = Command::new("locale").arg("-a").output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    if !listed.lines().any(|l| l == "en_US.utf8") {
        eprintln!("skipped: the en_US.UTF-8 locale is not installed (locale -a)");
        return;
    }
    let r = root();
    assert!(r.join("target/niji/libc.so.6").exists(), "the products are missing: run `cargo xtask so`");
    let products = r.join("target/niji");
    let ld = ["ld-linux-x86-64.so.2", "ld.so"].iter().map(|n| products.join(n)).find(|p| p.exists()).expect("the products' dynamic loader");
    let oracles = r.join("target/test-oracles");
    std::fs::create_dir_all(&oracles).unwrap();
    let preload = oracles.join("vm_size_at_exit-locale.so");
    let st = Command::new("cc")
        .args(["-O1", "-shared", "-fPIC", "-nostdlib", "-fno-builtin"])
        .arg(r.join("crates/core/tests/oracle/vm_size_at_exit.c"))
        .arg("-o")
        .arg(&preload)
        .status()
        .expect("cc");
    assert!(st.success(), "vm_size_at_exit.c does not build");
    let base = Dirs(shm.join(format!("niji-locale-as-{}", std::process::id())));
    std::fs::create_dir_all(&base.0).unwrap();
    for i in 0..3 {
        std::fs::File::create(base.0.join(i.to_string())).unwrap();
    }
    let niji = |lang: &str| locale_mapped(&ld, Some(&products), find, &base.0, &preload, lang);
    let reference = |lang: &str| locale_mapped(system_ld, None, find, &base.0, &preload, lang);
    let (n_c, n_en, n_cu) = (niji("C"), niji("en_US.UTF-8"), niji("C.UTF-8"));
    let (s_c, s_en) = (reference("C"), reference("en_US.UTF-8"));
    let (cost, ref_cost, builtin) = (n_en.saturating_sub(n_c), s_en.saturating_sub(s_c), n_cu.saturating_sub(n_c));
    eprintln!("find over 3 entries, VmSize at exit (KiB): products C {n_c}, en_US.UTF-8 {n_en} (+{cost}), C.UTF-8 {n_cu} (+{builtin}); reference C {s_c}, en_US.UTF-8 {s_en} (+{ref_cost})");
    let allowed = ref_cost.saturating_add(builtin).saturating_add(LOCALE_MARGIN);
    assert!(
        cost <= allowed,
        "en_US.UTF-8 adds {cost} KiB of address space over C with the products, the reference {ref_cost}: over the reference's + C.UTF-8's {builtin} + {LOCALE_MARGIN} = {allowed} (locale data back on the heap?)"
    );
}

/// The migration case of [`AS_STEPS`] (ruling #7441): the program starts on the first node's
/// CPUs and moves itself to a CPU of the second at point `at` (`heap_address_space.c`: 1 after
/// malloc1, 2 halfway through alloc, 3 after alloc, 4 after refill). Per point: the VmSize growth
/// over start of the alloc, refill and realloc steps without a limit, and the least RLIMIT_AS over
/// start the run needs (fixed layout, 16 KiB bisection). Measured identically on the Zen 1 and
/// Skylake-SP build hosts (3 of 3 runs each, HEAP-SPILL-COLD-2). The second node maps a segment
/// of its own (placement per node, D-3.12): its metadata (256 KiB) at every point, and without a
/// limit its first 16 slabs (4352 KiB with the metadata) while the first node's slabs sit idle at
/// points 1 and 4. Under a limit a node that has no segment takes the slabs other shards hold
/// before it maps one (`Heap::held_elsewhere`): the need was 45648, 41552, 41552 and 45648 KiB
/// before. A segment after the first reservation's maps its metadata and 8 slabs, one growth step
/// (HEAP-SECOND-NODE-MAP, D-3.24; Skylake-SP build host, 3 of 3 runs): at point 1 the need fell
/// from 43600 to 41552 and at point 4 the unlimited realloc step from 45640 to 43592; what is
/// left over [`AS_HEADROOM`] is the second node's metadata (+256).
const MOVE_STEPS: [(u32, [u64; 3], u64); 4] =
    [(1, [33348, 43588, 45640], 41552), (2, [29252, 39492, 41544], 41552), (3, [28996, 39492, 41544], 41552), (4, [28996, 39236, 43592], 41296)];

#[test]
fn the_heap_bounds_address_space_after_a_node_change() {
    // Clauses: s3_1_c_1 (T14: address space under a limit); GitHub issue #21, ruling #7441.
    // The node change must be reached whatever CPUs the test process was given: a job slot is an
    // affinity mask, often inside one node, so the nodes come from the machine, not the mask. The
    // program starts on the mask's CPUs of one node (the lowest with any) and moves to a CPU of
    // another node, one of the mask's when it has one, else that node's first online CPU (taken
    // for the moved part of a short run). Skipped only where no second node can be reached: a
    // machine with one node, or a cgroup cpuset that refuses every CPU of the others.
    let nodes = node_cpus();
    if nodes.len() < 2 {
        eprintln!("skipped: the machine has {} NUMA node(s) with online CPUs; the migration case needs two", nodes.len());
        return;
    }
    let mine = numa_nodes();
    let first: Vec<usize> = mine.first().cloned().unwrap_or_else(|| nodes.first().cloned().unwrap_or_default());
    let Some(to) = nodes
        .iter()
        .filter(|n| !n.contains(first.first().expect("a node in the list has a CPU")))
        .find_map(|n| mine.iter().flatten().find(|c| n.contains(c)).copied().or_else(|| n.iter().copied().find(|&c| may_run_on(c))))
    else {
        eprintln!("skipped: {} NUMA nodes, but no CPU off the start node {first:?} may be used (cgroup cpuset?)", nodes.len());
        return;
    };
    eprintln!("start on CPUs {first:?} (mask per node {mine:?}), move to CPU {to}");
    let first = first.as_slice();
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("heap_address_space_move_niji");
    build("heap_address_space.c", &exe);
    let to = to.to_string();
    for (at, bounds, need) in MOVE_STEPS {
        let at_s = at.to_string();
        // Fixed layout, as measured: a randomised one can end a segment's window early.
        let o = with_cpus(first, || with_fixed_layout(|| Command::new(&exe).args(["-", &to, &at_s]).env_clear().stdin(Stdio::null()).output().unwrap()));
        assert!(o.status.success(), "moved to CPU {to} at {at}: {} failed: {:?} (exit 5, 6: the move failed)", exe.display(), o.status);
        let out = String::from_utf8_lossy(&o.stdout);
        let kib = |step: &str| -> u64 {
            out.lines()
                .find_map(|l| l.strip_prefix(step).and_then(|v| v.strip_prefix(' ')).and_then(|v| v.parse().ok()))
                .unwrap_or_else(|| panic!("no step {step} in {out:?}"))
        };
        let start = kib("start");
        for (step, bound) in ["alloc", "refill", "realloc"].into_iter().zip(bounds) {
            let grew = kib(step).saturating_sub(start);
            eprintln!("moved at {at}: {step}: VmSize +{grew} KiB over start (bound {bound})");
            assert!(grew <= bound, "moved to another node at {at}: {step} added {grew} KiB of address space > {bound}");
        }
        let o = with_cpus(first, || with_fixed_layout(|| Command::new(&exe).args([&need.to_string(), &to, &at_s]).env_clear().stdin(Stdio::null()).output().unwrap()));
        assert!(
            o.status.success(),
            "moved to another node at {at}: under RLIMIT_AS = start + {need} KiB (ADDR_NO_RANDOMIZE) the issue's steps failed: {:?} (exit 9..12: a step; a node with no segment mapped one while other shards held slabs?)",
            o.status
        );
    }
}

/// Checks that every CPU the calling thread may run on, and so every CPU the programs it spawns
/// may run on, is on one NUMA node: a pinning that was skipped or set the wrong mask would leave
/// the measurement open to a node change again. A test process already confined to one node
/// (a job slot inside one node) passes with or without the pinning: the check can only see the
/// pinning when the inherited mask spans several nodes, which is how its plants run it.
fn assert_on_one_node() {
    let nodes = numa_nodes();
    eprintln!("the measurement may run on CPUs {nodes:?} (per NUMA node)");
    assert!(nodes.len() <= 1, "the measurement may run on CPUs {nodes:?} of {} nodes, not one (pinning skipped or wrong mask)", nodes.len());
}

/// The CPUs this process may run on, per NUMA node (lowest node first; a node with none of them
/// is left out), from `/sys/devices/system/node/node<N>/cpulist` and the affinity mask. Empty
/// without that directory (no NUMA support in the kernel): then nothing is pinned.
fn numa_nodes() -> Vec<Vec<usize>> {
    let mut mask = [0u8; 1024];
    let len = CSys.sched_getaffinity(0, &mut mask).expect("sched_getaffinity");
    let allowed = |c: &usize| *c / 8 < len && mask.get(*c / 8).is_some_and(|b| b & (1 << (*c % 8)) != 0);
    node_cpus().into_iter().map(|n| n.into_iter().filter(allowed).collect::<Vec<_>>()).filter(|n| !n.is_empty()).collect()
}

/// The machine's online CPUs per NUMA node, whatever this process's affinity mask (lowest node
/// first; a node with no online CPU is left out), from `/sys/devices/system/node/node<N>/cpulist`
/// and `/sys/devices/system/cpu/online`. Empty without the node directory (no NUMA support).
fn node_cpus() -> Vec<Vec<usize>> {
    let parse = |list: &str| -> Vec<usize> {
        let mut cpus = Vec::new();
        for part in list.trim().split(',').filter(|p| !p.is_empty()) {
            let (a, b) = part.split_once('-').unwrap_or((part, part));
            let (a, b): (usize, usize) = (a.parse().expect("cpulist"), b.parse().expect("cpulist"));
            cpus.extend(a..=b);
        }
        cpus
    };
    let online = parse(&std::fs::read_to_string("/sys/devices/system/cpu/online").unwrap_or_default());
    let Ok(dir) = std::fs::read_dir("/sys/devices/system/node") else { return Vec::new() };
    let mut ids: Vec<usize> = dir.filter_map(|e| e.ok()?.file_name().to_str()?.strip_prefix("node")?.parse().ok()).collect();
    ids.sort_unstable();
    ids.into_iter()
        .map(|id| parse(&std::fs::read_to_string(format!("/sys/devices/system/node/node{id}/cpulist")).unwrap_or_default()))
        .map(|cpus| cpus.into_iter().filter(|c| online.contains(c)).collect::<Vec<_>>())
        .filter(|cpus| !cpus.is_empty())
        .collect()
}

/// Whether this thread may run on `cpu`, which its affinity mask may lack (a job slot is a mask;
/// a cgroup cpuset can refuse the CPU): sets the mask to it and restores the old one.
fn may_run_on(cpu: usize) -> bool {
    let mut cur = [0u8; 1024];
    let len = CSys.sched_getaffinity(0, &mut cur).expect("sched_getaffinity");
    let ok = CSys.sched_setaffinity(0, cpu_mask(&[cpu]).get(..len).expect("mask length")).is_ok();
    CSys.sched_setaffinity(0, cur.get(..len).expect("mask length")).expect("sched_setaffinity restore");
    ok
}

/// The affinity mask of `cpus`.
fn cpu_mask(cpus: &[usize]) -> [u8; 1024] {
    let mut want = [0u8; 1024];
    for &c in cpus {
        if let Some(b) = want.get_mut(c / 8) {
            *b |= 1 << (c % 8);
        }
    }
    want
}

/// Runs `run` with the calling (test) thread's CPU affinity set to `cpus`, which the program it
/// spawns inherits, and restores the mask afterwards. Empty `cpus`: runs it as it is.
fn with_cpus(cpus: &[usize], run: impl FnOnce() -> Output) -> Output {
    if cpus.is_empty() {
        return run();
    }
    let mut cur = [0u8; 1024];
    let len = CSys.sched_getaffinity(0, &mut cur).expect("sched_getaffinity");
    CSys.sched_setaffinity(0, cpu_mask(cpus).get(..len).expect("mask length")).expect("sched_setaffinity");
    let o = run();
    CSys.sched_setaffinity(0, cur.get(..len).expect("mask length")).expect("sched_setaffinity restore");
    o
}

/// Runs `run` with `ADDR_NO_RANDOMIZE` in the calling (test) thread's persona, which the program
/// it spawns inherits, and restores the persona afterwards so that other runs in this thread keep
/// the randomised layout.
fn with_fixed_layout(run: impl FnOnce() -> Output) -> Output {
    let cur = CSys.personality(PERSONALITY_QUERY).expect("personality query");
    CSys.personality(cur | ADDR_NO_RANDOMIZE).expect("personality ADDR_NO_RANDOMIZE");
    assert_eq!(CSys.personality(PERSONALITY_QUERY).ok(), Some(cur | ADDR_NO_RANDOMIZE), "ADDR_NO_RANDOMIZE did not stick");
    let o = run();
    CSys.personality(cur).expect("personality restore");
    o
}
