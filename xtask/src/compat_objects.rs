//! The compat data objects of `libc.so.6` (phase 15, LG-6 `LEGACY-OBJECTS`), generated into
//! `csys/src/machine/compat_aliases.S` next to LG-1's stubs (`compat.rs`, R15-1, R15-2).
//!
//! An old binary that names one of these objects gets a copy relocation of the object's `st_size`
//! bytes, so a wrong size is memory corruption. Every `libc.so.6` compat `OBJECT` row of [`NAMES`]
//! becomes one label with the manifest's size and the non-default version (single `@`); the
//! versions of one name share one table, since each older version is a prefix of the newest
//! (`spec/behaviour/compat-objects.md`, probe-settled). Contents:
//! * `sys_errlist`/`_sys_errlist`: the `strerror` texts of `spec/abi/errmsgs.tsv`, NULL where the
//!   text is "Unknown error n" and past the last errno; `sys_nerr`/`_sys_nerr`: the pointers in the
//!   same version of `sys_errlist`;
//! * `sys_siglist`/`_sys_siglist`/`sys_sigabbrev`: `spec/abi/sigmsgs.tsv`, NULL elsewhere;
//! * the inert malloc hooks, `__morecore`, `mallwatch`, `_null_auth` and `_obstack`: zero bytes;
//! * `__ctype_b`, `__ctype_tolower`, `__ctype_toupper` and the `__ctype32_*` names: a pointer to
//!   the `"C"` locale's table at character 0 ([`ctype_tables`]; `spec/behaviour/ctype-objects.md`).
//!   The tables are rendered here, in `libc.so.6` only: the library's own copies live in a crate
//!   that cannot name a symbol (`niji_legacy`, `forbid(unsafe_code)`), and an assembled object can
//!   only point at a named symbol (D-5.13, records/decisions/D-5/D-5.13.toml).
//!
//! `cargo xtask so` checks that every exported object of `libc.so.6` whose row `versions.tsv`
//! has carries the row's size ([`problems`]).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// The compat-only data objects this module generates.
pub const NAMES: &[&str] = &[
    "sys_errlist",
    "_sys_errlist",
    "sys_nerr",
    "_sys_nerr",
    "sys_siglist",
    "_sys_siglist",
    "sys_sigabbrev",
    "__malloc_hook",
    "__free_hook",
    "__realloc_hook",
    "__memalign_hook",
    "__after_morecore_hook",
    "__malloc_initialize_hook",
    "__morecore",
    "mallwatch",
    "_null_auth",
    "loc1",
    "loc2",
    "locs",
    "_obstack",
    "__ctype_b",
    "__ctype_tolower",
    "__ctype_toupper",
    "__ctype32_b",
    "__ctype32_tolower",
    "__ctype32_toupper",
];

/// Entries of each `"C"` ctype table: characters -128 to 255, so that the table pointer (at
/// character 0) is indexable from -128 (`<ctype.h>`'s classification macros of old binaries).
pub const CTYPE_LEN: usize = 384;
/// Index of character 0 in the ctype tables.
pub const CTYPE_ZERO: usize = 128;

/// The `"C"` locale's class bits of byte `c` (POSIX "C" locale, ASCII only), in `<ctype.h>`'s
/// `_ISbit` order: upper, lower, alpha, digit, xdigit, space, print, graph, blank, cntrl, punct,
/// alnum (bits 0 to 11).
fn c_bits(c: u8) -> [bool; 12] {
    let (upper, lower, digit) = (c.is_ascii_uppercase(), c.is_ascii_lowercase(), c.is_ascii_digit());
    let graph = c.is_ascii_graphic();
    let alnum = upper || lower || digit;
    [
        upper,
        lower,
        upper || lower,
        digit,
        c.is_ascii_hexdigit(),
        matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'),
        graph || c == b' ',
        graph,
        matches!(c, b' ' | b'\t'),
        c < 0x20 || c == 0x7f,
        graph && !alnum,
        alnum,
    ]
}

