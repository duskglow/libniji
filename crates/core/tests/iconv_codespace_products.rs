//! The built-in multi-byte codecs over their whole code space (spec §6.4; issue #6: they work
//! with libniji's packages alone): `tests/oracle/iconv_codespace.c` built against the host C
//! library and statically against libniji's products; the outputs must be identical line by line.
//! The program decodes every byte sequence (growing it while the input is incomplete), decodes
//! each invalid one again with `//IGNORE`, and encodes every code point plainly, with
//! `//TRANSLIT` and with `//IGNORE`. Needs the products (`cargo xtask so`) and a C compiler; a
//! host whose C library cannot open a charset is said so and that charset is skipped.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]
#[path = "support/skip_note.rs"]
mod skip_note;

#[path = "support/oracle.rs"]
mod oracle;

use std::path::{Path, PathBuf};
use std::process::Command;

/// The built-in multi-byte codecs (`crates/core/src/iconv/jis.rs`).
const CHARSETS: [&str; 1] = ["EUC-JP"];

/// Blocks of 256 code points whose `//TRANSLIT` hash (`BT`) differs by decision, per charset:
/// the built-in `C` locale's transliteration is derived from the UCD, not the reference library's
/// table (D-6.9; e.g. U+00AD, U+00B2, U+00B5, U+2014). A `BT` difference in any other block fails;
/// a listed block that stops differing is only noted (the transliteration may improve).
const TRANSLIT_BLOCKS: [(&str, &[u32]); 1] = [(
    "EUC-JP",
    &[
        0x0, 0x2, 0x3, 0x4, 0x5, 0x1D, 0x20, 0x21, 0x22, 0x24, 0x25, 0x2A, 0x2C, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0xA6, 0xA7, 0xFB, 0xFE, 0xFF,
        0x107, 0x1D4, 0x1D6, 0x1D7, 0x1E0, 0x1F1, 0x1F2, 0x1FB,
    ],
)];

/// Whether line `w` is a `//TRANSLIT` block hash of `cs` that may differ (D-6.9).
fn recorded(cs: &str, w: &str) -> bool {
    let Some(block) = w.strip_prefix("BT ").and_then(|r| r.split(' ').next()).and_then(|b| u32::from_str_radix(b, 16).ok()) else { return false };
    TRANSLIT_BLOCKS.iter().any(|(c, blocks)| *c == cs && blocks.contains(&block))
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build(out: &Path, niji: bool) -> bool {
    let r = root();
    let src = r.join("crates/core/tests/oracle/iconv_codespace.c");
    let mut cmd = Command::new("cc");
    if niji {
        cmd.args(["-std=gnu17", "-O1", "-static", "-nostdlib", "-fno-builtin"]).arg(r.join("target/niji/niji-crt1.o")).arg(&src).arg(r.join("target/niji/libc.a")).arg("-lgcc");
    } else {
        cmd.args(["-std=gnu17", "-O1"]).arg(&src);
    }
    oracle::build(cmd, out)
}

#[test]
fn builtin_codecs_equal_the_reference_over_their_code_space() {
    // Clauses: s6_4_c_1, s6_4_c_1_1, s6_4_c_1_2, s6_4_c_2, s6_4_f_2_2, s6_4_f_2_3, s6_4_f_3.
    let r = root();
    assert!(r.join("target/niji/libc.a").exists(), "the products are missing: run `cargo xtask so`");
    if skip_note::missing("cc") {
        skip_note::skipped("skipped: no C compiler");
        return;
    }
    let dir = r.join("target/test-oracles");
    std::fs::create_dir_all(&dir).unwrap();
    let (host, ours) = (dir.join("iconv_codespace_host"), dir.join("iconv_codespace_niji"));
    assert!(build(&host, false), "iconv_codespace.c does not build against the host C library");
    assert!(build(&ours, true), "iconv_codespace.c does not link against the products");
    let mut compared = 0;
    for cs in CHARSETS {
        let run = |exe: &Path| -> String {
            let o = Command::new(exe).arg(cs).env_clear().output().unwrap();
            assert!(o.status.success(), "{} {cs} failed: {:?}", exe.display(), o.status);
            String::from_utf8_lossy(&o.stdout).into_owned()
        };
        let want = run(&host);
        if want.starts_with(&format!("charset {cs}: cannot open")) {
            skip_note::skipped(&format!("skipped {cs}: the host C library cannot open it"));
            continue;
        }
        let got = run(&ours);
        let (mut diffs, mut same) = (0, 0);
        for (k, (w, g)) in want.lines().zip(got.lines()).enumerate() {
            if recorded(cs, w) {
                same += usize::from(w == g);
                continue;
            }
            if w != g {
                diffs += 1;
                if diffs <= 40 {
                    eprintln!("{cs} line {}:\n  reference {w}\n  niji      {g}", k + 1);
                }
            }
        }
        assert_eq!(want.lines().count(), got.lines().count(), "{cs}: line counts differ");
        assert_eq!(diffs, 0, "{cs}: lines that differ from the reference");
        if same > 0 {
            eprintln!("{cs}: {same} recorded //TRANSLIT blocks no longer differ (D-6.9); the list can shrink");
        }
        compared += 1;
    }
    eprintln!("{compared} of {} charsets compared over their whole code space", CHARSETS.len());
}
