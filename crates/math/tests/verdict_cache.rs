//! The verdict cache of the math accuracy sweeps (`crates/core/tests/support/verdict.rs`,
//! D-10.57): every input of the key changes the key when it changes, the host libraries a
//! reference program loads are in it, a shard records its verdict only after it passed and only
//! for the inputs it checked, and `NIJI_NO_VERDICT_CACHE` runs a cached shard. Also the programs'
//! outputs the shards of one test run share (`verdict_programs::shared_outputs`), and the oracle's
//! value cache (`oracle_values`, D-10.131).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#[path = "../../core/tests/support/skip_note.rs"]
mod skip_note;

#[path = "../../core/tests/support/oracle.rs"]
mod oracle;

#[path = "../../core/tests/support/verdict.rs"]
mod verdict;
#[path = "../../core/tests/support/verdict_programs.rs"]
mod verdict_programs;
#[path = "support/oracle_values.rs"]
mod oracle_values;

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
    assert_eq!(changed.get(1).map(String::as_str), Some("ours line one\nours line 2\n"),"a shard read the output of the program before it changed");
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
        assert_eq!(o.first().map(String::as_str), Some("bad line one\nbad line two\n"),"the failed program's output is still the shard's to check");
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

// The oracle's value cache (D-10.131).

use oracle_values::{Host, Query, Values};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A stand-in tree: the oracle crate depends on a workspace crate, which depends on a registry
/// package; one more workspace crate is not in the oracle's closure.
fn value_tree(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("niji-oracle-values-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let files: [(&str, &str); 11] = [
        ("Cargo.toml", "[workspace]\n[profile.dev.package.niji-mathtools]\nopt-level = 3\n"),
        ("rust-toolchain.toml", "[toolchain]\nchannel = \"1.99.1\"\n"),
        (".cargo/config.toml", "[build]\n"),
        ("Cargo.lock", "version = 4\n\n[[package]]\nname = \"niji-mathtools\"\nversion = \"0.0.0\"\ndependencies = [\n \"helper\",\n]\n\n[[package]]\nname = \"helper\"\nversion = \"0.1.0\"\ndependencies = [\"ext 1.2.0\"]\n\n[[package]]\nname = \"ext\"\nversion = \"1.2.0\"\nsource = \"registry+https://example.invalid/index\"\nchecksum = \"aaaa\"\n\n[[package]]\nname = \"ext\"\nversion = \"2.0.0\"\nsource = \"registry+https://example.invalid/index\"\nchecksum = \"bbbb\"\n\n[[package]]\nname = \"other\"\nversion = \"0.0.0\"\n"),
        ("crates/mathtools/Cargo.toml", "[package]\nname = \"niji-mathtools\"\n"),
        ("crates/mathtools/src/lib.rs", "pub mod mp;\n"),
        ("crates/mathtools/src/mp/mod.rs", "pub fn sqrt() {}\n"),
        ("crates/helper/Cargo.toml", "[package]\nname = \"helper\"\n"),
        ("crates/helper/src/lib.rs", "pub fn help() {}\n"),
        ("crates/other/Cargo.toml", "[package]\nname = \"other\"\n"),
        ("crates/math/tests/sweep.rs", "fn cr() {}\n"),
    ];
    for (p, text) in files {
        let p = dir.join(p);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    std::fs::create_dir_all(dir.join("crates/math/tests/support")).unwrap();
    std::fs::write(dir.join("crates/math/tests/support/oracle_values.rs"), "// the cache\n").unwrap();
    dir
}

fn value_host() -> Host {
    Host { rustc: Some("rustc 1.99.1 (abc 2026-01-01)\nhost: x86_64-unknown-linux-gnu\n".into()), flags: vec!["build RUSTFLAGS=None".into()], machine: Some("x86_64 linux flags : fpu".into()) }
}

#[test]
fn every_input_of_the_oracle_key_changes_the_key() {
    // no spec clause: the value cache's key (D-10.131): the oracle's sources, its Cargo.lock
    // closure, the sweep's wrappers, the build settings and the toolchain.
    let dir = value_tree("key");
    let key = |h: &Host| oracle_values::oracle_key(&dir, &dir.join("crates/math/tests/sweep.rs"), h);
    let k0 = key(&value_host()).expect("the stand-in tree gives a complete key");
    let edits: [(&str, &str, &str); 10] = [
        ("crates/mathtools/src/mp/mod.rs", "pub fn sqrt() {}\n", "the oracle's source"),
        ("crates/mathtools/src/mp/new.rs", "pub fn new() {}\n", "a new file of the oracle"),
        ("crates/helper/src/lib.rs", "pub fn helps() {}\n", "a workspace crate the oracle uses"),
        ("Cargo.lock", "", "a registry package's checksum"),
        ("crates/math/tests/sweep.rs", "fn cr() { /* 2^18 */ }\n", "the sweep's wrappers"),
        ("crates/math/tests/support/oracle_values.rs", "// the cache, v2\n", "the cache's own code"),
        ("Cargo.toml", "[workspace]\n[profile.dev.package.niji-mathtools]\nopt-level = 2\n", "the build profile"),
        ("rust-toolchain.toml", "[toolchain]\nchannel = \"1.99.2\"\n", "the toolchain file"),
        (".cargo/config.toml", "[build]\nrustflags = [\"-Ctarget-cpu=native\"]\n", "the cargo configuration"),
        ("crates/other/src/lib.rs", "pub fn unrelated() {}\n", ""),
    ];
    for (path, text, what) in edits {
        let p = dir.join(path);
        let old = std::fs::read(&p).ok();
        if path == "Cargo.lock" {
            let lock = String::from_utf8(old.clone().unwrap()).unwrap();
            std::fs::write(&p, lock.replace("checksum = \"aaaa\"", "checksum = \"aaab\"")).unwrap();
        } else {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, if old.as_deref() == Some(text.as_bytes()) { format!("{text}// changed\n") } else { text.to_string() }).unwrap();
        }
        let k = key(&value_host()).unwrap();
        if what.is_empty() {
            assert_eq!(k, k0, "a crate outside the oracle's closure changed the key");
        } else {
            assert_ne!(k, k0, "{what} is not in the key");
        }
        match old {
            Some(b) => std::fs::write(&p, b).unwrap(),
            None => std::fs::remove_file(&p).unwrap(),
        }
        assert_eq!(key(&value_host()).unwrap(), k0, "restoring {path} did not restore the key");
    }
    // The second registry version is not in the closure: its checksum is not in the key.
    let lock = std::fs::read_to_string(dir.join("Cargo.lock")).unwrap();
    std::fs::write(dir.join("Cargo.lock"), lock.replace("checksum = \"bbbb\"", "checksum = \"bbbc\"")).unwrap();
    assert_eq!(key(&value_host()).unwrap(), k0, "a package outside the oracle's closure changed the key");
    let hosts: [(Host, &str); 3] = [
        (Host { rustc: Some("rustc 1.99.2\n".into()), ..value_host() }, "rustc -vV"),
        (Host { flags: vec!["build RUSTFLAGS=Some(\"-Copt-level=1\")".into()], ..value_host() }, "the build flags"),
        (Host { machine: Some("x86_64 linux flags : fpu avx512f".into()), ..value_host() }, "the machine"),
    ];
    for (h, what) in hosts {
        assert_ne!(key(&h).unwrap(), k0, "{what} is not in the key");
    }
    assert!(key(&Host { rustc: None, ..value_host() }).is_none(), "a key without the toolchain was complete");
    std::fs::remove_dir_all(dir.join("crates/helper")).unwrap();
    assert!(key(&value_host()).is_none(), "a key without a workspace crate of the closure was complete");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The cache directory of a test, and an oracle that counts its calls and answers `v<n>` (or
/// nothing for a query of part 9: an undecided value).
fn value_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("niji-oracle-values-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn query(func: &str, part: usize) -> Query {
    Query::new("cr", func, 'u', &[0x3fff_8000_0000_0000_0000, 7], part)
}

#[test]
fn a_warm_value_is_served_without_the_oracle_and_an_undecided_one_is_never_stored() {
    // no spec clause: the value cache (D-10.131) stores decided values only and serves them.
    let dir = value_dir("warm");
    let calls = AtomicUsize::new(0);
    let oracle = |q: &Query| {
        calls.fetch_add(1, Ordering::Relaxed);
        (q.part != 9).then(|| format!("{} {}", q.func, q.part))
    };
    let mut v = Values::open_in(&dir, Some("key one"), oracle);
    for f in ["a", "b", "c", "d"] {
        assert_eq!(v.get(&query(f, 0)).as_deref(), Some(format!("{f} 0").as_str()));
    }
    assert_eq!(v.get(&query("e", 9)), None);
    v.save();
    assert_eq!(calls.swap(0, Ordering::Relaxed), 5);
    let mut v = Values::open_in(&dir, Some("key one"), oracle);
    let canary = calls.swap(0, Ordering::Relaxed);
    assert_eq!(canary, 2, "the canary did not recompute its two entries");
    for f in ["a", "b", "c", "d"] {
        assert_eq!(v.get(&query(f, 0)).as_deref(), Some(format!("{f} 0").as_str()));
    }
    assert_eq!(calls.load(Ordering::Relaxed), 0, "a cached value was computed again");
    assert_eq!(v.get(&query("e", 9)), None);
    assert_eq!(calls.load(Ordering::Relaxed), 1, "an undecided value was stored");
    assert_eq!((v.hits, v.misses), (4, 1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_canary_that_differs_discards_the_whole_file() {
    // no spec clause: the value cache's canary (D-10.131) catches an oracle change the key missed.
    let dir = value_dir("canary");
    let mut v = Values::open_in(&dir, Some("key one"), |q: &Query| Some(format!("{} old", q.func)));
    for f in ["a", "b", "c"] {
        v.get(&query(f, 0));
    }
    v.save();
    // The changed oracle differs on one canary entry only ("a"); the file must not be used at all.
    let mut v = Values::open_in(&dir, Some("key one"), |q: &Query| Some(format!("{} {}", q.func, if q.func == "a" { "new" } else { "old" })));
    assert_eq!(v.get(&query("c", 0)).as_deref(), Some("c old"));
    assert_eq!((v.hits, v.misses), (0, 1), "a file whose canary differs was read");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_of_another_key_an_incomplete_file_and_unused_values_are_not_kept() {
    // no spec clause: the value cache (D-10.131) reads only complete files of its key and prunes.
    let dir = value_dir("prune");
    let oracle = |q: &Query| Some(format!("{} {}", q.func, q.part));
    let mut v = Values::open_in(&dir, Some("key one"), oracle);
    for f in ["a", "b", "c", "d"] {
        v.get(&query(f, 0));
    }
    v.save();
    let files = |d: &Path| {
        let mut n: Vec<String> = std::fs::read_dir(d).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        n.sort();
        n
    };
    let [one] = <[String; 1]>::try_from(files(&dir)).unwrap();
    // Another key reads nothing, and its save removes the other key's file.
    let mut v = Values::open_in(&dir, Some("key two"), oracle);
    v.get(&query("a", 0));
    v.get(&query("b", 0));
    v.get(&query("c", 0));
    assert_eq!(v.hits, 0, "a value of another key was served");
    v.save();
    let [two] = <[String; 1]>::try_from(files(&dir)).unwrap();
    assert_ne!(one, two);
    // "d" was not asked for under key two: it is gone; the file's three values are served.
    let mut v = Values::open_in(&dir, Some("key two"), oracle);
    for f in ["a", "b", "c", "d"] {
        v.get(&query(f, 0));
    }
    assert_eq!((v.hits, v.misses), (3, 1), "a stored value was not served");
    // A stored value the last run did not ask for is dropped when it saves.
    let mut v = Values::open_in(&dir, Some("key two"), oracle);
    v.get(&query("a", 0));
    v.get(&query("b", 0));
    v.save();
    let mut v = Values::open_in(&dir, Some("key two"), oracle);
    v.get(&query("c", 0));
    assert_eq!(v.hits, 0, "an unused value was kept");
    // A file cut short is not read.
    let text = std::fs::read_to_string(dir.join(&two)).unwrap();
    std::fs::write(dir.join(&two), text.lines().filter(|l| !l.starts_with("end ")).collect::<Vec<_>>().join("\n")).unwrap();
    let mut v = Values::open_in(&dir, Some("key two"), oracle);
    v.get(&query("a", 0));
    assert_eq!(v.hits, 0, "an incomplete file was read");
    // No key (off, or an input unreadable): nothing is read or written.
    let v = Values::open_in(&dir.join("off"), None, oracle);
    v.save();
    assert!(!dir.join("off").exists(), "a cache without a key wrote a file");
    // A file of another key under this key's name is not read either (its header names its key).
    let mut v = Values::open_in(&dir, Some("key one"), oracle);
    v.get(&query("a", 0));
    v.save();
    let [one_again] = <[String; 1]>::try_from(files(&dir)).unwrap();
    std::fs::rename(dir.join(&one_again), dir.join(&two)).unwrap();
    let mut v = Values::open_in(&dir, Some("key two"), oracle);
    v.get(&query("a", 0));
    assert_eq!(v.hits, 0, "a file holding another key's values was read");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_values_round_trip_and_the_variables_turn_the_cache_off() {
    // no spec clause: the value cache's encodings and switches (D-10.131).
    use niji_mathtools::mp::Rounded;
    let q = Query::new("hi", "cexpf64x", 'n', &[0, u128::MAX, 0x8000_0000_0000_0000_0001], 1);
    assert_eq!(Query::parse(&q.text()), Some(q));
    for v in [(0x7fff_0000_0000_0000_0000_0000_0000_0000_u128, Rounded { inexact: true, overflow: true, underflow: false }, false), (0, Rounded { inexact: true, overflow: false, underflow: true }, true), (1, Rounded::default(), false)] {
        assert_eq!(oracle_values::dec_cr(&oracle_values::enc_cr(&v)), Some(v));
    }
    for var in ["NIJI_NO_VERDICT_CACHE", "NIJI_NO_ORACLE_VALUES"] {
        assert!(oracle_values::disabled_by(|v| (v == var).then(|| "1".to_string())), "{var}=1 left the cache on");
        assert!(!oracle_values::disabled_by(|v| (v == var).then(|| "0".to_string())), "{var}=0 turned the cache off");
    }
}

#[test]
fn the_tally_value_is_the_decided_rounding_at_prec_plus_40_bits() {
    // no spec clause: the accuracy tally's reference value (D-10.131) is decided, so cacheable.
    use niji_mathtools::mp::{Approx, Big, Round};
    let two = Big::from_i64(2);
    let got = oracle_values::tally_value(53, |p| Approx { value: two.sqrt(p, Round::Nearest), exact: false }).unwrap();
    assert!(got.identical(&two.sqrt(93, Round::Nearest)), "not sqrt 2 rounded to 93 bits: {}", got.to_hex());
    // An approximation that always sits on a midpoint of 93-bit numbers is never decided.
    let mid = Big::one().add(&Big::pow2(-93));
    assert!(oracle_values::tally_value(53, |_| Approx { value: mid.clone(), exact: false }).is_err(), "an undecided value was returned as decided");
    // An exact value is rounded directly.
    let third = oracle_values::tally_value(53, |_| Approx { value: Big::from_i64(3), exact: true }).unwrap();
    assert!(third.identical(&Big::from_i64(3)));
}
