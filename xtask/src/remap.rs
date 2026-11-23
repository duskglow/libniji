//! No build path in the products (PRODUCT-PATH-REMAP).
//!
//! The product crates are built with `--remap-path-prefix` for the workspace root and
//! `$CARGO_HOME` (`align::remap_flags`), so panic-location strings read `/niji/crates/...` on
//! every role and host, and the products' bytes do not depend on where they were built.
//! `check` scans the built products for the build's own paths and fails when one is left (run at
//! the end of `cargo xtask so`). `static_check` needs no product: it checks the build configuration.

use std::fs;
use std::path::Path;

/// The files of `target/niji` that go public (libraries, archives, the start object).
pub(crate) const PRODUCTS: &[&str] = &["libc.so.6", "libm.so.6", "libc.a", "libm.a", "niji-crt1.o", "niji-gcrt1.o", "niji-grcrt1.o", "niji-crti.o", "niji-crtn.o", "niji-Mcrt1.o", "libresolv.a"];

/// The path strings that must not appear in a product: the workspace root (as given and
/// canonical), `$CARGO_HOME` and `$HOME`'s `.cargo` and `.rustup`. Never shorter than 4 bytes
/// (a root of `/` would match everything).
pub fn needles(root: &Path) -> Vec<String> {
    // Every spelling of the root, the logical one through `$PWD` included (BUILD-REPRODUCIBLE-PATHS).
    let pwd = std::env::var_os("PWD").map(std::path::PathBuf::from);
    let mut v = crate::align::root_spellings(root, pwd.as_deref());
    if let Ok(h) = std::env::var("CARGO_HOME") {
        v.push(h);
    }
    if let Ok(h) = std::env::var("HOME") {
        v.push(format!("{h}/.cargo"));
        v.push(format!("{h}/.rustup"));
    }
    v.retain(|n| n.len() >= 4 && n.starts_with('/'));
    v.sort();
    v.dedup();
    v
}

/// `(needle, offset)` of the first occurrence of each needle in `bytes`.
pub fn find_paths(bytes: &[u8], needles: &[String]) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    for n in needles {
        let n = n.as_bytes();
        if let Some(at) = bytes.windows(n.len()).position(|w| w == n) {
            found.push((String::from_utf8_lossy(n).into_owned(), at));
        }
    }
    found
}

/// Fails when a built product under `dir` contains one of `needles`: the listed products and every other file at
/// the top of `dir` (the utilities, libmvec, libBrokenLocale and the markers carried the root too: BUILD-REPRODUCIBLE-PATHS).
pub fn check_dir(dir: &Path, needles: &[String]) -> Result<usize, String> {
    let mut bad = Vec::new();
    let mut scanned = 0;
    let mut names: std::collections::BTreeSet<String> = PRODUCTS.iter().chain(&crate::placeholders::LIBRARIES).chain(&crate::placeholders::ARCHIVES).map(|p| (*p).to_string()).collect();
    if let Ok(rd) = fs::read_dir(dir) {
        names.extend(rd.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_file())).map(|e| e.file_name().to_string_lossy().into_owned()));
    }
    for p in &names {
        let path = dir.join(p);
        let Ok(bytes) = fs::read(&path) else { continue };
        scanned += 1;
        for (n, at) in find_paths(&bytes, needles) {
            bad.push(format!("{p} has {n:?} at offset {at:#x}"));
        }
    }
    if bad.is_empty() {
        return Ok(scanned);
    }
    Err(format!("remap: build paths in the products (is --remap-path-prefix in the product build? PRODUCT-PATH-REMAP): {}", bad.join("; ")))
}


/// What the product build configuration carries, judged from `flags` (the encoded rustflags of a
/// product build for `root`) and `so_src` (the source of `xtask/src/so.rs`): the root is
/// remapped to `/niji`, and every place `so.rs` sets `CARGO_ENCODED_RUSTFLAGS` gets its value
/// from `align::encoded_rustflags` (so no product build can bypass the remap).
pub fn config_problems(root: &Path, flags: &str, so_src: &str) -> Vec<String> {
    let mut bad = Vec::new();
    let want = format!("--remap-path-prefix={}={}", root.display(), crate::align::REMAP_ROOT);
    if !flags.split('\x1f').any(|f| f == want) {
        bad.push(format!("the product rustflags lack {want:?}"));
    }
    for (i, line) in so_src.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("//") || !code.contains("\"CARGO_ENCODED_RUSTFLAGS\"") {
            continue;
        }
        if !code.contains("encoded_rustflags(root") {
            bad.push(format!("so.rs:{} sets CARGO_ENCODED_RUSTFLAGS without align::encoded_rustflags(root, ..)", i + 1));
        }
    }
    bad
}

