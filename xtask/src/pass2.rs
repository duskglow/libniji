//! PASS2-NATIVE-RUNNER: the gate's second test pass as libniji processes.
//!
//! The first pass runs every test binary as the host's process (the host's loader and C library).
//! The second pass runs the same binaries (no second workspace build) as processes libniji's loader
//! started with libniji's `libc.so.6`: the harness, its threads, unwinding, `fork` and re-execution
//! then run on libniji's start-up, heap, string functions and threads (TEST-PASS2-NATIVE measured
//! 6755 of 6759 pass-1 tests passing so, none of the rest a libniji gap).
//!
//! How (`prepare`, `patch_listed`, `exec`):
//!
//! * a per-tree root `/tmp/niji2-<8 hex of the tree path>` (short: the binaries' `PT_INTERP` field
//!   has room for 27 bytes and a NUL) holding a loader compiled with that root as its installation
//!   prefix (`so::build_native_ld`, in a cargo target directory no other step uses), `lib/` of
//!   links to `target/niji/*.so*` plus the toolchain's `libgcc_s.so.1` (the unwinder every Rust
//!   `std` binary needs; a prefixed loader searches only `<prefix>/lib`), and a copy of this xtask
//!   binary as the runner;
//! * a copy of each test binary with its `PT_INTERP` rewritten to `<root>/ld.so`, in a sibling
//!   directory of the binary's own at the same depth with the same file name
//!   (`target/debug/deps/x-hash` -> `target/debug/niji-deps/x-hash`): tests that derive paths or
//!   names from `current_exe()` see the same shape, and re-executing `current_exe()` starts the
//!   copy again (a runner that ran `ld.so <binary>` would re-execute the loader);
//! * `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=<root>/xtask pass2-exec`: cargo and nextest
//!   hand it each test binary's path; it executes the copy, and refuses a copy that is missing,
//!   older than its binary, or made from another build of it (the length and modification time
//!   recorded when the copy was made).

use std::path::{Path, PathBuf};
use std::process::Command;

/// The runner variable cargo and nextest read for the host target.
pub const RUNNER_VAR: &str = "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER";

/// The runner's subcommand (`main.rs` dispatches it before looking for a tree).
pub const EXEC_CMD: &str = "pass2-exec";

/// The directory of the copies, a sibling of the binaries' own `deps`.
const COPY_DIR: &str = "niji-deps";

/// The suffix of the file beside each copy that records its binary's length and modification time.
const STAMP_SUFFIX: &str = ".niji-src";

/// ELF program header type of the interpreter path.
const PT_INTERP: u32 = 3;

/// The per-tree root of the native pass: `/tmp/niji2-<the first 8 hex digits of the SHA-256 of the
/// tree's path>`. Two trees never share it; two passes in one tree run one after the other.
pub fn root_dir(tree: &Path) -> Result<String, String> {
    let h = crate::sha256::sha256_text(&tree.display().to_string())?;
    Ok(format!("/tmp/niji2-{}", h.get(..8).ok_or("short digest")?))
}

/// The interpreter path the copies name.
pub fn interp_of(root: &str) -> String {
    format!("{root}/ld.so")
}

fn u16_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(u16::from_le_bytes(b.get(at..at.checked_add(2)?)?.try_into().ok()?)))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at.checked_add(8)?)?.try_into().ok()?))
}

