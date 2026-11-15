//! Shape and cross-check of the type and value tables (Appendix C.13, plan/headers.md slice 3).
//!
//! `spec/abi/typedefs.tsv` gives the size, alignment and resolved C++ spelling of each typedef, tag, data object
//! and function; `spec/abi/macros.tsv` the C type and value of each integer, floating and string constant. Both
//! are written by `spec/tools/type_probe.py` on the reference system. The check (part of `cargo xtask static`)
//! fails on a wrong shape (columns, kinds, an unsorted or repeated row, a name the visibility table does not show
//! in that header with that kind, a size that is not a multiple of a power-of-two alignment, a value that does
//! not fit its type or is not in the probe's spelling) and on any disagreement with the older tables: a size or
//! alignment that differs from `layouts.tsv`, an object size that differs from `versions.tsv`, an integer value
//! that differs from `constants.tsv`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TYPEDEFS: &str = "spec/abi/typedefs.tsv";
const MACROS: &str = "spec/abi/macros.tsv";
const TYPEDEFS_HEAD: &str = "kind\tname\theader\tsize\talign\tcxx";
const MACROS_HEAD: &str = "name\theader\ttype\tvalue";
/// Kinds in the order the rows of one name are sorted.
const KINDS: [&str; 6] = ["typedef", "struct", "union", "enum", "object", "function"];
/// Integer types with their (min, max).
const INTS: [(&str, i128, i128); 14] = [
    ("_Bool", 0, 1),
    ("char", -128, 127),
    ("signed char", -128, 127),
    ("unsigned char", 0, 255),
    ("short", -32768, 32767),
    ("unsigned short", 0, 65535),
    ("int", i32::MIN as i128, i32::MAX as i128),
    ("unsigned int", 0, u32::MAX as i128),
    ("long", i64::MIN as i128, i64::MAX as i128),
    ("unsigned long", 0, u64::MAX as i128),
    ("long long", i64::MIN as i128, i64::MAX as i128),
    ("unsigned long long", 0, u64::MAX as i128),
    ("__int128", i64::MIN as i128, i64::MAX as i128),
    ("unsigned __int128", 0, u64::MAX as i128),
];
const FLOATS: [&str; 8] = ["float", "double", "long double", "_Float32", "_Float64", "_Float32x", "_Float64x", "_Float128"];

fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(f) if f == '_' || f.is_ascii_alphabetic()) && c.all(|x| x == '_' || x.is_ascii_alphanumeric())
}

/// The probe's spelling of a floating value: `inf`, `nan` or a normalised hexadecimal constant `0x1.<hex>p<exp>`
/// with no trailing zero digit (or `0x0p+0`), each optionally negative.
fn is_hexfloat(v: &str) -> bool {
    let v = v.strip_prefix('-').unwrap_or(v);
    if v == "inf" || v == "nan" || v == "0x0p+0" {
        return true;
    }
    let Some(rest) = v.strip_prefix("0x1") else { return false };
    let Some((frac, exp)) = rest.split_once('p') else { return false };
    let frac_ok = match frac.strip_prefix('.') {
        None => frac.is_empty(),
        Some(d) => !d.is_empty() && !d.ends_with('0') && d.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
    };
    let exp_ok = (exp.starts_with('+') || exp.starts_with('-')) && exp.len() > 1 && exp[1..].chars().all(|c| c.is_ascii_digit());
    frac_ok && exp_ok
}

/// The byte length of a quoted string value in the probe's spelling (`\"`, `\\`, `\ooo`), or `None`.
fn string_len(v: &str) -> Option<usize> {
    let body = v.strip_prefix('"')?.strip_suffix('"')?;
    let b = body.as_bytes();
    let (mut i, mut n) = (0, 0);
    while i < b.len() {
        match b[i] {
            b'\\' if b.get(i + 1).is_some_and(|c| *c == b'"' || *c == b'\\') => i += 2,
            b'\\' if b.len() >= i + 4 && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c)) => i += 4,
            b'"' | b'\\' => return None,
            c if (0x20..0x7f).contains(&c) => i += 1,
            _ => return None,
        }
        n += 1;
    }
    Some(n)
}

/// Data rows of a TSV, split, after checking its header line and field count.
fn rows<'a>(file: &str, text: &'a str, head: &str, errors: &mut Vec<String>) -> Vec<(usize, Vec<&'a str>)> {
    let mut lines = text.lines();
    if lines.next() != Some(head) {
        errors.push(format!("{file}: the first line must be `{}`", head.replace('\t', "<TAB>")));
    }
    let want = head.split('\t').count();
    let mut out = Vec::new();
    for (n, line) in lines.enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() == want {
            out.push((n + 2, f));
        } else {
            errors.push(format!("{file}:{}: {} fields, expected {want}", n + 2, f.len()));
        }
    }
    out
}

