//! `old-name` (static): the project's former name does not come back (RENAME-NIJI-DOCS; the rename to libniji, D-X.114).
//!
//! The project was renamed from its former name to libniji on 2026-11-16. This check fails on the former name, in any
//! case, in every tracked file outside the allow-list below. Files that are not UTF-8 are read lossily. No spec clause:
//! a project-name policy, no library behaviour.
//!
//! The allow-list, all of it history or kept by an owner ruling:
//! - whole paths ([`ALLOWED_PATHS`]): `handoff/`, `records/`, `validation/results/` and `results/` (history and dated
//!   measurement records), the views generated from `records/` ([`generated_view`]), `tools/rename_niji.py` (the
//!   rename itself) and the fuzz seeds (binary inputs that hold the old run-time path bytes);
//! - the release notes' past entries: every line of `docs/RELEASE-NOTES.md` after its history marker
//!   ([`NOTES_MARKER`]); new releases go above the marker;
//! - the old-to-new name table for package users: the table rows (lines starting `|`) of the section
//!   [`RENAME_HEADING`] in [`RENAME_DOC`], up to the next `## ` heading (DOCS-SNAPSHOT-NIJI1);
//! - fixed spellings ([`ALLOWED`]): the author identity (owner, Q3), the ops checkout's name and the historic
//!   repository label (Q6), the one "was called" sentence, the two test names the code rename kept, and the compat
//!   records' column key in the two files that read it;
//! - story ids written `<OLD>-<NAME>` in capitals (they name history: handoff notes, plan files);
//! - quoted rulings: a double-quoted span on a line that names a ruling or the owner.
//!
//! The name is never written literally in this file (it is assembled at run time), so the check passes on itself.

use std::path::Path;

/// The former name, lower case, assembled so this file holds no literal copy of it.
fn old() -> String {
    ["lie", "lla"].concat()
}

/// Path prefixes (ending `/`) and exact paths where the former name may stand.
const ALLOWED_PATHS: [&str; 6] = ["handoff/", "records/", "validation/results/", "results/", "tools/rename_niji.py", "fuzz/seeds/"];

/// The release notes and the comment line above which new releases are written; past entries follow it.
const NOTES: &str = "docs/RELEASE-NOTES.md";
const NOTES_MARKER: &str = "<!-- History: the entries below describe what was released under the project's earlier names";

/// The page that maps the former names to the new ones, and the heading of that section; only its table rows may
/// name the former name.
const RENAME_DOC: &str = "docs/install-packages.md";
const RENAME_HEADING: &str = "## Names before libniji";

/// Fixed spellings that may stand anywhere: `%o` is the former name in lower case, `%O` capitalised.
const ALLOWED: [&str; 6] = [
    "%O Maintainers <%o@koiren.ai>",
    "%o-ops",
    "%o-historic",
    "libniji was called %O until November 2026.",
    "reference_ldconfig_lists_%o_cache_alike",
    "reference_loader_resolves_through_%o_cache",
];

/// The compat records' column key (`records/compat/*.toml` keep it as history), only in the files that read it.
const KEY_FILES: [&str; 2] = ["xtask/src/records/schema.rs", "tools/records_import.py"];

/// A view `cargo xtask records-gen` (and its kin) writes from `records/`: never edited by hand, it quotes the records.
fn generated_view(path: &str) -> bool {
    let decisions = path.strip_prefix("plan/07-decisions/").is_some_and(|f| !f.contains('/') && f.ends_with(".md") && f != "x3-regex-lazy-dfa.md");
    let saturation = path.strip_prefix("docs/saturation/").is_some_and(|f| f.len() > 3 && f.starts_with('0') && (b'1'..=b'7').contains(&f.as_bytes()[1]) && f.as_bytes()[2] == b'-');
    decisions || saturation || matches!(path, "docs/compat-flags.md" | "docs/saturation-register.md" | "crates/math/KERNELS.md" | "docs/codemap/symbols.tsv")
}

