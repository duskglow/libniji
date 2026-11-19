//! Type spelling: the scalar types, the DWARF spellings of layouts.tsv, typedef resolution, the words of a
//! typedefs.tsv type spelled for C, and member type sizes.
use super::*;

pub(super) const SCALARS: &[&str] = &[
    "char", "signed char", "unsigned char", "short", "unsigned short", "int", "unsigned int", "long", "unsigned long", "long long",
    "unsigned long long", "__int128", "unsigned __int128", "float", "double", "long double", "bool", "_Bool", "void",
];

pub(super) fn scalar(c: &str) -> bool {
    let base = c.trim_end_matches('*').trim().trim_start_matches("const ").trim();
    SCALARS.contains(&base)
}

/// The DWARF spelling of layouts.tsv (`long unsigned int`) in the C++ form of typedefs.tsv (`unsigned long`).
pub(super) fn dwarf_scalar(t: &str) -> Option<String> {
    let words: BTreeSet<&str> = t.split_whitespace().collect();
    let uns = words.contains("unsigned");
    let s = if words.contains("char") {
        if uns { "unsigned char" } else if words.contains("signed") { "signed char" } else { "char" }
    } else if words.contains("short") {
        if uns { "unsigned short" } else { "short" }
    } else if t.matches("long").count() == 2 {
        if uns { "unsigned long long" } else { "long long" }
    } else if words.contains("long") && !words.contains("double") {
        if uns { "unsigned long" } else { "long" }
    } else if words.contains("int") || words == BTreeSet::from(["unsigned"]) {
        if uns { "unsigned int" } else { "int" }
    } else {
        return SCALARS.contains(&t).then(|| t.to_string());
    };
    Some(s.to_string())
}

