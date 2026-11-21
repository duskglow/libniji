//! AFFECTED-COVERAGE: the test sets the ordinary runs do not build, the runs that close them, and the check that
//! every test set has a runner.
//!
//! The runners: the direct-tier gate (`cargo xtask affected --run`: per crate `cargo test -p <c>` with the features
//! the workspace gives it, then the native pass and the interposed pass with `nextest::INTERPOSED_FEATURES`), the CI
//! train (`cargo xtask affected --tier full --run`, which is `cargo xtask test`: `cargo test --workspace`, the native
//! pass over the workspace (`native_tests`), the interposed suite, conformance, `native-examples`, `rust-dylib`) and
//! the nightly and release runs (the `#[ignore]`d long sweeps, `longsweeps.rs`). `cargo test --workspace` builds each
//! member with the features its members and dev-dependencies turn on, so test code under any other crate feature
//! (`#[cfg(feature = "ld")] pub mod ld;` and every unit test in it) is compiled by clippy (`--all-features`) and run
//! by nothing; the same holds for a crate outside the workspace (`fuzz/`, a workspace of its own).
//!
//! [`RUNS`] runs those sets: in the direct tier when a file a row covers changes (`affected`, [`runs_for`]) and in
//! the train (`cargo xtask test`, step `test-sets`, [`run_all`]). [`check`] (`cargo xtask static`, `test-coverage`)
//! fails when a crate feature is neither run nor declared product-only, when such a feature gates test code, when a
//! crate under `crates/` holds tests and is not a workspace member, when a file that names a run's feature is not
//! among the files the row covers, or when the train's steps no longer run the native pass, the interposed suite and
//! these sets.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A test set no ordinary run builds, run on its own: `cargo test -p <krate> [--features <features>] [--lib]
/// [--test <t>]...` in `dir` (the root, or `fuzz` with its pinned nightly).
pub struct Run {
    /// The workspace directory relative to the root: `""` (the root workspace) or `"fuzz"`.
    pub dir: &'static str,
    pub krate: &'static str,
    /// The crate features (`a,b`), empty for none.
    pub features: &'static str,
    /// The crate's unit tests (`--lib`).
    pub lib: bool,
    /// Its test targets (`--test <t>`).
    pub tests: &'static [&'static str],
    /// The files whose change selects the run in the direct tier; one ending in `/` is a directory. [`check`] keeps
    /// it complete: every file of the crate that names the feature (`feature = "<f>"`) or a cfg in `cfgs` is here.
    pub covers: &'static [&'static str],
    /// The cfg names the build script sets from the feature (`niji_portable_scan` from `portable-scan`).
    pub cfgs: &'static [&'static str],
    /// The tests it holds and why nothing else runs them (the coverage table of `--list`).
    pub why: &'static str,
}

/// The runs (AFFECTED-COVERAGE, D-X.NEW pending): measured on the Zen 1 host, 60 vCPU, warm dependencies: `ld` 47 s,
/// `portable-scan` 55 s, `fastpath-stats` 45 s, the fuzz library 29 s (each a build of its own: the features change
/// the crate's code).
pub const RUNS: &[Run] = &[
    Run {
        dir: "",
        krate: "niji-boundary",
        features: "ld",
        lib: true,
        tests: &[],
        covers: &["crates/boundary/src/ld/", "crates/boundary/src/ld.rs", "crates/boundary/src/arch/", "crates/boundary/src/bootstrap.rs", "crates/boundary/src/lib.rs", "crates/boundary/src/posix/s27_dynlink.rs", "crates/boundary/src/dispatch.rs"],
        cfgs: &[],
        why: "the loader's unit tests (modules `ld`, `arch`, `bootstrap`: compiled only with `ld`, which only the ld.so and ldconfig products enable)",
    },
    Run {
        dir: "",
        krate: "niji-boundary",
        features: "portable-scan",
        lib: true,
        tests: &["machine_scanners"],
        covers: &["crates/boundary/src/strz.rs", "crates/boundary/src/ffi.rs", "crates/boundary/src/dispatch.rs", "crates/boundary/src/selftest.rs", "crates/boundary/build.rs", "crates/boundary/tests/machine_scanners.rs"],
        cfgs: &["niji_portable_scan"],
        why: "the portable Rust scanners in place of scan.S (A-8, D-4.22 PORTABLE-SCAN): machine_scanners and the unit tests under `cfg(niji_portable_scan)`",
    },
    Run {
        dir: "",
        krate: "niji-math",
        features: "fastpath-stats",
        lib: true,
        tests: &[],
        covers: &["crates/math/src/stats.rs"],
        cfgs: &[],
        why: "the fast-path counters (`stats::tests`, compiled only with `fastpath-stats`)",
    },
    Run {
        dir: "",
        krate: "niji-native",
        features: "dylib",
        lib: true,
        tests: &[],
        covers: &["crates/native/src/dylib.rs", "crates/native/src/lib.rs", "crates/native/src/ffi.rs"],
        cfgs: &[],
        why: "the Rust dylib's ABI note and symbol (`dylib::tests`, compiled only with `dylib`; the rust-dylib step builds the example, not the tests)",
    },
    Run {
        dir: "fuzz",
        krate: "niji-fuzz",
        features: "",
        lib: true,
        tests: &[],
        covers: &["fuzz/src/lib.rs"],
        cfgs: &[],
        why: "the fuzz harnesses' support library (`fuzz/` is a workspace of its own; `fuzz-check` only type-checks the targets)",
    },
];