fn allowed_path(path: &str) -> bool {
    ALLOWED_PATHS.iter().any(|p| if p.ends_with('/') { path.starts_with(p) } else { path == *p }) || generated_view(path)
}

fn spell(s: &str, lower: &str) -> String {
    let mut cap = lower.to_string();
    cap[..1].make_ascii_uppercase();
    s.replace("%o", lower).replace("%O", &cap)
}

/// Blank `len` bytes of `b` from `at` (the bytes stay ASCII spaces, so offsets do not move).
fn blank(b: &mut [u8], at: usize, len: usize) {
    b[at..at + len].iter_mut().for_each(|c| *c = b' ');
}

/// The line with every allowed occurrence blanked; the caller then looks for the name in what is left.
fn strip_allowed(path: &str, line: &str, name: &str) -> Vec<u8> {
    let mut b = line.as_bytes().to_vec();
    let mut tokens: Vec<String> = ALLOWED.iter().map(|s| spell(s, name)).collect();
    if KEY_FILES.contains(&path) {
        tokens.push(format!("\"{name}\""));
    }
    for t in &tokens {
        while let Some(i) = find(&b, t.as_bytes()) {
            blank(&mut b, i, t.len());
        }
    }
    // Story ids: `<OLD>-` followed by a capital letter or a digit.
    let upper = name.to_ascii_uppercase() + "-";
    let mut from = 0;
    while let Some(i) = find(&b[from..], upper.as_bytes()).map(|i| i + from) {
        let next = b.get(i + upper.len()).copied().unwrap_or(b' ');
        if next.is_ascii_uppercase() || next.is_ascii_digit() {
            let end = i + upper.len() + b[i + upper.len()..].iter().take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || **c == b'-').count();
            blank(&mut b, i, end - i);
        }
        from = i + 1;
    }
    // Quoted rulings.
    let lower = line.to_ascii_lowercase();
    if lower.contains("ruling") || lower.contains("owner") {
        let quotes: Vec<usize> = b.iter().enumerate().filter(|(_, c)| **c == b'"').map(|(i, _)| i).collect();
        for pair in quotes.chunks_exact(2) {
            blank(&mut b, pair[0], pair[1] - pair[0] + 1);
        }
    }
    b
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Findings (`path:line: text`) in one file's text.
fn scan(path: &str, text: &str) -> Vec<String> {
    let name = old();
    let mut found = Vec::new();
    let mut history = false;
    let mut rename_section = false;
    for (n, line) in text.lines().enumerate() {
        if path == NOTES && line.starts_with(NOTES_MARKER) {
            history = true;
        }
        if line.starts_with("## ") {
            rename_section = path == RENAME_DOC && line == RENAME_HEADING;
        }
        let rename_row = rename_section && line.starts_with('|');
        if history || rename_row || !line.to_ascii_lowercase().contains(&name) {
            continue;
        }
        let rest = strip_allowed(path, line, &name);
        if rest.to_ascii_lowercase().windows(name.len()).any(|w| w == name.as_bytes()) {
            let shown: String = line.chars().take(120).collect();
            found.push(format!("{path}:{}: {}", n + 1, shown.trim()));
        }
    }
    found
}

