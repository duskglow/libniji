//! Library search, the cache reader and `--list` (phase 14 DL-8, plan/ph14/design.md §4 and §19,
//! D-27.16): the search order with recorded candidate paths, the secure-mode rules of
//! spec/security/threat-model.md "The dynamic loader" that concern search (rules 1, 2, 3, 5, 9 and
//! 11; R14-15), the cache reader over hand-built files (Appendix D.3) and a corrupt-cache corpus, and
//! the built `ld.so --list` against the reference loader's for objects that are not libniji's.
//!
//! Clause: s27_1_c_1_1 (path search order). Pure tests take `Native` as an x86_64 table and say so;
//! the product tests skip (with the reason) when `cc`, the product or the reference loader is absent.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, missing_docs)]

use std::path::{Path, PathBuf};
#[path = "../../core/tests/support/scratch.rs"]
mod scratch;
#[path = "../../core/tests/support/contain.rs"]
mod contain;
use scratch::Scratch;
use std::process::Command;

use niji_ld::arch::{Arch, Native};
use niji_ld::cache::{compare, Cache};
use niji_ld::search::{self, Ctx, Scope, PREFIX};

const V3: &[&[u8]] = &[b"x86-64-v3", b"x86-64-v2"];

/// A new-format cache: entries `(flags, key, value, hwcaps index)` in file order, hwcaps names.
fn cache_file(entries: &[(u32, &str, &str, Option<u32>)], names: &[&str]) -> Vec<u8> {
    let strings_at = 48 + 24 * entries.len();
    let mut strings = Vec::new();
    let put = |s: &str, strings: &mut Vec<u8>| {
        let at = strings_at + strings.len();
        strings.extend_from_slice(s.as_bytes());
        strings.push(0);
        u32::try_from(at).unwrap()
    };
    let mut ents = Vec::new();
    for (flags, key, value, hw) in entries {
        let (k, v) = (put(key, &mut strings), put(value, &mut strings));
        ents.extend_from_slice(&flags.to_le_bytes());
        ents.extend_from_slice(&k.to_le_bytes());
        ents.extend_from_slice(&v.to_le_bytes());
        ents.extend_from_slice(&0u32.to_le_bytes());
        ents.extend_from_slice(&hw.map_or(0, |i| (1u64 << 62) | u64::from(i)).to_le_bytes());
    }
    let name_offs: Vec<u32> = names.iter().map(|n| put(n, &mut strings)).collect();
    let mut f = b"glibc-ld.so.cache1.1".to_vec();
    f.extend_from_slice(&u32::try_from(entries.len()).unwrap().to_le_bytes());
    f.extend_from_slice(&u32::try_from(strings.len()).unwrap().to_le_bytes());
    f.extend_from_slice(&[2, 0, 0, 0]);
    let ext = (strings_at + strings.len()).next_multiple_of(4);
    f.extend_from_slice(&u32::try_from(if names.is_empty() { 0 } else { ext }).unwrap().to_le_bytes());
    f.extend_from_slice(&[0; 12]);
    f.extend_from_slice(&ents);
    f.extend_from_slice(&strings);
    if !names.is_empty() {
        f.resize(ext, 0);
        let array = ext + 8 + 16;
        for w in [0xeaa4_2174u32, 1, 1, 0, u32::try_from(array).unwrap(), u32::try_from(4 * names.len()).unwrap()] {
            f.extend_from_slice(&w.to_le_bytes());
        }
        for o in name_offs {
            f.extend_from_slice(&o.to_le_bytes());
        }
    }
    f
}

fn ctx<'a>(secure: bool, library_path: Option<&'a [u8]>, cache: Option<&'a Cache<'a>>) -> Ctx<'a> {
    Ctx { secure, library_path, origin_path: None, platform: "haswell", hwcaps: &[], cache: cache.into() }
}

