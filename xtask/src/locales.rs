//! The generated locale sources (LOCALE-SOURCES-GEN, D-31.27; owner ruling #7532: libniji makes
//! its locale data itself). `cargo xtask locale-sources <dir>` (and `cargo xtask so`, into
//! `target/niji/locale-sources`) writes one POSIX locale definition (POSIX.1-2017 XBD 7.3, the
//! form `localedef` reads) for each name of `spec/cldr/locale-names.tsv`, the shared files they
//! copy, and `SUPPORTED` (the list `locale-gen` reads). A name Unicode CLDR covers takes its values
//! from CLDR, except `LC_NAME`, `LC_ADDRESS` and `LC_TELEPHONE`, which CLDR has no source for: those
//! come from the black-box probe of the reference's compiled locales (`crate::locale_probe`,
//! LOCALE-PROBE-GEN, ruling #7791), as does every category of a name CLDR does not cover.
//!
//! Inputs, all in the tree (nothing is fetched at build time):
//! * `spec/cldr/<v>/locales.tsv` and `territories.tsv`: the CLDR values, already resolved through
//!   CLDR's inheritance by `tools/cldr/cldr_extract.py` from the pinned release (its SHA-256);
//! * `spec/cldr/<v>/allkeys_CLDR.txt`: CLDR's root collation (the UCA table with CLDR's changes);
//! * `spec/unicode/15.1.0`: the classes and case mappings, through the derivation the built-in
//!   `C.UTF-8` uses (`crate::unicode`, D-5.4), so every generated locale classifies as it does.
//!
//! Every character in a generated file is written as its `<Uxxxx>` name except ASCII letters,
//! digits and the space, so the files do not depend on any charmap's other names.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::locale_probe::{Probe, Val};

/// The pinned CLDR release (`tools/cldr/cldr_extract.py` `PIN_VERSION`).
pub const CLDR: &str = "49";
const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

type Vals = BTreeMap<String, String>;

/// The extracted CLDR data: per locale name, per territory, and the name list.
pub struct Data {
    pub locales: BTreeMap<String, Vals>,
    pub territories: BTreeMap<String, Vals>,
    /// (source name, SUPPORTED entry, charset), in the list's order.
    pub names: Vec<(String, String, String)>,
}

fn unescape(v: &str) -> String {
    let mut out = String::new();
    let mut it = v.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn table(text: &str) -> BTreeMap<String, Vals> {
    let mut m: BTreeMap<String, Vals> = BTreeMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let mut f = line.splitn(3, '\t');
        if let (Some(a), Some(k), Some(v)) = (f.next(), f.next(), f.next()) {
            m.entry(a.to_string()).or_default().insert(k.to_string(), unescape(v));
        }
    }
    m
}

impl Data {
    /// Read the data from the texts of `locales.tsv`, `territories.tsv` and `locale-names.tsv`.
    pub fn parse(locales: &str, territories: &str, names: &str) -> Data {
        let names = names
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .filter_map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                (f.len() == 3).then(|| (f[0].to_string(), f[1].to_string(), f[2].to_string()))
            })
            .collect();
        Data { locales: table(locales), territories: table(territories), names }
    }

    pub fn load(root: &Path) -> Result<Data, String> {
        let rd = |p: &str| fs::read_to_string(root.join(p)).map_err(|e| format!("{p}: {e}"));
        Ok(Data::parse(
            &rd(&format!("spec/cldr/{CLDR}/locales.tsv"))?,
            &rd(&format!("spec/cldr/{CLDR}/territories.tsv"))?,
            &rd("spec/cldr/locale-names.tsv")?,
        ))
    }

    /// A territory's value, else the world's (`001`).
    fn terr(&self, cc: &str, k: &str) -> Option<&str> {
        self.territories.get(cc).and_then(|t| t.get(k)).or_else(|| self.territories.get("001").and_then(|t| t.get(k))).map(String::as_str)
    }
}

/// A string as a locale source string: ASCII letters, digits and space as they are, every other
/// character as its `<Uxxxx>` name.
pub fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == ' ' {
            out.push(c);
        } else {
            out.push_str(&uname(c as u32));
        }
    }
    out.push('"');
    out
}

/// The `<Uxxxx>` (or `<Uxxxxxxxx>` above U+FFFF) symbolic name of a code point.
pub fn uname(c: u32) -> String {
    if c > 0xFFFF { format!("<U{c:08X}>") } else { format!("<U{c:04X}>") }
}

/// A CLDR date or time pattern as a `strftime` format. Numeric fields are written padded; with
/// `year4`, a two-digit year is written in full. Fields without a `strftime` conversion (era,
/// quarter, week numbers) are left out.
pub fn strftime(pattern: &str, year4: bool) -> String {
    conv(pattern, year4).trim().replace("%H:%M:%S", "%T")
}

/// A CLDR `dateTimeFormat` pattern (`{1}, {0}`) with `{1}` the date and `{0}` the time, both
/// already `strftime` formats; the literal text between them is converted as a pattern.
pub fn combine(pattern: &str, date: &str, time: &str) -> String {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(i) = rest.find('{') {
        out.push_str(&conv(&rest[..i], false));
        let tok = rest.get(i..i + 3).unwrap_or("");
        out.push_str(match tok {
            "{1}" => date,
            "{0}" => time,
            _ => "",
        });
        rest = rest.get(i + 3..).unwrap_or("");
    }
    out.push_str(&conv(rest, false));
    out.trim().replace("%H:%M:%S", "%T")
}

fn conv(pattern: &str, year4: bool) -> String {
    let mut out = String::new();
    // CLDR separates fields with no-break spaces (U+00A0, U+202F before AM/PM); the formats use the
    // space, which strptime's whitespace matching and programs splitting the output expect.
    let cs: Vec<char> = pattern.chars().map(|c| if matches!(c, '\u{a0}' | '\u{202f}') { ' ' } else { c }).collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '\'' {
            if cs.get(i + 1) == Some(&'\'') {
                out.push('\'');
                i += 2;
                continue;
            }
            i += 1;
            while i < cs.len() {
                if cs[i] == '\'' {
                    if cs.get(i + 1) != Some(&'\'') {
                        break;
                    }
                    i += 1; // '' inside a quoted literal: one quote
                } else if cs[i] == '%' {
                    out.push('%');
                }
                out.push(cs[i]);
                i += 1;
            }
            i += 1;
            continue;
        }
        if !c.is_ascii_alphabetic() {
            if c == '%' {
                out.push('%');
            }
            out.push(c);
            i += 1;
            continue;
        }
        let mut n = 1;
        while cs.get(i + n) == Some(&c) {
            n += 1;
        }
        i += n;
        let conv = match c {
            'y' | 'u' | 'r' if n == 2 && !year4 => "%y",
            'y' | 'u' | 'r' => "%Y",
            'M' | 'L' if n <= 2 => "%m",
            'M' | 'L' if n == 3 => "%b",
            'M' | 'L' => "%B",
            'd' => "%d",
            'E' | 'c' | 'e' if n <= 3 => "%a",
            'E' | 'c' | 'e' => "%A",
            'a' | 'b' | 'B' => "%p",
            'h' | 'K' => "%I",
            'H' | 'k' => "%H",
            'm' => "%M",
            's' => "%S",
            'z' | 'v' | 'V' | 'O' | 'Z' => "%Z",
            _ => "",
        };
        out.push_str(conv);
    }
    out
}

/// The `grouping` of a CLDR number pattern's integer part: the primary group size, then the
/// secondary one where it differs (`#,##,##0`: `3;2`); `-1` for a pattern without grouping.
pub fn grouping(pattern: &str) -> String {
    let int = pattern.split(';').next().unwrap_or("").split('.').next().unwrap_or("");
    let digits: String = int.chars().filter(|c| matches!(c, '#' | '0' | ',')).collect();
    let parts: Vec<&str> = digits.split(',').collect();
    match parts.len() {
        0 | 1 => "-1".into(),
        2 => parts[1].len().to_string(),
        n if parts[n - 2].len() != parts[n - 1].len() => format!("{};{}", parts[n - 1].len(), parts[n - 2].len()),
        n => parts[n - 1].len().to_string(),
    }
}

/// From a CLDR currency pattern: (the symbol precedes the value, a space separates them).
pub fn currency_place(pattern: &str) -> (bool, bool) {
    let pos = pattern.split(';').next().unwrap_or("");
    let cs: Vec<char> = pos.chars().collect();
    let sym = cs.iter().position(|&c| c == '\u{a4}');
    let num = cs.iter().position(|&c| matches!(c, '#' | '0'));
    let numl = cs.iter().rposition(|&c| matches!(c, '#' | '0'));
    match (sym, num, numl) {
        (Some(s), Some(n), _) if s < n => (true, cs[s + 1..n].iter().any(|c| c.is_whitespace() || *c == '\u{a0}' || *c == '\u{202f}')),
        (Some(s), _, Some(l)) => (false, cs[l + 1..s].iter().any(|c| c.is_whitespace() || *c == '\u{a0}' || *c == '\u{202f}')),
        _ => (true, false),
    }
}

