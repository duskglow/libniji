//! The verdict cache of the math accuracy sweeps (`crates/core/tests/support/verdict.rs`,
//! D-10.57): every input of the key changes the key when it changes, the host libraries a
//! reference program loads are in it, a shard records its verdict only after it passed and only
//! for the inputs it checked, and `NIJI_NO_VERDICT_CACHE` runs a cached shard. Also the programs'
//! outputs the shards of one test run share (`verdict_programs::shared_outputs`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#[path = "../../core/tests/support/skip_note.rs"]
mod skip_note;

#[path = "../../core/tests/support/oracle.rs"]
mod oracle;

#[path = "../../core/tests/support/verdict.rs"]
mod verdict;
#[path = "../../core/tests/support/verdict_programs.rs"]
mod verdict_programs;

use std::path::{Path, PathBuf};
use std::process::Command;

use verdict::{Key, Sources};

/// A fresh directory of stand-in inputs (test binary, cpuinfo, kernel release) and records.
struct Bench {
    dir: PathBuf,
    src: Sources,
}

impl Bench {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("niji-verdict-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("records")).unwrap();
        let src = Sources { exe: dir.join("exe"), cpuinfo: dir.join("cpuinfo"), osrelease: dir.join("osrelease") };
        std::fs::write(&src.exe, b"\x7fELF test binary one").unwrap();
        std::fs::write(&src.cpuinfo, "processor\t: 0\nflags\t\t: fpu sse2 avx avx2 fma\n\nprocessor\t: 1\nflags\t\t: fpu sse2 avx avx2 fma\n").unwrap();
        std::fs::write(&src.osrelease, "6.8.0-test\n").unwrap();
        std::fs::write(dir.join("program"), b"program bytes").unwrap();
        Self { dir, src }
    }

    fn key(&self) -> Key {
        Key::with_sources("sweep.s3", &self.src, &self.dir.join("records")).shard(3, 8).file(&self.dir.join("program"))
    }