/// Every candidate path the search tries for `name`, in order (nothing is found).
fn candidates(name: &str, chain: &[Scope<'_>], c: &Ctx<'_>) -> Vec<String> {
    let mut seen = Vec::new();
    let found = search::search::<Native>(name.as_bytes(), chain, c, &mut |p| {
        seen.push(String::from_utf8(p.as_bytes().to_vec()).unwrap());
        false
    });
    assert!(!found);
    seen
}

const FLAGS: u32 = <Native as Arch>::CACHE_FLAGS;

/// s27_1_c_1_1 (the cache step), Appendix D.3.1/D.3.3, owner #940: entries of another architecture are skipped; the best
/// supported hwcaps entry before the first plain one wins; else the plain one. x86_64 table.
#[test]
fn cache_lookup_arch_flag_and_hwcaps() {
    let f = cache_file(
        &[
            (0x0803, "libv.so.1", "/x32/libv.so.1", None),
            (FLAGS, "libv.so.1", "/d/glibc-hwcaps/x86-64-v2/libv.so.1", Some(1)),
            (FLAGS, "libv.so.1", "/d/glibc-hwcaps/x86-64-v3/libv.so.1", Some(0)),
            (FLAGS, "libv.so.1", "/d/libv.so.1", None),
            (0x0003, "libonly32.so", "/32/libonly32.so", None),
        ],
        &["x86-64-v3", "x86-64-v2"],
    );
    let c = Cache::parse(&f).expect("well-formed cache");
    let get = |n: &str, hw: &[&[u8]]| c.lookup(n.as_bytes(), FLAGS, hw).map(|p| String::from_utf8(p.to_vec()).unwrap());
    assert_eq!(get("libv.so.1", V3).as_deref(), Some("/d/glibc-hwcaps/x86-64-v3/libv.so.1"));
    assert_eq!(get("libv.so.1", &[b"x86-64-v2"]).as_deref(), Some("/d/glibc-hwcaps/x86-64-v2/libv.so.1"));
    assert_eq!(get("libv.so.1", &[]).as_deref(), Some("/d/libv.so.1"), "no hwcaps supported: the plain entry, never the other arch's");
    assert_eq!(get("libonly32.so", V3), None, "an entry of another architecture is never taken");
}

/// s27_1_c_1_1 (the cache step), Appendix D.3.3: the numeric key comparison, and lookups in a sorted (descending) cache.
#[test]
fn cache_numeric_compare_and_sorted_search() {
    use std::cmp::Ordering::*;
    assert_eq!(compare(b"lib10.so", b"lib9.so"), Greater);
    assert_eq!(compare(b"libv.so.1.09", b"libv.so.1.9"), Equal);
    assert_eq!(compare(b"libv.so.1", b"libv.so.a"), Greater);
    assert_eq!(compare(b"libab.so", b"libab.so.1"), Less);
    let keys = ["lib10.so", "lib9.so", "libv.so.2", "libv.so.1.10", "libv.so.a", "libab.so.1", "libab.so", "libZed.so.1"];
    for w in keys.windows(2) {
        assert_eq!(compare(w[0].as_bytes(), w[1].as_bytes()), Greater, "{w:?}");
    }
    let entries: Vec<_> = keys.iter().map(|k| (FLAGS, *k, *k, None)).collect();
    let paths: Vec<String> = keys.iter().map(|k| format!("/l/{k}")).collect();
    let entries: Vec<_> = entries.iter().zip(&paths).map(|(e, p)| (e.0, e.1, p.as_str(), e.3)).collect();
    let f = cache_file(&entries, &[]);
    let c = Cache::parse(&f).unwrap();
    for k in keys {
        assert_eq!(c.lookup(k.as_bytes(), FLAGS, V3), Some(format!("/l/{k}").as_bytes()), "{k}");
    }
    assert_eq!(c.lookup(b"libnone.so", FLAGS, V3), None);
}

/// s27_1_c_1_1 (the cache step), threat model rule 5: a truncated or corrupted cache is "no cache" or "not found", never a crash
/// or a read outside the file; old and compat files are not read (#941).
#[test]
fn cache_corrupt_corpus_refused() {
    let f = cache_file(&[(FLAGS, "liba.so.1", "/x/liba.so.1", Some(0)), (FLAGS, "liba.so.1", "/y/liba.so.1", None)], &["x86-64-v2"]);
    // Appendix D.3.1: the string table follows the entries; every path returned lies inside it.
    let probe = |b: &[u8]| {
        if let Some(c) = Cache::parse(b) {
            let nlibs = usize::try_from(u32::from_le_bytes(b[20..24].try_into().unwrap())).unwrap();
            let table = 48 + 24 * nlibs..48 + 24 * nlibs + usize::try_from(u32::from_le_bytes(b[24..28].try_into().unwrap())).unwrap();
            for hw in [&[][..], V3] {
                if let Some(p) = c.lookup(b"liba.so.1", FLAGS, hw) {
                    let at = (p.as_ptr() as usize).wrapping_sub(b.as_ptr() as usize);
                    assert!(p.starts_with(b"/") && table.contains(&at) && at + p.len() < table.end, "a path from inside the string table: {at} not in {table:?}");
                }
            }
        }
    };
    // A value offset into the entry table at bytes that read as an absolute path (entry 1's
    // osversion word holds "/ev\0"): outside the string table, so the entry is inconsistent.
    let mut inside = f.clone();
    inside[48 + 24 + 12..48 + 24 + 16].copy_from_slice(b"/ev\0");
    inside[48 + 24 + 8..48 + 24 + 12].copy_from_slice(&u32::try_from(48 + 24 + 12).unwrap().to_le_bytes());
    probe(&inside);
    assert_eq!(Cache::parse(&inside).and_then(|c| c.lookup(b"liba.so.1", FLAGS, &[])), None, "a path from inside the entry table");
    for cut in 0..f.len() {
        probe(&f[..cut]);
        if cut < 48 + 48 {
            assert!(Cache::parse(&f[..cut]).is_none(), "cut at {cut}: the string table is incomplete");
        }
    }
    for at in (20..f.len() - 3).step_by(4) {
        // 48: the first entry's bytes, outside the string table.
        for wild in [0xffff_fff0u32, u32::try_from(f.len()).unwrap() - 1, 47, 48, 0x8000_0000] {
            let mut g = f.clone();
            g[at..at + 4].copy_from_slice(&wild.to_le_bytes());
            probe(&g);
        }
    }
    let mut old = b"ld.so-1.7.0\0".to_vec();
    old.extend_from_slice(&f);
    assert!(Cache::parse(&old).is_none(), "old and compat files are not read");
    assert!(Cache::parse(&[]).is_none());
}

/// s27_1_c_1_1: RPATH (requester, then its loader), LD_LIBRARY_PATH, RUNPATH, the cache, the
/// default directories; hwcaps subdirectories before each directory; RPATH ignored under RUNPATH.
#[test]
fn search_order() {
    let f = cache_file(&[(FLAGS, "libq.so.1", "/cached/libq.so.1", None)], &[]);
    let c = Cache::parse(&f).unwrap();
    let lib = Scope { rpath: Some(b"/r1"), runpath: None, origin: Some(b"/o/lib") };
    let prog = Scope { rpath: Some(b"/r0:$ORIGIN/$LIB"), runpath: None, origin: Some(b"/o") };
    let mut x = ctx(false, Some(b"/e1;/e2"), Some(&c));
    x.hwcaps = &[b"x86-64-v2"];
    let got = candidates("libq.so.1", &[lib, prog], &x);
    let lib_dir = format!("/o/{}", Native::LIB);
    let mut want: Vec<String> = Vec::new();
    for d in ["/r1", "/r0", lib_dir.as_str(), "/e1", "/e2"] {
        want.push(format!("{d}/glibc-hwcaps/x86-64-v2/libq.so.1"));
        want.push(format!("{d}/libq.so.1"));
    }
    want.push("/cached/libq.so.1".into());
    assert_eq!(&got[..want.len()], &want[..]);
    if PREFIX == "/" {
        let defaults: Vec<_> = got[want.len()..].iter().filter(|p| !p.contains("glibc-hwcaps")).cloned().collect();
        let sys: Vec<_> = Native::SYSTEM_DIRS.iter().map(|d| format!("{d}/libq.so.1")).collect();
        assert_eq!(defaults, sys);
    }
    let with_runpath = Scope { rpath: Some(b"/r1"), runpath: Some(b"/u1"), origin: None };
    let got = candidates("libq.so.1", &[with_runpath, prog], &ctx(false, None, None));
    assert_eq!(got.first().map(String::as_str), Some("/u1/libq.so.1"), "RUNPATH present: no RPATH at all");
    let rel = Scope { rpath: Some(b":rel"), runpath: None, origin: None };
    let got = candidates("libq.so.1", &[rel], &ctx(false, None, None));
    assert_eq!(&got[..2], ["./libq.so.1", "rel/libq.so.1"], "outside secure mode: empty = current directory");
}

/// s27_1_c_7 (`RTLD_DI_SERINFO` lists the search path; spec/behaviour/dynlink.md "dladdr and dlinfo"):
/// `search::dirs` gives exactly the directories `search` tries for a bare name, in its order, without
/// the cache and the hwcaps subdirectories, in plain and secure mode, with RPATH chains and RUNPATH.
#[test]
fn search_dirs_follow_the_search() {
    let lib = Scope { rpath: Some(b"/r1"), runpath: None, origin: Some(b"/o/lib") };
    let prog = Scope { rpath: Some(b"/r0:$ORIGIN/$LIB::rel"), runpath: None, origin: Some(b"/o") };
    let ru = Scope { rpath: Some(b"/ignored"), runpath: Some(b"/u1:$ORIGIN/x:$BAD/y"), origin: Some(b"/o/ru") };
    let prog_ru = Scope { rpath: None, runpath: Some(b"/pu"), origin: Some(b"/o") };
    for x in [ctx(false, Some(b"/e1;/e2:"), None), ctx(true, Some(b"/e1"), None)] {
        for chain in [&[lib, prog][..], &[ru, prog][..], &[lib, prog_ru][..], &[prog][..]] {
            let mut got = Vec::new();
            search::dirs::<Native>(chain, &x, &mut |d| {
                got.push(format!("{}/libq.so.1", String::from_utf8(d.to_vec()).unwrap()));
                true
            });
            let want: Vec<String> = candidates("libq.so.1", chain, &x).into_iter().map(|p| p.replace("/./", "/")).collect();
            let got: Vec<String> = got.into_iter().map(|p| p.replace("/./", "/")).collect();
            assert_eq!(got, want, "chain {chain:?} secure {}", x.secure);
        }
    }
    let mut first = Vec::new();
    search::dirs::<Native>(&[lib, prog], &ctx(false, None, None), &mut |d| {
        first.push(d.to_vec());
        false
    });
    assert_eq!(first, [b"/r1".to_vec()], "returning false stops the walk");
}

/// s27_1_c_1_1, threat model rules 1-3 and 5 in a secure process: LD_LIBRARY_PATH ignored, `$ORIGIN` only in a
/// trusted directory, empty and relative elements skipped, `$LIB` fixed.
#[test]
fn search_secure_rules() {
    let untrusted = Scope { rpath: None, runpath: Some(b"$ORIGIN/sub::rel:${ORIGIN}:/abs/$LIB"), origin: Some(b"/tmp/evil") };
    let x = ctx(true, Some(b"/env1:/env2"), None);
    let got = candidates("libq.so.1", &[untrusted], &x);
    assert!(got.iter().all(|p| p.starts_with('/')), "secure: only absolute candidates: {got:?}");
    assert!(!got.iter().any(|p| p.contains("/env")), "secure: LD_LIBRARY_PATH is never read: {got:?}");
    assert!(!got.iter().any(|p| p.contains("/tmp/evil")), "secure: $ORIGIN in an untrusted directory is not used: {got:?}");
    assert_eq!(got.first().map(String::as_str), Some(format!("/abs/{}/libq.so.1", Native::LIB).as_str()));
    if PREFIX == "/" {
        // Owner #4237: for prefix `/` the trusted directories are exactly the default ones.
        assert!(Native::SYSTEM_DIRS.iter().all(|d| search::trusted::<Native>(d.as_bytes())), "every default directory is trusted");
        for t in ["/lib64", "/usr/lib64", "/usr/local/lib", "/lib/x86_64-linux-gnu/sub"] {
            assert!(!search::trusted::<Native>(t.as_bytes()), "{t} is not a default directory, so not trusted");
        }
        let t = Native::SYSTEM_DIRS[0];
        let trusted = Scope { rpath: None, runpath: Some(b"$ORIGIN/sub"), origin: Some(t.as_bytes()) };
        let got = candidates("libq.so.1", &[trusted], &x);
        assert_eq!(got.first(), Some(&format!("{t}/sub/libq.so.1")), "a trusted $ORIGIN is used");
    }
    let open = candidates("libq.so.1", &[untrusted], &ctx(false, Some(b"/env1"), None));
    assert!(open.iter().any(|p| p.starts_with("/tmp/evil/sub/")) && open.iter().any(|p| p.starts_with("/env1/")));
}

/// s27_1_c_1_1, R14-3 (a), threat model rule 9: libniji's sonames resolve to `<prefix>/<libdir>` only (never
/// RPATH, RUNPATH or the cache); LD_LIBRARY_PATH may override outside secure mode only.
#[test]
fn search_pinned_sonames() {
    let f = cache_file(&[(FLAGS, "libc.so.6", "/host/libc.so.6", None)], &[]);
    let c = Cache::parse(&f).unwrap();
    let s = Scope { rpath: Some(b"/r"), runpath: None, origin: None };
    let mut own = search::PathBuf::default();
    assert!(search::own_lib(b"libc.so.6", &mut own));
    let own = String::from_utf8(own.as_bytes().to_vec()).unwrap();
    // The build-time layout (RELEASE-PACKAGING-DEB): `<NIJI_PREFIX>/<NIJI_LIBDIR>/NAME`, one `/`
    // between the parts; the default build (`/`, `lib`) keeps `/lib/libc.so.6`.
    let want = format!("{}/{}/libc.so.6", PREFIX.trim_end_matches('/'), search::LIBDIR.trim_matches('/'));
    assert_eq!(own, want, "pinned path is <prefix>/<libdir>/name");
    if option_env!("NIJI_PREFIX").is_none() && option_env!("NIJI_LIBDIR").is_none() {
        assert_eq!(own, "/lib/libc.so.6", "the default build's pinned directory is /lib");
    }
    let mut dir = search::PathBuf::default();
    assert!(search::own_lib(b"", &mut dir) && search::trusted::<Native>(dir.as_bytes()), "<prefix>/<libdir> is trusted");
    if PREFIX != "/" {
        // Another prefix: the default directory is <prefix>/<libdir> alone.
        let dir = String::from_utf8(dir.as_bytes().to_vec()).unwrap();
        let got = candidates("libq.so.1", &[Scope::default()], &ctx(false, None, None));
        assert_eq!(got, [format!("{dir}/libq.so.1")], "the default directory of a prefixed build is <prefix>/<libdir>");
    }
    assert_eq!(candidates("libc.so.6", &[s], &ctx(true, Some(b"/env"), Some(&c))), std::slice::from_ref(&own));
    assert_eq!(candidates("libc.so.6", &[s], &ctx(false, Some(b"/env"), Some(&c))), ["/env/libc.so.6".to_string(), own]);
}

/// s27_1_c_1_1 (the cache step; a cache read on first use, DL-START-SEARCH-3): the cell is read at most
/// once, keeps the bytes for the caller to release (none when there is no cache), and its empty state
/// takes no tag of its own: it is the bytes' address, a word every state writes, so the start-up loader's
/// release of an unread cache reads no unwritten stack memory (valgrind's memcheck reported a branch on a
/// nested `Option`'s unwritten payload at every program start). x86_64 table.
#[test]
fn lazy_cache_reads_once_and_its_empty_state_is_the_bytes_address() {
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
    static READS: AtomicUsize = AtomicUsize::new(0);
    static ABSENT_READS: AtomicUsize = AtomicUsize::new(0);
    fn read() -> Option<(&'static [u8], Cache<'static>)> {
        READS.fetch_add(1, Relaxed);
        let f: &'static [u8] = Box::leak(cache_file(&[(FLAGS, "libv.so.1", "/d/libv.so.1", None)], &[]).into_boxed_slice());
        Some((f, Cache::parse(f).expect("well-formed cache")))
    }
    fn absent() -> Option<(&'static [u8], Cache<'static>)> {
        ABSENT_READS.fetch_add(1, Relaxed);
        None
    }
    fn untagged<T>(_: &core::cell::OnceCell<T>) -> bool {
        size_of::<core::cell::OnceCell<T>>() == size_of::<T>()
    }
    let cell = search::LazyCache::new();
    assert!(untagged(&cell), "an empty lazy cache cell is a niche of its contents, not a tag beside unwritten contents");
    let lazy = search::CacheRef::Lazy(&cell, read);
    assert!(cell.get().is_none(), "nothing is read before a search asks");
    for _ in 0..2 {
        assert_eq!(lazy.get().and_then(|c| c.lookup(b"libv.so.1", FLAGS, &[])), Some(&b"/d/libv.so.1"[..]));
    }
    assert_eq!(READS.load(Relaxed), 1, "read once");
    assert!(cell.get().is_some_and(|(f, c)| !f.is_empty() && c.is_some()), "the bytes are kept for release");
    let none = search::LazyCache::new();
    let lazy = search::CacheRef::Lazy(&none, absent);
    assert!(lazy.get().is_none() && lazy.get().is_none());
    assert_eq!(ABSENT_READS.load(Relaxed), 1, "an absent cache is looked for once");
    assert_eq!(none.get().map(|(f, c)| (f.len(), c.is_some())), Some((0, false)), "no cache: no bytes to release");
}

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// The `name => path` lines of a `--list` output, without addresses and without libniji's own
/// libraries (pinned sonames) or the loader.
fn deps(out: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(out)
        .lines()
        .filter_map(|l| l.trim().split_once(" => ").map(|(n, p)| (n.to_string(), p.split(" (0x").next().unwrap_or("").to_string())))
        .filter(|(n, _)| !search::pinned(n.as_bytes()) && !n.ends_with(Native::INTERP))
        .map(|(n, p)| format!("{n} => {p}"))
        .collect()
}

/// `ld.so --list` and `LD_TRACE_LOADED_OBJECTS` against the reference loader (black box) for a
/// program with `$ORIGIN` RUNPATH dependencies, a missing one, and a system library from the
/// cache. s27_1_c_1_1.
#[cfg(target_arch = "x86_64")] // ARCH(x86_64): runs the host's x86-64 reference loader on objects the host toolchain builds
#[test]
fn list_matches_reference() {
    let ld = target().join("niji").join(Native::INTERP);
    let reference = Path::new("/lib64").join(Native::INTERP);
    if !ld.exists() || !reference.exists() || !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: needs cc, {} (cargo xtask so) and {}", ld.display(), reference.display());
        return;
    }
    // Canonical, so the reference's kernel-started `$ORIGIN` and the given path agree.
    let d = Scratch::dir_in(&std::fs::canonicalize(target()).unwrap(), "dl-search");
    std::fs::create_dir_all(d.join("sub")).unwrap();
    std::fs::write(d.join("x.c"), "int x(void){return 1;}\n").unwrap();
    std::fs::write(d.join("main.c"), "int x(void); int main(void){return x();}\n").unwrap();
    let cc = |args: &[&str]| assert!(Command::new("cc").current_dir(&d).args(args).status().unwrap().success(), "cc {args:?}");
    cc(&["-shared", "-fPIC", "-Wl,-soname,libgone.so.1", "-o", "libgone.so.1", "x.c"]);
    cc(&["-shared", "-fPIC", "-Wl,-soname,libb.so.1", "-o", "sub/libb.so.1", "x.c"]);
    cc(&["-shared", "-fPIC", "-Wl,-soname,liba.so.1", "-o", "liba.so.1", "x.c", "-Wl,--no-as-needed", "-L", "sub", "-l:libb.so.1", "-L", ".", "-l:libgone.so.1", "-lgcc_s", "-Wl,--enable-new-dtags,-rpath,$ORIGIN/sub"]);
    cc(&["-o", "prog", "main.c", "-L", ".", "-l:liba.so.1", "-Wl,--enable-new-dtags,-rpath,$ORIGIN"]);
    std::fs::remove_file(d.join("libgone.so.1")).unwrap();
    let prog = d.join("prog");
    let niji_libs = target().join("niji");
    let want = Command::new(&prog).env("LD_TRACE_LOADED_OBJECTS", "1").output().unwrap();
    let want_list = Command::new(&reference).arg("--list").arg(&prog).output().unwrap();
    let got = Command::new(&ld).arg("--list").arg(&prog).env("LD_LIBRARY_PATH", &niji_libs).output().unwrap();
    let traced = Command::new(&ld).arg(&prog).env("LD_TRACE_LOADED_OBJECTS", "1").env("LD_LIBRARY_PATH", &niji_libs).output().unwrap();
    eprintln!("reference:\n{}\nniji:\n{}{}", String::from_utf8_lossy(&want.stdout), String::from_utf8_lossy(&got.stdout), String::from_utf8_lossy(&got.stderr));
    assert!(deps(&want.stdout).iter().any(|l| l.starts_with("libgcc_s.so.1 => /")), "the reference found the system library");
    assert!(deps(&want.stdout).iter().any(|l| l == "libgone.so.1 => not found"), "the reference traced the missing library");
    assert_eq!(deps(&traced.stdout), deps(&want.stdout), "list_matches_reference: LD_TRACE_LOADED_OBJECTS differs from the reference");
    assert_eq!((traced.status.code(), want.status.code()), (Some(0), Some(0)), "tracing with a missing dependency: status 0");
    // `--list` with a missing dependency: the loader's error on stderr, nothing on stdout, 127.
    let err = |o: &std::process::Output| String::from_utf8_lossy(&o.stderr).split_once(": ").map(|(_, m)| m.trim_end().to_owned());
    assert_eq!((got.status.code(), want_list.status.code()), (Some(127), Some(127)), "--list with a missing dependency: status 127");
    assert!(got.stdout.is_empty() && want_list.stdout.is_empty(), "--list with a missing dependency prints no list");
    assert_eq!(err(&got), err(&want_list), "--list: the missing-dependency message differs from the reference");
}

