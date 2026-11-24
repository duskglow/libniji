//! `cargo xtask release-notes`: the unreleased release notes, one fragment file per change (D-X.166).
//!
//! Every story that adds a release note used to add lines under one "Unreleased" heading of
//! `docs/RELEASE-NOTES.md`, so every landing changed the lines the next note's branch had to merge into, and parallel
//! notes conflicted with each other in CI (2026-11-23: five branches, two rounds). A note is now its own file,
//! `docs/release-notes.d/<STORY-ID>.md`; adding one never edits a shared line, so two notes merge in either order. At
//! a snapshot or release, `release` assembles them into a new section of `docs/RELEASE-NOTES.md` and removes the
//! fragments it used, so that file holds only released sections. Format and workflow: `docs/release-notes.d/README.md`.
//!
//! A fragment:
//!
//! ```text
//! ---
//! section: changes
//! ---
//! - **`thing`:** what a user notices, wrapped at 120 columns, continuation lines
//!   indented by two spaces.
//! ```
//!
//! Commands:
//! * `cargo xtask release-notes [show]`: print the unreleased notes, assembled (the old "Unreleased" section).
//! * `cargo xtask release-notes check`: the static check (also in `cargo xtask static` and the precheck).
//! * `cargo xtask release-notes release --heading <text> [--intro <file>] [--outro <file>] [--dry-run]`: write the
//!   assembled notes into `docs/RELEASE-NOTES.md` as `## <text>` above the newest released section, then delete the
//!   fragments. Run again with the same heading, it changes nothing.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The fragment directory, relative to the workspace root.
pub(crate) const FRAGMENT_DIR: &str = "docs/release-notes.d";
/// The released notes, relative to the workspace root.
pub(crate) const DOC: &str = "docs/RELEASE-NOTES.md";
/// The heading `show` prints above the assembled notes (the heading the shared file used for them).
const UNRELEASED_HEADING: &str = "## Unreleased (on `next`, for the next snapshot)";
/// Files of [`FRAGMENT_DIR`] that are not fragments.
const NOT_FRAGMENTS: &[&str] = &["README.md"];

/// The known sections, in the order they are assembled: the key a fragment names, and the `###` heading it goes under
/// (`None`: directly under the release heading, the plain list of changes every snapshot since the fifth has).
pub(crate) const SECTIONS: &[(&str, Option<&str>)] = &[
    ("changes", None),
    ("compat", Some("Compatibility flags and differences")),
    ("known-limitations", Some("Known limitations")),
    ("contributors", Some("For contributors")),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fragment {
    /// The file name (`SYS-THREAD-DB-2.md`), the order within a section.
    pub name: String,
    /// The index into [`SECTIONS`].
    pub section: usize,
    /// The note: Markdown list items, no trailing newline.
    pub body: String,
}

/// A fragment file name: a story id (`[A-Z0-9][A-Z0-9-]*`) and `.md`.
fn valid_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".md") else { return false };
    let mut ch = stem.chars();
    matches!(ch.next(), Some(c) if c.is_ascii_uppercase() || c.is_ascii_digit())
        && ch.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
}

