//! `LC_COLLATE` (spec Appendix D.1.4) from a locale source's collation statements.
//!
//! The source orders *items*: collating symbols, characters, collating elements (contractions)
//! and the undefined characters. Each character or element line gives its weights per level as
//! references to items; a weight's value is its item's place in the final order. The writer
//! keeps the order as a linked list (`reorder-after` inserts after an existing item, and a line
//! for an item already placed moves it), numbers it, then writes the multibyte and wide lookup
//! tables (D.1.4 "Element lookup") with each element's weights.
//!
//! Settled by comparing keys with the reference's compiled locales (black box, UTIL-12):
//! * every character the source does not order shares one element, placed after the symbols
//!   listed before the first `order_start` and before every character, wherever an `UNDEFINED`
//!   line stands; its weights are that line's, or without one empty at every level but the
//!   last, where it has its own place;
//! * a line under `reorder-after` that moves a character keeps the rules of its section;
//! * in the multibyte tables each byte of a sequence that is not an ordered character is an
//!   element of its own with the undefined weights;
//! * `..` on a line of its own orders the characters between the code points of the lines
//!   before and after it; `..` as a weight is the character itself.
//!
//! The weight numbers are libniji's own (D-31.20): the order of keys equals the reference's,
//! their bytes need not.
#![allow(clippy::arithmetic_side_effects)] // positions and offsets count items of one source (far below u32::MAX; table sizes checked when written)

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::build::{File, MAGIC};
use crate::charset::Charset;
use crate::lex::{self, Tok};
use crate::source::Stmt;
use crate::{Error, Result, TryVec};

/// Level rule bits (D.1.4 `_NL_COLLATE_RULESETS`).
const FORWARD: u8 = 1;
const BACKWARD: u8 = 2;
const POSITION: u8 = 4;

/// The most levels a ruleset may have.
const MAX_LEVELS: usize = 16;

/// An item of the order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// The list's head (no item).
    Head,
    /// A collating symbol, by id.
    Sym(u32),
    /// A character.
    Char(u32),
    /// A collating element, by id.
    Elem(u32),
    /// The undefined characters.
    Undef,
}

/// One weight of a level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wt {
    /// The place of an item.
    Of(Key),
    /// The item itself (`..`, or a level the line leaves out).
    Me,
}

#[derive(Debug)]
struct Node {
    key: Key,
    /// Weights per level (`None`: a symbol or an item without weights).
    weights: Option<TryVec<TryVec<Wt>>>,
    ruleset: u8,
    prev: usize,
    next: usize,
    live: bool,
}

/// The order being built.
struct Order {
    nodes: TryVec<Node>,
    at: BTreeMap<Key, usize>,
    tail: usize,
}

impl Order {
    fn new() -> Result<Order> {
        let mut nodes = TryVec::new();
        nodes.push(Node { key: Key::Head, weights: None, ruleset: 0, prev: 0, next: usize::MAX, live: true })?;
        Ok(Order { nodes, at: BTreeMap::new(), tail: 0 })
    }

    fn unlink(&mut self, i: usize) {
        let (p, n) = (self.nodes[i].prev, self.nodes[i].next);
        self.nodes[p].next = n;
        if n == usize::MAX {
            self.tail = p;
        } else {
            self.nodes[n].prev = p;
        }
        self.nodes[i].live = false;
    }

    /// Place `key` after node `after` (moving it if it is placed already); the new node.
    fn place(&mut self, key: Key, weights: Option<TryVec<TryVec<Wt>>>, ruleset: u8, after: usize) -> Result<usize> {
        let mut after = after;
        if let Some(&old) = self.at.get(&key) {
            if old == after {
                after = self.nodes[old].prev;
            }
            self.unlink(old);
        }
        let i = self.nodes.len();
        let next = self.nodes[after].next;
        self.nodes.push(Node { key, weights, ruleset, prev: after, next, live: true })?;
        self.nodes[after].next = i;
        if next == usize::MAX {
            self.tail = i;
        } else {
            self.nodes[next].prev = i;
        }
        self.at.insert(key, i);
        Ok(i)
    }
}

