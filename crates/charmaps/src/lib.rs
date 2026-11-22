//! Charmaps read off libniji's built-in iconv codecs (LOCALE-CHARMAPS-GEN; spec §31.8, POSIX.1-2017
//! XBD 6.4 "Character Set Description File").
//!
//! `localedef -f NAME` needs `/usr/share/i18n/charmaps/NAME[.gz]`. For the charsets libniji's iconv
//! converts with a built-in codec ([`CHARSETS`]: ASCII, ISO-8859-1, UTF-8, EUC-JP and the CJK
//! codecs), the charmap is not written from any mapping file: [`charmap`] asks the codec itself,
//! through the same [`niji::iconv::Conv`] that `iconv` runs, so a charmap and its converter cannot
//! disagree. The single-byte charmaps libc6 ships for iconv (`spec/encoding/charmaps.tsv`,
//! D-6.16) are the converters themselves and are not generated here.
//!
//! **Which mappings.** Every byte sequence the codec decodes to one character (found by extending
//! each sequence the codec calls incomplete, up to [`niji::charmap::MAX_BYTES`] bytes), listed when
//! the codec encodes that character back to the same sequence. A sequence that decodes to a
//! character the codec writes as another sequence (CP932's duplicate NEC and IBM rows) is not
//! listed, so each symbolic name has one encoding and each encoding one name; the header says how
//! many such sequences there are. Code points the codec only encodes (one-way mappings) are not
//! listed either.
//!
//! **Format.** `<code_set_name>` is the file name; `<comment_char> %`, `<escape_char> /`;
//! `<mb_cur_min> 1`; `<mb_cur_max>` the longest listed sequence, except UTF-8: 6, the longest
//! sequence its encoder writes (code points up to 0x7FFFFFFF). Symbolic names are `<Uxxxx>`
//! (`<Uxxxxxxxx>` above the BMP). Consecutive characters whose sequences differ only by a last
//! byte counting up are one `<Ua>..<Ub> /x../x..` line, the range form `niji::charmap` and
//! localedef read. A `WIDTH` section lists the characters whose `C.UTF-8` `wcwidth`
//! ([`niji::unicode::width`]) is 0 or 2; every other character takes the default width 1.

use std::fmt::Write as _;

use niji::error::Errno;
use niji::iconv::{CjkSet, Codec, Conv, Order, State};
use niji::locale::Locale;

/// The generated charmaps: file name (and `<code_set_name>`, the name `locale -m` lists) and the
/// built-in codec it is read from. Each name is one of the codec's own iconv names.
pub const CHARSETS: [(&str, Codec); 10] = [
    ("ANSI_X3.4-1968", Codec::Ascii),
    ("BIG5", Codec::Cjk(CjkSet::Big5)),
    ("CP949", Codec::Cjk(CjkSet::Cp949)),
    ("EUC-JP", Codec::EucJp),
    ("EUC-KR", Codec::Cjk(CjkSet::EucKr)),
    ("GB18030", Codec::Cjk(CjkSet::Gb18030)),
    ("GBK", Codec::Cjk(CjkSet::Cp936)),
    ("ISO-8859-1", Codec::Latin1),
    ("UTF-8", Codec::Utf8),
    ("WINDOWS-31J", Codec::Cjk(CjkSet::Cp932)),
];

/// What a byte sequence is to a codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seq {
    /// The whole sequence is this one character.
    Char(u32),
    /// The sequence is a proper prefix of a character.
    Incomplete,
    /// Anything else.
    Invalid,
}

fn conv(from: Codec, to: Codec) -> Conv {
    Conv { from, to, translit: false, ignore: false, state: State::default() }
}

/// What `seq` is to `codec`, by an iconv conversion to UCS-4.
pub fn decode(codec: Codec, seq: &[u8]) -> Seq {
    let mut out = [0u8; 16];
    let o = conv(codec, Codec::Ucs4(Order::Be)).convert(seq, &mut out, &Locale::C);
    match o.result {
        Ok(0) if o.read == seq.len() && o.written == 4 => match out.first_chunk::<4>() {
            Some(w) => Seq::Char(u32::from_be_bytes(*w)),
            None => Seq::Invalid,
        },
        Err(e) if e == Errno::EINVAL && o.read == 0 => Seq::Incomplete,
        _ => Seq::Invalid,
    }
}

/// The sequence `codec` writes for `cp`, by an iconv conversion from UCS-4; `None` when it has
/// none (or writes nothing).
pub fn encode(codec: Codec, cp: u32) -> Option<Vec<u8>> {
    let mut out = [0u8; 16];
    let o = conv(Codec::Ucs4(Order::Be), codec).convert(&cp.to_be_bytes(), &mut out, &Locale::C);
    match o.result {
        Ok(0) if o.read == 4 && o.written > 0 => out.get(..o.written).map(<[u8]>::to_vec),
        _ => None,
    }
}