/// The crate features the ordinary runners build their tests with: `(crate, feature, runner, the runner runs the
/// crate's tests under it)`. A runner that only builds examples under a feature (`false`) leaves test code under it
/// unrun, so [`check`] treats it as product-only for that.
pub const RUN_BY: &[(&str, &str, &str, bool)] = &[
    ("niji-boundary", "selftest", "every `cargo test` (niji-boundary's own dev-dependency, and niji's, niji-math's and niji-rt's)", true),
    ("niji-boundary", "export", "the interposed pass (`nextest::INTERPOSED_FEATURES`)", true),
    ("niji", "selftest", "every `cargo test` (through niji-boundary's `selftest`)", true),
    ("niji-legacy", "selftest", "every `cargo test` (through niji-boundary's `selftest`)", true),
    ("niji-native", "static", "`cargo xtask test` step native-examples (the examples only)", false),
    ("niji-native", "dynamic", "`cargo xtask test` step native-examples (the examples only)", false),
];

/// The features only a product enables, `(crate, feature, why)`: [`check`] proves no test code sits under them.
pub const PRODUCT_ONLY: &[(&str, &str, &str)] = &[
    ("niji-abi", "export", "the products' exported data"),
    ("niji-boundary", "runtime", "libc.so.6's process runtime; a host-started test process must keep the host's (D-4.13)"),
    ("niji-boundary", "libm", "the libm product"),
    ("niji-boundary", "libresolv", "the libresolv product"),
    ("niji-boundary", "ldconfig", "the ldconfig product; its tests are those of `ld`, which it implies"),
];

/// The train's steps that run every test binary again as libniji processes and with the exports interposed, and
/// these sets: the text `cargo xtask test` must hold (`xtask/src/main.rs`), so the train keeps running them.
const TRAIN_STEPS: &[(&str, &str)] = &[
    ("native_tests(root, &cargo, extra, t, \"tests-native\")", "the native pass (pass 2: every test binary that holds niji-boundary code, as a libniji process)"),
    ("pass2::patch_listed(cargo, root, &[\"--workspace\".to_string()])", "the native pass covers the whole workspace"),
    ("interposed_suite(root, &cargo, extra, t, \"tests-interposed\")", "the interposed suite"),
    ("coverage::run_all(root, &cargo)", "the test sets of `coverage::RUNS`"),
];

impl Run {
    /// The run's name: `niji-boundary+ld`, `niji-fuzz` (no feature).
    pub fn id(&self) -> String {
        if self.features.is_empty() { self.krate.to_string() } else { format!("{}+{}", self.krate, self.features) }
    }

    /// Whether a change to `f` (a path from the root) selects the run.
    pub fn covers_file(&self, f: &str) -> bool {
        self.covers.iter().any(|c| if c.ends_with('/') { f.starts_with(c) } else { f == *c })
    }

