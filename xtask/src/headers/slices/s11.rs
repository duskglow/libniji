//! Slice 11 (plan/headers.md section 4: math): the markers of `complex.h` and `tgmath.h` (SL-HEADERS-COMPLEX-TGMATH).
//! Listed by `xtask/build.rs` (plan/headers.md section 4, "Adding a slice").
//!
//! The C++ standard library wraps both headers: its `<complex.h>` and `<tgmath.h>` come first on a C++ search path
//! and take libniji's header (`#include_next`) only in some modes; in the strict C++11 and later modes they take
//! `<complex>` and `<cmath>` instead. visibility.tsv probes the strict `-std=c++NN` modes, so its C++ rows of these
//! two headers are the C++ library's: in C++98 what the C header shows (the wrapper takes it), in C++11 and later
//! the whole of `<complex>` and `<cmath>` (about 2,700 names, none of them the C header's). What the C header itself
//! declares in C++ does not depend on the C++ standard (the wrapper decides whether it is read at all), so here a
//! row's configurations are its C configurations plus, when C++98 shows it, every C++ configuration; a row only the
//! later C++ modes show is the C++ library's and is not emitted.
//!
//! * `@@emit-c`: the header's table rows under those configurations (functions, macros with a macros.tsv value,
//!   typedefs), skipping what the included templates cover, the template's `@@owns-macro` names and reserved names,
//!   as `@@emit` does.
//! * `@@if-c NAME [KIND]`: `#if` and the condition of NAME's macro row (or KIND row) under those configurations (the template defines a
//!   function-like macro the tables have no value for: `CMPLX`, the type-generic macros).
//! * `@@if-probe NAME [FUNCTION]`: `#if` and the condition under which spec/abi/typegeneric.tsv (the type-generic
//!   probe, spec/tools/tgmath_probe.py) shows NAME defined, for the macros visibility.tsv has no row of
//!   (`_Complex_I`, the `CMPLX` family); with FUNCTION, that function's visibility row gives the condition and the
//!   probe is checked against it.
use super::super::cond::{condition, mask_of};
use super::super::{omitted, tsv, Gen, Marker, Slice};

pub(super) const SLICE: Slice = Slice {
    markers: &[Marker { name: "@@emit-c", run: emit_c }, Marker { name: "@@if-c", run: if_c }, Marker { name: "@@if-probe", run: if_probe }],
    inline: &[],
    linked_elsewhere: &[],
};

/// (every configuration, the C++ ones, C++98) as masks over ftm-configs.tsv.
fn masks(g: &mut Gen<'_>) -> Option<(u64, u64, u64)> {
    let rows = match tsv(g.root, "spec/abi/ftm-configs.tsv") {
        Ok(r) => r,
        Err(e) => {
            g.errors.push(format!("{}: slice 11 markers: {e}", g.header));
            return None;
        }
    };
    let all = (1u64 << g.t.lv.ids.len()) - 1;
    let (mut cxx, mut cxx98) = (0u64, 0u64);
    for (i, r) in rows.iter().enumerate() {
        if r["language"] == "c++" {
            cxx |= 1 << i;
            if r["id"] == "cxx98" {
                cxx98 |= 1 << i;
            }
        }
    }
    if cxx98 == 0 {
        g.errors.push(format!("{}: slice 11 markers: no cxx98 configuration in ftm-configs.tsv", g.header));
        return None;
    }
    Some((all, cxx, cxx98))
}

/// A visibility.tsv `configs` cell as the C header's own configurations (the module text).
fn c_header_mask(configs: &str, g: &Gen<'_>, (_, cxx, cxx98): (u64, u64, u64)) -> u64 {
    let m = mask_of(&g.t.lv, configs).unwrap_or(0);
    let c = m & !cxx;
    if m & cxx98 != 0 { c | cxx } else { c }
}

/// The condition of (name, kind) in this header under the C header's configurations, or an error.
fn c_header_cond(g: &mut Gen<'_>, name: &str, kind: &str, m: (u64, u64, u64)) -> String {
    let Some(r) = g.t.vis_row(&g.header, name, kind) else {
        g.errors.push(format!("{}: {name}: no {kind} row in visibility.tsv", g.header));
        return "0".into();
    };
    let target = c_header_mask(&r["configs"], g, m);
    let care = m.0 & !g.dont_care(name, kind);
    let fortify = g.fortify_mask();
    match condition(&g.t.lv, target, care).or_else(|| condition(&g.t.lv, target & !fortify, care & !fortify)) {
        Some(c) => c,
        None => {
            g.errors.push(format!("{}: {name}: no condition over the levels selects the C header's configurations", g.header));
            "0".into()
        }
    }
}