pub fn check(root: &Path) -> Result<(), String> {
    let mut found = Vec::new();
    for path in crate::labnames::tracked_with(root, true)? {
        if allowed_path(&path) {
            continue;
        }
        if let Ok(bytes) = std::fs::read(root.join(&path)) {
            found.extend(scan(&path, &String::from_utf8_lossy(&bytes)));
        }
    }
    if found.is_empty() {
        return Ok(());
    }
    let first: Vec<&str> = found.iter().take(40).map(String::as_str).collect();
    Err(format!(
        "{} use(s) of the project's former name (write libniji, or `niji` in identifiers and paths; history and the allow-list are in xtask/src/oldname.rs):\n  {}",
        found.len(),
        first.join("\n  ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o() -> String {
        old()
    }

    fn cap() -> String {
        spell("%O", &old())
    }

    #[test]
    fn planted_old_name_is_caught_in_any_case() {
        for l in [format!("{} is a C library", cap()), format!("cargo test -p {}", o()), format!("{}_FULL_PROFILE=1", o().to_uppercase()), format!("the {}-gcc wrapper", o())] {
            assert_eq!(scan("docs/x.md", &l).len(), 1, "{l}");
        }
        assert_eq!(scan("crates/core/src/x.rs", &format!("// {}'s loader", cap())).len(), 1);
    }

    #[test]
    fn allowed_spellings_pass() {
        for t in ALLOWED {
            assert!(scan("README.md", &format!("see {} here", spell(t, &o()))).is_empty(), "{t}");
        }
        assert!(scan("plan/x.md", &format!("plan {}-SYSTEM-LIBC-PLAN and {}-UTILITIES-DESIGN", o().to_uppercase(), o().to_uppercase())).is_empty());
        assert!(scan("plan/x.md", &format!("Owner ruling \"{} as the system libc\" (2026-11-13)", cap())).is_empty());
        assert!(scan("xtask/src/records/schema.rs", &format!("[\"glibc\", \"{}\", \"restore\"]", o())).is_empty());
    }

    #[test]
    fn allowance_is_narrow() {
        // The quoted-ruling and column-key allowances do not spread to other lines or files.
        assert_eq!(scan("plan/x.md", &format!("a message \"{} failed\"", cap())).len(), 1);
        assert_eq!(scan("xtask/src/so.rs", &format!("[\"{}\"]", o())).len(), 1);
        // A story-id prefix followed by a lower-case word is not a story id.
        assert_eq!(scan("docs/x.md", &format!("{}-gcc", o().to_uppercase())).len(), 1);
        // An allowed token and a stray use on one line: the stray use is still caught.
        assert_eq!(scan("docs/x.md", &format!("{} and {}", spell(ALLOWED[1], &o()), cap())).len(), 1);
    }

    #[test]
    fn release_notes_history_starts_at_the_marker() {
        let text = format!("# libniji: release notes\n\n{} above\n{NOTES_MARKER} keep them. -->\n## {}: an old entry\n", cap(), cap());
        assert_eq!(scan(NOTES, &text), vec![format!("{NOTES}:3: {} above", cap())]);
        // Elsewhere the marker grants nothing.
        assert_eq!(scan("docs/x.md", &text).len(), 2);
    }

    #[test]
    fn rename_table_rows_are_allowed_only_in_their_section() {
        let row = format!("| packages `{}6` | `niji6` |", o());
        let text = format!("# Installing\n\n{RENAME_HEADING}\n\nThe {} prose.\n\n{row}\n\n## What they are\n\n{row}\n", cap());
        // In the rename section only the table row passes; its prose and a row under the next heading do not.
        assert_eq!(scan(RENAME_DOC, &text), vec![format!("{RENAME_DOC}:5: The {} prose.", cap()), format!("{RENAME_DOC}:11: {row}")]);
        // The same section in another file grants nothing.
        assert_eq!(scan("docs/x.md", &text).len(), 3);
    }

    #[test]
    fn history_paths_and_views_are_allowed() {
        for p in ["handoff/x.md", "records/decisions/D-1/D-1.1.toml", "results/x.md", "validation/results/x.tsv", "tools/rename_niji.py", "fuzz/seeds/ld_cache/writer-1", "plan/07-decisions/s04-byte-arrays-and-strings.md", "docs/saturation/03-memory-and-string-kernels.md", "docs/compat-flags.md"] {
            assert!(allowed_path(p), "{p}");
        }
        for p in ["plan/07-decisions/x3-regex-lazy-dfa.md", "docs/saturation/10-fresh-measurement.md", "docs/saturation/08-x.md", "plan/system-libc.md", "README.md", "tools/deb/build.sh"] {
            assert!(!allowed_path(p), "{p}");
        }
    }
}
