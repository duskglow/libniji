//! The probed locale data (LOCALE-PROBE-GEN, D-31.27; owner ruling #7791): what the reference
//! system's compiled locales report through its `locale -k` (a black box), read from
//! `spec/locale-probe/locales.tsv` (written by `tools/probe/locale_dump.py`), and the LC_CTYPE /
//! LC_COLLATE behaviour fingerprints of `spec/locale-probe/fingerprints.tsv`
//! (`tools/probe/locale_fingerprint.c`). The generator takes `LC_NAME`, `LC_ADDRESS` and
//! `LC_TELEPHONE` of every locale from here (Unicode CLDR has no source for them), and every
//! category of the names CLDR does not cover.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// One probed locale: its codeset, the UTF-8 name whose rows it shares and its own rows.
#[derive(Default)]
struct Loc {
    charmap: String,
    same_as: Option<String>,
    rows: BTreeMap<(String, String), Vec<u8>>,
}

/// A keyword's value as `locale -k` printed it.
#[derive(Debug, PartialEq, Eq)]
pub enum Val {
    /// A string (printed in double quotes), decoded to text.
    Str(String),
    /// A number or a byte list (`3;3`, `-1`), as printed.
    Bare(String),
}

pub struct Probe {
    locs: BTreeMap<String, Loc>,
    /// Per probed name, `key` -> fingerprint (`ctype.alpha`, `coll.latin`, ...).
    prints: BTreeMap<String, BTreeMap<String, String>>,
    /// Per probed name, `ctype.<item>` -> where it differs from en_US.utf8's
    /// (`spec/locale-probe/ctype-diff.tsv`).
    ctype: BTreeMap<String, BTreeMap<String, CtypeDiff>>,
    /// The transliteration table (`spec/locale-probe/translit.tsv`): code point -> its
    /// alternatives in the order they are tried.
    translit: BTreeMap<u32, Vec<Vec<u32>>>,
    /// Per probed legacy-codeset name, its own entries for the code points its strings hold:
    /// code point -> the result in its codeset, None for none.
    own_translit: BTreeMap<String, Vec<(u32, Option<Vec<u32>>)>>,
}

/// One LC_CTYPE item of a probed name where it differs from en_US.utf8's: the code points it
/// adds to / removes from a class, or its own mappings of a case map.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CtypeDiff {
    pub added: Vec<u32>,
    pub removed: Vec<u32>,
    pub mapped: Vec<(u32, u32)>,
}

fn parse_ctype_diff(text: &str) -> Result<BTreeMap<String, BTreeMap<String, CtypeDiff>>, String> {
    let mut out: BTreeMap<String, BTreeMap<String, CtypeDiff>> = BTreeMap::new();
    let hex = |s: &str| u32::from_str_radix(s, 16).ok().filter(|&c| c <= 0x10_ffff);
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let bad = || format!("locale-probe ctype-diff: bad row {line:?}");
        let [name, item, val] = line.split('\t').collect::<Vec<_>>()[..] else { return Err(bad()) };
        let d = out.entry(name.to_string()).or_default().entry(item.to_string()).or_default();
        for v in val.split(' ') {
            match (v.split_once('>'), v.strip_prefix('+'), v.strip_prefix('-')) {
                (Some((a, b)), ..) => d.mapped.push((hex(a).ok_or_else(bad)?, hex(b).ok_or_else(bad)?)),
                (_, Some(c), _) => d.added.push(hex(c).ok_or_else(bad)?),
                (_, _, Some(c)) => d.removed.push(hex(c).ok_or_else(bad)?),
                _ => return Err(bad()),
            }
        }
        // A case map has only pairs, a class only + / - code points.
        let is_map = matches!(item, "ctype.toupper" | "ctype.tolower");
        let class_ops = !d.added.is_empty() || !d.removed.is_empty();
        if is_map == class_ops || is_map == d.mapped.is_empty() {
            return Err(bad());
        }
    }
    Ok(out)
}

