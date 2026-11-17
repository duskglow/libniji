//! The generated charmaps (spec §6.4; issue #6: charsets with libniji's packages alone): every
//! charmap `cargo xtask so` writes from `spec/encoding/charmaps.tsv` into `target/niji/charmaps`
//! converts like the host C library's converter of the same name. libniji's programs run in a user
//! and mount namespace (`unshare -r -m`) with `target/niji/charmaps` mounted over
//! `/usr/share/i18n/charmaps`, so they see the generated set alone, as on a system with libniji's
//! packages. Two oracles, built against the host C library and statically against the products:
//! `tests/oracle/iconv_codespace.c` over each charmap's whole code space (every byte sequence,
//! every code point encoded plainly and with `//IGNORE`), and `tests/oracle/iconv_names.c` for every
//! name of the table in one process (each opens both ways and decodes and re-encodes every byte
//! as its charset does). `//TRANSLIT` block hashes are not compared here: transliteration comes
//! from the locale (D-6.9), not from the charmap. Needs the products, a C compiler, `unshare -r -m`
//! and a `/usr/share/i18n/charmaps` directory to mount over; without one it says so and skips.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

#[path = "support/oracle.rs"]
mod oracle;

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The table's rows: (name, aliases).
fn table() -> Vec<(String, Vec<String>)> {
    let text = std::fs::read_to_string(root().join("spec/encoding/charmaps.tsv")).unwrap();
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("name\t"))
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert_eq!(f.len(), 4, "charmaps.tsv: {l}");
            (f[0].to_string(), f[3].split_whitespace().map(str::to_string).collect())
        })
        .collect()
}

fn build(src: &str, out: &Path, niji: bool) -> bool {
    let r = root();
    let src = r.join("crates/core/tests/oracle").join(src);
    let mut cmd = Command::new("cc");
    if niji {
        cmd.args(["-std=gnu17", "-O1", "-static", "-nostdlib", "-fno-builtin"]).arg(r.join("target/niji/niji-crt1.o")).arg(&src).arg(r.join("target/niji/libc.a")).arg("-lgcc");
    } else {
        cmd.args(["-std=gnu17", "-O1"]).arg(&src);
    }
    oracle::build(cmd, out)
}

const DIR: &str = "/usr/share/i18n/charmaps";

/// Run `exe args` on the host (`charmaps` none) or with the directory `charmaps` mounted over [`DIR`].
fn run(exe: &Path, args: &[String], charmaps: Option<&Path>) -> String {
    let o = if let Some(charmaps) = charmaps {
        Command::new("unshare")
            .args(["-r", "-m", "sh", "-c", "mount --bind \"$1\" \"$2\" && shift 2 && exec \"$@\"", "sh"])
            .arg(charmaps)
            .arg(DIR)
            .arg(exe)
            .args(args)
            .env_clear()
            .output()
            .unwrap()
    } else {
        Command::new(exe).args(args).env_clear().output().unwrap()
    };
    assert!(o.status.success(), "{} {args:?} failed: {:?} {}", exe.display(), o.status, String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}

/// Lines that differ (at most 20 printed); `skip` drops lines neither side should be held to.
fn differences(what: &str, want: &str, got: &str, skip: impl Fn(&str) -> bool) -> usize {
    assert_eq!(want.lines().count(), got.lines().count(), "{what}: line counts differ");
    let mut n = 0usize;
    for (k, (w, g)) in want.lines().zip(got.lines()).enumerate() {
        if w != g && !skip(w) {
            n = n.saturating_add(1);
            if n <= 20 {
                eprintln!("{what} line {}:\n  reference {w}\n  niji      {g}", k.saturating_add(1));
            }
        }
    }
    n
}

#[test]
fn generated_charmaps_convert_like_the_reference() {
    // Clauses: s6_4_c_1, s6_4_c_1_1, s6_4_c_1_2, s6_4_c_2, s6_4_f_2_2, s6_4_f_3.
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    let rows = table();
    for (name, _) in &rows {
        assert!(r.join("target/niji/charmaps").join(name).is_file(), "target/niji/charmaps/{name} is missing: run `cargo xtask so`");
    }
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler");
        return;
    }
    if !Path::new(DIR).is_dir() || !Command::new("unshare").args(["-r", "-m", "true"]).status().is_ok_and(|s| s.success()) {
        skip_note::skipped(&format!("skipped: needs `unshare -r -m` and a {DIR} directory to mount the generated charmaps over"));
        return;
    }
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = |n: &str| dir.join(n);
    for (src, base) in [("iconv_names.c", "iconv_names"), ("iconv_codespace.c", "iconv_codespace")] {
        assert!(build(src, &exe(&format!("{base}_host")), false), "{src} does not build against the host C library");
        assert!(build(src, &exe(&format!("{base}_niji")), true), "{src} does not link against the products");
    }

    // Every name of the table in one process: each opens, as the conversion of its row.
    let names: Vec<String> = rows.iter().flat_map(|(n, a)| std::iter::once(n.clone()).chain(a.iter().cloned())).collect();
    let generated = r.join("target/niji/charmaps");
    let want = run(&exe("iconv_names_host"), &names, None);
    let got = run(&exe("iconv_names_niji"), &names, Some(&generated));
    let not_ref = want.lines().filter(|l| !l.contains(" open 1 1")).count();
    assert_eq!(not_ref, 0, "the host C library does not open every name of charmaps.tsv:\n{want}");
    assert_eq!(differences("names", &want, &got, |_| false), 0, "names that do not convert as the reference does");

    // Each charmap over its whole code space.
    let mut diffs = 0usize;
    for (name, _) in &rows {
        let args = [name.clone()];
        let w = run(&exe("iconv_codespace_host"), &args, None);
        let g = run(&exe("iconv_codespace_niji"), &args, Some(&generated));
        diffs = differences(name, &w, &g, |l| l.starts_with("BT ")).saturating_add(diffs);
    }
    assert_eq!(diffs, 0, "code-space lines that differ from the reference");
    eprintln!("{} charmaps ({} names) compared with the reference", rows.len(), names.len());
}

