//! `cargo xtask abi-prototypes` (part of `cargo xtask static`): `spec/abi/prototypes.tsv` is what
//! `spec/tools/abi_extract.py` writes, the reference's rows merged with `spec/abi/prototypes-additions.tsv`.
//!
//! The extractor reads the reference's headers, which declare only the reference's functions. The prototypes of the
//! functions libniji adds beyond them (GLIBC_2.41's `sched_getattr` and `sched_setattr`, owner ruling R3, D-24.12)
//! are rows of the additions file, each naming the decision record that adds it, and the extractor merges them in:
//! each before the first row whose name sorts after its own. The extractor also writes `prototypes.sha256`, the
//! sha256 and count of the reference rows alone, so this check can redo the merge without the reference system (the
//! same shape as `abi-versions` for the versions tables):
//!
//! * every addition row is a prototypes row (name, header, declaration of that name) plus `decision`, a decision id
//!   whose record exists under `records/decisions/`, and repeats neither another addition nor a reference row's name;
//! * the committed table minus its addition rows hashes to the recorded digest: a row written into the table by hand
//!   (or dropped from it) fails here, naming the additions file;
//! * the committed table equals the merge of those reference rows with the additions, byte for byte.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub const ADDITIONS: &str = "spec/abi/prototypes-additions.tsv";
const TABLE: &str = "spec/abi/prototypes.tsv";
const DIGEST: &str = "spec/abi/prototypes.sha256";
const PROTO_HEAD: &str = "name\theader\tdeclaration";
const ADDITIONS_HEAD: &str = "name\theader\tdeclaration\tdecision";

/// One row of the additions file: its prototypes row (3 columns, tab-joined) and decision.
#[derive(Debug, Clone)]
pub struct Addition {
    pub row: String,
    pub decision: String,
}

impl Addition {
    fn name(&self) -> &str {
        name(&self.row)
    }
}

fn name(row: &str) -> &str {
    row.split('\t').next().unwrap_or("")
}

fn is_ident(s: &str) -> bool {
    s.bytes().next().is_some_and(|b| b == b'_' || b.is_ascii_alphabetic()) && s.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric())
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
        if f.len() != 4 {
            errors.push(format!("{ADDITIONS}:{n}: {} columns, want 4 ({})", f.len(), ADDITIONS_HEAD.replace('\t', " ")));
            continue;
        }
        let what = format!("{ADDITIONS}:{n}: {}", f[0]);
        if !is_ident(f[0]) {
            errors.push(format!("{what}: the name is not a C identifier"));
        }
        if f[1].is_empty() {
            errors.push(format!("{what}: the header must not be empty"));
        }
        if !f[2].contains(&format!(" {} (", f[0])) {
            errors.push(format!("{what}: the declaration does not declare `{} (...)`", f[0]));
        }
        if !crate::abi_versions::is_decision_id(f[3]) {
            errors.push(format!("{what}: names no decision (the decision column must hold the id of the decision record that adds the function, D-<section>.<n>)"));
        } else if !decision_exists(f[3]) {
            errors.push(format!("{what}: decision {} has no record under records/decisions/", f[3]));
        }
        let a = Addition { row: f[..3].join("\t"), decision: f[3].to_string() };
        if out.iter().any(|p| p.name() == a.name()) {
            errors.push(format!("{what}: repeats the addition of {}", a.name()));
        }
        out.push(a);
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// The reference rows with each addition before the first row whose name sorts after its own, in the additions'
/// order: the extractor's `write_prototypes`.
pub fn merge(reference: &[&str], adds: &[Addition]) -> Vec<String> {
    let mut out: Vec<String> = reference.iter().map(|s| s.to_string()).collect();
    for a in adds {
        let at = out.iter().position(|r| name(r) > a.name()).unwrap_or(out.len());
        out.insert(at, a.row.clone());
    }
    out
}