    /// The arguments after `cargo test`.
    pub fn args(&self) -> Vec<String> {
        let mut a = vec!["-p".to_string(), self.krate.to_string()];
        if !self.features.is_empty() {
            a.extend(["--features".to_string(), self.features.to_string()]);
        }
        if self.lib {
            a.push("--lib".to_string());
        }
        for t in self.tests {
            a.extend(["--test".to_string(), t.to_string()]);
        }
        if !self.dir.is_empty() {
            a.push("--locked".to_string());
        }
        a
    }

    /// `cargo test <args>` for this run: the root's cargo, or the fuzz workspace's pinned nightly.
    pub fn command(&self, root: &Path, cargo: &str) -> Result<Command, String> {
        let mut c = if self.dir == "fuzz" {
            crate::fuzz::nightly_cargo(root, &crate::fuzz::toolchain(root)?)
        } else {
            let mut c = Command::new(cargo);
            c.current_dir(root);
            c
        };
        crate::nextest::poison(&mut c);
        c.arg("test").args(self.args());
        Ok(c)
    }
}

/// The run named `id` ([`Run::id`]).
pub fn by_id(id: &str) -> Option<&'static Run> {
    RUNS.iter().find(|r| r.id() == id)
}

/// The runs a change to `f` selects.
pub fn runs_for(f: &str) -> impl Iterator<Item = &'static Run> + '_ {
    RUNS.iter().filter(move |r| r.covers_file(f))
}

/// The train's step (`cargo xtask test`): every run, each reported; `Err` names the red ones.
pub fn run_all(root: &Path, cargo: &str) -> Result<(), String> {
    let mut red = Vec::new();
    for r in RUNS {
        println!("test set {}: cargo test {} ({})", r.id(), r.args().join(" "), r.why);
        let ok = r.command(root, cargo).and_then(|mut c| c.status().map_err(|e| format!("cannot run cargo: {e}"))).is_ok_and(|s| s.success());
        if !ok {
            red.push(r.id());
        }
    }
    if red.is_empty() { Ok(()) } else { Err(format!("test sets failed: {}", red.join(", "))) }
}

/// A member's manifest: its package name and its features.
fn manifest(text: &str) -> (String, BTreeSet<String>) {
    let mut name = String::new();
    let mut features = BTreeSet::new();
    let mut section = "";
    for l in text.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            section = if t == "[package]" { "package" } else if t == "[features]" { "features" } else { "" };
            continue;
        }
        let Some((k, v)) = t.split_once('=') else { continue };
        let k = k.trim();
        if section == "package" && k == "name" && name.is_empty() {
            name = v.trim().trim_matches('"').to_string();
        } else if section == "features" && !k.starts_with('#') && k != "default" {
            features.insert(k.to_string());
        }
    }
    (name, features)
}

