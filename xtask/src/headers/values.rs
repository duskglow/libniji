//! Values: the C literal of each `macros.tsv` row, typed (an integer constant keeps its type as a suffix, a floating
//! constant its type through the builtin or suffix), and the brace initializer of each `initializers.tsv` row.
use super::*;

/// Whether a `macros.tsv` type is a pointer type (a pointer constant's row).
pub(crate) fn pointer_type(ty: &str) -> bool {
    ty.ends_with('*') || ty.contains("(*)")
}

/// The lines that define the `null` macros.tsv row `name` (a null pointer constant, value 0): `((void *) 0)` in C,
/// the compiler's `__null` in C++ (type `long`, as a pointer-sized null). Shared by the `@@null` marker and the rows test.
pub(crate) fn null_lines(name: &str, value: &str) -> Result<Vec<String>, String> {
    if value != "0" {
        return Err(format!("a null pointer constant has the value 0, not {value:?}"));
    }
    Ok(vec!["#  ifdef __cplusplus".into(), format!("#   define {name} __null"), "#  else".into(), format!("#   define {name} ((void *) 0)"), "#  endif".into()])
}

/// The floating types of macros.tsv: (type, builtin suffix of `__builtin_inf`/`__builtin_nan`, literal suffix, the
/// decimal digits that carry every value of the type exactly: C11 5.2.4.2.2's `*_DECIMAL_DIG` of binary32 (9), binary64
/// (17), the x87 80-bit format (21) and binary128 (36)). The `_FloatN` suffixes are TS 18661-3's (C2X 6.4.4.2), which
/// the reference toolchain's gcc and g++ take in every language mode (black-box probe, HEADERS-TABLES-COMPLETE-2).
pub(crate) const FLOATS: &[(&str, &str, &str, usize)] = &[
    ("float", "f", "f", 9),
    ("double", "", "", 17),
    ("long double", "l", "L", 21),
    ("_Float32", "f32", "f32", 9),
    ("_Float64", "f64", "f64", 17),
    ("_Float32x", "f32x", "f32x", 17),
    ("_Float64x", "f64x", "f64x", 21),
    ("_Float128", "f128", "f128", 36),
];