/// Everything wrong with the committed prototypes table (`table` text, its recorded digest line `digest`) against
/// the additions. `sha` hashes a text.
pub fn table_errors(table: &str, digest: &str, adds: &[Addition], sha: &dyn Fn(&str) -> Result<String, String>) -> Vec<String> {
    let mut errors = Vec::new();
    let mut lines = table.lines();
    if lines.next() != Some(PROTO_HEAD) {
        return vec![format!("{TABLE}: the header must be `{}`", PROTO_HEAD.replace('\t', " "))];
    }
    let rows: Vec<&str> = lines.collect();
    let added: BTreeSet<&str> = adds.iter().map(|a| a.row.as_str()).collect();
    let reference: Vec<&str> = rows.iter().copied().filter(|r| !added.contains(r)).collect();
    let ref_names: BTreeSet<&str> = reference.iter().map(|r| name(r)).collect();
    for a in adds {
        if ref_names.contains(a.name()) {
            errors.push(format!("{ADDITIONS}: {} duplicates a reference row of {TABLE} (an addition is only for a function the reference does not declare)", a.name()));
        }
        if !rows.contains(&a.row.as_str()) {
            errors.push(format!("{TABLE} lacks the addition {} ({}): regenerate it (python3 spec/tools/abi_extract.py --prototypes-only)", a.name(), a.decision));
        }
    }
    let ref_text: String = std::iter::once(PROTO_HEAD).chain(reference.iter().copied()).map(|l| format!("{l}\n")).collect();
    let mut d = digest.split('\t');
    let (want, count) = (d.next().unwrap_or("").trim(), d.next().unwrap_or("").trim());
    match sha(&ref_text) {
        Ok(got) if got == want && count == reference.len().to_string() => {}
        // The whole table is the reference: the additions in it are reference rows themselves.
        Ok(_) if sha(table).is_ok_and(|all| all == want) && count == rows.len().to_string() => errors.push(format!(
            "{ADDITIONS}: {} duplicate reference rows of {TABLE} (an addition is only for a function the reference does not declare)",
            adds.iter().filter(|a| rows.contains(&a.row.as_str())).map(|a| a.name().to_string()).collect::<Vec<_>>().join(", ")
        )),
        Ok(_) => errors.push(format!(
            "{TABLE} differs from a fresh generation: its {} rows that are not additions do not match the reference rows the extractor recorded ({count} rows, digest in {DIGEST}). Prototypes of functions libniji adds beyond the reference go in {ADDITIONS} with the decision that adds them, never into {TABLE} by hand; then regenerate",
            reference.len()
        )),
        Err(e) => errors.push(format!("{TABLE}: {e}")),
    }
    let mut want_text: String = std::iter::once(PROTO_HEAD.to_string()).chain(merge(&reference, adds)).collect::<Vec<_>>().join("\n");
    want_text.push('\n');
    if errors.is_empty() && want_text != table {
        errors.push(format!("{TABLE} differs from a fresh generation (an addition row is out of place or the file's line endings differ): regenerate it with spec/tools/abi_extract.py, never edit it by hand ({ADDITIONS})"));
    }
    errors
}

