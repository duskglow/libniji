//! The placement-independent rows test of the generator (HEADERS-GEN-COMPLETE; plan/headers.md section 2.4,
//! "Generated tests"): `cargo xtask headers-gen --rows DIR`. Every row of the tables goes through the emitter modules
//! into translation units in no header placement (no condition, no include structure), and is checked there:
//!
//! * declarations: every `typedefs.tsv` row (typedef, struct, union, enum, object, function), every aggregate of
//!   `layouts.tsv`, every `prototypes.tsv` function and every enumerator of `visibility.tsv`, through [`Gen::item`],
//!   the dispatcher `@@emit` uses; a generator error is a failure of that row;
//! * static assertions, compiled as C (gnu17) and as C++ (gnu++17): sizes, alignments and member offsets of
//!   `layouts.tsv` and (C) the types of its scalar, typedef and structure members; typedef sizes and alignments and
//!   (C++) typedef identity with the `cxx` spelling (vector typedefs in C and C++); object sizes and (C++) types, and the sizes of `versions.tsv`
//!   data objects; function types (C and C++: the `prototypes.tsv` declaration redeclared, with `noexcept.tsv` in
//!   C++; C++: `decltype` against the `typedefs.tsv` function type); every `macros.tsv` value and type (`_Generic` in
//!   C, `decltype` in C++; strings and floating values in C++ constant expressions); every `constants.tsv` value;
//!   enumerator values;
//! * a run-time program: each bit-field set to all ones covers exactly the bits its layouts.tsv row records (`bits
//!   W@P`: from bit P of its storage unit, as the extraction read them from DWARF), each `initializers.tsv` initializer gives exactly its bytes;
//! * call sites: the configurations of `ftm-configs.tsv` grouped into families with the same feature levels (all the
//!   emitted conditions can see), one unit per family, in that family's language and flags, taking the address of
//!   every function with a `callsites.tsv` `addr` row there; its undefined symbols (`nm -u`) are exactly the table's;
//! * `visibility.tsv`: each name of a declared kind has the table row its emitter reads.
//!
//! The expected values are spelled here, from the tables, not with the emitter's helpers (`literal`, `dwarf_scalar`),
//! so a wrong emitter is caught. A failing row is named by table and name. `headers/rows-exceptions.tsv` (`table`,
//! `name`, `lang` = `c`, `c++` or `*`, `reason`) lists the reviewed exceptions: the row is left out of that language's
//! units (`*`: of every unit, and its generator errors are not counted).
use super::*;
use std::process::Command;

pub(super) const EXCEPTIONS: &str = "headers/rows-exceptions.tsv";

/// The C spelling of the DWARF base type names of layouts.tsv, written out here (not through the emitter's own
/// `dwarf_scalar`) so a member type the emitter spells wrong is caught.
const DWARF: &[(&str, &str)] = &[
    ("char", "char"), ("signed char", "signed char"), ("unsigned char", "unsigned char"), ("short int", "short"),
    ("short unsigned int", "unsigned short"), ("int", "int"), ("unsigned int", "unsigned int"), ("long int", "long"),
    ("long unsigned int", "unsigned long"), ("long long int", "long long"), ("long long unsigned int", "unsigned long long"),
    ("float", "float"), ("double", "double"), ("long double", "long double"), ("_Bool", "_Bool"), ("__int128", "__int128"),
    ("__int128 unsigned", "unsigned __int128"), ("void", "void"),
];

/// The integer types of macros.tsv.
const INTS: &[&str] = &["int", "unsigned int", "long", "unsigned long", "long long", "unsigned long long"];

/// The largest finite value of each floating type's format (IEEE 754 binary32, binary64 and binary128, and the x87
/// 80-bit format the x86-64 psABI gives `long double` and `_Float64x`), without the type's suffix: a value above it is
/// an infinity.
const FMAX: &[(&str, &str)] = &[
    ("float", "3.40282346638528859811704183484516925e+38"),
    ("double", "1.79769313486231570814527423731704357e+308"),
    ("long double", "1.18973149535723176502126385303097021e+4932"),
    ("_Float32", "3.40282346638528859811704183484516925e+38"),
    ("_Float64", "1.79769313486231570814527423731704357e+308"),
    ("_Float32x", "1.79769313486231570814527423731704357e+308"),
    ("_Float64x", "1.18973149535723176502126385303097021e+4932"),
    ("_Float128", "1.18973149535723176508575932662800702e+4932"),
];

/// One translation unit: its lines and, for each, the row it belongs to.
#[derive(Default, Clone)]
struct Tu {
    lines: Vec<String>,
    owner: Vec<Option<usize>>,
}

impl Tu {
    fn put(&mut self, row: Option<usize>, lines: impl IntoIterator<Item = String>) {
        for l in lines {
            self.lines.push(l);
            self.owner.push(row);
        }
    }

    fn add(&mut self, other: &Tu) {
        self.lines.extend(other.lines.iter().cloned());
        self.owner.extend(other.owner.iter().copied());
    }

    fn text(&self) -> String {
        self.lines.join("\n") + "\n"
    }
}

/// The rows, their exceptions and what failed.
#[derive(Default)]
struct Check {
    rows: Vec<(String, String)>,
    index: BTreeMap<(String, String), usize>,
    /// (table, name) -> language the exception covers (`c`, `c++` or `*`).
    except: BTreeMap<(String, String), String>,
    failed: BTreeMap<usize, Vec<String>>,
    /// Failures no row owns (an error in a shared line, an unexpected symbol).
    unit: Vec<String>,
    /// Names with no table row of their own that templates define (function-like and C.9 macros, `#error` headers).
    template_owned: usize,
}

impl Check {
    fn row(&mut self, table: &str, name: &str) -> usize {
        let k = (table.to_string(), name.to_string());
        if let Some(&i) = self.index.get(&k) {
            return i;
        }
        self.rows.push(k.clone());
        self.index.insert(k, self.rows.len() - 1);
        self.rows.len() - 1
    }

    fn lang(&self, i: usize) -> Option<&str> {
        self.except.get(&self.rows[i]).map(String::as_str)
    }