/// The `"C"` ctype tables, each indexed by character + [`CTYPE_ZERO`].
pub struct CtypeTables {
    /// `unsigned short` class masks, `_ISbit(b)`: bits 0-7 in the high byte, 8-15 in the low.
    pub class: Vec<u16>,
    /// 32-bit class masks, the four bytes of `1 << b` reversed (`__ctype32_b`).
    pub class32: Vec<u32>,
    pub tolower: Vec<i32>,
    pub toupper: Vec<i32>,
}

/// The `"C"` ctype tables: no class outside 0-127; a case table maps a negative `char` other
/// than `EOF` to the byte it aliases (c + 256), `EOF` to itself, and every byte outside A-Z/a-z to
/// itself (probe-settled, `spec/behaviour/ctype-objects.md`).
pub fn ctype_tables() -> CtypeTables {
    let mut t = CtypeTables { class: vec![0; CTYPE_LEN], class32: vec![0; CTYPE_LEN], tolower: vec![0; CTYPE_LEN], toupper: vec![0; CTYPE_LEN] };
    for (i, c) in (-(CTYPE_ZERO as i32)..(CTYPE_LEN - CTYPE_ZERO) as i32).enumerate() {
        if let Some(b) = u8::try_from(c).ok().filter(u8::is_ascii) {
            for (bit, on) in c_bits(b).into_iter().enumerate() {
                if on {
                    t.class[i] |= if bit < 8 { (1u16 << bit) << 8 } else { (1u16 << bit) >> 8 };
                    t.class32[i] |= (1u32 << bit).swap_bytes();
                }
            }
        }
        let (lo, up) = match u8::try_from(c) {
            Ok(b) if b < 128 => (i32::from(b.to_ascii_lowercase()), i32::from(b.to_ascii_uppercase())),
            _ if c < -1 => (c + 256, c + 256),
            _ => (c, c),
        };
        t.tolower[i] = lo;
        t.toupper[i] = up;
    }
    t
}

/// The table a ctype object points at, and the byte offset of character 0 in it.
fn ctype_target(name: &str) -> (&'static str, usize) {
    match name {
        "__ctype_b" => ("class", 2 * CTYPE_ZERO),
        "__ctype32_b" => ("class32", 4 * CTYPE_ZERO),
        "__ctype_tolower" | "__ctype32_tolower" => ("tolower", 4 * CTYPE_ZERO),
        _ => ("toupper", 4 * CTYPE_ZERO),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Errlist,
    Nerr,
    Siglist,
    Sigabbrev,
    Zero,
    Ctype,
}

fn kind(name: &str) -> Kind {
    match name {
        "sys_errlist" | "_sys_errlist" => Kind::Errlist,
        "sys_nerr" | "_sys_nerr" => Kind::Nerr,
        "sys_siglist" | "_sys_siglist" => Kind::Siglist,
        "sys_sigabbrev" => Kind::Sigabbrev,
        n if n.starts_with("__ctype") => Kind::Ctype,
        _ => Kind::Zero,
    }
}

/// One generated object: `name@version`, `size` bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub name: String,
    pub version: String,
    pub size: usize,
    pub weak: bool,
}

impl Object {
    /// The object's own (local in `libc.so.6`) symbol.
    pub fn label(&self) -> String {
        format!("niji_compat_{}_{}", self.name, self.version.replace('.', "_"))
    }
}

/// The objects and the contents of the shared tables.
#[derive(Debug)]
pub struct Model {
    pub objects: Vec<Object>,
    pub errlist: Vec<Option<String>>,
    pub siglist: Vec<Option<String>>,
    pub sigabbrev: Vec<Option<String>>,
}

impl Model {
    /// The value of `sys_nerr@version`: the pointers in `sys_errlist@version`.
    pub fn nerr(&self, version: &str) -> Option<usize> {
        self.objects.iter().find(|o| kind(&o.name) == Kind::Errlist && o.version == version).map(|o| o.size / 8)
    }
}

fn version_key(v: &str) -> Vec<u32> {
    v.trim_start_matches("GLIBC_").split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))
}

