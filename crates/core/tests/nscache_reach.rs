//! Which differential programs reach the name-service cache (D-16.18, amending D-16.10's test
//! clause). A differential test runs libniji with the cache on and off (`oracle/nscache_mode.sh`)
//! only when its program makes a lookup the cache serves: at process start the cache only stores
//! its switch, and it reads the switch and attaches at the first lookup, so for a program that
//! makes none an "on" run executes the same code as an "off" run. The tests that run one mode stay
//! honest through three checks:
//!
//! 1. [`CALLERS`]: the library source files that name the cache are exactly the known ones, each
//!    with the C functions through which it is reached (a family of [`FAMILIES`]) or the reason it
//!    reads no switch. A new caller fails here until its entry points join [`FAMILIES`].
//! 2. [`ONE_MODE`]: the differential tests that use `nscache_mode.sh` but not both modes are
//!    exactly the known programs.
//! 3. No one-mode program (its `.c` file and the local headers it includes) names a function of
//!    [`FAMILIES`]. A program that starts making a lookup fails here and goes back to both modes.
//!
//! The families are wider than the direct callers (they include the functions that reach a
//! caller through another module: glob's and wordexp's tilde expansion, grantpt's group lookup,
//! cuserid), so a name a family lists may reach no lookup in a given call; the check errs towards
//! both modes. Clause: no spec clause (a static check of the test suite, D-16.18).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The library source files that name the cache (`nscache::` or `NSCACHE`), relative to the
/// repository, with how a program reaches the cache through them.
const CALLERS: &[(&str, &str)] = &[
    ("crates/boundary/src/conv.rs", "none: a contract comment only"),
    ("crates/boundary/src/fork.rs", "none: after_fork_child resets the lock and reader count and reads no switch"),
    ("crates/boundary/src/native/consts.rs", "interface: the status constants of niji_nscache_status"),
    ("crates/boundary/src/native/nscache.rs", "interface: niji_nscache_*"),
    ("crates/boundary/src/native/types.rs", "interface: the status structure of niji_nscache_status"),
    ("crates/boundary/src/netdb.rs", "network lookups"),
    ("crates/boundary/src/nscache.rs", "the cache's boundary: start, interface, unshare and setns"),
    ("crates/boundary/src/patterns.rs", "tilde expansion"),
    ("crates/boundary/src/posix/s18_processes.rs", "credentials: unshare, setns"),
    ("crates/boundary/src/resolv.rs", "resolver"),
    ("crates/boundary/src/runtime.rs", "none: process start stores the switch"),
    ("crates/boundary/src/selftest.rs", "none: the library's own unit tests"),
    ("crates/boundary/src/setxid.rs", "credentials: the setxid family, setgroups"),
    ("crates/boundary/src/users.rs", "user and group lookups"),
    ("crates/legacy/src/netdb_compat.rs", "network lookups"),
    ("crates/legacy/src/nscache.rs", "the cache itself"),
    ("crates/legacy/src/nss.rs", "user and group lookups; the NSS engine of every family"),
    ("crates/legacy/src/resolv_compat.rs", "resolver"),
    ("crates/native/src/ffi.rs", "interface: the status structure"),
];

/// The C functions that reach the cache, by family. A name ending in `*` is a prefix.
const FAMILIES: &[(&str, &[&str])] = &[
    (
        "user and group lookups (users.rs, legacy nss.rs)",
        &["getpw*", "getgr*", "getsp*", "getsg*", "setpwent", "setgrent", "setspent", "setsgent", "initgroups", "getlogin*", "__getlogin_r_chk", "cuserid"],
    ),
    (
        "network lookups (netdb.rs, gai_async.rs, netaddr.rs, legacy netdb_compat.rs)",
        &[
            "gethost*", "sethostent", "getaddrinfo*", "getnameinfo", "getnet*", "setnetent", "setnetgrent", "innetgr", "getserv*", "setservent", "getproto*", "setprotoent", "getrpc*", "setrpcent", "getalias*",
            "setaliasent", "ether_hostton", "ether_ntohost", "rcmd*", "rexec*", "ruserok*", "iruserok*", "__nss_*",
        ],
    ),
    ("resolver (resolv.rs, legacy resolv_compat.rs)", &["res_*", "__res_*"]),
    ("tilde expansion (patterns.rs)", &["glob", "glob64", "wordexp"]),
    ("grantpt's group lookup", &["grantpt"]),
    ("credentials (setxid.rs, nscache.rs)", &["setuid", "setgid", "seteuid", "setegid", "setreuid", "setregid", "setresuid", "setresgid", "setgroups", "unshare", "setns"]),
    ("the cache's interface (native/nscache.rs)", &["niji_nscache_*"]),
];