/// `yesexpr`/`noexpr` and `yesstr`/`nostr` from CLDR's `posix` messages (`yes:y`): the
/// one-character answers in either case, `+1` (`-0`), the ASCII `y` (`n`) where the other answer
/// does not use it, and each longer answer whose first character is not already accepted.
pub fn answers(yes: &str, no: &str) -> (String, String, String, String) {
    let alts = |s: &str| s.split(':').map(str::to_string).filter(|a| !a.is_empty()).collect::<Vec<_>>();
    let (ya, na) = (alts(yes), alts(no));
    let letters = |a: &[String]| {
        let mut set: Vec<char> = Vec::new();
        for w in a {
            let mut it = w.chars();
            if let (Some(c), None) = (it.next(), it.next()) {
                for v in c.to_lowercase().chain(c.to_uppercase()) {
                    if !set.contains(&v) {
                        set.push(v);
                    }
                }
            }
        }
        set
    };
    let (mut yl, mut nl) = (letters(&ya), letters(&na));
    if !nl.contains(&'y') && !yl.contains(&'y') {
        yl.extend(['y', 'Y']);
    }
    if !yl.contains(&'n') && !nl.contains(&'n') {
        nl.extend(['n', 'N']);
    }
    let expr = |lead: &str, set: &[char], words: &[String]| {
        let class: String = lead.chars().chain(set.iter().copied()).collect();
        let long: Vec<&String> = words.iter().filter(|w| w.chars().count() > 1 && !w.chars().next().is_some_and(|c| set.contains(&c))).collect();
        if long.is_empty() {
            format!("^[{class}]")
        } else {
            let alt: Vec<&str> = long.iter().map(|s| s.as_str()).collect();
            format!("^([{class}]|{})", alt.join("|"))
        }
    };
    (expr("+1", &yl, &ya), expr("-0", &nl, &na), ya.first().cloned().unwrap_or_default(), na.first().cloned().unwrap_or_default())
}

fn list(v: &Vals, keys: impl Iterator<Item = String>) -> Option<String> {
    let items: Option<Vec<String>> = keys.map(|k| v.get(&k).map(|s| quote(s))).collect();
    items.map(|i| i.join(";"))
}

fn day_index(d: &str) -> usize {
    DAYS.iter().position(|x| *x == d).unwrap_or(0)
}

/// The locale source for `name` (`ll_CC[@modifier]`), or None when CLDR does not cover it.
/// When the probe `p` has the name, every category after LC_COLLATE is the probe's (#7791 for
/// LC_NAME, LC_ADDRESS and LC_TELEPHONE, which CLDR has no source for; #7890 for the others: the
/// reference's value by default) and LC_CTYPE carries the probed edits; CLDR gives
/// LC_IDENTIFICATION, and every category of a name the probe lacks.
pub fn source(d: &Data, p: Option<&Probe>, name: &str) -> Result<Option<String>, String> {
    let Some(v) = d.locales.get(name) else { return Ok(None) };
    let g = |k: &str| v.get(k).map(String::as_str).unwrap_or("");
    let base = name.split('@').next().unwrap_or(name);
    let (lang, cc) = base.split_once('_').unwrap_or((base, ""));
    let probed = p.and_then(|p| p.resolve(name).map(|n| (p, n)));
    let mut s = String::new();
    let w = &mut s;
    writeln!(w, "comment_char %\nescape_char /\n").ok();
    writeln!(w, "% {name}: generated by libniji (cargo xtask locale-sources) from Unicode CLDR {CLDR}").ok();
    writeln!(w, "% (CLDR locale {}), Unicode License v3; see the package's copyright file.", g("cldr.id")).ok();
    if let Some((_, pn)) = probed {
        writeln!(w, "% Every category after LC_COLLATE, and LC_CTYPE's edits: from a black-box probe of the").ok();
        writeln!(w, "% reference's compiled {pn} (spec/locale-probe).").ok();
    }
    writeln!(w).ok();

    writeln!(w, "LC_IDENTIFICATION").ok();
    let title = format!("{} locale for {}", g("names.language.en"), g("names.territory.en"));
    for (k, val) in [
        ("title", title.as_str()),
        ("source", "libniji"),
        ("address", ""),
        ("contact", ""),
        ("email", ""),
        ("tel", ""),
        ("fax", ""),
        ("language", g("names.language.en")),
        ("territory", g("names.territory.en")),
        ("revision", CLDR),
        ("date", ""),
    ] {
        writeln!(w, "{k:<10} {}", quote(val)).ok();
    }
    for c in ["LC_IDENTIFICATION", "LC_CTYPE", "LC_COLLATE", "LC_TIME", "LC_NUMERIC", "LC_MONETARY", "LC_MESSAGES", "LC_PAPER", "LC_NAME", "LC_ADDRESS", "LC_TELEPHONE", "LC_MEASUREMENT"] {
        writeln!(w, "category   {};{c}", quote("i18n:2012")).ok();
    }
    writeln!(w, "END LC_IDENTIFICATION\n").ok();

    match probed {
        Some((p, pn)) => {
            let (edits, gaps) = ctype_edits(p, print_name(p, name, pn))?;
            if !gaps.is_empty() {
                writeln!(w, "% gap: LC_CTYPE differs from en_US's in {}; en_US's is used there.", gaps.join(", ")).ok();
            }
            ctype(w, &edits);
        }
        // SpecialCasing.txt's language-conditioned single mappings (dotted and dotless i).
        None if matches!(lang, "tr" | "az") => {
            ctype(w, &[format!("toupper ({},{})", uname(0x69), uname(0x130)), format!("tolower ({},{})", uname(0x49), uname(0x131))]);
        }
        None => ctype(w, &[]),
    }
    writeln!(w, "LC_COLLATE\ncopy {}\nEND LC_COLLATE\n", quote("iso14651_t1")).ok();
    if let Some((p, pn)) = probed {
        probed_rest(w, p, pn)?;
        return Ok(Some(s));
    }

    writeln!(w, "LC_NUMERIC").ok();
    writeln!(w, "decimal_point {}", quote(g("numbers.symbol.decimal"))).ok();
    writeln!(w, "thousands_sep {}", quote(g("numbers.symbol.group"))).ok();
    writeln!(w, "grouping      {}", grouping(g("numbers.decimalFormat"))).ok();
    writeln!(w, "END LC_NUMERIC\n").ok();

    let cur = if name.ends_with("@euro") { "EUR" } else { d.terr(cc, "currency").unwrap_or("") };
    let digits = d.terr(cc, "currency.digits").unwrap_or("2");
    let (pre, sep) = currency_place(g("numbers.currencyFormat"));
    let (pre, sep) = (u8::from(pre), u8::from(sep));
    let mdec = v.get("numbers.symbol.currencyDecimal").map_or(g("numbers.symbol.decimal"), String::as_str);
    let mgrp = v.get("numbers.symbol.currencyGroup").map_or(g("numbers.symbol.group"), String::as_str);
    writeln!(w, "LC_MONETARY").ok();
    writeln!(w, "int_curr_symbol   {}", quote(&format!("{cur} "))).ok();
    writeln!(w, "currency_symbol   {}", quote(v.get("currency.symbol").map_or(cur, String::as_str))).ok();
    writeln!(w, "mon_decimal_point {}", quote(mdec)).ok();
    writeln!(w, "mon_thousands_sep {}", quote(mgrp)).ok();
    writeln!(w, "mon_grouping      {}", grouping(g("numbers.currencyFormat"))).ok();
    writeln!(w, "positive_sign     \"\"\nnegative_sign     {}", quote(g("numbers.symbol.minusSign"))).ok();
    writeln!(w, "int_frac_digits   {digits}\nfrac_digits       {digits}").ok();
    for p in ["p", "n"] {
        writeln!(w, "{p}_cs_precedes     {pre}\n{p}_sep_by_space    {sep}\n{p}_sign_posn      1").ok();
        writeln!(w, "int_{p}_cs_precedes {pre}\nint_{p}_sep_by_space 1\nint_{p}_sign_posn  1").ok();
    }
    writeln!(w, "END LC_MONETARY\n").ok();

    writeln!(w, "LC_TIME").ok();
    let days = |ctx: &str, wd: &str| list(v, DAYS.iter().map(|x| format!("days.{ctx}.{wd}.{x}")));
    let months = |ctx: &str, wd: &str| list(v, (1..=12).map(|i| format!("months.{ctx}.{wd}.{i}")));
    for (k, val) in [
        ("abday", days("format", "abbreviated")),
        ("day", days("format", "wide")),
        ("abmon", months("format", "abbreviated")),
        ("mon", months("format", "wide")),
        ("ab_alt_mon", months("stand-alone", "abbreviated")),
        ("alt_mon", months("stand-alone", "wide")),
    ] {
        if let Some(val) = val {
            writeln!(w, "{k:<11}{val}").ok();
        }
    }
    let tmed = g("timeFormats.medium");
    let t_fmt = if tmed.contains('h') || tmed.contains('K') { "%r".to_string() } else { strftime(tmed, false) };
    let dfmt = strftime(v.get("skeleton.yMd").map_or(g("dateFormats.short"), String::as_str), true);
    let dlong = strftime(v.get("skeleton.yMMMEd").map_or(g("dateFormats.medium"), String::as_str), false);
    let dt = combine(g("dateTimeFormats.medium"), &dlong, &t_fmt);
    writeln!(w, "d_t_fmt    {}", quote(&dt)).ok();
    writeln!(w, "d_fmt      {}", quote(&dfmt)).ok();
    writeln!(w, "t_fmt      {}", quote(&t_fmt)).ok();
    writeln!(w, "am_pm      {};{}", quote(g("dayPeriods.am")), quote(g("dayPeriods.pm"))).ok();
    writeln!(w, "t_fmt_ampm {}", quote(&strftime(g("skeleton.hms"), false))).ok();
    writeln!(w, "date_fmt   {}", quote(&format!("{dt} %Z"))).ok();
    let first = day_index(d.terr(cc, "firstDay").unwrap_or("mon"));
    let wend = day_index(d.terr(cc, "weekendEnd").unwrap_or("sun"));
    writeln!(w, "week       7;19971130;{}", d.terr(cc, "minDays").unwrap_or("1")).ok();
    writeln!(w, "first_weekday {}\nfirst_workday {}", first + 1, (wend + 1) % 7 + 1).ok();
    writeln!(w, "END LC_TIME\n").ok();

    let (ye, ne, ys, ns) = answers(g("posix.yesstr"), g("posix.nostr"));
    writeln!(w, "LC_MESSAGES\nyesexpr {}\nnoexpr  {}\nyesstr  {}\nnostr   {}\nEND LC_MESSAGES\n", quote(&ye), quote(&ne), quote(&ys), quote(&ns)).ok();

    let (h, wd) = if d.terr(cc, "paper") == Some("US-Letter") { (279, 216) } else { (297, 210) };
    writeln!(w, "LC_PAPER\nheight {h}\nwidth  {wd}\nEND LC_PAPER\n").ok();

    writeln!(w, "LC_NAME\nEND LC_NAME\n").ok();

    writeln!(w, "LC_ADDRESS").ok();
    writeln!(w, "country_name {}", quote(g("names.territory"))).ok();
    writeln!(w, "country_ab2  {}", quote(cc)).ok();
    if let Some(a3) = d.territories.get(cc).and_then(|t| t.get("alpha3")).filter(|s| !s.is_empty()) {
        writeln!(w, "country_ab3  {}", quote(a3)).ok();
    }
    if let Some(n) = d.territories.get(cc).and_then(|t| t.get("numeric")).filter(|s| !s.is_empty()) {
        writeln!(w, "country_num  {}", n.trim_start_matches('0')).ok();
    }
    writeln!(w, "lang_name    {}", quote(g("names.language"))).ok();
    if lang.len() == 2 {
        writeln!(w, "lang_ab      {}", quote(lang)).ok();
    }
    let term = v.get("names.alpha3.terminology").map_or(if lang.len() == 3 { lang } else { "" }, String::as_str);
    if !term.is_empty() {
        writeln!(w, "lang_term    {}", quote(term)).ok();
        writeln!(w, "lang_lib     {}", quote(v.get("names.alpha3.bibliographic").map_or(term, String::as_str))).ok();
    }
    writeln!(w, "END LC_ADDRESS\n").ok();

    writeln!(w, "LC_TELEPHONE\nEND LC_TELEPHONE\n").ok();

    writeln!(w, "LC_MEASUREMENT\nmeasurement {}\nEND LC_MEASUREMENT", if d.terr(cc, "measurement") == Some("US") { 2 } else { 1 }).ok();
    Ok(Some(s))
}