    /// The lines of row `i`, left out of the languages its exception covers.
    fn put(&self, tu: &mut Tu, i: usize, lines: Vec<String>) {
        let wrap = match self.lang(i) {
            Some("*") => return,
            Some("c++") => Some("#ifndef __cplusplus"),
            Some("c") => Some("#ifdef __cplusplus"),
            _ => None,
        };
        if lines.is_empty() {
            return;
        }
        if let Some(w) = wrap {
            tu.put(Some(i), [w.to_string()]);
        }
        tu.put(Some(i), lines);
        if wrap.is_some() {
            tu.put(Some(i), ["#endif".to_string()]);
        }
    }

    fn fail(&mut self, i: usize, msg: String) {
        if self.lang(i) != Some("*") {
            self.failed.entry(i).or_default().push(msg);
        }
    }
}

/// An integer of the tables as an `__int128` constant expression, spelled here (not by the emitter's `literal`).
fn int128(v: &str) -> Option<String> {
    let (neg, mag) = v.strip_prefix('-').map_or((false, v), |m| (true, m));
    let m: u128 = mag.parse().ok()?;
    if m > u64::MAX as u128 {
        return None;
    }
    Some(if neg { format!("(-(__int128) {m}ULL)") } else { format!("((__int128) {m}ULL)") })
}

/// A C++ function type with `noexcept` on the function itself: after its own parameter list, which for a function
/// returning a function pointer (`void (*(int, void (*)(int)))(int)`) is the inner one.
fn with_noexcept(cxx: &str) -> String {
    if let Some(p) = cxx.find("(*(") {
        let start = p + 2;
        let mut depth = 0;
        for (k, ch) in cxx[start..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        let e = start + k + 1;
                        return format!("{} noexcept{}", &cxx[..e], &cxx[e..]);
                    }
                }
                _ => {}
            }
        }
    }
    format!("{cxx} noexcept")
}

/// A typedefs.tsv C++ type text as C++ source (spelling only): a name that is neither a keyword nor a typedef is a
/// class in that text and is written elaborated, with its typedefs.tsv kind (C++ finds a plain `stat` as the
/// function, `struct stat` as the class); the compiler's `va_list` (`__va_list_tag [1]`, decayed `__va_list_tag*`)
/// by its builtin name.
fn cxx_type(t: &Tables, text: &str) -> String {
    let text = text.replace("__va_list_tag (*) [1]", "__builtin_va_list*").replace("__va_list_tag [1]", "__builtin_va_list").replace("__va_list_tag*", "__builtin_va_list");
    let (mut out, mut word, mut prev) = (String::new(), String::new(), String::new());
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            let w = std::mem::take(&mut word);
            let plain = w.starts_with(|d: char| d.is_ascii_digit())
                || SCALARS.iter().any(|s| s.split(' ').any(|x| x == w))
                || matches!(w.as_str(), "const" | "volatile" | "signed" | "noexcept" | "struct" | "union" | "enum" | "_Complex" | "__builtin_va_list" | "wchar_t" | "char8_t" | "char16_t" | "char32_t")
                || w.starts_with("_Float")
                || matches!(prev.as_str(), "struct" | "union" | "enum")
                || t.typedefs.contains_key(&("typedef".to_string(), w.clone()));
            // Elaborated only where a plain name does not find the class: a function or object of the same name hides
            // it (`stat`), or no table declares it (`td_thrhandle`; the elaborated name declares it).
            let tag_row = ["struct", "union", "enum"].into_iter().any(|k| t.typedefs.contains_key(&(k.to_string(), w.clone())));
            let hidden = t.protos.contains_key(&w) || ["function", "object"].into_iter().any(|k| t.typedefs.contains_key(&(k.to_string(), w.clone())));
            let known = tag_row || t.layouts.contains_key(&format!("struct {w}")) || t.typedefs.values().any(|r| r["cxx"] == w);
            if !plain && (hidden || !known) {
                out.push_str(["union", "enum"].into_iter().find(|k| t.typedefs.contains_key(&(k.to_string(), w.clone()))).unwrap_or("struct"));
                out.push(' ');
            }
            out.push_str(&w);
            prev = w;
        }
        out.push(c);
    }
    out.trim_end().to_string()
}

/// The `__builtin_offsetof` designator of a layouts.tsv member path: `a.b`, with `[0]` after an array of aggregates.
fn designator(rows: &Rows, path: &str) -> Option<String> {
    if path.contains("<anon>") {
        return None;
    }
    let segs: Vec<&str> = path.split('.').collect();
    let (mut out, mut cur) = (String::new(), String::new());
    for (k, s) in segs.iter().enumerate() {
        if k > 0 {
            out.push('.');
            cur.push('.');
        }
        out.push_str(s);
        cur.push_str(s);
        if k + 1 < segs.len() && rows.iter().any(|r| r["member"] == cur && r["member_type"].contains('[')) {
            out.push_str("[0]");
        }
    }
    Some(out)
}

/// The C type a layouts.tsv member type names, when the tables spell it: a DWARF base type, a typedef of
/// typedefs.tsv or a described structure or union, with its `const`, pointers and array bounds.
fn c_type(t: &Tables, mt: &str) -> Option<String> {
    let (base, arr) = mt.find('[').map_or((mt, ""), |i| (&mt[..i], &mt[i..]));
    let core = base.trim_end_matches(['*', ' ']);
    let stars = base[core.len()..].matches('*').count();
    let (konst, core) = core.strip_prefix("const ").map_or(("", core), |c| ("const ", c));
    let ty = if let Some((_, c)) = DWARF.iter().find(|(d, _)| *d == core) {
        c.to_string()
    } else if t.typedefs.contains_key(&("typedef".to_string(), core.to_string())) {
        core.to_string()
    } else if let Some((kw, tag)) = core.split_once(' ').filter(|(kw, tag)| matches!(*kw, "struct" | "union") && !tag.contains(' ') && !tag.starts_with('<')) {
        if !t.layouts.contains_key(core) && !t.typedefs.contains_key(&(kw.to_string(), tag.to_string())) {
            return None;
        }
        core.to_string()
    } else {
        return None;
    };
    Some(format!("{konst}{ty}{}{arr}", " *".repeat(stars)))
}

/// What the rows test builds before compiling: the units and the rows.
struct Built {
    check: Check,
    /// `rows.c`: compiled as C and as C++, static assertions only.
    asserts: Tu,
    /// `rows-run.c`: the run-time checks.
    run: Tu,
    /// One unit per configuration family: (configuration ids, language, flags, unit, expected symbol per row).
    families: Vec<Family>,
}

