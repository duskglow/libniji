//! s31_8_i_2: the charmaps localedef reads from `/usr/share/i18n/charmaps`, for the charsets libniji
//! converts with built-in codecs, agree with those codecs over the whole code space.
//!
//! Each generated text is read back with `niji::charmap::Charmap` (the parser localedef and iconv
//! use), then: every listed mapping decodes through the codec to its character and encodes back to
//! its sequence; and, for every code point 0..=0x10FFFF, the charmap encodes it exactly when the
//! codec writes it as a sequence that decodes back to it, and to that sequence. Together these say
//! the charmap is the codec's round-trip set, no more and no less, whatever way the generator found
//! it. The header and the `WIDTH` section are checked against their definitions in the crate doc.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, missing_docs)]

use niji::charmap::{Charmap, Step};
use niji::iconv::Codec;
use niji_charmaps::{CHARSETS, Seq, charmap, decode, encode};

fn header(text: &str, key: &str) -> String {
    let line = text.lines().find(|l| l.starts_with(key)).unwrap_or_else(|| panic!("no {key} line"));
    line[key.len()..].trim().to_string()
}

fn check(name: &str, codec: Codec) {
    let text = charmap(name, codec);
    let cm = Charmap::parse(text.as_bytes()).unwrap_or_else(|_| panic!("{name}: the charmap does not parse"));
    assert_eq!(cm.name(), name.as_bytes(), "{name}: <code_set_name>");
    assert_eq!(header(&text, "<mb_cur_min>"), "1", "{name}");
    // Every listed mapping is the codec's, both ways.
    let mut listed = 0usize;
    let mut longest = 0usize;
    for (cp, seq, n) in cm.mappings() {
        let seq = &seq[..n];
        assert_eq!(decode(codec, seq), Seq::Char(cp), "{name}: {seq:02x?} is listed as U+{cp:04X}");
        assert_eq!(encode(codec, cp).as_deref(), Some(seq), "{name}: U+{cp:04X} is listed as {seq:02x?}");
        assert_eq!(cm.decode(seq), Step::Char(cp, n), "{name}: the parsed charmap reads {seq:02x?}");
        listed += 1;
        longest = longest.max(n);
    }
    // Every round trip of the codec is listed.
    let mut round_trips = 0usize;
    let mut buf = [0u8; 8];
    for cp in 0..=0x10_FFFFu32 {
        let codec_seq = encode(codec, cp).filter(|s| decode(codec, s) == Seq::Char(cp));
        let mine = cm.encode(cp, &mut buf).map(|n| buf[..n].to_vec());
        assert_eq!(mine, codec_seq, "{name}: U+{cp:04X}");
        round_trips += usize::from(codec_seq.is_some());
    }
    assert_eq!(listed, round_trips, "{name}: a mapping listed twice");
    let want_max = if codec == Codec::Utf8 { 6 } else { longest };
    assert_eq!(header(&text, "<mb_cur_max>"), want_max.to_string(), "{name}");
    // WIDTH: exactly the listed characters of width 0 or 2.
    let mut widths = std::collections::BTreeMap::new();
    if let Some(sec) = text.split("\nWIDTH\n").nth(1) {
        for line in sec.lines().take_while(|l| *l != "END WIDTH") {
            let (range, w) = line.split_once(' ').unwrap();
            let (a, b) = range.split_once("..").unwrap_or((range, range));
            let cp = |s: &str| u32::from_str_radix(s.trim_start_matches("<U").trim_end_matches('>'), 16).unwrap();
            for c in cp(a)..=cp(b) {
                assert!(widths.insert(c, w.parse::<i32>().unwrap()).is_none(), "{name}: U+{c:04X} has two widths");
            }
        }
    }
    for (cp, seq, n) in cm.mappings() {
        let w = niji_charmaps::width(codec, &seq[..n], cp);
        assert_eq!(widths.remove(&cp), (w == 0 || w == 2).then_some(w), "{name}: WIDTH of U+{cp:04X}");
    }
    assert!(widths.is_empty(), "{name}: WIDTH lists characters the charmap does not: {:?}", widths.keys().take(5).collect::<Vec<_>>());
}

/// s31_8_i_2: one test per charmap, so a failure names it.
macro_rules! charset_tests {
    ($($test:ident: $k:expr;)*) => {$(
        #[test]
        fn $test() {
            let (name, codec) = CHARSETS[$k];
            check(name, codec);
        }
    )*};
}

charset_tests! {
    ansi_x3_4_1968_is_its_codec: 0;
    big5_is_its_codec: 1;
    cp949_is_its_codec: 2;
    euc_jp_is_its_codec: 3;
    euc_kr_is_its_codec: 4;
    gb18030_is_its_codec: 5;
    gbk_is_its_codec: 6;
    iso_8859_1_is_its_codec: 7;
    utf_8_is_its_codec: 8;
    windows_31j_is_its_codec: 9;
}

/// s31_7_g_3: each name is a distinct file name and one of its codec's iconv names (built in
/// before any charmap file, so installing the file changes no conversion).
#[test]
fn names_are_distinct_builtin_names() {
    let mut seen = std::collections::BTreeSet::new();
    for (name, codec) in CHARSETS {
        assert!(seen.insert(name), "{name} twice");
        let spec = niji::iconv::parse_name(name.as_bytes());
        assert_eq!(niji::iconv::builtin(spec.name), Some(codec), "{name} is not a built-in name of its codec");
    }
}