/// The check: the additions file and the prototypes table.
pub fn check(root: &Path) -> Result<(), String> {
    let text = fs::read_to_string(root.join(ADDITIONS)).map_err(|e| format!("{ADDITIONS}: {e}; it lists the prototypes libniji adds beyond the reference (without it a regeneration drops them)"))?;
    let exists = |id: &str| id.split_once('.').is_some_and(|(sec, _)| root.join("records/decisions").join(sec).join(format!("{id}.toml")).is_file());
    let adds = parse_additions(&text, &exists).map_err(|e| e.join("\n"))?;
    let table = fs::read_to_string(root.join(TABLE)).map_err(|e| format!("{TABLE}: {e}"))?;
    let dig = fs::read_to_string(root.join(DIGEST)).map_err(|e| format!("{DIGEST}: {e} (written by spec/tools/abi_extract.py)"))?;
    let errors = table_errors(&table, &dig, &adds, &crate::sha256::sha256_text);
    if errors.is_empty() {
        println!("abi-prototypes: {} addition rows, prototypes.tsv matches a fresh generation", adds.len());
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    //! no spec clause: the provenance of spec/abi/prototypes.tsv (Appendix C.0, "Additions").
    use super::*;

    const HEAD: &str = "name\theader\tdeclaration";
    const ADDS: &str = "name\theader\tdeclaration\tdecision\nnew_fn\tsched.h\textern int new_fn (int)\tD-24.12\n";

    fn fake_sha(t: &str) -> Result<String, String> {
        Ok(format!("{:x}", t.len()))
    }
    fn reference() -> String {
        format!("{HEAD}\na\tstdio.h\textern int a (void)\nnew_\tstdio.h\textern int new_ (void)\nzz\tstdlib.h\textern int zz (void)\n")
    }
    fn digest() -> String {
        format!("{}\t3\n", fake_sha(&reference()).unwrap())
    }
    fn merged() -> String {
        reference().replace("zz\t", "new_fn\tsched.h\textern int new_fn (int)\nzz\t")
    }
    fn adds(text: &str) -> Vec<Addition> {
        parse_additions(text, &|id| id == "D-24.12").unwrap()
    }

    #[test]
    fn a_merged_table_passes() {
        assert_eq!(table_errors(&merged(), &digest(), &adds(ADDS), &fake_sha), Vec::<String>::new());
    }

    #[test]
    fn a_hand_added_row_fails_naming_the_additions_file() {
        // In sorted place, as a hand edit would put it: only the digest sees it.
        let t = merged().replace("new_fn\t", "new_extra\tsched.h\textern int new_extra (void)\nnew_fn\t");
        let e = table_errors(&t, &digest(), &adds(ADDS), &fake_sha);
        assert!(e.iter().any(|m| m.contains("differs from a fresh generation") && m.contains(ADDITIONS)), "{e:?}");
    }

    #[test]
    fn a_missing_addition_row_fails() {
        let e = table_errors(&reference(), &digest(), &adds(ADDS), &fake_sha);
        assert!(e.iter().any(|m| m.contains("lacks the addition")), "{e:?}");
    }

    #[test]
    fn a_misplaced_addition_fails() {
        let t = format!("{}new_fn\tsched.h\textern int new_fn (int)\n", reference());
        let e = table_errors(&t, &digest(), &adds(ADDS), &fake_sha);
        assert!(e.iter().any(|m| m.contains("out of place")), "{e:?}");
    }

    #[test]
    fn an_addition_duplicating_a_reference_row_fails() {
        // The reference's own row, and the same name with another declaration.
        for a in [ADDS.replace("new_fn\tsched.h\textern int new_fn (int)", "a\tstdio.h\textern int a (void)"), ADDS.replace("new_fn\tsched.h\textern int new_fn (int)", "a\tsched.h\textern long a (void)")] {
            let row = adds(&a)[0].row.clone();
            let t = if a.contains("long") { reference().replace("new_\t", &format!("{row}\nnew_\t")) } else { reference() };
            let e = table_errors(&t, &digest(), &adds(&a), &fake_sha);
            assert!(e.iter().any(|m| m.contains("duplicate")), "{a}: {e:?}");
        }
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
    fn a_malformed_addition_fails() {
        for (bad, msg) in [("1x\tsched.h\textern int 1x (int)", "not a C identifier"), ("new_fn\t\textern int new_fn (int)", "header"), ("new_fn\tsched.h\textern int other (int)", "does not declare")] {
            let a = ADDS.replace("new_fn\tsched.h\textern int new_fn (int)", bad);
            let e = parse_additions(&a, &|_| true).unwrap_err();
            assert!(e.iter().any(|m| m.contains(msg)), "{bad}: {e:?}");
        }
    }

    #[test]
    fn a_repeated_addition_fails() {
        let a = format!("{ADDS}{}", ADDS.lines().nth(1).unwrap());
        assert!(parse_additions(&a, &|_| true).unwrap_err().iter().any(|m| m.contains("repeats the addition")));
    }
}