/// Parse one fragment; the error names the file and the rule it breaks.
pub(crate) fn parse_fragment(name: &str, text: &str) -> Result<Fragment, String> {
    let err = |what: String| Err(format!("{FRAGMENT_DIR}/{name}: {what}"));
    if !valid_name(name) {
        return err("the file name is not <STORY-ID>.md (capitals, digits and '-'; a second note of one story: <STORY-ID>-2.md)".into());
    }
    if text.contains('\r') {
        return err("carriage return (the repository is LF)".into());
    }
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return err("does not start with the front matter line `---`".into());
    }
    let mut section = None;
    let mut closed = false;
    for line in lines.by_ref() {
        if line == "---" {
            closed = true;
            break;
        }
        match line.split_once(':') {
            Some(("section", v)) => {
                let v = v.trim();
                let known: Vec<&str> = SECTIONS.iter().map(|(k, _)| *k).collect();
                match SECTIONS.iter().position(|(k, _)| *k == v) {
                    Some(i) if section.is_none() => section = Some(i),
                    Some(_) => return err("`section:` given twice".into()),
                    None => return err(format!("unknown section `{v}` (known: {})", known.join(", "))),
                }
            }
            _ => return err(format!("front matter line `{line}` is not `section: <name>`")),
        }
    }
    if !closed {
        return err("the front matter has no closing `---`".into());
    }
    let Some(section) = section else { return err("the front matter has no `section:`".into()) };
    let body: Vec<&str> = lines.skip_while(|l| l.trim().is_empty()).collect();
    let end = body.iter().rposition(|l| !l.trim().is_empty()).map_or(0, |i| i + 1);
    let body = &body[..end];
    if body.is_empty() {
        return err("the note is empty".into());
    }
    if !body[0].starts_with("- ") {
        return err("the note does not start with a list item `- `".into());
    }
    if let Some(l) = body.iter().find(|l| !(l.starts_with("- ") || l.starts_with("  ") && !l.trim().is_empty())) {
        return err(format!("line `{l}` is neither a list item `- ` nor a continuation indented by two spaces (no blank lines, no headings)"));
    }
    if let Some(l) = body.iter().find(|l| l.ends_with(' ') || l.ends_with('\t')) {
        return err(format!("trailing white space on `{l}`"));
    }
    Ok(Fragment { name: name.to_string(), section, body: body.join("\n") })
}

/// The fragment files of `dir`, sorted by name, and the problems of the files that are neither fragments nor
/// [`NOT_FRAGMENTS`]. A missing directory has no fragments.
fn read_fragments(dir: &Path) -> (Vec<(String, PathBuf)>, Vec<String>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else { return (out, problems) };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if NOT_FRAGMENTS.contains(&name.as_str()) {
            continue;
        }
        if !e.path().is_file() {
            problems.push(format!("{FRAGMENT_DIR}/{name}: not a file (the directory holds only fragments and README.md)"));
            continue;
        }
        out.push((name, e.path()));
    }
    out.sort();
    (out, problems)
}

/// Read and parse every fragment of the tree: the fragments and every problem found (all of them, not the first).
pub(crate) fn load(root: &Path) -> (Vec<Fragment>, Vec<String>) {
    let (files, mut problems) = read_fragments(&root.join(FRAGMENT_DIR));
    let mut frags = Vec::new();
    for (name, path) in files {
        match fs::read_to_string(&path).map_err(|e| format!("{FRAGMENT_DIR}/{name}: cannot read: {e}")).and_then(|t| parse_fragment(&name, &t)) {
            Ok(f) => frags.push(f),
            Err(e) => problems.push(e),
        }
    }
    (frags, problems)
}

/// The assembled notes: by section in [`SECTIONS`] order, within a section by file name; a section's notes are one
/// list, sections are separated by a blank line and (but `changes`) start with their `###` heading. The order of
/// `frags` does not matter. Empty for no fragments; no trailing newline.
pub(crate) fn assemble(frags: &[Fragment]) -> String {
    let mut sorted: Vec<&Fragment> = frags.iter().collect();
    sorted.sort_by(|a, b| (a.section, &a.name).cmp(&(b.section, &b.name)));
    let mut parts = Vec::new();
    for (i, (_, heading)) in SECTIONS.iter().enumerate() {
        let bodies: Vec<&str> = sorted.iter().filter(|f| f.section == i).map(|f| f.body.as_str()).collect();
        if bodies.is_empty() {
            continue;
        }
        let list = bodies.join("\n");
        parts.push(match heading {
            Some(h) => format!("### {h}\n\n{list}"),
            None => list,
        });
    }
    parts.join("\n\n")
}

/// `doc` with a new section `## <heading>` (then `intro`, the assembled `body`, `outro`, each a paragraph block) above
/// its first `## ` section. Everything else of `doc` is kept byte for byte.
pub(crate) fn insert_release(doc: &str, heading: &str, intro: Option<&str>, body: &str, outro: Option<&str>) -> Result<String, String> {
    let at = first_section(doc).ok_or_else(|| format!("{DOC} has no `## ` section to insert above"))?;
    let mut sec = format!("## {heading}\n\n");
    for block in [intro, Some(body), outro].into_iter().flatten().map(str::trim).filter(|b| !b.is_empty()) {
        sec.push_str(block);
        sec.push_str("\n\n");
    }
    Ok(format!("{}{sec}{}", &doc[..at], &doc[at..]))
}