/// The model from the texts of `versions.tsv`, `errmsgs.tsv` and `sigmsgs.tsv`.
pub fn model_from(versions: &str, errmsgs: &str, sigmsgs: &str) -> Result<Model, String> {
    let mut objects = Vec::new();
    for line in versions.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if let [lib, sym, ver, "compat", typ, scope, size] = f.as_slice() {
            if *lib != "libc.so.6" || !NAMES.contains(sym) {
                continue;
            }
            if *typ != "OBJECT" {
                return Err(format!("compat-gen: {sym}@{ver} is {typ}, not OBJECT"));
            }
            let size: usize = size.parse().map_err(|_| format!("compat-gen: {sym}@{ver} has no size"))?;
            objects.push(Object { name: sym.to_string(), version: ver.to_string(), size, weak: *scope == "WEAK" });
        }
    }
    for n in NAMES {
        if !objects.iter().any(|o| o.name == *n) {
            return Err(format!("compat-gen: {n} has no libc.so.6 compat row in spec/abi/versions.tsv"));
        }
    }
    objects.sort_by(|a, b| (a.name.as_str(), version_key(&a.version)).cmp(&(b.name.as_str(), version_key(&b.version))));
    let entries = |k: Kind| objects.iter().filter(|o| kind(&o.name) == k).map(|o| o.size / 8).max().unwrap_or(0);
    let mut errlist = vec![None; entries(Kind::Errlist)];
    for line in errmsgs.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if let [n, _, msg] = f.as_slice() {
            let n: usize = n.parse().map_err(|_| format!("errmsgs.tsv: bad errno {n}"))?;
            if n < errlist.len() && *msg != format!("Unknown error {n}") {
                errlist[n] = Some(msg.to_string());
            }
        }
    }
    let (mut siglist, mut sigabbrev) = (vec![None; entries(Kind::Siglist)], vec![None; entries(Kind::Sigabbrev)]);
    for line in sigmsgs.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if let [n, abbrev, descr] = f.as_slice() {
            let n: usize = n.parse().map_err(|_| format!("sigmsgs.tsv: bad signal {n}"))?;
            if n >= siglist.len() || n >= sigabbrev.len() {
                return Err(format!("sigmsgs.tsv: signal {n} does not fit the manifest's tables"));
            }
            siglist[n] = Some(descr.to_string());
            sigabbrev[n] = Some(abbrev.to_string());
        }
    }
    let model = Model { objects, errlist, siglist, sigabbrev };
    for o in &model.objects {
        match kind(&o.name) {
            Kind::Nerr if o.size != 4 || model.nerr(&o.version).is_none() => {
                return Err(format!("compat-gen: {}@{} needs 4 bytes and a sys_errlist of its version", o.name, o.version));
            }
            Kind::Ctype if o.size != 8 => {
                return Err(format!("compat-gen: {}@{} is a table pointer: 8 bytes, not {}", o.name, o.version, o.size));
            }
            Kind::Errlist | Kind::Siglist | Kind::Sigabbrev if o.size % 8 != 0 => {
                return Err(format!("compat-gen: {}@{} is not a whole number of pointers", o.name, o.version));
            }
            _ => {}
        }
    }
    Ok(model)
}

