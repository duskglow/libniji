//! `cargo xtask abi-versions` (part of `cargo xtask static`): `spec/abi/versions.tsv` and `versions-aarch64.tsv`
//! are what `spec/tools/abi_extract.py` writes, the reference's rows merged with `spec/abi/versions-additions.tsv`.
//!
//! The extractor reads the reference's dynamic symbol tables, which give only the reference's exports. The exports
//! libniji adds beyond them (GLIBC_2.41's `sched_getattr` and `sched_setattr`, owner ruling R3, D-24.12) are rows of
//! the additions file, each naming the decision record that adds it, and the extractor merges them in: each after the
//! last row of its library, in file order. The extractor also writes `versions{,-aarch64}.sha256`, the sha256 and
//! count of the reference rows alone, so this check can redo the merge without the reference system:
//!
//! * every addition row has the shape of a versions row plus `arch` (`all`, `x86_64`, `aarch64`) and `decision`,
//!   a decision id whose record exists under `records/decisions/`, and repeats neither another addition nor a
//!   reference row (library, symbol, version);
//! * the committed table minus its addition rows hashes to the recorded digest: a row written into the table by hand
//!   (or dropped from it) fails here, naming the additions file;
//! * the committed table equals the merge of those reference rows with the additions, byte for byte: a missing
//!   additions file or row (the export would vanish at the next regeneration) or a misplaced row fails.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub const ADDITIONS: &str = "spec/abi/versions-additions.tsv";
const VERSIONS_HEAD: &str = "library\tsymbol\tversion\tbinding\ttype\tscope\tsize";
const ADDITIONS_HEAD: &str = "arch\tlibrary\tsymbol\tversion\tbinding\ttype\tscope\tsize\tdecision";
/// (arch, versions table, digest of its reference rows).
const TABLES: [(&str, &str, &str); 2] = [("x86_64", "spec/abi/versions.tsv", "spec/abi/versions.sha256"), ("aarch64", "spec/abi/versions-aarch64.tsv", "spec/abi/versions-aarch64.sha256")];

/// One row of the additions file: its arch (`all` or one), its versions row (7 columns, tab-joined) and decision.
#[derive(Debug, Clone)]
pub struct Addition {
    pub arch: String,
    pub row: String,
    pub decision: String,
}

impl Addition {
    fn library(&self) -> &str {
        self.row.split('\t').next().unwrap_or("")
    }
    fn key(&self) -> String {
        key(&self.row)
    }
}

/// Library, symbol and version of a versions row.
fn key(row: &str) -> String {
    row.split('\t').take(3).collect::<Vec<_>>().join("\t")
}

fn is_decision_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("D-") else { return false };
    let Some((sec, n)) = rest.split_once('.') else { return false };
    !sec.is_empty() && sec.bytes().all(|b| b.is_ascii_digit() || b.is_ascii_uppercase()) && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())
}