fn unescape(v: &str) -> Result<Vec<u8>, String> {
    let b = v.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            let hex = v.get(i + 2..i + 4).filter(|_| b.get(i + 1) == Some(&b'x')).ok_or_else(|| format!("bad escape in {v:?}"))?;
            out.push(u8::from_str_radix(hex, 16).map_err(|e| format!("{v:?}: {e}"))?);
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Ok(out)
}

/// Bytes of a single-byte codeset whose characters 0xA0..0xFF are Latin-1's (ISO-8859-15 with
/// its eight replacements), else only ASCII.
fn decode(bytes: &[u8], charmap: &str) -> Result<String, String> {
    if charmap == "UTF-8" {
        return String::from_utf8(bytes.to_vec()).map_err(|e| format!("not UTF-8: {e}"));
    }
    bytes
        .iter()
        .map(|&b| match (charmap, b) {
            (_, 0..=0x7f) => Ok(char::from(b)),
            ("ISO-8859-1", _) => Ok(char::from(b)),
            ("ISO-8859-15", 0xa4) => Ok('\u{20ac}'),
            ("ISO-8859-15", 0xa6) => Ok('\u{160}'),
            ("ISO-8859-15", 0xa8) => Ok('\u{161}'),
            ("ISO-8859-15", 0xb4) => Ok('\u{17d}'),
            ("ISO-8859-15", 0xb8) => Ok('\u{17e}'),
            ("ISO-8859-15", 0xbc) => Ok('\u{152}'),
            ("ISO-8859-15", 0xbd) => Ok('\u{153}'),
            ("ISO-8859-15", 0xbe) => Ok('\u{178}'),
            ("ISO-8859-15", _) => Ok(char::from(b)),
            _ => Err(format!("byte {b:#x} of codeset {charmap} is not decoded here")),
        })
        .collect()
}

