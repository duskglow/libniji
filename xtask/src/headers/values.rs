//! Values: the C literal of each `macros.tsv` row, typed (an integer constant keeps its type as a suffix, a floating
//! constant its type through the builtin or suffix), and the brace initializer of each `initializers.tsv` row.
use super::*;

/// The C literal of a `macros.tsv` value of the given type.
pub(crate) fn literal(ty: &str, value: &str) -> Result<String, String> {
    if ty.starts_with("char[") {
        return Ok(value.to_string());
    }
    // A floating constant: an infinity or a NaN has no literal, the compiler's builtin of the type gives it; a
    // hexadecimal value is written as it is, with the type's suffix (C99 6.4.4.2, exact in its type).
    if let Some((f, sfx)) = match ty {
        "float" => Some(("f", "f")),
        "double" => Some(("", "")),
        "long double" => Some(("l", "L")),
        _ => None,
    } {
        return match value {
            "inf" => Ok(format!("(__builtin_inf{f} ())")),
            "-inf" => Ok(format!("(-__builtin_inf{f} ())")),
            "nan" => Ok(format!("(__builtin_nan{f} (\"\"))")),
            v if v.trim_start_matches('-').starts_with("0x") && v.contains('p') => Ok(format!("({v}{sfx})")),
            v => Err(format!("no literal form for the {ty} value {v:?}")),
        };
    }
    // A pointer constant is its address as an integer: the template casts it to the pointer type it names.
    if ty == "pointer" {
        let v: i64 = value.parse().map_err(|_| format!("bad value {value:?}"))?;
        return Ok(if v < 0 { format!("(-{}L)", v.unsigned_abs()) } else { format!("{v}L") });
    }
    let suffix = match ty {
        "int" => "",
        "long" => "L",
        "long long" => "LL",
        "unsigned int" => "U",
        "unsigned long" => "UL",
        "unsigned long long" => "ULL",
        _ => return Err(format!("no literal form for type {ty:?}")),
    };
    if let Some(mag) = value.strip_prefix('-') {
        let mag: u128 = mag.parse().map_err(|_| format!("bad value {value:?}"))?;
        let min = match ty {
            "int" => 1u128 << 31,
            _ => 1u128 << 63,
        };
        // The most negative value has no literal: write it as (-(max) - 1).
        return Ok(if mag == min { format!("(-{}{suffix}-1)", mag - 1) } else { format!("(-{mag}{suffix})") });
    }
    value.parse::<u128>().map_err(|_| format!("bad value {value:?}"))?;
    Ok(format!("{value}{suffix}"))
}

impl Gen<'_> {
    pub(super) fn value(&mut self, name: &str) -> String {
        match self.t.macros.get(name).map(|(ty, v)| literal(ty, v)) {
            Some(Ok(s)) => s,
            Some(Err(e)) => {
                self.errors.push(format!("{}: {name}: {e}", self.header));
                String::new()
            }
            None => {
                self.errors.push(format!("{}: {name} has no macros.tsv value", self.header));
                String::new()
            }
        }
    }

    /// The size in bytes of a layouts.tsv member type (`T`, `T*`, `T[n]`), when the tables give it.
    fn size_of(&self, ty: &str) -> Option<usize> {
        if let Some(i) = ty.find('[') {
            let n: usize = ty[i + 1..ty.len() - 1].parse().ok()?;
            return Some(n * self.size_of(&ty[..i])?);
        }
        let core = ty.trim_end_matches(['*', ' ']);
        self.type_size(core, &ty[core.len()..], "").and_then(|n| usize::try_from(n).ok())
    }

    /// The brace initializer that gives an object of layouts.tsv type `ty` the bytes `b` (little-endian): a union by
    /// its first member, a structure by its members in table order, an array by its elements, a scalar (also a
    /// scalar typedef, whose layout is a size row only: `pthread_once_t`) by its value (negative when its type is
    /// signed and the top bit is set). None when the rows do not cover the type.
    fn init_of(&self, ty: &str, b: &[u8]) -> Option<String> {
        if let Some(i) = ty.find('[') {
            let esz = self.size_of(&ty[..i])?;
            let n: usize = ty[i + 1..ty.len() - 1].parse().ok()?;
            let parts: Option<Vec<String>> = (0..n).map(|k| self.init_of(&ty[..i], b.get(k * esz..(k + 1) * esz)?)).collect();
            return Some(format!("{{ {} }}", parts?.join(", ")));
        }
        if let Some(rows) = self.t.layouts.get(ty).or_else(|| self.cxx(ty).and_then(|c| self.t.layouts.get(&format!("struct {c}")))) {
            if rows.iter().any(|r| !r["member"].is_empty()) {
                return self.init_rows(rows, "", b);
            }
            if !scalar(&self.resolve(ty)) {
                return None;
            }
        }
        let sz = self.size_of(ty).filter(|n| (1..=8).contains(n))?;
        let v = b.get(..sz)?.iter().rev().fold(0u128, |a, x| a << 8 | u128::from(*x));
        if ty.ends_with('*') {
            return (v == 0).then(|| "0".to_string());
        }
        let c = dwarf_scalar(ty).unwrap_or_else(|| self.resolve(ty));
        let signed = scalar(&c) && !c.contains("unsigned") && c != "bool" && c != "_Bool";
        let top = 1u128 << (sz * 8 - 1);
        Some(if signed && v & top != 0 { format!("(-{} - 1)", (top << 1) - 1 - v) } else { v.to_string() })
    }

    /// The initializer of the members of a layout's `rows` directly below `prefix` (`""`, or `a.` for the members
    /// of member `a`); `b` is the whole object, the rows' offsets count from its start. A union (members at one
    /// offset) by its first member; a member of anonymous aggregate type (`__value` of `mbstate_t`) by its own rows.
    fn init_rows(&self, rows: &Rows, prefix: &str, b: &[u8]) -> Option<String> {
        let members: Vec<&BTreeMap<String, String>> = rows.iter().filter(|r| r["member"].strip_prefix(prefix).is_some_and(|m| !m.is_empty() && !m.contains('.'))).collect();
        if members.is_empty() || members.iter().any(|r| r["member"].ends_with("<anon>") || !r["bitfield"].is_empty()) {
            return None;
        }
        let union = members.len() > 1 && members.iter().all(|r| r["offset_or_size"] == members[0]["offset_or_size"]);
        let mut parts = Vec::new();
        for m in members.iter().take(if union { 1 } else { members.len() }) {
            let off: usize = m["offset_or_size"].parse().ok()?;
            let mt = &m["member_type"];
            if matches!(mt.as_str(), "struct <anon>" | "union <anon>") {
                parts.push(self.init_rows(rows, &format!("{}.", m["member"]), b)?);
                continue;
            }
            let sz = self.size_of(mt)?;
            parts.push(self.init_of(mt, b.get(off..off + sz)?)?);
        }
        Some(format!("{{ {} }}", parts.join(", ")))
    }

    /// `@@init(NAME)`: the brace initializer of the `initializers.tsv` row NAME, from the layout of its type.
    pub(super) fn init(&mut self, name: &str) -> String {
        let Some((ty, hex)) = self.t.inits.get(name).cloned() else {
            self.errors.push(format!("{}: {name} has no initializers.tsv row", self.header));
            return String::new();
        };
        let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()).collect();
        match self.init_of(&ty, &bytes) {
            Some(s) => s,
            None => {
                self.errors.push(format!("{}: {name}: the layout rows of {ty} do not give an initializer of its bytes", self.header));
                String::new()
            }
        }
    }
}
