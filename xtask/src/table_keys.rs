//! `table-keys` (part of `cargo xtask static`, HEADERS-FRAGMENTS): every hand-edited table under `headers/` and
//! `spec/abi/` declares its key in `spec/tools/table_keys.tsv` (path, key columns, sort, unit; the format is
//! documented at the top of that file), and no table has two records with the same key. A story that adds a row
//! another story already added fails here, on its own branch, instead of landing a duplicate; merge tooling reads
//! the same file to merge a table row by row.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// The declarations, relative to the repository root.
const KEYS: &str = "spec/tools/table_keys.tsv";
/// Directories whose `*.tsv` tables (at any depth) must each have a line in [`KEYS`].
const SCANNED: [&str; 2] = ["headers", "spec/abi"];

/// One line of [`KEYS`].
#[derive(Debug, Clone, PartialEq)]
struct Decl {
    path: String,
    key: Vec<String>,
    sorted: bool,
    block: bool,
    line: usize,
}

/// The non-comment lines of a table: `#` lines and blank lines are comments. Each with its 1-based line number.
fn data_lines(text: &str) -> Vec<(usize, &str)> {
    text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#')).map(|(i, l)| (i + 1, l)).collect()
}

/// Parse [`KEYS`]; problems are `table_keys.tsv:<line>: ...`.
fn parse_keys(text: &str) -> (Vec<Decl>, Vec<String>) {
    let mut out = Vec::new();
    let mut errs = Vec::new();
    let lines = data_lines(text);
    let Some(((_, head), rows)) = lines.split_first() else {
        return (out, vec![format!("{KEYS}: no header line")]);
    };
    if *head != "path\tkey\tsort\tunit" {
        errs.push(format!("{KEYS}: header is {head:?}, not path, key, sort, unit"));
        return (out, errs);
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (n, l) in rows {
        let c: Vec<&str> = l.split('\t').collect();
        if c.len() != 4 || c.iter().any(|x| x.is_empty()) {
            errs.push(format!("{KEYS}:{n}: {} cells, want 4 non-empty (path, key, sort, unit)", c.len()));
            continue;
        }
        let sorted = match c[2] {
            "key" => true,
            "-" => false,
            s => {
                errs.push(format!("{KEYS}:{n}: sort is {s:?}, not `key` or `-`"));
                continue;
            }
        };
        let block = match c[3] {
            "block" => true,
            "row" => false,
            u => {
                errs.push(format!("{KEYS}:{n}: unit is {u:?}, not `row` or `block`"));
                continue;
            }
        };
        if let Some(prev) = seen.insert(c[0].to_string(), *n) {
            errs.push(format!("{KEYS}:{n}: {} is also on line {prev}", c[0]));
            continue;
        }
        out.push(Decl { path: c[0].to_string(), key: c[1].split(',').map(String::from).collect(), sorted, block, line: *n });
    }
    (out, errs)
}

/// Check one table's text against its declaration; problems are `<path>:<line>: ...`.
fn check_table(d: &Decl, text: &str) -> Vec<String> {
    let mut errs = Vec::new();
    let lines = data_lines(text);
    let Some(((_, head), rows)) = lines.split_first() else {
        return vec![format!("{}: no header line", d.path)];
    };
    let head: Vec<&str> = head.split('\t').collect();
    let mut idx = Vec::new();
    for k in &d.key {
        match head.iter().position(|h| h == k) {
            Some(i) => idx.push(i),
            None => errs.push(format!("{KEYS}:{}: {}: no column {k:?} (header: {})", d.line, d.path, head.join(", "))),
        }
    }
    if !errs.is_empty() {
        return errs;
    }
    let what = if d.block { "block" } else { "row" };
    // The records: each row (unit row), or each run of rows with one key (unit block), with its first line.
    let mut recs: Vec<(Vec<&str>, usize)> = Vec::new();
    for (n, l) in rows {
        let cells: Vec<&str> = l.split('\t').collect();
        let key: Vec<&str> = idx.iter().map(|&i| cells.get(i).copied().unwrap_or("")).collect();
        if d.block && recs.last().is_some_and(|(k, _)| *k == key) {
            continue;
        }
        recs.push((key, *n));
    }
    let mut first: BTreeMap<&Vec<&str>, usize> = BTreeMap::new();
    for (key, n) in &recs {
        if let Some(prev) = first.get(key) {
            errs.push(format!("{}:{n}: key {} ({}) is also the {what} on line {prev}", d.path, key.join(" | "), d.key.join(", ")));
        } else {
            first.insert(key, *n);
        }
    }
    if d.sorted {
        if let Some(w) = recs.windows(2).find(|w| w[0].0 >= w[1].0) {
            errs.push(format!("{}:{}: key {} is not after {} (line {}): the table is declared sorted by key", d.path, w[1].1, w[1].0.join(" | "), w[0].0.join(" | "), w[0].1));
        }
    }
    errs
}

/// Every `*.tsv` below `dir` (relative to `root`), at any depth.
fn tables_below(root: &Path, dir: &str, out: &mut Vec<String>) -> Result<(), String> {
    let at = root.join(dir);
    for e in fs::read_dir(&at).map_err(|e| format!("{}: {e}", at.display()))? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().into_owned();
        let rel = format!("{dir}/{name}");
        if e.file_type().map_err(|e| e.to_string())?.is_dir() {
            tables_below(root, &rel, out)?;
        } else if name.ends_with(".tsv") {
            out.push(rel);
        }
    }
    Ok(())
}

