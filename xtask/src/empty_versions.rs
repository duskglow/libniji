//! The reference's version definitions that no exported symbol carries (`spec/abi/empty-versions.tsv`,
//! from `spec/tools/abi_extract.py`; SYS-ABI-DT-RELR). `versions.tsv` has rows for symbols only, so
//! these nodes have no row there, yet a program can require one: the link editor records a
//! requirement on `libc.so.6`'s `GLIBC_ABI_DT_RELR` when it packs relative relocations (`DT_RELR`),
//! so that a C library that cannot apply them refuses the program at start-up. The product defines
//! each node it honours ([`HONOURED`]) in its version script, and [`check`] reads the linked products'
//! version definitions back.

use std::fs;
use std::path::Path;

/// The table, relative to the repository root.
pub const TABLE: &str = "spec/abi/empty-versions.tsv";

/// The symbol-less nodes libniji defines, with why defining it is honest. A table row missing here
/// stops the build: a node that stands for a feature needs a ruling (define it, or leave it out so
/// that programs which need the feature are refused), never a silent definition.
pub const HONOURED: &[(&str, &str, &str)] = &[
    ("libc.so.6", "GLIBC_ABI_DT_RELR", "the loader applies DT_RELR to every object it loads (crates/ld/src/reloc.rs) and in its own start (boot.rs)"),
    ("libm.so.6", "GLIBC_2.4", "an ABI level with no symbol: every libniji libm.so.6 has all of it"),
    ("ld-linux-x86-64.so.2", "GLIBC_2.4", "an ABI level with no symbol: every libniji loader has all of it"),
];

/// `(library, version, parent)` of every table row; an error for a row [`HONOURED`] does not list.
pub fn rows(root: &Path) -> Result<Vec<(String, String, String)>, String> {
    let text = fs::read_to_string(root.join(TABLE)).map_err(|e| format!("{TABLE}: {e}"))?;
    parse(&text)
}

fn parse(text: &str) -> Result<Vec<(String, String, String)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        let [lib, ver, parent] = f.as_slice() else {
            return Err(format!("{TABLE} line {}: not library, version, parent", i + 1));
        };
        if !HONOURED.iter().any(|(l, v, _)| l == lib && v == ver) {
            return Err(format!(
                "{TABLE}: {lib} {ver} is a symbol-less version node libniji does not define; whether it stands for a feature libniji supports is a ruling (xtask/src/empty_versions.rs HONOURED)"
            ));
        }
        out.push(((*lib).to_string(), (*ver).to_string(), (*parent).to_string()));
    }
    Ok(out)
}

/// `(version, parent)` of the symbol-less nodes library `lib` defines.
pub fn nodes(root: &Path, lib: &str) -> Result<Vec<(String, String)>, String> {
    Ok(rows(root)?.into_iter().filter(|(l, _, _)| l == lib).map(|(_, v, p)| (v, p)).collect())
}

/// `(name, flags, first parent)` of each version definition in `readelf -V -W` text.
fn definitions(readelf_v: &str) -> Vec<(String, String, Option<String>)> {
    let mut out: Vec<(String, String, Option<String>)> = Vec::new();
    let mut inside = false;
    for line in readelf_v.lines() {
        if line.contains("Version definition section") {
            inside = true;
        } else if line.contains("section '") {
            inside = false;
        } else if inside && line.contains("Rev:") {
            let flags = line.split_once("Flags: ").and_then(|(_, r)| r.split_whitespace().next()).unwrap_or("");
            if let Some((_, name)) = line.split_once("Name: ") {
                out.push((name.trim().to_string(), flags.to_string(), None));
            }
        } else if inside && line.contains("Parent 1: ") {
            if let (Some(last), Some((_, p))) = (out.last_mut(), line.split_once("Parent 1: ")) {
                last.2 = Some(p.trim().to_string());
            }
        }
    }
    out
}

/// What is wrong with the table's nodes in `lib`, from its `readelf -V` text: a node it does not
/// define, or one whose flags or parent differ from the reference's (flags `none`: a WEAK flag marks
/// a definition the link editor saw no symbol or pattern for, which the reference's nodes are not).
fn missing(rows: &[(String, String, String)], lib: &str, readelf_v: &str) -> Vec<String> {
    let defined = definitions(readelf_v);
    let mut out = Vec::new();
    for (_, v, parent) in rows.iter().filter(|(l, _, _)| l == lib) {
        match defined.iter().find(|(n, flags, _)| n == v && flags != "BASE") {
            None => out.push(format!("{lib} does not define the version node {v} ({TABLE}): a program that requires it is refused at start-up")),
            Some((_, flags, p)) => {
                if flags != "none" {
                    out.push(format!("{lib} defines the version node {v} with flags {flags}; the reference's has none ({TABLE})"));
                }
                if p.as_deref() != Some(parent.as_str()) {
                    out.push(format!("{lib} defines the version node {v} with parent {}; the reference's is {parent} ({TABLE})", p.as_deref().unwrap_or("(none)")));
                }
            }
        }
    }
    out
}

