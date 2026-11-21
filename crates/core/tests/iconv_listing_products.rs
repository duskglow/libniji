//! `iconv -l` of the products lists exactly the names the command accepts, for the listed names
//! that stand for a charmap (`spec/encoding/iconv-aliases.tsv`) and the charmaps they stand for
//! (spec §31.6). Each name is given to the products' `iconv -f <name> -t UTF-8` with empty input;
//! it must be listed exactly when that command accepts it, whichever charmaps the host installs (a
//! row's charmap may be an installed file, a name in another file's header such as CP874 in
//! IBM874's, or absent). Needs the products and a `/usr/share/i18n/charmaps` directory; without
//! one it says so and skips.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The products' `iconv` with `args`, through the products' loader and libraries.
fn iconv(niji: &Path, args: &[&str]) -> std::process::Output {
    Command::new(niji.join("ld-linux-x86-64.so.2"))
        .arg(niji.join("iconv"))
        .args(args)
        .env_clear()
        .env("LD_LIBRARY_PATH", niji)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn iconv_listing_holds_the_alias_rows_it_accepts() {
    // Trace: s31_6_g_3 (`-l` lists the names the command accepts).
    let r = root();
    let niji = r.join("target/niji");
    assert!(niji.join("iconv").exists(), "the products are missing: run `cargo xtask so`");
    if !Path::new("/usr/share/i18n/charmaps").is_dir() {
        skip_note::skipped("skipped: no /usr/share/i18n/charmaps on this host");
        return;
    }
    let out = iconv(&niji, &["-l"]);
    assert!(out.status.success(), "iconv -l failed: {:?}", out.status);
    let listed: BTreeSet<String> = String::from_utf8(out.stdout).unwrap().lines().map(|l| l.trim_end_matches('/').to_ascii_uppercase()).collect();
    let table = std::fs::read_to_string(r.join("spec/encoding/iconv-aliases.tsv")).unwrap();
    let mut names = BTreeSet::new();
    for line in table.lines().filter(|l| !l.starts_with('#')) {
        let (name, charmap) = line.split_once('\t').unwrap();
        names.insert(name.to_owned());
        names.insert(charmap.to_owned());
    }
    let (mut wrong, mut accepted) = (Vec::new(), 0);
    for n in &names {
        let ok = iconv(&niji, &["-f", n, "-t", "UTF-8"]).status.success();
        accepted += usize::from(ok);
        if ok != listed.contains(n) {
            wrong.push(format!("{n} (accepted {ok}, listed {})", !ok));
        }
    }
    assert!(wrong.is_empty(), "listed and accepted differ: {wrong:?}");
    assert!(accepted > 0, "no alias row is accepted: the host's charmaps hold none of them");
}