impl Probe {
    pub fn parse(dump: &str, prints: &str) -> Result<Probe, String> {
        let mut locs: BTreeMap<String, Loc> = BTreeMap::new();
        for line in dump.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
            let f: Vec<&str> = line.split('\t').collect();
            let [name, cat, kw, val] = f[..] else { return Err(format!("locale-probe: bad row {line:?}")) };
            let l = locs.entry(name.to_string()).or_default();
            match (cat, kw) {
                ("-", "charmap") => l.charmap = val.to_string(),
                ("-", "same-as") => l.same_as = Some(val.to_string()),
                _ => {
                    l.rows.insert((cat.to_string(), kw.to_string()), unescape(val)?);
                }
            }
        }
        let prints = prints
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .filter_map(|l| {
                let mut f = l.split(' ');
                let name = f.next()?.to_string();
                Some((name, f.filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), v.to_string())).collect()))
            })
            .collect();
        Ok(Probe { locs, prints, ctype: BTreeMap::new(), translit: BTreeMap::new(), own_translit: BTreeMap::new() })
    }

    /// Adds the transliteration table (`spec/locale-probe/translit.tsv`, tools/probe/translit_table.py):
    /// `<cp>\t<alt>;<alt>...` (each alternative space-separated hex code points) and
    /// `@<name>\t<cp>\t<result or ->`.
    pub fn with_translit(mut self, text: &str) -> Result<Probe, String> {
        for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
            let bad = || format!("locale-probe translit: bad row {line:?}");
            let hex = |s: &str| u32::from_str_radix(s, 16).ok().filter(|&c| c <= 0x10_ffff && !(0xd800..0xe000).contains(&c));
            let string = |s: &str| s.split(' ').map(hex).collect::<Option<Vec<u32>>>().filter(|v| !v.is_empty());
            match line.split('\t').collect::<Vec<_>>()[..] {
                [name, cp, res] if name.starts_with('@') => {
                    let res = if res == "-" { None } else { Some(string(res).ok_or_else(bad)?) };
                    self.own_translit.entry(name[1..].to_string()).or_default().push((hex(cp).ok_or_else(bad)?, res));
                }
                [cp, alts] => {
                    let alts = alts.split(';').map(string).collect::<Option<Vec<_>>>().ok_or_else(bad)?;
                    if self.translit.insert(hex(cp).ok_or_else(bad)?, alts).is_some() {
                        return Err(format!("locale-probe translit: {cp} twice"));
                    }
                }
                _ => return Err(bad()),
            }
        }
        Ok(self)
    }

    /// The shared transliteration table, code point order.
    pub fn translit(&self) -> &BTreeMap<u32, Vec<Vec<u32>>> {
        &self.translit
    }

    /// The own transliteration entries of the source `src` (`ll_CC[@modifier]`): those of every
    /// probed legacy-codeset name compiled from it (`ll_CC`, `ll_CC.<codeset>`, with the same
    /// modifier), merged per code point: the results in name order, then the shared table's
    /// alternatives; an empty list where the names give none (a code point the locale does not
    /// transliterate). Err when one name gives none and another a result: no single entry says both.
    pub fn own_translit(&self, src: &str) -> Result<BTreeMap<u32, Vec<Vec<u32>>>, String> {
        let (base, m) = src.split_once('@').map_or((src, ""), |(b, m)| (b, m));
        let mut out: BTreeMap<u32, (bool, Vec<Vec<u32>>)> = BTreeMap::new();
        for (name, rows) in &self.own_translit {
            let (nb, nm) = name.split_once('@').map_or((name.as_str(), ""), |(b, m)| (b, m));
            if nm != m || nb.split('.').next() != Some(base) {
                continue;
            }
            for (cp, res) in rows {
                let e = out.entry(*cp).or_insert((res.is_none(), Vec::new()));
                if e.0 != res.is_none() {
                    return Err(format!("locale-probe translit: {src} U+{cp:04X}: one name gives none, another a result"));
                }
                if let Some(r) = res.as_ref().filter(|r| !e.1.contains(r)) {
                    e.1.push(r.clone());
                }
            }
        }
        Ok(out
            .into_iter()
            .map(|(cp, (none, mut alts))| {
                if !none {
                    alts.extend(self.translit.get(&cp).into_iter().flatten().filter(|a| !alts.contains(a)).cloned().collect::<Vec<_>>());
                }
                (cp, alts)
            })
            .collect())
    }

    /// `text`, written in the single-byte `charmap` (one [`decode`] knows) through the
    /// transliteration table as localedef does (probed, LOCALEDEF-TRANSLIT): a character the
    /// codeset lacks takes `name`'s own entry, else the shared one: its first alternative the
    /// codeset encodes whole; a character with none empties the whole string. None when the
    /// codeset is not one [`decode`] knows.
    fn in_codeset(&self, name: &str, text: &str, charmap: &str) -> Option<String> {
        let table: BTreeMap<char, u8> = (0..=255u8).filter_map(|b| decode(&[b], charmap).ok().and_then(|s| s.chars().next()).map(|c| (c, b))).collect();
        if table.len() < 256 {
            return None;
        }
        let own: BTreeMap<u32, Option<&Vec<u32>>> = self.own_translit.get(name).into_iter().flatten().map(|(c, r)| (*c, r.as_ref())).collect();
        let mut out = String::new();
        for ch in text.chars() {
            if table.contains_key(&ch) {
                out.push(ch);
                continue;
            }
            let alts: Vec<&Vec<u32>> = match own.get(&(ch as u32)) {
                Some(r) => r.iter().copied().collect(),
                None => self.translit.get(&(ch as u32)).into_iter().flatten().collect(),
            };
            let fit = alts.into_iter().find(|a| a.iter().all(|&c| char::from_u32(c).is_some_and(|c| table.contains_key(&c))));
            match fit {
                Some(a) => out.extend(a.iter().filter_map(|&c| char::from_u32(c))),
                None => return Some(String::new()),
            }
        }
        Some(out)
    }

    /// Adds the LC_CTYPE differences (`spec/locale-probe/ctype-diff.tsv`'s rows).
    pub fn with_ctype_diff(mut self, text: &str) -> Result<Probe, String> {
        self.ctype = parse_ctype_diff(text)?;
        Ok(self)
    }

    pub fn load(root: &Path) -> Result<Probe, String> {
        let rd = |p: &str| fs::read_to_string(root.join(p)).map_err(|e| format!("{p}: {e}"));
        Probe::parse(&rd("spec/locale-probe/locales.tsv")?, &rd("spec/locale-probe/fingerprints.tsv")?)?
            .with_ctype_diff(&rd("spec/locale-probe/ctype-diff.tsv")?)?
            .with_translit(&rd("spec/locale-probe/translit.tsv")?)
    }

    /// Where the probed name's LC_CTYPE differs from en_US.utf8's, per item (`ctype.space`,
    /// `ctype.toupper`, ...); empty when it behaves the same.
    pub fn ctype_diff(&self, name: &str) -> Option<&BTreeMap<String, CtypeDiff>> {
        self.ctype.get(name)
    }

    /// The probed name for a source name (`ll_CC[@modifier]`): its UTF-8 locale (`ll_CC.utf8`,
    /// `ll_CC.utf8@mod`, or the name itself when its codeset is UTF-8), else the name itself.
    pub fn resolve(&self, src: &str) -> Option<&str> {
        let (base, m) = src.split_once('@').map_or((src, String::new()), |(b, m)| (b, format!("@{m}")));
        let utf8 = |n: &str| self.locs.get_key_value(n).filter(|(_, l)| l.charmap == "UTF-8").map(|(k, _)| k.as_str());
        utf8(&format!("{base}.utf8{m}")).or_else(|| utf8(src)).or_else(|| self.locs.get_key_value(src).map(|(k, _)| k.as_str()))
    }

    /// The value `locale -k` printed for `cat`'s `kw` under the probed name `name`; None when it
    /// printed none.
    pub fn get(&self, name: &str, cat: &str, kw: &str) -> Result<Option<Val>, String> {
        let Some(l) = self.locs.get(name) else { return Ok(None) };
        let key = (cat.to_string(), kw.to_string());
        let (raw, charmap) = match (l.rows.get(&key), &l.same_as) {
            (Some(r), _) => (r, l.charmap.as_str()),
            (None, Some(s)) => match self.locs.get(s).and_then(|o| o.rows.get(&key)) {
                Some(r) => (r, "UTF-8"),
                None => return Ok(None),
            },
            (None, None) => return Ok(None),
        };
        if raw.as_slice() == b"<absent>" {
            return Ok(None);
        }
        let mut text = decode(raw, charmap).map_err(|e| format!("{name} {cat} {kw}: {e}"))?;
        // A legacy-codeset name's own value that is its UTF-8 sibling's written in the codeset
        // through the transliteration table (U+202F as U+00A0 in ISO-8859-15) stands for the
        // sibling's text: the source holds that, and the wide form keeps it.
        if let Some(sib) = l.same_as.as_ref().and_then(|s| self.locs.get(s)).and_then(|o| o.rows.get(&key)).filter(|_| charmap != "UTF-8") {
            let inner = |t: &str| t.strip_prefix('"').and_then(|t| t.strip_suffix('"')).map(str::to_string);
            if let (Ok(sib), Some(own)) = (decode(sib, "UTF-8"), inner(&text)) {
                if sib != text && inner(&sib).and_then(|s| self.in_codeset(name, &s, charmap)) == Some(own) {
                    text = sib;
                }
            }
        }
        Ok(Some(match text.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
            Some(s) if text.len() >= 2 => Val::Str(s.to_string()),
            _ => Val::Bare(text),
        }))
    }

    /// A string keyword's text ("" when absent or not a string).
    pub fn text(&self, name: &str, cat: &str, kw: &str) -> Result<String, String> {
        Ok(match self.get(name, cat, kw)? {
            Some(Val::Str(s)) => s,
            _ => String::new(),
        })
    }

    /// The behaviour fingerprints of a probed name.
    pub fn prints(&self, name: &str) -> Option<&BTreeMap<String, String>> {
        self.prints.get(name)
    }
}

