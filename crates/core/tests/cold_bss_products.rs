//! D-X.22 (PAGEFAULTS-4): in the shipped `libc.a`, every zero-initialised table of 1 KiB or more
//! sits in a `.lbss.*` section, so a static program's linker places it after the small state
//! words of `.bss` and start-up writes touch few pages.

#![allow(clippy::unwrap_used, missing_docs)]

#[test]
fn abi_libc_a_cold_bss_tables_are_lbss() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let archive = root.join("target/niji/libc.a");
    assert!(archive.exists(), "the products are missing: run `cargo xtask so` (`cargo xtask test` does)");
    let o = std::process::Command::new("readelf").arg("-SW").arg(&archive).output().unwrap();
    assert!(o.status.success(), "readelf -SW libc.a: {}", String::from_utf8_lossy(&o.stderr));
    let text = String::from_utf8_lossy(&o.stdout);
    let (mut lbss, mut big_bss) = (0, Vec::new());
    for line in text.lines() {
        let Some((_, after)) = line.split_once(']') else { continue };
        let f: Vec<&str> = after.split_whitespace().collect();
        let [name, "NOBITS", _, _, size, ..] = f.as_slice() else { continue };
        let size = u64::from_str_radix(size, 16).unwrap();
        if name.starts_with(".lbss.") {
            assert!(size >= 1024, "{name} is {size} bytes: small state words belong in .bss");
            lbss += 1;
        } else if name.starts_with(".bss.") && size >= 1024 {
            big_bss.push(format!("{name} ({size} bytes)"));
        }
    }
    assert!(big_bss.is_empty(), "tables of 1 KiB or more left in .bss: {big_bss:?}");
    // The library has such tables (thread keys, the heap's reserve, locale interning): none
    // moved means the step did not run.
    assert!(lbss >= 5, "only {lbss} .lbss sections");
}

/// LAYOUT-LDSO-WINDOW: in the shipped interpreter, every zero-initialised object of 4 KiB or more lies
/// in `.lbss`, after the small state words of `.bss` that start-up writes. A 20 KiB table among those
/// words splits them over one more page: one more zero-fill fault in every dynamic program. 4 KiB, not
/// libc's 1 KiB: the start-up object list and the fini hooks (under 4 KiB) are written by start-up.
/// No spec clause (layout of the products).
#[test]
fn abi_ldso_cold_bss_objects_are_lbss() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ldso = root.join("target/niji/ld-linux-x86-64.so.2");
    assert!(ldso.exists(), "the products are missing: run `cargo xtask so` (`cargo xtask test` does)");
    let run = |flag: &str| {
        let o = std::process::Command::new("readelf").arg(flag).arg(&ldso).output().unwrap();
        assert!(o.status.success(), "readelf {flag} ld.so: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).into_owned()
    };
    // Section index -> name, for the zero-filled sections.
    let mut nobits = std::collections::BTreeMap::new();
    for line in run("-SW").lines() {
        let Some((idx, after)) = line.split_once(']') else { continue };
        let Some(idx) = idx.trim_start().strip_prefix('[').and_then(|i| i.trim().parse::<u32>().ok()) else { continue };
        let f: Vec<&str> = after.split_whitespace().collect();
        if let [name, "NOBITS", ..] = f.as_slice() {
            nobits.insert(idx.to_string(), name.to_string());
        }
    }
    let symbols = run("-sW");
    assert!(symbols.contains("'.symtab'"), "ld.so has no .symtab: the objects cannot be told apart");
    let (mut lbss, mut big_bss) = (0, Vec::new());
    for line in symbols.lines() {
        // Num: Value Size Type Bind Vis Ndx Name
        let f: Vec<&str> = line.split_whitespace().collect();
        let [_, _, size, "OBJECT", _, _, ndx, name, ..] = f.as_slice() else { continue };
        let size: u64 = if let Some(h) = size.strip_prefix("0x") { u64::from_str_radix(h, 16).unwrap() } else { size.parse().unwrap() };
        match nobits.get(*ndx).map(String::as_str) {
            Some(".lbss") if size >= 4096 => lbss += 1,
            Some(".bss") if size >= 4096 => big_bss.push(format!("{name} ({size} bytes)")),
            _ => {}
        }
    }
    assert!(big_bss.is_empty(), "ld.so objects of 4 KiB or more left in .bss among the start-up words: {big_bss:?}");
    // The interpreter has such tables (the thread module table, the readers table): none in .lbss
    // means the step did not run.
    assert!(lbss >= 2, "only {lbss} objects of 4 KiB or more in ld.so's .lbss");
}