/// The workspace members (`members = [...]` of the root manifest), as directories.
fn members(root_manifest: &str) -> Vec<String> {
    let Some(rest) = root_manifest.split_once("members = [").map(|(_, r)| r) else { return Vec::new() };
    let list = rest.split_once(']').map_or("", |(l, _)| l);
    list.split(',').map(|s| s.trim().trim_matches('"').to_string()).filter(|s| !s.is_empty()).collect()
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// The calls enclosing each `feature = "<f>"` of a cfg predicate: true when the feature is required (no `not(`
/// around it), with whether the predicate also requires `test`.
fn feature_in_cfg(pred: &str, feature: &str) -> Option<(bool, bool)> {
    let needle = format!("feature = \"{feature}\"");
    let at = pred.find(&needle)?;
    let mut stack: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut positive_test = false;
    for (i, ch) in pred.char_indices() {
        if i == at {
            let required = !stack.iter().any(|s| s == "not");
            // `test` required beside it: an `all(..., test, ...)` level or the whole predicate.
            let test = positive_test || pred.trim() == "test";
            return Some((required, test));
        }
        match ch {
            '(' => {
                stack.push(std::mem::take(&mut word));
            }
            ')' => {
                if word == "test" && !stack.iter().any(|s| s == "not" || s == "any") {
                    positive_test = true;
                }
                word.clear();
                stack.pop();
            }
            ',' | ' ' => {
                if word == "test" && !stack.iter().any(|s| s == "not" || s == "any") {
                    positive_test = true;
                }
                word.clear();
            }
            c if c.is_alphanumeric() || c == '_' => word.push(c),
            _ => word.clear(),
        }
    }
    None
}

/// Test code under a required `feature = "<f>"` in `src` (a file at `path`): `(line, what)` for a
/// `cfg(all(test, feature = ...))` item, or a gated `mod <m>;` whose files hold a `#[test]`, or a gated inline module
/// holding one.
fn gated_tests(path: &Path, src: &str, feature: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        let Some(pred) = t.strip_prefix("#[cfg(").and_then(|r| r.strip_suffix(")]")) else { continue };
        let Some((true, test)) = feature_in_cfg(pred, feature) else { continue };
        if test {
            out.push((i + 1, format!("`#[cfg({pred})]`")));
            continue;
        }
        // The gated item: the next line that is not an attribute or a comment.
        let Some((j, item)) = lines.iter().enumerate().skip(i + 1).find(|(_, l)| !l.trim_start().starts_with("#[") && !l.trim_start().starts_with("//")) else { continue };
        let item = item.trim_start();
        let item = item.strip_prefix("pub(crate) ").or_else(|| item.strip_prefix("pub ")).unwrap_or(item);
        let Some(m) = item.strip_prefix("mod ") else { continue };
        let name: String = m.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if m[name.len()..].trim_start().starts_with('{') {
            // An inline module: up to its closing brace at its own indentation.
            let indent = lines[j].len() - lines[j].trim_start().len();
            let end = lines.iter().enumerate().skip(j + 1).find(|(_, l)| l.len() - l.trim_start().len() == indent && l.trim_start().starts_with('}')).map_or(lines.len(), |(k, _)| k);
            if lines[j..end].iter().any(|l| l.trim_start().starts_with("#[test]")) {
                out.push((i + 1, format!("`mod {name}` under `#[cfg({pred})]`")));
            }
            continue;
        }
        // `mod <name>;`: `<dir>/<name>.rs` and `<dir>/<name>/`, where `<dir>` is the file's own directory for a
        // `lib.rs`, `main.rs` or `mod.rs`, else `<file stem>/`.
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parent = path.parent().unwrap_or(Path::new(""));
        let dir = if matches!(stem, "lib" | "main" | "mod") { parent.to_path_buf() } else { parent.join(stem) };
        let mut files = vec![dir.join(format!("{name}.rs"))];
        rs_files(&dir.join(&name), &mut files);
        let n: usize = files.iter().filter_map(|f| fs::read_to_string(f).ok()).map(|s| s.lines().filter(|l| l.trim_start().starts_with("#[test]")).count()).sum();
        if n > 0 {
            out.push((i + 1, format!("`mod {name};` under `#[cfg({pred})]` ({n} tests)")));
        }
    }
    out
}

/// The `#[test]` functions under `dir`.
fn test_count(dir: &Path) -> usize {
    let mut files = Vec::new();
    rs_files(dir, &mut files);
    files.iter().filter_map(|f| fs::read_to_string(f).ok()).map(|s| s.lines().filter(|l| l.trim_start().starts_with("#[test]")).count()).sum()
}

/// `cargo xtask static` step `test-coverage` (see the module text).
pub fn check(root: &Path) -> Result<(), String> {
    let problems = problems(root)?;
    if problems.is_empty() {
        return Ok(());
    }
    for p in &problems {
        eprintln!("test-coverage: {p}");
    }
    Err(format!("{} test set(s) no runner runs, or a stale row (xtask/src/coverage.rs)", problems.len()))
}