/// The interpreter's line in a list (DL-TRACE-INTERP; spec/behaviour/dynlink.md "Trace lists"): listed only
/// when an object names the interpreter in DT_NEEDED (by soname or path), at its breadth-first place, the
/// vDSO's line after it when it comes first; libniji's libc and libm name it after their own needs, where
/// the reference's do; no DT_NEEDED at all is `statically linked`, status 0. Every entry point (kernel
/// start, `ld.so PROGRAM` and `--list`, and `--list` with the loader run through a link, which spells the
/// line `NAME => PATH`) against the reference loader, whole outputs compared with the
/// addresses, the loaders' paths and the C library's directory masked. s27_1_c_1_1, s31_3_g_1.
#[cfg(target_arch = "x86_64")] // ARCH(x86_64): runs the host's x86-64 reference loader on objects the host toolchain builds
#[test]
fn list_interpreter_line_matches_reference() {
    let ld = std::fs::canonicalize(target().join("niji").join(Native::INTERP)).unwrap_or_default();
    let reference = Path::new("/lib64").join(Native::INTERP);
    if !ld.is_file() || !reference.exists() || !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIP: needs cc, target/niji/{} (cargo xtask so) and {}", Native::INTERP, reference.display());
        return;
    }
    let libs = std::fs::canonicalize(target().join("niji")).unwrap();
    let d = Scratch::dir_in(&std::fs::canonicalize(target()).unwrap(), "dl-trace-interp");
    let src = [
        ("foo.c", "int foo(void) { return 1; }\n"),
        ("start.c", "void _start(void) { for (;;) {} }\n"),
        ("startfoo.c", "int foo(void); void _start(void) { foo(); for (;;) {} }\n"),
        ("hello.c", "#include <stdio.h>\nint main(void) { puts(\"hi\"); return 0; }\n"),
        ("barc.c", "#include <stdio.h>\nint bar(void) { return puts(\"x\"); }\n"),
    ];
    for (f, s) in src {
        std::fs::write(d.join(f), s).unwrap();
    }
    let cc = |args: &[&str]| assert!(Command::new("cc").current_dir(&d).args(args).status().unwrap().success(), "cc {args:?}");
    let n = ["-nostdlib", "-fno-stack-protector", "-O1", "-Wl,--build-id=none"];
    let so = |soname: &str, out: &str, rest: &[&str]| {
        let sn = format!("-Wl,-soname,{soname}");
        cc(&[&n[..], &["-shared", "-fPIC", sn.as_str(), "-o", out], rest].concat())
    };
    let rld = reference.to_str().unwrap();
    so("libfoo.so.1", "libfoo.so.1", &["foo.c"]);
    so("libbar.so.1", "libbar.so.1", &["foo.c"]);
    so("libfoo2.so.1", "libfoo2.so.1", &["foo.c", "-L.", "-Wl,--no-as-needed", "-l:libbar.so.1"]);
    so("libni.so.1", "libni.so.1", &["foo.c", "-Wl,--no-as-needed", rld]);
    cc(&["-shared", "-fPIC", "-Wl,-soname,libcl.so.1", "-o", "libcl.so.1", "barc.c"]);
    cc(&["-static-pie", "-o", "spie", "hello.c"]);
    // Each program twice: its PT_INTERP (and a DT_NEEDED by path) naming the reference's loader, then libniji's.
    let progs = ["foo", "interp", "interp_last", "ni", "full", "order", "m"];
    for (side, interp) in [("ref", rld), ("niji", ld.to_str().unwrap())] {
        so(interp, &format!("stub-{side}.so"), &["foo.c"]);
        let dl = format!("-Wl,--dynamic-linker={interp}");
        let p = |name: &str, args: &[&str]| {
            let out = format!("{side}-{name}");
            cc(&[&["-o", out.as_str(), dl.as_str(), "-L.", "-Wl,--no-as-needed"], args].concat())
        };
        let stub = format!("./stub-{side}.so");
        p("foo", &[&n[..], &["startfoo.c", "-l:libfoo.so.1"]].concat());
        p("interp", &[&n[..], &["start.c", rld, "-l:libfoo.so.1"]].concat());
        p("interp_last", &[&n[..], &["startfoo.c", "-l:libfoo.so.1", rld]].concat());
        p("ni", &[&n[..], &["startfoo.c", "-l:libni.so.1", "-l:libfoo.so.1"]].concat());
        p("full", &[&n[..], &["start.c", stub.as_str()]].concat());
        p("order", &["hello.c", "-l:libfoo2.so.1", "-lc", "-Wl,-rpath-link,."]);
        p("m", &["hello.c", "-lm", "-l:libfoo2.so.1", "-Wl,-rpath-link,."]);
    }
    let dir = d.to_str().unwrap();
    let norm = |o: &std::process::Output, lds: &[&str]| {
        let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
        for l in lds {
            s = s.replace(l, "@LD@");
        }
        s = s.replace("/lib/x86_64-linux-gnu/", "@LIB@/").replace(&format!("{}/", libs.display()), "@LIB@/").replace(dir, "D");
        let s: Vec<String> = s.lines().map(|l| l.split(" (0x").next().unwrap_or(l).to_string()).collect();
        format!("{s:?} status {:?}", o.status.code())
    };
    let ldp = ld.to_str().unwrap();
    let run = |side: &str, loader: &Path, operand: &str| {
        let (lp, lds) = if side == "ref" { (dir.to_string(), vec![rld]) } else { (format!("{dir}:{}", libs.display()), vec![ldp, rld]) };
        let mut outs = Vec::new();
        if !operand.ends_with(".so.1") && operand != "spie" {
            outs.push(Command::new(d.join(format!("{side}-{operand}"))).env("LD_TRACE_LOADED_OBJECTS", "1").env("LD_LIBRARY_PATH", &lp).output().unwrap());
        }
        let file = if operand.ends_with(".so.1") || operand == "spie" { d.join(operand) } else { d.join(format!("{side}-{operand}")) };
        outs.push(Command::new(loader).arg(&file).env("LD_TRACE_LOADED_OBJECTS", "1").env("LD_LIBRARY_PATH", &lp).output().unwrap());
        outs.push(Command::new(loader).arg("--list").arg(&file).env("LD_LIBRARY_PATH", &lp).output().unwrap());
        // The loader run by another path than the program's PT_INTERP names: the interpreter's
        // line spells `NAME => PATH`, PATH the loader as run (the same link name on both sides).
        let link = d.join("ldrun");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(loader, &link).unwrap();
        outs.push(Command::new(&link).arg("--list").arg(&file).env("LD_LIBRARY_PATH", &lp).output().unwrap());
        outs.iter().map(|o| norm(o, &lds)).collect::<Vec<_>>()
    };
    let mut bad = Vec::new();
    for operand in progs.iter().copied().chain(["libfoo.so.1", "libni.so.1", "libcl.so.1", "spie"]) {
        let (want, got) = (run("ref", &reference, operand), run("niji", &ld, operand));
        if want != got {
            bad.push(format!("{operand}:\n  reference {want:?}\n  niji      {got:?}"));
        }
    }
    let _ = std::fs::remove_dir_all(&d);
    assert!(bad.is_empty(), "list_interpreter_line_matches_reference: lists differ from the reference:\n{}", bad.join("\n"));
}