/// Every table node is a version definition of the linked product in `release`. The number checked.
pub fn check(root: &Path, release: &Path) -> Result<usize, String> {
    let rows = rows(root)?;
    let mut errors = Vec::new();
    let mut libs: Vec<&str> = rows.iter().map(|(l, _, _)| l.as_str()).collect();
    libs.dedup();
    for lib in libs {
        let v = crate::placeholders::readelf(&["-V", "-W"], &release.join(lib))?;
        errors.extend(missing(&rows, lib, &v));
    }
    if errors.is_empty() { Ok(rows.len()) } else { Err(errors.join("\n")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // no spec clause: the table's rule and the export check over readelf text (SYS-ABI-DT-RELR).
    const V: &str = "Version definition section '.gnu.version_d' contains 3 entries:\n  Addr: 0x0000000000000400  Offset: 0x000400  Link: 4 (.dynstr)\n  \
        000000: Rev: 1  Flags: BASE  Index: 1  Cnt: 1  Name: libc.so.6\n  \
        0x001c: Rev: 1  Flags: none  Index: 2  Cnt: 1  Name: GLIBC_2.39\n  \
        0x0038: Rev: 1  Flags: none  Index: 3  Cnt: 2  Name: GLIBC_ABI_DT_RELR\n  0x0054: Parent 1: GLIBC_2.39\n\n\
        Version needs section '.gnu.version_r' contains 1 entry:\n  000000: Version: 1  File: ld-linux-x86-64.so.2  Cnt: 1\n  \
        0x0010:   Name: GLIBC_ABI_DT_RELR  Flags: none  Version: 4\n";

    fn row(l: &str, v: &str) -> (String, String, String) {
        (l.to_string(), v.to_string(), "GLIBC_2.39".to_string())
    }

    #[test]
    fn a_defined_node_passes_and_a_missing_one_is_named() {
        assert!(missing(&[row("libc.so.6", "GLIBC_ABI_DT_RELR")], "libc.so.6", V).is_empty());
        // A requirement (version needs) is not a definition.
        let only_needed = V.replace("Name: GLIBC_ABI_DT_RELR\n  0x0054", "Name: GLIBC_2.40\n  0x0054");
        let m = missing(&[row("libc.so.6", "GLIBC_ABI_DT_RELR")], "libc.so.6", &only_needed);
        assert_eq!(m.len(), 1, "{m:?}");
        assert!(m[0].contains("does not define the version node GLIBC_ABI_DT_RELR"), "{m:?}");
    }

    // no spec clause: the product's definition matches the reference's readelf -V (flags none, parent).
    #[test]
    fn a_weak_node_or_another_parent_is_named() {
        let weak = V.replace("Flags: none  Index: 3", "Flags: WEAK  Index: 3");
        let m = missing(&[row("libc.so.6", "GLIBC_ABI_DT_RELR")], "libc.so.6", &weak);
        assert_eq!(m.len(), 1, "{m:?}");
        assert!(m[0].contains("with flags WEAK"), "{m:?}");
        let other = V.replace("Parent 1: GLIBC_2.39", "Parent 1: GLIBC_2.41");
        let m = missing(&[row("libc.so.6", "GLIBC_ABI_DT_RELR")], "libc.so.6", &other);
        assert_eq!(m.len(), 1, "{m:?}");
        assert!(m[0].contains("with parent GLIBC_2.41; the reference's is GLIBC_2.39"), "{m:?}");
        let orphan = V.replace("\n  0x0054: Parent 1: GLIBC_2.39", "");
        let m = missing(&[row("libc.so.6", "GLIBC_ABI_DT_RELR")], "libc.so.6", &orphan);
        assert!(m.len() == 1 && m[0].contains("with parent (none)"), "{m:?}");
    }

    #[test]
    fn a_node_not_honoured_is_refused() {
        let ok = "library\tversion\tparent\nlibc.so.6\tGLIBC_ABI_DT_RELR\tGLIBC_2.39\n";
        assert_eq!(parse(ok).unwrap().len(), 1);
        let err = parse("library\tversion\tparent\nlibc.so.6\tGLIBC_ABI_FUTURE\tGLIBC_2.39\n").unwrap_err();
        assert!(err.contains("ruling"), "{err}");
    }
}