impl Gen<'_> {
    /// The C spelling of a typedef-resolvable type token; aggregates named for linkage keep their name.
    pub(super) fn resolve(&self, ty: &str) -> String {
        // C 7.8.2.3: wcstoimax takes wchar_t, which <inttypes.h> must not define in C: features.h's
        // __NIJI_WCHAR is the compiler's wchar_t type in C and wchar_t itself in C++.
        if ty == "__gwchar_t" {
            return "__NIJI_WCHAR".into();
        }
        match self.t.typedefs.get(&("typedef".to_string(), ty.to_string())) {
            Some(r) if scalar(&r["cxx"]) => r["cxx"].replace('*', " *"),
            _ => ty.to_string(),
        }
    }

    /// The size in bytes of a member type of layouts.tsv (`core`, its `stars` and `[n]` suffix), when known.
    pub(super) fn type_size(&self, core: &str, stars: &str, arr: &str) -> Option<u64> {
        let one = if !stars.is_empty() {
            8
        } else if let Some(r) = self.t.layouts.get(core).and_then(|r| r.iter().find(|r| r["member"].is_empty())) {
            r["offset_or_size"].parse().ok()?
        } else {
            self.scalar_size_align(core)?.0
        };
        let n: u64 = if arr.is_empty() { 1 } else { arr.trim_matches(['[', ']']).parse().ok()? };
        Some(one * n)
    }

    /// The alignment in bytes of a member type of layouts.tsv (`core` and its `stars`; an array's is its element's).
    pub(super) fn type_align(&self, core: &str, stars: &str) -> Option<u64> {
        if !stars.is_empty() {
            return Some(8);
        }
        match self.t.layouts.get(core).and_then(|r| r.iter().find(|r| r["member"].is_empty())) {
            Some(r) => r["align"].parse().ok(),
            None => Some(self.scalar_size_align(core)?.1),
        }
    }

    /// Size and alignment (psABI) of a scalar, an enumeration, or a typedef or tag with a typedefs.tsv row.
    fn scalar_size_align(&self, core: &str) -> Option<(u64, u64)> {
        let t = dwarf_scalar(core).unwrap_or_else(|| self.resolve(core));
        let n = match t.as_str() {
            "char" | "signed char" | "unsigned char" | "bool" | "_Bool" => 1,
            "short" | "unsigned short" => 2,
            "int" | "unsigned int" | "float" => 4,
            "long" | "unsigned long" | "long long" | "unsigned long long" | "double" => 8,
            "long double" | "__int128" | "unsigned __int128" => 16,
            _ if core.starts_with("enum ") => 4,
            _ => {
                let key = core.split_once(' ').filter(|(k, _)| matches!(*k, "struct" | "union")).map_or(("typedef".to_string(), core.to_string()), |(k, n)| (k.to_string(), n.to_string()));
                let r = self.t.typedefs.get(&key)?;
                return Some((r["size"].parse().ok()?, r["align"].parse().ok()?));
            }
        };
        Some((n, n))
    }

    /// Whether a word of a typedefs.tsv type text is spelled as it is in C: a keyword, a number, a typedef name.
    pub(super) fn c_plain(&self, w: &str) -> bool {
        w.starts_with(|d: char| d.is_ascii_digit())
            || SCALARS.iter().any(|s| s.split(' ').any(|x| x == w))
            || matches!(w, "const" | "volatile" | "signed" | "_Complex" | "restrict" | "__restrict" | "__va_list_tag")
            || w.starts_with("_Float")
            || self.t.typedefs.contains_key(&("typedef".to_string(), w.to_string()))
    }

    /// A `typedefs.tsv` C++ type text spelled for C, declaring nothing: a name that is neither a keyword nor a
    /// typedef is a tag in the C++ spelling (`sgttyb*`, `utimbuf const*`), of its typedefs.tsv kind, else a structure.
    pub(super) fn c_words(&self, text: &str) -> String {
        let mut out = String::new();
        let mut word = String::new();
        for c in text.chars().chain(std::iter::once(' ')) {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            if !word.is_empty() {
                let w = std::mem::take(&mut word);
                if !self.c_plain(&w) {
                    out.push_str(["union", "enum"].into_iter().find(|k| self.t.typedefs.contains_key(&(k.to_string(), w.clone()))).unwrap_or("struct"));
                    out.push(' ');
                }
                out.push_str(&w);
            }
            out.push(c);
        }
        out.trim_end().replace('*', " *").replace("  *", " *")
    }

    /// The words of a `typedefs.tsv` function type spelled for C: a structure tag gets its keyword and a typedef
    /// that is not a scalar is emitted first (`stat const*` is `struct stat const *`).
    pub(super) fn type_words(&mut self, text: &str) -> String {
        let mut out = String::new();
        let mut word = String::new();
        for c in text.chars().chain(std::iter::once(' ')) {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            if !word.is_empty() {
                let w = std::mem::take(&mut word);
                let typedef_known = self.t.typedefs.contains_key(&("typedef".to_string(), w.clone()));
                // A tag: a structure of layouts.tsv, the tag a typedef is spelled through, or a typedefs.tsv tag row.
                let kind = ["struct", "union"].into_iter().find(|k| self.t.typedefs.contains_key(&(k.to_string(), w.clone()))).unwrap_or("struct");
                let tag = format!("{kind} {w}");
                if matches!(w.as_str(), "const" | "volatile" | "void" | "int" | "long" | "short" | "char" | "unsigned" | "signed") {
                    out.push_str(&w);
                } else if !typedef_known && (self.t.layouts.contains_key(&tag) || self.tag_of_typedef(&w) || self.t.typedefs.contains_key(&(kind.to_string(), w.clone()))) {
                    // The tag is declared first: a tag first named in a parameter list is a different, unreachable type.
                    self.out.push(format!("{tag};"));
                    out.push_str(&tag);
                } else if typedef_known && !scalar(&self.resolve(&w)) && w != "__va_list_tag" {
                    self.typedef(&w);
                    out.push_str(&w);
                } else if !typedef_known && !self.c_plain(&w) && self.resolve(&w) == w {
                    // A name no table describes is a class in the C++ spelling (`td_thrhandle const*`): an
                    // incomplete structure, declared first.
                    self.out.push(format!("struct {w};"));
                    out.push_str(&format!("struct {w}"));
                } else {
                    out.push_str(&self.resolve(&w));
                }
            }
            out.push(c);
        }
        out.trim_end().replace('*', " *").replace("  *", " *").replace("( *)", "(*)").replace("__va_list_tag (*) [1]", "__builtin_va_list *")
    }
}
