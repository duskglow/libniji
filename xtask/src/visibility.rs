//! Shape and coverage check of the visibility tables (Appendix C.12, SL-HEADERS-2).
//!
//! `spec/abi/ftm-configs.tsv` lists the feature-test configurations; `spec/abi/visibility.tsv` gives, per public
//! header, each visible name, its kind and the configurations it is visible in (`*` = all). Both are written by
//! `spec/tools/header_probe.py` on the reference system. The check (part of `cargo xtask static`) fails when a
//! table's shape is wrong (columns, ids, kinds, a header outside the public inventory, unsorted or duplicate rows,
//! a configuration list that is not a sorted subset of the table) or when a `prototypes.tsv` or `constants.tsv`
//! name is visible in no (header, configuration).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const CONFIGS: &str = "spec/abi/ftm-configs.tsv";
const VISIBILITY: &str = "spec/abi/visibility.tsv";
const INVENTORY: &str = "plan/headers-inventory.tsv";
const CONFIG_HEAD: &str = "id\tlanguage\tflags\tprobes";
const VIS_HEAD: &str = "header\tname\tkind\tconfigs";
/// Kinds in the order the rows of one (header, name) are sorted.
const KINDS: [&str; 9] = ["macro", "typedef", "function", "object", "enumerator", "struct", "union", "enum", "include-error"];
/// The header of the rows for the empty translation unit (compiler predefines).
const EMPTY_UNIT: &str = "-";

fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(f) if f == '_' || f.is_ascii_alphabetic()) && c.all(|x| x == '_' || x.is_ascii_alphanumeric())
}

/// Configuration ids in table order, or the problems found.
fn parse_configs(text: &str, errors: &mut Vec<String>) -> Vec<String> {
    let mut lines = text.lines();
    if lines.next() != Some(CONFIG_HEAD) {
        errors.push(format!("{CONFIGS}: the first line must be `{}`", CONFIG_HEAD.replace('\t', "<TAB>")));
    }
    let mut ids: Vec<String> = Vec::new();
    for (n, line) in lines.enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        let at = format!("{CONFIGS}:{}", n + 2);
        if f.len() != 4 {
            errors.push(format!("{at}: {} fields, expected 4", f.len()));
            continue;
        }
        if f[0].is_empty() || !f[0].chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            errors.push(format!("{at}: id `{}` is not lower-case letters and digits", f[0]));
        }
        if ids.iter().any(|i| i == f[0]) {
            errors.push(format!("{at}: id `{}` repeated", f[0]));
        }
        if f[1] != "c" && f[1] != "c++" {
            errors.push(format!("{at}: language `{}` is not c or c++", f[1]));
        }
        if !f[2].split(' ').any(|x| x.starts_with("-std=")) {
            errors.push(format!("{at}: flags name no -std="));
        }
        ids.push(f[0].to_string());
    }
    if ids.is_empty() {
        errors.push(format!("{CONFIGS}: no configuration"));
    }
    ids
}