/// The additions file's rows, or every error in its shape. `decision_exists` says whether a decision id has a record.
pub fn parse_additions(text: &str, decision_exists: &dyn Fn(&str) -> bool) -> Result<Vec<Addition>, Vec<String>> {
    let mut lines = text.lines();
    let mut errors = Vec::new();
    if lines.next() != Some(ADDITIONS_HEAD) {
        return Err(vec![format!("{ADDITIONS}: the header must be `{}`", ADDITIONS_HEAD.replace('\t', " "))]);
    }
    let mut out: Vec<Addition> = Vec::new();
    for (i, line) in lines.enumerate() {
        let n = i + 2;
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 9 {
            errors.push(format!("{ADDITIONS}:{n}: {} columns, want 9 ({})", f.len(), ADDITIONS_HEAD.replace('\t', " ")));
            continue;
        }
        let what = format!("{ADDITIONS}:{n}: {} {}@{}", f[1], f[2], f[3]);
        if !matches!(f[0], "all" | "x86_64" | "aarch64") {
            errors.push(format!("{what}: arch `{}` is none of all, x86_64, aarch64", f[0]));
        }
        if f[1].is_empty() || f[2].is_empty() || f[3].is_empty() {
            errors.push(format!("{what}: library, symbol and version must not be empty"));
        }
        if !matches!(f[4], "default" | "compat") {
            errors.push(format!("{what}: binding `{}` is neither default nor compat", f[4]));
        }
        if !is_decision_id(f[8]) {
            errors.push(format!("{what}: names no decision (the decision column must hold the id of the decision record that adds the export, D-<section>.<n>)"));
        } else if !decision_exists(f[8]) {
            errors.push(format!("{what}: decision {} has no record under records/decisions/", f[8]));
        }
        let a = Addition { arch: f[0].to_string(), row: f[1..8].join("\t"), decision: f[8].to_string() };
        if let Some(prev) = out.iter().find(|p| p.key() == a.key() && (p.arch == a.arch || p.arch == "all" || a.arch == "all")) {
            errors.push(format!("{what}: repeats the addition of {} for {}", prev.key().replace('\t', " "), prev.arch));
        }
        out.push(a);
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// The reference rows with each addition after the last row of its library (at the end for a library they lack), in
/// the additions' order: the extractor's `merged`.
pub fn merge(reference: &[&str], adds: &[&Addition]) -> Vec<String> {
    let mut out: Vec<String> = reference.iter().map(|s| s.to_string()).collect();
    for a in adds {
        let at = out.iter().rposition(|r| r.split('\t').next() == Some(a.library())).map_or(out.len(), |i| i + 1);
        out.insert(at, a.row.clone());
    }
    out
}

/// Everything wrong with one committed versions table (`table` text, its recorded digest line `digest`) against the
/// additions of `arch`. `sha` hashes a text.
pub fn table_errors(arch: &str, file: &str, table: &str, digest: &str, adds: &[Addition], sha: &dyn Fn(&str) -> Result<String, String>) -> Vec<String> {
    let mut errors = Vec::new();
    let mine: Vec<&Addition> = adds.iter().filter(|a| a.arch == "all" || a.arch == arch).collect();
    let mut lines = table.lines();
    if lines.next() != Some(VERSIONS_HEAD) {
        return vec![format!("{file}: the header must be `{}`", VERSIONS_HEAD.replace('\t', " "))];
    }
    let rows: Vec<&str> = lines.collect();
    let added: BTreeSet<&str> = mine.iter().map(|a| a.row.as_str()).collect();
    let reference: Vec<&str> = rows.iter().copied().filter(|r| !added.contains(r)).collect();
    let ref_keys: BTreeSet<String> = reference.iter().map(|r| key(r)).collect();
    for a in &mine {
        if ref_keys.contains(&a.key()) {
            errors.push(format!("{ADDITIONS}: {} duplicates a reference row of {file} (an addition is only for an export the reference lacks)", a.key().replace('\t', " ")));
        }
        if !rows.contains(&a.row.as_str()) {
            errors.push(format!("{file} lacks the addition {} ({}): regenerate it (python3 spec/tools/abi_extract.py --versions-only, or --arch aarch64 ... --symbols-only)", a.key().replace('\t', " "), a.decision));
        }
    }
    let ref_text: String = std::iter::once(VERSIONS_HEAD).chain(reference.iter().copied()).map(|l| format!("{l}\n")).collect();
    let mut d = digest.split('\t');
    let (want, count) = (d.next().unwrap_or("").trim(), d.next().unwrap_or("").trim());
    match sha(&ref_text) {
        Ok(got) if got == want && count == reference.len().to_string() => {}
        // The whole table is the reference: the additions in it are reference rows themselves.
        Ok(_) if sha(table).is_ok_and(|all| all == want) && count == rows.len().to_string() => errors.push(format!(
            "{ADDITIONS}: {} duplicate reference rows of {file} (an addition is only for an export the reference lacks)",
            mine.iter().filter(|a| rows.contains(&a.row.as_str())).map(|a| a.key().replace('\t', " ")).collect::<Vec<_>>().join(", ")
        )),
        Ok(_) => errors.push(format!(
            "{file} differs from a fresh generation: its {} rows that are not additions do not match the reference rows the extractor recorded ({count} rows, digest in the .sha256 beside it). Rows libniji exports beyond the reference go in {ADDITIONS} with the decision that adds them, never into {file} by hand; then regenerate",
            reference.len()
        )),
        Err(e) => errors.push(format!("{file}: {e}")),
    }
    let mut want_text: String = std::iter::once(VERSIONS_HEAD.to_string()).chain(merge(&reference, &mine)).collect::<Vec<_>>().join("\n");
    want_text.push('\n');
    if errors.is_empty() && want_text != table {
        errors.push(format!("{file} differs from a fresh generation (an addition row is out of place or the file's line endings differ): regenerate it with spec/tools/abi_extract.py, never edit it by hand ({ADDITIONS})"));
    }
    errors
}

/// The check: the additions file and both versions tables.
pub fn check(root: &Path) -> Result<(), String> {
    let text = fs::read_to_string(root.join(ADDITIONS)).map_err(|e| format!("{ADDITIONS}: {e}; it lists the exports libniji adds beyond the reference (without it a regeneration drops them)"))?;
    let exists = |id: &str| id.split_once('.').is_some_and(|(sec, _)| root.join("records/decisions").join(sec).join(format!("{id}.toml")).is_file());
    let adds = parse_additions(&text, &exists).map_err(|e| e.join("\n"))?;
    let mut errors = Vec::new();
    for (arch, file, digest) in TABLES {
        let table = fs::read_to_string(root.join(file)).map_err(|e| format!("{file}: {e}"))?;
        let dig = fs::read_to_string(root.join(digest)).map_err(|e| format!("{digest}: {e} (written by spec/tools/abi_extract.py)"))?;
        errors.extend(table_errors(arch, file, &table, &dig, &adds, &crate::sha256::sha256_text));
    }
    if errors.is_empty() {
        println!("abi-versions: {} addition rows, versions tables match a fresh generation", adds.len());
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    //! no spec clause: the provenance of spec/abi/versions.tsv (Appendix C.0, "Additions").
    use super::*;

    const HEAD: &str = "library\tsymbol\tversion\tbinding\ttype\tscope\tsize";
    const ADDS: &str = "arch\tlibrary\tsymbol\tversion\tbinding\ttype\tscope\tsize\tdecision\nall\tlibc.so.6\tnew_fn\tGLIBC_2.41\tdefault\tFUNC\tGLOBAL\t\tD-24.12\n";

    fn fake_sha(t: &str) -> Result<String, String> {
        Ok(format!("{:x}", t.len()))
    }
    fn reference() -> String {
        format!("{HEAD}\nlibc.so.6\ta\tGLIBC_2.2.5\tdefault\tFUNC\tGLOBAL\t\nlibc.so.6\tb\tGLIBC_2.2.5\tdefault\tFUNC\tGLOBAL\t\nlibm.so.6\tsin\tGLIBC_2.2.5\tdefault\tFUNC\tGLOBAL\t\n")
    }
    fn digest() -> String {
        format!("{}\t3\n", fake_sha(&reference()).unwrap())
    }
    fn merged() -> String {
        reference().replace("b\tGLIBC_2.2.5\tdefault\tFUNC\tGLOBAL\t\n", "b\tGLIBC_2.2.5\tdefault\tFUNC\tGLOBAL\t\nlibc.so.6\tnew_fn\tGLIBC_2.41\tdefault\tFUNC\tGLOBAL\t\n")
    }
    fn adds(text: &str) -> Vec<Addition> {
        parse_additions(text, &|id| id == "D-24.12").unwrap()
    }

    #[test]
    fn a_merged_table_passes() {
        assert_eq!(table_errors("x86_64", "v.tsv", &merged(), &digest(), &adds(ADDS), &fake_sha), Vec::<String>::new());
    }

    #[test]
    fn a_hand_added_row_fails_naming_the_additions_file() {
        // Just before the addition: the merge alone would place the addition after it again, only the digest sees it.
        let t = merged().replace("libc.so.6\tnew_fn", "libc.so.6\textra\tGLIBC_2.41\tdefault\tFUNC\tGLOBAL\t\nlibc.so.6\tnew_fn");
        let e = table_errors("x86_64", "v.tsv", &t, &digest(), &adds(ADDS), &fake_sha);
        assert!(e.iter().any(|m| m.contains("differs from a fresh generation") && m.contains(ADDITIONS)), "{e:?}");
    }

    #[test]
    fn a_missing_addition_row_fails() {
        let e = table_errors("x86_64", "v.tsv", &reference(), &digest(), &adds(ADDS), &fake_sha);
        assert!(e.iter().any(|m| m.contains("lacks the addition")), "{e:?}");
    }

    #[test]
    fn a_misplaced_addition_fails() {
        let t = format!("{}libc.so.6\tnew_fn\tGLIBC_2.41\tdefault\tFUNC\tGLOBAL\t\n", reference());
        let e = table_errors("x86_64", "v.tsv", &t, &digest(), &adds(ADDS), &fake_sha);
        assert!(e.iter().any(|m| m.contains("out of place")), "{e:?}");
    }

    #[test]
    fn an_addition_duplicating_a_reference_row_fails() {
        // The same row as the reference's, and the same symbol and version with another binding.
        for a in [ADDS.replace("new_fn\tGLIBC_2.41", "a\tGLIBC_2.2.5"), ADDS.replace("new_fn\tGLIBC_2.41\tdefault", "a\tGLIBC_2.2.5\tcompat")] {
            let row = adds(&a)[0].row.clone();
            let t = if a.contains("compat") { merged().replace("libc.so.6\tnew_fn\tGLIBC_2.41\tdefault\tFUNC\tGLOBAL\t", &row) } else { reference() };
            let e = table_errors("x86_64", "v.tsv", &t, &digest(), &adds(&a), &fake_sha);
            assert!(e.iter().any(|m| m.contains("duplicate")), "{a}: {e:?}");
        }
    }

    #[test]
    fn an_addition_for_the_other_arch_is_not_merged() {
        let a = ADDS.replacen("all", "aarch64", 1);
        assert_eq!(table_errors("x86_64", "v.tsv", &reference(), &digest(), &adds(&a), &fake_sha), Vec::<String>::new());
    }

    #[test]
    fn an_addition_without_a_decision_fails() {
        for bad in ["", "R3", "D-99.1"] {
            let a = ADDS.replace("D-24.12", bad);
            let e = parse_additions(&a, &|id| id == "D-24.12").unwrap_err();
            assert!(e.iter().any(|m| m.contains("names no decision") || m.contains("has no record")), "{bad}: {e:?}");
        }
    }

    #[test]
    fn a_repeated_addition_fails() {
        let a = format!("{ADDS}{}", ADDS.lines().nth(1).unwrap());
        assert!(parse_additions(&a, &|_| true).unwrap_err().iter().any(|m| m.contains("repeats the addition")));
    }
}
