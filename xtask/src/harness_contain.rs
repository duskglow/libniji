//! `harness-contain` (ROOT-HARNESS-CONTAIN, `docs/testing/harness-containment.md`): a test that
//! starts processes through `sudo` or `unshare` goes through the containment helpers
//! (`crates/core/tests/support/contain.rs`), so that a killed or hung test leaves nothing behind.
//! Two rules over the tracked test sources (`crates/*/tests/**/*.rs`):
//!
//! * A line that spawns `sudo` or `unshare` directly (`Command::new("sudo")`, `Command::new("unshare")`)
//!   is a capability probe: it runs `true` (the literal `"true"` on the line), or carries
//!   `contain: probe(<why>)` on it or on one of the two lines above (a fixed command that ends by
//!   itself, such as a mount and exit). Any other run goes through `contain::user` / `contain::output`.
//! * A file that runs an oracle script (`oracle/<name>.sh`) whose code runs `sudo` or `unshare`, or
//!   whose own code has a `sudo -n` command line, uses `contain::`.
//!
//! ROOT-HARNESS-FOLLOWUP extends it past the tests:
//!
//! * xtask's own sources (`xtask/src/**/*.rs`): the line rule above, with `Command::new(unshare)` (a resolved
//!   path) too and a third marker, `contain: inside(<why>)`, for a spawn already inside a contained run. Other runs
//!   go through `xtask/src/contain.rs` (`contain::user`, `contain::output`).
//! * Scripts run by hand or by tooling (`validation/**/*.sh`, `tools/**/*.sh`, and the `oracle/*.sh` that no
//!   test names): one whose code runs `sudo` or `unshare` sources `tools/contained.sh`, with
//!   `contained_kind=root` or `rootopt` when it runs `sudo`.

use std::path::Path;

/// The containment helpers themselves.
const HELPERS: [&str; 6] = ["crates/core/tests/support/contain.rs", "crates/core/tests/support/oracle.rs", "xtask/src/contain.rs", "xtask/src/harness_contain.rs", "tools/contained.sh", "crates/core/tests/oracle/contain.sh"];

/// The spawns the line rule looks for.
const SPAWNS: [&str; 4] = ["Command::new(\"sudo\")", "Command::new(\"unshare\")", "Command::new(unshare)", "Command::new(&unshare)"];

fn is_xtask_source(path: &str) -> bool {
    path.starts_with("xtask/src/") && path.ends_with(".rs") && !HELPERS.contains(&path)
}

/// A script in scope of the script rule (`hand`: an oracle script that no test names).
fn is_tool_script(path: &str, hand: bool) -> bool {
    let tool = (path.starts_with("validation/") || path.starts_with("tools/")) && path.ends_with(".sh");
    let oracle = path.starts_with("crates/") && path.contains("/tests/oracle/") && path.ends_with(".sh") && hand;
    (tool || oracle) && !HELPERS.contains(&path)
}

/// The finding for one script, if any: it runs `sudo` or `unshare` without sourcing `tools/contained.sh`
/// (with a root kind when it runs `sudo`).
pub fn scan_script(path: &str, text: &str) -> Option<String> {
    if !script_namespaces(text) {
        return None;
    }
    let code: Vec<&str> = text.lines().map(|l| l.split('#').next().unwrap_or("")).collect();
    let sources = code.iter().any(|l| l.trim_start().starts_with(". ") && l.contains("tools/contained.sh"));
    let kind = code.iter().find_map(|l| l.split_whitespace().find_map(|w| w.strip_prefix("contained_kind=")));
    let sudo = code.iter().any(|l| l.contains("sudo "));
    match (sources, kind) {
        (true, Some("root" | "rootopt")) => None,
        (true, Some("user")) if !sudo => None,
        (true, Some(k)) => Some(format!("{path}: runs sudo under contained_kind={k}; use contained_kind=root or rootopt (tools/contained.sh)")),
        _ => Some(format!("{path}: starts processes through sudo or unshare without sourcing tools/contained.sh (contained_kind=root|rootopt|user)")),
    }
}

/// The line rule: a direct spawn of `sudo` or `unshare` is a probe or is marked.
fn direct_spawns(path: &str, lines: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let c = code(line);
        if !SPAWNS.iter().any(|s| c.contains(s)) {
            continue;
        }
        let marked = lines[i.saturating_sub(2)..=i].iter().any(|l| l.contains("contain: probe(") || l.contains("contain: inside("));
        if !c.contains("\"true\"") && !marked {
            found.push(format!("{path}:{}: spawns sudo/unshare directly; run it through contain::user or contain::output (support/contain.rs, xtask/src/contain.rs), or mark it `contain: probe(<why>)` / `contain: inside(<why>)`", i + 1));
        }
    }
    found
}

fn is_test_source(path: &str) -> bool {
    path.starts_with("crates/") && path.contains("/tests/") && path.ends_with(".rs") && !HELPERS.contains(&path)
}

/// The code of a line: the text before a `//` comment (good enough for these patterns).
fn code(line: &str) -> &str {
    line.find("//").map_or(line, |i| &line[..i])
}

/// Whether a script's code (comments and blank lines aside) runs `sudo` or `unshare`.
pub fn script_namespaces(text: &str) -> bool {
    text.lines().map(|l| l.split('#').next().unwrap_or("")).any(|l| l.contains("sudo ") || l.contains("unshare "))
}