fn problems(root: &Path) -> Result<Vec<String>, String> {
    let read = |p: &str| fs::read_to_string(root.join(p)).map_err(|e| format!("{p}: {e}"));
    let mut problems = Vec::new();
    let mut crates: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
    let member_dirs = members(&read("Cargo.toml")?);
    for d in &member_dirs {
        let (name, features) = manifest(&read(&format!("{d}/Cargo.toml"))?);
        crates.insert(name, (d.clone(), features));
    }
    let fuzz = manifest(&read("fuzz/Cargo.toml")?);
    let feature_of = |c: &str, f: &str| crates.get(c).is_some_and(|(_, fs)| fs.contains(f));
    // 1. Every feature of every member: run, or product-only.
    for (name, (dir, features)) in &crates {
        for f in features {
            let run = RUNS.iter().any(|r| r.krate == name && r.features.split(',').any(|x| x == f));
            let by = RUN_BY.iter().find(|(c, x, _, _)| c == name && x == f);
            let product = PRODUCT_ONLY.iter().any(|(c, x, _)| c == name && x == f);
            if [run, by.is_some(), product].iter().filter(|b| **b).count() != 1 {
                problems.push(format!("{name} feature `{f}` ({dir}/Cargo.toml): name it once, in coverage::RUNS (a runner runs its tests), RUN_BY or PRODUCT_ONLY"));
                continue;
            }
            // 2. A feature whose tests no runner runs gates no test code.
            if product || by.is_some_and(|(_, _, _, tests)| !tests) {
                let mut files = Vec::new();
                rs_files(&root.join(dir).join("src"), &mut files);
                rs_files(&root.join(dir).join("tests"), &mut files);
                for p in files {
                    let src = fs::read_to_string(&p).unwrap_or_default();
                    for (line, what) in gated_tests(&p, &src, f) {
                        problems.push(format!("{}:{line}: {what}: test code under `{f}`, which no runner runs it with (add a coverage::RUNS row)", p.strip_prefix(root).unwrap_or(&p).display()));
                    }
                }
            }
        }
    }
    // 3. The rows name real crates, features, targets and files, and cover every file that names their feature.
    for r in RUNS {
        let (dir, known) = if r.dir.is_empty() {
            match crates.get(r.krate) {
                Some((d, _)) => (d.clone(), r.features.split(',').filter(|f| !f.is_empty()).all(|f| feature_of(r.krate, f))),
                None => (String::new(), false),
            }
        } else {
            (r.dir.to_string(), fuzz.0 == r.krate && r.features.is_empty())
        };
        if !known {
            problems.push(format!("coverage::RUNS {}: no such crate or feature", r.id()));
            continue;
        }
        for c in r.covers {
            if !root.join(c.trim_end_matches('/')).exists() {
                problems.push(format!("coverage::RUNS {}: covers `{c}`, which does not exist", r.id()));
            }
        }
        for t in r.tests {
            if !root.join(&dir).join("tests").join(format!("{t}.rs")).exists() {
                problems.push(format!("coverage::RUNS {}: no test target `{t}`", r.id()));
            }
        }
        let mut names: Vec<String> = r.features.split(',').filter(|f| !f.is_empty()).map(|f| format!("feature = \"{f}\"")).collect();
        names.extend(r.cfgs.iter().map(|c| c.to_string()));
        // What the run builds: the crate's sources, its tests and its build script (not its examples).
        let mut files = Vec::new();
        rs_files(&root.join(&dir).join("src"), &mut files);
        rs_files(&root.join(&dir).join("tests"), &mut files);
        files.push(root.join(&dir).join("build.rs"));
        for p in files {
            let rel = p.strip_prefix(root).unwrap_or(&p).display().to_string();
            if rel.contains("/target/") {
                continue;
            }
            // A `cfg` that compiles code in or out (a `cfg_attr` only changes a lint or an attribute).
            let src = fs::read_to_string(&p).unwrap_or_default();
            let names_it = src.lines().filter(|l| !l.contains("cfg_attr(")).any(|l| names.iter().any(|n| l.contains(n.as_str())));
            if names_it && !r.covers_file(&rel) {
                problems.push(format!("coverage::RUNS {}: {rel} names its feature or cfg but is not among the files it covers", r.id()));
            }
        }
    }
    // 4. A crate under `crates/` that is not a member holds no test (`cargo test --workspace` never builds it).
    let mut dirs: Vec<PathBuf> = fs::read_dir(root.join("crates")).map_err(|e| format!("crates: {e}"))?.flatten().map(|e| e.path()).filter(|p| p.join("Cargo.toml").exists()).collect();
    dirs.sort();
    for d in dirs {
        let rel = d.strip_prefix(root).unwrap_or(&d).display().to_string();
        let n = test_count(&d);
        if !member_dirs.contains(&rel) && n > 0 {
            problems.push(format!("{rel}: {n} tests in a crate outside the workspace, which no runner builds (make it a member or add a coverage::RUNS row)"));
        }
    }
    // The fuzz workspace's tests: its row.
    if test_count(&root.join("fuzz/src")) > 0 && !RUNS.iter().any(|r| r.dir == "fuzz") {
        problems.push("fuzz/src: tests in the fuzz workspace, which no runner runs (a coverage::RUNS row)".to_string());
    }
    // 5. The train keeps its passes and these sets.
    let main = read("xtask/src/main.rs")?;
    for (text, what) in TRAIN_STEPS {
        if !main.contains(text) {
            problems.push(format!("xtask/src/main.rs: `cargo xtask test` (the train) no longer runs {what} (`{text}` is gone)"));
        }
    }
    Ok(problems)
}

