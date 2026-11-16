//! `libmvec.so.1` and `libmvec.a`, the vector math library (SYS-LIBMVEC, ledger M-0/M-1,
//! plan/package-parity-audit.md row 2), built by `cargo xtask so` after `libm.so.6`.
//!
//! **Generated from `spec/abi/versions.tsv`, nothing hand-listed.** The exports are the table's
//! `libmvec.so.1` rows (216 `_ZGV*` names at `GLIBC_2.22` and `GLIBC_2.35`), each at its row's
//! version, with the version nodes of [`crate::placeholders::plan`].
//!
//! **The calling convention is the compiler's.** A name `_ZGV<isa>N<lanes><params>_<f>` is the
//! x86-64 vector function ABI's unmasked (`N`) variant of scalar `f` for ISA class `b` (SSE, 128-bit
//! registers), `c` (AVX), `d` (AVX2, 256-bit) or `e` (AVX-512, 512-bit), `<lanes>` elements per
//! call, one `v` per vector parameter. For each scalar function the generated C defines one wrapper
//! `__niji_mvec_<f>` that calls libniji's scalar `f`, declared `__attribute__((simd("notinbranch")))`:
//! the compiler emits the four unmasked clones `_ZGV<isa>N<lanes><params>___niji_mvec_<f>` with
//! the ABI's register layout (the same code that lays out the arguments of a vectorized caller),
//! and a `.symver` per export names the clone. If the compiler's lane count for an ISA differed from
//! the table's, the `.symver` would name an undefined clone and the build fails.
//!
//! Each lane is the scalar function's result: libmvec's results are libm's, bit for bit (the
//! accuracy bound of the vector variants is looser; D-NEW of SYS-LIBMVEC, docs/differences-from-glibc.md).
//!
//! **The check** is [`crate::placeholders::product_errors`] (soname, `DT_NEEDED libc.so.6`, version
//! definitions, every defined dynamic symbol against the table), plus `DT_NEEDED libm.so.6`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::placeholders::{Export, Plan};

/// The library's soname.
pub const SONAME: &str = "libmvec.so.1";

/// The static archive and the development link the development package ships.
pub const ARCHIVE: &str = "libmvec.a";
pub const DEV_LINK: &str = "libmvec.so";

/// The prefix of the scalar wrapper whose clones are the exports.
pub const WRAPPER: &str = "__niji_mvec_";

/// One export's vector function ABI name, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// ISA class letter: `b`, `c`, `d` or `e`.
    pub isa: char,
    /// Elements per call.
    pub lanes: u32,
    /// One `v` per vector parameter.
    pub params: String,
    /// The scalar function.
    pub scalar: String,
}

/// The vector register width of an ISA class for floating-point elements, in bits.
fn width(isa: char) -> Option<u32> {
    match isa {
        'b' => Some(128),
        'c' | 'd' => Some(256),
        'e' => Some(512),
        _ => None,
    }
}

/// Decode `_ZGV<isa>N<lanes><params>_<scalar>`; `None` for any other shape (masked `M` variants,
/// `l`/`u` parameters and unknown ISA classes are not libmvec.so.1 exports).
pub fn parse(name: &str) -> Option<Variant> {
    let rest = name.strip_prefix("_ZGV")?;
    let mut chars = rest.chars();
    let isa = chars.next()?;
    width(isa)?;
    let rest = chars.as_str().strip_prefix('N')?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let lanes: u32 = digits.parse().ok()?;
    let rest = rest.get(digits.len()..)?;
    let (params, scalar) = rest.split_once('_')?;
    if params.is_empty() || !params.chars().all(|c| c == 'v') || scalar.is_empty() {
        return None;
    }
    Some(Variant { isa, lanes, params: params.to_string(), scalar: scalar.to_string() })
}

/// The C element type of a variant: the register width over the lane count (64 bits: `double`,
/// 32 bits: `float`).
pub fn element(v: &Variant) -> Result<&'static str, String> {
    match width(v.isa).and_then(|w| w.checked_div(v.lanes)) {
        Some(64) => Ok("double"),
        Some(32) => Ok("float"),
        other => Err(format!("_ZGV{}N{}{}_{}: {other:?} bits per lane is neither double nor float", v.isa, v.lanes, v.params, v.scalar)),
    }
}