/// `LC_CTYPE`: the shared classes and maps (`i18n`), then the edits, and the transliterations.
fn ctype(w: &mut String, edits: &[String]) {
    writeln!(w, "LC_CTYPE\ncopy {}", quote("i18n")).ok();
    for e in edits {
        writeln!(w, "{e}").ok();
    }
    writeln!(w, "translit_start").ok();
    for t in ["translit_combining", "translit_compat"] {
        writeln!(w, "include {};{}", quote(t), quote("")).ok();
    }
    writeln!(w, "translit_end\nEND LC_CTYPE\n").ok();
}

const NAME_KW: [&str; 6] = ["name_fmt", "name_gen", "name_mr", "name_mrs", "name_miss", "name_ms"];
const ADDRESS_KW: [&str; 12] =
    ["postal_fmt", "country_name", "country_post", "country_ab2", "country_ab3", "country_num", "country_car", "country_isbn", "lang_name", "lang_ab", "lang_term", "lang_lib"];
const TELEPHONE_KW: [&str; 4] = ["tel_int_fmt", "tel_dom_fmt", "int_select", "int_prefix"];

/// One keyword line with the probed value: a string quoted, a number or byte list as printed
/// (checked to be one); nothing when the probe printed none or an empty list.
fn probed_kw(w: &mut String, p: &Probe, pn: &str, cat: &str, kw: &str) -> Result<(), String> {
    match p.get(pn, cat, kw)? {
        Some(Val::Str(s)) => writeln!(w, "{kw} {}", quote(&s)).ok(),
        Some(Val::Bare(b)) if b.is_empty() => None,
        Some(Val::Bare(b)) => {
            if !b.split(';').all(|n| n.strip_prefix('-').unwrap_or(n).bytes().all(|c| c.is_ascii_digit()) && !n.is_empty()) {
                return Err(format!("{pn} {cat} {kw}: {b:?} is not a number list"));
            }
            writeln!(w, "{kw} {b}").ok()
        }
        None => None,
    };
    Ok(())
}

/// A string-list keyword (`abday`, `alt_digits`, ...): `"a";"b"`, checked to have `n` items when
/// given; nothing when the probe printed an empty list. `locale -k` prints a fixed-length list as
/// one string (`"a;b"`) and `era` / `alt_digits` as quoted items (`"a";"b"`).
fn probed_list(w: &mut String, p: &Probe, pn: &str, cat: &str, kw: &str, n: Option<usize>) -> Result<(), String> {
    let Some(Val::Str(s)) = p.get(pn, cat, kw)? else { return Ok(()) };
    let items: Vec<&str> = s.split(if n.is_none() { "\";\"" } else { ";" }).collect();
    if n.is_some_and(|n| n != items.len()) {
        return Err(format!("{pn} {cat} {kw}: {} items, not {n:?}", items.len()));
    }
    writeln!(w, "{kw} {}", items.iter().map(|i| quote(i)).collect::<Vec<_>>().join(";")).ok();
    Ok(())
}

/// `LC_NAME`, `LC_ADDRESS` and `LC_TELEPHONE` from the probe of `pn` (no CLDR source: #7791).
fn probed_contact(w: &mut String, p: &Probe, pn: &str) -> Result<(), String> {
    for (cat, kws) in [("LC_NAME", &NAME_KW[..]), ("LC_ADDRESS", &ADDRESS_KW[..]), ("LC_TELEPHONE", &TELEPHONE_KW[..])] {
        writeln!(w, "{cat}\n% probed: the reference's {pn} (spec/locale-probe)").ok();
        for kw in kws {
            probed_kw(w, p, pn, cat, kw)?;
        }
        writeln!(w, "END {cat}\n").ok();
    }
    Ok(())
}

/// The LC_CTYPE lines that make `i18n` (en_US's classes and case maps) behave as the reference's
/// probed name `fp` (#7892): the code points the probe found it adds to a class (U+1361 ETHIOPIC
/// WORDSPACE in `space` for the Ethiopic names) and its own case pairs (dotted and dotless i for
/// the Turkic names), from `spec/locale-probe/ctype-diff.tsv`. What an addition cannot express (a
/// code point taken out of a class, or a fingerprint difference the diff does not list, as
/// `wcwidth`) is returned as a gap, the second list.
fn ctype_edits(p: &Probe, fp: &str) -> Result<(Vec<String>, Vec<String>), String> {
    let diff = p.ctype_diff(fp);
    let listed = |k: &str| diff.is_some_and(|d| d.contains_key(&format!("ctype.{k}")));
    let mut gaps: Vec<String> = differs(p, fp, "en_US.utf8", "ctype.")?.into_iter().filter(|k| !listed(k)).collect();
    let mut lines = Vec::new();
    for (item, d) in diff.into_iter().flatten() {
        let kw = item.strip_prefix("ctype.").ok_or_else(|| format!("{fp}: ctype-diff item {item:?}"))?;
        if !d.added.is_empty() {
            lines.push(format!("{kw} {}", runs(&d.added)));
        }
        if !d.mapped.is_empty() {
            lines.push(format!("{kw} {}", d.mapped.iter().map(|&(a, b)| format!("({},{})", uname(a), uname(b))).collect::<Vec<_>>().join(";")));
        }
        if !d.removed.is_empty() {
            gaps.push(format!("{kw} (without {})", d.removed.iter().map(|&c| format!("U+{c:04X}")).collect::<Vec<_>>().join(" ")));
        }
    }
    Ok((lines, gaps))
}

/// The probed name whose fingerprints stand for `pn` (a name probed only through its
/// modifier-less sibling uses the sibling's).
fn print_name<'a>(p: &'a Probe, name: &str, pn: &'a str) -> &'a str {
    p.resolve(name.split('@').next().unwrap_or(name)).filter(|_| p.prints(pn).is_none()).unwrap_or(pn)
}

/// The fingerprint keys of `pn` that differ from the probed name `like`'s (`ctype.` or `coll.`).
fn differs(p: &Probe, pn: &str, like: &str, prefix: &str) -> Result<Vec<String>, String> {
    let (Some(a), Some(b)) = (p.prints(pn), p.prints(like)) else { return Err(format!("no fingerprints for {pn} or {like}")) };
    Ok(a.iter().filter(|(k, v)| k.starts_with(prefix) && b.get(*k) != Some(v)).map(|(k, _)| k[prefix.len()..].to_string()).collect())
}