/// Every check over the table texts; the problems found.
pub fn check_tables(typedefs: &str, macros: &str, visibility: &str, layouts: &str, versions: &str, constants: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let shown: BTreeSet<(&str, &str, &str)> = visibility
        .lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() == 4).then(|| (f[0], f[1], f[2]))
        })
        .collect();
    let mut layout: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    for l in layouts.lines().skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() >= 4 && f[1].is_empty() {
            layout.insert(f[0], (f[2], f[3]));
        }
    }
    let mut object_sizes: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for l in versions.lines().skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() == 7 && f[0] == "libc.so.6" && f[4] == "OBJECT" && !f[6].is_empty() {
            object_sizes.entry(f[1]).or_default().insert(f[6]);
        }
    }
    let constant: BTreeMap<&str, &str> = constants.lines().skip(1).filter_map(|l| l.split_once('\t')).collect();

    let mut last: Option<(&str, usize)> = None;
    for (n, f) in rows(TYPEDEFS, typedefs, TYPEDEFS_HEAD, &mut errors) {
        let at = format!("{TYPEDEFS}:{n}");
        let (kind, name, header, size, align, cxx) = (f[0], f[1], f[2], f[3], f[4], f[5]);
        let Some(k) = KINDS.iter().position(|x| *x == kind) else {
            errors.push(format!("{at}: unknown kind `{kind}`"));
            continue;
        };
        if !is_ident(name) {
            errors.push(format!("{at}: `{name}` is not an identifier"));
        }
        if last.is_some_and(|p| p >= (name, k)) {
            errors.push(format!("{at}: `{name}` {kind} is out of order or repeated (rows sort by name, then kind)"));
        }
        last = Some((name, k));
        if !shown.contains(&(header, name, kind)) {
            errors.push(format!("{at}: visibility.tsv does not show `{name}` as a {kind} in `{header}`"));
        }
        if size.is_empty() != align.is_empty() || (kind == "function" && !size.is_empty()) {
            errors.push(format!("{at}: size and align are both given (never for a function) or both empty"));
        }
        if size.is_empty() && cxx.is_empty() {
            errors.push(format!("{at}: a row holds a size or a C++ spelling"));
        }
        if !size.is_empty() {
            match (size.parse::<u64>(), align.parse::<u64>()) {
                (Ok(s), Ok(a)) if a.is_power_of_two() && s % a == 0 => {}
                _ => errors.push(format!("{at}: size {size} and align {align}: not a multiple of a power of two")),
            }
            let label = if kind == "struct" || kind == "union" { format!("{kind} {name}") } else { name.to_string() };
            if let Some((ls, la)) = layout.get(label.as_str()).filter(|_| kind != "object") {
                if (*ls, *la) != (size, align) {
                    errors.push(format!("{at}: `{label}` is {size}/{align} here but {ls}/{la} in layouts.tsv"));
                }
            }
            if let Some(vs) = object_sizes.get(name).filter(|_| kind == "object") {
                if vs.iter().any(|v| *v != size) {
                    errors.push(format!("{at}: object `{name}` is {size} bytes here but {vs:?} in versions.tsv"));
                }
            }
        }
    }

    let mut last_macro: Option<&str> = None;
    for (n, f) in rows(MACROS, macros, MACROS_HEAD, &mut errors) {
        let at = format!("{MACROS}:{n}");
        let (name, header, ty, value) = (f[0], f[1], f[2], f[3]);
        if !is_ident(name) {
            errors.push(format!("{at}: `{name}` is not an identifier"));
        }
        if last_macro.is_some_and(|p| p >= name) {
            errors.push(format!("{at}: `{name}` is out of order or repeated"));
        }
        last_macro = Some(name);
        if !shown.contains(&(header, name, "macro")) && !shown.contains(&(header, name, "enumerator")) {
            errors.push(format!("{at}: visibility.tsv does not show `{name}` as a macro or enumerator in `{header}`"));
        }
        if let Some((_, lo, hi)) = INTS.iter().find(|(t, _, _)| *t == ty) {
            match value.parse::<i128>() {
                Ok(v) if (*lo..=*hi).contains(&v) => {
                    if let Some(c) = constant.get(name).and_then(|c| c.parse::<i128>().ok()) {
                        if c.rem_euclid(1 << 64) != v.rem_euclid(1 << 64) {
                            errors.push(format!("{at}: `{name}` is {value} here but {c} in constants.tsv"));
                        }
                    }
                }
                _ => errors.push(format!("{at}: `{value}` is not a value of type {ty}")),
            }
        } else if FLOATS.contains(&ty) {
            if !is_hexfloat(value) {
                errors.push(format!("{at}: `{value}` is not a normalised hexadecimal floating value"));
            }
        } else if let Some(len) = ty.strip_prefix("char[").and_then(|r| r.strip_suffix(']')).and_then(|r| r.parse::<usize>().ok()) {
            if string_len(value).is_none_or(|l| l + 1 != len) {
                errors.push(format!("{at}: `{value}` is not a quoted string of {} bytes", len.saturating_sub(1)));
            }
        } else {
            errors.push(format!("{at}: unknown type `{ty}`"));
        }
    }
    errors
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))
}

