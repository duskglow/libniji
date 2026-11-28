//! The product cache of `cargo xtask so` (XTASK-SO-PRODUCT-CACHE, coordinator ruling #8827, challenge #8851).
//!
//! A product build re-made every product on every run: a no-op build of the default (traced) layout took 329 s on
//! the Zen 1 build host, 274 s of it the trace-at-build of `libc.a`. Two caches, both keyed on content only (sha256 of
//! bytes, never a modification time), let a build skip work whose inputs are those of an earlier full build:
//!
//! * **The set** (`target/so-cache/sets/<key>`): every product of `target/niji` and the files later commands read
//!   from `target/product` ([`side_files`]: the start-up lists the products' marker names, the bin plan, the trace
//!   logs, the placeholder bodies). The key ([`set_key`]) covers every file of the tree but `target/`,
//!   `fuzz/target/` and `.git` ([`tree_lines`]), the headers the build staged from the host before the key is taken
//!   (the kernel UAPI copies, [`staged_lines`]), the system headers the trace's probes include ([`probe_lines`]),
//!   the xtask binary, the tools by content and by version, the environment ([`env_lines`]) and the profile and
//!   layout. Every cargo output is a function of those (the sources, `Cargo.lock`, the toolchain and the flags), so
//!   the set key does not need them. A hit copies the set back, checks every copy against the stored sha256 and
//!   writes the manifest afresh; a set that does not verify is a miss and the build runs in full.
//! * **The trace** (`target/so-cache/traces/<key>`): the lists and log of a trace-at-build. Its key ([`trace_key`])
//!   takes the archive the trace links (`hot-text.pre-group.a`, a cargo output after the post-processing) by
//!   content instead of the Rust sources, so a change that leaves the library's archive as it was (a test, another
//!   product's crate, a document) keeps the trace; the tree lines leave out `crates/**/*.rs` only (the trace reads no
//!   Rust source: `tools/hottext/regen.sh` compiles C probes and reads `page_faults.c`).
//!
//! Only a finished build is stored (the set after its manifest, the trace after a successful trace). An entry that
//! does not read back exactly (magic, key, sum of its index, every file's sha256) is a miss, never a hit. Under the CI
//! train (`NIJI_TRAIN` set, not `0`) and with `NIJI_SO_CACHE=0` ([`OFF_ENV`]) neither cache is read or written, as the
//! layout verdict cache does (the train decision: RULING NEEDED in the story's hand-back).
use std::fs;
use std::path::{Path, PathBuf};

use crate::import::Entry;
use crate::sha256::{sha256_files, sha256_text};

/// Where the entries live, relative to the workspace root.
pub(crate) const DIR: &str = "target/so-cache";
/// `0` turns both caches off.
pub(crate) const OFF_ENV: &str = "NIJI_SO_CACHE";
const SET_MAGIC: &str = "niji so set cache 1";
const TRACE_MAGIC: &str = "niji so trace cache 1";
/// Sets kept (about 34 MB each for the test profile), the newest by use.
const KEEP_SETS: usize = 3;
/// Traces kept (about 0.2 MB each).
const KEEP_TRACES: usize = 4;
/// Variables that change what the build makes, besides those of `layout_verdict` and the prefixed ones below.
const ENV_EXTRA: &[&str] = &["AR", "CFLAGS", "CPPFLAGS", "LDFLAGS", "SOURCE_DATE_EPOCH", "HOTTEXT_GDB", "LANG", "LC_ALL", "LC_COLLATE", "LC_CTYPE", "PYTHONPATH", "PYTHONHASHSEED", "TZ", "SHELL"];
/// Every variable with one of these prefixes is a key input (`NIJI_FULL_PROFILE`, `NIJI_LAYOUT_TRACE`, the plants
/// tag, `RUSTFLAGS`, `CARGO_*`, ...).
const ENV_PREFIXES: &[&str] = &["NIJI_", "CARGO", "RUST"];
/// Tools the build runs besides the compiler driver's (which `layout_verdict::host_inputs` covers), by content.
const TOOLS_EXTRA: &[&str] = &["ar", "objcopy", "strip", "ld", "as", "nm", "readelf", "gzip", "python3", "bash", "sh", "sort", "sha256sum", "rustc", "cargo"];
/// The trace's C probes and the start-up probe, preprocessed for the system headers they include.
const PROBES_DIR: &str = "tools/hottext/probes";
const PAGE_FAULTS: &str = "crates/core/tests/oracle/page_faults.c";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Mode {
    Use,
    Train,
    Off,
}

pub(crate) fn mode(env: &dyn Fn(&str) -> Option<String>) -> Mode {
    if env("NIJI_TRAIN").is_some_and(|v| !v.is_empty() && v != "0") {
        Mode::Train
    } else if env(OFF_ENV).is_some_and(|v| v == "0") {
        Mode::Off
    } else {
        Mode::Use
    }
}

/// What a key is taken from, besides the tree: the build's own process, or a test's stand-ins.
pub(crate) struct Ctx<'a> {
    pub root: &'a Path,
    /// The xtask binary (the steps, their constants and flags).
    pub exe: PathBuf,
    pub env: &'a dyn Fn(&str) -> Option<String>,
    pub env_names: Vec<String>,
    /// Tools by name and resolved path (`None`: not found), hashed by content.
    pub tools: Vec<(String, Option<PathBuf>)>,
    /// Lines that need no hashing: tool versions, `rustc -vV`, the compiler's own tools and libraries.
    pub versions: Vec<String>,
    /// `<profile> <layout>`.
    pub flags: String,
    /// The C compiler that preprocesses the probes.
    pub cc: String,
}

