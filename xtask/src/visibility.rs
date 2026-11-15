//! Shape and coverage check of the visibility tables (Appendix C.12, SL-HEADERS-2).
//!
//! `spec/abi/ftm-configs.tsv` lists the feature-test configurations; `spec/abi/visibility.tsv` gives, per public
//! header, each visible name, its kind and the configurations it is visible in (`*` = all). Both are written by
//! `spec/tools/header_probe.py` on the reference system. The check (part of `cargo xtask static`) fails when a
//! table's shape is wrong (columns, ids, kinds, a header outside the public inventory, unsorted or duplicate rows,
//! a configuration list that is not a sorted subset of the table or not in the probe's one spelling of its set)
//! or when a `prototypes.tsv` or `constants.tsv`
//! name is visible in no (header, configuration).

use std::collections::BTreeSet;
use std::path::Path;

const CONFIGS: &str = "spec/abi/ftm-configs.tsv";
const VISIBILITY: &str = "spec/abi/visibility.tsv";
const INVENTORY: &str = "plan/headers-inventory.tsv";
const CONFIG_HEAD: &str = "id\tlanguage\tflags\tprobes";
const VIS_HEAD: &str = "header\tname\tkind\tconfigs";
/// Kinds in the order the rows of one (header, name) are sorted.
const KINDS: [&str; 9] = ["macro", "typedef", "function", "object", "enumerator", "struct", "union", "enum", "include-error"];
/// Table names the probe cannot show, with the reason.
const EXEMPT: [(&str, &str); 2] = [
    ("sched_getattr", "owner ruling R3: a GLIBC_2.41 addition, absent from the 2.39 reference"),
    ("sched_setattr", "owner ruling R3: a GLIBC_2.41 addition, absent from the 2.39 reference"),
];
/// The header of the rows for the empty translation unit (compiler predefines).
const EMPTY_UNIT: &str = "-";

fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(f) if f == '_' || f.is_ascii_alphabetic()) && c.all(|x| x == '_' || x.is_ascii_alphanumeric())
}