/// `cargo xtask static` entry.
pub fn check(root: &Path) -> Result<(), String> {
    let t = |n: &str| read(root, &format!("spec/abi/{n}"));
    let errors = check_tables(&read(root, TYPEDEFS)?, &read(root, MACROS)?, &t("visibility.tsv")?, &t("layouts.tsv")?, &t("versions.tsv")?, &t("constants.tsv")?);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("abi-types: {} problem(s):\n{}", errors.len(), errors.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    //! no spec clause: Appendix C.13's table format (SL-HEADERS slice 3); each test breaks one rule of the check.
    use super::*;

    const VIS: &str = "header\tname\tkind\tconfigs\n-\tlinux\tmacro\t*\nmath.h\tM_PI\tmacro\t*\nmath.h\tFP_NAN\tenumerator\t*\nstdio.h\tFILE\ttypedef\t*\nstdio.h\tfopen\tfunction\t*\nstdio.h\tstdin\tobject\t*\nstdio.h\tP_tmpdir\tmacro\t*\nstdint.h\tINT64_MAX\tmacro\t*\nsys/stat.h\tstat\tstruct\t*\n";
    const TD: &str = "kind\tname\theader\tsize\talign\tcxx\ntypedef\tFILE\tstdio.h\t216\t8\t_IO_FILE\nfunction\tfopen\tstdio.h\t\t\t_IO_FILE* (char const*, char const*)\nstruct\tstat\tsys/stat.h\t144\t8\tstat\nobject\tstdin\tstdio.h\t8\t8\t_IO_FILE*\n";
    const MAC: &str = "name\theader\ttype\tvalue\nFP_NAN\tmath.h\tint\t0\nINT64_MAX\tstdint.h\tlong\t9223372036854775807\nM_PI\tmath.h\tdouble\t0x1.921fb54442d18p+1\nP_tmpdir\tstdio.h\tchar[5]\t\"/tmp\"\nlinux\t-\tint\t1\n";
    const LAY: &str = "type\tmember\toffset_or_size\talign\tmember_type\tbitfield\nFILE\t\t216\t8\t\t\nFILE\t_flags\t0\t\tint\t\nstruct stat\t\t144\t8\t\t\n";
    const VER: &str = "library\tsymbol\tversion\tbinding\ttype\tscope\tsize\nlibc.so.6\tstdin\tGLIBC_2.2.5\tdefault\tOBJECT\tGLOBAL\t8\n";
    const CONST: &str = "macro\tvalue\nINT64_MAX\t9223372036854775807\nFP_NAN\t0\n";

    fn errs(td: &str, mac: &str, lay: &str, ver: &str, cons: &str) -> Vec<String> {
        check_tables(td, mac, VIS, lay, ver, cons)
    }

    fn fails_td(td: &str, needle: &str) {
        let e = errs(td, MAC, LAY, VER, CONST);
        assert!(e.iter().any(|x| x.contains(needle)), "expected `{needle}` in {e:?}");
    }

    fn fails_mac(mac: &str, needle: &str) {
        let e = errs(TD, mac, LAY, VER, CONST);
        assert!(e.iter().any(|x| x.contains(needle)), "expected `{needle}` in {e:?}");
    }

    #[test]
    fn good_tables_pass() {
        assert_eq!(errs(TD, MAC, LAY, VER, CONST), Vec::<String>::new());
    }

    #[test]
    fn typedefs_shape() {
        fails_td(&TD.replace("kind\tname", "sort\tname"), "first line");
        fails_td(&TD.replace("\t_IO_FILE\n", "\n").replacen("\t216\t8\n", "\t216\n", 1), "fields, expected 6");
        fails_td(&TD.replace("struct\tstat", "class\tstat"), "unknown kind");
        fails_td(&TD.replace("typedef\tFILE\tstdio.h", "typedef\tFILE\tstdlib.h"), "does not show `FILE`");
        fails_td(&TD.replace("object\tstdin", "typedef\tstdin"), "does not show `stdin`");
        fails_td(&TD.replace("\t\t\t_IO_FILE* (", "\t8\t8\t_IO_FILE* ("), "never for a function");
        fails_td(&TD.replace("216\t8\t_IO_FILE", "216\t\t_IO_FILE"), "both empty");
        fails_td(&TD.replace("144\t8\tstat", "\t\t"), "a size or a C++ spelling");
        fails_td(&TD.replace("216\t8\t", "216\t6\t"), "power of two");
        fails_td(&TD.replace("216\t8\t", "212\t8\t"), "power of two");
    }

    #[test]
    fn typedefs_order() {
        fails_td(&TD.replace("typedef\tFILE", "typedef\tg"), "out of order");
        fails_td(&TD.replace("typedef\tFILE", "struct\tFILE\tstdio.h\t\t\tx\ntypedef\tFILE"), "out of order");
        let swapped = TD.replace("struct\tstat\tsys/stat.h\t144\t8\tstat\nobject\tstdin\tstdio.h\t8\t8\t_IO_FILE*\n", "object\tstdin\tstdio.h\t8\t8\t_IO_FILE*\nstruct\tstat\tsys/stat.h\t144\t8\tstat\n");
        fails_td(&swapped, "out of order");
        fails_td(&format!("{TD}object\tstdin\tstdio.h\t8\t8\t_IO_FILE*\n"), "out of order");
    }

    #[test]
    fn typedefs_cross_check() {
        fails_td(&TD.replace("216\t8\t", "224\t8\t"), "in layouts.tsv");
        fails_td(&TD.replace("144\t8\t", "128\t8\t"), "`struct stat` is 128/8 here");
        fails_td(&TD.replace("144\t8\t", "144\t16\t"), "`struct stat` is 144/16 here");
        let e = errs(TD, MAC, LAY, &VER.replace("\t8\n", "\t16\n"), CONST);
        assert!(e.iter().any(|x| x.contains("in versions.tsv")), "{e:?}");
        // A typedef and a struct of the same name are looked up under different labels.
        assert_eq!(errs(TD, MAC, &LAY.replace("struct stat\t", "stat\t"), VER, CONST), Vec::<String>::new());
    }

    #[test]
    fn macros_shape() {
        fails_mac(&MAC.replace("name\theader", "macro\theader"), "first line");
        fails_mac(&MAC.replace("linux\t-", "linux\tstdio.h"), "does not show `linux`");
        fails_mac(&MAC.replace("FP_NAN\tmath.h", "FP_NAN\tstdio.h"), "does not show `FP_NAN`");
        fails_mac(&MAC.replace("linux\t-\tint\t1\n", "").replace("FP_NAN", "linux").replace("linux\tmath.h", "linux\t-"), "out of order");
        fails_mac(&format!("{MAC}linux\t-\tint\t1\n"), "out of order");
        fails_mac(&MAC.replace("\tint\t0", "\tbool\t0"), "unknown type");
        fails_mac(&MAC.replace("\tint\t1", "\tint\t2147483648"), "not a value of type int");
        fails_mac(&MAC.replace("\tint\t1", "\tunsigned int\t-1"), "not a value of type unsigned int");
        fails_mac(&MAC.replace("long\t9223372036854775807", "long\t9223372036854775806"), "in constants.tsv");
        fails_mac(&MAC.replace("p+1", "p1"), "hexadecimal floating");
        fails_mac(&MAC.replace("42d18p", "42d10p"), "hexadecimal floating");
        fails_mac(&MAC.replace("0x1.9", "0x3.9"), "hexadecimal floating");
        fails_mac(&MAC.replace("char[5]", "char[4]"), "quoted string");
        fails_mac(&MAC.replace("\"/tmp\"", "/tmp"), "quoted string");
        fails_mac(&MAC.replace("\"/tmp\"", "\"/t\"p\""), "quoted string");
    }

    #[test]
    fn value_spellings() {
        assert!(is_hexfloat("-inf") && is_hexfloat("nan") && is_hexfloat("0x1p-1074") && is_hexfloat("-0x0p+0"));
        assert!(!is_hexfloat("0x1.p+0") && !is_hexfloat("0x1.8p") && !is_hexfloat("0x1.8Ap+0") && !is_hexfloat("1.5"));
        assert_eq!(string_len("\"a\\\"b\\\\c\\012\""), Some(6));
        assert_eq!(string_len("\"\\01\""), None);
        // Unsigned values compare with constants.tsv modulo 2^64 (it holds them as long long).
        let mac = "name\theader\ttype\tvalue\nINT64_MAX\tstdint.h\tunsigned long\t18446744073709551615\n";
        assert_eq!(errs(TD, mac, LAY, VER, "macro\tvalue\nINT64_MAX\t-1\n"), Vec::<String>::new());
        assert!(!errs(TD, mac, LAY, VER, "macro\tvalue\nINT64_MAX\t-2\n").is_empty());
    }
}