/// The charmap's mappings, in byte order, and how many decoded sequences were left out because
/// the codec writes their character as another sequence.
pub fn mappings(codec: Codec) -> (Vec<(Vec<u8>, u32)>, usize) {
    let mut listed = Vec::new();
    let mut other = 0usize;
    let mut stack: Vec<Vec<u8>> = (0..=255u8).rev().map(|b| vec![b]).collect();
    while let Some(seq) = stack.pop() {
        match decode(codec, &seq) {
            Seq::Char(cp) if encode(codec, cp).as_deref() == Some(seq.as_slice()) => listed.push((seq, cp)),
            Seq::Char(_) => other = other.saturating_add(1),
            Seq::Incomplete if seq.len() < niji::charmap::MAX_BYTES => {
                for b in (0..=255u8).rev() {
                    let mut next = seq.clone();
                    next.push(b);
                    stack.push(next);
                }
            }
            Seq::Incomplete | Seq::Invalid => {}
        }
    }
    (listed, other)
}

fn name(cp: u32) -> String {
    if cp <= 0xFFFF { format!("<U{cp:04X}>") } else { format!("<U{cp:08X}>") }
}

fn bytes(seq: &[u8]) -> String {
    seq.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "/x{b:02x}");
        s
    })
}

/// Whether `(seq, cp)` continues the run that ends with `(last, prev)`: the next character, and
/// the same sequence with its last byte one higher.
fn continues(last: &[u8], prev: u32, seq: &[u8], cp: u32) -> bool {
    let (Some((&lb, lhead)), Some((&b, head))) = (last.split_last(), seq.split_last()) else { return false };
    prev.checked_add(1) == Some(cp) && lhead == head && lb.checked_add(1) == Some(b)
}

/// The `WIDTH` of `cp`, which `codec` writes as `seq`: in UTF-8 and for a one-byte sequence its
/// `C.UTF-8` `wcwidth`; in a multi-byte charset a character of two or more bytes takes two
/// columns (unless it is zero-width), except EUC-JP's single-shift-2 half-width katakana (0x8E
/// and one byte) and GB18030's four-byte sequences (the rest of Unicode), which keep their
/// `C.UTF-8` width. Settled black-box: locales compiled with these charmaps and with the
/// reference's, by both localedefs, compared by `wcwidth` (D-31.22).
pub fn width(codec: Codec, seq: &[u8], cp: u32) -> i32 {
    let w = niji::unicode::width(cp);
    let narrow = codec == Codec::Utf8
        || seq.len() < 2
        || (codec == Codec::EucJp && seq.first() == Some(&0x8E))
        || (codec == Codec::Cjk(CjkSet::Gb18030) && seq.len() == 4);
    if narrow || w == 0 { w } else { 2 }
}

/// The charmap text for the charset `name` decoded and encoded by `codec`.
pub fn charmap(code_set: &str, codec: Codec) -> String {
    let (map, other) = mappings(codec);
    let longest = map.iter().map(|(s, _)| s.len()).max().unwrap_or(1);
    let mb_cur_max = if codec == Codec::Utf8 { 6 } else { longest };
    let mut t = String::new();
    let _ = writeln!(t, "<code_set_name> {code_set}\n<comment_char> %\n<escape_char> /\n<mb_cur_min> 1\n<mb_cur_max> {mb_cur_max}");
    let _ = writeln!(t, "% Generated by `cargo xtask so` (crates/charmaps) from libniji's built-in {code_set} iconv codec:");
    let _ = writeln!(t, "% every byte sequence it decodes to one character and encodes back the same way.");
    if other > 0 {
        let _ = writeln!(t, "% Not listed: {other} sequences that decode to a character the codec writes as another sequence.");
    }
    t.push_str("CHARMAP\n");
    let mut k = 0;
    while let Some((first, cp0)) = map.get(k) {
        let mut end = k;
        while let (Some((a, pa)), Some((b, pb))) = (map.get(end), map.get(end.saturating_add(1))) {
            if !continues(a, *pa, b, *pb) {
                break;
            }
            end = end.saturating_add(1);
        }
        let last = map.get(end).map_or(*cp0, |m| m.1);
        if end == k {
            let _ = writeln!(t, "{} {}", name(*cp0), bytes(first));
        } else {
            let _ = writeln!(t, "{}..{} {}", name(*cp0), name(last), bytes(first));
        }
        k = end.saturating_add(1);
    }
    t.push_str("END CHARMAP\n");
    let mut cps: Vec<(u32, i32)> = map.iter().map(|(s, cp)| (*cp, width(codec, s, *cp))).collect();
    cps.sort_unstable();
    let mut widths: Vec<(u32, u32, i32)> = Vec::new();
    for (cp, w) in cps {
        if w != 0 && w != 2 {
            continue;
        }
        match widths.last_mut() {
            Some((_, end, lw)) if *lw == w && end.checked_add(1) == Some(cp) => *end = cp,
            _ => widths.push((cp, cp, w)),
        }
    }
    if !widths.is_empty() {
        t.push_str("WIDTH\n");
        for (a, b, w) in widths {
            if a == b {
                let _ = writeln!(t, "{} {w}", name(a));
            } else {
                let _ = writeln!(t, "{}..{} {w}", name(a), name(b));
            }
        }
        t.push_str("END WIDTH\n");
    }
    t
}
