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

#[path = "support/oracle.rs"]
mod oracle;

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

const DIR: &str = "/usr/share/i18n/charmaps";

/// A private charmap directory's files: (file stem, gzip-compressed, header lines, the byte `é` maps to). Their
/// `<code_set_name>` and alias names are carried by no file name, built-in name or alias row. The plain one
/// sets `<comment_char> %`; the first gzip one keeps the default `#` and spells its alias in lower case; the
/// second puts its names after 3 KiB of comment lines (several deflate blocks' worth of back-references).
fn private_charmaps() -> Vec<(&'static str, bool, String, u8)> {
    let long: String = (0..60).map(|i| format!("% comment line {i} before the names of the third charmap\n")).collect();
    vec![
        ("LISTP", false, "<comment_char> %\n<escape_char> /\n<code_set_name> LISTP-SET\n% alias LISTP-ALIAS\n".to_string(), 0xc1),
        ("LISTG", true, "<code_set_name> LISTG-SET\n<mb_cur_max> 1\n# alias listg-alias\n".to_string(), 0xc2),
        ("LISTD", true, format!("<comment_char> %\n{long}<code_set_name> LISTD-SET\n%  alias   LISTD-ALIAS  \n"), 0xc3),
    ]
}

/// The products' `iconv args` with the directory `charmaps` mounted over [`DIR`], in a user and mount
/// namespace (contained: support/contain.rs).
fn iconv_private(niji: &Path, charmaps: &Path, args: &[&str]) -> std::process::Output {
    oracle::contain::user("unshare")
        .args(["-r", "-m", "sh", "-c", "mount --bind \"$1\" \"$2\" && shift 2 && exec \"$@\"", "sh"])
        .arg(charmaps)
        .arg(DIR)
        .arg(niji.join("ld-linux-x86-64.so.2"))
        .arg(niji.join("iconv"))
        .args(args)
        .env_clear()
        .env("LD_LIBRARY_PATH", niji)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// `text` converted from `from` to `to` by the products' iconv over the private directory, if it converts.
fn convert(niji: &Path, charmaps: &Path, work: &Path, from: &str, to: &str, text: &[u8]) -> Option<Vec<u8>> {
    let input = work.join("input");
    std::fs::write(&input, text).unwrap();
    let o = iconv_private(niji, charmaps, &["-f", from, "-t", to, input.to_str().unwrap()]);
    o.status.success().then_some(o.stdout)
}

#[test]
fn iconv_listing_holds_the_names_charmap_headers_give() {
    // Trace: s31_6_g_3 (`-l` lists exactly the names the command can convert: the header names of plain and
    // gzip charmaps).
    let r = root();
    let niji = r.join("target/niji");
    assert!(niji.join("iconv").exists(), "the products are missing: run `cargo xtask so`");
    if skip_note::missing("gzip") {
        skip_note::skipped("skipped: no gzip to compress the private charmaps");
        return;
    }
    if !Path::new(DIR).is_dir() || !Command::new("unshare").args(["-r", "-m", "true"]).status().is_ok_and(|s| s.success()) {
        skip_note::skipped(&format!("skipped: needs `unshare -r -m` and a {DIR} directory to mount the private charmaps over"));
        return;
    }
    let work = r.join(format!("target/test-oracles/iconv-list-headers-{}", std::process::id()));
    let charmaps = work.join("charmaps");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&charmaps).unwrap();
    let maps = private_charmaps();
    for (stem, gz, header, e) in &maps {
        let text = format!("{header}CHARMAP\n<U0041>     /x41         LATIN CAPITAL LETTER A\n<U00E9>     /x{e:02x}         LATIN SMALL LETTER E WITH ACUTE\nEND CHARMAP\n");
        let plain = charmaps.join(stem);
        std::fs::write(&plain, text).unwrap();
        if *gz {
            assert!(Command::new("gzip").args(["-n", "-9"]).arg(&plain).status().unwrap().success(), "gzip {stem}");
        }
    }
    let out = iconv_private(&niji, &charmaps, &["-l"]);
    assert!(out.status.success(), "iconv -l failed: {:?} {}", out.status, String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8(out.stdout).unwrap();
    let listed: Vec<&str> = text.lines().map(|l| l.strip_suffix("//").or_else(|| l.strip_suffix('/')).expect("NAME// form")).collect();
    // Every name of each charmap is listed, spelled as iconv_open matches it, and converts both ways in a
    // fresh process: `Aé` to the charmap's bytes and back.
    for (stem, _, _, e) in &maps {
        for name in [stem.to_string(), format!("{stem}-SET"), format!("{stem}-ALIAS")] {
            assert!(listed.contains(&name.as_str()), "{name} is not listed by iconv -l:\n{text}");
            let bytes = convert(&niji, &charmaps, &work, "UTF-8", &name, "Aé".as_bytes());
            assert_eq!(bytes.as_deref(), Some(&[0x41, *e][..]), "UTF-8 to {name}");
            let back = convert(&niji, &charmaps, &work, &name, "UTF-8", &[0x41, *e]);
            assert_eq!(back.as_deref(), Some("Aé".as_bytes()), "{name} to UTF-8");
        }
    }
    // Every listed name converts: a one-byte round trip of `A`.
    for name in &listed {
        let bytes = convert(&niji, &charmaps, &work, "UTF-8", name, b"A").unwrap_or_else(|| panic!("{name} is listed but UTF-8 to it fails"));
        let back = convert(&niji, &charmaps, &work, name, "UTF-8", &bytes);
        assert_eq!(back.as_deref(), Some(&b"A"[..]), "{name} is listed but does not round-trip `A`");
    }
    let _ = std::fs::remove_dir_all(&work);
}