#[cfg(test)]
mod tests {
    //! The dump reader. Trace: no spec clause (generator tooling; the compiled values are checked
    //! against the probe by crates/locale/tests/reference.rs, s31_8_c_1).
    use super::*;

    const DUMP: &str = "# fixture\n\
        xx_DE\t-\tcharmap\tISO-8859-15\nxx_DE\t-\tsame-as\txx_DE.utf8\n\
        xx_DE\tLC_ADDRESS\tcountry_name\t\"\\xa4\\x5c\"\nxx_DE\tLC_NAME\tname_mr\t<absent>\n\
        xx_DE.utf8\t-\tcharmap\tUTF-8\nxx_DE.utf8\tLC_ADDRESS\tcountry_name\t\"D\\xc3\\xa4\"\n\
        xx_DE.utf8\tLC_ADDRESS\tcountry_num\t276\nxx_DE.utf8\tLC_NAME\tname_mr\t\"Herr\"\n\
        yy_ZZ\t-\tcharmap\tUTF-8\nyy_ZZ\tLC_TIME\tera\t\n";
    const PRINTS: &str = "# fp\nxx_DE.utf8 ctype.alpha=01 coll.latin=02\n";

    #[test]
    fn rows_resolve_through_the_utf8_sibling_and_decode_their_codeset() {
        let p = Probe::parse(DUMP, PRINTS).unwrap();
        assert_eq!(p.resolve("xx_DE"), Some("xx_DE.utf8"));
        assert_eq!(p.resolve("yy_ZZ"), Some("yy_ZZ"));
        assert_eq!(p.resolve("zz_ZZ"), None);
        assert_eq!(p.get("xx_DE", "LC_ADDRESS", "country_name").unwrap(), Some(Val::Str("\u{20ac}\\".into())));
        assert_eq!(p.get("xx_DE", "LC_ADDRESS", "country_num").unwrap(), Some(Val::Bare("276".into())));
        assert_eq!(p.get("xx_DE", "LC_NAME", "name_mr").unwrap(), None);
        assert_eq!(p.text("xx_DE.utf8", "LC_ADDRESS", "country_name").unwrap(), "D\u{e4}");
        assert_eq!(p.get("yy_ZZ", "LC_TIME", "era").unwrap(), Some(Val::Bare(String::new())));
        assert_eq!(p.prints("xx_DE.utf8").and_then(|m| m.get("coll.latin")).map(String::as_str), Some("02"));
    }