/// Problems of the two tables; `public` is the inventory's public header set, `required` the names (with their
/// source table) that must be visible somewhere.
fn check_tables(configs: &str, vis: &str, public: &BTreeSet<String>, required: &[(String, &str)]) -> Vec<String> {
    let mut errors = Vec::new();
    let ids = parse_configs(configs, &mut errors);
    let mut lines = vis.lines();
    if lines.next() != Some(VIS_HEAD) {
        errors.push(format!("{VISIBILITY}: the first line must be `{}`", VIS_HEAD.replace('\t', "<TAB>")));
    }
    let mut prev: Option<(String, String, usize)> = None;
    let mut visible: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for (n, line) in lines.enumerate() {
        let at = format!("{VISIBILITY}:{}", n + 2);
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 4 {
            errors.push(format!("{at}: {} fields, expected 4", f.len()));
            continue;
        }
        let (header, name, kind, cfgs) = (f[0], f[1], f[2], f[3]);
        if header != EMPTY_UNIT && !public.contains(header) {
            errors.push(format!("{at}: `{header}` is not a public header of {INVENTORY}"));
        }
        let Some(k) = KINDS.iter().position(|x| *x == kind) else {
            errors.push(format!("{at}: unknown kind `{kind}`"));
            continue;
        };
        if (kind == "include-error") != (name == "-") || (name != "-" && !is_ident(name)) {
            errors.push(format!("{at}: name `{name}` does not fit kind `{kind}`"));
        }
        if cfgs != "*" {
            let mut last = None;
            for c in cfgs.split(',') {
                match ids.iter().position(|i| i == c) {
                    Some(p) if last.is_none_or(|l| p > l) => last = Some(p),
                    Some(_) => errors.push(format!("{at}: configuration `{c}` out of table order or repeated")),
                    None => errors.push(format!("{at}: unknown configuration `{c}`")),
                }
            }
            if last.is_some() && cfgs.split(',').count() == ids.len() {
                errors.push(format!("{at}: every configuration listed; write `*`"));
            }
        }
        let key = (header.to_string(), name.to_string(), k);
        if let Some(p) = &prev
            && *p >= key
        {
            errors.push(format!("{at}: row out of order or repeated (sorted by header, name, kind)"));
        }
        prev = Some(key);
        if header != EMPTY_UNIT && kind != "include-error" {
            visible.entry(name.to_string()).or_default().insert(KINDS[k]);
        }
    }
    for (name, table) in required {
        if !visible.contains_key(name) {
            errors.push(format!("{VISIBILITY}: `{name}` ({table}) is visible in no (header, configuration)"));
        }
    }
    errors
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))
}

/// First column of each data row of a TSV with a header line.
fn first_column(text: &str) -> impl Iterator<Item = &str> {
    text.lines().skip(1).filter_map(|l| l.split('\t').next()).filter(|s| !s.is_empty())
}

