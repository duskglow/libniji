//! The shared root of libniji's generated `LC_COLLATE` (LOCALE-COLLATE-TAILORINGS-2; coordinator
//! decisions #7890 and #7958, owner ruling #7791: locale data by black-box probe).
//!
//! [`source`] writes a locale source (spec §31.8, POSIX.1-2017 XBD 7.3.2) whose `LC_COLLATE`
//! orders every character the root defines, built from two inputs:
//!
//! * the DUCET, `spec/unicode/15.1.0/allkeys.txt` (UTS #10, Unicode 15.1), with `PropList.txt`
//!   (`Unified_Ideograph`, `Variation_Selector`) and `UnicodeData.txt` (general category);
//! * `spec/locale/collate-root.txt`: the code points the root leaves undefined, two class rules
//!   and the single placements, each with the probe of the reference that shows it (that file's
//!   header has the formats).
//!
//! **Weights.** Levels 1-3 are the DUCET's with variable weighting "shifted" (UTS #10 section 4:
//! a variable collation element, and the ignorable ones after it, weigh nothing at levels 1-3).
//! Han ideographs the DUCET does not list take the implicit weights of UTS #10 section 10.1.
//! Level 4 is the character itself: characters equal at levels 1-3 keep their place in the
//! order, which is code point order apart from the rules. Each distinct weight of a level is one
//! collating symbol, listed in weight order; a character's weight at a level is the string of
//! its elements' symbols (`IGNORE` when it has none).
//!
//! **Rules.** `class han-variant-block`: a code point whose first primary is an implicit Han
//! primary and that is not in U+4E00-U+9FFF takes a block primary before every implicit Han
//! primary first. `class format-first`: controls, format characters and variation selectors with
//! all-zero DUCET weights come first among the characters equal at levels 1-3. `after cp anchor
//! level` places `cp` directly after `anchor`: at level 4 with the anchor's weights; at levels
//! 1-3 with a weight of its own just after the anchor's last one at that level, and the anchor's
//! weights at the other levels; `level+` appends a weight below every other to the anchor's at
//! that level instead (it sorts before what extends the anchor). `same cp anchor` gives `cp` the
//! anchor's weights at levels 1-3. The undefined characters sort first (`UNDEFINED` before every
//! character, ignorable at levels 1-3).

#![forbid(unsafe_code)]
#![allow(clippy::arithmetic_side_effects)] // sub-weight counters bounded by the rule count (far below u32::MAX)

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

/// One weight: the DUCET value and a sub-step (0 for the DUCET's own; rules mint 1, 2, ...).
type W = (u32, u32);

/// A character's weights at levels 1-3.
type Key = [Vec<W>; 3];

/// How a rule line places a code point.
#[derive(Clone, Copy)]
enum Place {
    /// `after cp anchor level` (`level+`: appended below every other weight).
    After(usize, bool),
    /// `same cp anchor`.
    Same,
}

/// The sub-steps of weights appended by `level+` rules (below every DUCET weight, apart from the
/// ones minted after an anchor without weights at that level).
const APPENDED: u32 = 1 << 30;

/// One DUCET collation element: variable, primary, secondary, tertiary.
type Ce = (bool, u32, u32, u32);

/// The block primary of class `han-variant-block`: after every explicit DUCET primary (the
/// DUCET's diagnostic primary range ends at 0x5F0E), before the first implicit Han base 0xFB40.
const VARIANT_BLOCK: W = (0xFB3F, 1);

fn read(repo: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(repo.join(rel)).map_err(|e| format!("{rel}: {e}"))
}

fn hex(s: &str, what: &str) -> Result<u32, String> {
    u32::from_str_radix(s.trim(), 16).map_err(|_| format!("{what}: not hex: {s:?}"))
}

/// The single-code-point entries of allkeys.txt.
fn ducet(text: &str) -> Result<HashMap<u32, Vec<Ce>>, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('@') {
            continue;
        }
        let (cps, ces) = line.split_once(';').ok_or_else(|| format!("allkeys: no ';': {line}"))?;
        let mut cps = cps.split_whitespace();
        let (Some(cp), None) = (cps.next(), cps.next()) else { continue };
        let mut v = Vec::new();
        for el in ces.split('[').skip(1) {
            let el = el.trim_end().trim_end_matches(']');
            let var = el.starts_with('*');
            let mut f = el.get(1..).unwrap_or("").split('.');
            let mut next = || hex(f.next().unwrap_or(""), "allkeys weight");
            v.push((var, next()?, next()?, next()?));
        }
        out.insert(hex(cp, "allkeys code point")?, v);
    }
    Ok(out)
}

