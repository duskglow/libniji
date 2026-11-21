//! The tables of the built-in East Asian multi-byte codecs (BIG5, EUC-KR, CP932, CP936, GB18030),
//! part of `cargo xtask encodings-gen`.
//!
//! Each set is a public mapping table plus the reference library's variant of it:
//! * the public tables, vendored unchanged: the Unicode Consortium's mapping tables
//!   (`spec/encoding/unicode/VENDORS/MICSFT/WINDOWS/CP950.TXT` for BIG5, `CP932.TXT`, `CP936.TXT`,
//!   `spec/encoding/unicode/OBSOLETE/EASTASIA/KSC/KSC5601.TXT` for EUC-KR's KS X 1001) and, for
//!   GB18030, the WHATWG Encoding Standard's `index-gb18030.txt` (two bytes) and
//!   `index-gb18030-ranges.txt` (four bytes, a piecewise-linear map);
//! * the variant, `spec/encoding/reference/<set>.tsv`: what a black-box probe of the reference
//!   library found beyond the public table (format in that directory's README.md), written by
//!   `tools/probe/iconv_variant.py`. Every difference is data there, none is code.
//!
//! Generated file: `crates/core/src/iconv/cjk_tables.rs`, one `Set` per charset (laid out in
//! `crates/core/src/iconv/cjk.rs`): the bytes 0x80..=0xFF, the two-byte cells by lead (a row
//! index of offset and trail span into packed cells), the trail bytes after which `//IGNORE`
//! skips one byte (distinct bitmaps, one index per lead) and the encoder as pages of 64 code
//! points over the BMP (identical pages shared). The tables of all sets are concatenated per kind
//! (`ROWS`, `CELLS`, ...) and a `Set` holds (start, count) spans into them, never a pointer, so the
//! descriptors need no load-time relocation and every program that never converts pays nothing
//! for them at start-up (ICONV-CJK-STARTUP).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

pub const OUT: &str = "crates/core/src/iconv/cjk_tables.rs";

/// Marks in `high`: a lead byte, and a byte that is invalid alone.
const LEAD: u16 = 0xFFFF;
const BAD: u16 = 0xFFFE;
/// An encode page entry: no character, and the four-byte form is not tried either (GB18030).
const BLOCKED: u16 = 0xFFFF;

/// (identifier, charset, public table, its format, variant file).
const SETS: [(&str, &str, &str, Fmt, &str); 5] = [
    ("BIG5", "BIG5", "unicode/VENDORS/MICSFT/WINDOWS/CP950.TXT", Fmt::Unicode, "BIG5.tsv"),
    ("EUC_KR", "EUC-KR", "unicode/OBSOLETE/EASTASIA/KSC/KSC5601.TXT", Fmt::Unicode, "EUC-KR.tsv"),
    ("CP932", "CP932", "unicode/VENDORS/MICSFT/WINDOWS/CP932.TXT", Fmt::Unicode, "CP932.tsv"),
    ("CP936", "CP936", "unicode/VENDORS/MICSFT/WINDOWS/CP936.TXT", Fmt::Unicode, "CP936.tsv"),
    ("GB18030", "GB18030", "whatwg/index-gb18030.txt", Fmt::Gb18030, "GB18030.tsv"),
];

#[derive(Clone, Copy, PartialEq)]
enum Fmt {
    Unicode,
    Gb18030,
}

fn hex(s: &str, what: &str) -> Result<u32, String> {
    u32::from_str_radix(s, 16).map_err(|_| format!("{what}: `{s}` is not hexadecimal"))
}

/// `A1-F9,FE` -> the bytes.
fn byte_ranges(s: &str, what: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    if s == "-" {
        return Ok(out);
    }
    for part in s.split(',') {
        let (a, b) = part.split_once('-').unwrap_or((part, part));
        let (a, b) = (hex(a, what)?, hex(b, what)?);
        for v in a..=b {
            out.push(u8::try_from(v).map_err(|_| format!("{what}: {v:X} is not a byte"))?);
        }
    }
    Ok(out)
}