/// Rewrite the `PT_INTERP` string of the 64-bit little-endian ELF image `elf` to `interp`, padded
/// with NULs to the segment's file size (the kernel requires the segment's last byte to be NUL).
/// Refused: not such an image, no or two `PT_INTERP` segments, a segment outside the image, or an
/// `interp` that does not fit with its NUL. Nothing else in the image changes.
pub fn patch_interp(elf: &mut [u8], interp: &str) -> Result<(), String> {
    if elf.get(..4) != Some(b"\x7fELF".as_slice()) || elf.get(4) != Some(&2) || elf.get(5) != Some(&1) {
        return Err("not a 64-bit little-endian ELF file".into());
    }
    let bad = || "truncated ELF header or program headers".to_string();
    let phoff = usize::try_from(u64_at(elf, 0x20).ok_or_else(bad)?).map_err(|_| bad())?;
    let phentsize = usize::try_from(u16_at(elf, 0x36).ok_or_else(bad)?).map_err(|_| bad())?;
    let phnum = usize::try_from(u16_at(elf, 0x38).ok_or_else(bad)?).map_err(|_| bad())?;
    if phentsize < 0x38 {
        return Err(format!("program header entries of {phentsize} bytes"));
    }
    let mut found: Option<(usize, usize)> = None;
    for i in 0..phnum {
        let ph = i.checked_mul(phentsize).and_then(|o| o.checked_add(phoff)).ok_or_else(bad)?;
        if u32_at(elf, ph).ok_or_else(bad)? != PT_INTERP {
            continue;
        }
        if found.is_some() {
            return Err("two PT_INTERP segments".into());
        }
        let off = usize::try_from(u64_at(elf, ph + 8).ok_or_else(bad)?).map_err(|_| bad())?;
        let size = usize::try_from(u64_at(elf, ph + 0x20).ok_or_else(bad)?).map_err(|_| bad())?;
        found = Some((off, size));
    }
    let (off, size) = found.ok_or("no PT_INTERP segment (a static binary)")?;
    let end = off.checked_add(size).filter(|e| *e <= elf.len()).ok_or("the PT_INTERP segment lies outside the file")?;
    if interp.len() >= size || interp.as_bytes().contains(&0) {
        return Err(format!("interpreter {interp:?} does not fit the PT_INTERP segment of {size} bytes with its NUL"));
    }
    let seg = elf.get_mut(off..end).ok_or("the PT_INTERP segment lies outside the file")?;
    seg.fill(0);
    seg.get_mut(..interp.len()).ok_or("short segment")?.copy_from_slice(interp.as_bytes());
    Ok(())
}

/// The copy of test binary `bin`: `<dir of bin>/../niji-deps/<file name of bin>`. `None` when `bin`
/// has no parent directory or file name.
pub fn copy_of(bin: &Path) -> Option<PathBuf> {
    Some(bin.parent()?.parent()?.join(COPY_DIR).join(bin.file_name()?))
}

fn stamp_path(copy: &Path) -> PathBuf {
    let mut s = copy.as_os_str().to_owned();
    s.push(STAMP_SUFFIX);
    PathBuf::from(s)
}

/// A binary's length and modification time (nanoseconds since the epoch), as the stamp records them.
fn stamp_of(bin: &Path) -> Result<String, String> {
    let m = std::fs::metadata(bin).map_err(|e| format!("{}: {e}", bin.display()))?;
    let t = m.modified().map_err(|e| format!("{}: {e}", bin.display()))?.duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?;
    Ok(format!("{} {}", m.len(), t.as_nanos()))
}

