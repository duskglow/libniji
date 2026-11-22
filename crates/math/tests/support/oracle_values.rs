//! Value cache of the math oracle (ORACLE-VALUE-CACHE, D-10.131): the correctly rounded
//! values the C-oracle sweeps ask `niji_mathtools` for, kept across runs whose libniji bytes
//! differ. Only the oracle's side is cached; libniji's and the reference's results are read from
//! the programs' outputs on every run and compared with the (cached or fresh) oracle value, so a
//! line gets the same verdict either way.
//!
//! * **Key** ([`oracle_key`]): the bytes of every workspace crate in the oracle's `Cargo.lock`
//!   closure (`niji-mathtools` and what it depends on), the name, version, source and checksum of
//!   every other package in that closure, the sweep's own source (its wrappers `part`/`cr` choose
//!   the oracle routine, the precisions and the Ziv cap), this file, the root `Cargo.toml` (build
//!   profiles), `rust-toolchain.toml`, `.cargo/config.toml`, `rustc -vV`, the `RUSTFLAGS` the test
//!   was built with, the `RUSTFLAGS`/`CARGO_*` build variables of the run, the target, the CPU
//!   flags and the cache's format version. Any input that cannot be read makes the key incomplete:
//!   nothing is read or written. A value is found by its exact query ([`Query`]: kind, function,
//!   rounding mode, operand encodings, part).
//! * **Canary**: each file stores a few of its entries apart; on load the current oracle
//!   recomputes them and the whole file is discarded on any difference (a dependency the key
//!   missed).
//! * **Decided values only**: a correctly rounded result and its flags, or the reference value of
//!   the accuracy tally rounded to nearest at `prec + 40` bits ([`tally_value`]), both decided by
//!   Ziv's test, so they do not depend on what the process evaluated before. An undecided query is
//!   never stored.
//! * **Files**: `target/test-oracles/values/<shard>/<key id>`, one per shard and key, written to a
//!   temporary name and renamed (concurrent writers of a shard each leave a complete file; the
//!   last rename wins). A file holds only the values its shard asked for in the run that wrote it,
//!   and saving removes the shard's files of other keys, so the cache is at most one sweep's
//!   values per tree.
//! * `NIJI_NO_VERDICT_CACHE=1` (the nightly and release runs) or `NIJI_NO_ORACLE_VALUES=1`
//!   computes every value and writes nothing.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::hash::{DefaultHasher, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use niji_mathtools::mp::{Approx, Big, Round, Rounded};

/// The cache's format; part of the key.
const FORMAT: &str = "oracle-values 1";
/// The variable that turns the value cache off (the verdict cache's `NIJI_NO_VERDICT_CACHE` does too).
pub(crate) const NO_VALUES: &str = "NIJI_NO_ORACLE_VALUES";
/// The entries of each kind a file keeps apart as its canary.
const CANARY_PER_KIND: usize = 2;

/// One question to the oracle: `kind` (`cr`: the correctly rounded part and flags; `hi`: the
/// tally's reference value), the line's function, rounding mode, operand encodings and part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Query {
    pub(crate) kind: String,
    pub(crate) func: String,
    pub(crate) mode: char,
    pub(crate) ops: Vec<u128>,
    pub(crate) part: usize,
}

impl Query {
    pub(crate) fn new(kind: &str, func: &str, mode: char, ops: &[u128], part: usize) -> Self {
        Self { kind: kind.to_string(), func: func.to_string(), mode, ops: ops.to_vec(), part }
    }

    /// `<kind> <func> <mode> <part> <op>...` (operands in hex).
    pub(crate) fn text(&self) -> String {
        let mut s = format!("{} {} {} {}", self.kind, self.func, self.mode, self.part);
        for o in &self.ops {
            let _ = write!(s, " {o:x}");
        }
        s
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        let mut w = s.split(' ');
        let kind = w.next()?.to_string();
        let func = w.next()?.to_string();
        let mut m = w.next()?.chars();
        let mode = m.next()?;
        if m.next().is_some() {
            return None;
        }
        let part = w.next()?.parse().ok()?;
        let ops = w.map(|o| u128::from_str_radix(o, 16).ok()).collect::<Option<Vec<_>>>()?;
        Some(Self { kind, func, mode, ops, part })
    }
}