/// The locale source for a name CLDR does not cover, every category from the probe of the
/// reference's locale (#7791). `LC_CTYPE` and `LC_COLLATE` cannot be dumped: they are those of a
/// generated locale's with the probed edits (fingerprints of every class, case map and width,
/// and of `wcscoll` over each script group; `ctype_edits`): en_US's `i18n` and root order. A
/// difference the edits cannot express is a gap, written in the source's comment, and the source
/// takes en_US's (docs/differences-from-glibc.md).
pub fn probe_source(p: &Probe, name: &str) -> Result<Option<String>, String> {
    let Some(pn) = p.resolve(name) else { return Ok(None) };
    let fp = print_name(p, name, pn);
    let (edits, gaps) = ctype_edits(p, fp)?;
    let coll = differs(p, fp, "en_US.utf8", "coll.")?;
    let mut s = String::new();
    let w = &mut s;
    writeln!(w, "comment_char %\nescape_char /\n").ok();
    writeln!(w, "% {name}: generated by libniji (cargo xtask locale-sources) from a black-box probe of the").ok();
    writeln!(w, "% reference's compiled {pn} (spec/locale-probe; Unicode CLDR has no data for it).").ok();
    if !gaps.is_empty() {
        writeln!(w, "% gap: LC_CTYPE differs from en_US's in {}; en_US's is used there.", gaps.join(", ")).ok();
    }
    if !coll.is_empty() {
        writeln!(w, "% gap: LC_COLLATE differs from en_US's (root order) in {}; en_US's is used.", coll.join(", ")).ok();
    }
    writeln!(w).ok();
    let id = |k: &str| p.text(pn, "LC_IDENTIFICATION", k);
    let (language, territory) = (id("language")?, id("territory")?);
    writeln!(w, "LC_IDENTIFICATION").ok();
    let title = format!("{language} locale for {territory}");
    for (k, val) in
        [("title", title.as_str()), ("source", "libniji"), ("address", ""), ("contact", ""), ("email", ""), ("tel", ""), ("fax", ""), ("language", &language), ("territory", &territory), ("revision", "probe"), ("date", "")]
    {
        writeln!(w, "{k:<10} {}", quote(val)).ok();
    }
    for c in ["LC_IDENTIFICATION", "LC_CTYPE", "LC_COLLATE", "LC_TIME", "LC_NUMERIC", "LC_MONETARY", "LC_MESSAGES", "LC_PAPER", "LC_NAME", "LC_ADDRESS", "LC_TELEPHONE", "LC_MEASUREMENT"] {
        writeln!(w, "category   {};{c}", quote("i18n:2012")).ok();
    }
    writeln!(w, "END LC_IDENTIFICATION\n").ok();
    ctype(w, &edits);
    writeln!(w, "LC_COLLATE\ncopy {}\nEND LC_COLLATE\n", quote("iso14651_t1")).ok();
    probed_rest(w, p, pn)?;
    Ok(Some(s))
}

/// Every category after LC_COLLATE from the probe of `pn`: LC_NUMERIC, LC_MONETARY, LC_TIME,
/// LC_MESSAGES, LC_PAPER, LC_NAME, LC_ADDRESS, LC_TELEPHONE and LC_MEASUREMENT.
fn probed_rest(w: &mut String, p: &Probe, pn: &str) -> Result<(), String> {
    let plain = |w: &mut String, cat: &str, kws: &[&str]| -> Result<(), String> {
        writeln!(w, "{cat}").ok();
        for kw in kws {
            probed_kw(w, p, pn, cat, kw)?;
        }
        writeln!(w, "END {cat}\n").ok();
        Ok(())
    };
    plain(w, "LC_NUMERIC", &["decimal_point", "thousands_sep", "grouping"])?;
    plain(
        w,
        "LC_MONETARY",
        &[
            "int_curr_symbol", "currency_symbol", "mon_decimal_point", "mon_thousands_sep", "mon_grouping", "positive_sign", "negative_sign", "int_frac_digits", "frac_digits", "p_cs_precedes",
            "p_sep_by_space", "n_cs_precedes", "n_sep_by_space", "p_sign_posn", "n_sign_posn", "int_p_cs_precedes", "int_p_sep_by_space", "int_n_cs_precedes", "int_n_sep_by_space",
            "int_p_sign_posn", "int_n_sign_posn",
        ],
    )?;
    writeln!(w, "LC_TIME").ok();
    for (kw, n) in [("abday", Some(7)), ("day", Some(7)), ("abmon", Some(12)), ("mon", Some(12)), ("am_pm", Some(2))] {
        probed_list(w, p, pn, "LC_TIME", kw, n)?;
    }
    for kw in ["d_t_fmt", "d_fmt", "t_fmt", "t_fmt_ampm"] {
        probed_kw(w, p, pn, "LC_TIME", kw)?;
    }
    probed_list(w, p, pn, "LC_TIME", "era", None)?;
    for kw in ["era_d_fmt", "era_d_t_fmt", "era_t_fmt"] {
        if !p.text(pn, "LC_TIME", kw)?.is_empty() {
            probed_kw(w, p, pn, "LC_TIME", kw)?;
        }
    }
    probed_list(w, p, pn, "LC_TIME", "alt_digits", None)?;
    let num = |kw: &str| -> Result<String, String> {
        match p.get(pn, "LC_TIME", kw)? {
            Some(Val::Bare(b)) if !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit()) => Ok(b),
            other => Err(format!("{pn} LC_TIME {kw}: {other:?} is not a number")),
        }
    };
    writeln!(w, "week {};{};{}", num("week-ndays")?, num("week-1stday")?, num("week-1stweek")?).ok();
    for kw in ["first_weekday", "first_workday", "cal_direction"] {
        writeln!(w, "{kw} {}", num(kw)?).ok();
    }
    probed_kw(w, p, pn, "LC_TIME", "date_fmt")?;
    for kw in ["alt_mon", "ab_alt_mon"] {
        probed_list(w, p, pn, "LC_TIME", kw, Some(12))?;
    }
    writeln!(w, "END LC_TIME\n").ok();
    plain(w, "LC_MESSAGES", &["yesexpr", "noexpr", "yesstr", "nostr"])?;
    plain(w, "LC_PAPER", &["height", "width"])?;
    probed_contact(w, p, pn)?;
    plain(w, "LC_MEASUREMENT", &["measurement"])
}

/// The source `C`, compiled as `C.UTF-8`: the classes and case maps of the built-in `C.UTF-8`
/// (`i18n`, D-5.4), every code point collated in code point order (one `..` run; localedef
/// writes it as the compact no-rules table, D-31.31) and the POSIX locale's values (POSIX.1-2024
/// XBD 7.3) in the other categories. The run's names have equal widths: the reference's
/// localedef refuses `<U0000>..<U10FFFF>`.
pub fn c_source() -> String {
    let mut s = String::from("comment_char %\nescape_char /\n\n");
    let w = &mut s;
    writeln!(w, "% C: generated by libniji (cargo xtask locale-sources): the built-in C.UTF-8 locale as a source,").ok();
    writeln!(w, "% its collation code point order, its other categories the POSIX locale's (POSIX.1-2024 XBD 7.3).\n").ok();
    writeln!(w, "LC_IDENTIFICATION").ok();
    for (k, val) in [("title", "C locale"), ("source", "libniji"), ("address", ""), ("contact", ""), ("email", ""), ("tel", ""), ("fax", ""), ("language", ""), ("territory", ""), ("revision", "1.0"), ("date", "")] {
        writeln!(w, "{k:<10} {}", quote(val)).ok();
    }
    for c in ["LC_IDENTIFICATION", "LC_CTYPE", "LC_COLLATE", "LC_TIME", "LC_NUMERIC", "LC_MONETARY", "LC_MESSAGES", "LC_PAPER", "LC_NAME", "LC_ADDRESS", "LC_TELEPHONE", "LC_MEASUREMENT"] {
        writeln!(w, "category   {};{c}", quote("i18n:2012")).ok();
    }
    writeln!(w, "END LC_IDENTIFICATION\n").ok();
    writeln!(w, "LC_CTYPE\ncopy {}\nEND LC_CTYPE\n", quote("i18n")).ok();
    writeln!(w, "LC_COLLATE\norder_start forward\n<U{:08X}>\n..\n<U{:08X}>\norder_end\nEND LC_COLLATE\n", 0, 0x10_FFFF).ok();
    let strs = |v: &[&str]| v.iter().map(|x| quote(x)).collect::<Vec<_>>().join(";");
    writeln!(w, "LC_TIME").ok();
    writeln!(w, "abday   {}", strs(&["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"])).ok();
    writeln!(w, "day     {}", strs(&["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"])).ok();
    writeln!(w, "abmon   {}", strs(&["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"])).ok();
    writeln!(w, "mon     {}", strs(&["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"])).ok();
    for (k, val) in [("d_t_fmt", "%a %b %e %H:%M:%S %Y"), ("d_fmt", "%m/%d/%y"), ("t_fmt", "%H:%M:%S"), ("t_fmt_ampm", "%I:%M:%S %p"), ("date_fmt", "%a %b %e %H:%M:%S %Z %Y")] {
        writeln!(w, "{k:<10} {}", quote(val)).ok();
    }
    writeln!(w, "am_pm   {}\nEND LC_TIME\n", strs(&["AM", "PM"])).ok();
    writeln!(w, "LC_NUMERIC\ndecimal_point {}\nthousands_sep {}\ngrouping -1\nEND LC_NUMERIC\n", quote("."), quote("")).ok();
    writeln!(w, "LC_MONETARY").ok();
    for k in ["int_curr_symbol", "currency_symbol", "mon_decimal_point", "mon_thousands_sep", "positive_sign", "negative_sign"] {
        writeln!(w, "{k:<17} {}", quote("")).ok();
    }
    for k in ["mon_grouping", "int_frac_digits", "frac_digits", "p_cs_precedes", "p_sep_by_space", "n_cs_precedes", "n_sep_by_space", "p_sign_posn", "n_sign_posn"] {
        writeln!(w, "{k:<17} -1").ok();
    }
    writeln!(w, "END LC_MONETARY\n").ok();
    writeln!(w, "LC_MESSAGES\nyesexpr {}\nnoexpr  {}\nEND LC_MESSAGES\n", quote("^[yY]"), quote("^[nN]")).ok();
    writeln!(w, "LC_PAPER\nheight 297\nwidth  210\nEND LC_PAPER\n").ok();
    // The built-in C.UTF-8's formats (spec/abi/langinfo.tsv), which localedef requires.
    writeln!(w, "LC_NAME\nname_fmt {}\nEND LC_NAME\n", quote("%p%t%g%t%m%t%f")).ok();
    writeln!(w, "LC_ADDRESS\npostal_fmt {}\nEND LC_ADDRESS\n", quote("%a%N%f%N%d%N%b%N%s %h %e %r%N%C-%z %T%N%c%N")).ok();
    writeln!(w, "LC_TELEPHONE\ntel_int_fmt {}\nEND LC_TELEPHONE\n", quote("+%c %a %l")).ok();
    writeln!(w, "LC_MEASUREMENT\nmeasurement 1\nEND LC_MEASUREMENT").ok();
    s
}

