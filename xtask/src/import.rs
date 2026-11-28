//! The products manifest and the products import (CI-PRODUCTS-ONCE, D-27.82, a cache trust boundary).
//!
//! Every finished product build (`so::run_so_as`) writes [`MANIFEST`] into `target/niji`, last: the build tag of the
//! sources (`so::build_tag`), the commit ([`commit_of`]), the toolchain ([`toolchain`]: the compilers, linkers and
//! tracer and the environment variables that change their output), the profile with its layout mode, and every
//! product: the sha256 of each regular file and the target of each symbolic link below `target/niji`, except
//! `bench/` (the bench programs and the tables of earlier runs, built and written by `cargo xtask bench`).
//!
//! `NIJI_IMPORT_PRODUCTS=<dir>` ([`IMPORT_ENV`]; `<dir>` a copy of another tree's `target/niji`, its manifest
//! included) makes a product build take those products instead of building them, ONLY when every field of the
//! manifest equals this tree's own ([`header_errors`]) and the copies made into this tree hash to the manifest's
//! values, with no file of `<dir>` unlisted and none missing ([`entry_errors`]). A wrong import would make a bench
//! measure code that is not this tree's, so any mismatch prints `so: import refused: <field>: ...` and the build
//! runs here as it would without the variable; the imported files are never used then. The copies are verified
//! after they are made (a source that changes between the check and the copy is caught) and moved into place only
//! when all of them match; the profile marker and the manifest come last, so an interrupted import leaves a set that
//! no reader takes as finished.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::sha256::sha256_files;

/// The manifest's name in a products directory (`target/niji`).
pub(crate) const MANIFEST: &str = ".products-manifest";
/// The variable naming the products directory to import.
pub(crate) const IMPORT_ENV: &str = "NIJI_IMPORT_PRODUCTS";
/// The products directory's part a product build does not write (`cargo xtask bench` builds and writes it).
const SKIPPED: &str = "bench";
/// The profile marker (`so::run_so_as`), moved into place after every other imported file.
const PROFILE_MARKER: &str = ".profile";
/// The environment variables that change what the product build makes from the same sources and tools.
const BUILD_ENV: &[&str] = &["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS", "CC", "CFLAGS", "LDFLAGS", "RUSTC", "RUSTC_WRAPPER"];

/// What a products set was built from: the fields an import compares, in this order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    pub tag: String,
    pub head: String,
    /// One line per tool and variable ([`toolchain`]).
    pub toolchain: Vec<String>,
    /// `<profile> <layout>`, e.g. `release traced`.
    pub profile: String,
}

/// One product below the products directory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Entry {
    /// A regular file: its path (relative, `/`-separated) and sha256.
    File { path: String, sha: String },
    /// A symbolic link: its path and target.
    Link { path: String, target: String },
}

impl Entry {
    pub(crate) fn path(&self) -> &str {
        match self {
            Entry::File { path, .. } | Entry::Link { path, .. } => path,
        }
    }
}

/// The manifest's text.
pub(crate) fn manifest_text(h: &Header, entries: &[Entry]) -> String {
    let mut s = format!("# products manifest (CI-PRODUCTS-ONCE): an import of these products checks every line\ntag\t{}\nhead\t{}\nprofile\t{}\n", h.tag, h.head, h.profile);
    for t in &h.toolchain {
        s.push_str(&format!("tool\t{t}\n"));
    }
    for e in entries {
        match e {
            Entry::File { path, sha } => s.push_str(&format!("file\t{sha}\t{path}\n")),
            Entry::Link { path, target } => s.push_str(&format!("link\t{target}\t{path}\n")),
        }
    }
    s
}