/// The code points of a PropList.txt property.
fn property(text: &str, name: &str) -> Result<HashSet<u32>, String> {
    let mut out = HashSet::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("");
        let Some((range, prop)) = line.split_once(';') else { continue };
        if prop.trim() != name {
            continue;
        }
        let (lo, hi) = range.trim().split_once("..").unwrap_or((range.trim(), range.trim()));
        out.extend(hex(lo, name)?..=hex(hi, name)?);
    }
    Ok(out)
}

/// Shifted levels 1-3 of a collation element array (UTS #10 section 4).
fn shifted(ces: &[Ce]) -> Key {
    let mut key: Key = Default::default();
    let mut after_variable = false;
    for &(var, p, s, t) in ces {
        if var {
            after_variable = true;
            continue;
        }
        if (p == 0 && s == 0 && t == 0) || (p == 0 && after_variable) {
            continue;
        }
        after_variable = false;
        for (level, w) in key.iter_mut().zip([p, s, t]) {
            if w != 0 {
                level.push((w, 0));
            }
        }
    }
    key
}

/// The implicit weights of a unified ideograph (UTS #10 section 10.1.3).
fn implicit(cp: u32) -> Vec<Ce> {
    let core = (0x4E00..=0x9FFF).contains(&cp) || (0xF900..=0xFAFF).contains(&cp);
    let base = if core { 0xFB40 } else { 0xFB80 };
    vec![(false, base + (cp >> 15), 0x20, 0x2), (false, (cp & 0x7FFF) | 0x8000, 0, 0)]
}

fn name(cp: u32) -> String {
    if cp > 0xFFFF { format!("<U{cp:08X}>") } else { format!("<U{cp:04X}>") }
}

fn symbol(level: usize, w: W) -> String {
    let p = ["p", "s", "t"].get(level).copied().unwrap_or("x");
    if w.1 == 0 { format!("<{p}{:04X}>", w.0) } else { format!("<{p}{:04X}-{}>", w.0, w.1) }
}