/// The model of this tree.
pub fn model(root: &Path) -> Result<Model, String> {
    model_from(&read(root, "spec/abi/versions.tsv")?, &read(root, "spec/abi/errmsgs.tsv")?, &read(root, "spec/abi/sigmsgs.tsv")?)
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn declare(s: &mut String, objects: &[&Object]) {
    for o in objects {
        let l = o.label();
        let bind = if o.weak { "weak" } else { "globl" };
        s.push_str(&format!("\t.{bind} {l}\n\t.type {l}, @object\n\t.size {l}, {}\n", o.size));
    }
    for o in objects {
        s.push_str(&format!("{}:\n", o.label()));
    }
}

fn symver(s: &mut String, objects: &[&Object]) {
    for o in objects {
        s.push_str(&format!("\t.symver {}, {}@{}\n", o.label(), o.name, o.version));
    }
}

/// The assembly of the objects (directives only, K2).
pub fn render(m: &Model) -> String {
    let mut s = format!(
        "/* The compat data objects (LG-6): {} versions of {} names, each at the manifest's size; the versions of\n\
         * one name share one table (spec/behaviour/compat-objects.md). */\n",
        m.objects.len(),
        NAMES.len()
    );
    let tables = [(Kind::Errlist, "err", &m.errlist), (Kind::Siglist, "sig", &m.siglist), (Kind::Sigabbrev, "abbr", &m.sigabbrev)];
    s.push_str("\t.section .rodata.niji_compat_strings,\"aMS\",@progbits,1\n");
    for (_, tag, t) in tables {
        for (i, e) in t.iter().enumerate() {
            if let Some(text) = e {
                s.push_str(&format!(".Lniji_compat_{tag}_{i}:\n\t.asciz \"{}\"\n", escape(text)));
            }
        }
    }
    for (k, tag, t) in tables {
        let objs: Vec<&Object> = m.objects.iter().filter(|o| kind(&o.name) == k).collect();
        s.push_str(&format!("\t.section .data.rel.ro.niji_compat_{tag},\"aw\",@progbits\n\t.p2align 3\n"));
        declare(&mut s, &objs);
        for (i, e) in t.iter().enumerate() {
            match e {
                Some(_) => s.push_str(&format!("\t.quad .Lniji_compat_{tag}_{i}\n")),
                None => s.push_str("\t.quad 0\n"),
            }
        }
        symver(&mut s, &objs);
    }
    let mut nerr: BTreeMap<usize, Vec<&Object>> = BTreeMap::new();
    for o in m.objects.iter().filter(|o| kind(&o.name) == Kind::Nerr) {
        nerr.entry(m.nerr(&o.version).unwrap_or(0)).or_default().push(o);
    }
    for (v, objs) in nerr {
        s.push_str(&format!("\t.section .rodata.niji_compat_nerr_{v},\"a\",@progbits\n\t.p2align 2\n"));
        declare(&mut s, &objs);
        s.push_str(&format!("\t.long {v}\n"));
        symver(&mut s, &objs);
    }
    for o in m.objects.iter().filter(|o| kind(&o.name) == Kind::Zero) {
        s.push_str(&format!("\t.section .bss.{},\"aw\",@nobits\n\t.p2align 3\n", o.label()));
        declare(&mut s, &[o]);
        s.push_str(&format!("\t.zero {}\n", o.size));
        symver(&mut s, &[o]);
    }
    let ctype: Vec<&Object> = m.objects.iter().filter(|o| kind(&o.name) == Kind::Ctype).collect();
    if !ctype.is_empty() {
        let t = ctype_tables();
        s.push_str("\t.section .rodata.niji_compat_ctype,\"a\",@progbits\n\t.p2align 6\n");
        let rows = |s: &mut String, label: &str, dir: &str, v: Vec<String>| {
            s.push_str(&format!(".Lniji_compat_ctype_{label}:\n"));
            for chunk in v.chunks(16) {
                s.push_str(&format!("\t{dir} {}\n", chunk.join(",")));
            }
        };
        rows(&mut s, "class", ".short", t.class.iter().map(|v| format!("{v:#06x}")).collect());
        rows(&mut s, "class32", ".long", t.class32.iter().map(|v| format!("{v:#010x}")).collect());
        rows(&mut s, "tolower", ".long", t.tolower.iter().map(i32::to_string).collect());
        rows(&mut s, "toupper", ".long", t.toupper.iter().map(i32::to_string).collect());
        // Writable, as the reference's: the pointers are variables an old binary may assign.
        for &o in &ctype {
            let (table, off) = ctype_target(&o.name);
            s.push_str(&format!("\t.section .data.{},\"aw\",@progbits\n\t.p2align 3\n", o.label()));
            declare(&mut s, &[o]);
            s.push_str(&format!("\t.quad .Lniji_compat_ctype_{table}+{off}\n"));
            symver(&mut s, &[o]);
        }
        store(&mut s, &ctype);
    }
    s
}

/// The order of the six words `niji_compat_ctype_store` takes (`crates/boundary/src/locale.rs`,
/// `publish_global`): the 16-bit class, `tolower`, `toupper` tables, then the 32-bit ones.
pub const CTYPE_STORE_ORDER: [&str; 6] = ["__ctype_b", "__ctype_tolower", "__ctype_toupper", "__ctype32_b", "__ctype32_tolower", "__ctype32_toupper"];

/// CTYPE-OBJECTS-SETLOCALE: `niji_compat_ctype_store(const uintptr_t words[6])`, which `setlocale`
/// calls (through csys's weak hook, absent from libc.a) when the global `LC_CTYPE` changes. Each
/// word goes through the GOT of the object's versioned name, so an old binary's COPY of the
/// object is the one written (the reference's objects follow the global locale:
/// spec/behaviour/ctype-objects.md). Hidden: only the library calls it.
fn store(s: &mut String, ctype: &[&Object]) {
    let f = "niji_compat_ctype_store";
    s.push_str(&format!("\t.section .text.{f},\"ax\",@progbits\n\t.globl {f}\n\t.hidden {f}\n\t.type {f}, @function\n\t.p2align 4\n{f}:\n"));
    let mut refs = String::new();
    for (i, name) in CTYPE_STORE_ORDER.iter().enumerate() {
        let Some(o) = ctype.iter().find(|o| o.name == *name) else { continue };
        let r = format!("niji_compat_{name}_store_ref");
        s.push_str(&format!("\tmovq {}(%rdi), %rax\n\tmovq {r}@GOTPCREL(%rip), %rcx\n\tmovq %rax, (%rcx)\n", 8 * i));
        refs.push_str(&format!("\t.symver {r}, {name}@{}\n", o.version));
    }
    s.push_str(&format!("\tret\n\t.size {f}, .-{f}\n{refs}"));
}

/// `text` (LG-1's rendering) with the objects before its closing GNU-stack note.
pub fn insert(text: &str, objects: &str) -> String {
    const NOTE: &str = "\t.section .note.GNU-stack";
    match text.rfind(NOTE) {
        Some(i) => format!("{}{objects}{}", &text[..i], &text[i..]),
        None => format!("{text}{objects}"),
    }
}

/// The problems of `readelf --dyn-syms -W` text of `libc.so.6`: every generated object exported as
/// `name@version` with type `OBJECT`, and every exported `OBJECT` (default or compat) that has a
/// row in `versions` with the row's size.
pub fn problems(readelf: &str, objects: &[Object], versions: &str) -> Vec<String> {
    let mut rows: BTreeMap<(String, String), usize> = BTreeMap::new();
    for line in versions.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if let ["libc.so.6", sym, ver, _, "OBJECT", _, size] = f.as_slice() {
            if let Ok(size) = size.parse() {
                rows.insert((sym.to_string(), ver.to_string()), size);
            }
        }
    }
    let mut exported: BTreeMap<(String, String), (usize, String)> = BTreeMap::new();
    for line in readelf.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(size), Some(typ), Some(ndx), Some(name)) = (f.get(2), f.get(3), f.get(6), f.get(7)) else { continue };
        if *ndx == "UND" {
            continue;
        }
        let size = match size.strip_prefix("0x") {
            Some(h) => usize::from_str_radix(h, 16).ok(),
            None => size.parse().ok(),
        };
        let (Some(size), Some((sym, ver))) = (size, name.split_once('@')) else { continue };
        exported.insert((sym.to_string(), ver.trim_start_matches('@').to_string()), (size, typ.to_string()));
    }
    let mut out = Vec::new();
    for o in objects {
        match exported.get(&(o.name.clone(), o.version.clone())) {
            None => out.push(format!("libc.so.6 does not export the compat object {}@{}", o.name, o.version)),
            Some((_, typ)) if typ != "OBJECT" => out.push(format!("{}@{} is {typ}, not OBJECT", o.name, o.version)),
            _ => {}
        }
    }
    for ((sym, ver), (size, typ)) in &exported {
        if let (Some(want), "OBJECT") = (rows.get(&(sym.clone(), ver.clone())), typ.as_str()) {
            if size != want {
                out.push(format!("{sym}@{ver}: st_size {size}, spec/abi/versions.tsv says {want} (a copy relocation copies the manifest's size)"));
            }
        }
    }
    out
}