/// The variant file of one set.
#[derive(Default)]
struct Variant {
    leads: Vec<u8>,
    trails: Vec<u8>,
    /// (first byte, last byte, first code point).
    singles: Vec<(u8, u8, u32)>,
    /// (first code, last code, first code point or None): runs in cell order.
    cells: Vec<(u32, u32, Option<u32>)>,
    /// four-byte pointer -> code point.
    four: Vec<(u32, u32)>,
    /// code point -> encoding (empty: not encodable).
    encode: Vec<(u32, Vec<u8>)>,
    /// (leads, trail bytes skipped alone).
    skip1: Vec<(Vec<u8>, Vec<u8>)>,
    ignore_error: bool,
}

fn variant(path: &Path) -> Result<Variant, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut v = Variant::default();
    let mut ignore = None;
    for (k, line) in text.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let what = format!("{}:{}", path.display(), k + 1);
        let f: Vec<&str> = line.split('\t').collect();
        let field = |i: usize| f.get(i).copied().ok_or_else(|| format!("{what}: missing field {}", i + 1));
        match field(0)? {
            "leads" => v.leads = byte_ranges(field(1)?, &what)?,
            "trails" => v.trails = byte_ranges(field(1)?, &what)?,
            "single" => {
                let b = |s: &str| hex(s, &what).and_then(|x| u8::try_from(x).map_err(|_| format!("{what}: not a byte")));
                v.singles.push((b(field(1)?)?, b(field(2)?)?, hex(field(3)?, &what)?));
            }
            "cells" => {
                let cp = match field(3)? {
                    "none" => None,
                    s => Some(hex(s, &what)?),
                };
                v.cells.push((hex(field(1)?, &what)?, hex(field(2)?, &what)?, cp));
            }
            "four" => v.four.push((field(1)?.parse().map_err(|_| format!("{what}: bad pointer"))?, hex(field(2)?, &what)?)),
            "encode" => {
                let cp = hex(field(1)?, &what)?;
                let bytes = match field(2)? {
                    "none" => Vec::new(),
                    s => (0..s.len() / 2).map(|i| s.get(2 * i..2 * i + 2).map_or(Ok(0), |h| hex(h, &what).map(|x| x as u8))).collect::<Result<_, _>>()?,
                };
                v.encode.push((cp, bytes));
            }
            "skip1" => v.skip1.push((byte_ranges(field(1)?, &what)?, byte_ranges(field(2)?, &what)?)),
            "ignore-result" => ignore = Some(field(1)? == "error"),
            other => return Err(format!("{what}: unknown record `{other}`")),
        }
    }
    v.ignore_error = ignore.ok_or_else(|| format!("{}: no ignore-result", path.display()))?;
    if v.leads.is_empty() || v.trails.is_empty() {
        return Err(format!("{}: no leads or trails", path.display()));
    }
    Ok(v)
}

/// A public table: two-byte code -> code point, and single byte 0x80.. -> code point.
type Public = (BTreeMap<u32, u32>, BTreeMap<u8, u32>);

/// The public table's two-byte cells (code -> code point) and single bytes.
fn public(path: &Path, fmt: Fmt) -> Result<Public, String> {
    let text = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = String::from_utf8_lossy(&text);
    let (mut two, mut one) = (BTreeMap::new(), BTreeMap::new());
    for (k, line) in text.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let what = format!("{}:{}", path.display(), k + 1);
        let mut f = line.split_whitespace();
        let (a, b) = (f.next().unwrap_or(""), f.next().unwrap_or(""));
        let (code, cp) = match fmt {
            Fmt::Unicode => {
                let (Some(a), Some(b)) = (a.strip_prefix("0x"), b.strip_prefix("0x")) else { continue };
                (hex(a, &what)?, hex(b, &what)?)
            }
            Fmt::Gb18030 => {
                let p: u32 = a.parse().map_err(|_| format!("{what}: bad pointer"))?;
                let t = p % 190;
                ((p / 190 + 0x81) << 8 | (t + if t < 0x3F { 0x40 } else { 0x41 }), hex(b.trim_start_matches("0x"), &what)?)
            }
        };
        if code > 0xFF {
            two.insert(code, cp);
        } else {
            one.insert(code as u8, cp);
        }
    }
    Ok((two, one))
}