/// A configuration family's call-site unit: (configuration ids, language, flags, unit, expected symbol per row).
type Family = (Vec<String>, String, String, Tu, BTreeMap<usize, String>);

/// Build the units for every row (`only`: just the rows of these names, for a unit test on synthetic rows).
fn build(root: &Path, t: &Tables, except: BTreeMap<(String, String), String>, only: Option<&BTreeSet<String>>) -> Result<Built, String> {
    let mut c = Check { except, ..Check::default() };
    let wanted = |n: &str| only.is_none_or(|o| o.contains(n));
    // A function, object, macro or enumerator that the reserved-name policy leaves out (`omitted`: C 7.1.3, owner
    // rulings R-H2 and R-H3) is in no header; reserved typedefs and tags still are, where a declaration needs them.
    let named = |n: &str| wanted(n) && !omitted(&t.reserved, n);
    let mut g = Gen { root, t, header: "rows".into(), out: Vec::new(), errors: Vec::new(), emitted: BTreeSet::new(), covered: BTreeMap::new(), owned_macros: BTreeSet::new(), hidden_members: BTreeMap::new() };
    let mut decls = Tu::default();
    let item = |c: &mut Check, g: &mut Gen, decls: &mut Tu, i: usize, name: &str, kind: &str| {
        if c.lang(i) == Some("*") {
            return;
        }
        g.item(name, kind);
        let lines = std::mem::take(&mut g.out);
        for e in std::mem::take(&mut g.errors) {
            c.fail(i, format!("generator: {e}"));
        }
        c.put(decls, i, lines);
    };
    // Tags, typedefs, objects, functions, enumerators: the order `@@emit` gives them.
    let kinds = |k: &str| t.typedefs.keys().filter(|(kk, n)| kk == k && wanted(n)).map(|(_, n)| n.clone()).collect::<Vec<_>>();
    for kind in ["struct", "union", "enum"] {
        for n in kinds(kind) {
            let i = c.row("typedefs", &format!("{kind} {n}"));
            item(&mut c, &mut g, &mut decls, i, &n, kind);
        }
    }
    for key in t.layouts.keys().filter(|k| wanted(k.split_once(' ').map_or(k.as_str(), |(_, n)| n))) {
        let i = c.row("layouts", key);
        match key.split_once(' ').filter(|(kw, _)| matches!(*kw, "struct" | "union")) {
            Some((kw, n)) if !t.typedefs.contains_key(&(kw.to_string(), n.to_string())) => item(&mut c, &mut g, &mut decls, i, n, kw),
            Some(_) => {}
            None if !t.typedefs.contains_key(&("typedef".to_string(), key.clone())) => item(&mut c, &mut g, &mut decls, i, key, "typedef"),
            None => {}
        }
    }
    for n in kinds("typedef") {
        let i = c.row("typedefs", &format!("typedef {n}"));
        item(&mut c, &mut g, &mut decls, i, &n, "typedef");
    }
    for n in kinds("object").into_iter().filter(|n| named(n)) {
        let i = c.row("typedefs", &format!("object {n}"));
        item(&mut c, &mut g, &mut decls, i, &n, "object");
    }
    let mut functions: BTreeSet<String> = t.protos.keys().filter(|n| named(n)).cloned().collect();
    functions.extend(kinds("function").into_iter().filter(|n| named(n)));
    for n in &functions {
        let i = if t.protos.contains_key(n) { c.row("prototypes", n) } else { c.row("typedefs", &format!("function {n}")) };
        item(&mut c, &mut g, &mut decls, i, n, "function");
    }
    let enumerators: BTreeSet<String> = t.vis.iter().filter(|r| r["kind"] == "enumerator" && named(&r["name"])).map(|r| r["name"].clone()).collect();
    for n in &enumerators {
        let i = c.row("visibility", &format!("enumerator {n}"));
        item(&mut c, &mut g, &mut decls, i, n, "enumerator");
    }

    // The prototypes redeclared from the table's own text: a declaration the emitter spelled differently is a
    // conflicting redeclaration (C) or a different exception specification (C++17).
    let mut redecl = Tu::default();
    for (n, d) in t.protos.iter().filter(|(n, _)| named(n)) {
        let i = c.row("prototypes", n);
        let nx = if t.nothrow.contains(n) { " NIJI_NX" } else { "" };
        // Spelling equivalences only: the decayed `va_list` and `complex`, the macro of <complex.h> for `_Complex`,
        // C's `_Bool` (C++ `bool`, through NIJI_BOOL on these lines only: an emitter `_Bool` in C++ still fails).
        let d = d.replace("__va_list_tag *", "__builtin_va_list").replace("__va_list_tag*", "__builtin_va_list").replace("complex ", "_Complex ").replace("_Bool ", "NIJI_BOOL ");
        c.put(&mut redecl, i, vec![format!("{d}{nx};")]);
    }

    let mut a = Tu::default();
    // Aggregates of layouts.tsv.
    for (key, rows) in t.layouts.iter().filter(|(k, _)| wanted(k.split_once(' ').map_or(k.as_str(), |(_, n)| n))) {
        let i = c.row("layouts", key);
        let mut lines = Vec::new();
        if let Some(s) = rows.iter().find(|r| r["member"].is_empty()) {
            lines.push(format!("NIJI_A (sizeof ({key}) == {} && __alignof__ ({key}) == {}, \"layouts {key}\");", s["offset_or_size"], s["align"]));
        }
        // The rows of an array typedef (`jmp_buf`, one `struct __jmp_buf_tag`) describe its element.
        let subj = if t.typedefs.get(&("typedef".to_string(), key.clone())).is_some_and(|r| r["cxx"].ends_with(']')) { format!("__typeof__ ((({key} *) 0)[0][0])") } else { key.clone() };
        // A transparent union is a GNU C type: in C++ the name is its first member's pointer type (Gen::pointer_form;
        // the function types that take it check that form), so its members are checked in C.
        let tu = super::aggregates::transparent_union(rows.iter().find(|r| r["member"].is_empty()));
        if tu {
            // Transparent: a parameter of it takes a `void *` argument, which no ordinary union parameter does.
            lines.push("#ifndef __cplusplus".into());
            lines.push(format!("void niji_tu_{i} ({key}); __attribute__ ((__unused__)) static void niji_tu_call_{i} (void *p) {{ niji_tu_{i} (p); }}"));
        }
        for r in rows.iter().filter(|r| !r["member"].is_empty() && r["bitfield"].is_empty()) {
            let Some(d) = designator(rows, &r["member"]) else { continue };
            lines.push(format!("NIJI_A (__builtin_offsetof ({subj}, {d}) == {}, \"layouts {key} {d}\");", r["offset_or_size"]));
            if let Some(ty) = c_type(t, &r["member_type"]) {
                lines.push(format!("NIJI_C (__builtin_types_compatible_p (__typeof__ ((({subj} *) 0)->{d}), {ty}), \"layouts {key} {d} type\");"));
            }
        }
        if tu {
            // C++: the first member's pointer type, `__restrict` where the head row says `restrict` (a top-level
            // qualifier is no part of a function type, so only an object of it shows it).
            let head = rows.iter().find(|r| r["member"].is_empty()).map_or("", |r| r["bitfield"].as_str());
            let first = rows.iter().find(|r| !r["member"].is_empty()).map_or("", |r| r["member_type"].trim_end());
            let ty = if head.split_whitespace().any(|w| w == "restrict") { format!("{first}__restrict") } else { first.to_string() };
            lines.push("#else".into());
            lines.push(format!("extern {key} niji_tux_{i}; NIJI_A (__is_same (decltype (&niji_tux_{i}), {ty} *), \"layouts {key} pointer form\");"));
            lines.push("#endif".into());
        }
        c.put(&mut a, i, lines);
    }
    // typedefs.tsv.
    for ((kind, n), r) in t.typedefs.iter().filter(|((k, n), _)| if matches!(k.as_str(), "object" | "function") { named(n) } else { wanted(n) }) {
        let i = c.row("typedefs", &format!("{kind} {n}"));
        let spelled = match kind.as_str() {
            "struct" | "union" | "enum" => format!("{kind} {n}"),
            _ => n.clone(),
        };
        let mut lines = Vec::new();
        if !r["size"].is_empty() && kind != "function" {
            lines.push(format!("NIJI_A (sizeof ({spelled}) == {} && __alignof__ ({spelled}) == {}, \"typedefs {kind} {n}\");", r["size"], r["align"]));
        }
        // A vector typedef (the demangled `float __vector(4)`): the same type, in C and C++, as a vector of that
        // element and count spelled here (a structure of its size and alignment is not).
        let vector = r["cxx"].strip_suffix(')').and_then(|c| c.split_once(" __vector(")).filter(|_| kind == "typedef");
        if let Some((elem, k)) = vector {
            let v = format!("niji_vector_{n}");
            lines.push(format!("typedef {elem} {v} __attribute__ ((__vector_size__ ({k} * sizeof ({elem}))));"));
            lines.push(format!("NIJI_C (__builtin_types_compatible_p ({n}, {v}), \"typedefs typedef {n} is {elem} __vector({k})\");"));
            lines.push(format!("NIJI_X (niji_same<{n}, {v} >::v, \"typedefs typedef {n} is {elem} __vector({k})\");"));
        }
        let cxx = &cxx_type(t, &r["cxx"]);
        match kind.as_str() {
            "typedef" if !cxx.is_empty() && cxx != n && vector.is_none() => lines.push(format!("NIJI_X (niji_same<{n}, {cxx} >::v, \"typedefs typedef {n} is {cxx}\");")),
            "object" if !cxx.is_empty() => lines.push(format!("NIJI_X (niji_same<__typeof__ ({n}), {cxx} >::v, \"typedefs object {n} is {cxx}\");")),
            "function" if !cxx.is_empty() && functions.contains(n) => {
                let ty = if t.nothrow.contains(n) { with_noexcept(cxx) } else { cxx.clone() };
                lines.push(format!("NIJI_X (niji_same<decltype ({n}), {ty} >::v, \"typedefs function {n} is {ty}\");"));
            }
            _ => {}
        }
        c.put(&mut a, i, lines);
    }
    // The data objects libniji exports (versions.tsv): each declared, at its size.
    let mut objects = BTreeMap::new();
    for r in tsv(root, "spec/abi/versions.tsv")?.into_iter().filter(|r| r["type"] == "OBJECT" && r["binding"] == "default" && named(&r["symbol"])) {
        objects.insert(r["symbol"].clone(), r["size"].clone());
    }
    for (sym, size) in &objects {
        let i = c.row("versions", sym);
        // An exported object no header declares (no typedefs.tsv object row) is not a generator construct: counted.
        if !t.typedefs.contains_key(&("object".to_string(), sym.clone())) {
            c.template_owned += 1;
        } else if !size.is_empty() {
            c.put(&mut a, i, vec![format!("NIJI_A (sizeof ({sym}) == {size}, \"versions {sym}\");")]);
        }
    }
    // Enumerations the tables declare as tags (typedefs.tsv `enum`): every enumerators.tsv member has its value and, in
    // C++, the enumeration's type (an enumerator emitted alone is an `int` of an anonymous enumeration).
    for (tag, members) in t.enums.iter().filter(|(k, _)| k.strip_prefix("enum ").is_some_and(|n| wanted(n) && t.typedefs.contains_key(&("enum".to_string(), n.to_string())))) {
        let i = c.row("typedefs", tag);
        let mut lines = Vec::new();
        for (n, v) in members {
            let e = int128(v).ok_or(format!("enumerators.tsv {tag} {n}: value {v:?} is not an integer"))?;
            lines.push(format!("NIJI_A ((__int128) ({n}) == {e}, \"enumerators {tag} {n}\");"));
            lines.push(format!("NIJI_X (niji_same<decltype ({n}), {tag}>::v, \"enumerators {n} is {tag}\");"));
        }
        c.put(&mut a, i, lines);
    }
    // Enumerations without a tag that a typedef names (enumerators.tsv `typedef NAME`, ruling #7643): every member has
    // its value and, in C++, the typedef's type, so the typedef names an enumeration (not an opaque structure of its
    // size) and that enumeration has exactly these members.
    for (tag, members) in t.enums.iter().filter(|(k, _)| k.strip_prefix("typedef ").is_some_and(|n| wanted(n) && t.typedefs.contains_key(&("typedef".to_string(), n.to_string())))) {
        let td = &tag["typedef ".len()..];
        let i = c.row("typedefs", &format!("typedef {td}"));
        let mut lines = Vec::new();
        for (n, v) in members {
            let e = int128(v).ok_or(format!("enumerators.tsv {tag} {n}: value {v:?} is not an integer"))?;
            lines.push(format!("NIJI_A ((__int128) ({n}) == {e}, \"enumerators {tag} {n}\");"));
            lines.push(format!("NIJI_X (niji_same<decltype ({n}), {td}>::v, \"enumerators {n} is {td}\");"));
        }
        c.put(&mut a, i, lines);
    }
    // Enumerators.
    for n in &enumerators {
        let i = c.row("visibility", &format!("enumerator {n}"));
        if let Some(e) = t.macros.get(n).and_then(|(_, v)| int128(v)) {
            c.put(&mut a, i, vec![format!("NIJI_A ((__int128) ({n}) == {e}, \"enumerator {n}\");")]);
        }
    }
    // Macros, each defined, checked and undefined in turn, after every declaration.
    let constants: BTreeMap<String, String> = tsv(root, "spec/abi/constants.tsv")?.into_iter().filter(|r| named(&r["macro"])).map(|r| (r["macro"].clone(), r["value"].clone())).collect();
    let mut ptr_runs: Vec<(usize, Vec<String>)> = Vec::new();
    for (n, (ty, v)) in t.macros.iter().filter(|(n, _)| named(n)) {
        let i = c.row("macros", n);
        if c.lang(i) == Some("*") {
            continue;
        }
        if ty == "null" {
            // A null pointer constant: the generator's own definition (one per language), then its type in each
            // language (C `void *`, C++ `long` as g++'s `__null`) and its value 0, which a run-time check reads.
            let def = match super::values::null_lines(n, v) {
                Ok(l) => l,
                Err(e) => {
                    c.fail(i, format!("generator: {e}"));
                    continue;
                }
            };
            let mut lines = vec![format!("#undef {n}")];
            lines.extend(def.iter().cloned());
            lines.push(format!("NIJI_C (__builtin_types_compatible_p (__typeof__ ({n}), void *), \"macros {n} is void * in C\");"));
            lines.push(format!("NIJI_X (niji_same<decltype ({n}), long>::v, \"macros {n} is long in C++\");"));
            c.put(&mut a, i, lines);
            for row in [i, c.row("constants", n)] {
                let mut l = vec![format!("#undef {n}")];
                l.extend(def.iter().cloned());
                l.push(format!("  if ((long) ({n}) != 0) niji_bad ({row});"));
                l.push(format!("#undef {n}"));
                ptr_runs.push((row, l));
            }
            c.put(&mut a, i, vec![format!("#undef {n}")]);
            continue;
        }
        let lit = g.value(n);
        for e in std::mem::take(&mut g.errors) {
            c.fail(i, format!("generator: {e}"));
        }
        let mut lines = vec![format!("#undef {n}"), format!("#define {n} {lit}")];
        if INTS.contains(&ty.as_str()) {
            lines.push(format!("NIJI_C (_Generic (({n}), {ty}: 1, default: 0), \"macros {n} is {ty}\");"));
            lines.push(format!("NIJI_X (niji_same<decltype ({n}), {ty}>::v, \"macros {n} is {ty}\");"));
            match int128(v) {
                Some(e) => lines.push(format!("NIJI_A ((__int128) ({n}) == {e}, \"macros {n} = {v}\");")),
                None => c.fail(i, format!("value {v:?} is not an integer of 64 bits")),
            }
        } else if ty == "char" {
            // A character constant (D-X.171): `int` in C, `char` in C++, the value of a `char`.
            lines.push(format!("NIJI_C (_Generic (({n}), int: 1, default: 0), \"macros {n} is int in C\");"));
            lines.push(format!("NIJI_X (niji_same<decltype ({n}), char>::v, \"macros {n} is char in C++\");"));
            match int128(v) {
                Some(e) => lines.push(format!("NIJI_A ((__int128) ({n}) == {e}, \"macros {n} = {v}\");")),
                None => c.fail(i, format!("value {v:?} is not an integer")),
            }
        } else if let Some(len) = ty.strip_prefix("char[").and_then(|l| l.strip_suffix(']')) {
            lines.push(format!("NIJI_C (sizeof ({n}) == {len} && _Generic (({n}), char *: 1, default: 0), \"macros {n} is {ty}\");"));
            lines.push(format!("NIJI_X (sizeof ({n}) == {len} && niji_streq ({n}, {v}), \"macros {n} value\");"));
        } else if let Some((_, max)) = FMAX.iter().find(|(f, _)| f == ty) {
            lines.push(format!("NIJI_C (_Generic (({n}), {ty}: 1, default: 0), \"macros {n} is {ty}\");"));
            lines.push(format!("NIJI_X (niji_same<decltype ({n}), {ty}>::v, \"macros {n} is {ty}\");"));
            // The table's hexadecimal value, read by the C++ unit (gnu++17 has hexadecimal floating constants) in
            // the type: an independent spelling of the value the generator writes in decimal.
            let sfx = super::values::FLOATS.iter().find(|r| r.0 == ty).map_or("", |r| r.2);
            let test = match v.as_str() {
                "inf" => format!("({n}) > {max}{sfx}"),
                "-inf" => format!("({n}) < -{max}{sfx}"),
                "nan" => format!("({n}) != ({n}) && !__builtin_issignaling ({n})"),
                "snan" => format!("__builtin_issignaling ({n})"),
                x => format!("({n}) == {x}{sfx}"),
            };
            lines.push(format!("NIJI_X ({test}, \"macros {n} = {v}\");"));
        } else if super::values::pointer_type(ty) {
            // A pointer constant: its type in both languages; its address (a pointer converted to an integer is no
            // constant expression) at run time, against macros.tsv and constants.tsv.
            let cty = g.c_words(ty);
            lines.push(format!("NIJI_C (__builtin_types_compatible_p (__typeof__ ({n}), {cty}), \"macros {n} is {ty}\");"));
            lines.push(format!("NIJI_X (niji_same<decltype ({n}), {cty}>::v, \"macros {n} is {ty}\");"));
            for (row, val) in [(i, Some(v)), (c.row("constants", n), constants.get(n))] {
                match val.map(|x| super::values::literal("pointer", x)) {
                    Some(Ok(e)) => ptr_runs.push((row, vec![format!("#define {n} {lit}"), format!("  if ((long) ({n}) != {e}) niji_bad ({row});"), format!("#undef {n}")])),
                    Some(Err(e)) => c.fail(row, e),
                    None => {}
                }
            }
        } else {
            c.fail(i, format!("macros.tsv type {ty:?}: the rows test has no check for it"));
        }
        c.put(&mut a, i, lines);
        // constants.tsv holds integers only (a floating macro's row there is not its value) and spells a value in
        // its own signedness (`NODEV` -1): compared in the macro's type.
        if let Some(cv) = constants.get(n).filter(|_| INTS.contains(&ty.as_str())) {
            let j = c.row("constants", n);
            match int128(cv) {
                Some(e) => c.put(&mut a, j, vec![format!("NIJI_A (({n}) == ({ty}) {e}, \"constants {n} = {cv}\");")]),
                None => c.fail(j, format!("value {cv:?} is not an integer of 64 bits")),
            }
        }
        c.put(&mut a, i, vec![format!("#undef {n}")]);
    }
    for n in constants.keys().filter(|n| !t.macros.contains_key(*n)) {
        let i = c.row("constants", n);
        c.fail(i, "no macros.tsv row: the generator has no value to emit".into());
    }

    // The run-time program: bit-fields and initializers.
    let mut run = Tu::default();
    for (i, l) in ptr_runs {
        c.put(&mut run, i, l);
    }
    for (key, rows) in t.layouts.iter().filter(|(k, _)| wanted(k.split_once(' ').map_or(k.as_str(), |(_, n)| n))) {
        let i = c.row("layouts", key);
        let mut lines = Vec::new();
        // A bit-field row `bits W@P` gives its first bit P from the low bit of the storage unit at the row's byte offset
        // (psABI 3.1.2, little-endian; recorded by the extraction from DWARF): set to all ones, it covers exactly the
        // bits [8 * offset + P, + W) of an otherwise zero object.
        for r in rows.iter().filter(|r| r["bitfield"].starts_with("bits ")) {
            let bad = || format!("layouts.tsv {key} {}: bad bit-field {:?} (bits W@P)", r["member"], r["bitfield"]);
            let off: u64 = r["offset_or_size"].parse().map_err(|_| bad())?;
            let (w, p) = r["bitfield"][5..].split_once('@').ok_or_else(bad)?;
            let (w, p): (u64, u64) = (w.parse().map_err(|_| bad())?, p.parse().map_err(|_| bad())?);
            let pos = off * 8 + p;
            let Some(d) = designator(rows, &r["member"]) else { continue };
            lines.push(format!("  {{ {key} x; __builtin_memset (&x, 0, sizeof x); x.{d} = -1; if (niji_bits (&x, sizeof x, {pos}, {w})) niji_bad ({i}); }}"));
        }
        c.put(&mut run, i, lines);
    }
    let mut inits = Vec::new();
    for r in tsv(root, "spec/abi/initializers.tsv")?.into_iter().filter(|r| wanted(&r["type"])) {
        let i = c.row("initializers", &r["initializer"]);
        if c.lang(i) == Some("*") {
            continue;
        }
        let init = g.init(&r["initializer"]);
        for e in std::mem::take(&mut g.errors) {
            c.fail(i, format!("generator: {e}"));
        }
        let hex = &r["bytes_hex"];
        let bytes: Vec<String> = (0..hex.len() / 2).map(|k| format!("0x{}", &hex[2 * k..2 * k + 2])).collect();
        inits.push((i, vec![format!("  {{ static const {} x = {init}; static const unsigned char b[] = {{ {} }}; if (sizeof x != sizeof b || __builtin_memcmp (&x, b, sizeof b)) niji_bad ({i}); }}", r["type"], bytes.join(", "))]));
    }
    for (i, l) in inits {
        c.put(&mut run, i, l);
    }

    // visibility.tsv: each declared name has the row its emitter reads.
    let mut seen = BTreeSet::new();
    for r in t.vis.iter().filter(|r| r["header"] != "-" && named(&r["name"])) {
        let (n, kind) = (r["name"].as_str(), r["kind"].as_str());
        if !seen.insert((n.to_string(), kind.to_string())) {
            continue;
        }
        let has = |k: &str| t.typedefs.contains_key(&(k.to_string(), n.to_string()));
        let ok = match kind {
            "struct" | "union" | "enum" => has(kind) || t.layouts.contains_key(&format!("{kind} {n}")),
            "typedef" => has("typedef") || t.layouts.contains_key(n),
            "object" => has("object"),
            // A type-generic macro's function form (providers.tsv `generic`): its overloads are the C++ library's
            // or a template's, as the macro is.
            "function" if t.providers.get(n).is_some_and(|p| p == "generic") => {
                c.template_owned += 1;
                true
            }
            "function" => t.protos.contains_key(n) || has("function"),
            "enumerator" => true,
            "macro" | "include-error" => {
                if !t.macros.contains_key(n) {
                    c.template_owned += 1;
                }
                true
            }
            _ => false,
        };
        if !ok {
            let i = c.row("visibility", &format!("{kind} {n}"));
            c.fail(i, format!("kind {kind}: no table row the emitter of that kind reads"));
        }
    }

    // Call sites: one unit per family of configurations with the same feature levels.
    let cfgs = tsv(root, "spec/abi/ftm-configs.tsv")?;
    let skip = t.lv.reject | g.fortify_mask();
    let mut fams: Vec<(Vec<i64>, Vec<usize>)> = Vec::new();
    for (k, v) in t.lv.values.iter().enumerate().filter(|(k, _)| skip >> k & 1 == 0) {
        match fams.iter_mut().find(|(fv, _)| fv == v) {
            Some((_, ks)) => ks.push(k),
            None => fams.push((v.clone(), vec![k])),
        }
    }
    let mut families = Vec::new();
    for (_, ks) in fams {
        let m = ks.iter().fold(0u64, |m, k| m | 1 << k);
        let ids: Vec<String> = ks.iter().map(|&k| t.lv.ids[k].clone()).collect();
        let first = cfgs.iter().find(|r| r["id"] == ids[0]).ok_or(format!("ftm-configs.tsv: no row {}", ids[0]))?;
        let lang = first["language"].clone();
        let mut expect = BTreeMap::new();
        let mut used = Tu::default();
        for (n, list) in t.addr.iter().filter(|(n, _)| named(n)) {
            let i = c.row("callsites", n);
            let mut syms = BTreeSet::new();
            for (configs, sym) in list {
                if sym == "!error" || mask_of(&t.lv, configs).unwrap_or(0) & m == 0 {
                    continue;
                }
                // A C++-mangled name no library exports: libniji declares every function `extern "C"`
                // (docs/differences-from-glibc.md), so the plain name is the symbol.
                syms.insert(if sym.starts_with("_Z") && !t.exported.contains(sym) { n.clone() } else { sym.clone() });
            }
            if syms.len() > 1 {
                c.fail(i, format!("configurations {} expect different symbols {syms:?}: their feature levels are the same", ids.join(",")));
                continue;
            }
            let Some(sym) = syms.into_iter().next() else { continue };
            let pi = if t.protos.contains_key(n) { c.row("prototypes", n) } else { c.row("typedefs", &format!("function {n}")) };
            let excluded = match c.lang(pi) {
                Some("*") => true,
                Some(l) => l == lang,
                None => false,
            };
            if excluded || !functions.contains(n) || c.lang(i) == Some("*") || c.lang(i) == Some(lang.as_str()) {
                continue;
            }
            used.put(Some(i), [format!("  (void *) &{n},")]);
            expect.insert(i, sym);
        }
        if expect.is_empty() {
            continue;
        }
        let mut tu = Tu::default();
        tu.add(&decls);
        tu.put(None, ["__NIJI_END_C".to_string(), "static void *const niji_used[] __attribute__ ((__used__)) = {".to_string()]);
        tu.add(&used);
        tu.put(None, ["};".to_string()]);
        families.push((ids, lang, first["flags"].clone(), tu, expect));
    }

    let mut asserts = Tu::default();
    asserts.add(&decls);
    asserts.add(&redecl);
    asserts.put(None, ["__NIJI_END_C".to_string()]);
    asserts.put(None, ASSERT_HELPERS.lines().map(String::from));
    asserts.add(&a);
    let mut r = Tu::default();
    r.add(&decls);
    r.put(None, ["__NIJI_END_C".to_string()]);
    r.put(None, RUN_HELPERS.lines().map(String::from));
    r.add(&run);
    r.put(None, ["  return niji_status;".to_string(), "}".to_string()]);
    Ok(Built { check: c, asserts, run: r, families })
}