/// `cargo xtask so`'s check of `so` (`libc.so.6`). The number of objects checked.
pub fn check(root: &Path, so: &Path) -> Result<usize, String> {
    let m = model(root)?;
    let out = Command::new("readelf").args(["--dyn-syms", "-W"]).arg(so).output().map_err(|e| format!("readelf: {e}"))?;
    let readelf = String::from_utf8_lossy(&out.stdout);
    let p = problems(&readelf, &m.objects, &read(root, "spec/abi/versions.tsv")?);
    let objects = readelf.lines().filter(|l| l.split_whitespace().nth(3) == Some("OBJECT")).count();
    if p.is_empty() { Ok(objects) } else { Err(format!("data objects of libc.so.6:\n  {}", p.join("\n  "))) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
    }

    #[test]
    fn contents_equal_the_glibc_probe() {
        let m = model(root()).unwrap();
        let probe = read(root(), "spec/behaviour/compat-objects.glibc.txt").unwrap();
        let mut seen = BTreeMap::new();
        for line in probe.lines() {
            let Some((key, val)) = line.split_once("]=") else { continue };
            let (table, i) = key.split_once('[').unwrap();
            let t = match table {
                "sys_errlist@GLIBC_2.12" => &m.errlist,
                "sys_siglist@GLIBC_2.3.3" => &m.siglist,
                "sys_sigabbrev@GLIBC_2.3.3" => &m.sigabbrev,
                other => panic!("unexpected probe table {other}"),
            };
            let i: usize = i.parse().unwrap();
            let want = (val != "(null)").then(|| val.to_string());
            assert_eq!(t[i], want, "{table}[{i}]");
            *seen.entry(table.to_string()).or_insert(0) += 1;
        }
        assert_eq!(seen["sys_errlist@GLIBC_2.12"], m.errlist.len());
        assert_eq!(seen["sys_siglist@GLIBC_2.3.3"], m.siglist.len());
        assert_eq!(seen["sys_sigabbrev@GLIBC_2.3.3"], m.sigabbrev.len());
        // Older versions are prefixes, the zero objects are zero.
        assert!(probe.lines().filter(|l| l.contains(" vs ")).all(|l| l.contains(": 0 of ")));
        let nerr = |v: &str| m.nerr(v).unwrap();
        let line = format!(
            "sys_nerr 2.2.5={} 2.3={} 2.4={} 2.12={}; _sys_nerr 2.2.5={} 2.12={}",
            nerr("GLIBC_2.2.5"),
            nerr("GLIBC_2.3"),
            nerr("GLIBC_2.4"),
            nerr("GLIBC_2.12"),
            nerr("GLIBC_2.2.5"),
            nerr("GLIBC_2.12")
        );
        assert!(probe.lines().any(|l| l == line), "{line}");
        assert!(probe.lines().any(|l| l.starts_with("hooks null:") && !l.contains("=0")));
        assert!(probe.lines().any(|l| l == "_null_auth nonzero bytes=0"));
    }

    // LG-10 (spec/behaviour/regexp-h.md, s29_4_c_0): step/advance write loc1/loc2, so the three
    // `<regexp.h>` objects are writable zeroed storage, never a read-only table.
    #[test]
    fn the_regexp_h_objects_are_writable_zeroed_storage() {
        let s = render(&model(root()).unwrap());
        for o in ["loc1", "loc2", "locs"] {
            let l = format!("niji_compat_{o}_GLIBC_2_2_5");
            let want = format!("\t.section .bss.{l},\"aw\",@nobits\n\t.p2align 3\n\t.globl {l}\n\t.type {l}, @object\n\t.size {l}, 8\n{l}:\n\t.zero 8\n\t.symver {l}, {o}@GLIBC_2.2.5\n");
            assert!(s.contains(&want), "{o}");
        }
    }

    // LG-6 (spec/behaviour/ctype-objects.md, s5_1_c_0 compat): every entry of the "C" tables the
    // legacy pointers reach equals the reference's, read through the GLIBC_2.2.5 objects.
    #[test]
    fn ctype_tables_equal_the_glibc_probe() {
        let t = ctype_tables();
        let probe = read(root(), "spec/behaviour/ctype-objects.glibc.txt").unwrap();
        let (mut narrow, mut wide) = (0, 0);
        for line in probe.lines() {
            let f: Vec<&str> = line.split_whitespace().collect();
            let ["start", w @ ("b" | "w"), c, class, lo, up] = f.as_slice() else { continue };
            let c: i32 = c.parse().unwrap();
            let i = usize::try_from(c + CTYPE_ZERO as i32).unwrap();
            let class = u32::from_str_radix(class, 16).unwrap();
            let got = if *w == "b" { u32::from(t.class[i]) } else { t.class32[i] };
            assert_eq!((got, t.tolower[i], t.toupper[i]), (class, lo.parse().unwrap(), up.parse().unwrap()), "{line}");
            if *w == "b" { narrow += 1 } else { wide += 1 }
        }
        assert_eq!((narrow, wide), (CTYPE_LEN, 256));
    }

    // Each pointer is the table's address at character 0 (an old binary indexes it from -128),
    // writable, at the compat version only (single `@`: no new link can bind it).
    #[test]
    fn ctype_objects_point_at_character_zero_at_the_compat_version() {
        let s = render(&model(root()).unwrap());
        for (name, table, off) in [("__ctype_b", "class", 256), ("__ctype32_b", "class32", 512), ("__ctype_tolower", "tolower", 512), ("__ctype32_toupper", "toupper", 512)] {
            let l = format!("niji_compat_{name}_GLIBC_2_2_5");
            let want = format!("\t.section .data.{l},\"aw\",@progbits\n\t.p2align 3\n\t.globl {l}\n\t.type {l}, @object\n\t.size {l}, 8\n{l}:\n\t.quad .Lniji_compat_ctype_{table}+{off}\n\t.symver {l}, {name}@GLIBC_2.2.5\n");
            assert!(s.contains(&want), "{name}");
        }
        let l = "niji_compat__obstack_GLIBC_2_2_5";
        assert!(s.contains(&format!("{l}:\n\t.zero 8\n\t.symver {l}, _obstack@GLIBC_2.2.5\n")));
    }

    #[test]
    fn every_object_row_is_generated_at_its_size() {
        let m = model(root()).unwrap();
        assert_eq!(m.objects.len(), 41);
        let s = render(&m);
        for o in &m.objects {
            assert!(s.contains(&format!("\t.size {}, {}\n", o.label(), o.size)), "{o:?}");
            assert!(s.contains(&format!("\t.symver {}, {}@{}\n", o.label(), o.name, o.version)), "{o:?}");
        }
        assert!(!s.contains("@@"));
    }

    #[test]
    fn problems_name_a_wrong_size_a_missing_object_and_a_function() {
        let versions = "library\tsymbol\tversion\tbinding\ttype\tscope\tsize\nlibc.so.6\tsys_nerr\tGLIBC_2.12\tcompat\tOBJECT\tGLOBAL\t4\nlibc.so.6\tenviron\tGLIBC_2.2.5\tdefault\tOBJECT\tWEAK\t8\n";
        let o = vec![Object { name: "sys_nerr".into(), version: "GLIBC_2.12".into(), size: 4, weak: false }];
        let good = "   1: 0000000000002000     4 OBJECT  GLOBAL DEFAULT   14 sys_nerr@GLIBC_2.12\n   2: 0000000000003000     8 OBJECT  WEAK   DEFAULT   20 environ@@GLIBC_2.2.5\n";
        assert!(problems(good, &o, versions).is_empty());
        let short = good.replace("    4 OBJECT", "    2 OBJECT");
        assert_eq!(problems(&short, &o, versions).len(), 1);
        assert!(problems(&short, &o, versions)[0].contains("st_size 2"));
        assert_eq!(problems(&good.replace("     8 OBJECT", "    16 OBJECT"), &o, versions).len(), 1);
        assert_eq!(problems(&good.replace("4 OBJECT", "4 FUNC  "), &o, versions).len(), 1);
        assert_eq!(problems("", &o, versions).len(), 1);
    }
}