/// The static check: what static can know, with no product built (a fresh tree, or a stale
/// `target/` left by an older build, say nothing about this source). It checks the product build
/// configuration; `cargo xtask so` checks the built bytes (`check`) after every product build.
pub fn static_check(root: &Path) -> Result<(), String> {
    let src = fs::read_to_string(root.join("xtask/src/so.rs")).map_err(|e| format!("remap: xtask/src/so.rs: {e}"))?;
    let flags = crate::align::encoded_rustflags(root, None);
    let bad = config_problems(root, &flags, &src);
    if !bad.is_empty() {
        return Err(format!("remap: the product build does not remap build paths (PRODUCT-PATH-REMAP): {}", bad.join("; ")));
    }
    println!("remap: the product build maps {} to {} (the built products are checked at the end of `cargo xtask so`)", root.display(), crate::align::REMAP_ROOT);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_build_path_and_passes_a_remapped_one() {
        let n = vec!["/mnt/nvme/claude/w/n4".to_string()];
        let planted = b"\0panic at /mnt/nvme/claude/w/n4/crates/rt/src/heap.rs:10\0";
        let remapped = b"\0panic at /niji/crates/rt/src/heap.rs:10\0";
        assert_eq!(find_paths(planted, &n), vec![("/mnt/nvme/claude/w/n4".to_string(), 10)]);
        assert!(find_paths(remapped, &n).is_empty());
    }

    #[test]
    fn check_dir_names_the_product_that_carries_a_path() {
        let dir = std::env::temp_dir().join(format!("niji-remap-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("libc.so.6"), b"clean /niji/x.rs").unwrap();
        fs::write(dir.join("libm.so.6"), b"dirty /srv/build/w/r3/crates/x.rs").unwrap();
        // A file no list names (a utility, as the C-built ones carried the root in DW_AT_comp_dir) is scanned too.
        fs::write(dir.join("getent"), b"comp dir /srv/build/w/r3/crates").unwrap();
        let n = vec!["/srv/build/w/r3".to_string()];
        let err = check_dir(&dir, &n).unwrap_err();
        let _ = fs::remove_dir_all(&dir);
        assert!(err.contains("libm.so.6 has") && !err.contains("libc.so.6 has"), "{err}");
        assert!(err.contains("getent has"), "an unlisted product file was not scanned: {err}");
    }

    #[test]
    fn config_check_needs_the_remap_flag_and_the_helper() {
        let root = Path::new("/x/y");
        let good_src = "cmd.env(\"CARGO_ENCODED_RUSTFLAGS\", crate::align::encoded_rustflags(root, rustflags));\n// \"CARGO_ENCODED_RUSTFLAGS\" in a comment";
        let flags = crate::align::encoded_rustflags(root, None);
        assert!(config_problems(root, &flags, good_src).is_empty());
        // The remap removed from the flags (the plant), or another root: reported.
        let no_remap = "-Cllvm-args=-align-all-functions=6";
        assert!(config_problems(root, no_remap, good_src)[0].contains("lack"));
        assert!(!config_problems(Path::new("/other"), &flags, good_src).is_empty());
        // A product build that sets its own flags: reported with its line number.
        let bypass = "a\ncmd.env(\"CARGO_ENCODED_RUSTFLAGS\", \"-Copt-level=3\");";
        assert!(config_problems(root, &flags, bypass)[0].starts_with("so.rs:2"));
    }

    /// no spec clause (tooling, BUILD-REPRODUCIBLE-PATHS): a C product compiled with the release debug flags is
    /// byte-identical in two trees of different paths and depths, each reached through a symlink as a role is
    /// (`~/w/<role>` -> `/mnt/.../w/<role>`; the compiler records `$PWD`, the logical path, as the compile
    /// directory) and built from the tree's top or a subdirectory; neither object names its tree.
    #[test]
    fn c_debug_info_is_the_same_in_two_tree_paths() {
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
        if !std::process::Command::new(&cc).arg("--version").output().is_ok_and(|o| o.status.success()) {
            eprintln!("c_debug_info_is_the_same_in_two_tree_paths: no C compiler `{cc}` here; NOT TESTED");
            return;
        }
        let base = std::env::temp_dir().join(format!("niji-remap-two-paths-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let src = "#include <errno.h>\nint niji_ph_placeholder(int x) { if (x < 0) { errno = EINVAL; return -1; } return x * 3; }\n";
        for sub in ["", "csys"] {
            let mut objs = Vec::new();
            let mut paths = Vec::new();
            for (i, (real, link)) in [("real/a", "la"), ("real/second/tree-b", "lb")].into_iter().enumerate() {
                let real = base.join(real);
                let link = base.join(link);
                if !real.is_dir() {
                    fs::create_dir_all(real.join("csys")).unwrap();
                    fs::write(real.join("csys/p.c"), src).unwrap();
                    std::os::unix::fs::symlink(&real, &link).unwrap();
                }
                // xtask's root is what getcwd gives (canonical); the build runs with the logical `$PWD`.
                let root = real.canonicalize().unwrap();
                let pwd = link.join(sub);
                let obj = base.join(format!("p-{i}.o"));
                let st = std::process::Command::new(&cc)
                    .current_dir(&pwd)
                    .env("PWD", &pwd)
                    .args(["-c", "-O2", "-fPIC", "-g1"])
                    .args(crate::so::debug_prefix_flags(&root, Some(&pwd)))
                    .arg(root.join("csys/p.c"))
                    .arg("-o")
                    .arg(&obj)
                    .status()
                    .unwrap();
                assert!(st.success(), "{cc} failed in {}", pwd.display());
                objs.push(fs::read(&obj).unwrap());
                paths.extend([link.display().to_string(), root.display().to_string()]);
            }
            let leaks = find_paths(&objs[0], &paths).into_iter().chain(find_paths(&objs[1], &paths)).collect::<Vec<_>>();
            assert!(leaks.is_empty(), "built from {sub:?}: an object names its tree: {leaks:?}");
            assert!(objs[0] == objs[1], "built from {sub:?}: the objects of the two trees differ");
        }
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn needles_ignore_a_bare_root() {
        assert!(needles(Path::new("/")).iter().all(|n| n.len() >= 4));
    }

    /// BUILD-PATH-HASH: cargo hashes a path package's path into `-C metadata` relative to the workspace
    /// root when the package lies inside it, else absolutely. The product crates therefore share the
    /// workspace rooted at `crates/` (not one of their own), so the products do not depend on the build
    /// directory: no product manifest may declare its own `[workspace]`, and every path dependency must
    /// stay under `crates/`.
    #[test]
    fn product_path_packages_lie_inside_the_product_workspace() {
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates");
        let ws = std::fs::read_to_string(crates.join("Cargo.toml")).expect("crates/Cargo.toml");
        assert!(ws.lines().any(|l| l.trim() == "[workspace]"), "crates/Cargo.toml is the product workspace");
        for member in ["libc-so", "libm-so", "libresolv-so"] {
            assert!(ws.contains(&format!("\"{member}\"")), "{member} is a member of the product workspace");
            let m = std::fs::read_to_string(crates.join(member).join("Cargo.toml")).expect("product manifest");
            assert!(!m.lines().any(|l| l.trim() == "[workspace]"), "{member}/Cargo.toml declares its own workspace: its path dependencies then lie outside the workspace root and their absolute paths enter -C metadata (BUILD-PATH-HASH)");
            for dep in m.lines().filter_map(|l| l.split("path = \"").nth(1)).filter_map(|r| r.split('"').next()) {
                let rel = Path::new(member).join(dep);
                let depth: i32 = rel.components().map(|c| if c == std::path::Component::ParentDir { -1 } else { 1 }).sum();
                assert!(depth >= 1, "{member}: path dependency {dep} lies outside crates/ (BUILD-PATH-HASH)");
            }
        }
    }
}
