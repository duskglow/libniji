//! The shadow stack's start-up decision on real objects (D-21.6): `niji_ld::shstk::marked` reads the
//! x86-64 psABI §2.1.7 `GNU_PROPERTY_X86_FEATURE_1_AND` property's SHSTK bit from a shared object the
//! host linker built, and `start_allowed` is the AND over every object of a start, so one unmarked
//! object anywhere in the list (not only the first) keeps the shadow stack off. Needs a C compiler
//! whose linker takes `-z shstk` (else SKIP); no CPU feature is needed.
//!
//! Trace: no spec clause (psABI §2.1.7 program property; the start-up rule is D-21.6, probe-settled).
// ARCH(x86_64): the property is the x86 feature word; the objects are built for the host.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use niji_ld::arch::Native;
use niji_ld::elf::Elf;
use niji_ld::shstk::{marked, start_allowed};

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// Builds `name` from one C function with `flags`, without start files or libraries (the host's
/// own start files would change the AND), or `None` when the compiler is missing or refuses.
fn build(d: &Path, name: &str, flags: &[&str]) -> Option<Vec<u8>> {
    let src = d.join("f.c");
    std::fs::write(&src, "int f(int x) { return x + 1; }\n").unwrap();
    let out = d.join(name);
    let st = Command::new("cc")
        .args(["-shared", "-fPIC", "-nostdlib", "-O1"])
        .args(flags)
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .output()
        .ok()?;
    if !st.status.success() {
        eprintln!("SKIP: cc {flags:?}: {}", String::from_utf8_lossy(&st.stderr));
        return None;
    }
    Some(std::fs::read(out).unwrap())
}

// D-21.6 (no spec clause: psABI §2.1.7): a start enables the shadow stack only when every object is
// marked; an unmarked object in any position of the list vetoes it.
#[test]
fn start_needs_every_object_marked() {
    let d = target().join(format!("ld-shstk-marking-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let Some(m) = build(&d, "m.so", &["-fcf-protection=full", "-Wl,-z,shstk"]) else { return };
    let Some(u) = build(&d, "u.so", &["-fcf-protection=none"]) else { return };
    let (m, u) = (Elf::parse::<Native>(&m).unwrap(), Elf::parse::<Native>(&u).unwrap());
    assert!(marked::<Native>(&m), "-z shstk object not seen as marked");
    assert!(!marked::<Native>(&u), "object without the property seen as marked");
    assert!(start_allowed::<Native>([&m, &m, &m].into_iter()), "all marked: start refused");
    for objs in [[&u, &m, &m], [&m, &u, &m], [&m, &m, &u]] {
        assert!(!start_allowed::<Native>(objs.into_iter()), "an unmarked object did not veto the start");
    }
    std::fs::remove_dir_all(&d).unwrap();
}