/// `cargo xtask static` entry: the tables' shape and the coverage of `prototypes.tsv` and `constants.tsv`.
pub fn check(root: &Path) -> Result<(), String> {
    let inventory = read(root, INVENTORY)?;
    let public: BTreeSet<String> = inventory
        .lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() > 3 && f[3] == "public").then(|| f[0].to_string())
        })
        .collect();
    let mut required: Vec<(String, &str)> = Vec::new();
    for table in ["prototypes.tsv", "constants.tsv"] {
        let text = read(root, &format!("spec/abi/{table}"))?;
        required.extend(first_column(&text).map(|n| (n.to_string(), table)));
    }
    let errors = check_tables(&read(root, CONFIGS)?, &read(root, VISIBILITY)?, &public, &required);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("visibility: {} problem(s):\n{}", errors.len(), errors.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    //! no spec clause: Appendix C.12's table format (SL-HEADERS-2); each test breaks one rule of the check.
    use super::*;

    const CFG: &str = "id\tlanguage\tflags\tprobes\nc89\tc\t-std=c89\tISO C90\ngnu\tc\t-std=gnu17 -D_GNU_SOURCE\tGNU\ncxx98\tc++\t-std=c++98\tC++98\n";
    const VIS: &str = "header\tname\tkind\tconfigs\n-\t_GNU_SOURCE\tmacro\tgnu\nstdio.h\t-\tinclude-error\tcxx98\nstdio.h\tEOF\tmacro\t*\nstdio.h\tFILE\ttypedef\t*\nstdio.h\tstdin\tmacro\tc89,gnu\nstdio.h\tstdin\tobject\tc89,gnu\nstdio.h\ttm\tstruct\tgnu\n";

    fn public() -> BTreeSet<String> {
        ["stdio.h".to_string()].into_iter().collect()
    }

    fn req(names: &[&str]) -> Vec<(String, &'static str)> {
        names.iter().map(|n| (n.to_string(), "constants.tsv")).collect()
    }

    fn errs(cfg: &str, vis: &str, names: &[&str]) -> Vec<String> {
        check_tables(cfg, vis, &public(), &req(names))
    }

    fn fails(cfg: &str, vis: &str, names: &[&str], needle: &str) {
        let e = errs(cfg, vis, names);
        assert!(e.iter().any(|x| x.contains(needle)), "expected `{needle}` in {e:?}");
    }

    #[test]
    fn good_tables_pass() {
        assert_eq!(errs(CFG, VIS, &["EOF", "stdin"]), Vec::<String>::new());
    }

    #[test]
    fn config_shape() {
        fails(&CFG.replace("id\tlanguage", "id\tlang"), VIS, &[], "first line");
        fails(&CFG.replace("\tISO C90", ""), VIS, &[], "fields, expected 4");
        fails(&CFG.replace("c89\tc\t", "C89\tc\t"), VIS, &[], "lower-case");
        fails(&CFG.replace("gnu\tc\t", "c89\tc\t"), VIS, &[], "repeated");
        fails(&CFG.replace("cxx98\tc++", "cxx98\tcpp"), VIS, &[], "language");
        fails(&CFG.replace("-std=c89", "-ansi"), VIS, &[], "no -std=");
        fails("id\tlanguage\tflags\tprobes\n", VIS, &[], "no configuration");
    }

    #[test]
    fn visibility_shape() {
        fails(CFG, &VIS.replace("header\tname", "hdr\tname"), &[], "first line");
        fails(CFG, &VIS.replace("\tFILE\ttypedef\t*", "\tFILE\ttypedef"), &[], "fields, expected 4");
        fails(CFG, &VIS.replace("stdio.h\ttm", "bits/types.h\ttm"), &[], "not a public header");
        fails(CFG, &VIS.replace("\ttm\tstruct", "\ttm\tclass"), &[], "unknown kind");
        fails(CFG, &VIS.replace("-\tinclude-error", "x\tinclude-error"), &[], "does not fit kind");
        fails(CFG, &VIS.replace("\tEOF\tmacro", "\tEOF+\tmacro"), &[], "does not fit kind");
        fails(CFG, &VIS.replace("\tEOF\tmacro\t*", "\tEOF\tmacro\tc99"), &[], "unknown configuration");
        fails(CFG, &VIS.replace("\tstdin\tmacro\tc89,gnu", "\tstdin\tmacro\tgnu,c89"), &[], "out of table order");
        fails(CFG, &VIS.replace("\tEOF\tmacro\t*", "\tEOF\tmacro\tc89,gnu,cxx98"), &[], "write `*`");
    }

    #[test]
    fn row_order() {
        fails(CFG, &VIS.replace("stdio.h\tstdin\tmacro\tc89,gnu\nstdio.h\tstdin\tobject", "stdio.h\tstdin\tobject\tc89,gnu\nstdio.h\tstdin\tmacro"), &[], "out of order");
        fails(CFG, &VIS.replace("stdio.h\ttm\tstruct\tgnu\n", "stdio.h\ttm\tstruct\tgnu\nstdio.h\ttm\tstruct\tgnu\n"), &[], "out of order");
        fails(CFG, &VIS.replace("-\t_GNU_SOURCE\tmacro\tgnu\n", "").replace("stdio.h\ttm\tstruct\tgnu\n", "stdio.h\ttm\tstruct\tgnu\n-\t_GNU_SOURCE\tmacro\tgnu\n"), &[], "out of order");
    }

    #[test]
    fn coverage() {
        fails(CFG, VIS, &["EOF", "fopen"], "`fopen` (constants.tsv) is visible in no");
        // The empty unit and include errors do not make a name visible in a header.
        fails(CFG, VIS, &["_GNU_SOURCE"], "`_GNU_SOURCE`");
    }
}