/// Code points as `<Ua>..<Ub>` runs joined by `;`, wrapped with `/` continuation lines.
fn runs(cps: &[u32]) -> String {
    let mut items = Vec::new();
    let mut i = 0;
    while i < cps.len() {
        let mut j = i;
        while j + 1 < cps.len() && cps[j + 1] == cps[j] + 1 {
            j += 1;
        }
        items.push(if i == j { uname(cps[i]) } else { format!("{}..{}", uname(cps[i]), uname(cps[j])) });
        i = j + 1;
    }
    let mut out = String::new();
    for (n, it) in items.iter().enumerate() {
        if n > 0 {
            out.push_str(if n % 4 == 0 { ";/\n   " } else { ";" });
        }
        out.push_str(it);
    }
    out
}

/// `i18n_ctype`: the classes and case mappings of every code point, as `C.UTF-8` has them.
pub fn i18n_ctype(root: &Path) -> Result<String, String> {
    let props = crate::unicode::ctype_props(root)?;
    let mut s = String::from("comment_char %\nescape_char /\n\n");
    let w = &mut s;
    writeln!(w, "% i18n_ctype: generated by libniji (cargo xtask locale-sources) from spec/unicode/15.1.0,").ok();
    writeln!(w, "% the classes and simple case mappings of the built-in C.UTF-8 locale (D-5.4).\n").ok();
    writeln!(w, "LC_CTYPE").ok();
    for (name, bit) in crate::unicode::CLASS_KEYWORDS {
        let cps: Vec<u32> = (0u32..).zip(&props).filter(|(_, p)| p.class & bit != 0).map(|(c, _)| c).collect();
        writeln!(w, "{name} {}\n", runs(&cps)).ok();
    }
    for (name, bit) in [("combining", 1u8), ("combining_level3", 2)] {
        let cps: Vec<u32> = (0u32..).zip(&props).filter(|(_, p)| p.extra & bit != 0).map(|(c, _)| c).collect();
        writeln!(w, "class {};/\n   {}\n", quote(name), runs(&cps)).ok();
    }
    type Map = fn(&crate::unicode::CtypeProps) -> u32;
    let maps: [(&str, Map); 3] = [("toupper", |p| p.upper), ("tolower", |p| p.lower), ("map \"totitle\";", |p| p.title)];
    for (name, to) in maps {
        let pairs: Vec<String> = (0u32..).zip(&props).filter(|&(c, p)| to(p) != c).map(|(c, p)| format!("({},{})", uname(c), uname(to(p)))).collect();
        let lines: Vec<String> = pairs.chunks(6).map(|c| c.join(";")).collect();
        let sep = if name.starts_with("map") { "/\n   " } else { "" };
        writeln!(w, "{name} {sep}{}\n", lines.join(";/\n   ")).ok();
    }
    writeln!(w, "END LC_CTYPE").ok();
    Ok(s)
}

/// `translit_combining` (`compat` false) and `translit_compat` (`compat` true): transliteration
/// tables from the Unicode decompositions (UAX #15's full decompositions, UnicodeData.txt field
/// 5) with the combining marks (General_Category M*) left out: `é` to `e`, `Å` (U+212B) to `A`,
/// and, for the compatibility table, `ﬁ` to `fi` and `²` to `2`. A code point whose result is
/// empty or itself is not listed; the compatibility table lists only compatibility mappings.
pub fn translit(root: &Path, compat: bool) -> Result<String, String> {
    let marks = crate::unicode::category(root, "M")?;
    let name = if compat { "translit_compat" } else { "translit_combining" };
    let mut s = String::from("comment_char %\nescape_char /\n\n");
    let w = &mut s;
    writeln!(w, "% {name}: generated by libniji (cargo xtask locale-sources) from spec/unicode/15.1.0's").ok();
    writeln!(w, "% decompositions, combining marks removed.\n\nLC_CTYPE\ntranslit_start").ok();
    for (c, (is_compat, d)) in crate::unicode::full_decompositions(root, compat)? {
        let to: Vec<u32> = d.into_iter().filter(|x| !marks.contains(x)).collect();
        if is_compat != compat || to.is_empty() || to == [c] || marks.contains(&c) {
            continue;
        }
        let to = if let [x] = to[..] { uname(x) } else { format!("\"{}\"", to.iter().map(|&x| uname(x)).collect::<String>()) };
        writeln!(w, "{} {to}", uname(c)).ok();
    }
    writeln!(w, "translit_end\nEND LC_CTYPE").ok();
    Ok(s)
}

/// `i18n`: the `LC_CTYPE` other locales copy.
pub fn i18n() -> String {
    format!("comment_char %\nescape_char /\n\n% i18n: generated by libniji (cargo xtask locale-sources).\n\nLC_CTYPE\ncopy {}\nEND LC_CTYPE\n", quote("i18n_ctype"))
}

/// One collation element of `allkeys_CLDR.txt`: the weights of levels 1 to 3.
type Ce = [u16; 3];

/// One row of `allkeys_CLDR.txt`: its code points and its elements, each with whether it is
/// variable (`*`: CLDR's root marks spaces and punctuation, its default maxVariable).
type Row = (Vec<u32>, Vec<(Ce, bool)>);

/// The elements of one row under UTS #10's "shifted" variable weighting (section 4, "Variable
/// Weighting") with UTS #35's maxVariable `symbol`: an element marked variable or with a
/// primary weight from 1 to `max_variable` (spaces, punctuation and the symbols other than
/// currency), and an element ignorable at level 1 that follows one, is ignorable at levels 1 to
/// 3 (its distinction is left to the last level).
fn shifted(ces: &[(Ce, bool)], max_variable: u16) -> Vec<Ce> {
    let mut after_variable = false;
    let mut out = Vec::new();
    for &(ce, var) in ces {
        if ce == [0, 0, 0] {
            // Completely ignorable: no weight at any of these levels.
            continue;
        }
        if var || (ce[0] != 0 && ce[0] <= max_variable) || (after_variable && ce[0] == 0) {
            after_variable = true;
        } else {
            after_variable = false;
            out.push(ce);
        }
    }
    out
}

fn parse_allkeys(text: &str) -> Result<Vec<Row>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('@') {
            continue;
        }
        let (cps, ces) = line.split_once(';').ok_or_else(|| format!("allkeys:{}: no ';'", n + 1))?;
        let cps = cps.split_whitespace().map(|h| u32::from_str_radix(h, 16).map_err(|e| format!("allkeys:{}: {e}", n + 1))).collect::<Result<Vec<_>, _>>()?;
        let mut v = Vec::new();
        for ce in ces.split('[').skip(1) {
            let ce = ce.trim_end_matches(|c: char| c == ']' || c.is_whitespace());
            let ws: Vec<u16> = ce[1..].split('.').map(|h| u16::from_str_radix(h, 16)).collect::<Result<_, _>>().map_err(|e| format!("allkeys:{}: {e}", n + 1))?;
            if ws.len() < 3 {
                return Err(format!("allkeys:{}: a weight list of {}", n + 1, ws.len()));
            }
            v.push(([ws[0], ws[1], ws[2]], ce.starts_with('*')));
        }
        out.push((cps, v));
    }
    Ok(out)
}