/// `@@if-probe NAME`: `#if` and the condition under which spec/abi/typegeneric.tsv shows NAME (a `complex.h` row
/// whose expression is NAME or `NAME(...)`) defined: the probe's configurations where it is not `undefined`, over
/// the probe's configurations that ftm-configs.tsv has (C++98 standing for every C++ configuration, as above).
/// For the macros the visibility table has no row of (`_Complex_I`, the `CMPLX` family).
fn if_probe(g: &mut Gen<'_>, args: &[&str]) {
    let Some(m) = masks(g) else { return };
    let name = args.first().copied().unwrap_or("");
    let rows = match tsv(g.root, "spec/abi/typegeneric.tsv") {
        Ok(r) => r,
        Err(e) => {
            g.errors.push(format!("{}: @@if-probe {name}: {e}", g.header));
            return;
        }
    };
    let (mut target, mut care) = (0u64, 0u64);
    for r in &rows {
        let e = &r["expr"];
        if r["header"] != g.header || !(e == name || e.strip_prefix(name).is_some_and(|s| s.starts_with('('))) {
            continue;
        }
        let Some(i) = g.t.lv.ids.iter().position(|id| *id == r["config"]) else { continue };
        let bit = if (1u64 << i) & m.2 != 0 { m.1 } else { 1u64 << i };
        care |= bit;
        if r["type"] != "undefined" {
            target |= bit;
        }
    }
    if care == 0 {
        g.errors.push(format!("{}: @@if-probe {name}: no row in spec/abi/typegeneric.tsv", g.header));
        g.out.push("#if 0".into());
        return;
    }
    // `@@if-probe NAME FUNCTION`: NAME is visible with FUNCTION (the CMPLXF32 family with the _Float32 functions,
    // TS 18661-3); FUNCTION's visibility row gives the condition over every configuration, where the probe's few
    // configurations cannot tell the feature levels apart, and the probe must agree with it.
    if let Some(&proxy) = args.get(1) {
        let Some(r) = g.t.vis_row(&g.header, proxy, "function") else {
            g.errors.push(format!("{}: @@if-probe {name}: no function row of {proxy} in visibility.tsv", g.header));
            g.out.push("#if 0".into());
            return;
        };
        let have = c_header_mask(&r["configs"], g, m);
        if have & care != target {
            g.errors.push(format!("{}: @@if-probe {name}: spec/abi/typegeneric.tsv disagrees with {proxy}'s visibility", g.header));
        }
        let c = c_header_cond(g, proxy, "function", m);
        g.out.push(format!("#if {c}"));
        return;
    }
    match condition(&g.t.lv, target, care) {
        Some(c) => g.out.push(format!("#if {c}")),
        None => {
            g.errors.push(format!("{}: @@if-probe {name}: no condition over the levels selects the probe's configurations", g.header));
            g.out.push("#if 0".into());
        }
    }
}

fn if_c(g: &mut Gen<'_>, args: &[&str]) {
    let Some(m) = masks(g) else { return };
    let name = args.first().copied().unwrap_or("");
    let kind = args.get(1).copied().unwrap_or("macro");
    let c = c_header_cond(g, name, kind, m);
    g.out.push(format!("#if {c}"));
}

fn emit_c(g: &mut Gen<'_>, _args: &[&str]) {
    let Some(m) = masks(g) else { return };
    let rows: Vec<(String, String, String)> = g.t.header_rows(&g.header).map(|r| (r["name"].clone(), r["kind"].clone(), r["configs"].clone())).collect();
    let mut items: Vec<(usize, String, String, String)> = Vec::new();
    for (name, kind, configs) in rows {
        let want = c_header_mask(&configs, g, m) & !g.dont_care(&name, &kind);
        let generic = kind == "function" && g.t.providers.get(&name).is_some_and(|p| p == "generic");
        if want == 0 || kind == "include-error" || generic || (kind == "macro" && g.owned_macros.contains(&name)) || omitted(&g.t.reserved, &name) {
            continue;
        }
        // Covered by an included template where it shows the name in the C header's configurations too.
        if let Some(&have) = g.covered.get(&(name.clone(), kind.clone())) {
            let have = if have & m.2 != 0 { have | m.1 } else { have };
            if want & !have == 0 {
                continue;
            }
        }
        let rank = ["typedef", "function", "macro"].iter().position(|k| *k == kind);
        let Some(rank) = rank else {
            g.errors.push(format!("{}: {name}: kind {kind} is not emitted by @@emit-c", g.header));
            continue;
        };
        if !g.emitted.insert(format!("{kind} {name}")) {
            g.errors.push(format!("{}: {kind} {name} emitted twice", g.header));
            continue;
        }
        let c = c_header_cond(g, &name, &kind, m);
        items.push((rank, c, name, kind));
    }
    items.sort();
    let mut open: Option<String> = None;
    for (_, c, name, kind) in items {
        if open.as_ref() != Some(&c) {
            if open.is_some() {
                g.out.push("#endif".into());
            }
            g.out.push(format!("#if {c}"));
            open = Some(c);
        }
        g.item(&name, &kind);
    }
    if open.is_some() {
        g.out.push("#endif".into());
    }
}