/// The collation statements, parsed.
struct Collation {
    order: Order,
    levels: usize,
    rulesets: TryVec<TryVec<u8>>,
    elems: TryVec<TryVec<u32>>,
    /// `UNDEFINED` was written (its node is `Key::Undef`).
    explicit_undef: bool,
    /// The first node placed inside an `order_start` section.
    first_in_section: Option<usize>,
}

/// Split `toks` at `;`.
fn fields(toks: &[Tok]) -> impl Iterator<Item = &[Tok]> {
    toks.split(|t| matches!(t, Tok::Semi))
}

/// The ids of the names of a string's `<name>` list.
fn names(raw: &[u8], esc: u8) -> Result<TryVec<TryVec<u8>>> {
    let mut out = TryVec::new();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] != b'<' {
            return Err(Error::syntax(0, "a weight string holds text that is not a <name>"));
        }
        let mut name = TryVec::new();
        i += 1;
        while i < raw.len() && raw[i] != b'>' {
            if raw[i] == esc && i + 1 < raw.len() {
                i += 1;
            }
            name.push(raw[i])?;
            i += 1;
        }
        i += 1;
        out.push(name)?;
    }
    Ok(out)
}

/// The names of a symbol range `a..b` (a shared prefix, then hexadecimal numbers of one width).
fn name_range(a: &[u8], b: &[u8], f: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()> {
    let bad = Error::syntax(0, "a symbol range whose ends differ in more than a hexadecimal number");
    if a.len() != b.len() {
        return Err(bad);
    }
    let hex_from = |s: &[u8]| s.iter().rposition(|c| !c.is_ascii_hexdigit()).map_or(0, |p| p + 1);
    let mut start = hex_from(a).max(hex_from(b));
    while start < a.len() && a[..start] != b[..start] {
        start += 1;
    }
    if start >= a.len() || a[..start] != b[..start] {
        return Err(bad);
    }
    let num = |s: &[u8]| core::str::from_utf8(s).ok().and_then(|t| u32::from_str_radix(t, 16).ok());
    let (Some(lo), Some(hi)) = (num(&a[start..]), num(&b[start..])) else { return Err(bad) };
    let width = a.len() - start;
    for v in lo..=hi {
        let mut name = TryVec::from_slice(&a[..start])?;
        for k in (0..width).rev() {
            let d = (v >> (4 * k)) & 15;
            name.push(b"0123456789ABCDEF"[usize::try_from(d).unwrap_or(0)])?;
        }
        f(&name)?;
    }
    Ok(())
}

struct Names {
    syms: BTreeMap<Vec<u8>, u32>,
    elems: BTreeMap<Vec<u8>, u32>,
}

impl Names {
    fn key(&self, name: &[u8], no: u32) -> Result<Key> {
        if let Some(&e) = self.elems.get(name) {
            return Ok(Key::Elem(e));
        }
        if let Some(&s) = self.syms.get(name) {
            return Ok(Key::Sym(s));
        }
        lex::ucs_name(name).map(Key::Char).ok_or(Error::syntax(no, "an unknown collating symbol"))
    }
}

/// The rule byte of one level field (`forward`, `backward`, `position`, joined by `,`).
fn rule(field: &[Tok], no: u32) -> Result<u8> {
    let mut r = 0;
    for t in field {
        match t {
            Tok::Word(w) if w.as_slice() == b"forward" => r |= FORWARD,
            Tok::Word(w) if w.as_slice() == b"backward" => r |= BACKWARD,
            Tok::Word(w) if w.as_slice() == b"position" => r |= POSITION,
            Tok::Comma => {}
            _ => return Err(Error::syntax(no, "an unknown level rule")),
        }
    }
    if r & (FORWARD | BACKWARD) == 0 {
        r |= FORWARD;
    }
    Ok(r)
}

/// The weights of a line after its first token (`None` with no weights: the item itself at every level).
fn weights(toks: &[Tok], levels: usize, n: &Names, esc: u8, no: u32) -> Result<TryVec<TryVec<Wt>>> {
    let mut out: TryVec<TryVec<Wt>> = TryVec::new();
    if !toks.is_empty() {
        for f in fields(toks) {
            let mut w = TryVec::new();
            match f {
                [Tok::Word(x)] if x.as_slice() == b"IGNORE" => {}
                [Tok::Word(x)] if x.as_slice() == b".." || x.as_slice() == b"..." => w.push(Wt::Me)?,
                [Tok::Sym(s)] => w.push(Wt::Of(n.key(s, no)?))?,
                [Tok::Str(raw)] => {
                    for name in names(raw, esc).map_err(|e| Error { line: no, ..e })?.iter() {
                        w.push(Wt::Of(n.key(name, no)?))?;
                    }
                }
                [] => w.push(Wt::Me)?,
                _ => return Err(Error::syntax(no, "a weight is not a symbol, a string, IGNORE or ..")),
            }
            out.push(w)?;
        }
    }
    if out.len() > levels {
        return Err(Error::syntax(no, "more weights than levels"));
    }
    while out.len() < levels {
        let mut w = TryVec::new();
        w.push(Wt::Me)?;
        out.push(w)?;
    }
    Ok(out)
}

fn clone_weights(ws: &[TryVec<Wt>]) -> Result<TryVec<TryVec<Wt>>> {
    let mut out = TryVec::with_capacity(ws.len())?;
    for w in ws {
        out.push(w.try_clone()?)?;
    }
    Ok(out)
}

fn parse(stmts: &[Stmt], cs: &Charset) -> Result<Collation> {
    let mut c = Collation {
        order: Order::new()?,
        levels: 0,
        rulesets: TryVec::new(),
        elems: TryVec::new(),
        explicit_undef: false,
        first_in_section: None,
    };
    let mut n = Names { syms: BTreeMap::new(), elems: BTreeMap::new() };
    let mut defined: TryVec<TryVec<u8>> = TryVec::new();
    // `ifdef` nesting: whether each open branch is taken.
    let mut cond: TryVec<(bool, bool)> = TryVec::new();
    let mut in_section = false;
    let mut ruleset: u8 = 0;
    // Where the next line goes (`reorder-after` moves it), or the tail.
    let mut cursor: Option<usize> = None;
    // `..` waiting for the next line: the node it follows, its weights and the code point before it.
    let mut ellipsis: Option<(usize, TryVec<TryVec<Wt>>, u32)> = None;
    for s in stmts {
        let kw = s.keyword();
        let no = s.no;
        let args = s.args();
        let arg_word = || args.first().and_then(Tok::word).unwrap_or(&[]);
        match kw {
            b"ifdef" | b"ifndef" => {
                let on = defined.iter().any(|d| d.as_slice() == arg_word());
                let on = if kw == b"ifdef" { on } else { !on };
                let outer = cond.iter().all(|c| c.0);
                cond.push((on && outer, on))?;
                continue;
            }
            b"else" => {
                let outer = cond.len() < 2 || cond[..cond.len() - 1].iter().all(|c| c.0);
                let Some(top) = cond.last_mut() else { return Err(Error::syntax(no, "else without ifdef")) };
                *top = (!top.1 && outer, true);
                continue;
            }
            b"endif" => {
                cond.pop().ok_or(Error::syntax(no, "endif without ifdef"))?;
                continue;
            }
            _ => {}
        }
        if !cond.iter().all(|c| c.0) {
            continue;
        }
        match kw {
            b"define" => defined.push(TryVec::from_slice(arg_word())?)?,
            b"script" => {}
            b"collating-symbol" => {
                let mut add = |name: &[u8]| -> Result<()> {
                    let id = u32::try_from(n.syms.len()).unwrap_or(u32::MAX);
                    if !n.syms.contains_key(name) {
                        n.syms.insert(TryVec::from_slice(name)?.into_inner(), id);
                    }
                    Ok(())
                };
                match args {
                    [Tok::Sym(a)] => add(a)?,
                    [Tok::Sym(a), Tok::Word(d), Tok::Sym(b)] if d.as_slice() == b".." => name_range(a, b, &mut add).map_err(|e| Error { line: no, ..e })?,
                    _ => return Err(Error::syntax(no, "collating-symbol needs a <name>")),
                }
            }
            b"collating-element" => {
                let (Some(Tok::Sym(name)), Some(Tok::Word(from)), Some(Tok::Str(raw))) = (args.first(), args.get(1), args.get(2)) else {
                    return Err(Error::syntax(no, "collating-element needs <name> from \"string\""));
                };
                if from.as_slice() != b"from" {
                    return Err(Error::syntax(no, "collating-element needs from"));
                }
                let seq = lex::decode_string(raw, s.esc, no)?;
                if seq.len() < 2 {
                    return Err(Error::syntax(no, "a collating element needs two characters or more"));
                }
                match n.elems.get(name.as_slice()) {
                    Some(&id) => {
                        if let Some(old) = c.elems.get_mut(usize::try_from(id).unwrap_or(usize::MAX)) {
                            *old = seq;
                        }
                    }
                    None => {
                        let id = u32::try_from(c.elems.len()).unwrap_or(u32::MAX);
                        c.elems.push(seq)?;
                        n.elems.insert(name.try_clone()?.into_inner(), id);
                    }
                }
            }
            b"order_start" => {
                let mut rules = TryVec::new();
                for f in fields(args) {
                    if let [Tok::Sym(_)] = f {
                        continue; // the section's script name
                    }
                    rules.push(rule(f, no)?)?;
                }
                if rules.is_empty() {
                    rules.push(FORWARD)?;
                }
                if rules.len() > MAX_LEVELS {
                    return Err(Error::syntax(no, "too many levels"));
                }
                if c.levels == 0 {
                    c.levels = rules.len();
                } else if c.levels != rules.len() {
                    return Err(Error::syntax(no, "sections with different numbers of levels"));
                }
                let at = match c.rulesets.iter().position(|r| r.as_slice() == rules.as_slice()) {
                    Some(at) => at,
                    None => {
                        c.rulesets.push(rules)?;
                        c.rulesets.len() - 1
                    }
                };
                ruleset = u8::try_from(at).map_err(|_| Error::syntax(no, "too many level rule sets"))?;
                in_section = true;
                cursor = None;
            }
            b"order_end" => {
                in_section = false;
                cursor = None;
            }
            b"reorder-after" => {
                let [Tok::Sym(a)] = args else { return Err(Error::syntax(no, "reorder-after needs a <name>")) };
                let k = n.key(a, no)?;
                let &at = c.order.at.get(&k).ok_or(Error::syntax(no, "reorder-after names an item without a place"))?;
                ruleset = c.order.nodes[at].ruleset;
                cursor = Some(at);
            }
            b"reorder-end" => cursor = None,
            b"reorder-sections-after" | b"reorder-sections-end" => {
                return Err(Error::syntax(no, "reorder-sections-after is not supported"));
            }
            _ => {
                // An order line: an item (or UNDEFINED, or an ellipsis), then its weights.
                let toks = s.toks.as_slice();
                let rest = toks.get(1..).unwrap_or(&[]);
                let after = cursor.unwrap_or(c.order.tail);
                let levels = c.levels;
                let placed = match toks.first() {
                    Some(Tok::Word(w)) if w.as_slice() == b".." || w.as_slice() == b"..." => {
                        let prev = c.order.nodes[after].key;
                        let Key::Char(cp) = prev else { return Err(Error::syntax(no, "an ellipsis not after a character")) };
                        ellipsis = Some((after, weights(rest, levels, &n, s.esc, no)?, cp));
                        continue;
                    }
                    Some(Tok::Word(w)) if w.as_slice() == b"UNDEFINED" => {
                        c.explicit_undef = true;
                        let ws = weights(rest, levels, &n, s.esc, no)?;
                        c.order.place(Key::Undef, Some(ws), ruleset, after)?
                    }
                    Some(Tok::Sym(name)) => {
                        let k = n.key(name, no)?;
                        let mut after = after;
                        if let Some((from, ws, lo)) = ellipsis.take() {
                            let Key::Char(hi) = k else { return Err(Error::syntax(no, "an ellipsis not before a character")) };
                            after = from;
                            for cp in lo.saturating_add(1)..hi {
                                if !cs.has(cp) || c.order.at.contains_key(&Key::Char(cp)) {
                                    continue;
                                }
                                after = c.order.place(Key::Char(cp), Some(clone_weights(&ws)?), ruleset, after)?;
                            }
                        }
                        let ws = if matches!(k, Key::Sym(_)) && rest.is_empty() { None } else { Some(weights(rest, levels, &n, s.esc, no)?) };
                        // A line under reorder-after moves an item placed already: it keeps its
                        // section's rules (tr_TR moves letters after symbols of no section).
                        let rules = match (cursor, c.order.at.get(&k)) {
                            (Some(_), Some(&old)) => c.order.nodes[old].ruleset,
                            _ => ruleset,
                        };
                        c.order.place(k, ws, rules, after)?
                    }
                    _ => return Err(Error::syntax(no, "an unknown collation statement")),
                };
                if cursor.is_some() {
                    cursor = Some(placed);
                }
                if in_section && c.first_in_section.is_none() {
                    c.first_in_section = Some(placed);
                }
            }
        }
    }
    if ellipsis.is_some() {
        return Err(Error::syntax(0, "an ellipsis at the end of the order"));
    }
    if !cond.is_empty() {
        return Err(Error::syntax(0, "ifdef without endif"));
    }
    Ok(c)
}

/// Prefix-free, order-keeping byte codes for the ranks `0..n` of one level's weights: no code
/// byte is 0 and no first byte is 0 or 1 (the key's terminator and level separator).
struct Codes {
    one: usize,
    three: bool,
}

impl Codes {
    fn new(n: usize) -> Codes {
        if n <= 254 {
            return Codes { one: 254, three: false };
        }
        let leads = (n - 254).div_ceil(254);
        if leads <= 254 {
            Codes { one: 254 - leads, three: false }
        } else {
            Codes { one: 0, three: true }
        }
    }

    fn put(&self, r: usize, out: &mut TryVec<u8>) -> Result<()> {
        let b = |v: usize| u8::try_from(v).map_err(|_| Error::syntax(0, "too many weights in one level"));
        if self.three {
            out.push(b(2 + r / (255 * 255))?)?;
            out.push(b(1 + (r / 255) % 255)?)?;
            out.push(b(1 + r % 255)?)?;
        } else if r < self.one {
            out.push(b(2 + r)?)?;
        } else {
            let r = r - self.one;
            out.push(b(2 + self.one + r / 255)?)?;
            out.push(b(1 + r % 255)?)?;
        }
        Ok(())
    }
}

/// One element to write: its multibyte bytes, its code points, ruleset and weights per level.
struct Out {
    bytes: Option<TryVec<u8>>,
    cps: TryVec<u32>,
    ruleset: u8,
    weights: TryVec<TryVec<u32>>,
}

/// Write `LC_COLLATE` for the statements `stmts` (after `copy`) and the charmap `cs`.
pub fn write(stmts: &[Stmt], cs: &Charset) -> Result<TryVec<u8>> {
    let mut c = parse(stmts, cs)?;
    let mut f = File::new();
    if c.levels == 0 {
        // No rules: code point order (D.1.4 `_NL_COLLATE_NRULES` 0).
        f.word(0)?;
        for _ in 1..19 {
            f.raw(true, TryVec::new())?;
        }
        return f.finish(MAGIC[3]);
    }
    let levels = c.levels;
    // The shared undefined element goes before the first character of the sections, wherever
    // an UNDEFINED line stands (ja_JP's last line: its undefined characters sort first).
    let (ws, rules) = match c.order.at.get(&Key::Undef).copied() {
        Some(u) => {
            if c.first_in_section == Some(u) {
                c.first_in_section = Some(c.order.nodes[u].next).filter(|&n| n != usize::MAX);
            }
            let node = &mut c.order.nodes[u];
            (node.weights.take(), node.ruleset)
        }
        None => (None, 0),
    };
    let ws = match ws {
        Some(ws) => ws,
        None => {
            let mut ws: TryVec<TryVec<Wt>> = TryVec::new();
            for l in 0..levels {
                let mut w = TryVec::new();
                if l + 1 == levels {
                    w.push(Wt::Me)?;
                }
                ws.push(w)?;
            }
            ws
        }
    };
    if let Some(u) = c.order.at.get(&Key::Undef).copied() {
        c.order.unlink(u);
        c.order.at.remove(&Key::Undef);
    }
    let after = c.first_in_section.map_or(c.order.tail, |i| c.order.nodes[i].prev);
    c.order.place(Key::Undef, Some(ws), rules, after)?;
    // Number the order.
    let mut pos: BTreeMap<Key, u32> = BTreeMap::new();
    let mut next: u32 = 2;
    let mut i = c.order.nodes[0].next;
    while i != usize::MAX {
        pos.insert(c.order.nodes[i].key, next);
        next += 1;
        i = c.order.nodes[i].next;
    }
    let resolve = |ws: &TryVec<TryVec<Wt>>, me: u32| -> Result<TryVec<TryVec<u32>>> {
        let mut out = TryVec::new();
        for l in ws.iter() {
            let mut v = TryVec::new();
            for w in l.iter() {
                v.push(match *w {
                    Wt::Me => me,
                    Wt::Of(k) => *pos.get(&k).ok_or(Error::syntax(0, "a weight names an item without a place in the order"))?,
                })?;
            }
            out.push(v)?;
        }
        Ok(out)
    };
    // The undefined element first: its weights are at offset 0, which absent entries name.
    let undef_node = c.order.at.get(&Key::Undef).copied().ok_or(Error::syntax(0, "no undefined element"))?;
    let undef_pos = pos.get(&Key::Undef).copied().unwrap_or(0);
    let undef_ws = c.order.nodes[undef_node].weights.as_ref().ok_or(Error::syntax(0, "UNDEFINED without weights"))?;
    let mut outs: TryVec<Out> = TryVec::new();
    outs.push(Out { bytes: None, cps: TryVec::new(), ruleset: 0, weights: resolve(undef_ws, undef_pos)? })?;
    // Collation-sequence values: each character's place among the characters.
    let mut collseq: BTreeMap<u32, u32> = BTreeMap::new();
    let mut seq: u32 = 0;
    let mut i = c.order.nodes[0].next;
    while i != usize::MAX {
        let node = &c.order.nodes[i];
        i = node.next;
        let Some(ws) = node.weights.as_ref() else { continue };
        let me = pos.get(&node.key).copied().unwrap_or(0);
        let cps = match node.key {
            Key::Char(cp) => {
                collseq.insert(cp, seq);
                seq += 1;
                TryVec::from_slice(&[cp])?
            }
            Key::Elem(e) => c.elems.get(usize::try_from(e).unwrap_or(usize::MAX)).ok_or(Error::syntax(0, "a bad element"))?.try_clone()?,
            _ => continue,
        };
        if !cps.iter().all(|&cp| cs.has(cp)) {
            continue;
        }
        let mut b = TryVec::new();
        let mut ok = true;
        for &cp in cps.iter() {
            ok &= cs.encode(cp, &mut b)?;
        }
        outs.push(Out { bytes: ok.then_some(b), cps, ruleset: node.ruleset, weights: resolve(ws, me)? })?;
    }
    write_tables(&mut f, &c, &outs, &collseq, cs)?;
    f.finish(MAGIC[3])
}

/// Store `data` in `pool`, sharing identical runs; its offset in units of `unit` bytes.
fn share(pool: &mut TryVec<u8>, seen: &mut BTreeMap<Vec<u8>, usize>, data: TryVec<u8>, unit: usize) -> Result<usize> {
    if let Some(&at) = seen.get(data.as_slice()) {
        return Ok(at);
    }
    let at = pool.len() / unit;
    pool.extend_from_slice(&data)?;
    seen.insert(data.into_inner(), at);
    Ok(at)
}

fn entry(ruleset: u8, offset: usize) -> Result<i32> {
    if offset >= 1 << 24 {
        return Err(Error::syntax(0, "the weight table is over 16 MiB"));
    }
    i32::try_from(u32::from(ruleset) << 24 | u32::try_from(offset).unwrap_or(0)).map_err(|_| Error::syntax(0, "too many level rule sets"))
}

fn write_tables(f: &mut File, c: &Collation, outs: &[Out], collseq: &BTreeMap<u32, u32>, cs: &Charset) -> Result<()> {
    let levels = c.levels;
    // Per level, the ranks of the weights used, for the multibyte codes.
    let mut codes: TryVec<(BTreeMap<u32, usize>, Codes)> = TryVec::new();
    for l in 0..levels {
        let mut used: BTreeMap<u32, usize> = BTreeMap::new();
        for o in outs.iter().filter(|o| o.bytes.is_some() || o.cps.is_empty()) {
            for &w in o.weights[l].iter() {
                used.insert(w, 0);
            }
        }
        for (r, v) in used.values_mut().enumerate() {
            *v = r;
        }
        let n = used.len();
        codes.push((used, Codes::new(n)))?;
    }
    let mut wmb: TryVec<u8> = TryVec::new();
    let mut wwc: TryVec<u8> = TryVec::new();
    let mut seen_mb: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    let mut seen_wc: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    // Per element: (multibyte entry, wide entry).
    let mut ents: TryVec<(i32, i32)> = TryVec::new();
    for o in outs {
        let mut mb = TryVec::new();
        let mut wc = TryVec::new();
        for (l, ws) in o.weights.iter().enumerate() {
            let mut b = TryVec::new();
            for &w in ws.iter() {
                if let Some(&r) = codes[l].0.get(&w) {
                    codes[l].1.put(r, &mut b)?;
                }
            }
            mb.push(u8::try_from(b.len()).map_err(|_| Error::syntax(0, "an element's weights at one level are over 255 bytes"))?)?;
            mb.extend_from_slice(&b)?;
            wc.extend_from_slice(&u32::try_from(ws.len()).unwrap_or(0).to_le_bytes())?;
            for &w in ws.iter() {
                wc.extend_from_slice(&w.to_le_bytes())?;
            }
        }
        let m = share(&mut wmb, &mut seen_mb, mb, 1)?;
        let w = share(&mut wwc, &mut seen_wc, wc, 4)?;
        ents.push((entry(o.ruleset, m)?, entry(o.ruleset, w)?))?;
    }
    // Multibyte lookup: one-byte elements in TABLEMB, longer ones in lists by their first byte.
    let mut tablemb = [0i32; 256];
    let mut lists: BTreeMap<u8, TryVec<(usize, i32)>> = BTreeMap::new();
    for (k, o) in outs.iter().enumerate().skip(1) {
        let Some(b) = o.bytes.as_ref() else { continue };
        match b.as_slice() {
            [one] => tablemb[usize::from(*one)] = ents[k].0,
            [first, ..] => lists.entry(*first).or_default().push((k, ents[k].0))?,
            [] => {}
        }
    }
    let mut extramb: TryVec<u8> = TryVec::new();
    extramb.extend_from_slice(&[0; 4])?;
    for (&first, list) in lists.iter_mut() {
        let rest = |k: usize| outs[k].bytes.as_ref().map_or(&[][..], |b| b.get(1..).unwrap_or(&[]));
        list.sort_by(|a, b| rest(b.0).len().cmp(&rest(a.0).len()).then(rest(a.0).cmp(rest(b.0))));
        let own = tablemb[usize::from(first)];
        let at = i32::try_from(extramb.len()).map_err(|_| Error::syntax(0, "the element lists are too large"))?;
        tablemb[usize::from(first)] = -at;
        let mut last: Option<&[u8]> = None;
        for &(k, e) in list.iter() {
            let r = rest(k);
            if last == Some(r) {
                continue; // two elements with the same bytes: the first placed wins
            }
            last = Some(r);
            extramb.extend_from_slice(&e.to_le_bytes())?;
            extramb.push(u8::try_from(r.len()).map_err(|_| Error::syntax(0, "a collating element over 255 bytes"))?)?;
            extramb.extend_from_slice(r)?;
            while extramb.len() % 4 != 0 {
                extramb.push(0)?;
            }
        }
        extramb.extend_from_slice(&own.to_le_bytes())?;
        extramb.extend_from_slice(&[0; 4])?;
    }
    // Wide lookup: code points in TABLEWC, elements in lists by their first code point.
    let mut single: BTreeMap<u32, i32> = BTreeMap::new();
    let mut wlists: BTreeMap<u32, TryVec<usize>> = BTreeMap::new();
    for (k, o) in outs.iter().enumerate().skip(1) {
        match o.cps.as_slice() {
            [one] => {
                single.insert(*one, ents[k].1);
            }
            [first, ..] => wlists.entry(*first).or_default().push(k)?,
            [] => {}
        }
    }
    let mut extrawc: TryVec<u8> = TryVec::new();
    extrawc.extend_from_slice(&[0; 4])?;
    let mut wide: BTreeMap<u32, i32> = single.clone();
    for (&first, list) in wlists.iter_mut() {
        let rest = |k: usize| outs[k].cps.get(1..).unwrap_or(&[]);
        list.sort_by(|&a, &b| rest(b).len().cmp(&rest(a).len()).then(rest(a).cmp(rest(b))));
        let at = i32::try_from(extrawc.len() / 4).map_err(|_| Error::syntax(0, "the element lists are too large"))?;
        let mut last: Option<&[u32]> = None;
        for &k in list.iter() {
            let r = rest(k);
            if last == Some(r) {
                continue;
            }
            last = Some(r);
            extrawc.extend_from_slice(&ents[k].1.to_le_bytes())?;
            extrawc.extend_from_slice(&u32::try_from(r.len()).unwrap_or(0).to_le_bytes())?;
            for &cp in r {
                extrawc.extend_from_slice(&cp.to_le_bytes())?;
            }
        }
        extrawc.extend_from_slice(&single.get(&first).copied().unwrap_or(0).to_le_bytes())?;
        extrawc.extend_from_slice(&[0; 4])?;
        wide.insert(first, -at);
    }
    let tablewc = crate::ctype::table_i32(&|cp| wide.get(&cp).copied())?;
    // Collation sequence by byte (one-byte characters only; 0xff otherwise) and by code point.
    let mut seqmb = [0xffu8; 256];
    let mut bytes: TryVec<(u32, u8)> = TryVec::new();
    for b in 0..=255u8 {
        if let Some(&s) = cs.byte_char(b).and_then(|cp| collseq.get(&cp)) {
            bytes.push((s, b))?;
        }
    }
    bytes.sort_unstable();
    for (r, &(_, b)) in bytes.iter().enumerate() {
        seqmb[usize::from(b)] = u8::try_from(r).unwrap_or(0xff);
    }
    let seqwc = crate::ctype::table_u32(&|cp| collseq.get(&cp).copied(), u32::MAX)?;

    f.word(u32::try_from(levels).unwrap_or(0))?; // 0 NRULES
    let mut rs = TryVec::new();
    for r in c.rulesets.iter() {
        rs.extend_from_slice(r)?;
    }
    f.raw(false, rs)?; // 1 RULESETS
    f.raw(true, i32s(&tablemb)?)?; // 2 TABLEMB
    f.raw(false, wmb)?; // 3 WEIGHTMB
    f.raw(true, extramb)?; // 4 EXTRAMB
    f.raw(true, TryVec::from_slice(&[0; 4])?)?; // 5 INDIRECTMB
    for _ in 6..9 {
        f.raw(true, TryVec::new())?;
    }
    f.raw(true, tablewc)?; // 9 TABLEWC
    f.raw(true, wwc)?; // 10 WEIGHTWC
    f.raw(true, extrawc)?; // 11 EXTRAWC
    f.raw(true, TryVec::from_slice(&[0; 4])?)?; // 12 INDIRECTWC
    // 13-15: the collating-symbol hash (names for `[[.name.]]`): one empty slot.
    f.word(1)?;
    f.raw(true, TryVec::from_slice(&[0; 8])?)?;
    f.raw(true, TryVec::new())?;
    f.raw(false, TryVec::from_slice(&seqmb)?)?; // 16 COLLSEQMB
    f.raw(true, seqwc)?; // 17 COLLSEQWC
    let mut name = TryVec::from_slice(cs.name())?;
    name.push(0)?;
    f.raw(false, name)?; // 18 CODESET
    Ok(())
}

fn i32s(v: &[i32]) -> Result<TryVec<u8>> {
    let mut out = TryVec::with_capacity(4 * v.len())?;
    for x in v {
        out.extend_from_slice(&x.to_le_bytes())?;
    }
    Ok(out)
}