/// `ld.so PROGRAM` finds a name without `/` through the loader cache only (D-27.69, probe recorded
/// in spec/behaviour/dynlink.md): a path with `/` as given, a cache hit, and never a miss, a
/// relative cache path or the working directory. x86_64 table. s27_1_c_1_1 (the cache step).
#[test]
fn program_find_cache_only() {
    use niji_ld::program::{find, Found};
    let f = cache_file(&[(FLAGS, "libprogx.so.1", "/c/libprogx.so.1", None)], &[]);
    let c = Cache::parse(&f).expect("well-formed cache");
    let rel = cache_file(&[(FLAGS, "libprogy.so.1", "rel/libprogy.so.1", None)], &[]);
    let rel = Cache::parse(&rel).expect("parsed: the entry is checked at lookup");
    assert_eq!(find(b"./prog", Some(&c), FLAGS, &[]), Found::AsGiven);
    assert_eq!(find(b"lib/libprogx.so.1", Some(&c), FLAGS, &[]), Found::AsGiven, "a relative path with `/` is the caller's choice");
    assert_eq!(find(b"libprogx.so.1", Some(&c), FLAGS, &[]), Found::Cache(b"/c/libprogx.so.1"));
    assert_eq!(find(b"libprogx.so.1", Some(&c), 0x0803, &[]), Found::NotFound, "another architecture's entry is never taken");
    assert_eq!(find(b"libprogy.so.1", Some(&rel), FLAGS, &[]), Found::NotFound, "a relative cache path would open from the working directory");
    for name in [&b"prog"[..], b""] {
        assert_eq!(find(name, Some(&c), FLAGS, &[]), Found::NotFound, "{name:?}");
    }
    assert_eq!(find(b"libprogx.so.1", None, FLAGS, &[]), Found::NotFound, "no cache: not found, never the working directory");
}