/// A correctly rounded part as cache text: `<bits> <inexact><underflow><overflow><exact zero>`.
pub(crate) fn enc_cr(v: &(u128, Rounded, bool)) -> String {
    let b = |x: bool| if x { '1' } else { '0' };
    format!("{:x} {}{}{}{}", v.0, b(v.1.inexact), b(v.1.underflow), b(v.1.overflow), b(v.2))
}

/// The inverse of [`enc_cr`].
pub(crate) fn dec_cr(s: &str) -> Option<(u128, Rounded, bool)> {
    let (bits, fl) = s.split_once(' ')?;
    let bits = u128::from_str_radix(bits, 16).ok()?;
    let f: Vec<bool> = fl.chars().map(|c| match c {
        '0' => Some(false),
        '1' => Some(true),
        _ => None,
    }).collect::<Option<_>>()?;
    let [inexact, underflow, overflow, zero] = <[bool; 4]>::try_from(f).ok()?;
    Some((bits, Rounded { inexact, overflow, underflow }, zero))
}

/// The accuracy tally's reference value of a function: the true value rounded to nearest at
/// `prec + 40` bits (unbounded exponent), decided by Ziv's test on the approximations `f` gives
/// (from `prec + 64` bits, doubling up to `16 (prec + 40)`). `Err` holds the last approximation
/// when the test cannot decide. Its error against the true value is below `2^-40` units in the
/// last place of a `prec`-bit format, far below the tally's printed `0.001`.
pub(crate) fn tally_value(prec: u32, f: impl Fn(u32) -> Approx) -> Result<Big, Big> {
    let q = prec.saturating_add(40);
    let cap = q.saturating_mul(16);
    let mut w = q.saturating_add(24);
    loop {
        let a = f(w);
        if a.exact || !a.value.is_finite() {
            return Ok(a.value.round(q, Round::Nearest));
        }
        if !a.value.is_zero() {
            let d = a.value.abs().mul_pow2(i64::from(w).saturating_neg());
            let lo = a.value.sub(&d).round(q, Round::Nearest);
            let hi = a.value.add(&d).round(q, Round::Nearest);
            if lo.identical(&hi) {
                return Ok(lo);
            }
        }
        if w >= cap {
            return Err(a.value);
        }
        w = w.saturating_mul(2).min(cap);
    }
}

/// The 128-bit content hash of `bytes` (two SipHash streams with different prefixes).
fn digest(bytes: &[u8]) -> String {
    let mut a = DefaultHasher::new();
    let mut b = DefaultHasher::new();
    a.write_u8(0x3c);
    b.write_u8(0xc3);
    a.write(bytes);
    b.write(bytes);
    format!("{:016x}{:016x}", a.finish(), b.finish())
}

/// One `[[package]]` of a `Cargo.lock`: name, version, source, checksum and dependency specs.
#[derive(Debug, Default)]
struct LockPackage {
    name: String,
    version: String,
    source: Option<String>,
    checksum: Option<String>,
    deps: Vec<String>,
}

fn unquote(s: &str) -> String {
    s.trim().trim_end_matches(',').trim().trim_matches('"').to_string()
}