/// Parses [`manifest_text`]'s output; a line of another shape, or a header field missing or given twice, is an error.
pub(crate) fn parse_manifest(text: &str) -> Result<(Header, Vec<Entry>), String> {
    let (mut tag, mut head, mut profile) = (None, None, None);
    let (mut toolchain, mut entries) = (Vec::new(), Vec::new());
    for l in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let set = |slot: &mut Option<String>, v: &str, k: &str| match slot {
            Some(_) => Err(format!("{k} given twice")),
            None => {
                *slot = Some(v.to_string());
                Ok(())
            }
        };
        match l.split('\t').collect::<Vec<_>>().as_slice() {
            ["tag", v] => set(&mut tag, v, "tag")?,
            ["head", v] => set(&mut head, v, "head")?,
            ["profile", v] => set(&mut profile, v, "profile")?,
            ["tool", v] => toolchain.push((*v).to_string()),
            ["file", sha, path] if sha.len() == 64 => entries.push(Entry::File { path: (*path).to_string(), sha: (*sha).to_string() }),
            ["link", target, path] => entries.push(Entry::Link { path: (*path).to_string(), target: (*target).to_string() }),
            _ => return Err(format!("unreadable line {l:?}")),
        }
    }
    let need = |v: Option<String>, k: &str| v.ok_or_else(|| format!("no {k} line"));
    let h = Header { tag: need(tag, "tag")?, head: need(head, "head")?, toolchain, profile: need(profile, "profile")? };
    Ok((h, entries))
}

/// Why products built as `got` describes are not this tree's (`want`): one message per differing field, none when
/// they match. A head that is unknown or has uncommitted changes never matches, not even itself.
pub(crate) fn header_errors(want: &Header, got: &Header) -> Vec<String> {
    let mut errs = Vec::new();
    if want.tag != got.tag {
        errs.push(format!("tag: this tree's {} vs the products' {}", want.tag, got.tag));
    }
    let clean = |h: &str| h.len() == 40 && h.bytes().all(|b| b.is_ascii_hexdigit());
    if !clean(&want.head) || !clean(&got.head) {
        errs.push(format!("head: this tree's {} and the products' {} must both be a commit without uncommitted changes", want.head, got.head));
    } else if want.head != got.head {
        errs.push(format!("head: this tree's {} vs the products' {}", want.head, got.head));
    }
    if want.toolchain != got.toolchain {
        let n = want.toolchain.len().max(got.toolchain.len());
        let at = (0..n).find(|&i| want.toolchain.get(i) != got.toolchain.get(i)).unwrap_or(0);
        let show = |v: &[String]| v.get(at).cloned().unwrap_or_else(|| "(none)".into());
        errs.push(format!("toolchain: this host's {:?} vs the products' {:?}", show(&want.toolchain), show(&got.toolchain)));
    }
    if want.profile != got.profile {
        errs.push(format!("profile: this build's {} vs the products' {}", want.profile, got.profile));
    }
    errs
}

/// Why the products found (`found`, the entries of the copies with their hashes) are not the ones `listed`: a
/// listed product missing or different, a found one unlisted. Both sorted by path.
pub(crate) fn entry_errors(listed: &[Entry], found: &[Entry]) -> Vec<String> {
    let mut errs = Vec::new();
    for e in listed {
        match found.iter().find(|f| f.path() == e.path()) {
            None => errs.push(format!("product {}: listed, not in the products directory", e.path())),
            Some(f) if f != e => errs.push(format!("product {}: listed as {e:?}, found {f:?}", e.path())),
            Some(_) => {}
        }
    }
    for f in found.iter().filter(|f| !listed.iter().any(|e| e.path() == f.path())) {
        errs.push(format!("product {}: in the products directory, not listed", f.path()));
    }
    errs
}

/// The commit `root` was built from: git when the tree has it, else the `head=` line of the `.synced` file a role
/// directory carries (it has no .git), else `unknown`. `+dirty` marks uncommitted changes.
pub(crate) fn commit_of(root: &Path) -> String {
    let git = |args: &[&str]| Command::new("git").arg("-C").arg(root).args(args).stderr(Stdio::null()).output().ok().filter(|o| o.status.success());
    if let Some(o) = git(&["rev-parse", "HEAD"]) {
        let head = String::from_utf8_lossy(&o.stdout).trim().to_string();
        let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|o| !o.stdout.is_empty());
        return if dirty { format!("{head}+dirty") } else { head };
    }
    let synced = fs::read_to_string(root.join(".synced")).unwrap_or_default();
    let val = |k: &str| synced.lines().find_map(|l| l.strip_prefix(k).map(|v| v.trim().to_string())).filter(|v| !v.is_empty());
    match (val("head="), val("dirty=")) {
        (Some(h), Some(d)) if d != "clean" => format!("{h}+dirty({d})"),
        (Some(h), _) => h,
        _ => "unknown".into(),
    }
}