/// `iso14651_t1`: CLDR's root collation as an `LC_COLLATE` table. Each distinct weight of each
/// level is a collating symbol, placed in weight order; each character (or contraction) of
/// `allkeys_CLDR.txt` has, per level, the string of its non-zero weights (`IGNORE` when none),
/// and itself at level 4. Spaces, punctuation and the symbols other than currency (the primary
/// weights below the first currency symbol's: UTS #35 maxVariable `symbol`) are weighted
/// "shifted" (UTS #10 section 4; D-31.27): `a-b`, `ab` and `a b` differ only at the last level,
/// as `sort` and `ls` users of the C library expect. At that level the characters ignorable from
/// the start come first, then the shifted ones, each in code point order. The code points UTS #10
/// orders by implicit weights (Han and the other ideographic scripts,
/// `crate::unicode::implicit_groups`) follow every listed item, each its own primary weight in
/// code point order, as ranges (`..` lines). The rest (unassigned and private-use code points)
/// share the undefined element: ignorable but at the last level, where it sorts first.
pub fn iso14651_t1(root: &Path) -> Result<String, String> {
    let p = root.join(format!("spec/cldr/{CLDR}/allkeys_CLDR.txt"));
    let text = fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let parsed = parse_allkeys(&text)?;
    let currency = crate::unicode::category(root, "Sc")?;
    let first_currency = parsed
        .iter()
        .filter(|(cps, _)| matches!(cps[..], [c] if currency.contains(&c)))
        .filter_map(|(_, ces)| ces.first().map(|(ce, _)| ce[0]))
        .filter(|&p| p != 0)
        .min()
        .ok_or("allkeys_CLDR.txt: no currency symbol")?;
    // Each row: its code points, its elements once shifted, and whether it had a primary weight
    // before (a shifted row sorts after the rows ignorable from the start).
    let rows: Vec<(Vec<u32>, Vec<Ce>, bool)> =
        parsed.into_iter().map(|(cps, ces)| (cps, shifted(&ces, first_currency - 1), ces.iter().any(|(ce, _)| ce[0] != 0))).collect();
    let mut levels: [BTreeSet<u16>; 3] = Default::default();
    for (_, ces, _) in &rows {
        for ce in ces {
            for l in 0..3 {
                if ce[l] != 0 {
                    levels[l].insert(ce[l]);
                }
            }
        }
    }
    let sym = |l: usize, x: u16| format!("<L{}-{x:04X}>", l + 1);
    let name = |cps: &[u32]| {
        if cps.len() == 1 { uname(cps[0]) } else { format!("<CE-{}>", cps.iter().map(|c| format!("{c:04X}")).collect::<Vec<_>>().join("-")) }
    };
    let mut s = String::from("comment_char %\nescape_char /\n\n");
    let w = &mut s;
    writeln!(w, "% iso14651_t1: generated by libniji (cargo xtask locale-sources) from Unicode CLDR {CLDR}'s").ok();
    writeln!(w, "% root collation (allkeys_CLDR.txt), Unicode License v3; variable elements shifted (UTS #10).\n\nLC_COLLATE").ok();
    for (l, set) in levels.iter().enumerate() {
        for &x in set {
            writeln!(w, "collating-symbol {}", sym(l, x)).ok();
        }
    }
    for (cps, _, _) in rows.iter().filter(|(c, _, _)| c.len() > 1) {
        writeln!(w, "collating-element {} from \"{}\"", name(cps), cps.iter().map(|&c| uname(c)).collect::<String>()).ok();
    }
    writeln!(w, "\norder_start forward;forward;forward;forward,position").ok();
    for (l, set) in levels.iter().enumerate().rev() {
        for &x in set {
            writeln!(w, "{}", sym(l, x)).ok();
        }
    }
    // The UCA's comparison: level 1's weights, then level 2's, then level 3's (UTS #10 section 7).
    let key = |ces: &[Ce]| -> [Vec<u16>; 3] { [0, 1, 2].map(|l| ces.iter().map(|c| c[l]).filter(|&x| x != 0).collect()) };
    let mut sorted: Vec<&(Vec<u32>, Vec<Ce>, bool)> = rows.iter().collect();
    sorted.sort_by(|a, b| key(&a.1).cmp(&key(&b.1)).then_with(|| a.2.cmp(&b.2)).then_with(|| a.0.cmp(&b.0)));
    let mut listed = BTreeSet::new();
    for (cps, ces, _) in sorted {
        let mut f = Vec::new();
        for l in 0..3 {
            let ws: String = ces.iter().filter(|c| c[l] != 0).map(|c| sym(l, c[l])).collect();
            f.push(if ws.is_empty() { "IGNORE".to_string() } else { format!("\"{ws}\"") });
        }
        f.push(name(cps));
        writeln!(w, "{} {}", name(cps), f.join(";")).ok();
        if let [c] = cps[..] {
            listed.insert(c);
        }
    }
    // Implicit weights: each code point its own primary (`..`, itself), the common secondary and
    // tertiary; runs of consecutive code points as `<a>`, `..`, `<b>` lines.
    let common = |l: usize| levels[l].iter().next().map_or("IGNORE".to_string(), |&x| format!("\"{}\"", sym(l, x)));
    let ws = format!("..;{};{};..", common(1), common(2));
    for group in crate::unicode::implicit_groups(root)? {
        let cps: Vec<u32> = group.into_iter().filter(|c| !listed.contains(c)).collect();
        for run in cps.chunk_by(|a, b| b.checked_sub(*a) == Some(1)) {
            match run {
                [a] => writeln!(w, "{} {ws}", uname(*a)).ok(),
                [a, .., b] => writeln!(w, "{} {ws}\n.. {ws}\n{} {ws}", uname(*a), uname(*b)).ok(),
                [] => None,
            };
        }
    }
    writeln!(w, "order_end\nEND LC_COLLATE").ok();
    Ok(s)
}