impl<'a> Ctx<'a> {
    /// The context of this process.
    pub(crate) fn of_process(root: &'a Path, env: &'a dyn Fn(&str) -> Option<String>, flags: String) -> Result<Ctx<'a>, String> {
        let exe = std::env::current_exe().map_err(|e| format!("the xtask binary's path: {e}"))?;
        let env_names: Vec<String> = std::env::vars_os().filter_map(|(k, _)| k.into_string().ok()).collect();
        let path = env("PATH").unwrap_or_default();
        let gdb = env("HOTTEXT_GDB").unwrap_or_else(|| "gdb".into());
        let cc = env("CC").unwrap_or_else(|| "cc".into());
        let mut tools: Vec<(String, Option<PathBuf>)> = TOOLS_EXTRA.iter().map(|t| (t.to_string(), crate::layout_verdict::which(t, &path).ok())).collect();
        tools.push(("gdb".into(), crate::layout_verdict::which(&gdb, &path).ok()));
        tools.push(("cc".into(), crate::layout_verdict::which(&cc, &path).ok()));
        let mut versions = crate::layout_verdict::host_inputs(env, &env_names)?;
        versions.extend(crate::import::toolchain(root));
        Ok(Ctx { root, exe, env, env_names, tools, versions, flags, cc })
    }
}

/// The key lines of the environment: every variable of [`ENV_EXTRA`], `layout_verdict`'s list and the
/// [`ENV_PREFIXES`] ones, set or not.
pub(crate) fn env_lines(env: &dyn Fn(&str) -> Option<String>, names: &[String]) -> Vec<String> {
    let mut keys: Vec<String> = ENV_EXTRA.iter().chain(crate::layout_verdict::ENV).map(|s| s.to_string()).chain(names.iter().filter(|k| ENV_PREFIXES.iter().any(|p| k.starts_with(p))).cloned()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter().map(|k| format!("env {k}={:?}", env(&k))).collect()
}

/// The key lines of the host: the xtask binary and every tool by content, the environment and the version lines.
pub(crate) fn host_lines(ctx: &Ctx) -> Result<Vec<String>, String> {
    let mut out = vec![format!("checker {}", file_sha(&ctx.exe)?)];
    out.extend(env_lines(ctx.env, &ctx.env_names));
    for (name, p) in &ctx.tools {
        out.push(match p {
            Some(p) => format!("tool {name} {}", file_sha(p)?),
            None => format!("tool {name} absent"),
        });
    }
    out.extend(ctx.versions.iter().map(|v| format!("version {v}")));
    Ok(out)
}

/// The sha256 of one file (symbolic links followed).
pub(crate) fn file_sha(p: &Path) -> Result<String, String> {
    let c = p.canonicalize().map_err(|e| format!("{}: {e}", p.display()))?;
    sha256_files(Path::new("/"), &[c.display().to_string()])?.pop().ok_or_else(|| format!("{}: no sum", p.display()))
}

/// Whether a tree path (relative, `/`-separated) is left out of every key: the build's own output, git's data, and
/// Python's bytecode caches (`__pycache__`, which a test run leaves beside `spec/tools/*.py`): Python derives them from
/// the `.py` files, which are key inputs, and compiles the source again when a cache does not match it.
fn outside_tree(rel: &str) -> bool {
    rel == "target" || rel == "fuzz/target" || rel == ".git" || rel.rsplit('/').next() == Some("__pycache__")
}

/// The key lines of the tree under `root`: `file <path> <sha256>` for every regular file, `link <path> <target>`
/// for every symbolic link (not followed), sorted by path, but [`outside_tree`] and, with `skip_rust`, the
/// `crates/**/*.rs` files. Contents only: a file whose modification time alone changed keeps its line.
pub(crate) fn tree_lines(root: &Path, skip_rust: bool) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    let mut lines = Vec::new();
    let mut stack = vec![String::new()];
    while let Some(rel) = stack.pop() {
        let dir = if rel.is_empty() { root.to_path_buf() } else { root.join(&rel) };
        for e in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
            let name = e.file_name().to_string_lossy().into_owned();
            let path = if rel.is_empty() { name } else { format!("{rel}/{name}") };
            if outside_tree(&path) {
                continue;
            }
            let ft = e.file_type().map_err(|er| format!("{path}: {er}"))?;
            if ft.is_symlink() {
                let t = fs::read_link(e.path()).map_err(|er| format!("{path}: {er}"))?;
                lines.push(format!("link {path} {}", t.display()));
            } else if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                if !(skip_rust && path.starts_with("crates/") && path.ends_with(".rs")) {
                    files.push(path);
                }
            } else {
                lines.push(format!("other {path}"));
            }
        }
    }
    files.sort();
    for (f, h) in files.iter().zip(sha256_files(root, &files)?) {
        lines.push(format!("file {f} {h}"));
    }
    lines.sort();
    Ok(lines)
}

/// The key lines of what the build has staged in `release` before the key is taken (the headers, among them the
/// host's kernel UAPI copies).
pub(crate) fn staged_lines(release: &Path) -> Result<Vec<String>, String> {
    let entries = crate::import::hash_entries(release, owned_listing(release)?, str::to_string)?;
    Ok(entries
        .into_iter()
        .map(|e| match e {
            Entry::File { path, sha } => format!("staged {path} {sha}"),
            Entry::Link { path, target } => format!("staged-link {path} {target}"),
        })
        .collect())
}