/// The differential programs run with the cache in one mode (D-16.18).
const ONE_MODE: &[&str] = &["aio", "catalogs", "fstree", "mallocdbg", "mntent", "posixipc", "syslog"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn rel(r: &Path, p: &Path) -> String {
    p.strip_prefix(r).unwrap().to_string_lossy().replace('\\', "/")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `word` in `text` with no identifier character before it (and, unless it ends in `::`, after it).
fn has_word(text: &str, word: &str) -> bool {
    let b = text.as_bytes();
    text.match_indices(word).any(|(i, _)| {
        let before = i == 0 || !is_ident(b[i - 1]);
        let end = i + word.len();
        let after = word.ends_with("::") || end == b.len() || !is_ident(b[end]);
        before && after
    })
}

/// The library sources that name the cache.
fn callers(r: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut crates: Vec<_> = std::fs::read_dir(r.join("crates")).unwrap().map(|e| e.unwrap().path()).collect();
    crates.sort();
    for c in crates {
        let src = c.join("src");
        if !src.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        rust_files(&src, &mut files);
        for f in files {
            let t = std::fs::read_to_string(&f).unwrap();
            if has_word(&t, "nscache::") || has_word(&t, "NSCACHE") {
                found.insert(rel(r, &f));
            }
        }
    }
    found
}

/// The C source with comments, string literals and character literals blanked out.
fn code_only(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"/*") {
            let end = src[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
            out.push(b' ');
            i = end;
        } else if b[i..].starts_with(b"//") {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i] == b'"' || b[i] == b'\'' {
            let q = b[i];
            i += 1;
            while i < b.len() && b[i] != q {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            out.push(b' ');
            i += 1;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The program's source and the local headers it includes (`#include "..."`), transitively.
fn sources(p: &Path, seen: &mut BTreeSet<PathBuf>) {
    let p = p.canonicalize().unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    if !seen.insert(p.clone()) {
        return;
    }
    let t = std::fs::read_to_string(&p).unwrap();
    for line in t.lines() {
        let l = line.trim_start();
        if let Some(rest) = l.strip_prefix("#include \"") {
            let name = rest.split('"').next().unwrap();
            sources(&p.parent().unwrap().join(name), seen);
        }
    }
}

fn matches(pattern: &str, name: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == pattern,
    }
}

/// The library symbols of a family a C source names outside comments and literals, with the family.
fn lookups_in(src: &str, syms: &BTreeSet<String>) -> Vec<(String, &'static str)> {
    let code = code_only(src);
    let idents: BTreeSet<&str> = code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).collect();
    let mut out = Vec::new();
    for id in idents {
        if !(syms.contains(id) || id.starts_with("niji_")) {
            continue;
        }
        for (family, names) in FAMILIES {
            if names.iter().any(|p| matches(p, id)) {
                out.push((id.to_owned(), *family));
            }
        }
    }
    out
}

/// The C library's symbols (`spec/symbols.tsv`, first column).
fn symbols(r: &Path) -> BTreeSet<String> {
    let t = std::fs::read_to_string(r.join("spec/symbols.tsv")).unwrap();
    t.lines().filter_map(|l| l.split('\t').next()).filter(|s| !s.is_empty() && !s.starts_with('#')).map(str::to_owned).collect()
}

/// The `*_products.rs` tests that use `nscache_mode.sh` in one mode: their program names.
fn one_mode_tests(r: &Path) -> BTreeSet<String> {
    let dir = r.join("crates/core/tests");
    let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    let mut one = BTreeSet::new();
    for n in names {
        let Some(stem) = n.strip_suffix("_products.rs") else { continue };
        let text = std::fs::read_to_string(dir.join(&n)).unwrap();
        // The code only: a comment may name either mode.
        let t: String = text.lines().map(|l| l.split("//").next().unwrap_or("")).collect::<Vec<_>>().join("\n");
        if !t.contains("nscache_mode.sh") {
            continue;
        }
        if !(t.contains("\"on\"") && t.contains("\"off\"")) {
            one.insert(stem.to_owned());
        }
    }
    one
}

#[test]
fn the_scan_sees_calls_and_skips_comments_and_literals() {
    // no spec clause: D-16.18 (the scan behind no_one_mode_program_makes_a_lookup).
    let syms = symbols(&root());
    let sample = "/* getpwnam(n) */\n// gethostbyname(h)\nconst char *s = \" getaddrinfo(\\\"x\\\")\";\nchar q = '\"'; int res_len = 0;\nstruct group *g = getgrnam(name);\nint e = getuid() + glob(p, 0, 0, &gl) + res_len;\n";
    let found: Vec<String> = lookups_in(sample, &syms).into_iter().map(|(id, _)| id).collect();
    assert_eq!(found, ["getgrnam", "glob"], "scan of the sample");
}

#[test]
fn the_cache_has_only_the_known_callers() {
    // no spec clause: D-16.18 (a new caller must name the C functions that reach it).
    let r = root();
    let found = callers(&r);
    let known: BTreeSet<String> = CALLERS.iter().map(|(f, _)| (*f).to_owned()).collect();
    let new: Vec<_> = found.difference(&known).collect();
    let gone: Vec<_> = known.difference(&found).collect();
    assert!(
        new.is_empty() && gone.is_empty(),
        "the name-service cache's callers changed: new {new:?}, gone {gone:?}. Add the C functions that reach a new caller to FAMILIES, then update CALLERS (D-16.18)"
    );
}

#[test]
fn every_lookup_family_name_is_a_library_symbol() {
    // no spec clause: D-16.18 (a misspelt family name would match nothing and check nothing).
    let r = root();
    let syms = symbols(&r);
    for (family, names) in FAMILIES {
        for p in *names {
            if p.starts_with("niji_") {
                continue;
            }
            assert!(syms.iter().any(|s| matches(p, s)), "{p} ({family}) names no symbol of spec/symbols.tsv");
        }
    }
}

#[test]
fn the_one_mode_tests_are_the_known_programs() {
    // no spec clause: D-16.18.
    let r = root();
    let found = one_mode_tests(&r);
    let known: BTreeSet<String> = ONE_MODE.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(found, known, "the differential tests running the name-service cache in one mode changed: check the program with no_one_mode_program_makes_a_lookup and update ONE_MODE (D-16.18)");
}

#[test]
fn no_one_mode_program_makes_a_lookup() {
    // no spec clause: D-16.18 (a program that reaches the cache runs with it on and off, D-16.10).
    let r = root();
    let syms = symbols(&r);
    let programs: BTreeSet<String> = one_mode_tests(&r).into_iter().chain(ONE_MODE.iter().map(|s| (*s).to_owned())).collect();
    let mut reached = Vec::new();
    for prog in &programs {
        let main = r.join(format!("crates/core/tests/oracle/{prog}_diff.c"));
        assert!(main.exists(), "{prog}: no program {}", main.display());
        let mut files = BTreeSet::new();
        sources(&main, &mut files);
        for f in &files {
            for (id, family) in lookups_in(&std::fs::read_to_string(f).unwrap(), &syms) {
                reached.push(format!("{prog}: {} names {id} ({family})", rel(&r, f)));
            }
        }
    }
    assert!(
        reached.is_empty(),
        "a program run with the name-service cache in one mode reaches the cache; run its test with the cache on and off (D-16.10) and drop it from ONE_MODE:\n{}",
        reached.join("\n")
    );
}