/// The root locale source, from the spec files under the repository root `repo`.
pub fn source(repo: &Path) -> Result<String, String> {
    let keys = ducet(&read(repo, "spec/unicode/15.1.0/allkeys.txt")?)?;
    let props = read(repo, "spec/unicode/15.1.0/PropList.txt")?;
    let unified = property(&props, "Unified_Ideograph")?;
    let selectors = property(&props, "Variation_Selector")?;
    let mut format_gc = HashSet::new();
    for line in read(repo, "spec/unicode/15.1.0/UnicodeData.txt")?.lines() {
        let mut f = line.split(';');
        if let (Some(cp), Some(_), Some(gc)) = (f.next(), f.next(), f.next())
            && (gc == "Cc" || gc == "Cf")
        {
            format_gc.insert(hex(cp, "UnicodeData")?);
        }
    }
    let rules = read(repo, "spec/locale/collate-root.txt")?;

    let mut defined: BTreeMap<u32, Key> = BTreeMap::new();
    for (&cp, ces) in &keys {
        defined.insert(cp, shifted(ces));
    }
    for &cp in &unified {
        defined.entry(cp).or_insert_with(|| shifted(&implicit(cp)));
    }
    defined.retain(|&cp, _| cp != 0 && !(0xD800..=0xDFFF).contains(&cp));

    let (mut variant_block, mut format_first) = (false, false);
    let mut placements = Vec::new();
    for (no, line) in rules.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        let f: Vec<&str> = line.split_whitespace().collect();
        let bad = || format!("spec/locale/collate-root.txt:{}: {line:?}", no + 1);
        match f.as_slice() {
            [] => {}
            ["class", "han-variant-block"] => variant_block = true,
            ["class", "format-first"] => format_first = true,
            ["undefined", lo, hi] => {
                let (lo, hi) = (hex(lo, "undefined")?, hex(hi, "undefined")?);
                defined.retain(|&cp, _| !(lo..=hi).contains(&cp));
            }
            ["after", cp, anchor, level] => {
                let (level, append) = level.strip_suffix('+').map_or((*level, false), |l| (l, true));
                let level: usize = level.parse().map_err(|_| bad())?;
                if !(1..=4).contains(&level) || (append && level == 4) {
                    return Err(bad());
                }
                placements.push((hex(cp, "after")?, hex(anchor, "after")?, Place::After(level, append), bad()));
            }
            ["same", cp, anchor] => placements.push((hex(cp, "same")?, hex(anchor, "same")?, Place::Same, bad())),
            _ => return Err(bad()),
        }
    }

    if variant_block {
        for (&cp, key) in defined.iter_mut() {
            let implicit_han = key[0].first().is_some_and(|w| (0xFB40..0xFBC0).contains(&w.0) && w.1 == 0);
            if implicit_han && !(0x4E00..=0x9FFF).contains(&cp) {
                key[0].insert(0, VARIANT_BLOCK);
            }
        }
    }
    let first: HashSet<u32> = if format_first {
        keys.iter()
            .filter(|(cp, ces)| ces.iter().all(|&(_, p, s, t)| p == 0 && s == 0 && t == 0) && (format_gc.contains(cp) || selectors.contains(cp)))
            .map(|(&cp, _)| cp)
            .collect()
    } else {
        HashSet::new()
    };

    // Placements at levels 1-3 change weights; level 4 ones move a character in the order.
    let mut minted: BTreeSet<(usize, W)> = BTreeSet::new();
    let mut moves = Vec::new();
    let mut appended = 0;
    for (cp, anchor, place, at) in placements {
        let mut key = defined.get(&anchor).cloned().ok_or_else(|| format!("{at}: the anchor is not defined"))?;
        if !defined.contains_key(&cp) {
            return Err(format!("{at}: the code point is not defined"));
        }
        match place {
            Place::Same => {}
            Place::After(4, _) => moves.push((cp, anchor)),
            Place::After(level, append) => {
                let l = key.get_mut(level - 1).ok_or_else(|| format!("{at}: no such level"))?;
                // A weight of its own: after the anchor's last one at this level, or (append)
                // below every DUCET weight, after the anchor's whole string.
                let new = if append {
                    appended += 1;
                    (0, APPENDED + appended)
                } else {
                    let last = l.pop().unwrap_or((0, 0));
                    (last.0, last.1 + 1)
                };
                if !minted.insert((level, new)) {
                    return Err(format!("{at}: two placements after one weight"));
                }
                l.push(new);
            }
        }
        defined.insert(cp, key);
    }

    let mut order: Vec<u32> = defined.keys().copied().collect();
    order.sort_by_key(|cp| (&defined[cp], !first.contains(cp), *cp));
    for (cp, anchor) in moves {
        order.retain(|&c| c != cp);
        let at = order.iter().position(|&c| c == anchor).ok_or("a moved anchor")?;
        order.insert(at + 1, cp);
    }

    let mut symbols: [BTreeSet<W>; 3] = Default::default();
    for key in defined.values() {
        for (set, ws) in symbols.iter_mut().zip(key) {
            set.extend(ws.iter().copied());
        }
    }
    let mut out = String::new();
    out.push_str("comment_char %\nescape_char /\n");
    out.push_str("% The root collation of libniji's generated locales. Generated by niji-collate-root\n");
    out.push_str("% (crates/collate-root) from spec/unicode/15.1.0 and spec/locale/collate-root.txt.\n\n");
    out.push_str("LC_COLLATE\n");
    for (level, set) in symbols.iter().enumerate() {
        for &w in set {
            let _ = writeln!(out, "collating-symbol {}", symbol(level, w));
        }
    }
    out.push_str("order_start forward;forward;forward;forward\n");
    for (level, set) in symbols.iter().enumerate() {
        for &w in set {
            let _ = writeln!(out, "{}", symbol(level, w));
        }
    }
    out.push_str("UNDEFINED IGNORE;IGNORE;IGNORE\n");
    for cp in order {
        let n = name(cp);
        let _ = write!(out, "{n} ");
        for (level, ws) in defined[&cp].iter().enumerate() {
            match ws.as_slice() {
                [] => out.push_str("IGNORE"),
                [w] => out.push_str(&symbol(level, *w)),
                _ => {
                    out.push('"');
                    for &w in ws {
                        out.push_str(&symbol(level, w));
                    }
                    out.push('"');
                }
            }
            out.push(';');
        }
        let _ = writeln!(out, "{n}");
    }
    out.push_str("order_end\nEND LC_COLLATE\n");
    Ok(out)
}