/// The key lines of the system headers the trace's probes include: each probe preprocessed by `cc`.
pub(crate) fn probe_lines(root: &Path, cc: &str) -> Result<Vec<String>, String> {
    let mut probes: Vec<String> = match fs::read_dir(root.join(PROBES_DIR)) {
        Ok(rd) => rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".c")).map(|n| format!("{PROBES_DIR}/{n}")).collect(),
        Err(_) => Vec::new(),
    };
    probes.sort();
    if root.join(PAGE_FAULTS).is_file() {
        probes.push(PAGE_FAULTS.to_string());
    }
    let mut out = Vec::new();
    for p in probes {
        let o = std::process::Command::new(cc).args(["-E", "-std=gnu17", "-O2"]).arg(root.join(&p)).current_dir(root).output().map_err(|e| format!("{cc} -E {p}: {e}"))?;
        let body = if o.status.success() { sha256_text(&String::from_utf8_lossy(&o.stdout))? } else { "does-not-preprocess".into() };
        out.push(format!("probe {p} {body}"));
    }
    Ok(out)
}

/// Whether a products path is the product build's own: not below another command's entry (`so::OTHER_COMMANDS`).
fn owned(path: &str) -> bool {
    !crate::so::OTHER_COMMANDS.contains(&path.split('/').next().unwrap_or(""))
}

/// The listing of `release` without the entries of other commands.
fn owned_listing(release: &Path) -> Result<Vec<Entry>, String> {
    Ok(crate::import::listing(release)?.into_iter().filter(|e| owned(e.path())).collect())
}

fn key_of(magic: &str, parts: &[(&str, &[String])]) -> Result<String, String> {
    let mut t = format!("{magic}\n");
    for (class, lines) in parts {
        for l in *lines {
            t.push_str(class);
            t.push(' ');
            t.push_str(l);
            t.push('\n');
        }
    }
    sha256_text(&t)
}

/// The set's key: the profile and layout, the tree's root, the host, the whole tree, the staged headers and the
/// probes' system headers.
pub(crate) fn set_key(ctx: &Ctx, release: &Path) -> Result<String, String> {
    let flags = vec![ctx.flags.clone(), format!("root {}", ctx.root.canonicalize().map_err(|e| e.to_string())?.display())];
    let (host, tree, staged, probes) = (host_lines(ctx)?, tree_lines(ctx.root, false)?, staged_lines(release)?, probe_lines(ctx.root, &ctx.cc)?);
    key_of(SET_MAGIC, &[("flags", &flags), ("host", &host), ("tree", &tree), ("staged", &staged), ("probes", &probes)])
}

/// The trace's key: the profile, the tree's root, the host, the archive it links by content, the tree but its
/// Rust sources, and the probes' system headers.
pub(crate) fn trace_key(ctx: &Ctx, archive: &Path) -> Result<String, String> {
    let flags = vec![ctx.flags.clone(), format!("root {}", ctx.root.canonicalize().map_err(|e| e.to_string())?.display())];
    let cargo = vec![format!("pre-group-archive {}", file_sha(archive)?)];
    let (host, tree, probes) = (host_lines(ctx)?, tree_lines(ctx.root, true)?, probe_lines(ctx.root, &ctx.cc)?);
    key_of(TRACE_MAGIC, &[("flags", &flags), ("host", &host), ("cargo", &cargo), ("tree", &tree), ("probes", &probes)])
}

/// The files of `product` (`target/product`) a later command reads: every regular file directly in the profile's
/// directory but the archives and dep-info (`*.a`, `*.d`), and every regular file below `placeholders/`. Relative,
/// sorted.
pub(crate) fn side_files(product: &Path, profile: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(product.join(profile)) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if e.file_type().is_ok_and(|t| t.is_file()) && !n.ends_with(".a") && !n.ends_with(".d") {
                out.push(format!("{profile}/{n}"));
            }
        }
    }
    let mut stack = vec!["placeholders".to_string()];
    while let Some(rel) = stack.pop() {
        let Ok(rd) = fs::read_dir(product.join(&rel)) else { continue };
        for e in rd.flatten() {
            let p = format!("{rel}/{}", e.file_name().to_string_lossy());
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(p),
                Ok(t) if t.is_file() => out.push(p),
                _ => {}
            }
        }
    }
    out.sort();
    out
}

/// An entry's index: magic, key, one line per record, and the sha256 of all that.
fn index_text(magic: &str, key: &str, records: &[String]) -> Result<String, String> {
    let mut t = format!("{magic}\nkey {key}\n");
    for r in records {
        if r.contains('\n') {
            return Err("a record holds a newline".into());
        }
        t.push_str(r);
        t.push('\n');
    }
    let sum = sha256_text(&t)?;
    Ok(format!("{t}sum {sum}\n"))
}

