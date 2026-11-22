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

use std::path::Path;

/// The containment helpers themselves.
const HELPERS: [&str; 2] = ["crates/core/tests/support/contain.rs", "crates/core/tests/support/oracle.rs"];

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
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let c = code(line);
        if !(c.contains("Command::new(\"sudo\")") || c.contains("Command::new(\"unshare\")")) {
            continue;
        }
        let marked = lines[i.saturating_sub(2)..=i].iter().any(|l| l.contains("contain: probe("));
        if !c.contains("\"true\"") && !marked {
            found.push(format!("{path}:{}: spawns sudo/unshare directly; run it through contain::user or contain::output (support/contain.rs), or mark a probe `contain: probe(<why>)`", i + 1));
        }
    }
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
    for path in crate::labnames::tracked_with(root, true)? {
        if !is_test_source(&path) {
            continue;
        }
        if let Ok(src) = std::fs::read_to_string(root.join(&path)) {
            found.extend(scan(&path, &src, &script));
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
}