/// The byte offset of the first line starting `## `.
fn first_section(doc: &str) -> Option<usize> {
    let mut off = 0;
    for line in doc.split_inclusive('\n') {
        if line.starts_with("## ") {
            return Some(off);
        }
        off += line.len();
    }
    None
}

/// The `## ` headings of `doc`, without the `## `.
fn headings(doc: &str) -> Vec<&str> {
    doc.lines().filter_map(|l| l.strip_prefix("## ")).collect()
}

/// The problems of the shared document: it holds only released sections (an `## Unreleased` heading means a note
/// went into the shared file instead of a fragment), no heading twice, and the assembled notes insert into it.
pub(crate) fn doc_problems(doc: &str, body: &str) -> Vec<String> {
    let mut p = Vec::new();
    let hs = headings(doc);
    if hs.is_empty() {
        p.push(format!("{DOC}: no `## ` section"));
    }
    if hs.iter().any(|h| h.to_ascii_lowercase().starts_with("unreleased")) {
        p.push(format!("{DOC}: an `## Unreleased` section: unreleased notes go in {FRAGMENT_DIR}/<STORY-ID>.md (see its README.md), the shared file holds only released sections"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for h in &hs {
        if !seen.insert(*h) {
            p.push(format!("{DOC}: the heading `## {h}` twice"));
        }
    }
    if p.is_empty() && !body.is_empty() {
        let probe = "release-notes check (probe)";
        match insert_release(doc, probe, None, body, None) {
            Ok(out) if headings(&out).first() == Some(&probe) && out.ends_with(doc.get(first_section(doc).unwrap_or(0)..).unwrap_or("")) => {}
            Ok(_) => p.push(format!("{DOC}: the assembled notes do not insert above the newest section")),
            Err(e) => p.push(e),
        }
    }
    p
}

/// `cargo xtask release-notes check` and the `release-notes` static check.
pub(crate) fn check(root: &Path) -> Result<(), String> {
    let (frags, mut problems) = load(root);
    match fs::read_to_string(root.join(DOC)) {
        Ok(doc) => problems.extend(doc_problems(&doc, &assemble(&frags))),
        Err(e) => problems.push(format!("{DOC}: cannot read: {e}")),
    }
    if problems.is_empty() {
        println!("release-notes: ok ({} fragment(s) in {FRAGMENT_DIR})", frags.len());
        Ok(())
    } else {
        Err(format!("{}\nfix: the fragment format is in {FRAGMENT_DIR}/README.md", problems.join("\n")))
    }
}

/// What `release` did.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Released {
    /// The section was written and these fragments removed.
    Written(Vec<String>),
    /// The heading is already in the document and no fragment is left: a second run of the same release.
    AlreadyDone,
}

/// The fragments of the released commit `rev` (`--at`): file name -> text, from git.
fn fragments_at(root: &Path, rev: &str) -> Result<BTreeMap<String, String>, String> {
    let git = |args: &[&str]| -> Result<String, String> {
        let o = std::process::Command::new("git").current_dir(root).args(args).output().map_err(|e| format!("cannot run git: {e}"))?;
        if o.status.success() { Ok(String::from_utf8_lossy(&o.stdout).into_owned()) } else { Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&o.stderr).trim())) }
    };
    let mut out = BTreeMap::new();
    for path in git(&["ls-tree", "--name-only", rev, &format!("{FRAGMENT_DIR}/")])?.lines() {
        let name = path.rsplit('/').next().unwrap_or(path);
        if !NOT_FRAGMENTS.contains(&name) {
            out.insert(name.to_string(), git(&["show", &format!("{rev}:{path}")])?);
        }
    }
    Ok(out)
}