/// The records of the index at `dir/index` for `key`: `Ok(None)` when there is none, `Err` when it does not read
/// back exactly.
fn read_index(dir: &Path, magic: &str, key: &str) -> Result<Option<Vec<String>>, String> {
    let text = match fs::read(dir.join("index")) {
        Ok(b) => String::from_utf8(b).map_err(|_| "index not UTF-8".to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let body = text.strip_suffix('\n').ok_or("index: no final newline")?;
    let (head, sum_line) = body.rsplit_once('\n').ok_or("index too short")?;
    if sum_line.strip_prefix("sum ") != Some(sha256_text(&format!("{head}\n"))?.as_str()) {
        return Err("index: the sum does not match the text".into());
    }
    let mut lines = head.lines();
    if lines.next() != Some(magic) {
        return Err("index: not an entry of this cache".into());
    }
    if lines.next().and_then(|l| l.strip_prefix("key ")) != Some(key) {
        return Err("index: the entry is for another key".into());
    }
    Ok(Some(lines.map(str::to_string).collect()))
}

fn copy_into(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(d) = dst.parent() {
        fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    fs::copy(src, dst).map(|_| ()).map_err(|e| format!("copy {} to {}: {e}", src.display(), dst.display()))
}

/// The use count of the newest entry under `kind_dir`, plus one (the order kept by [`evict`]; never a time).
fn next_stamp(kind_dir: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(kind_dir) else { return 1 };
    rd.flatten().filter_map(|e| fs::read_to_string(e.path().join("stamp")).ok()).filter_map(|s| s.trim().parse::<u64>().ok()).max().unwrap_or(0) + 1
}

fn stamp(entry: &Path, kind_dir: &Path) {
    let n = next_stamp(kind_dir);
    let _ = fs::write(entry.join("stamp"), format!("{n}\n"));
}

/// Keeps the `keep` entries of `kind_dir` used last, and removes left-over temporary directories.
fn evict(kind_dir: &Path, keep: usize) {
    let Ok(rd) = fs::read_dir(kind_dir) else { return };
    let mut entries: Vec<(u64, PathBuf)> = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        if e.file_name().to_string_lossy().starts_with(".tmp-") {
            let _ = fs::remove_dir_all(&p);
            continue;
        }
        let n = fs::read_to_string(p.join("stamp")).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        entries.push((n, p));
    }
    entries.sort();
    let extra = entries.len().saturating_sub(keep);
    for (_, p) in entries.into_iter().take(extra) {
        let _ = fs::remove_dir_all(p);
    }
}

/// Writes an entry: the files (`rel` under `from` -> `files/<tag>/<rel>`), its index, then moves it into place.
fn write_entry(kind_dir: &Path, key: &str, magic: &str, records: &[String], files: &[(&str, &Path, String)]) -> Result<(), String> {
    fs::create_dir_all(kind_dir).map_err(|e| format!("{}: {e}", kind_dir.display()))?;
    let tmp = kind_dir.join(format!(".tmp-{}-{key}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    for (tag, from, rel) in files {
        copy_into(&from.join(rel), &tmp.join("files").join(tag).join(rel))?;
    }
    fs::write(tmp.join("index"), index_text(magic, key, records)?).map_err(|e| format!("{}: {e}", tmp.display()))?;
    let dst = kind_dir.join(key);
    let _ = fs::remove_dir_all(&dst);
    fs::rename(&tmp, &dst).map_err(|e| format!("{}: {e}", dst.display()))?;
    stamp(&dst, kind_dir);
    Ok(())
}

/// The set cache of one build.
pub(crate) struct Sets {
    dir: PathBuf,
    key: String,
}

impl Sets {
    /// The set entry for this build's key (`None`, with the reason printed, when the cache is off or the key cannot
    /// be taken: the build then runs in full and stores nothing).
    pub(crate) fn open(cache_root: &Path, ctx: &Ctx, release: &Path) -> Option<Sets> {
        match mode(ctx.env) {
            Mode::Train => {
                println!("so: product cache off: NIJI_TRAIN is set (the cache serves local builds only)");
                None
            }
            Mode::Off => {
                println!("so: product cache off: {OFF_ENV}=0");
                None
            }
            Mode::Use => match set_key(ctx, release) {
                Ok(key) => Some(Sets { dir: cache_root.join(DIR).join("sets"), key }),
                Err(e) => {
                    println!("so: product cache not usable ({e}): every product built");
                    None
                }
            },
        }
    }

    /// Restores the stored set into `release` (holding only what the build staged so far) and its side files into
    /// `product`, each copy checked against its stored sha256, the profile marker last. `Ok(true)`: restored. On a
    /// miss or an entry that does not verify, `Ok(false)` (the reason printed; `release` may hold partial copies but
    /// no profile marker: the caller clears it).
    pub(crate) fn restore(&self, release: &Path, product: &Path) -> Result<bool, String> {
        let entry = self.dir.join(&self.key);
        let records = match read_index(&entry, SET_MAGIC, &self.key) {
            Ok(Some(r)) => r,
            Ok(None) => {
                println!("so: product cache miss {}: every product built", self.key);
                return Ok(false);
            }
            Err(e) => {
                println!("so: product cache entry {} is not usable ({e}): a miss, every product built", entry.display());
                return Ok(false);
            }
        };
        match restore_records(&entry, &records, release, product) {
            Ok(n) => {
                stamp(&entry, &self.dir);
                println!("so: product cache hit {}: the tree, the staged headers, the tools, the xtask binary, the environment, the profile and the layout are those of a full build; {n} products restored and checked by sha256 ({OFF_ENV}=0 builds them)", self.key);
                Ok(true)
            }
            Err(e) => {
                let _ = fs::remove_file(release.join(".profile"));
                println!("so: product cache entry {} does not verify ({e}): a miss, every product built", entry.display());
                Ok(false)
            }
        }
    }

    /// Stores the finished set (its manifest written): the products as the manifest lists them and the side files.
    /// A failure is printed, never a build error.
    pub(crate) fn store(&self, release: &Path, product: &Path, profile: &str) {
        let r = (|| -> Result<usize, String> {
            let text = fs::read_to_string(release.join(crate::import::MANIFEST)).map_err(|e| format!("manifest: {e}"))?;
            let (_, entries) = crate::import::parse_manifest(&text)?;
            let side = side_files(product, profile);
            let side_sums = sha256_files(product, &side)?;
            let mut records = Vec::new();
            let mut files: Vec<(&str, &Path, String)> = Vec::new();
            for e in entries.iter().filter(|e| owned(e.path())) {
                match e {
                    Entry::File { path, sha } => {
                        records.push(format!("product file {sha} {path}"));
                        files.push(("products", release, path.clone()));
                    }
                    Entry::Link { path, target } => records.push(format!("product link {target} {path}")),
                }
            }
            for (p, h) in side.iter().zip(&side_sums) {
                records.push(format!("side file {h} {p}"));
                files.push(("side", product, p.clone()));
            }
            write_entry(&self.dir, &self.key, SET_MAGIC, &records, &files)?;
            evict(&self.dir, KEEP_SETS);
            Ok(entries.len())
        })();
        match r {
            Ok(n) => println!("so: product cache stored {} ({n} products)", self.key),
            Err(e) => println!("so: product cache: could not store {} ({e}); the build is complete", self.key),
        }
    }
}

/// Copies an entry's records into place and checks every copy; the number of products.
fn restore_records(entry: &Path, records: &[String], release: &Path, product: &Path) -> Result<usize, String> {
    let mut want: Vec<Entry> = Vec::new();
    let mut side: Vec<(String, String)> = Vec::new();
    for r in records {
        let f: Vec<&str> = r.splitn(4, ' ').collect();
        match f.as_slice() {
            ["product", "file", sha, path] => want.push(Entry::File { path: (*path).to_string(), sha: (*sha).to_string() }),
            ["product", "link", target, path] => want.push(Entry::Link { path: (*path).to_string(), target: (*target).to_string() }),
            ["side", "file", sha, path] => side.push(((*path).to_string(), (*sha).to_string())),
            _ => return Err(format!("unreadable record {r:?}")),
        }
    }
    let unsafe_path = |p: &str| p.is_empty() || p.starts_with('/') || p.split('/').any(|c| c == ".." || c.is_empty());
    if let Some(p) = want.iter().map(Entry::path).find(|p| !owned(p)) {
        return Err(format!("record path {p:?} is another command's"));
    }
    if let Some(p) = want.iter().map(Entry::path).chain(side.iter().map(|(p, _)| p.as_str())).find(|p| unsafe_path(p)) {
        return Err(format!("record path {p:?} leaves its directory"));
    }
    // The profile marker last, so a set that did not verify has none.
    want.sort_by_key(|e| e.path() == ".profile");
    for e in &want {
        let dst = release.join(e.path());
        match e {
            Entry::File { path, .. } if path != ".profile" => copy_into(&entry.join("files/products").join(path), &dst)?,
            Entry::File { .. } => {}
            Entry::Link { target, .. } => {
                if dst.symlink_metadata().is_ok() {
                    fs::remove_file(&dst).map_err(|e| format!("{}: {e}", dst.display()))?;
                }
                if let Some(d) = dst.parent() {
                    fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
                }
                std::os::unix::fs::symlink(target, &dst).map_err(|e| format!("{}: {e}", dst.display()))?;
            }
        }
    }
    for (p, _) in &side {
        copy_into(&entry.join("files/side").join(p), &product.join(p))?;
    }
    let mut found = crate::import::hash_entries(release, owned_listing(release)?, str::to_string)?;
    let has_marker = want.iter().any(|e| e.path() == ".profile");
    if has_marker {
        // Checked before it is put in place: the copy beside the set.
        let marker = entry.join("files/products/.profile");
        found.push(Entry::File { path: ".profile".into(), sha: file_sha(&marker)? });
        found.sort();
    }
    let mut sorted = want.clone();
    sorted.sort();
    let errs = crate::import::entry_errors(&sorted, &found);
    if !errs.is_empty() {
        return Err(errs.into_iter().take(3).collect::<Vec<_>>().join("; "));
    }
    let side_rel: Vec<String> = side.iter().map(|(p, _)| p.clone()).collect();
    for ((p, want), got) in side.iter().zip(sha256_files(product, &side_rel)?) {
        if *want != got {
            return Err(format!("side file {p}: stored {want}, copied {got}"));
        }
    }
    if has_marker {
        let tmp = crate::so::tmp_name(&release.join(".profile"));
        copy_into(&entry.join("files/products/.profile"), &tmp)?;
        let marker_sha = want.iter().find_map(|e| if let Entry::File { path, sha } = e { (path == ".profile").then(|| sha.clone()) } else { None });
        if Some(file_sha(&tmp)?) != marker_sha {
            let _ = fs::remove_file(&tmp);
            return Err("the profile marker's copy does not match".into());
        }
        crate::so::publish_verbatim(&tmp, &release.join(".profile"))?;
    }
    Ok(want.len())
}

/// The trace cache: `Ok(Some(lines))` when the stored trace for `key` was put into `lists` (its lists and log, each
/// checked by sha256), `Ok(None)` on a miss (the reason printed).
pub(crate) fn trace_restore(cache_root: &Path, key: &str, lists: &Path) -> Result<Option<Vec<String>>, String> {
    let entry = cache_root.join(DIR).join("traces").join(key);
    let records = match read_index(&entry, TRACE_MAGIC, key) {
        Ok(Some(r)) => r,
        Ok(None) => {
            println!("so: trace cache miss {key}: tracing");
            return Ok(None);
        }
        Err(e) => {
            println!("so: trace cache entry {} is not usable ({e}): a miss, tracing", entry.display());
            return Ok(None);
        }
    };
    let (mut files, mut lines) = (Vec::new(), Vec::new());
    for r in &records {
        if let Some(l) = r.strip_prefix("line ") {
            lines.push(l.to_string());
        } else if let Some(rest) = r.strip_prefix("file ") {
            let (sha, name) = rest.split_once(' ').ok_or_else(|| format!("unreadable record {r:?}"))?;
            if name.contains('/') || name.is_empty() || name == ".." {
                return Err(format!("trace cache entry {}: record path {name:?} leaves its directory", entry.display()));
            }
            files.push((name.to_string(), sha.to_string()));
        } else {
            println!("so: trace cache entry {} has an unreadable record: a miss, tracing", entry.display());
            return Ok(None);
        }
    }
    for (name, _) in &files {
        copy_into(&entry.join("files/lists").join(name), &lists.join(name))?;
    }
    let names: Vec<String> = files.iter().map(|(n, _)| n.clone()).collect();
    for ((name, want), got) in files.iter().zip(sha256_files(lists, &names)?) {
        if *want != got {
            // The lists are rewritten by the full trace that follows.
            println!("so: trace cache entry {} does not verify ({name}: stored {want}, copied {got}): a miss, tracing", entry.display());
            return Ok(None);
        }
    }
    stamp(&entry, &entry.parent().map(Path::to_path_buf).unwrap_or_default());
    println!("so: trace cache hit {key}: the archive traced, the tree but its Rust sources, the tools, the xtask binary, the environment and the profile are those of a full trace; {} lists restored and checked by sha256 ({OFF_ENV}=0 traces)", files.len());
    Ok(Some(lines))
}

/// Stores a finished trace: `names` in `lists` and the lines it printed. A failure is printed, never a build error.
pub(crate) fn trace_store(cache_root: &Path, key: &str, lists: &Path, names: &[String], lines: &[String]) {
    let kind = cache_root.join(DIR).join("traces");
    let r = (|| -> Result<(), String> {
        let present: Vec<String> = names.iter().filter(|n| lists.join(n).is_file()).cloned().collect();
        let mut records: Vec<String> = present.iter().zip(sha256_files(lists, &present)?).map(|(n, h)| format!("file {h} {n}")).collect();
        records.extend(lines.iter().map(|l| format!("line {l}")));
        let files: Vec<(&str, &Path, String)> = present.iter().map(|n| ("lists", lists, n.clone())).collect();
        write_entry(&kind, key, TRACE_MAGIC, &records, &files)?;
        evict(&kind, KEEP_TRACES);
        Ok(())
    })();
    match r {
        Ok(()) => println!("so: trace cache stored {key}"),
        Err(e) => println!("so: trace cache: could not store {key} ({e}); the trace is complete"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree, its products and stand-ins for the xtask binary and a tool, in a fresh directory.
    struct Fx {
        root: PathBuf,
        release: PathBuf,
        product: PathBuf,
        exe: PathBuf,
        tool: PathBuf,
        outside: PathBuf,
    }

    fn put(p: &Path, text: &str) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn fx(tag: &str) -> Fx {
        let d = std::env::temp_dir().join(format!("xtask-so-cache-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let root = d.join("tree");
        put(&root.join("spec/abi/versions.tsv"), "name\tversion\n");
        put(&root.join("crates/c/src/lib.rs"), "pub fn f() {}\n");
        put(&root.join("crates/c/data.tsv"), "row\n");
        put(&root.join("target/junk"), "build output\n");
        let release = root.join("target/niji");
        put(&release.join("include/stdio.h"), "/* staged */\n");
        let product = root.join("target/product");
        put(&product.join("fast/hot-text.pre-group.a"), "!<arch>\n1\n");
        put(&d.join("bin/xtask"), "xtask 1\n");
        put(&d.join("bin/ld"), "ld 1\n");
        put(&d.join("outside/h.h"), "#define H 1\n");
        // A trace probe including a header outside the tree (as the system headers are).
        put(&root.join(PROBES_DIR).join("p_true.c"), &format!("#include \"{}\"\nint v = H;\n", d.join("outside/h.h").display()));
        Fx { exe: d.join("bin/xtask"), tool: d.join("bin/ld"), outside: d.join("outside/h.h"), root, release, product }
    }

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let v: Vec<(String, String)> = pairs.iter().map(|(k, x)| (k.to_string(), x.to_string())).collect();
        move |k: &str| v.iter().find(|(n, _)| n == k).map(|(_, x)| x.clone())
    }

    fn ctx<'a>(f: &'a Fx, env: &'a dyn Fn(&str) -> Option<String>, names: &[&str], flags: &str) -> Ctx<'a> {
        Ctx { root: &f.root, exe: f.exe.clone(), env, env_names: names.iter().map(|s| s.to_string()).collect(), tools: vec![("ld".into(), Some(f.tool.clone()))], versions: vec!["rustc 1.0".into()], flags: flags.into(), cc: "cc".into() }
    }

    fn set_key_of(f: &Fx, pairs: &[(&str, &str)], flags: &str) -> String {
        let env = env_of(pairs);
        let names: Vec<&str> = pairs.iter().map(|(k, _)| *k).collect();
        set_key(&ctx(f, &env, &names, flags), &f.release).unwrap()
    }

    /// Finishes the set (products, marker, manifest) and stores it under the key of `pairs` and `flags`.
    fn build_and_store(f: &Fx, pairs: &[(&str, &str)], flags: &str) -> Sets {
        let key = set_key_of(f, pairs, flags);
        put(&f.release.join("libc.so.6"), "\x7fELF libc\n");
        put(&f.release.join("charmaps/UTF-8.gz"), "gz\n");
        put(&f.release.join(".profile"), "fast\n");
        put(&f.product.join("fast/hot-text.list"), "f1\nf2\n");
        put(&f.product.join("placeholders/libnsl.so.1.bodies.c"), "int x;\n");
        let h = crate::import::Header { tag: "src-1".into(), head: "unknown".into(), toolchain: vec![], profile: flags.into() };
        crate::import::write_manifest(&f.release, &h).unwrap();
        let s = Sets { dir: f.root.join(DIR).join("sets"), key };
        s.store(&f.release, &f.product, "fast");
        s
    }

    /// The products as a fresh build starts them: the staged headers only.
    fn clear(f: &Fx) {
        crate::so::clear_products(&f.release).unwrap();
        put(&f.release.join("include/stdio.h"), "/* staged */\n");
    }

    /// The claim of every input class: after `change`, the build's key is another, so no stored set serves it
    /// (the products are built again), while the unchanged inputs are served.
    fn assert_rebuilds(tag: &str, change: impl FnOnce(&Fx) -> (Vec<(&'static str, &'static str)>, &'static str)) {
        let f = fx(tag);
        let base = [("NIJI_LAYOUT_TRACE", "ungrouped")];
        let stored = build_and_store(&f, &base, "fast ungrouped");
        clear(&f);
        assert!(stored.restore(&f.release, &f.product).unwrap(), "{tag}: the unchanged inputs are served");
        clear(&f);
        let (pairs, flags) = change(&f);
        let key = set_key_of(&f, &pairs, flags);
        assert_ne!(key, stored.key, "{tag}: the change must change the key");
        let s = Sets { dir: f.root.join(DIR).join("sets"), key };
        assert!(!s.restore(&f.release, &f.product).unwrap(), "{tag}: a changed input must rebuild the products");
        assert!(!f.release.join(".profile").exists());
    }

    #[test]
    fn a_tree_file_change_rebuilds_the_products() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): a table, a Rust source and a C source of the tree.
        assert_rebuilds("tree-table", |f| {
            put(&f.root.join("spec/abi/versions.tsv"), "name\tversion\nx\tGLIBC_2.2.5\n");
            (vec![("NIJI_LAYOUT_TRACE", "ungrouped")], "fast ungrouped")
        });
        assert_rebuilds("tree-rust", |f| {
            put(&f.root.join("crates/c/src/lib.rs"), "pub fn f() { g() }\n");
            (vec![("NIJI_LAYOUT_TRACE", "ungrouped")], "fast ungrouped")
        });
        assert_rebuilds("tree-new-file", |f| {
            put(&f.root.join("csys/src/new.c"), "int y;\n");
            (vec![("NIJI_LAYOUT_TRACE", "ungrouped")], "fast ungrouped")
        });
    }

    #[test]
    fn a_change_of_the_xtask_binary_rebuilds_the_products() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): the steps, their constants and flags live in the binary.
        assert_rebuilds("checker", |f| {
            put(&f.exe, "xtask 2\n");
            (vec![("NIJI_LAYOUT_TRACE", "ungrouped")], "fast ungrouped")
        });
    }

    #[test]
    fn a_change_of_a_tool_rebuilds_the_products() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): a tool of the same version string with other bytes.
        assert_rebuilds("tool", |f| {
            put(&f.tool, "ld 1 patched\n");
            (vec![("NIJI_LAYOUT_TRACE", "ungrouped")], "fast ungrouped")
        });
    }

    #[test]
    fn a_change_of_an_output_changing_variable_rebuilds_the_products() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): the layout switch, the full profile, the compiler flags
        // and the archiver.
        for (tag, extra) in [("env-trace", ("NIJI_LAYOUT_TRACE", "1")), ("env-full", ("NIJI_FULL_PROFILE", "1")), ("env-rustflags", ("RUSTFLAGS", "-Copt-level=1")), ("env-ar", ("AR", "llvm-ar"))] {
            assert_rebuilds(tag, |_| {
                let mut v = vec![("NIJI_LAYOUT_TRACE", "ungrouped")];
                if extra.0 == "NIJI_LAYOUT_TRACE" {
                    v = vec![extra];
                } else {
                    v.push(extra);
                }
                (v, "fast ungrouped")
            });
        }
    }

    #[test]
    fn a_change_of_the_profile_or_layout_rebuilds_the_products() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): `<profile> <layout>`.
        assert_rebuilds("flags-profile", |_| (vec![("NIJI_LAYOUT_TRACE", "ungrouped")], "release ungrouped"));
        assert_rebuilds("flags-layout", |_| (vec![("NIJI_LAYOUT_TRACE", "ungrouped")], "fast traced"));
    }

    #[test]
    fn a_change_of_the_staged_headers_rebuilds_the_products() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): the staged headers copy the host's kernel UAPI headers,
        // which are not in the tree.
        assert_rebuilds("staged", |f| {
            put(&f.release.join("uapi/linux/types.h"), "/* host */\n");
            (vec![("NIJI_LAYOUT_TRACE", "ungrouped")], "fast ungrouped")
        });
    }

    #[test]
    fn a_change_of_a_system_header_a_probe_includes_rebuilds_the_products() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): the trace compiles its probes with the system headers.
        let f = fx("probe");
        let a = probe_lines(&f.root, "cc").unwrap();
        put(&f.outside, "#define H 2\n");
        let b = probe_lines(&f.root, "cc").unwrap();
        assert_eq!(a.len(), 1);
        assert_ne!(a, b, "a header outside the tree that a probe includes is a key input");
        assert_rebuilds("probe-set", |f| {
            put(&f.outside, "#define H 3\n");
            (vec![("NIJI_LAYOUT_TRACE", "ungrouped")], "fast ungrouped")
        });
    }

    #[test]
    fn a_modification_time_alone_keeps_the_key() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): contents only, never modification times, and no
        // build output or bytecode cache.
        let f = fx("mtime");
        let a = set_key_of(&f, &[], "fast traced");
        let file = fs::File::options().write(true).open(f.root.join("spec/abi/versions.tsv")).unwrap();
        file.set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000)).unwrap();
        put(&f.root.join("target/junk"), "other build output\n");
        // A test run's Python bytecode beside a tool of the tree.
        put(&f.root.join("spec/tools/__pycache__/probe.cpython-312.pyc"), "bytecode\n");
        assert_eq!(a, set_key_of(&f, &[], "fast traced"));
        put(&f.root.join("spec/tools/probe.py"), "print(1)\n");
        assert_ne!(a, set_key_of(&f, &[], "fast traced"), "the Python source itself is a key input");
    }

    #[test]
    fn a_restored_set_is_the_stored_one_and_a_damaged_entry_is_a_miss() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): every copy is checked by sha256, the marker last.
        let f = fx("restore");
        let s = build_and_store(&f, &[], "fast traced");
        let before = fs::read_to_string(f.release.join(crate::import::MANIFEST)).unwrap();
        clear(&f);
        fs::remove_file(f.product.join("fast/hot-text.list")).unwrap();
        assert!(s.restore(&f.release, &f.product).unwrap());
        assert_eq!(fs::read(f.release.join("libc.so.6")).unwrap(), b"\x7fELF libc\n");
        assert_eq!(fs::read(f.product.join("fast/hot-text.list")).unwrap(), b"f1\nf2\n");
        assert_eq!(fs::read(f.release.join(".profile")).unwrap(), b"fast\n");
        crate::import::write_manifest(&f.release, &crate::import::Header { tag: "src-1".into(), head: "unknown".into(), toolchain: vec![], profile: "fast traced".into() }).unwrap();
        assert_eq!(before, fs::read_to_string(f.release.join(crate::import::MANIFEST)).unwrap());
        // A stored product whose bytes changed.
        put(&f.root.join(DIR).join("sets").join(&s.key).join("files/products/libc.so.6"), "\x7fELF other\n");
        clear(&f);
        assert!(!s.restore(&f.release, &f.product).unwrap());
        assert!(!f.release.join(".profile").exists(), "a set that does not verify has no profile marker");
        // An index whose text no longer matches its sum.
        let s = build_and_store(&f, &[], "fast traced");
        let idx = f.root.join(DIR).join("sets").join(&s.key).join("index");
        let t = fs::read_to_string(&idx).unwrap().replace("product file", "product  file");
        fs::write(&idx, t).unwrap();
        clear(&f);
        assert!(!s.restore(&f.release, &f.product).unwrap());
    }

    #[test]
    fn the_trace_key_follows_the_archive_and_not_the_rust_sources() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): the trace links the pre-group archive, a cargo output.
        let f = fx("trace");
        let env = env_of(&[]);
        let archive = f.product.join("fast/hot-text.pre-group.a");
        let k = |f: &Fx| trace_key(&ctx(f, &env, &[], "fast"), &archive).unwrap();
        let a = k(&f);
        put(&f.root.join("crates/c/src/lib.rs"), "pub fn f() { /* a test-only change */ }\n");
        assert_eq!(a, k(&f), "a Rust source reaches the trace only through the archive");
        put(&archive, "!<arch>\n2\n");
        let b = k(&f);
        assert_ne!(a, b, "another archive is another trace");
        put(&f.root.join("tools/hottext/regen.sh"), "#!/bin/sh\n");
        assert_ne!(b, k(&f), "a trace script is a trace input");
        put(&f.exe, "xtask 3\n");
        assert_ne!(b, k(&f));
        let lists = f.product.join("fast");
        put(&lists.join("hot-text.list"), "f1\n");
        trace_store(&f.root, &b, &lists, &["hot-text.list".to_string(), "missing.keys".to_string()], &["so: trace-at-build: hot-text: 1".to_string()]);
        put(&lists.join("hot-text.list"), "stale\n");
        assert_eq!(trace_restore(&f.root, &b, &lists).unwrap(), Some(vec!["so: trace-at-build: hot-text: 1".to_string()]));
        assert_eq!(fs::read(lists.join("hot-text.list")).unwrap(), b"f1\n");
        assert_eq!(trace_restore(&f.root, &a, &lists).unwrap(), None);
        put(&f.root.join(DIR).join("traces").join(&b).join("files/lists/hot-text.list"), "f9\n");
        assert_eq!(trace_restore(&f.root, &b, &lists).unwrap(), None, "a stored list that changed is a miss");
    }

    #[test]
    fn the_train_and_the_opt_out_never_use_the_cache() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): as the layout verdict cache.
        assert_eq!(mode(&env_of(&[("NIJI_TRAIN", "1")])), Mode::Train);
        assert_eq!(mode(&env_of(&[(OFF_ENV, "0")])), Mode::Off);
        assert_eq!(mode(&env_of(&[("NIJI_TRAIN", "0")])), Mode::Use);
        assert_eq!(mode(&env_of(&[])), Mode::Use);
    }

    #[test]
    fn eviction_keeps_the_entries_used_last() {
        // no spec clause: tooling (XTASK-SO-PRODUCT-CACHE): a use count orders them, never a time.
        let f = fx("evict");
        let kind = f.root.join(DIR).join("traces");
        for k in ["k1", "k2", "k3"] {
            write_entry(&kind, k, TRACE_MAGIC, &[], &[]).unwrap();
        }
        stamp(&kind.join("k1"), &kind);
        evict(&kind, 2);
        assert!(kind.join("k1").exists() && kind.join("k3").exists() && !kind.join("k2").exists());
    }
}