/// Write every generated file into `out`; returns the number of locale sources.
pub fn write(root: &Path, out: &Path) -> Result<usize, String> {
    let d = Data::load(root)?;
    let p = Probe::load(root)?;
    fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let put = |n: &str, text: &str| fs::write(out.join(n), text).map_err(|e| format!("{}/{n}: {e}", out.display()));
    let mut done = BTreeSet::new();
    let mut supported = String::new();
    for (src, entry, cs) in &d.names {
        if !done.contains(src) {
            // `C` (C.UTF-8) is libniji's own definition (D-31.31); every other name is generated from
            // CLDR with the probed values (D-31.30), or from the probe alone where CLDR lacks it.
            let text = if src == "C" {
                Some(c_source())
            } else {
                match source(&d, Some(&p), src)? {
                    Some(t) => Some(t),
                    None => probe_source(&p, src)?,
                }
            };
            if let Some(text) = text {
                put(src, &text)?;
                done.insert(src.clone());
            }
        }
        if done.contains(src) {
            writeln!(supported, "{entry} {cs}").ok();
        }
    }
    put("SUPPORTED", &supported)?;
    put("i18n", &i18n())?;
    put("i18n_ctype", &i18n_ctype(root)?)?;
    put("iso14651_t1", &iso14651_t1(root)?)?;
    put("translit_combining", &translit(root, false)?)?;
    put("translit_compat", &translit(root, true)?)?;
    Ok(done.len())
}

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let [dir] = args else { return Err("usage: cargo xtask locale-sources <output directory>".into()) };
    let n = write(root, Path::new(dir))?;
    println!("locale-sources: {n} locale sources from Unicode CLDR {CLDR} and the probe (spec/locale-probe) in {dir}");
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The generator over a fixed CLDR-shaped subset (two locales, three territories) and its
    //! conversions. Trace: no spec clause (generator tooling; the compiled result is checked
    //! against the reference by crates/locale/tests/reference.rs, s31_8_c_1).
    use super::*;

    const LOCALES: &str = "# fixture\n\
        xx_DE\tcldr.id\tde\nxx_DE\tnames.language.en\tGerman\nxx_DE\tnames.territory.en\tGermany\n\
        xx_DE\tnames.language\tDeutsch\nxx_DE\tnames.territory\tDeutschland\nxx_DE\tnames.alpha3.terminology\tdeu\n\
        xx_DE\tnames.alpha3.bibliographic\tger\nxx_DE\tnumbers.symbol.decimal\t,\nxx_DE\tnumbers.symbol.group\t.\n\
        xx_DE\tnumbers.symbol.minusSign\t-\nxx_DE\tnumbers.decimalFormat\t#,##0.###\n\
        xx_DE\tnumbers.currencyFormat\t#,##0.00\u{a0}\u{a4}\nxx_DE\tcurrency.symbol\t\u{20ac}\n\
        xx_DE\tposix.yesstr\tja:j\nxx_DE\tposix.nostr\tnein:n\nxx_DE\ttimeFormats.medium\tHH:mm:ss\n\
        xx_DE\tskeleton.yMd\td.M.y\nxx_DE\tskeleton.yMMMEd\tE, d. MMM y\nxx_DE\tskeleton.hms\th:mm:ss a\n\
        xx_DE\tdateTimeFormats.medium\t{1}, {0}\nxx_DE\tdayPeriods.am\tAM\nxx_DE\tdayPeriods.pm\tPM\n\
        xx_US\tcldr.id\ten\nxx_US\tnumbers.currencyFormat\t\u{a4}#,##0.00\nxx_US\ttimeFormats.medium\th:mm:ss a\n";
    const TERRITORIES: &str = "001\tfirstDay\tmon\n001\tminDays\t1\n001\tpaper\tA4\n001\tweekendEnd\tsun\n\
        DE\talpha3\tDEU\nDE\tnumeric\t276\nDE\tcurrency\tEUR\nDE\tcurrency.digits\t2\nDE\tminDays\t4\n\
        US\tpaper\tUS-Letter\nUS\tmeasurement\tUS\nUS\tfirstDay\tsun\nUS\tcurrency\tUSD\n";
    const NAMES: &str = "# names\nxx_DE\txx_DE.UTF-8\tUTF-8\nxx_US\txx_US.UTF-8\tUTF-8\nzz_ZZ\tzz_ZZ.UTF-8\tUTF-8\n";

    fn data() -> Data {
        Data::parse(LOCALES, TERRITORIES, NAMES)
    }

    /// The C source's collation is one run of equal-width names (the reference's localedef refuses
    /// `<U0000>..<U10FFFF>`) and its classes are i18n's (the compiled locale: reference.rs, D-31.31).
    #[test]
    fn c_source_orders_every_code_point_in_one_equal_width_run() {
        let s = c_source();
        assert!(s.contains("\nLC_COLLATE\norder_start forward\n<U00000000>\n..\n<U0010FFFF>\norder_end\nEND LC_COLLATE\n"), "{s}");
        assert!(s.contains("\nLC_CTYPE\ncopy \"i18n\"\nEND LC_CTYPE\n"), "{s}");
    }

    #[test]
    fn cldr_patterns_become_strftime_formats() {
        assert_eq!(strftime("M/d/y", true), "%m/%d/%Y");
        assert_eq!(strftime("dd.MM.yy", true), "%d.%m.%Y");
        assert_eq!(strftime("dd.MM.yy", false), "%d.%m.%y");
        assert_eq!(strftime("h:mm:ss a", false), "%I:%M:%S %p");
        assert_eq!(strftime("h:mm:ss\u{202f}a", false), "%I:%M:%S %p");
        assert_eq!(strftime("HH:mm:ss", false), "%T");
        assert_eq!(strftime("d 'de' MMMM 'de' y", false), "%d de %B de %Y");
        assert_eq!(strftime("EEEE, d. MMMM y G", false), "%A, %d. %B %Y");
        assert_eq!(strftime("y\u{5e74}M\u{6708}d\u{65e5}", false), "%Y\u{5e74}%m\u{6708}%d\u{65e5}");
        assert_eq!(strftime("H 'h' mm '%'", false), "%H h %M %%");
        assert_eq!(strftime("HH 'o''clock'", false), "%H o'clock");
        assert_eq!(combine("{1} 'um' {0}", "%d.%m.%Y", "%T"), "%d.%m.%Y um %T");
    }

    #[test]
    fn number_patterns_give_grouping_and_currency_placement() {
        assert_eq!(grouping("#,##0.###"), "3");
        assert_eq!(grouping("#,##,##0.###"), "3;2");
        assert_eq!(grouping("0.###"), "-1");
        assert_eq!(grouping("#,##0.00;(#,##0.00)"), "3");
        assert_eq!(currency_place("\u{a4}#,##0.00"), (true, false));
        assert_eq!(currency_place("#,##0.00\u{a0}\u{a4}"), (false, true));
        assert_eq!(currency_place("\u{a4}\u{a0}#,##0.00"), (true, true));
        assert_eq!(currency_place("#,##0.00\u{a4}"), (false, false));
    }

    #[test]
    fn posix_messages_give_answer_expressions() {
        assert_eq!(answers("yes:y", "no:n"), ("^[+1yY]".into(), "^[-0nN]".into(), "yes".into(), "no".into()));
        assert_eq!(answers("ja:j", "nein:n").0, "^[+1jJyY]");
        // A one-letter "no" of y keeps y out of the yes class.
        assert_eq!(answers("da:d", "ne:y").0, "^[+1dD]");
        assert_eq!(answers("\u{306f}\u{3044}:y", "\u{3044}\u{3044}\u{3048}:n").0, "^([+1yY]|\u{306f}\u{3044})");
    }

    #[test]
    fn a_source_carries_the_cldr_values_in_every_category() {
        let d = data();
        let s = source(&d, None, "xx_DE").unwrap().expect("xx_DE is covered");
        for cat in ["LC_IDENTIFICATION", "LC_CTYPE", "LC_COLLATE", "LC_NUMERIC", "LC_MONETARY", "LC_TIME", "LC_MESSAGES", "LC_PAPER", "LC_NAME", "LC_ADDRESS", "LC_TELEPHONE", "LC_MEASUREMENT"] {
            assert!(s.contains(&format!("\n{cat}\n")) && s.contains(&format!("END {cat}")), "{cat} missing:\n{s}");
        }
        let has = |l: &str| assert!(s.lines().any(|x| x.split_whitespace().collect::<Vec<_>>() == l.split_whitespace().collect::<Vec<_>>()), "no line {l:?} in:\n{s}");
        has("decimal_point \"<U002C>\"");
        has("thousands_sep \"<U002E>\"");
        has("grouping 3");
        has("int_curr_symbol \"EUR \"");
        has("currency_symbol \"<U20AC>\"");
        has("p_cs_precedes 0");
        has("p_sep_by_space 1");
        has("d_fmt \"<U0025>d<U002E><U0025>m<U002E><U0025>Y\"");
        has("t_fmt \"<U0025>T\"");
        has("d_t_fmt \"<U0025>a<U002C> <U0025>d<U002E> <U0025>b <U0025>Y<U002C> <U0025>T\"");
        has("week 7;19971130;4");
        has("first_weekday 2");
        has("first_workday 2");
        has("yesexpr \"<U005E><U005B><U002B>1jJyY<U005D>\"");
        has("height 297");
        has("measurement 1");
        has("country_ab3 \"DEU\"");
        has("country_num 276");
        has("lang_term \"deu\"");
        has("lang_lib \"ger\"");
        assert!(!s.contains("toupper"), "only Turkish and Azerbaijani change the case maps");
    }

    #[test]
    fn territory_data_picks_paper_measurement_and_week() {
        let s = source(&data(), None, "xx_US").unwrap().expect("xx_US is covered");
        for l in ["height 279", "width  216", "measurement 2", "first_weekday 1", "week       7;19971130;1", "t_fmt      \"<U0025>r\""] {
            assert!(s.lines().any(|x| x == l), "no line {l:?} in:\n{s}");
        }
        assert!(s.contains("int_curr_symbol   \"USD \""));
    }

    #[test]
    fn a_name_cldr_does_not_cover_is_not_generated() {
        assert!(source(&data(), None, "zz_ZZ").unwrap().is_none());
    }

    /// A probe dump: xx_DE (CLDR covers it) and zz_ZZ / tt_ZZ (it does not), with fingerprints
    /// where zz_ZZ behaves as en_US, tt_ZZ as tr_TR and yy_ZZ as neither.
    fn probe() -> Probe {
        let mut dump = String::from("# fixture\n");
        let mut row = |n: &str, c: &str, k: &str, v: &str| dump.push_str(&format!("{n}\t{c}\t{k}\t{v}\n"));
        for n in ["xx_DE.utf8", "zz_ZZ", "tt_ZZ", "yy_ZZ"] {
            row(n, "-", "charmap", "UTF-8");
            for (c, k, v) in [
                ("LC_NAME", "name_fmt", "\"%d%t%g\""),
                ("LC_NAME", "name_mr", "\"Herr\""),
                ("LC_ADDRESS", "postal_fmt", "\"%f%N%a\""),
                ("LC_ADDRESS", "country_num", "276"),
                ("LC_ADDRESS", "country_car", "\"D\""),
                ("LC_TELEPHONE", "tel_int_fmt", "\"+%c %a %l\""),
                ("LC_TELEPHONE", "int_prefix", "\"49\""),
                ("LC_IDENTIFICATION", "language", "\"Zed\""),
                ("LC_IDENTIFICATION", "territory", "\"Zedland\""),
                ("LC_NUMERIC", "decimal_point", "\",\""),
                ("LC_NUMERIC", "grouping", "3;3"),
                ("LC_MONETARY", "int_frac_digits", "2"),
                ("LC_TIME", "abday", "\"Su;Mo;Tu;We;Th;Fr;Sa\""),
                ("LC_TIME", "mon", "\"J\\xc3\\xa4n;F;M;A;M;J;J;A;S;O;N;D\""),
                ("LC_TIME", "era", ""),
                ("LC_TIME", "alt_digits", "\"o\";\"i\";\"ii\""),
                ("LC_TIME", "week-ndays", "7"),
                ("LC_TIME", "week-1stday", "19971130"),
                ("LC_TIME", "week-1stweek", "4"),
                ("LC_TIME", "first_weekday", "2"),
                ("LC_TIME", "first_workday", "2"),
                ("LC_TIME", "cal_direction", "1"),
                ("LC_MESSAGES", "yesexpr", "\"^[yY]\""),
                ("LC_PAPER", "height", "297"),
                ("LC_MEASUREMENT", "measurement", "2"),
            ] {
                row(n, c, k, v);
            }
        }
        for n in ["en_US.utf8", "tr_TR.utf8"] {
            row(n, "-", "charmap", "UTF-8");
        }
        let prints = "en_US.utf8 ctype.alpha=1 ctype.toupper=2 coll.latin=3 coll.greek=4\n\
            tr_TR.utf8 ctype.alpha=1 ctype.toupper=9 coll.latin=8 coll.greek=4\n\
            xx_DE.utf8 ctype.alpha=1 ctype.toupper=2 coll.latin=3 coll.greek=4\n\
            zz_ZZ ctype.alpha=1 ctype.toupper=2 coll.latin=3 coll.greek=4\n\
            tt_ZZ ctype.alpha=1 ctype.toupper=9 coll.latin=8 coll.greek=4\n\
            yy_ZZ ctype.alpha=5 ctype.toupper=2 coll.latin=3 coll.greek=7\n";
        // tt_ZZ: the Turkic case pairs; yy_ZZ: a class addition and a removal (a gap); xx_DE: an
        // addition in a name CLDR covers.
        let diff = "# diff\ntt_ZZ\tctype.toupper\t0069>0130\ntt_ZZ\tctype.tolower\t0049>0131\n\
            yy_ZZ\tctype.space\t+1361 -0020\nxx_DE.utf8\tctype.space\t+1361\n";
        Probe::parse(&dump, prints).unwrap().with_ctype_diff(diff).unwrap()
    }

    fn has_line(s: &str, l: &str) -> bool {
        s.lines().any(|x| x.split_whitespace().collect::<Vec<_>>() == l.split_whitespace().collect::<Vec<_>>())
    }

    #[test]
    fn every_category_after_collation_comes_from_the_probe() {
        let p = probe();
        let s = source(&data(), Some(&p), "xx_DE").unwrap().unwrap();
        for l in [
            "name_fmt \"<U0025>d<U0025>t<U0025>g\"",
            "name_mr \"Herr\"",
            "postal_fmt \"<U0025>f<U0025>N<U0025>a\"",
            "country_num 276",
            "country_car \"D\"",
            "int_prefix \"49\"",
            // #7890: the reference's value where CLDR's differs (CLDR: 3, ^[+1jJyY], 1).
            "grouping 3;3",
            "yesexpr \"<U005E><U005B>yY<U005D>\"",
            "measurement 2",
            "abday \"Su\";\"Mo\";\"Tu\";\"We\";\"Th\";\"Fr\";\"Sa\"",
            // #7892: LC_CTYPE takes the probed additions.
            "space <U1361>",
            "title \"German locale for Germany\"",
        ] {
            assert!(has_line(&s, l), "no line {l:?} in:\n{s}");
        }
        for cldr in ["country_ab2", "int_curr_symbol", "thousands_sep", "d_fmt", "noexpr", "width"] {
            assert!(!s.contains(cldr), "{cldr}: the probe printed none, so CLDR's must not appear:\n{s}");
        }
        assert!(!s.contains("toupper") && !s.contains("% gap"), "{s}");
        assert!(s.contains("\nEND LC_TELEPHONE\n\nLC_MEASUREMENT\nmeasurement 2\n"), "{s}");
        // An Azerbaijani name the probe lacks keeps SpecialCasing.txt's pairs.
        let mut d = data();
        let az = d.locales.get("xx_DE").cloned().unwrap();
        d.locales.insert("az_AZ".into(), az);
        let t = source(&d, Some(&p), "az_AZ").unwrap().unwrap();
        assert!(t.contains("toupper (<U0069>,<U0130>)") && t.contains("tolower (<U0049>,<U0131>)") && t.contains("thousands_sep"), "{t}");
    }

    #[test]
    fn a_name_cldr_lacks_is_generated_from_the_probe() {
        let p = probe();
        let s = probe_source(&p, "zz_ZZ").unwrap().unwrap();
        for l in [
            "title \"Zed locale for Zedland\"",
            "decimal_point \"<U002C>\"",
            "grouping 3;3",
            "int_frac_digits 2",
            "abday \"Su\";\"Mo\";\"Tu\";\"We\";\"Th\";\"Fr\";\"Sa\"",
            "mon \"J<U00E4>n\";\"F\";\"M\";\"A\";\"M\";\"J\";\"J\";\"A\";\"S\";\"O\";\"N\";\"D\"",
            "week 7;19971130;4",
            "alt_digits \"o\";\"i\";\"ii\"",
            "first_weekday 2",
            "yesexpr \"<U005E><U005B>yY<U005D>\"",
            "height 297",
            "name_mr \"Herr\"",
            "tel_int_fmt \"<U002B><U0025>c <U0025>a <U0025>l\"",
        ] {
            assert!(has_line(&s, l), "no line {l:?} in:\n{s}");
        }
        assert!(!s.contains("\nera") && !s.contains("toupper") && !s.contains("% gap"), "{s}");
        let t = probe_source(&p, "tt_ZZ").unwrap().unwrap();
        assert!(t.contains("toupper (<U0069>,<U0130>)\ntolower (<U0049>,<U0131>)\n") || t.contains("tolower (<U0049>,<U0131>)\ntoupper (<U0069>,<U0130>)\n"), "{t}");
        assert!(t.contains("% gap: LC_COLLATE differs from en_US's (root order) in latin;") && !t.contains("% gap: LC_CTYPE"), "{t}");
        let y = probe_source(&p, "yy_ZZ").unwrap().unwrap();
        assert!(y.contains("% gap: LC_CTYPE differs from en_US's in alpha, space (without U+0020);") && y.contains("in greek;") && !y.contains("toupper"), "{y}");
        assert!(y.contains("\ncopy \"i18n\"\nspace <U1361>\ntranslit_start\n"), "{y}");
        assert!(probe_source(&p, "qq_ZZ").unwrap().is_none());
    }

    #[test]
    fn a_list_with_the_wrong_count_or_a_bad_number_is_refused() {
        let base = "q_Q\t-\tcharmap\tUTF-8\nen_US.utf8\t-\tcharmap\tUTF-8\ntr_TR.utf8\t-\tcharmap\tUTF-8\n";
        let fp = "en_US.utf8 ctype.a=1\ntr_TR.utf8 ctype.a=1\nq_Q ctype.a=1\n";
        let p = Probe::parse(&format!("{base}q_Q\tLC_TIME\tabday\t\"a;b\"\n"), fp).unwrap();
        assert!(probe_source(&p, "q_Q").unwrap_err().contains("abday: 2 items"));
        let p = Probe::parse(&format!("{base}q_Q\tLC_NUMERIC\tgrouping\t3;x\n"), fp).unwrap();
        assert!(probe_source(&p, "q_Q").unwrap_err().contains("not a number list"));
    }

    #[test]
    fn allkeys_rows_parse_with_expansions_and_contractions() {
        let rows = parse_allkeys("@version 16.0.0\n0061 ; [.2075.0020.0002] # a\n00E6 ; [.2075.0020.0004][.0000.0110.0004][.20B9.0020.0004] # ae\n0061 0308 ; [.2076.0020.0002] # contraction\n002D ; [*020D.0020.0002] # hyphen\n").expect("parses");
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[1].1, vec![([0x2075, 0x20, 4], false), ([0, 0x110, 4], false), ([0x20B9, 0x20, 4], false)]);
        assert_eq!(rows[2].0, vec![0x61, 0x308]);
        assert_eq!(rows[3].1, vec![([0x20D, 0x20, 2], true)]);
        assert!(parse_allkeys("0061 [.1.2.3]\n").is_err());
    }

    #[test]
    fn variable_elements_and_the_marks_after_them_are_shifted_out_of_levels_1_to_3() {
        // UTS #10 section 4: `(` `1` `)` keeps only the digit; a mark after a variable element
        // goes with it, a mark after a letter stays.
        let (open, one, mark) = (([0x317, 0x20, 2], true), ([0x1F9A, 0x20, 2], false), ([0, 0x2F, 2], false));
        assert_eq!(shifted(&[open, one, open], 0), vec![one.0]);
        assert_eq!(shifted(&[open, mark, one, mark], 0), vec![one.0, mark.0]);
        assert!(shifted(&[open], 0).is_empty());
        // maxVariable: a symbol (not marked) at or below the bound is variable, above it is not.
        let plus = ([0x6DB, 0x20, 2], false);
        assert!(shifted(&[plus], 0x6DB).is_empty());
        assert_eq!(shifted(&[plus], 0x6DA), vec![plus.0]);
    }

    #[test]
    fn transliteration_drops_marks_and_spells_out_compatibility_forms() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the workspace");
        let comb = translit(root, false).expect("generates");
        let compat = translit(root, true).expect("generates");
        let has = |t: &str, l: &str| assert!(t.lines().any(|x| x == l), "no line {l:?}");
        has(&comb, "<U00E9> <U0065>"); // é: e
        has(&comb, "<U212B> <U0041>"); // ANGSTROM SIGN: A, through U+00C5
        has(&compat, "<UFB01> \"<U0066><U0069>\""); // ﬁ: fi
        has(&compat, "<U00B2> <U0032>");
        assert!(!comb.contains("<UFB01> "), "a compatibility mapping in the canonical table");
        assert!(!compat.lines().any(|l| l == "<U00E9> <U0065>"), "a canonical mapping repeated in the compatibility table");
        assert!(!comb.lines().any(|l| l.starts_with("<U0301> ")), "a mark is not transliterated");
    }

    #[test]
    fn the_root_table_shifts_punctuation_and_orders_han_after_every_listed_item() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the workspace");
        let t = iso14651_t1(root).expect("generates");
        // HYPHEN-MINUS is ignorable at levels 1 to 3; LATIN SMALL LETTER A is not.
        // HYPHEN-MINUS and PLUS SIGN (a symbol) are ignorable at levels 1 to 3; DOLLAR SIGN (a
        // currency symbol) and LATIN SMALL LETTER A are not.
        for c in ["<U002D>", "<U002B>"] {
            assert!(t.lines().any(|l| l == format!("{c} IGNORE;IGNORE;IGNORE;{c}")), "{c} not shifted");
        }
        let pos = |p: &str| t.lines().position(|l| l.starts_with(p)).unwrap_or_else(|| panic!("no line {p}"));
        for c in ["<U0024> ", "<U0061> "] {
            assert!(!t.lines().nth(pos(c)).is_some_and(|l| l.contains("IGNORE")), "{c} shifted");
        }
        // At the last level: SOFT HYPHEN (ignorable from the start) before the shifted ones, which
        // keep code point order (`+` before `,`).
        assert!(pos("<U00AD> ") < pos("<U0020> ") && pos("<U002B> ") < pos("<U002C> "));
        // The ideographs: Tangut, Nushu, Khitan, core Han, the other Han, after every listed item.
        let (tangut, nushu, khitan, han, ext_a) = (pos("<U00017000> .."), pos("<U0001B170> .."), pos("<U00018B00> .."), pos("<U4E00> .."), pos("<U3400> .."));
        assert!(pos("<U0061> ") < tangut && tangut < nushu && nushu < khitan && khitan < han && han < ext_a);
        assert_eq!(t.lines().nth(han + 1).map(|l| l.starts_with(".. ..;")), Some(true));
        assert!(!t.contains("UNDEFINED"), "unassigned code points are the undefined element");
    }
}