/// Write the release section and remove the fragments it used (the document first, so a failed write loses no note).
///
/// `at` (`--at <rev>`, the snapshot's commit): release only the fragments that commit holds, each unchanged since,
/// so a note that landed after the snapshot stays for the next one. `None`: every fragment of the tree.
pub(crate) fn release(root: &Path, heading: &str, intro: Option<&str>, outro: Option<&str>, at: Option<&BTreeMap<String, String>>, dry_run: bool) -> Result<Released, String> {
    if heading.trim().is_empty() || heading.contains('\n') || heading.starts_with('#') {
        return Err("--heading takes one line of text without the leading `## `".into());
    }
    let (mut frags, problems) = load(root);
    if !problems.is_empty() {
        return Err(format!("{}\nfix the fragments first (`cargo xtask release-notes check`)", problems.join("\n")));
    }
    if let Some(at) = at {
        frags.retain(|f| at.contains_key(&f.name));
    }
    let doc_path = root.join(DOC);
    let doc = fs::read_to_string(&doc_path).map_err(|e| format!("{DOC}: cannot read: {e}"))?;
    let present = headings(&doc).contains(&heading);
    match (present, frags.is_empty()) {
        (true, true) => return Ok(Released::AlreadyDone),
        (true, false) => return Err(format!("{DOC} already has `## {heading}`, and {} of its fragment(s) remain: release them under a new heading", frags.len())),
        (false, true) => return Err(format!("no fragments in {FRAGMENT_DIR}{}: nothing to release", if at.is_some() { " at --at's commit" } else { "" })),
        (false, false) => {}
    }
    if let Some(at) = at {
        let mut bad = Vec::new();
        for (name, text) in at {
            match fs::read_to_string(root.join(FRAGMENT_DIR).join(name)) {
                Ok(now) if now == *text => {}
                Ok(_) => bad.push(format!("{FRAGMENT_DIR}/{name}: changed since the released commit")),
                Err(_) => bad.push(format!("{FRAGMENT_DIR}/{name}: in the released commit but not in this tree")),
            }
        }
        if !bad.is_empty() {
            return Err(format!("{}\nthe notes released must be the snapshot's: settle these by hand first", bad.join("\n")));
        }
    }
    let body = assemble(&frags);
    let out = insert_release(&doc, heading, intro, &body, outro)?;
    if dry_run {
        print!("{}", &out[first_section(&doc).unwrap_or(0)..out.len() - (doc.len() - first_section(&doc).unwrap_or(0))]);
        return Ok(Released::Written(vec![]));
    }
    fs::write(&doc_path, out).map_err(|e| format!("{DOC}: cannot write: {e}"))?;
    let mut removed = Vec::new();
    for f in &frags {
        let p = root.join(FRAGMENT_DIR).join(&f.name);
        fs::remove_file(&p).map_err(|e| format!("{}: cannot remove: {e} ({DOC} is already written)", p.display()))?;
        removed.push(f.name.clone());
    }
    Ok(Released::Written(removed))
}

fn read_opt_file(args: &[String], flag: &str) -> Result<Option<String>, String> {
    match args.iter().position(|a| a == flag) {
        None => Ok(None),
        Some(i) => {
            let p = args.get(i + 1).ok_or_else(|| format!("{flag} needs a file"))?;
            fs::read_to_string(p).map(Some).map_err(|e| format!("{flag} {p}: {e}"))
        }
    }
}