/// The findings for one test source; `script` says whether a named oracle script runs `sudo` or `unshare`.
pub fn scan(path: &str, src: &str, script: &dyn Fn(&str) -> bool) -> Vec<String> {
    let lines: Vec<&str> = src.lines().collect();
    let mut found = direct_spawns(path, &lines);
    if !src.contains("contain::") {
        let named = lines.iter().map(|l| code(l)).find_map(|c| {
            let at = c.find("oracle/")?;
            let rest = &c[at + "oracle/".len()..];
            let end = rest.find(".sh")?;
            let name = &rest[..end];
            (!name.contains(['"', ' ', '/']) && (script(name) || c.contains("sudo -n"))).then(|| name.to_string())
        });
        let sudo_line = lines.iter().position(|l| code(l).contains("sudo -n "));
        if let Some(name) = named {
            found.push(format!("{path}: runs oracle/{name}.sh, which starts processes through sudo or unshare, without contain:: (support/contain.rs)"));
        } else if let Some(i) = sudo_line {
            found.push(format!("{path}:{}: a `sudo -n` command line without contain:: (support/contain.rs)", i + 1));
        }
    }
    found
}

pub fn check(root: &Path) -> Result<(), String> {
    let script = |name: &str| std::fs::read_to_string(root.join(format!("crates/core/tests/oracle/{name}.sh"))).is_ok_and(|t| script_namespaces(&t));
    let mut found = Vec::new();
    let tracked = crate::labnames::tracked_with(root, true)?;
    let tests: Vec<String> = tracked.iter().filter(|p| p.starts_with("crates/") && p.contains("/tests/")).filter_map(|p| std::fs::read_to_string(root.join(p)).ok().map(|t| format!("{p}\n{t}"))).collect();
    let named = |path: &str| {
        let file = path.rsplit('/').next().unwrap_or(path);
        tests.iter().any(|t| !t.starts_with(&format!("{path}\n")) && t.contains(file))
    };
    for path in &tracked {
        let Ok(src) = std::fs::read_to_string(root.join(path)) else { continue };
        if is_test_source(path) {
            found.extend(scan(path, &src, &script));
        } else if is_xtask_source(path) {
            found.extend(direct_spawns(path, &src.lines().collect::<Vec<_>>()));
        } else if is_tool_script(path, path.contains("/tests/oracle/") && !named(path)) {
            found.extend(scan_script(path, &src));
        }
    }
    if found.is_empty() {
        return Ok(());
    }
    Err(format!("{} uncontained harness run(s) (docs/testing/harness-containment.md):\n  {}", found.len(), found.join("\n  ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ROOT-HARNESS-CONTAIN: no spec clause (developer tooling). Probes pass, a direct run and an
    // uncontained root script are named, a contained file passes.
    #[test]
    fn direct_runs_and_uncontained_scripts_are_named() {
        let none = |_: &str| false;
        let probe = "if Command::new(\"sudo\").args([\"-n\", \"true\"]).status().is_ok() {}\n";
        assert!(scan("t.rs", probe, &none).is_empty());
        let marked = "// contain: probe(mounts and exits)\nlet ok = Command::new(\"unshare\").args([\"-r\", \"sh\", \"-c\", \"mount x\"]).status();\n";
        assert!(scan("t.rs", marked, &none).is_empty());
        let direct = "let o = Command::new(\"unshare\").args([\"-r\"]).arg(exe).output();\n";
        assert_eq!(scan("t.rs", direct, &none).len(), 1);
        let sudo = "let o = Command::new(\"sudo\").args([\"-n\", \"id\"]).output();\n";
        assert!(scan("t.rs", sudo, &none)[0].contains("t.rs:1:"));
        let root = |n: &str| n == "x_run";
        let file = "let s = r.join(\"crates/core/tests/oracle/x_run.sh\");\n";
        assert_eq!(scan("t.rs", file, &root).len(), 1);
        let contained = "let s = r.join(\"crates/core/tests/oracle/x_run.sh\");\nlet o = oracle::contain::output(&c);\n";
        assert!(scan("t.rs", contained, &root).is_empty());
        assert!(script_namespaces("# sudo is not here\nsudo -n unshare -m true\n") && !script_namespaces("# sudo -n x\necho\n"));
    }

    // ROOT-HARNESS-FOLLOWUP: no spec clause (developer tooling). Past the tests: an xtask spawn of a resolved
    // `unshare` is named unless marked `inside(...)`; a script that runs sudo or unshare must source
    // tools/contained.sh, with a root kind for sudo; scope covers by-hand oracle scripts only.
    #[test]
    fn xtask_spawns_and_tool_scripts_are_named() {
        let direct = ["let mut c = Command::new(unshare);"];
        assert_eq!(direct_spawns("x.rs", &direct).len(), 1);
        let inside = ["// contain: inside(started through contain::user)", "Command::new(unshare).arg(\"--user\").exec()"];
        assert!(direct_spawns("x.rs", &inside).is_empty());
        let bare = "#!/bin/sh\nset -eu\nunshare -r -m sh -c 'mount x'\n";
        assert!(scan_script("t.sh", bare).is_some_and(|f| f.contains("without sourcing")));
        let prologue = "contained_kind=user contained_tree=$(pwd)\n. \"$contained_tree/tools/contained.sh\"\n";
        let user = format!("#!/bin/sh\n{prologue}unshare -r -m true\n");
        assert!(scan_script("t.sh", &user).is_none());
        let sudo_user = format!("#!/bin/sh\n{prologue}sudo -n perf stat true\n");
        assert!(scan_script("t.sh", &sudo_user).is_some_and(|f| f.contains("contained_kind=user")));
        let sudo_root = sudo_user.replace("contained_kind=user", "contained_kind=root");
        assert!(scan_script("t.sh", &sudo_root).is_none());
        assert!(scan_script("t.sh", "# unshare -r in a comment\necho\n").is_none());
        assert!(is_tool_script("validation/p.sh", false) && is_tool_script("crates/core/tests/oracle/h.sh", true));
        assert!(!is_tool_script("crates/core/tests/oracle/t.sh", false) && !is_tool_script("tools/contained.sh", false));
    }
}