/// The tables found with no line in [`KEYS`].
fn undeclared(decls: &[Decl], tables: &[String]) -> Vec<String> {
    tables.iter().filter(|t| !decls.iter().any(|d| &d.path == *t)).map(|t| format!("{t}: no line in {KEYS} (declare its key columns there)")).collect()
}

/// The static check: every problem on stderr, one error naming how many.
pub(crate) fn check(root: &Path) -> Result<(), String> {
    let text = fs::read_to_string(root.join(KEYS)).map_err(|e| format!("{KEYS}: {e}"))?;
    let (decls, mut errs) = parse_keys(&text);
    let mut tables = Vec::new();
    for d in SCANNED {
        tables_below(root, d, &mut tables)?;
    }
    tables.sort();
    errs.extend(undeclared(&decls, &tables));
    for d in &decls {
        match fs::read_to_string(root.join(&d.path)) {
            Ok(t) => errs.extend(check_table(d, &t)),
            Err(e) => errs.push(format!("{KEYS}:{}: {}: {e}", d.line, d.path)),
        }
    }
    for e in &errs {
        eprintln!("table-keys: {e}");
    }
    if errs.is_empty() { Ok(()) } else { Err(format!("{} problem(s) in the tables of {KEYS}", errs.len())) }
}

#[cfg(test)]
mod tests {
    //! no spec clause: the table-key declarations of spec/tools/table_keys.tsv (HEADERS-FRAGMENTS); each test
    //! breaks one rule of the check.
    use super::*;

    fn decl(key: &str, sorted: bool, block: bool) -> Decl {
        Decl { path: "t.tsv".into(), key: key.split(',').map(String::from).collect(), sorted, block, line: 9 }
    }

    #[test]
    fn distinct_keys_pass_and_comments_are_skipped() {
        let t = "# c\na\tb\tc\nx\t1\tz\n\n# c\nx\t2\tz\ny\t1\n";
        assert!(check_table(&decl("a,b", true, false), t).is_empty());
    }

    #[test]
    fn a_repeated_row_key_fails_even_with_other_cells() {
        let e = check_table(&decl("a", false, false), "a\tb\nx\t1\ny\t2\nx\t3\n");
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].starts_with("t.tsv:4: key x (a) is also the row on line 2"), "{e:?}");
    }

    #[test]
    fn a_missing_cell_is_an_empty_key_part() {
        let e = check_table(&decl("a,b", false, false), "a\tb\nx\nx\t\n");
        assert_eq!(e.len(), 1, "{e:?}");
    }

    #[test]
    fn a_block_may_repeat_inside_but_not_start_again() {
        let ok = "a\tb\nx\t1\nx\t1\ny\t1\n";
        assert!(check_table(&decl("a", false, true), ok).is_empty());
        let e = check_table(&decl("a", false, true), "a\tb\nx\t1\ny\t1\nx\t2\n");
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("is also the block on line 2"), "{e:?}");
    }

    #[test]
    fn a_sorted_table_out_of_order_fails() {
        let e = check_table(&decl("a", true, false), "a\nb\na\n");
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("declared sorted"), "{e:?}");
        assert!(check_table(&decl("a", false, false), "a\nb\na\n").is_empty());
    }

    #[test]
    fn an_unknown_key_column_fails() {
        let e = check_table(&decl("a,q", false, false), "a\tb\nx\t1\n");
        assert!(e.len() == 1 && e[0].contains("no column \"q\""), "{e:?}");
    }

    #[test]
    fn a_table_without_a_declaration_fails() {
        let e = undeclared(&[decl("a", false, false)], &["t.tsv".into(), "u.tsv".into()]);
        assert!(e.len() == 1 && e[0].starts_with("u.tsv: no line in"), "{e:?}");
    }

    #[test]
    fn the_declarations_are_checked() {
        let (d, e) = parse_keys("# doc\npath\tkey\tsort\tunit\nt.tsv\ta,b\tkey\trow\nu.tsv\ta\t-\tblock\n");
        assert!(e.is_empty(), "{e:?}");
        assert_eq!(d.len(), 2);
        assert!(d[0].sorted && !d[0].block && d[0].key == ["a", "b"] && d[0].line == 3);
        let (_, e) = parse_keys("path\tkey\tsort\tunit\nt.tsv\ta\tup\trow\nt.tsv\ta\t-\trow\nt.tsv\ta\t-\trow\nv.tsv\ta\t-\n");
        assert_eq!(e.len(), 3, "{e:?}");
        assert!(e[0].contains("sort is \"up\"") && e[1].contains("also on line 3") && e[2].contains("3 cells"), "{e:?}");
        let (_, e) = parse_keys("path\tkey\n");
        assert_eq!(e.len(), 1, "{e:?}");
    }
}