/// Write the patched copy of `bin` (interpreter `interp`) and its stamp. The copy is written under a
/// temporary name and renamed, so a reader never sees half a copy.
pub fn make_copy(bin: &Path, interp: &str) -> Result<PathBuf, String> {
    use std::os::unix::fs::PermissionsExt as _;
    let copy = copy_of(bin).ok_or_else(|| format!("{}: no directory to put its copy beside", bin.display()))?;
    let dir = copy.parent().ok_or("copy without a directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let stamp = stamp_of(bin)?;
    let mut bytes = std::fs::read(bin).map_err(|e| format!("{}: {e}", bin.display()))?;
    patch_interp(&mut bytes, interp).map_err(|e| format!("{}: {e}", bin.display()))?;
    let tmp = dir.join(format!(".{}.tmp", copy.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
    std::fs::write(&tmp, &bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &copy).map_err(|e| format!("{}: {e}", copy.display()))?;
    std::fs::write(stamp_path(&copy), stamp).map_err(|e| format!("{}: {e}", copy.display()))?;
    Ok(copy)
}

/// Whether the copy of `bin` may run for it: it exists, is not older than `bin`, and its stamp
/// names `bin`'s present length and modification time (a rebuild after the copy was made changes
/// them). The error says which and how to make the copy again.
pub fn check_copy(bin: &Path, copy: &Path) -> Result<(), String> {
    let again = "run the pass through `cargo xtask test` or `cargo xtask affected --run`, which make the copies";
    let cm = std::fs::metadata(copy).map_err(|e| format!("pass2-exec: no native copy {} of {} ({e}): {again}", copy.display(), bin.display()))?;
    let bm = std::fs::metadata(bin).map_err(|e| format!("pass2-exec: {}: {e}", bin.display()))?;
    if let (Ok(c), Ok(b)) = (cm.modified(), bm.modified()) {
        if c < b {
            return Err(format!("pass2-exec: the native copy {} is older than its binary {}: {again}", copy.display(), bin.display()));
        }
    }
    let want = stamp_of(bin)?;
    let had = std::fs::read_to_string(stamp_path(copy)).unwrap_or_default();
    if had != want {
        return Err(format!("pass2-exec: the native copy {} was made from another build of {} (stamp {had:?}, binary now {want:?}): {again}", copy.display(), bin.display()));
    }
    Ok(())
}

/// The runner: `pass2-exec <binary> <args>...` executes the binary's checked copy with the same
/// arguments. Returns only on an error.
pub fn exec(args: &[String]) -> String {
    use std::os::unix::process::CommandExt as _;
    let Some(bin) = args.first() else { return format!("usage: xtask {EXEC_CMD} <test binary> [args...]") };
    let bin = Path::new(bin);
    let Some(copy) = copy_of(bin) else { return format!("pass2-exec: {}: no copy path", bin.display()) };
    if let Err(e) = check_copy(bin, &copy) {
        return e;
    }
    let e = Command::new(&copy).args(args.get(1..).unwrap_or_default()).exec();
    format!("pass2-exec: cannot execute {}: {e}", copy.display())
}

/// The test executables of a `cargo test --no-run --message-format json` output: the `executable`
/// of each `compiler-artifact` message whose profile is a test profile.
pub fn test_executables(json: &str) -> Vec<String> {
    let mut out: Vec<String> = json
        .lines()
        .filter(|l| l.contains("\"reason\":\"compiler-artifact\""))
        .filter(|l| l.split_once("\"profile\":{").and_then(|(_, r)| r.split_once('}')).is_some_and(|(p, _)| p.contains("\"test\":true")))
        .filter_map(|l| l.split_once("\"executable\":\"").and_then(|(_, r)| r.split_once('"')).map(|(p, _)| p.to_string()))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The native pass's root, made afresh: the prefixed loader, `lib/` and the runner. Returns the
/// runner variable's value. Needs the products (`cargo xtask so`).
pub fn prepare(tree: &Path) -> Result<String, String> {
    let root = root_dir(tree)?;
    let r = Path::new(&root);
    let _ = std::fs::remove_dir_all(r);
    let lib = r.join("lib");
    std::fs::create_dir_all(&lib).map_err(|e| format!("{}: {e}", lib.display()))?;
    crate::so::build_native_ld(tree, &root, &r.join("ld.so"))?;
    let products = tree.join("target/niji");
    let mut libs: Vec<PathBuf> = std::fs::read_dir(&products).map_err(|e| format!("{}: {e} (run `cargo xtask so`)", products.display()))?.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().contains(".so"))).collect();
    libs.sort();
    if !libs.iter().any(|p| p.ends_with("libc.so.6")) {
        return Err(format!("{}: no libc.so.6 (run `cargo xtask so`)", products.display()));
    }
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let out = Command::new(&cc).arg("-print-file-name=libgcc_s.so.1").output().map_err(|e| format!("{cc}: {e}"))?;
    let gcc_s = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    if !gcc_s.is_absolute() || !gcc_s.exists() {
        return Err(format!("{cc} -print-file-name=libgcc_s.so.1 names no file ({}): Rust std binaries need it", gcc_s.display()));
    }
    libs.push(gcc_s);
    for p in &libs {
        let name = p.file_name().ok_or("no file name")?;
        std::os::unix::fs::symlink(p, lib.join(name)).map_err(|e| format!("{}: {e}", lib.join(name).display()))?;
    }
    let me = std::env::current_exe().map_err(|e| format!("this xtask's path: {e}"))?;
    std::fs::copy(&me, r.join("xtask")).map_err(|e| format!("copy {}: {e}", me.display()))?;
    println!("pass2: native root {root}: ld.so (prefix {root}), lib/ ({} libraries), runner", libs.len());
    Ok(format!("{root}/xtask {EXEC_CMD}"))
}

/// Make the copies of the test binaries a `cargo test --no-run` with `args` lists (they are built
/// by then: the same profile and features as the run that follows). Returns how many.
pub fn patch_listed(cargo: &str, tree: &Path, args: &[String]) -> Result<usize, String> {
    let out = crate::nextest::poison(&mut Command::new(cargo))
        .current_dir(tree)
        .args(["test", "--no-run", "--message-format", "json"])
        .args(args)
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|e| format!("cargo test --no-run: {e}"))?;
    if !out.status.success() {
        return Err(format!("cargo test --no-run {} failed: {}", args.join(" "), out.status));
    }
    let interp = interp_of(&root_dir(tree)?);
    let bins = test_executables(&String::from_utf8_lossy(&out.stdout));
    for b in &bins {
        make_copy(Path::new(b), &interp)?;
    }
    println!("pass2: {} test binaries copied with interpreter {interp}", bins.len());
    Ok(bins.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal ELF64 image: header, `n` program headers at 64, an interpreter segment at 0x100.
    fn image(interp: &[u8], seg: usize, kinds: &[u32]) -> Vec<u8> {
        let mut b = vec![0u8; 0x100 + seg + 16];
        b[..4].copy_from_slice(b"\x7fELF");
        b[4] = 2;
        b[5] = 1;
        b[0x20..0x28].copy_from_slice(&64u64.to_le_bytes());
        b[0x36..0x38].copy_from_slice(&0x38u16.to_le_bytes());
        b[0x38..0x3a].copy_from_slice(&(kinds.len() as u16).to_le_bytes());
        for (i, k) in kinds.iter().enumerate() {
            let ph = 64 + i * 0x38;
            b[ph..ph + 4].copy_from_slice(&k.to_le_bytes());
            b[ph + 8..ph + 16].copy_from_slice(&0x100u64.to_le_bytes());
            b[ph + 0x20..ph + 0x28].copy_from_slice(&(seg as u64).to_le_bytes());
        }
        b[0x100..0x100 + interp.len()].copy_from_slice(interp);
        b[0x100 + seg..].fill(0xee);
        b
    }

    const HOST: &[u8] = b"/lib64/ld-linux-x86-64.so.2\0";

    #[test]
    fn the_interpreter_is_rewritten_and_padded_and_nothing_else_changes() {
        // Trace: no spec clause (PASS2-NATIVE-RUNNER tooling: the PT_INTERP patcher).
        let before = image(HOST, HOST.len(), &[6, PT_INTERP, 1]);
        let mut b = before.clone();
        patch_interp(&mut b, "/tmp/niji2-0123abcd/ld.so").unwrap();
        assert_eq!(&b[0x100..0x100 + HOST.len()], b"/tmp/niji2-0123abcd/ld.so\0\0\0");
        let changed: Vec<usize> = (0..b.len()).filter(|&i| b[i] != before[i]).collect();
        assert!(changed.iter().all(|&i| (0x100..0x100 + HOST.len()).contains(&i)), "bytes outside the segment changed: {changed:?}");
        // A path exactly one byte short of the segment still ends in its NUL.
        let mut b = before.clone();
        let longest = "/".repeat(HOST.len() - 1);
        patch_interp(&mut b, &longest).unwrap();
        assert_eq!(b[0x100 + HOST.len() - 1], 0);
    }

    #[test]
    fn a_patch_that_cannot_be_made_is_refused() {
        // Trace: no spec clause (PASS2-NATIVE-RUNNER tooling: the PT_INTERP patcher's refusals).
        let ok = image(HOST, HOST.len(), &[PT_INTERP]);
        let too_long = "/".repeat(HOST.len());
        assert!(patch_interp(&mut ok.clone(), &too_long).unwrap_err().contains("does not fit"));
        assert!(patch_interp(&mut ok.clone(), "/a\0b").is_err(), "an embedded NUL");
        assert!(patch_interp(&mut image(HOST, HOST.len(), &[1, 6]), "/x").unwrap_err().contains("no PT_INTERP"));
        assert!(patch_interp(&mut image(HOST, HOST.len(), &[PT_INTERP, PT_INTERP]), "/x").unwrap_err().contains("two PT_INTERP"));
        let mut not_elf = ok.clone();
        not_elf[0] = 0;
        assert!(patch_interp(&mut not_elf, "/x").unwrap_err().contains("not a 64-bit"));
        let mut elf32 = ok.clone();
        elf32[4] = 1;
        assert!(patch_interp(&mut elf32, "/x").is_err());
        let mut outside = ok.clone();
        outside[64 + 0x20..64 + 0x28].copy_from_slice(&(1u64 << 40).to_le_bytes());
        assert!(patch_interp(&mut outside, "/x").unwrap_err().contains("outside"));
        let mut cut = ok.clone();
        cut.truncate(0x50);
        assert!(patch_interp(&mut cut, "/x").is_err(), "program headers past the end");
    }

    #[test]
    fn the_root_fits_the_host_interpreter_field_and_differs_per_tree() {
        // Trace: no spec clause (PASS2-NATIVE-RUNNER tooling: the per-tree root).
        let a = root_dir(Path::new("/srv/w/role-a")).unwrap();
        let b = root_dir(Path::new("/srv/w/role-b")).unwrap();
        assert_ne!(a, b);
        assert_eq!(a, root_dir(Path::new("/srv/w/role-a")).unwrap(), "stable for one tree");
        assert!(interp_of(&a).len() < HOST.len(), "{} must fit the 28-byte field of the host interpreter", interp_of(&a));
    }

    #[test]
    fn a_copy_sits_beside_the_binaries_at_the_same_depth_with_the_same_name() {
        // Trace: no spec clause (PASS2-NATIVE-RUNNER tooling: tests derive names and paths from current_exe).
        assert_eq!(copy_of(Path::new("/t/target/debug/deps/s23-0f1e")), Some(PathBuf::from("/t/target/debug/niji-deps/s23-0f1e")));
        assert_eq!(copy_of(Path::new("x")), None);
    }

    #[test]
    fn the_runner_refuses_a_missing_older_or_rebuilt_copy() {
        // Trace: no spec clause (PASS2-NATIVE-RUNNER tooling: a stale copy never runs for its binary).
        let dir = std::env::temp_dir().join(format!("pass2-check-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let bin = dir.join("debug/deps/t-1");
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(&bin, image(HOST, HOST.len(), &[PT_INTERP])).unwrap();
        let copy = copy_of(&bin).unwrap();
        assert!(check_copy(&bin, &copy).unwrap_err().contains("no native copy"));
        assert_eq!(make_copy(&bin, "/n/ld.so").unwrap(), copy);
        check_copy(&bin, &copy).unwrap();
        assert_eq!(&std::fs::read(&copy).unwrap()[0x100..0x109], b"/n/ld.so\0");
        // A rebuild: the binary is written again after the copy (a later time, another length).
        let mut again = image(HOST, HOST.len(), &[PT_INTERP]);
        again.push(1);
        std::fs::write(&bin, again).unwrap();
        let f = std::fs::File::options().write(true).open(&bin).unwrap();
        f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5)).unwrap();
        drop(f);
        assert!(check_copy(&bin, &copy).unwrap_err().contains("older than its binary"));
        // A time not after the copy's, but not the build the copy was made from (its stamp).
        let f = std::fs::File::options().write(true).open(&bin).unwrap();
        f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1)).unwrap();
        drop(f);
        assert!(check_copy(&bin, &copy).unwrap_err().contains("another build"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_test_profile_executables_are_listed() {
        // Trace: no spec clause (PASS2-NATIVE-RUNNER tooling: which binaries get copies).
        let json = concat!(
            "{\"reason\":\"compiler-artifact\",\"target\":{\"name\":\"a\",\"test\":true},\"profile\":{\"opt_level\":\"0\",\"test\":true},\"executable\":\"/t/debug/deps/a-1\"}\n",
            "{\"reason\":\"compiler-artifact\",\"target\":{\"name\":\"tool\",\"test\":true},\"profile\":{\"opt_level\":\"0\",\"test\":false},\"executable\":\"/t/debug/tool\"}\n",
            "{\"reason\":\"compiler-artifact\",\"target\":{\"name\":\"l\"},\"profile\":{\"test\":false},\"executable\":null}\n",
            "{\"reason\":\"build-finished\",\"success\":true}\n"
        );
        assert_eq!(test_executables(json), vec!["/t/debug/deps/a-1".to_string()]);
    }
}