    #[test]
    fn ctype_differences_parse_into_class_edits_and_case_pairs() {
        let p = Probe::parse(DUMP, PRINTS).unwrap().with_ctype_diff("# d\nxx_DE.utf8\tctype.space\t+1361 -0020\nxx_DE.utf8\tctype.toupper\t0069>0130\n").unwrap();
        let d = p.ctype_diff("xx_DE.utf8").unwrap();
        assert_eq!(d["ctype.space"], CtypeDiff { added: vec![0x1361], removed: vec![0x20], mapped: vec![] });
        assert_eq!(d["ctype.toupper"].mapped, vec![(0x69, 0x130)]);
        assert!(p.ctype_diff("yy_ZZ").is_none());
        for bad in ["a\tctype.space\t0069>0130\n", "a\tctype.toupper\t+0069\n", "a\tctype.space\t\n", "a\tctype.space\t+110000\n", "a\tctype.space\n"] {
            assert!(Probe::parse(DUMP, PRINTS).unwrap().with_ctype_diff(bad).is_err(), "{bad:?}");
        }
    }

    /// A legacy name's own value that is its UTF-8 sibling's text through the table (U+202F as
    /// U+00A0, `’` as `'`, a character without an entry emptying the string) reads as the
    /// sibling's text; one the table does not give stays.
    #[test]
    fn a_legacy_value_the_table_explains_is_the_siblings_text() {
        let dump = "# f\nxx_DE\t-\tcharmap\tISO-8859-15\nxx_DE\t-\tsame-as\txx_DE.utf8\n\
            xx_DE\tLC_NUMERIC\tthousands_sep\t\"\\xa0\"\nxx_DE\tLC_TIME\td_fmt\t\"d'a\"\nxx_DE\tLC_TIME\tt_fmt\t\"\"\n\
            xx_DE\tLC_ADDRESS\tcountry_name\t\"\\xa0\"\n\
            xx_DE.utf8\t-\tcharmap\tUTF-8\nxx_DE.utf8\tLC_NUMERIC\tthousands_sep\t\"\\xe2\\x80\\xaf\"\n\
            xx_DE.utf8\tLC_TIME\td_fmt\t\"d\\xe2\\x80\\x99a\"\nxx_DE.utf8\tLC_TIME\tt_fmt\t\"\\xe2\\x82\\xbd\"\n\
            xx_DE.utf8\tLC_ADDRESS\tcountry_name\t\"\\xe2\\x80\\x99\"\n";
        let p = Probe::parse(dump, PRINTS).unwrap();
        assert_eq!(p.text("xx_DE", "LC_NUMERIC", "thousands_sep").unwrap(), "\u{a0}");
        let p = p.with_translit("# t\n202F\t00A0;0020\n2019\t0027\n").unwrap();
        assert_eq!(p.text("xx_DE", "LC_NUMERIC", "thousands_sep").unwrap(), "\u{202f}");
        assert_eq!(p.text("xx_DE", "LC_TIME", "d_fmt").unwrap(), "d\u{2019}a");
        assert_eq!(p.text("xx_DE", "LC_TIME", "t_fmt").unwrap(), "\u{20bd}");
        assert_eq!(p.text("xx_DE", "LC_ADDRESS", "country_name").unwrap(), "\u{a0}");
    }