/// The scalar prototype and the wrapper body of one function shape: `v` is `T f(T)`, `vv` is
/// `T f(T, T)`, `vvv` is the `sincos` shape `void f(T, T *, T *)` (its two pointer parameters are
/// vectors of pointers, one per lane). Any other shape is an error.
fn shape(scalar: &str, params: &str, t: &str) -> Result<(String, String), String> {
    let w = format!("{WRAPPER}{scalar}");
    match params {
        "v" => Ok((format!("{t} {scalar}({t});"), format!("{t} {w}({t} x) {{ return {scalar}(x); }}"))),
        "vv" => Ok((format!("{t} {scalar}({t}, {t});"), format!("{t} {w}({t} x, {t} y) {{ return {scalar}(x, y); }}"))),
        "vvv" if scalar.starts_with("sincos") => Ok((
            format!("void {scalar}({t}, {t} *, {t} *);"),
            format!("void {w}({t} x, {t} *s, {t} *c) {{ {scalar}(x, s, c); }}"),
        )),
        _ => Err(format!("{SONAME}: {scalar} with parameters `{params}` has no known scalar shape")),
    }
}

/// The C source of the library: per scalar function its prototype and the `simd` wrapper; per
/// export a `.symver` naming the wrapper's clone (`shared`), or a global alias of it (the archive).
pub fn c_source(plan: &Plan, shared: bool) -> Result<String, String> {
    let mut s = format!("/* @generated by `cargo xtask so` from spec/abi/versions.tsv: {SONAME} (SYS-LIBMVEC). */\n");
    // scalar -> (params, element type, exports)
    let mut functions: BTreeMap<String, (String, &'static str, Vec<&Export>)> = BTreeMap::new();
    for e in &plan.exports {
        let v = parse(&e.name).ok_or_else(|| format!("{SONAME}: export {} is not an unmasked vector function ABI name", e.name))?;
        let t = element(&v)?;
        let entry = functions.entry(v.scalar.clone()).or_insert_with(|| (v.params.clone(), t, Vec::new()));
        if entry.0 != v.params || entry.1 != t {
            return Err(format!("{SONAME}: {} disagrees with another export of {} (parameters `{}`, {})", e.name, v.scalar, entry.0, entry.1));
        }
        entry.2.push(e);
    }
    for (scalar, (params, t, exports)) in &functions {
        let (proto, body) = shape(scalar, params, t)?;
        s.push_str(&format!("{proto}\n__attribute__((simd(\"notinbranch\"))) {body}\n"));
        for e in exports {
            let v = parse(&e.name).ok_or_else(|| format!("{SONAME}: {}", e.name))?;
            let clone = format!("_ZGV{}N{}{}_{WRAPPER}{scalar}", v.isa, v.lanes, v.params);
            if shared {
                let at = if e.default { "@@" } else { "@" };
                s.push_str(&format!("__asm__(\".symver {clone}, {}{at}{}, remove\");\n", e.name, e.version));
            } else {
                s.push_str(&format!("__asm__(\".globl {n}\\n.type {n}, %function\\n.set {n}, {clone}\");\n", n = e.name));
            }
        }
    }
    Ok(s)
}

/// The version script: every node with its exports, each node extending the one before; the first
/// makes everything else (the wrappers) local. (A `local: *` alone would also hide the names the
/// `.symver`s define, so each export is named.)
pub fn version_script(plan: &Plan) -> String {
    let mut s = format!("/* @generated by `cargo xtask so` from spec/abi/versions.tsv: {SONAME} (SYS-LIBMVEC). */\n");
    let mut prev: Option<&String> = None;
    for v in &plan.nodes {
        let names: String = plan.exports.iter().filter(|e| &e.version == v).map(|e| format!(" {};", e.name)).collect();
        let local = if prev.is_none() { " local: *;" } else { "" };
        s.push_str(&format!("{v} {{ global:{names}{local} }}"));
        match prev {
            Some(p) => s.push_str(&format!(" {p};\n")),
            None => s.push_str(";\n"),
        }
        prev = Some(v);
    }
    s
}

/// The compiler flags of both forms: no builtin expansion (each lane is the named scalar call),
/// no stack protector (nothing the wrappers write is a buffer).
const CFLAGS: [&str; 4] = ["-O2", "-fPIC", "-fno-builtin", "-fno-stack-protector"];

/// Build `libmvec.so.1` (linked against `release/libm.so.6` and `release/libc.so.6`), check it,
/// build `libmvec.a` and the `libmvec.so` link. One summary line.
pub fn build(root: &Path, release: &Path, work: &Path, cc: &str, ar: &str) -> Result<String, String> {
    let table = fs::read_to_string(root.join("spec/abi/versions.tsv")).map_err(|e| format!("versions.tsv: {e}"))?;
    fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let plan = crate::placeholders::plan(&table, SONAME)?;
    let c = work.join(format!("{SONAME}.c"));
    let map = work.join(format!("{SONAME}.map"));
    fs::write(&c, c_source(&plan, true)?).map_err(|e| e.to_string())?;
    fs::write(&map, version_script(&plan)).map_err(|e| e.to_string())?;
    let dst = release.join(SONAME);
    let tmp = crate::so::tmp_name(&dst);
    crate::so::run(Command::new(cc)
        .args(["-shared", "-nostdlib"])
        .args(CFLAGS)
        .args(["-Wl,-z,now", "-Wl,-z,relro", "-Wl,--hash-style=gnu"])
        .arg(format!("-Wl,-soname,{SONAME}"))
        .arg(format!("-Wl,--version-script={}", map.display()))
        .arg(&c)
        .arg("-Wl,--no-as-needed")
        .arg(release.join("libm.so.6"))
        .arg(release.join("libc.so.6"))
        .arg("-o")
        .arg(&tmp))?;
    let dynamic = crate::placeholders::readelf(&["-d"], &tmp)?;
    let mut errors = crate::placeholders::product_errors(&table, SONAME, &dynamic, &crate::placeholders::readelf(&["-V"], &tmp)?, &crate::placeholders::readelf(&["--dyn-syms", "-W"], &tmp)?);
    if !dynamic.contains("Shared library: [libm.so.6]") {
        errors.push(format!("{SONAME}: no DT_NEEDED libm.so.6 (each lane calls libm's scalar function)"));
    }
    if !errors.is_empty() {
        let _ = fs::remove_file(&tmp);
        return Err(format!("{SONAME} differs from spec/abi/versions.tsv:\n{}", errors.join("\n")));
    }
    crate::so::publish(&tmp, &dst)?;
    // The archive: the same wrappers, each export a global alias of its clone.
    let ac = work.join(format!("{ARCHIVE}.c"));
    let o = work.join("libmvec.o");
    fs::write(&ac, c_source(&plan, false)?).map_err(|e| e.to_string())?;
    crate::so::run(Command::new(cc).arg("-c").args(CFLAGS).arg(&ac).arg("-o").arg(&o))?;
    let adst = release.join(ARCHIVE);
    let atmp = crate::so::tmp_name(&adst);
    let _ = fs::remove_file(&atmp);
    crate::so::run(Command::new(ar).arg("rcD").arg(&atmp).arg(&o))?;
    crate::so::publish(&atmp, &adst)?;
    let link = release.join(DEV_LINK);
    let _ = fs::remove_file(&link);
    std::os::unix::fs::symlink(SONAME, &link).map_err(|e| format!("{}: {e}", link.display()))?;
    Ok(format!("{SONAME} ({} nodes, {} exports, checked against spec/abi/versions.tsv), {ARCHIVE}, {DEV_LINK}", plan.nodes.len(), plan.exports.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // no spec clause: decoding the vector function ABI names (SYS-LIBMVEC).
    #[test]
    fn names_decode_to_isa_lanes_parameters_and_scalar() {
        assert_eq!(parse("_ZGVbN2v_sin"), Some(Variant { isa: 'b', lanes: 2, params: "v".into(), scalar: "sin".into() }));
        assert_eq!(parse("_ZGVeN16vvv_sincosf"), Some(Variant { isa: 'e', lanes: 16, params: "vvv".into(), scalar: "sincosf".into() }));
        assert_eq!(parse("_ZGVbM2v_sin"), None, "masked variants are not exports");
        assert_eq!(parse("_ZGVxN2v_sin"), None);
        assert_eq!(parse("_ZGVbN2vl8_sin"), None);
        assert_eq!(parse("sin"), None);
    }

    // no spec clause: the element type is the register width over the lanes (SYS-LIBMVEC).
    #[test]
    fn the_element_type_follows_width_over_lanes() {
        let t = |n: &str| parse(n).map(|v| element(&v));
        assert_eq!(t("_ZGVbN2v_exp"), Some(Ok("double")));
        assert_eq!(t("_ZGVcN8v_expf"), Some(Ok("float")));
        assert_eq!(t("_ZGVdN4vv_pow"), Some(Ok("double")));
        assert_eq!(t("_ZGVeN16v_logf"), Some(Ok("float")));
        assert!(matches!(t("_ZGVbN8v_exp"), Some(Err(_))), "16-bit lanes are neither type");
    }

    fn plan_of(names: &[(&str, &str)]) -> Plan {
        Plan {
            soname: SONAME.into(),
            nodes: vec!["GLIBC_2.22".into(), "GLIBC_2.35".into()],
            exports: names.iter().map(|(n, v)| Export { name: (*n).into(), version: (*v).into(), default: true }).collect(),
        }
    }

    // no spec clause: one wrapper per scalar function, one symbol version per export (SYS-LIBMVEC).
    #[test]
    fn the_source_has_one_wrapper_per_function_and_one_symver_per_export() {
        let p = plan_of(&[("_ZGVbN2v_sin", "GLIBC_2.22"), ("_ZGVeN8v_sin", "GLIBC_2.22"), ("_ZGVbN4vvv_sincosf", "GLIBC_2.22"), ("_ZGVdN8vv_atan2f", "GLIBC_2.35")]);
        let s = c_source(&p, true).unwrap();
        assert_eq!(s.matches("simd(\"notinbranch\")").count(), 3);
        assert!(s.contains("double __niji_mvec_sin(double x) { return sin(x); }"));
        assert!(s.contains("void __niji_mvec_sincosf(float x, float *s, float *c) { sincosf(x, s, c); }"));
        assert!(s.contains("float __niji_mvec_atan2f(float x, float y) { return atan2f(x, y); }"));
        assert!(s.contains(".symver _ZGVeN8v___niji_mvec_sin, _ZGVeN8v_sin@@GLIBC_2.22, remove"));
        assert!(s.contains(".symver _ZGVdN8vv___niji_mvec_atan2f, _ZGVdN8vv_atan2f@@GLIBC_2.35, remove"));
        assert_eq!(s.matches(".symver").count(), 4);
        let a = c_source(&p, false).unwrap();
        assert!(a.contains(".set _ZGVbN2v_sin, _ZGVbN2v___niji_mvec_sin"));
        assert!(!a.contains(".symver"));
    }

    // no spec clause: an export the generator cannot give a body is an error, never a silent gap.
    #[test]
    fn unknown_shapes_and_mismatched_exports_are_errors() {
        assert!(c_source(&plan_of(&[("_ZGVbN2vvv_frexp", "GLIBC_2.22")]), true).is_err());
        assert!(c_source(&plan_of(&[("_ZGVbN2v_sin", "GLIBC_2.22"), ("_ZGVbN4v_sin", "GLIBC_2.22")]), true).is_err());
        assert!(c_source(&plan_of(&[("_ZGVbM2v_sin", "GLIBC_2.22")]), true).is_err());
    }

    // no spec clause: the first node hides the wrappers, each later node extends the one before.
    #[test]
    fn the_version_script_chains_the_nodes() {
        let s = version_script(&plan_of(&[("_ZGVbN2v_sin", "GLIBC_2.22"), ("_ZGVbN2v_tan", "GLIBC_2.35")]));
        assert!(s.contains("GLIBC_2.22 { global: _ZGVbN2v_sin; local: *; };\nGLIBC_2.35 { global: _ZGVbN2v_tan; } GLIBC_2.22;\n"), "{s}");
    }
}