fn ranges(root: &Path) -> Result<Vec<(u32, u32)>, String> {
    let path = root.join("spec/encoding/whatwg/index-gb18030-ranges.txt");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let mut f = line.split_whitespace();
        let p = f.next().and_then(|s| s.parse().ok());
        let c = f.next().and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok());
        let (Some(p), Some(c)) = (p, c) else { return Err(format!("{}: bad line `{line}`", path.display())) };
        out.push((p, c));
    }
    out.sort_unstable();
    Ok(out)
}

/// The four-byte GB18030 code point of pointer `p` by the ranges (no variant applied).
pub fn four_public(r: &[(u32, u32)], p: u32) -> Option<u32> {
    if (189_000..=1_237_575).contains(&p) {
        return Some(0x1_0000 + p - 189_000);
    }
    if p > 39_419 {
        return None;
    }
    let i = r.partition_point(|&(q, _)| q <= p).checked_sub(1)?;
    let &(q, c) = r.get(i)?;
    Some(c + p - q)
}

fn list<T: std::fmt::Display>(w: &mut String, decl: &str, values: &[T], per_line: usize) {
    let _ = writeln!(w, "{decl} = [");
    for row in values.chunks(per_line.max(1)) {
        let _ = writeln!(w, "    {},", row.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "));
    }
    w.push_str("];\n\n");
}

fn hex16(v: &[u16]) -> Vec<String> {
    v.iter().map(|x| format!("0x{x:04X}")).collect()
}

/// One set: its tables go into `pool`, its descriptor into `w`; returns its size in bytes.
fn set(w: &mut String, pool: &mut Pool, root: &Path, rg: &[(u32, u32)], s: &(&str, &str, &str, Fmt, &str)) -> Result<usize, String> {
    let (ident, name, file, fmt, var) = *s;
    let v = variant(&root.join("spec/encoding/reference").join(var))?;
    let (pub2, pub1) = public(&root.join("spec/encoding").join(file), fmt)?;
    let (lead_lo, lead_hi) = (*v.leads.first().unwrap_or(&0), *v.leads.last().unwrap_or(&0));
    let is_trail = |t: u8| v.trails.contains(&t);
    // The decode cells: the public table within the leads and trails, then the variant's runs.
    let mut cells: BTreeMap<u32, u32> = pub2.iter().filter(|&(&c, _)| v.leads.contains(&((c >> 8) as u8)) && is_trail((c & 0xFF) as u8)).map(|(&c, &p)| (c, p)).collect();
    let order: Vec<u32> = v.leads.iter().flat_map(|&l| v.trails.iter().map(move |&t| u32::from(l) << 8 | u32::from(t))).collect();
    for &(a, b, cp) in &v.cells {
        let (Some(i), Some(j)) = (order.iter().position(|&c| c == a), order.iter().position(|&c| c == b)) else {
            return Err(format!("{var}: cells {a:04X}..{b:04X} outside the leads and trails"));
        };
        for (k, &c) in order.get(i..=j).unwrap_or(&[]).iter().enumerate() {
            match cp {
                None => cells.remove(&c),
                Some(x) => cells.insert(c, x + k as u32),
            };
        }
    }
    // Single bytes 0x80..=0xFF.
    let mut high = [BAD; 128];
    for &l in &v.leads {
        if let Some(h) = l.checked_sub(0x80).and_then(|i| high.get_mut(usize::from(i))) {
            *h = LEAD;
        }
    }
    for &(a, b, cp) in &v.singles {
        for x in a..=b {
            let c = u16::try_from(cp + u32::from(x - a)).map_err(|_| format!("{var}: single byte beyond the BMP"))?;
            if let Some(h) = x.checked_sub(0x80).and_then(|i| high.get_mut(usize::from(i))) {
                *h = c;
            }
        }
    }
    let _ = pub1;
    // Row index: per lead, the span of trails with a cell, packed.
    let (mut rows, mut packed) = (Vec::new(), Vec::<u16>::new());
    for lead in lead_lo..=lead_hi {
        let row: Vec<(u8, u32)> = cells.range(u32::from(lead) << 8..=u32::from(lead) << 8 | 0xFF).map(|(&c, &p)| ((c & 0xFF) as u8, p)).collect();
        let (Some(&(first, _)), Some(&(last, _))) = (row.first(), row.last()) else {
            rows.push("(0, 1, 0)".to_owned());
            continue;
        };
        rows.push(format!("({}, 0x{first:02X}, 0x{last:02X})", packed.len()));
        for t in first..=last {
            let cp = row.iter().find(|&&(x, _)| x == t).map_or(0, |&(_, p)| p);
            packed.push(u16::try_from(cp).map_err(|_| format!("{var}: cell beyond the BMP"))?);
        }
    }
    // Skip bitmaps.
    let (mut maps, mut skip_row) = (Vec::<[u32; 8]>::new(), vec![0u8; usize::from(lead_hi - lead_lo) + 1]);
    maps.push([0; 8]);
    for (leads, trails) in &v.skip1 {
        let mut m = [0u32; 8];
        for &t in trails {
            if let Some(word) = m.get_mut(usize::from(t / 32)) {
                *word |= 1 << (t % 32);
            }
        }
        let idx = maps.iter().position(|x| *x == m).unwrap_or_else(|| {
            maps.push(m);
            maps.len() - 1
        });
        for &l in leads {
            if let Some(r) = l.checked_sub(lead_lo).and_then(|i| skip_row.get_mut(usize::from(i))) {
                *r = u8::try_from(idx).map_err(|_| format!("{var}: too many skip maps"))?;
            }
        }
    }
    // Encoder: default = the shortest, then lowest code of each code point; the variant's choices win.
    let mut enc: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    for (i, &h) in high.iter().enumerate() {
        if h != LEAD && h != BAD {
            enc.entry(u32::from(h)).or_insert_with(|| vec![0x80 + i as u8]);
        }
    }
    for (&c, &cp) in &cells {
        enc.entry(cp).or_insert_with(|| vec![(c >> 8) as u8, (c & 0xFF) as u8]);
    }
    let mut four_enc = Vec::new();
    for (cp, bytes) in &v.encode {
        if bytes.len() == 4 {
            let [b1, b2, b3, b4] = [bytes[0], bytes[1], bytes[2], bytes[3]].map(u32::from);
            four_enc.push((*cp, ((b1 - 0x81) * 10 + b2 - 0x30) * 1260 + (b3 - 0x81) * 10 + b4 - 0x30));
            enc.remove(cp);
        } else {
            enc.insert(*cp, bytes.clone());
        }
    }
    // A four-byte departure's code point is written as that pointer unless a shorter form or an
    // `encode` record names another (probed: all 19 of GB18030's).
    for &(p, cp) in &v.four {
        if !enc.contains_key(&cp) && !four_enc.iter().any(|&(c, _)| c == cp) {
            four_enc.push((cp, p));
        }
    }
    let mut pages: Vec<[u16; 64]> = vec![[0; 64]];
    let mut index = vec![0u16; 1024];
    for (pg, slot) in index.iter_mut().enumerate() {
        let mut page = [0u16; 64];
        for (k, e) in page.iter_mut().enumerate() {
            let cp = (pg * 64 + k) as u32;
            if cp < 0x80 {
                continue;
            }
            *e = match enc.get(&cp).map(Vec::as_slice) {
                None => 0,
                Some([]) => BLOCKED,
                Some([b]) => u16::from(*b),
                Some([a, b]) => u16::from(*a) << 8 | u16::from(*b),
                Some(_) => return Err(format!("{var}: U+{cp:04X} encodes to more than two bytes")),
            };
        }
        *slot = match pages.iter().position(|p| *p == page) {
            Some(i) => i as u16,
            None => {
                pages.push(page);
                (pages.len() - 1) as u16
            }
        };
    }
    if let Some((cp, _)) = enc.iter().find(|&(&c, _)| c > 0xFFFF) {
        return Err(format!("{var}: U+{cp:X} beyond the BMP in the two-byte encoder"));
    }
    // GB18030's four-byte departures, checked against the ranges.
    for &(p, cp) in &v.four {
        if four_public(rg, p) == Some(cp) {
            return Err(format!("{var}: four {p} repeats the ranges"));
        }
    }
    let maps_s: Vec<String> = maps.iter().map(|m| format!("[{}]", m.iter().map(|x| format!("0x{x:08X}")).collect::<Vec<_>>().join(", "))).collect();
    let flat: Vec<u16> = pages.iter().flatten().copied().collect();
    let four_s: Vec<String> = v.four.iter().map(|(p, c)| format!("({p}, 0x{c:04X})")).collect();
    let mut four_enc_s: Vec<(u32, u32)> = four_enc;
    four_enc_s.sort_unstable();
    let four_enc_s: Vec<String> = four_enc_s.iter().map(|(c, p)| format!("(0x{c:04X}, {p})")).collect();
    let size = 256 + rows.len() * 4 + packed.len() * 2 + maps.len() * 32 + skip_row.len() + 2048 + flat.len() * 2 + (four_s.len() + four_enc_s.len()) * 8;
    // The descriptor holds (start, count) spans into the pooled tables, never a pointer: an
    // address in a static would be a load-time relocation in every process (ICONV-CJK-STARTUP).
    let sp = |at: usize, n: usize| format!("({at}, {n})");
    let spans = [
        ("rows", sp(pool.rows.len(), rows.len())),
        ("cells", sp(pool.cells.len(), packed.len())),
        ("skip1", sp(pool.skip1.len(), maps_s.len())),
        ("skip_row", sp(pool.skip_row.len(), skip_row.len())),
        ("pages", sp(pool.pages.len(), index.len())),
        ("codes", sp(pool.codes.len(), flat.len())),
        ("four", sp(pool.four.len(), four_s.len())),
        ("four_enc", sp(pool.four_enc.len(), four_enc_s.len())),
    ];
    pool.rows.extend(rows);
    pool.cells.extend(packed);
    pool.skip1.extend(maps_s);
    pool.skip_row.extend(skip_row);
    pool.pages.extend(index);
    pool.codes.extend(flat);
    pool.four.extend(four_s);
    pool.four_enc.extend(four_enc_s);
    let high_s = hex16(&high);
    let _ = writeln!(w, "    // {ident}: {name}, {} two-byte cells; public table spec/encoding/{file}, variant spec/encoding/reference/{var}.", cells.len());
    let _ = writeln!(w, "    Set {{\n        high: [\n            {}\n        ],", high_s.chunks(16).map(|c| c.join(", ")).collect::<Vec<_>>().join(",\n            "));
    let _ = writeln!(w, "        lead_lo: 0x{lead_lo:02X},");
    for (field, span) in &spans {
        let _ = writeln!(w, "        {field}: {span},");
    }
    let _ = writeln!(w, "        ignore_error: {},\n    }},", v.ignore_error);
    Ok(size)
}

/// Every set's tables, concatenated per kind (in `SETS` order); each descriptor names its parts.
#[derive(Default)]
struct Pool {
    rows: Vec<String>,
    cells: Vec<u16>,
    skip1: Vec<String>,
    skip_row: Vec<u8>,
    pages: Vec<u16>,
    codes: Vec<u16>,
    four: Vec<String>,
    four_enc: Vec<String>,
}

/// The generated file and each set's table size in bytes.
type Generated = (String, Vec<(&'static str, usize)>);

pub fn generate(root: &Path) -> Result<Generated, String> {
    let rg = ranges(root)?;
    let mut w = String::new();
    w.push_str("// @generated by `cargo xtask encodings-gen` (xtask/src/encodings_cjk.rs) from public mapping tables\n");
    w.push_str("// and the reference library's variants (spec/encoding/reference/*.tsv, from a black-box probe).\n");
    w.push_str("// Sources: the Unicode Consortium's mapping tables (https://www.unicode.org/Public/MAPPINGS/, Unicode\n");
    w.push_str("// terms of use) CP950.TXT, CP932.TXT, CP936.TXT, KSC5601.TXT; the WHATWG Encoding Standard\n");
    w.push_str("// (https://encoding.spec.whatwg.org/, WHATWG (Apple, Google, Mozilla, Microsoft), CC BY 4.0)\n");
    w.push_str("// index-gb18030.txt and index-gb18030-ranges.txt. Do not edit.\n");
    w.push_str("#![allow(clippy::unreadable_literal)]\n\nuse super::cjk::Set;\n\n");
    let rs: Vec<String> = rg.iter().map(|(p, c)| format!("({p}, 0x{c:04X})")).collect();
    list(&mut w, &format!("/// GB18030's four-byte ranges (pointer, first code point), ascending.\npub(super) static GB18030_RANGES: [(u32, u32); {}]", rs.len()), &rs, 6);
    let (mut sizes, mut pool, mut descs) = (Vec::new(), Pool::default(), String::new());
    for s in &SETS {
        let n = set(&mut descs, &mut pool, root, &rg, s)?;
        sizes.push((s.1, n));
    }
    let what = "every set's in turn; a descriptor's (start, count) names its part";
    list(&mut w, &format!("/// Per lead: offset in its set's cells, first and last trail ({what}).\npub(super) static ROWS: [(u16, u8, u8); {}]", pool.rows.len()), &pool.rows, 6);
    list(&mut w, &format!("/// The two-byte cells ({what}).\npub(super) static CELLS: [u16; {}]", pool.cells.len()), &hex16(&pool.cells), 12);
    list(&mut w, &format!("/// Skip bitmaps ({what}).\npub(super) static SKIP1: [[u32; 8]; {}]", pool.skip1.len()), &pool.skip1, 1);
    list(&mut w, &format!("/// Per lead: its bitmap in its set's `SKIP1` ({what}).\npub(super) static SKIP_ROW: [u8; {}]", pool.skip_row.len()), &pool.skip_row, 32);
    list(&mut w, &format!("/// Per 64 BMP code points: its page in its set's `CODES` ({what}).\npub(super) static PAGES: [u16; {}]", pool.pages.len()), &pool.pages, 32);
    list(&mut w, &format!("/// The encoder pages ({what}).\npub(super) static CODES: [u16; {}]", pool.codes.len()), &hex16(&pool.codes), 12);
    list(&mut w, &format!("/// Four-byte departures, (pointer, code point) ({what}).\npub(super) static FOUR: [(u32, u32); {}]", pool.four.len()), &pool.four, 4);
    list(&mut w, &format!("/// Four-byte encodings, (code point, pointer) ({what}).\npub(super) static FOUR_ENC: [(u32, u32); {}]", pool.four_enc.len()), &pool.four_enc, 4);
    let _ = writeln!(w, "/// The descriptors, in `Which::ALL` order; integers only, so no load-time relocation.\npub(super) static SETS: [Set; {}] = [\n{descs}];", SETS.len());
    while w.ends_with("\n\n") {
        w.pop();
    }
    Ok((w, sizes))
}