/// Charmaps the many-charmaps test loads in one process before limiting memory: more than the
/// fixed tables of earlier libniji held (32, then 256).
const MANY_FIRST: usize = 300;
/// Charmaps it may load after limiting memory (the first failure comes long before these run out).
const MANY_MORE: usize = 600;

#[test]
fn charmaps_load_past_any_fixed_count_and_fail_cleanly_without_memory() {
    // Clauses: s6_4_c_1, s6_4_f_1; POSIX iconv_open ERRORS [ENOMEM] (no spec clause names it).
    // libniji's charmap table grows by fallible allocation (SYS-ICONV-COVERAGE-2): a process
    // loads MANY_FIRST distinct charmaps, and under RLIMIT_AS the first load that cannot get
    // memory fails ENOMEM while every loaded charmap keeps converting. libniji only: the host C
    // library does not build converters from charmap files.
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler");
        return;
    }
    if !Path::new(DIR).is_dir() || !Command::new("unshare").args(["-r", "-m", "true"]).status().is_ok_and(|s| s.success()) {
        skip_note::skipped(&format!("skipped: needs `unshare -r -m` and a {DIR} directory to mount the test charmaps over"));
        return;
    }
    let dir = r.join("target/test-oracles");
    let many = dir.join("many-charmaps");
    if many.exists() {
        std::fs::remove_dir_all(&many).unwrap();
    }
    std::fs::create_dir_all(&many).unwrap();
    for k in 0..MANY_FIRST.saturating_add(MANY_MORE) {
        let cp = k.saturating_add(0x100);
        let text = format!("<code_set_name> MANY-{k}\n<comment_char> %\n<escape_char> /\n<mb_cur_min> 1\n<mb_cur_max> 1\nCHARMAP\n<U000A> /x0a\n<U{cp:04X}> /x41\nEND CHARMAP\n");
        std::fs::write(many.join(format!("MANY-{k}")), text).unwrap();
    }
    let exe = dir.join("iconv_many_charmaps_niji");
    assert!(build("iconv_many_charmaps.c", &exe, true), "iconv_many_charmaps.c does not link against the products");
    let out = run(&exe, &[MANY_FIRST.to_string(), MANY_MORE.to_string()], Some(&many));
    eprintln!("{}", out.trim());
    assert!(out.starts_with(&format!("ok first={MANY_FIRST} ")), "iconv_many_charmaps: {out}");
}