/// A standard type of a `_FloatN` type's format: its name, literal suffix and builtin suffix.
type FloatBase = (&'static str, &'static str, &'static str);

/// The `_FloatN` types and constant suffixes of TS 18661-3 a compiler may lack (clang has none of them), with the type
/// of the same format on x86-64 (psABI) and that type's literal and builtin suffixes; `None`: `_Float128`, whose
/// format (binary128) no other standard type has.
const FLOATN_BASE: &[(&str, &str, Option<FloatBase>)] = &[
    ("_Float32", "f32", Some(("float", "f", "f"))),
    ("_Float64", "f64", Some(("double", "", ""))),
    ("_Float32x", "f32x", Some(("double", "", ""))),
    ("_Float64x", "f64x", Some(("long double", "L", "l"))),
    ("_Float128", "f128", None),
];

/// What a generated line needs of the `_FloatN` types: `Some(true)` when it names `_Float128` or its suffix,
/// `Some(false)` when it names only the others (a type, `1.5f32`, `__builtin_inff64x`), `None` when it names none;
/// and the line spelled for a compiler without the keywords (`_FloatN` as its base type in a declaration, each
/// suffix as the base type's).
fn floatn_line(line: &str, decl: bool) -> (Option<bool>, String, bool) {
    let (mut found, mut suffixed, mut out, mut word) = (None, false, String::new(), String::new());
    for c in line.chars().chain(std::iter::once('\0')) {
        if c.is_ascii_alphanumeric() || c == '_' {
            word.push(c);
            continue;
        }
        let w = std::mem::take(&mut word);
        let mut rep = w.clone();
        for &(ty, sfx, base) in FLOATN_BASE {
            let lit = w.starts_with(|d: char| d.is_ascii_digit()) && !w.starts_with("0x") && !w.starts_with("0X") && w.ends_with(sfx);
            // A suffix match must end the word exactly: `1f32x` is no `f32` literal.
            let lit = lit && w[..w.len() - sfx.len()].ends_with(|d: char| d.is_ascii_digit() || d == 'e' || d == '.');
            let builtin = w.starts_with("__builtin_") && w.ends_with(sfx) && !w.ends_with(&format!("{sfx}x"));
            let lit = lit && !w.ends_with(&format!("{sfx}x"));
            if w == ty || lit || builtin {
                found = Some(found.unwrap_or(false) || base.is_none());
                if let Some((bty, lsfx, bsfx)) = base {
                    if w == ty && decl {
                        rep = bty.to_string();
                    } else if lit {
                        rep = format!("{}{lsfx}", &w[..w.len() - sfx.len()]);
                        suffixed = true;
                    } else if builtin {
                        rep = format!("{}{bsfx}", &w[..w.len() - sfx.len()]);
                        suffixed = true;
                    }
                }
            }
        }
        out.push_str(&rep);
        if c != '\0' {
            out.push(c);
        }
    }
    (found, out, suffixed)
}

/// Each generated declaration and macro definition that names a `_FloatN` type or spells its constants, under the
/// compiler's support (features.h's `__NIJI_FLOATN`, `__NIJI_FLOAT128`): `_Float128`'s only where the compiler has
/// the type; the others' where it has the keywords, else a declaration in C with the base types and a macro with
/// the base types' suffixes (its casts name the typedefs features.h gives such a compiler). A macro whose only
/// `_FloatN` words are casts needs neither.
pub(crate) fn floatn_guards(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    for l in lines {
        let decl = l.starts_with("extern ");
        let def = l.strip_prefix('#').is_some_and(|r| r.trim_start().starts_with("define "));
        if !(decl || def) {
            out.push(l);
            continue;
        }
        let (found, base, suffixed) = floatn_line(&l, decl);
        let hash = if def { l[..l.find("define").unwrap_or(1)].to_string() } else { "#  ".to_string() };
        match found {
            Some(true) => out.extend([format!("{hash}if __NIJI_FLOAT128"), l, format!("{hash}endif")]),
            Some(false) if decl => out.extend([format!("{hash}if __NIJI_FLOATN"), l, format!("{hash}elif !defined __cplusplus"), base, format!("{hash}endif")]),
            Some(false) if suffixed => out.extend([format!("{hash}if __NIJI_FLOATN"), l, format!("{hash}else"), base, format!("{hash}endif")]),
            _ => out.push(l),
        }
    }
    out
}

/// The decimal spelling (`d.ddde+N`) of the hexadecimal floating constant `hex` (`[-]0xH[.H]p[+-]N`), rounded to
/// `digits` significant digits, half to even. A hexadecimal floating constant is no token in C89 and in C++98 to C++14
/// (C99 6.4.4.2, C++17 [lex.fcon]); the decimal one is a token in every mode, and with the type's `*_DECIMAL_DIG`
/// digits the compiler reads back the same value (C11 5.2.4.2.2).
pub(crate) fn decimal(hex: &str, digits: usize) -> Result<String, String> {
    let bad = || format!("bad hexadecimal floating constant {hex:?}");
    let (neg, h) = match hex.strip_prefix('-') {
        Some(h) => (true, h),
        None => (false, hex),
    };
    let (mant, exp) = h.strip_prefix("0x").and_then(|h| h.split_once('p')).ok_or_else(bad)?;
    let (ip, fp) = mant.split_once('.').unwrap_or((mant, ""));
    let m = u128::from_str_radix(&format!("{ip}{fp}"), 16).map_err(|_| bad())?;
    let e = exp.parse::<i64>().map_err(|_| bad())? - 4 * fp.len() as i64;
    if digits == 0 || m == 0 {
        return if m == 0 { Ok(format!("{}0.0e+0", if neg { "-" } else { "" })) } else { Err(bad()) };
    }
    // The exact value is N * 10^dexp with N = m * 2^e (e >= 0) or m * 5^-e (e < 0): N in base-1e9 limbs, low first.
    const BASE: u64 = 1_000_000_000;
    let mut n: Vec<u64> = Vec::new();
    let mut x = m;
    while x > 0 {
        n.push((x % BASE as u128) as u64);
        x /= BASE as u128;
    }
    let (mut left, step, unit, dexp) = if e >= 0 { (e, 30, 2u64, 0) } else { (-e, 13, 5u64, e) };
    while left > 0 {
        let k = left.min(step);
        left -= k;
        let mul = unit.pow(k as u32);
        let mut carry = 0u64;
        for l in n.iter_mut() {
            let v = *l * mul + carry;
            *l = v % BASE;
            carry = v / BASE;
        }
        while carry > 0 {
            n.push(carry % BASE);
            carry /= BASE;
        }
    }
    let mut s = n.iter().rev().enumerate().map(|(i, l)| if i == 0 { l.to_string() } else { format!("{l:09}") }).collect::<String>().into_bytes();
    let mut exp10 = s.len() as i64 - 1 + dexp;
    if s.len() > digits {
        let rest = &s[digits..];
        let half = rest[0].cmp(&b'5').then_with(|| if rest[1..].iter().any(|&d| d != b'0') { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Equal });
        let odd = (s[digits - 1] - b'0') % 2 == 1;
        s.truncate(digits);
        if half == std::cmp::Ordering::Greater || (half == std::cmp::Ordering::Equal && odd) {
            let mut i = digits;
            loop {
                if i == 0 {
                    s.insert(0, b'1');
                    s.truncate(digits);
                    exp10 += 1;
                    break;
                }
                i -= 1;
                if s[i] == b'9' {
                    s[i] = b'0';
                } else {
                    s[i] += 1;
                    break;
                }
            }
        }
    }
    while s.len() > 1 && s.last() == Some(&b'0') {
        s.pop();
    }
    let s = String::from_utf8(s).map_err(|_| bad())?;
    let frac = if s.len() > 1 { &s[1..] } else { "0" };
    Ok(format!("{}{}.{frac}e{}{}", if neg { "-" } else { "" }, &s[..1], if exp10 < 0 { "-" } else { "+" }, exp10.abs()))
}

/// The C literal of a `macros.tsv` value of the given type.
pub(crate) fn literal(ty: &str, value: &str) -> Result<String, String> {
    if ty.starts_with("char[") {
        return Ok(value.to_string());
    }
    // A floating constant: an infinity or a NaN (quiet `nan`, signaling `snan`) has no literal, the compiler's builtin
    // of the type gives it; a finite
    // value is its decimal spelling with the type's suffix (exact in its type, a token in every language mode).
    if let Some(&(_, f, sfx, digits)) = FLOATS.iter().find(|r| r.0 == ty) {
        return match value {
            "inf" => Ok(format!("(__builtin_inf{f} ())")),
            "-inf" => Ok(format!("(-__builtin_inf{f} ())")),
            "nan" => Ok(format!("(__builtin_nan{f} (\"\"))")),
            "snan" => Ok(format!("(__builtin_nans{f} (\"\"))")),
            v if v.trim_start_matches('-').starts_with("0x") && v.contains('p') => Ok(format!("({}{sfx})", decimal(v, digits)?)),
            v => Err(format!("no literal form for the {ty} value {v:?}")),
        };
    }
    // A pointer constant is its address as an integer: the template casts it to the pointer type it names.
    if ty == "pointer" {
        let v: i64 = value.parse().map_err(|_| format!("bad value {value:?}"))?;
        return Ok(if v < 0 { format!("(-{}L)", v.unsigned_abs()) } else { format!("{v}L") });
    }
    // A character constant (HEADERS-CXX-ENUM-TYPES): type `char` in C++, `int` in C (C11 6.4.4.4, C++17 [lex.ccon]),
    // as the reference's; printable characters as themselves, the others as an octal escape.
    if ty == "char" {
        // `char` is signed on x86-64 (psABI): a byte above 0x7f has a negative value (`'\377'` is -1).
        let v: i16 = value.parse().ok().filter(|v| (-128..=255).contains(v)).ok_or(format!("bad char value {value:?}"))?;
        let v = v as u8;
        return Ok(match v {
            b'\'' | b'\\' => format!("'\\{}'", v as char),
            0x20..=0x7e => format!("'{}'", v as char),
            _ => format!("'\\{v:03o}'"),
        });
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
    /// The (type, value) of macro `name` in this header: its macro-values.tsv row here, else its macros.tsv row.
    pub(super) fn macro_row(&self, name: &str) -> Option<&(String, String)> {
        self.t.header_values.get(&(self.header.clone(), name.to_string())).or_else(|| self.t.macros.get(name))
    }

    pub(super) fn value(&mut self, name: &str) -> String {
        // A pointer constant (macros.tsv type `void (*)(int)`, `fenv_t const*`: type_probe's C++ spelling): its address
        // cast to the type, spelled for C (`__locale_struct*` is `struct __locale_struct *`).
        if let Some((ty, v)) = self.macro_row(name).filter(|(ty, _)| pointer_type(ty)) {
            return match literal("pointer", v) {
                Ok(l) => format!("(({}) {l})", self.c_words(ty)),
                Err(e) => {
                    self.errors.push(format!("{}: {name}: {e}", self.header));
                    String::new()
                }
            };
        }
        match self.macro_row(name).map(|(ty, v)| literal(ty, v)) {
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