/// `cargo xtask release-notes ...`
pub(crate) fn run(root: &Path, args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: cargo xtask release-notes [show | check | release --heading <text> [--at <rev>] [--intro <file>] [--outro <file>] [--dry-run]]";
    match args.first().map(String::as_str) {
        None | Some("show") => {
            let (frags, problems) = load(root);
            if !problems.is_empty() {
                return Err(problems.join("\n"));
            }
            let body = assemble(&frags);
            if body.is_empty() {
                println!("{UNRELEASED_HEADING}\n\nNo release-note fragments in {FRAGMENT_DIR}.");
            } else {
                println!("{UNRELEASED_HEADING}\n\n{body}");
            }
            Ok(())
        }
        Some("check") => check(root),
        Some("release") => {
            let known = ["--heading", "--at", "--intro", "--outro"];
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    a if known.contains(&a) => i += 2,
                    "--dry-run" => i += 1,
                    a => return Err(format!("unknown argument `{a}`\n{USAGE}")),
                }
            }
            let heading = args.iter().position(|a| a == "--heading").and_then(|i| args.get(i + 1)).ok_or_else(|| format!("release needs --heading\n{USAGE}"))?;
            let (intro, outro) = (read_opt_file(args, "--intro")?, read_opt_file(args, "--outro")?);
            let at = match args.iter().position(|a| a == "--at") {
                None => None,
                Some(i) => Some(fragments_at(root, args.get(i + 1).ok_or("--at needs a revision")?)?),
            };
            match release(root, heading, intro.as_deref(), outro.as_deref(), at.as_ref(), args.iter().any(|a| a == "--dry-run"))? {
                Released::AlreadyDone => println!("release-notes: `## {heading}` is already in {DOC} and no fragment is left: nothing to do"),
                Released::Written(names) if names.is_empty() => {}
                Released::Written(names) => println!("release-notes: wrote `## {heading}` into {DOC}; removed {} fragment(s): {}", names.len(), names.join(" ")),
            }
            Ok(())
        }
        Some(a) => Err(format!("unknown subcommand `{a}`\n{USAGE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(name: &str, section: &str, body: &str) -> Fragment {
        parse_fragment(name, &format!("---\nsection: {section}\n---\n{body}\n")).unwrap()
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("xtask-release-notes-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join(FRAGMENT_DIR)).unwrap();
        d
    }

    #[test]
    fn assembly_is_independent_of_input_order() {
        // no spec clause: two notes of one section (and one of another) assemble identically in either order
        let a = frag("AAA-1.md", "changes", "- **a:** first.");
        let b = frag("BBB-2.md", "changes", "- **b:** second,\n  wrapped.");
        let c = frag("CCC-3.md", "known-limitations", "- **c:** a limit.");
        let one = assemble(&[a.clone(), b.clone(), c.clone()]);
        let two = assemble(&[c, b, a]);
        assert_eq!(one, two, "assembly depends on the input order");
        assert_eq!(one, "- **a:** first.\n- **b:** second,\n  wrapped.\n\n### Known limitations\n\n- **c:** a limit.");
    }

    #[test]
    fn sections_assemble_in_the_declared_order() {
        // no spec clause: sections come out in SECTIONS order whatever the names, empty sections leave nothing
        let out = assemble(&[frag("A.md", "contributors", "- x."), frag("Z.md", "changes", "- y."), frag("M.md", "compat", "- z.")]);
        assert_eq!(out, "- y.\n\n### Compatibility flags and differences\n\n- z.\n\n### For contributors\n\n- x.");
        assert_eq!(assemble(&[]), "");
    }

    #[test]
    fn a_fragment_naming_an_unknown_section_is_rejected() {
        // no spec clause: a typo in the section must not drop or misfile the note
        let e = parse_fragment("X-1.md", "---\nsection: chnages\n---\n- a.\n").unwrap_err();
        assert!(e.contains("unknown section `chnages`") && e.contains("known: changes, compat"), "{e}");
    }

    #[test]
    fn malformed_fragments_are_rejected() {
        // no spec clause: each rule of the fragment format has its own message
        let cases = [
            ("x.md", "---\nsection: changes\n---\n- a.\n", "file name"),
            ("README2.txt", "---\nsection: changes\n---\n- a.\n", "file name"),
            ("X.md", "section: changes\n- a.\n", "front matter line `---`"),
            ("X.md", "---\nsection: changes\n- a.\n", "front matter line `- a.`"),
            ("X.md", "---\nsection: changes\n", "no closing"),
            ("X.md", "---\n---\n- a.\n", "no `section:`"),
            ("X.md", "---\nsection: changes\nsection: compat\n---\n- a.\n", "twice"),
            ("X.md", "---\nsection: changes\n---\n\n\n", "empty"),
            ("X.md", "---\nsection: changes\n---\nA paragraph.\n", "list item"),
            ("X.md", "---\nsection: changes\n---\n- a.\n\n- b.\n", "no blank lines"),
            ("X.md", "---\nsection: changes\n---\n- a.\n### H\n", "neither a list item"),
            ("X.md", "---\nsection: changes\n---\n- a. \n", "trailing white space"),
            ("X.md", "---\r\nsection: changes\r\n---\r\n- a.\r\n", "carriage return"),
        ];
        for (name, text, want) in cases {
            let e = parse_fragment(name, text).expect_err(text);
            assert!(e.contains(want), "{name} {text:?}: `{e}` lacks `{want}`");
        }
        let ok = parse_fragment("UTIL-14-2.md", "---\nsection: changes\n---\n\n- a,\n  b.\n\n").unwrap();
        assert_eq!((ok.section, ok.body.as_str()), (0, "- a,\n  b."));
    }

    #[test]
    fn the_shared_document_holds_only_released_sections() {
        // no spec clause: a note written into the shared file instead of a fragment is caught
        let doc = "# T\n\n## Unreleased (on `next`)\n\n- a.\n\n## One\n\nx\n";
        assert!(doc_problems(doc, "").iter().any(|p| p.contains("Unreleased")));
        assert!(doc_problems("# T\n\n## One\n\n## One\n", "").iter().any(|p| p.contains("twice")));
        assert!(doc_problems("# T\n", "").iter().any(|p| p.contains("no `## ` section")));
        assert!(doc_problems("# T\n\n## One\n\nx\n", "- a.").is_empty());
    }

    #[test]
    fn release_inserts_above_the_newest_section_removes_the_fragments_and_is_idempotent() {
        // no spec clause: a release writes one section, keeps the released text byte for byte, and a rerun is a no-op
        let d = tmp("release");
        let old = "# Notes\n\nPreamble.\n\n## Second\n\n- s.\n\n## First\n\n- f.\n";
        fs::write(d.join(DOC), old).unwrap();
        fs::write(d.join(FRAGMENT_DIR).join("README.md"), "not a fragment\n").unwrap();
        fs::write(d.join(FRAGMENT_DIR).join("B-1.md"), "---\nsection: changes\n---\n- b.\n").unwrap();
        fs::write(d.join(FRAGMENT_DIR).join("A-1.md"), "---\nsection: changes\n---\n- a.\n").unwrap();
        let r = release(&d, "Third", Some("Intro."), Some("Outro."), None, false).unwrap();
        assert_eq!(r, Released::Written(vec!["A-1.md".into(), "B-1.md".into()]));
        let want = "# Notes\n\nPreamble.\n\n## Third\n\nIntro.\n\n- a.\n- b.\n\nOutro.\n\n## Second\n\n- s.\n\n## First\n\n- f.\n";
        assert_eq!(fs::read_to_string(d.join(DOC)).unwrap(), want);
        assert!(!d.join(FRAGMENT_DIR).join("A-1.md").exists() && d.join(FRAGMENT_DIR).join("README.md").exists());
        assert_eq!(release(&d, "Third", Some("Intro."), Some("Outro."), None, false).unwrap(), Released::AlreadyDone);
        assert_eq!(fs::read_to_string(d.join(DOC)).unwrap(), want, "a second run changed the document");
        fs::write(d.join(FRAGMENT_DIR).join("C-1.md"), "---\nsection: changes\n---\n- c.\n").unwrap();
        assert!(release(&d, "Third", None, None, None, false).unwrap_err().contains("already has"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn release_at_a_commit_keeps_the_notes_that_landed_after_it() {
        // no spec clause: a note that landed after the snapshot's commit stays for the next snapshot
        let d = tmp("at");
        fs::write(d.join(DOC), "# Notes\n\n## First\n\n- f.\n").unwrap();
        let (a, late) = ("---\nsection: changes\n---\n- a.\n", "---\nsection: changes\n---\n- late.\n");
        fs::write(d.join(FRAGMENT_DIR).join("A-1.md"), a).unwrap();
        fs::write(d.join(FRAGMENT_DIR).join("LATE-1.md"), late).unwrap();
        let at: BTreeMap<String, String> = [("A-1.md".to_string(), a.to_string())].into();
        assert_eq!(release(&d, "Second", None, None, Some(&at), false).unwrap(), Released::Written(vec!["A-1.md".into()]));
        assert_eq!(fs::read_to_string(d.join(DOC)).unwrap(), "# Notes\n\n## Second\n\n- a.\n\n## First\n\n- f.\n");
        assert!(d.join(FRAGMENT_DIR).join("LATE-1.md").exists(), "a note newer than the release commit was consumed");
        assert_eq!(release(&d, "Second", None, None, Some(&at), false).unwrap(), Released::AlreadyDone, "a rerun with --at");
        let edited: BTreeMap<String, String> = [("LATE-1.md".to_string(), "---\nsection: changes\n---\n- older.\n".to_string())].into();
        assert!(release(&d, "Third", None, None, Some(&edited), false).unwrap_err().contains("changed since the released commit"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_tree_fragments_and_document_pass_the_check() {
        // no spec clause: the committed fragments and docs/RELEASE-NOTES.md are well formed (the static check, as a test)
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        check(root).unwrap();
    }
}