/// The test program for [`ldso_program_without_slash`]: prints `ran:<TAG>` and one `arg:` line per argument.
#[cfg(target_arch = "x86_64")] // ARCH(x86_64): x86-64 system call numbers and assembly
const TAGGED: &str = r#"
static long sys3(long n, long a, long b, long c) {
    long r;
    __asm__ volatile("syscall" : "=a"(r) : "a"(n), "D"(a), "S"(b), "d"(c) : "rcx", "r11", "memory");
    return r;
}
static unsigned long len(const char *s) { unsigned long n = 0; while (s[n]) n++; return n; }
static void put(const char *p, const char *s) { sys3(1, 1, (long)p, len(p)); sys3(1, 1, (long)s, len(s)); sys3(1, 1, (long)"\n", 1); }
void start_c(long *sp) {
    char **argv = (char **)(sp + 1);
    put("ran:", TAG);
    for (long i = 0; i < sp[0]; i++) put("arg:", argv[i]);
    sys3(231, 0, 0, 0);
}
__asm__(".globl _start\n_start:\n\tmov %rsp, %rdi\n\tand $-16, %rsp\n\tcall start_c\n\thlt\n");
"#;

/// The built `ld.so` run as a command with a program name without `/` (D-27.69; every case of the
/// probe recorded in spec/behaviour/dynlink.md): a file of that name in the working directory, on
/// `LD_LIBRARY_PATH` (`.`, an empty element, absolute, relative) or on `PATH` is never run; the
/// reference's message and 127, also for `--list` and `LD_TRACE_LOADED_OBJECTS`; `./prog` and
/// `lib/prog` run as given. With a private loader cache (`unshare -r -m`, skipped with the reason
/// when unavailable) a listed name runs the cache's file with `argv[0]` as given, ahead of copies in
/// the working directory and on `LD_LIBRARY_PATH`. s27_1_c_1_1.
#[cfg(target_arch = "x86_64")] // ARCH(x86_64): the test program is x86-64
#[test]
fn ldso_program_without_slash() {
    let ld = target().join("niji").join(Native::INTERP);
    if !ld.exists() || !Command::new("cc").arg("--version").output().is_ok_and(|o| o.status.success()) {
        return eprintln!("SKIP: needs cc and {} (cargo xtask so)", ld.display());
    }
    let d = Scratch::dir_in(&target(), "ldso-program-name");
    for sub in ["lib", "bin", "cache"] {
        std::fs::create_dir_all(d.join(sub)).unwrap();
    }
    std::fs::write(d.join("prog.c"), TAGGED).unwrap();
    let cc = |out: &str, tag: &str| {
        let o = Command::new("cc")
            .args(["-nostdlib", "-fPIE", "-pie", "-O1", "-fno-stack-protector", "-fcf-protection=none"])
            .arg(format!("-DTAG=\"{tag}\""))
            .arg(d.join("prog.c"))
            .arg("-o")
            .arg(d.join(out))
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    };
    for (out, tag) in [("prog", "cwd"), ("lib/prog", "libdir"), ("bin/prog", "path"), ("lib/libprogx.so.1", "libdir"), ("cache/libprogx.so.1", "cache")] {
        cc(out, tag);
    }
    // The working directory's `libprogx.so.1` is not an object: running, listing or tracing it
    // instead of the cache's file gives a different result.
    std::fs::write(d.join("libprogx.so.1"), "not an object\n").unwrap();
    let run = |args: &[&str], env: &[(&str, String)]| {
        // No wrapper process: `LD_TRACE_LOADED_OBJECTS` would list the wrapper instead.
        let o = Command::new(&ld).args(args).envs(env.iter().map(|(k, v)| (k, v))).current_dir(&d).output().unwrap();
        (o.status.code(), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned())
    };
    let missing = |n: &str| (Some(127), String::new(), format!("{n}: error while loading shared libraries: {n}: cannot open shared object file\n"));
    let lib = d.join("lib").display().to_string();
    let bin = format!("{}:/usr/bin:/bin", d.join("bin").display());
    let envs: [(&str, &[(&str, String)]); 7] = [
        ("cwd", &[]),
        ("LD_LIBRARY_PATH=.", &[("LD_LIBRARY_PATH", ".".into())]),
        ("LD_LIBRARY_PATH=:", &[("LD_LIBRARY_PATH", ":".into())]),
        ("LD_LIBRARY_PATH=lib:", &[("LD_LIBRARY_PATH", "lib:".into())]),
        ("LD_LIBRARY_PATH absolute", &[("LD_LIBRARY_PATH", lib.clone())]),
        ("PATH", &[("PATH", bin)]),
        ("LD_TRACE_LOADED_OBJECTS", &[("LD_TRACE_LOADED_OBJECTS", "1".into())]),
    ];
    for (what, env) in envs {
        assert_eq!(run(&["prog", "a"], env), missing("prog"), "{what}");
    }
    assert_eq!(run(&["--list", "prog"], &[]), missing("prog"), "--list");
    assert_eq!(run(&["absentprog", "a"], &[]), missing("absentprog"), "absent");
    assert_eq!(run(&[""], &[]), (Some(127), String::new(), ": error while loading shared libraries: cannot open shared object file\n".into()), "empty name");
    assert_eq!(run(&["./prog", "a"], &[]), (Some(0), "ran:cwd\narg:./prog\narg:a\n".into(), String::new()), "./prog");
    assert_eq!(run(&["lib/prog", "a"], &[]), (Some(0), "ran:libdir\narg:lib/prog\narg:a\n".into(), String::new()), "lib/prog");
    // A private cache listing `libprogx.so.1` (the copy in `cache/`).
    let mut cache_path = search::PathBuf::default();
    assert!(search::prefixed(search::CACHE_FILE, &mut cache_path));
    let system_cache = String::from_utf8(cache_path.as_bytes().to_vec()).unwrap();
    if !Path::new(&system_cache).exists() || !Command::new("unshare").args(["-r", "-m", "true"]).status().is_ok_and(|s| s.success()) {
        let _ = std::fs::remove_dir_all(&d);
        return eprintln!("SKIP (cache cases): needs {system_cache} and `unshare -r -m`");
    }
    let target_x = d.join("cache/libprogx.so.1").display().to_string();
    std::fs::write(d.join("ld.so.cache"), cache_file(&[(FLAGS, "libprogx.so.1", &target_x, None)], &[])).unwrap();
    // `trace` exports `LD_TRACE_LOADED_OBJECTS=1` just before the loader starts (not to `unshare`
    // or `sh`, which would list themselves).
    let in_ns_with = |trace: bool, args: &[&str]| {
        // Contained (support/contain.rs).
        let o = contain::user("unshare")
            .args(["-r", "-m", "sh", "-c", "mount --bind \"$1\" \"$2\" && t=$3 && shift 3 && if [ \"$t\" = 1 ]; then export LD_TRACE_LOADED_OBJECTS=1; fi && exec \"$@\"", "sh"])
            .arg(d.join("ld.so.cache"))
            .arg(&system_cache)
            .arg(if trace { "1" } else { "0" })
            .arg(&ld)
            .args(args)
            .env("LD_LIBRARY_PATH", &lib)
            .current_dir(&d)
            .output()
            .unwrap();
        (o.status.code(), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned())
    };
    let in_ns = |args: &[&str]| in_ns_with(false, args);
    assert_eq!(in_ns(&["libprogx.so.1", "a"]), (Some(0), "ran:cache\narg:libprogx.so.1\narg:a\n".into(), String::new()), "cache hit");
    assert_eq!(in_ns(&["--list", "libprogx.so.1"]), (Some(0), "\tstatically linked\n".into(), String::new()), "--list cache hit");
    assert_eq!(in_ns_with(true, &["libprogx.so.1"]), (Some(0), "\tstatically linked\n".into(), String::new()), "LD_TRACE_LOADED_OBJECTS cache hit");
    assert_eq!(in_ns(&["prog", "a"]), missing("prog"), "not in the cache");
    let _ = std::fs::remove_dir_all(&d);
}