#[cfg(test)]
mod tests {
    // no spec clause: tooling (AFFECTED-COVERAGE: the test sets no runner runs).
    use super::*;

    #[test]
    fn a_required_feature_with_test_is_test_code_and_a_negated_one_is_not() {
        assert_eq!(feature_in_cfg("all(test, feature = \"ld\")", "ld"), Some((true, true)));
        assert_eq!(feature_in_cfg("feature = \"ld\"", "ld"), Some((true, false)));
        assert_eq!(feature_in_cfg("all(feature = \"libresolv\", not(feature = \"runtime\"))", "runtime"), Some((false, false)));
        assert_eq!(feature_in_cfg("all(feature = \"libresolv\", not(feature = \"runtime\"))", "libresolv"), Some((true, false)));
        assert_eq!(feature_in_cfg("any(test, feature = \"ld\")", "ld"), Some((true, false)), "`any(test, ...)` does not require test");
        assert_eq!(feature_in_cfg("feature = \"ld\"", "runtime"), None);
    }

    #[test]
    fn gated_test_modules_are_found_inline_by_cfg_and_by_file() {
        let dir = std::env::temp_dir().join(format!("coverage-gated-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("ldx")).unwrap();
        fs::write(dir.join("ldx/a.rs"), "#[test]\nfn t() {}\n").unwrap();
        fs::write(dir.join("plain.rs"), "fn f() {}\n").unwrap();
        let lib = "#[cfg(feature = \"x\")]\npub mod ldx;\n#[cfg(feature = \"x\")]\nmod plain;\n#[cfg(all(test, feature = \"x\"))]\nmod t {}\n#[cfg(feature = \"x\")]\nmod inl {\n    #[test]\n    fn u() {}\n}\n#[cfg(not(feature = \"x\"))]\nmod n {\n    #[test]\n    fn v() {}\n}\n";
        let found: Vec<usize> = gated_tests(&dir.join("lib.rs"), lib, "x").into_iter().map(|(l, _)| l).collect();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(found, vec![1, 5, 7], "the file module with a test, the test cfg, the inline module; not `plain`, not `not(x)`");
    }

    #[test]
    fn members_and_features_are_read_from_the_manifests() {
        assert_eq!(members("[workspace]\nmembers = [\n    \"crates/a\",\n    \"xtask\",\n]\nexclude = [\"b\"]\n"), vec!["crates/a", "xtask"]);
        let (name, f) = manifest("[package]\nname = \"niji-x\"\n[features]\n# c\ndefault = []\nld = []\nb = [\"ld\"]\n[dependencies]\nz = \"1\"\n");
        assert_eq!((name.as_str(), f.into_iter().collect::<Vec<_>>()), ("niji-x", vec!["b".to_string(), "ld".to_string()]));
    }

    #[test]
    fn a_run_is_selected_by_its_files_and_names_its_command() {
        let ld = by_id("niji-boundary+ld").unwrap();
        assert!(ld.covers_file("crates/boundary/src/ld/image.rs") && !ld.covers_file("crates/boundary/src/ldconfig.rs"));
        assert_eq!(ld.args(), ["-p", "niji-boundary", "--features", "ld", "--lib"]);
        assert_eq!(by_id("niji-fuzz").unwrap().args(), ["-p", "niji-fuzz", "--lib", "--locked"]);
        assert_eq!(runs_for("crates/boundary/tests/machine_scanners.rs").map(Run::id).collect::<Vec<_>>(), ["niji-boundary+portable-scan"]);
    }

    #[test]
    fn the_tree_has_no_test_set_without_a_runner() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert_eq!(problems(root).unwrap(), Vec::<String>::new());
    }
}