/// The first lines of every unit.
const PRELUDE: &str = "/* The rows test of `cargo xtask headers-gen` (xtask/src/headers/rows.rs): every table row, no header. */
#include <features.h>
#ifdef __cplusplus
# define NIJI_NX noexcept (true)
# define NIJI_BOOL bool
#else
# define NIJI_NX
# define NIJI_BOOL _Bool
#endif
__NIJI_BEGIN_C";

/// The assertions' helpers: `NIJI_A` in both languages, `NIJI_C` in C only, `NIJI_X` in C++ only.
const ASSERT_HELPERS: &str = "#ifdef __cplusplus
template <class A, class B> struct niji_same { enum { v = 0 }; };
template <class A> struct niji_same<A, A> { enum { v = 1 }; };
constexpr bool niji_streq (const char *a, const char *b) { return *a == *b && (*a == 0 || niji_streq (a + 1, b + 1)); }
# define NIJI_A(c, m) static_assert (c, m)
# define NIJI_C(c, m)
# define NIJI_X(...) static_assert (__VA_ARGS__)
#else
# define NIJI_A(c, m) _Static_assert (c, m)
# define NIJI_C(c, m) _Static_assert (c, m)
# define NIJI_X(...)
#endif";

/// The run-time program's helpers and the start of `main`.
const RUN_HELPERS: &str = "static int niji_status;
static void niji_bad (int row) { __builtin_printf (\"row %d\\n\", row); niji_status = 1; }
/* Whether the bytes at p hold exactly the bits [pos, pos + w) set. */
static int niji_bits (const void *p, unsigned long n, unsigned long pos, unsigned long w)
{
  const unsigned char *b = (const unsigned char *) p;
  for (unsigned long i = 0; i < n * 8; i++)
    if (((b[i / 8] >> (i % 8)) & 1) != (i >= pos && i < pos + w))
      return 1;
  return 0;
}
int main (void)
{";

/// Compile `tu` as `file` below `dir`; the errors, by row (None: a line no row owns).
fn compile(dir: &Path, file: &str, tu: &Tu, cmd: &[String]) -> Result<Vec<(Option<usize>, String)>, String> {
    let mut text = PRELUDE.to_string() + "\n";
    let pre = PRELUDE.lines().count();
    text.push_str(&tu.text());
    let path = dir.join(file);
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    let o = Command::new(&cmd[0]).args(&cmd[1..]).arg(&path).current_dir(dir).output().map_err(|e| format!("{}: {e}", cmd[0]))?;
    let mut out = Vec::new();
    for l in String::from_utf8_lossy(&o.stderr).lines().filter(|l| l.contains(": error: ")) {
        let at = l.strip_prefix(&format!("{}:", path.display())).and_then(|r| r.split(':').next()).and_then(|n| n.parse::<usize>().ok());
        let row = at.and_then(|n| n.checked_sub(pre + 1)).and_then(|k| tu.owner.get(k).copied().flatten());
        out.push((row, l.split_once(": error: ").map_or(l, |(_, m)| m).to_string()));
    }
    if !o.status.success() && out.is_empty() {
        out.push((None, format!("{} failed: {}", cmd[0], String::from_utf8_lossy(&o.stderr).lines().take(3).collect::<Vec<_>>().join(" | "))));
    }
    Ok(out)
}

fn exceptions(root: &Path) -> Result<BTreeMap<(String, String), String>, String> {
    let mut out = BTreeMap::new();
    for (k, r) in tsv(root, EXCEPTIONS)?.into_iter().enumerate() {
        if !matches!(r["lang"].as_str(), "c" | "c++" | "*") || r["reason"].trim().is_empty() {
            return Err(format!("{EXCEPTIONS}: row {}: lang must be c, c++ or *, and a reason is required", k + 1));
        }
        out.insert((r["table"].clone(), r["name"].clone()), r["lang"].clone());
    }
    Ok(out)
}

/// Run the rows test on `t` below `dir` (`only`: the rows of these names). Ok: the summary; Err: the failures.
pub(super) fn check(root: &Path, t: &Tables, dir: &Path, only: Option<&BTreeSet<String>>) -> Result<String, String> {
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut b = build(root, t, exceptions(root)?, only)?;
    let inc = root.join(OUT).display().to_string();
    let gccinc = Command::new("gcc").arg("-print-file-name=include").output().map_err(|e| format!("gcc: {e}"))?;
    let gccinc = String::from_utf8_lossy(&gccinc.stdout).trim().to_string();
    let flags = |cc: &str, extra: &[&str]| -> Vec<String> {
        let mut v = vec![cc.to_string()];
        v.extend(extra.iter().map(|s| s.to_string()));
        // An assertion fails at the line that uses the macro, which names its row.
        v.extend(["-w", "-ftrack-macro-expansion=0", "-nostdinc", "-isystem", &inc, "-isystem", &gccinc].iter().map(|s| s.to_string()));
        v
    };
    let mut errs = compile(dir, "rows.c", &b.asserts, &flags("gcc", &["-std=gnu17", "-fsyntax-only"]))?;
    errs.extend(compile(dir, "rows-cxx.c", &b.asserts, &flags("g++", &["-x", "c++", "-std=gnu++17", "-fsyntax-only"]))?.into_iter().map(|(r, m)| (r, format!("C++: {m}"))));
    let run_errs = compile(dir, "rows-run.c", &b.run, &flags("gcc", &["-std=gnu17", "-o", "rows-run"]))?;
    let ran = run_errs.is_empty();
    errs.extend(run_errs.into_iter().map(|(r, m)| (r, format!("run-time unit: {m}"))));
    if ran {
        let o = Command::new(dir.join("rows-run")).output().map_err(|e| format!("rows-run: {e}"))?;
        for l in String::from_utf8_lossy(&o.stdout).lines() {
            if let Some(i) = l.strip_prefix("row ").and_then(|n| n.parse::<usize>().ok()) {
                errs.push((Some(i), "run time: a bit-field or initializer does not have the table's bits".into()));
            }
        }
        if !o.status.success() && o.stdout.is_empty() {
            errs.push((None, format!("rows-run exited {:?}", o.status.code())));
        }
    }
    let nfam = b.families.len();
    for (k, (ids, lang, cflags, tu, expect)) in b.families.iter().enumerate() {
        let cc = if lang == "c++" { "g++" } else { "gcc" };
        let mut extra: Vec<&str> = if lang == "c++" { vec!["-x", "c++"] } else { vec![] };
        extra.extend(cflags.split_whitespace());
        let obj = format!("rows-cs-{k}.o");
        extra.extend(["-c", "-o", obj.as_str()]);
        let ce = compile(dir, &format!("rows-cs-{k}.c"), tu, &flags(cc, &extra))?;
        let fam = format!("family {} ({lang})", ids.join(","));
        if !ce.is_empty() {
            errs.extend(ce.into_iter().map(|(r, m)| (r, format!("{fam}: {m}"))));
            continue;
        }
        let o = Command::new("nm").arg("-u").arg(dir.join(&obj)).output().map_err(|e| format!("nm: {e}"))?;
        let undef: BTreeSet<String> = String::from_utf8_lossy(&o.stdout).lines().filter_map(|l| l.split_whitespace().last().map(String::from)).collect();
        let want: BTreeSet<&String> = expect.values().collect();
        for (i, sym) in expect {
            if !undef.contains(sym) {
                errs.push((Some(*i), format!("{fam}: the address does not reference {sym}")));
            }
        }
        for s in undef.iter().filter(|s| !want.contains(s)) {
            errs.push((None, format!("{fam}: unexpected symbol {s}")));
        }
    }
    for (row, m) in errs {
        match row {
            Some(i) => b.check.fail(i, m),
            None => b.check.unit.push(m),
        }
    }
    let c = &b.check;
    let mut per: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    for (i, (table, _)) in c.rows.iter().enumerate() {
        let e = per.entry(table.as_str()).or_default();
        e.0 += 1;
        e.1 += usize::from(c.failed.contains_key(&i));
        e.2 += usize::from(c.except.contains_key(&c.rows[i]));
    }
    let mut summary: Vec<String> = per.iter().map(|(t, (n, f, x))| format!("{t} {n} rows, {f} failed, {x} excepted")).collect();
    summary.push(format!("{nfam} call-site families; {} visibility names templates define", c.template_owned));
    let unused: Vec<String> = c.except.keys().filter(|k| !c.index.contains_key(*k)).map(|(t, n)| format!("{t} {n}")).collect();
    if only.is_none() && !unused.is_empty() {
        b.check.unit.push(format!("{EXCEPTIONS}: rows that name no table row: {}", unused.join(", ")));
    }
    let c = &b.check;
    if c.failed.is_empty() && c.unit.is_empty() {
        return Ok(summary.join("; "));
    }
    let mut out: Vec<String> = c.unit.iter().map(|m| format!("FAIL (unit) {m}")).collect();
    for (i, ms) in &c.failed {
        let (table, name) = &c.rows[*i];
        out.push(format!("FAIL {table} {name}: {}", ms.iter().take(3).cloned().collect::<Vec<_>>().join(" | ")));
    }
    let n = out.len();
    fs::write(dir.join("failures.txt"), out.join("\n") + "\n").map_err(|e| format!("{}: {e}", dir.display()))?;
    out.truncate(400);
    out.push(format!("headers-gen --rows: {} failed ({} rows, {}); see {}", n, c.rows.len(), summary.join("; "), dir.display()));
    Err(out.join("\n"))
}

/// `cargo xtask headers-gen --rows DIR`.
pub(super) fn run(root: &Path, dir: &Path) -> Result<(), String> {
    let t = load(root)?;
    let s = check(root, &t, dir, None)?;
    println!("headers-gen --rows: ok: {s}");
    Ok(())
}