/// The tools a product build runs and the [`BUILD_ENV`] variables, one line each: `rustc -vV` (every line: the
/// version, commit and LLVM), `cargo -V`, and the first line of `--version` of the C compiler, `ld`, `as` and `gdb`
/// (the tracer). A tool that does not run is a line saying so, which matches only another host without it.
pub(crate) fn toolchain(root: &Path) -> Vec<String> {
    let out = |cmd: &str, args: &[&str]| -> String {
        match Command::new(cmd).args(args).current_dir(root).stdin(Stdio::null()).stderr(Stdio::null()).output() {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
            _ => format!("{cmd}: does not run"),
        }
    };
    let first = |s: String| s.lines().next().unwrap_or("").to_string();
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let mut v: Vec<String> = out("rustc", &["-vV"]).lines().map(|l| format!("rustc: {l}")).collect();
    v.push(first(out("cargo", &["-V"])));
    v.push(format!("cc: {}", first(out(&cc, &["--version"]))));
    for t in ["ld", "as", "gdb"] {
        v.push(format!("{t}: {}", first(out(t, &["--version"]))));
    }
    for k in BUILD_ENV {
        v.push(format!("env {k}={}", std::env::var(k).unwrap_or_default()));
    }
    v
}

/// The products below `dir` (not `bench/`, not the manifest, not this module's temporary copies), sorted by path,
/// files without their hash (filled by [`hash_entries`]).
pub(crate) fn listing(dir: &Path) -> Result<Vec<Entry>, String> {
    let mut out = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        let rd = fs::read_dir(dir.join(&rel)).map_err(|e| format!("{}: {e}", dir.join(&rel).display()))?;
        for e in rd {
            let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
            let name = e.file_name().to_string_lossy().into_owned();
            let path = if rel.as_os_str().is_empty() { name.clone() } else { format!("{}/{name}", rel.to_string_lossy()) };
            if path == SKIPPED || path == MANIFEST || is_import_tmp(&name) {
                continue;
            }
            let ft = e.file_type().map_err(|e| format!("{path}: {e}"))?;
            if ft.is_symlink() {
                let target = fs::read_link(e.path()).map_err(|e| format!("{path}: {e}"))?;
                out.push(Entry::Link { path, target: target.to_string_lossy().into_owned() });
            } else if ft.is_dir() {
                stack.push(PathBuf::from(&path));
            } else if ft.is_file() {
                out.push(Entry::File { path, sha: String::new() });
            } else {
                return Err(format!("{path}: neither a file, a directory nor a symbolic link"));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// `entries` with each file's sha256, read from `dir` (`rename` maps a path to the file holding its bytes).
pub(crate) fn hash_entries(dir: &Path, entries: Vec<Entry>, rename: impl Fn(&str) -> String) -> Result<Vec<Entry>, String> {
    let files: Vec<String> = entries.iter().filter_map(|e| if let Entry::File { path, .. } = e { Some(rename(path)) } else { None }).collect();
    let mut hashes = sha256_files(dir, &files)?.into_iter();
    Ok(entries
        .into_iter()
        .map(|e| match e {
            Entry::File { path, .. } => Entry::File { path, sha: hashes.next().unwrap_or_default() },
            link => link,
        })
        .collect())
}

/// This tree's header for a build of `profile_layout` (`<profile> <layout>`).
pub(crate) fn header(root: &Path, tag: String, profile_layout: &str) -> Header {
    Header { tag, head: commit_of(root), toolchain: toolchain(root), profile: profile_layout.to_string() }
}

/// Writes the manifest of the finished products in `release` (the last step of a product build).
pub(crate) fn write_manifest(release: &Path, h: &Header) -> Result<(), String> {
    let entries = hash_entries(release, listing(release)?, str::to_string)?;
    let dst = release.join(MANIFEST);
    let tmp = crate::so::tmp_name(&dst);
    fs::write(&tmp, manifest_text(h, &entries)).map_err(|e| format!("{}: {e}", tmp.display()))?;
    crate::so::publish(&tmp, &dst)
}

/// Removes a products set's manifest (a build that starts makes the set unfinished until it writes a new one).
pub(crate) fn remove_manifest(release: &Path) {
    let _ = fs::remove_file(release.join(MANIFEST));
}

/// The temporary name of an imported file beside its final place.
fn import_tmp(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((d, n)) => format!("{d}/.{n}.niji-import"),
        None => format!(".{path}.niji-import"),
    }
}

fn is_import_tmp(name: &str) -> bool {
    name.starts_with('.') && name.ends_with(".niji-import")
}

/// Imports the products of `src` into `release` when they are this tree's (`want`): `Ok(Ok(n))` with the number of
/// products moved in, `Ok(Err(reasons))` when they are not (nothing of `src` is left in `release` then), `Err` when
/// `release` cannot be written.
pub(crate) fn import_into(src: &Path, release: &Path, want: &Header) -> Result<Result<usize, Vec<String>>, String> {
    let text = match fs::read_to_string(src.join(MANIFEST)) {
        Ok(t) => t,
        Err(e) => return Ok(Err(vec![format!("manifest: {}: {e}", src.join(MANIFEST).display())])),
    };
    let (got, listed) = match parse_manifest(&text) {
        Ok(m) => m,
        Err(e) => return Ok(Err(vec![format!("manifest: {}: {e}", src.join(MANIFEST).display())])),
    };
    let errs = header_errors(want, &got);
    if !errs.is_empty() {
        return Ok(Err(errs));
    }
    let found = match listing(src) {
        Ok(f) => f,
        Err(e) => return Ok(Err(vec![e])),
    };
    // Copy first, hash the copies: what is checked is what gets moved in.
    let mut tmps = Vec::new();
    let mut copy = || -> Result<(), String> {
        for e in &found {
            if let Entry::File { path, .. } = e {
                let tmp = release.join(import_tmp(path));
                if let Some(d) = tmp.parent() {
                    fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
                }
                tmps.push(tmp.clone());
                fs::copy(src.join(path), &tmp).map_err(|e| format!("copy {path}: {e}"))?;
            }
        }
        Ok(())
    };
    let copied = copy();
    let checked = copied.and_then(|()| hash_entries(release, found, import_tmp)).map(|f| entry_errors(&listed, &f));
    let errs = match checked {
        Ok(errs) => errs,
        Err(e) => vec![e],
    };
    if !errs.is_empty() {
        for t in &tmps {
            let _ = fs::remove_file(t);
        }
        return Ok(Err(errs));
    }
    // Every copy matches: move them in, the profile marker last; then the links; then the manifest.
    let mut order: Vec<&Entry> = listed.iter().filter(|e| e.path() != PROFILE_MARKER).collect();
    order.extend(listed.iter().filter(|e| e.path() == PROFILE_MARKER));
    for e in &order {
        let dst = release.join(e.path());
        match e {
            Entry::File { path, .. } => crate::so::publish_verbatim(&release.join(import_tmp(path)), &dst)?,
            Entry::Link { target, .. } => {
                if dst.symlink_metadata().is_ok() {
                    fs::remove_file(&dst).map_err(|e| format!("{}: {e}", dst.display()))?;
                }
                std::os::unix::fs::symlink(target, &dst).map_err(|e| format!("{}: {e}", dst.display()))?;
            }
        }
    }
    let tmp = crate::so::tmp_name(&release.join(MANIFEST));
    fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    crate::so::publish(&tmp, &release.join(MANIFEST))?;
    Ok(Ok(listed.len()))
}

/// The product build's import step: with [`IMPORT_ENV`] set, imports its products when they are this tree's and
/// returns true; prints why not and returns false otherwise (the caller builds as without the variable).
pub(crate) fn try_import(release: &Path, want: &Header) -> Result<bool, String> {
    import_from(std::env::var_os(IMPORT_ENV).filter(|v| !v.is_empty()).map(PathBuf::from), release, want)
}

/// [`try_import`] of `src` (`None`: the variable unset): true only when the products were moved in.
fn import_from(src: Option<PathBuf>, release: &Path, want: &Header) -> Result<bool, String> {
    let Some(src) = src else { return Ok(false) };
    match import_into(&src, release, want)? {
        Ok(n) => {
            println!("so: imported {n} products from {} (tag {}, head {}, profile {}: every field and checksum matched)", src.display(), want.tag, want.head, want.profile);
            Ok(true)
        }
        Err(reasons) => {
            for r in &reasons {
                println!("so: import refused: {r}");
            }
            println!("so: import of {} refused ({} reasons above); building here", src.display(), reasons.len());
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head() -> Header {
        Header { tag: "src-0123456789abcdef".into(), head: "a".repeat(40), toolchain: vec!["rustc: 1.0".into(), "gdb: 15".into()], profile: "release traced".into() }
    }

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("xtask-import-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// A products directory with a file, a nested file, a link, the profile marker, a bench/ part and a manifest.
    fn products(tag: &str, h: &Header) -> PathBuf {
        let d = scratch(tag);
        fs::write(d.join("libc.a"), "archive").unwrap();
        fs::create_dir_all(d.join("include/sys")).unwrap();
        fs::write(d.join("include/sys/x.h"), "header").unwrap();
        std::os::unix::fs::symlink("libc.a", d.join("libg.a")).unwrap();
        fs::write(d.join(PROFILE_MARKER), "release\n").unwrap();
        fs::create_dir_all(d.join("bench")).unwrap();
        fs::write(d.join("bench/counters.tsv"), "old run").unwrap();
        write_manifest(&d, h).unwrap();
        d
    }

    // no spec clause (tooling: the products import, CI-PRODUCTS-ONCE)
    #[test]
    fn manifest_round_trip() {
        let e = vec![Entry::File { path: "a".into(), sha: "f".repeat(64) }, Entry::Link { path: "b".into(), target: "a".into() }];
        assert_eq!(parse_manifest(&manifest_text(&head(), &e)).unwrap(), (head(), e));
        assert!(parse_manifest("tag\tx\ntag\ty\nhead\th\nprofile\tp\n").is_err());
        assert!(parse_manifest("head\th\nprofile\tp\n").is_err());
        assert!(parse_manifest("tag\tx\nhead\th\nprofile\tp\nfile\tshort\tp\n").is_err());
    }

    // no spec clause (tooling: each header field refuses on its own)
    #[test]
    fn header_fields_each_refuse() {
        assert!(header_errors(&head(), &head()).is_empty());
        let one = |f: &dyn Fn(&mut Header)| {
            let mut g = head();
            f(&mut g);
            header_errors(&head(), &g)
        };
        let tag = one(&|g| g.tag = "src-ffffffffffffffff".into());
        assert!(tag.len() == 1 && tag[0].starts_with("tag:"), "{tag:?}");
        let other = one(&|g| g.head = "b".repeat(40));
        assert!(other.len() == 1 && other[0].starts_with("head:"), "{other:?}");
        let tool = one(&|g| g.toolchain[1] = "gdb: 16".into());
        assert!(tool.len() == 1 && tool[0].starts_with("toolchain:") && tool[0].contains("gdb: 16"), "{tool:?}");
        let fewer = one(&|g| {
            g.toolchain.pop();
        });
        assert!(fewer.len() == 1 && fewer[0].starts_with("toolchain:"), "{fewer:?}");
        let prof = one(&|g| g.profile = "release ungrouped".into());
        assert!(prof.len() == 1 && prof[0].starts_with("profile:"), "{prof:?}");
    }

    // no spec clause (tooling: products of uncommitted sources are never imported, even into the same head)
    #[test]
    fn dirty_or_unknown_head_refuses() {
        for bad in [format!("{}+dirty", "a".repeat(40)), "unknown".into(), "a".repeat(39)] {
            let mut w = head();
            w.head = bad.clone();
            let errs = header_errors(&w, &w);
            assert!(errs.len() == 1 && errs[0].starts_with("head:"), "{bad}: {errs:?}");
        }
    }

    // no spec clause (tooling: a missing, changed or unlisted product refuses)
    #[test]
    fn entries_must_match_exactly() {
        let f = |p: &str, s: char| Entry::File { path: p.into(), sha: s.to_string().repeat(64) };
        let listed = vec![f("a", '1'), Entry::Link { path: "l".into(), target: "a".into() }];
        assert!(entry_errors(&listed, &listed).is_empty());
        let changed = entry_errors(&listed, &[f("a", '2'), listed[1].clone()]);
        assert!(changed.len() == 1 && changed[0].contains("found"), "{changed:?}");
        let relinked = entry_errors(&listed, &[f("a", '1'), Entry::Link { path: "l".into(), target: "b".into() }]);
        assert_eq!(relinked.len(), 1, "{relinked:?}");
        let missing = entry_errors(&listed, &listed[..1]);
        assert!(missing.len() == 1 && missing[0].contains("not in the products"), "{missing:?}");
        let extra = entry_errors(&listed, &[listed[0].clone(), listed[1].clone(), f("z", '3')]);
        assert!(extra.len() == 1 && extra[0].contains("not listed"), "{extra:?}");
    }

    // no spec clause (tooling: a matching import moves every product in, bench/ untouched, the manifest last)
    #[test]
    fn import_copies_matching_products() {
        let src = products("src-ok", &head());
        let dst = scratch("dst-ok");
        fs::create_dir_all(dst.join("bench")).unwrap();
        fs::write(dst.join("bench/counters.tsv"), "mine").unwrap();
        let n = import_into(&src, &dst, &head()).unwrap().unwrap();
        assert_eq!(n, 4);
        assert_eq!(fs::read_to_string(dst.join("include/sys/x.h")).unwrap(), "header");
        assert_eq!(fs::read_link(dst.join("libg.a")).unwrap(), Path::new("libc.a"));
        assert_eq!(fs::read_to_string(dst.join("bench/counters.tsv")).unwrap(), "mine");
        assert_eq!(fs::read(dst.join(MANIFEST)).unwrap(), fs::read(src.join(MANIFEST)).unwrap());
        assert!(listing(&dst).unwrap().iter().all(|e| !is_import_tmp(e.path())));
        let _ = (fs::remove_dir_all(&src), fs::remove_dir_all(&dst));
    }

    // no spec clause (tooling: a product changed after its manifest was written refuses, and nothing is moved in)
    #[test]
    fn import_refuses_changed_or_unlisted_product() {
        let src = products("src-bad", &head());
        let dst = scratch("dst-bad");
        fs::write(src.join("libc.a"), "other archive").unwrap();
        let errs = import_into(&src, &dst, &head()).unwrap().unwrap_err();
        assert!(errs.len() == 1 && errs[0].contains("product libc.a"), "{errs:?}");
        assert!(listing(&dst).unwrap().is_empty(), "nothing moved in: {:?}", listing(&dst));
        fs::write(src.join("libc.a"), "archive").unwrap();
        fs::write(src.join("stray.o"), "x").unwrap();
        let errs = import_into(&src, &dst, &head()).unwrap().unwrap_err();
        assert!(errs.len() == 1 && errs[0].contains("stray.o"), "{errs:?}");
        assert!(listing(&dst).unwrap().is_empty(), "nothing moved in: {:?}", listing(&dst));
        fs::remove_file(src.join("stray.o")).unwrap();
        let mut other = head();
        other.tag = "src-0000000000000000".into();
        let errs = import_into(&src, &dst, &other).unwrap().unwrap_err();
        assert!(errs.len() == 1 && errs[0].starts_with("tag:"), "{errs:?}");
        let _ = (fs::remove_dir_all(&src), fs::remove_dir_all(&dst));
    }

    // no spec clause (tooling: the build step builds here unless the import happened)
    #[test]
    fn build_step_builds_unless_imported() {
        let src = products("src-step", &head());
        let dst = scratch("dst-step");
        assert!(!import_from(None, &dst, &head()).unwrap());
        let mut other = head();
        other.profile = "fast traced".into();
        assert!(!import_from(Some(src.clone()), &dst, &other).unwrap(), "a refused import builds here");
        assert!(listing(&dst).unwrap().is_empty());
        assert!(import_from(Some(src.clone()), &dst, &head()).unwrap());
        assert_eq!(listing(&dst).unwrap(), listing(&src).unwrap());
        let _ = (fs::remove_dir_all(&src), fs::remove_dir_all(&dst));
    }

    // no spec clause (tooling: a directory without a manifest refuses)
    #[test]
    fn import_refuses_without_manifest() {
        let src = scratch("src-none");
        let dst = scratch("dst-none");
        let errs = import_into(&src, &dst, &head()).unwrap().unwrap_err();
        assert!(errs.len() == 1 && errs[0].starts_with("manifest:"), "{errs:?}");
        let _ = (fs::remove_dir_all(&src), fs::remove_dir_all(&dst));
    }
}
