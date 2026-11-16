//! Rendering the views: `@generated` line, head file, header row, separator, one row per record (sorted by id), tail
//! file. Cells are the stored text, verbatim. Derived markers: a row superseded by an active record starts
//! `**Superseded by <id>.** `, a withdrawn row `**Withdrawn.** `, and an amended row ends ` (Amended by <ids>.)`; they
//! go in the last column of decision and saturation views (compat rows say it in their Status cell).

use std::collections::BTreeMap;
use std::path::Path;

use super::schema::{self, Kind};
use super::{GENERATED, Record, View, ViewKind};

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))
}

/// Who supersedes and amends each id (active records only), sorted.
struct Links {
    superseded_by: BTreeMap<String, Vec<String>>,
    amended_by: BTreeMap<String, Vec<String>>,
}

fn links(records: &[Record]) -> Links {
    let mut l = Links { superseded_by: BTreeMap::new(), amended_by: BTreeMap::new() };
    for r in records.iter().filter(|r| !r.pending && r.str("status") != Some("withdrawn")) {
        for t in r.list("supersedes") {
            l.superseded_by.entry(t.clone()).or_default().push(r.id().to_string());
        }
        for t in r.list("amends") {
            l.amended_by.entry(t.clone()).or_default().push(r.id().to_string());
        }
    }
    for v in l.superseded_by.values_mut().chain(l.amended_by.values_mut()) {
        v.sort();
    }
    l
}

fn cells(r: &Record, v: &View, l: &Links) -> Vec<String> {
    let mut out: Vec<String> = v
        .columns
        .iter()
        .map(|c| match c.as_str() {
            "status" => r.str("status_text").or(r.str("status")).unwrap_or("").to_string(),
            c => r.str(c).unwrap_or("").to_string(),
        })
        .map(|c| rename_code(&c, &v.renames))
        .collect();
    if r.kind != Kind::Compat
        && let Some(last) = out.last_mut()
    {
        let id = r.id();
        if let Some(by) = l.superseded_by.get(id) {
            *last = format!("**Superseded by {}.** {last}", by.join(", "));
        }
        if r.str("status") == Some("withdrawn") {
            *last = format!("**Withdrawn.** {last}");
        }
        if let Some(by) = l.amended_by.get(id) {
            last.push_str(&format!(" (Amended by {}.)", by.join(", ")));
        }
    }
    out
}

/// The closed backtick code spans of `cell`, as (start, end) byte ranges of their contents. An unclosed backtick
/// opens no span.
fn code_spans(cell: &str) -> Vec<(usize, usize)> {
    let ticks: Vec<usize> = cell.match_indices('`').map(|(i, _)| i).collect();
    ticks.chunks_exact(2).map(|p| (p[0] + 1, p[1])).collect()
}

/// `cell` with each view rename applied inside its code spans only (the prose around them is the record's own words).
fn rename_code(cell: &str, renames: &[(String, String)]) -> String {
    if renames.is_empty() {
        return cell.to_string();
    }
    let mut out = String::with_capacity(cell.len());
    let mut at = 0;
    for (start, end) in code_spans(cell) {
        out.push_str(&cell[at..start]);
        out.push_str(&renames.iter().fold(cell[start..end].to_string(), |s, (old, new)| s.replace(old.as_str(), new)));
        at = end;
    }
    out.push_str(&cell[at..]);
    out
}

/// The renames of `v` that match no code span in any cell of its rows: a stale rename hides nothing and is removed.
pub fn unused_renames<'a>(v: &'a View, records: &[Record]) -> Vec<&'a str> {
    let cells: Vec<String> = view_records(v, records).iter().flat_map(|r| v.columns.iter().filter_map(|c| r.str(if c == "status" { "status_text" } else { c }).or(r.str(c)).map(str::to_string))).collect();
    v.renames.iter().filter(|(old, _)| !cells.iter().any(|c| code_spans(c).iter().any(|&(s, e)| c[s..e].contains(old.as_str())))).map(|(old, _)| old.as_str()).collect()
}

/// A table view's records: its kind's records in its section, sorted by id.
fn view_records<'r>(v: &View, records: &'r [Record]) -> Vec<&'r Record> {
    let ViewKind::Table(kind) = v.kind else { return Vec::new() };
    let mut mine: Vec<(&Record, (usize, u64, u8))> = records
        .iter()
        .filter(|r| r.kind == kind && !r.pending && (kind == Kind::Compat || r.str("section") == v.section.as_deref()))
        .map(|r| (r, schema::parse_id(kind, r.id()).map_or((usize::MAX, 0, 0), |p| schema::sort_key(&p, &v.series))))
        .collect();
    mine.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.id().cmp(b.0.id())));
    mine.into_iter().map(|(r, _)| r).collect()
}

/// A table view's rows: its kind's records in its section, sorted by id.
pub fn rows(v: &View, records: &[Record]) -> Vec<String> {
    let l = links(records);
    view_records(v, records).into_iter().map(|r| format!("| {} |", cells(r, v, &l).join(" | "))).collect()
}

/// The section files of an index view's directory with a number >= 1, in number order: (number, file name).
fn sections(root: &Path, dir: &str) -> Result<Vec<(u32, String)>, String> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(root.join(dir)).map_err(|e| format!("{dir}: {e}"))?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if let Some(n) = name.strip_suffix(".md").and_then(|s| s.split('-').next()).and_then(|p| p.parse::<u32>().ok())
            && n >= 1
        {
            out.push((n, name));
        }
    }
    out.sort();
    Ok(out)
}