    fn record(&self) -> PathBuf {
        self.dir.join("records/sweep.s3")
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn each_input_of_the_key_changes_the_key() {
    // no spec clause: the verdict cache (D-10.57) reruns a shard when any input changes.
    let b = Bench::new("inputs");
    let base = b.key();
    assert!(base.complete(), "{}", base.text());
    assert_eq!(base, b.key(), "the same inputs give the same key");
    let changed = |what: &str, path: &Path, bytes: &[u8]| {
        let old = std::fs::read(path).unwrap();
        std::fs::write(path, bytes).unwrap();
        let k = b.key();
        std::fs::write(path, old).unwrap();
        assert_ne!(k, base, "changing {what} left the key as it was:\n{}", k.text());
        assert_ne!(k.id(), base.id(), "{what}: the key's short form");
    };
    changed("the test binary", &b.src.exe, b"\x7fELF test binary two");
    changed("the CPU flags", &b.src.cpuinfo, b"processor\t: 0\nflags\t\t: fpu sse2 avx\n");
    changed("the kernel release", &b.src.osrelease, b"6.9.1-test\n");
    changed("a program", &b.dir.join("program"), b"program byteS");
    let other_shard = Key::with_sources("sweep.s3", &b.src, &b.dir.join("records")).shard(4, 8).file(&b.dir.join("program"));
    assert_ne!(other_shard, base, "the shard index");
    let other_count = Key::with_sources("sweep.s3", &b.src, &b.dir.join("records")).shard(3, 9).file(&b.dir.join("program"));
    assert_ne!(other_count, base, "the shard count");
    // An input that cannot be read makes the key incomplete: never cached, never recorded.
    let missing = Key::with_sources("sweep.s3", &b.src, &b.dir.join("records")).file(&b.dir.join("absent"));
    assert!(!missing.complete());
    missing.record();
    assert!(!missing.cached() && !b.record().exists());
}

#[test]
fn the_running_test_binary_and_the_host_are_in_the_key() {
    // no spec clause: the verdict cache (D-10.57) keys on this process's image, CPU flags and kernel.
    let k = Key::new("verdict_cache.self");
    let text = k.text();
    assert!(k.complete(), "{text}");
    let len = std::fs::read("/proc/self/exe").unwrap().len();
    assert!(text.lines().any(|l| l.starts_with(&format!("test binary\t{len}\t"))), "the test binary ({len} bytes) is not in the key:\n{text}");
    let flags = std::fs::read_to_string("/proc/cpuinfo").unwrap();
    let flags = verdict::cpu_flags(&flags).unwrap();
    assert!(text.lines().any(|l| l == format!("cpu\t{flags}")), "the CPU flags are not in the key:\n{text}");
    let release = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap();
    assert!(text.lines().any(|l| l == format!("kernel\t{}", release.trim())), "the kernel release is not in the key:\n{text}");
}

#[test]
fn the_libraries_the_reference_program_loads_are_in_the_key() {
    // no spec clause: the verdict cache (D-10.57) keys on the host libm and libc the reference loads.
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler");
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let prog = root.join("target/test-oracles/verdict_probe_host");
    let mut cmd = Command::new("cc");
    cmd.args(["-std=gnu17", "-O1", "-fno-builtin"]).arg(root.join("crates/math/tests/oracle/verdict_probe.c")).arg("-lm");
    assert!(oracle::build(cmd, &prog), "verdict_probe.c does not build against the host C library");
    let objs = verdict_programs::loaded_objects(&prog).expect("the loader lists the program's objects");
    for lib in ["libm.so.6", "libc.so.6"] {
        assert!(objs.iter().any(|p| p.file_name().is_some_and(|n| n == lib)), "{lib} is not among {objs:?}");
    }
    let b = Bench::new("libs");
    let with = b.key().loaded_by(&prog);
    let text = with.text();
    assert!(with.complete(), "{text}");
    for p in &objs {
        let len = std::fs::read(p).unwrap().len();
        assert!(text.lines().any(|l| l.starts_with(&format!("file\t{}\t{len}\t", p.display()))), "{} ({len} bytes) is not in the key:\n{text}", p.display());
    }
    assert_ne!(with, b.key(), "the loaded objects changed nothing");
    // A program that cannot be started gives an incomplete key.
    assert!(!b.key().loaded_by(&b.dir.join("absent")).complete());
}

#[test]
fn only_a_shard_that_passed_is_recorded_and_then_cached() {
    // no spec clause: a cached verdict (D-10.57) is only ever a green one for an identical key.
    let b = Bench::new("record");
    // A shard that fails records nothing.
    let run = b.key().begin_with(false).expect("nothing recorded yet: the shard runs");
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _run = run;
        panic!("a shard's check failed");
    }));
    assert!(failed.is_err());
    assert!(!b.record().exists(), "a failed shard left a record");
    assert!(!b.key().cached());
    // One that passes records its key; the next run with the same inputs is cached.
    b.key().begin_with(false).expect("the failed shard runs again").pass();
    assert!(b.record().exists() && b.key().cached());
    assert!(b.key().begin_with(false).is_none(), "an unchanged shard ran again");
    // NIJI_NO_VERDICT_CACHE=1 runs it all the same.
    assert!(b.key().begin_with(true).is_some(), "the override did not run the shard");
    b.key().begin_with(false).expect("the forced run removed the record until it passes").pass();
    // A changed input reruns it, removing the old record first: if the rerun fails, nothing is cached.
    std::fs::write(b.dir.join("program"), b"a changed program").unwrap();
    let rerun = b.key().begin_with(false).expect("a changed program runs the shard");
    assert!(!b.record().exists(), "the record of the old inputs survived the start of a rerun");
    drop(rerun);
    assert!(!b.key().cached());
}

/// A program (a shell script) that counts its runs in `count` and prints `text`, exiting with `status`.
fn counting_program(dir: &Path, text: &str, status: u8) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let prog = dir.join(format!("prog-{text}-{status}"));
    let script = format!("#!/bin/sh\necho run >> {}\necho {text} line one\necho {text} line two\nexit {status}\n", dir.join(format!("count-{text}")).display());
    std::fs::write(&prog, script).unwrap();
    std::fs::set_permissions(&prog, std::fs::Permissions::from_mode(0o755)).unwrap();
    prog
}