/// The spelling of a configuration set (`visible_in[i]` for `ids[i]`): `*` for all of them, else the shorter of
/// the ids it holds and `!` with the ids it lacks (the ids on a tie), both comma-joined in table order; `None` for
/// the empty set. `spec/tools/header_probe.py` writes the same form.
fn configs_form(ids: &[String], visible_in: &[bool]) -> Option<String> {
    let pick = |want: bool| ids.iter().zip(visible_in).filter(|(_, v)| **v == want).map(|(i, _)| i.as_str()).collect::<Vec<_>>().join(",");
    match visible_in.iter().filter(|v| **v).count() {
        0 => None,
        n if n == ids.len() => Some("*".to_string()),
        _ => {
            let (listed, rest) = (pick(true), format!("!{}", pick(false)));
            Some(if rest.len() < listed.len() { rest } else { listed })
        }
    }
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
    let mut visible: BTreeSet<String> = BTreeSet::new();
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
            let (negated, list) = cfgs.strip_prefix('!').map_or((false, cfgs), |l| (true, l));
            let mut last = None;
            let mut picked = vec![false; ids.len()];
            let mut ok = true;
            for c in list.split(',') {
                match ids.iter().position(|i| i == c) {
                    Some(p) if last.is_none_or(|l| p > l) => {
                        last = Some(p);
                        picked[p] = true;
                    }
                    Some(_) => {
                        ok = false;
                        errors.push(format!("{at}: configuration `{c}` out of table order or repeated"));
                    }
                    None => {
                        ok = false;
                        errors.push(format!("{at}: unknown configuration `{c}`"));
                    }
                }
            }
            if ok {
                // The configurations the row is visible in, then the one spelling the probe writes for them.
                let visible_in: Vec<bool> = picked.iter().map(|&p| p != negated).collect();
                match configs_form(&ids, &visible_in) {
                    None => errors.push(format!("{at}: visible in no configuration; drop the row")),
                    Some(form) if form != cfgs => errors.push(format!("{at}: configurations `{cfgs}` must be written `{form}`")),
                    Some(_) => {}
                }
            }
        }
        let key = (header.to_string(), name.to_string(), k);
        if let Some(p) = &prev
            && *p >= key
        {
            errors.push(format!("{at}: row out of order or repeated (sorted by header, name, kind)"));
        }
        prev = Some(key);
        if kind != "include-error" {
            visible.insert(name.to_string());
        }
    }
    for (name, table) in required {
        if !visible.contains(name) && !EXEMPT.iter().any(|(n, _)| n == name) {
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
    const VIS: &str = "header\tname\tkind\tconfigs\n-\t_GNU_SOURCE\tmacro\tgnu\nstdio.h\t-\tinclude-error\tcxx98\nstdio.h\tEOF\tmacro\t*\nstdio.h\tFILE\ttypedef\t*\nstdio.h\tstdin\tmacro\t!cxx98\nstdio.h\tstdin\tobject\t!cxx98\nstdio.h\ttm\tstruct\tgnu\n";

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
        fails(CFG, &VIS.replace("\tstdin\tmacro\t!cxx98", "\tstdin\tmacro\tgnu,c89"), &[], "out of table order");
        fails(CFG, &VIS.replace("\tstdin\tmacro\t!cxx98", "\tstdin\tmacro\tc89,c89"), &[], "out of table order");
        fails(CFG, &VIS.replace("\tEOF\tmacro\t*", "\tEOF\tmacro\tc89,gnu,cxx98"), &[], "must be written `*`");
        fails(CFG, &VIS.replace("\tEOF\tmacro\t*", "\tEOF\tmacro\t!c89,gnu,cxx98"), &[], "drop the row");
        fails(CFG, &VIS.replace("\tEOF\tmacro\t*", "\tEOF\tmacro\t!c89,cxx98"), &[], "must be written `gnu`");
        // Two ids, `c89,gnu` (7 characters), against one, `!cxx98` (6): the shorter string wins, not fewer ids.
        fails(CFG, &VIS.replace("\tEOF\tmacro\t*", "\tEOF\tmacro\tc89,gnu"), &[], "must be written `!cxx98`");
        fails(CFG, &VIS.replace("\tEOF\tmacro\t*", "\tEOF\tmacro\t!gnu,c89"), &[], "out of table order");
        assert_eq!(errs(CFG, &VIS.replace("\tEOF\tmacro\t*", "\tEOF\tmacro\t!gnu"), &[]), Vec::<String>::new());
    }

    #[test]
    fn configs_form_tie() {
        let ids: Vec<String> = ["a", "bb", "c"].iter().map(|s| s.to_string()).collect();
        // `a,c` and `!bb` are both 3 characters: the ids are written.
        assert_eq!(configs_form(&ids, &[true, false, true]).as_deref(), Some("a,c"));
        assert_eq!(configs_form(&ids, &[true, false, false]).as_deref(), Some("a"));
        assert_eq!(configs_form(&ids, &[false, true, true]).as_deref(), Some("!a"));
        assert_eq!(configs_form(&ids, &[true; 3]).as_deref(), Some("*"));
        assert_eq!(configs_form(&ids, &[false; 3]), None);
    }

    #[test]
    fn row_order() {
        fails(CFG, &VIS.replace("stdio.h\tstdin\tmacro\t!cxx98\nstdio.h\tstdin\tobject", "stdio.h\tstdin\tobject\t!cxx98\nstdio.h\tstdin\tmacro"), &[], "out of order");
        fails(CFG, &VIS.replace("stdio.h\ttm\tstruct\tgnu\n", "stdio.h\ttm\tstruct\tgnu\nstdio.h\ttm\tstruct\tgnu\n"), &[], "out of order");
        fails(CFG, &VIS.replace("-\t_GNU_SOURCE\tmacro\tgnu\n", "").replace("stdio.h\ttm\tstruct\tgnu\n", "stdio.h\ttm\tstruct\tgnu\n-\t_GNU_SOURCE\tmacro\tgnu\n"), &[], "out of order");
    }

    #[test]
    fn coverage() {
        fails(CFG, VIS, &["EOF", "fopen"], "`fopen` (constants.tsv) is visible in no");
        // An include error does not make a name visible; the empty unit and the exemptions do.
        fails(CFG, &VIS.replace("stdio.h\tFILE\ttypedef\t*\n", ""), &["FILE"], "`FILE`");
        assert_eq!(errs(CFG, VIS, &["_GNU_SOURCE", "sched_getattr"]), Vec::<String>::new());
    }
}