/// Render every view: (path, text), in views.toml order. Table views first, so the index reads the section files'
/// headings from their new text.
pub fn render_all(root: &Path, views: &[View], records: &[Record]) -> Result<Vec<(String, String)>, String> {
    let mut out: Vec<(String, String)> = Vec::new();
    for v in views.iter().filter(|v| v.kind != ViewKind::Index) {
        let mut t = format!("{GENERATED}\n{}{}\n{}\n", read(root, &v.head)?, v.header, v.separator);
        for row in rows(v, records) {
            t.push_str(&row);
            t.push('\n');
        }
        if let Some(tail) = &v.tail {
            t.push_str(&read(root, tail)?);
        }
        out.push((v.path.clone(), t));
    }
    for v in views.iter().filter(|v| v.kind == ViewKind::Index) {
        let dir = v.dir.as_deref().unwrap_or_default();
        let parent = Path::new(&v.path).parent().map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        let link = dir.strip_prefix(&format!("{parent}/")).unwrap_or(dir);
        let mut t = format!("{GENERATED}\n{}", read(root, &v.head)?);
        for (n, file) in sections(root, dir)? {
            let p = format!("{dir}/{file}");
            let text = match out.iter().find(|(q, _)| *q == p) {
                Some((_, t)) => t.clone(),
                None => read(root, &p)?,
            };
            let title = text.lines().find_map(|l| l.strip_prefix("## ")).ok_or_else(|| format!("{p}: no `## {n}. <title>` heading for the index"))?;
            t.push_str(&format!("- [{title}]({link}/{file}): section {n}\n"));
        }
        if let Some(tail) = &v.tail {
            t.push_str(&read(root, tail)?);
        }
        out.push((v.path.clone(), t));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{dec, root_with};
    use super::super::{gen_cmd, load_records, load_views};
    use super::*;

    #[test]
    fn derived_markers() {
        let root = root_with(&[
            ("records/decisions/D-4/D-4.11.toml", Some(dec("D-4.11", "supersedes = [\"D-4.2\"]\namends = [\"D-3.9\"]\nstory = \"S-1\"\ndate = \"2026-11-08\"\n"))),
            ("records/decisions/D-4/D-4.10.toml", Some(dec("D-4.10", "").replace("active", "withdrawn"))),
        ]);
        let views = load_views(&root).expect("views");
        let rows = rows(&views[0], &load_records(&root).0);
        assert_eq!(rows, ["| D-3.9 | body of D-3.9 (Amended by D-4.11.) |", "| D-4.2 | **Superseded by D-4.11.** body of D-4.2 |", "| D-4.10 | **Withdrawn.** body of D-4.10 |", "| D-4.11 | body of D-4.11 |"]);
    }

    /// No spec clause (tooling): a view rename rewrites names inside closed code spans and leaves the record's prose,
    /// an unclosed backtick and the record file itself alone.
    #[test]
    fn renames_touch_code_spans_only() {
        let views = format!("{}renames = [\"old.x => new.x\", \"OLD_N => NEW_N\"]\n", super::super::tests::VIEWS_TOML);
        let compat = "id = \"COMPAT-1\"\nstatus = \"registered\"\nstatus_text = \"set `GLIBC_TUNABLES=old.x.y=1` at `OLD_N`; old.x in prose `keep` old.x; open `old.x\"\nbody = '''\n'''\n";
        let root = root_with(&[("records/views.toml", Some(views)), ("records/compat/COMPAT-1.toml", Some(compat.into()))]);
        let views = load_views(&root).expect("views");
        let records = load_records(&root).0;
        assert_eq!(rows(&views[1], &records), ["| COMPAT-1 | set `GLIBC_TUNABLES=new.x.y=1` at `NEW_N`; old.x in prose `keep` old.x; open `old.x |"]);
        assert!(unused_renames(&views[1], &records).is_empty());
        assert!(std::fs::read_to_string(root.join("records/compat/COMPAT-1.toml")).expect("record").contains("`OLD_N`"));
    }

    /// No spec clause (tooling): a rename no code span uses is a problem, and a rename that is not `old => new` is refused.
    #[test]
    fn stale_and_malformed_renames() {
        let root = root_with(&[("records/views.toml", Some(format!("{}renames = [\"gone.x => new.x\"]\n", super::super::tests::VIEWS_TOML)))]);
        let views = load_views(&root).expect("views");
        let bad = super::super::check::validate(&root, &views, &load_records(&root).0);
        assert!(bad.iter().any(|b| b.contains("rename `gone.x` matches no code span")), "{bad:?}");
        for r in ["a=>b", "a => a", " => b", "`a` => b"] {
            let root = root_with(&[("records/views.toml", Some(format!("{}renames = [\"{r}\"]\n", super::super::tests::VIEWS_TOML)))]);
            assert!(load_views(&root).is_err_and(|e| e.contains("is not `old => new`")), "{r}");
        }
    }

    #[test]
    fn index_lines_come_from_headings() {
        let root = root_with(&[
            ("records/views.toml", Some(format!("{}\n[[view]]\nname = \"idx\"\npath = \"docs/reg.md\"\nkind = \"index\"\ndir = \"docs/sat\"\nhead = \"records/views/idx.head.md\"\n", super::super::tests::VIEWS_TOML))),
            ("records/views/idx.head.md", Some("# Register\n".into())),
            ("docs/sat/00-intro.md", Some("# intro\n".into())),
            ("docs/sat/10-ten.md", Some("## 10. Ten\n".into())),
            ("docs/sat/2-two.md", Some("## 2. Two (x)\n".into())),
        ]);
        gen_cmd(&root, false, true).expect("gen");
        let t = std::fs::read_to_string(root.join("docs/reg.md")).expect("index");
        assert_eq!(t, format!("{GENERATED}\n# Register\n- [2. Two (x)](sat/2-two.md): section 2\n- [10. Ten](sat/10-ten.md): section 10\n"));
    }
}