fn runs(dir: &Path, text: &str) -> usize {
    std::fs::read_to_string(dir.join(format!("count-{text}"))).map_or(0, |c| c.lines().count())
}

#[test]
fn the_shards_of_one_run_share_the_programs_outputs() {
    // no spec clause: the oracle sweeps run both programs once per test run (MATH-ORACLE-PROFILE).
    let b = Bench::new("share");
    let (host, ours) = (counting_program(&b.dir, "host", 0), counting_program(&b.dir, "ours", 0));
    let out = b.dir.join("outputs");
    let key = || Key::with_sources("sweep.outputs", &b.src, &b.dir.join("records")).file(&host).file(&ours);
    let first = verdict_programs::shared_outputs_in("sweep", &key(), &[&host, &ours], &out, "run-1");
    assert_eq!(first, ["host line one\nhost line two\n", "ours line one\nours line two\n"]);
    // The other shards of the run read the outputs: no program runs again.
    for _ in 0..3 {
        assert_eq!(verdict_programs::shared_outputs_in("sweep", &key(), &[&host, &ours], &out, "run-1"), first);
    }
    assert_eq!((runs(&b.dir, "host"), runs(&b.dir, "ours")), (1, 1), "a shard of the run ran a program again");
    // Another run with the same inputs runs them again and removes the first run's outputs.
    let stored = |run: &str| std::fs::read_dir(&out).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains(run)).count();
    assert_eq!(stored("run-1"), 2);
    assert_eq!(verdict_programs::shared_outputs_in("sweep", &key(), &[&host, &ours], &out, "run-2"), first);
    assert_eq!((runs(&b.dir, "host"), runs(&b.dir, "ours")), (2, 2), "an output served a second test run");
    assert_eq!((stored("run-1"), stored("run-2")), (0, 2));
    // A changed program (a plant, a rebuild) changes the key: it runs, and its new output is read.
    std::fs::write(&ours, std::fs::read_to_string(&ours).unwrap().replace("ours line two", "ours line 2")).unwrap();
    let changed = verdict_programs::shared_outputs_in("sweep", &key(), &[&host, &ours], &out, "run-2");
    assert_eq!(changed[1], "ours line one\nours line 2\n", "a shard read the output of the program before it changed");
    assert_eq!(runs(&b.dir, "ours"), 3);
}

#[test]
fn an_output_of_a_failed_program_or_an_incomplete_key_is_not_shared() {
    // no spec clause: only a complete output of a program that exited with 0 is shared (MATH-ORACLE-PROFILE).
    let b = Bench::new("noshare");
    let (bad, good) = (counting_program(&b.dir, "bad", 3), counting_program(&b.dir, "good", 0));
    let out = b.dir.join("outputs");
    let key = Key::with_sources("sweep.outputs", &b.src, &b.dir.join("records")).file(&bad).file(&good);
    for n in 1..=2 {
        let o = verdict_programs::shared_outputs_in("sweep", &key, &[&bad, &good], &out, "run-1");
        assert_eq!(o[0], "bad line one\nbad line two\n", "the failed program's output is still the shard's to check");
        assert_eq!(runs(&b.dir, "bad"), n, "the output of a program that failed was shared");
    }
    // An input that cannot be read: every call runs the programs and stores nothing.
    let incomplete = key.file(&b.dir.join("absent"));
    assert!(!incomplete.complete());
    let before = std::fs::read_dir(&out).unwrap().count();
    for n in 1..=2 {
        verdict_programs::shared_outputs_in("sweep", &incomplete, &[&good], &out, "run-1");
        // Twice above: the second call found the failed program's output missing and ran both.
        assert_eq!(runs(&b.dir, "good"), 2 + n, "an incomplete key shared an output");
    }
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), before);
}

#[test]
fn an_input_that_changes_during_the_run_is_not_recorded() {
    // no spec clause: the verdict cache (D-10.57) records the inputs the shard checked, not later ones.
    let b = Bench::new("during");
    let run = b.key().begin_with(false).unwrap();
    std::fs::write(b.dir.join("program"), b"rebuilt while the shard ran").unwrap();
    run.pass();
    assert!(!b.record().exists(), "a verdict was recorded for inputs the shard did not check");
}