/// The packages of a `Cargo.lock` text (the fields the key needs; nothing else is parsed).
fn lock_packages(text: &str) -> Vec<LockPackage> {
    let mut out: Vec<LockPackage> = Vec::new();
    let mut in_deps = false;
    for line in text.lines() {
        let t = line.trim();
        if t == "[[package]]" {
            out.push(LockPackage::default());
            in_deps = false;
            continue;
        }
        let Some(p) = out.last_mut() else { continue };
        if in_deps {
            if t.starts_with(']') {
                in_deps = false;
            } else if !t.is_empty() {
                p.deps.push(unquote(t));
            }
            continue;
        }
        let Some((k, v)) = t.split_once('=') else { continue };
        match k.trim() {
            "name" => p.name = unquote(v),
            "version" => p.version = unquote(v),
            "source" => p.source = Some(unquote(v)),
            "checksum" => p.checksum = Some(unquote(v)),
            "dependencies" => {
                let v = v.trim();
                if let Some(inner) = v.strip_prefix('[') {
                    match inner.strip_suffix(']') {
                        Some(one_line) => p.deps.extend(one_line.split(',').map(unquote).filter(|d| !d.is_empty())),
                        None => in_deps = true,
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// The packages `root` depends on in `pkgs`, transitively, itself included (indices, sorted).
/// `None` when a dependency spec matches no package or more than one.
fn lock_closure(pkgs: &[LockPackage], root: &str) -> Option<BTreeSet<usize>> {
    let find = |spec: &str| -> Option<usize> {
        let mut w = spec.split_whitespace();
        let name = w.next()?;
        let version = w.next();
        let mut hits = pkgs.iter().enumerate().filter(|(_, p)| p.name == name && version.is_none_or(|v| p.version == v)).map(|(i, _)| i);
        let first = hits.next()?;
        if hits.next().is_some() { None } else { Some(first) }
    };
    let mut seen = BTreeSet::new();
    let mut todo = vec![find(root)?];
    while let Some(i) = todo.pop() {
        if !seen.insert(i) {
            continue;
        }
        for d in &pkgs.get(i)?.deps {
            todo.push(find(d)?);
        }
    }
    Some(seen)
}

/// Every file under `dir` (sorted by path), as `(path relative to dir, bytes)`.
fn tree_files(dir: &Path) -> Option<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).ok()? {
            let p = e.ok()?.path();
            let ft = std::fs::symlink_metadata(&p).ok()?.file_type();
            if ft.is_dir() {
                todo.push(p);
            } else {
                let rel = p.strip_prefix(dir).ok()?.to_string_lossy().into_owned();
                out.push((rel, std::fs::read(&p).ok()?));
            }
        }
    }
    out.sort();
    Some(out)
}

/// The directory of workspace package `name` under `root`: the `crates/*/` or top-level
/// directory whose `Cargo.toml` names it. `None` unless exactly one does.
fn workspace_dir(root: &Path, name: &str) -> Option<PathBuf> {
    let want = format!("name = \"{name}\"");
    let mut cands = Vec::new();
    for base in [root.join("crates"), root.to_path_buf()] {
        let Ok(rd) = std::fs::read_dir(&base) else { continue };
        for e in rd.flatten() {
            let toml = e.path().join("Cargo.toml");
            if let Ok(t) = std::fs::read_to_string(&toml) {
                if t.lines().any(|l| l.trim() == want) {
                    cands.push(e.path());
                }
            }
        }
    }
    cands.sort();
    match cands.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

/// The inputs a key is made of besides the tree (kept apart so tests can vary them).
#[derive(Clone, Debug)]
pub(crate) struct Host {
    /// `rustc -vV` (`None`: could not run).
    pub(crate) rustc: Option<String>,
    /// The build variables: compile-time `RUSTFLAGS`, then the run's `RUSTFLAGS`/`CARGO_*` build settings.
    pub(crate) flags: Vec<String>,
    /// The target and CPU flags line.
    pub(crate) machine: Option<String>,
}

impl Host {
    pub(crate) fn current(root: &Path) -> Self {
        let rustc = std::process::Command::new("rustc").arg("-vV").current_dir(root).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
        let mut flags = vec![
            format!("build RUSTFLAGS={:?}", option_env!("RUSTFLAGS")),
            format!("build CARGO_ENCODED_RUSTFLAGS={:?}", option_env!("CARGO_ENCODED_RUSTFLAGS")),
            format!("debug_assertions={}", cfg!(debug_assertions)),
        ];
        let mut run: Vec<String> = std::env::vars()
            .filter(|(k, _)| k == "RUSTFLAGS" || k == "CARGO_ENCODED_RUSTFLAGS" || k.starts_with("CARGO_PROFILE_") || k.starts_with("CARGO_BUILD_") || (k.starts_with("CARGO_TARGET_") && k != "CARGO_TARGET_DIR" && k != "CARGO_TARGET_TMPDIR") || k.starts_with("CARGO_UNSTABLE_"))
            .map(|(k, v)| format!("run {k}={v}"))
            .collect();
        run.sort();
        flags.extend(run);
        let cpu = std::fs::read_to_string("/proc/cpuinfo").ok().and_then(|c| c.lines().find(|l| l.starts_with("flags")).map(|l| l.trim_end().to_string()));
        let machine = cpu.map(|c| format!("{} {} {c}", std::env::consts::ARCH, std::env::consts::OS));
        Self { rustc, flags, machine }
    }
}

/// The key text of the oracle as `sweep_src` calls it in the tree at `root` (one input per line);
/// `None` when an input cannot be read.
pub(crate) fn oracle_key(root: &Path, sweep_src: &Path, host: &Host) -> Option<String> {
    let mut lines = vec![FORMAT.to_string()];
    let file = |lines: &mut Vec<String>, label: &str, p: &Path, optional: bool| -> Option<()> {
        match std::fs::read(p) {
            Ok(b) => lines.push(format!("{label}\t{}\t{}", b.len(), digest(&b))),
            Err(e) if optional && e.kind() == std::io::ErrorKind::NotFound => lines.push(format!("{label}\tabsent")),
            Err(_) => return None,
        }
        Some(())
    };
    file(&mut lines, "sweep", sweep_src, false)?;
    file(&mut lines, "value cache", &root.join("crates/math/tests/support/oracle_values.rs"), false)?;
    file(&mut lines, "workspace manifest", &root.join("Cargo.toml"), false)?;
    file(&mut lines, "toolchain file", &root.join("rust-toolchain.toml"), true)?;
    file(&mut lines, "cargo config", &root.join(".cargo/config.toml"), true)?;
    file(&mut lines, "cargo config (old name)", &root.join(".cargo/config"), true)?;
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).ok()?;
    let pkgs = lock_packages(&lock);
    for i in lock_closure(&pkgs, "niji-mathtools")? {
        let p = pkgs.get(i)?;
        match &p.source {
            Some(src) => lines.push(format!("package\t{}\t{}\t{src}\t{}", p.name, p.version, p.checksum.as_deref().unwrap_or("-"))),
            None => {
                let dir = workspace_dir(root, &p.name)?;
                lines.push(format!("workspace package\t{}\t{}", p.name, p.version));
                for (rel, bytes) in tree_files(&dir)? {
                    lines.push(format!("  {rel}\t{}\t{}", bytes.len(), digest(&bytes)));
                }
            }
        }
    }
    lines.push(format!("rustc\t{}", host.rustc.as_deref()?.trim_end().replace('\n', " | ")));
    lines.extend(host.flags.iter().map(|f| format!("flags\t{f}")));
    lines.push(format!("machine\t{}", host.machine.as_deref()?));
    Some(lines.join("\n"))
}

/// Whether the environment turns the cache off.
pub(crate) fn disabled() -> bool {
    disabled_by(|v| std::env::var(v).ok())
}

/// Whether the variables `get` reads turn the cache off.
pub(crate) fn disabled_by(get: impl Fn(&str) -> Option<String>) -> bool {
    ["NIJI_NO_VERDICT_CACHE", NO_VALUES].iter().any(|v| get(v).is_some_and(|s| s == "1"))
}

/// Writes `line` straight to stderr (libtest hides a passing test's `eprintln!`).
fn say(line: &str) {
    use std::io::Write as _;
    let _ = std::io::stderr().write_all(line.as_bytes());
}

/// What a cache file holds: its canary and its other entries (query text -> value text).
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Contents {
    pub(crate) canary: BTreeMap<String, String>,
    pub(crate) values: BTreeMap<String, String>,
}

/// `<FORMAT>`, `key <id>`, `c|v <query>\t<value>` lines, then `end <entries>`.
fn render(id: &str, c: &Contents) -> String {
    let mut s = format!("{FORMAT}\nkey {id}\n");
    for (tag, map) in [('c', &c.canary), ('v', &c.values)] {
        for (q, v) in map {
            let _ = writeln!(s, "{tag} {q}\t{v}");
        }
    }
    let _ = writeln!(s, "end {}", c.canary.len().saturating_add(c.values.len()));
    s
}

/// The inverse of [`render`]; `None` for a file of another format or key, or an incomplete one.
fn parse_file(id: &str, text: &str) -> Option<Contents> {
    let mut lines = text.lines();
    if lines.next()? != FORMAT || lines.next()? != format!("key {id}") {
        return None;
    }
    let mut c = Contents::default();
    for l in lines {
        if let Some(n) = l.strip_prefix("end ") {
            let n: usize = n.parse().ok()?;
            return (n == c.canary.len().saturating_add(c.values.len())).then_some(c);
        }
        let (tag, rest) = l.split_once(' ')?;
        let (q, v) = rest.split_once('\t')?;
        let map = match tag {
            "c" => &mut c.canary,
            "v" => &mut c.values,
            _ => return None,
        };
        map.insert(q.to_string(), v.to_string());
    }
    None
}

/// The values of one shard: those loaded (after the canary held) and those asked for in this run.
pub(crate) struct Values<F: Fn(&Query) -> Option<String>> {
    /// `<dir>/<key id>`; `None` when the cache is off or the key incomplete.
    file: Option<(PathBuf, String)>,
    eval: F,
    loaded: BTreeMap<String, String>,
    asked: BTreeMap<String, String>,
    /// Hits and computed values of this run.
    pub(crate) hits: usize,
    pub(crate) misses: usize,
}

impl<F: Fn(&Query) -> Option<String>> Values<F> {
    /// The cache of shard `name` of the sweep whose source is `sweep_src`, in this tree.
    pub(crate) fn open(name: &str, sweep_src: &str, eval: F) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let key = if disabled() { None } else { oracle_key(&root, Path::new(sweep_src), &Host::current(&root)) };
        Self::open_in(&root.join("target/test-oracles/values").join(name), key.as_deref(), eval)
    }

    /// [`Values::open`] with the directory and key given (`None`: no cache).
    pub(crate) fn open_in(dir: &Path, key: Option<&str>, eval: F) -> Self {
        let mut v = Self { file: key.map(|k| (dir.to_path_buf(), digest(k.as_bytes()))), eval, loaded: BTreeMap::new(), asked: BTreeMap::new(), hits: 0, misses: 0 };
        let Some((dir, id)) = &v.file else { return v };
        let Some(c) = std::fs::read_to_string(dir.join(id)).ok().and_then(|t| parse_file(id, &t)) else { return v };
        // The canary: the current oracle must give every value it holds.
        for (q, want) in &c.canary {
            let got = Query::parse(q).and_then(|q| (v.eval)(&q));
            if got.as_deref() != Some(want.as_str()) {
                say(&format!("oracle values {}: canary {q} gave {got:?}, the file holds {want}; the file is discarded\n", dir.display()));
                return v;
            }
        }
        v.loaded = c.values;
        v.loaded.extend(c.canary);
        v
    }

    /// The value of query text `t` if this run or the file has it (a miss is counted).
    fn cached(&mut self, t: &str) -> Option<String> {
        if let Some(s) = self.asked.get(t) {
            return Some(s.clone());
        }
        if let Some(s) = self.loaded.remove(t) {
            self.hits = self.hits.saturating_add(1);
            self.asked.insert(t.to_string(), s.clone());
            return Some(s);
        }
        self.misses = self.misses.saturating_add(1);
        None
    }

    /// The value of `q`: the stored one, or `eval`'s (stored when it decided one).
    pub(crate) fn get(&mut self, q: &Query) -> Option<String> {
        let t = q.text();
        if let Some(s) = self.cached(&t) {
            return Some(s);
        }
        let s = (self.eval)(q)?;
        self.asked.insert(t, s.clone());
        Some(s)
    }

    /// Writes the values asked for in this run (and removes the shard's files of other keys).
    pub(crate) fn save(self) {
        let Some((dir, id)) = &self.file else { return };
        let mut c = Contents::default();
        for kind in ["cr ", "hi "] {
            for (q, v) in self.asked.iter().filter(|(q, _)| q.starts_with(kind)).take(CANARY_PER_KIND) {
                c.canary.insert(q.clone(), v.clone());
            }
        }
        c.values = self.asked.into_iter().filter(|(q, _)| !c.canary.contains_key(q)).collect();
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let tmp = dir.join(format!(".{id}.{}.{nanos}.tmp", std::process::id()));
        if std::fs::write(&tmp, render(id, &c)).is_err() || std::fs::rename(&tmp, dir.join(id)).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return;
        }
        // FS-ORDER: each file of another key is removed on its own; the order does not matter.
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let n = e.file_name();
                if n.to_str().is_some_and(|n| n != id && !n.starts_with('.')) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
    }

    pub(crate) fn summary(&self, name: &str) {
        say(&format!("oracle values {name}: {} cached, {} computed\n", self.hits, self.misses));
    }
}

/// The oracle of a sweep checked on several threads.
pub(crate) type Eval = fn(&Query) -> Option<String>;

/// [`Values::get`] for a cache the threads of a shard share: the lock is not held while the
/// oracle computes (two threads may compute one value; both get the same).
pub(crate) fn get_shared(m: &Mutex<Values<Eval>>, q: &Query) -> Option<String> {
    let t = q.text();
    let eval = {
        let mut v = m.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(s) = v.cached(&t) {
            return Some(s);
        }
        v.eval
    };
    let s = eval(q)?;
    m.lock().unwrap_or_else(PoisonError::into_inner).asked.insert(t, s.clone());
    Some(s)
}