    /// A source's own entries merge those of its legacy names (same modifier) in name order, then
    /// the shared alternatives; a name giving none where another gives a result is refused, as
    /// is a malformed row.
    #[test]
    fn own_entries_merge_per_source_and_refuse_a_contradiction() {
        let p = Probe::parse(DUMP, PRINTS)
            .unwrap()
            .with_translit("# t\n021B\t0074\n@ro_RO\t021B\t0163\n@ro_RO.iso88592\t021B\t0074\n@ro_RO@x\t2019\t-\n@uz_UZ\t02BB\t0027\n")
            .unwrap();
        assert_eq!(p.translit()[&0x21b], vec![vec![0x74]]);
        assert_eq!(p.own_translit("ro_RO").unwrap(), BTreeMap::from([(0x21b, vec![vec![0x163], vec![0x74]])]));
        assert_eq!(p.own_translit("ro_RO@x").unwrap(), BTreeMap::from([(0x2019, vec![])]));
        assert!(p.own_translit("en_US").unwrap().is_empty());
        assert!(p.with_translit("@ro_RO.koi8r\t021B\t-\n").unwrap().own_translit("ro_RO").is_err());
        for bad in ["20AC\tzz\n", "20AC\n", "D800\t0041\n", "@x\t20AC\n", "20AC\t0041;\n", "20AC\t0041\n20AC\t0042\n", "@x\t20AC\t\n"] {
            assert!(Probe::parse(DUMP, PRINTS).unwrap().with_translit(bad).is_err(), "{bad:?}");
        }
    }
}
