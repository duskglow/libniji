//! Slice 5 (plan/headers.md section 4: generator and base layer): the markers of `stdint.h`, `stdc-predef.h` and
//! `gnu/stubs.h`. Listed by `xtask/build.rs` (plan/headers.md section 4, "Adding a slice").
//!
//! * `@@intc`: the `INTn_C`/`UINTn_C`/`INTMAX_C`/`UINTMAX_C` macros, derived from the `typedefs.tsv` rows of the
//!   least-width types (C 7.20.4); a missing row is an error.
//! * `@@iso10646` (inline): the `__STDC_ISO_10646__` value of the Unicode tables in `spec/unicode/`.
//! * `@@stubs`: `__stub_<f>` for each function `spec/abi/stubs.tsv` lists as linkable but ENOSYS-only.
use super::super::{tsv, Gen, Inline, Marker, Slice};
use std::fs;
use std::path::Path;

pub(super) const SLICE: Slice = Slice {
    markers: &[Marker { name: "@@intc", run: intc }, Marker { name: "@@stubs", run: stubs }],
    inline: &[Inline { token: "@@iso10646", value: iso10646 }],
    linked_elsewhere: &[],
};

/// C 7.20.4: `INTn_C(c)` has the promoted type of `int_leastn_t`.
fn intc(g: &mut Gen<'_>, _args: &[&str]) {
    let mut v: Vec<(String, String)> = Vec::new();
    for n in ["8", "16", "32", "64"] {
        v.push((format!("INT{n}_C"), format!("int_least{n}_t")));
        v.push((format!("UINT{n}_C"), format!("uint_least{n}_t")));
    }
    v.push(("INTMAX_C".into(), "intmax_t".into()));
    v.push(("UINTMAX_C".into(), "uintmax_t".into()));
    for (m, ty) in v {
        let Some(c) = g.cxx(&ty) else {
            g.errors.push(format!("{}: {m}: typedef {ty} not in typedefs.tsv", g.header));
            continue;
        };
        let suffix = match c.as_str() {
            "long" => "L",
            "unsigned long" => "UL",
            "long long" => "LL",
            "unsigned long long" => "ULL",
            "unsigned int" => "U",
            _ => "",
        };
        let body = if suffix.is_empty() { "c".to_string() } else { format!("c ## {suffix}") };
        g.out.extend([format!("# ifndef {m}"), format!("#  define {m}(c) {body}"), "# endif".into()]);
    }
}

fn stubs(g: &mut Gen<'_>, _args: &[&str]) {
    match tsv(g.root, "spec/abi/stubs.tsv") {
        Ok(rows) => {
            for f in rows.iter().filter(|r| r["link"] == "default").map(|r| &r["function"]) {
                g.out.extend([format!("#ifndef __stub_{f}"), format!("# define __stub_{f}"), "#endif".into()]);
            }
        }
        Err(e) => g.errors.push(format!("{}: @@stubs: {e}", g.header)),
    }
}

fn iso10646(root: &Path) -> Result<String, String> {
    // The ISO/IEC 10646 edition that matches each Unicode version (yyyymm of the Unicode release).
    let editions = tsv(root, "headers/iso10646.tsv")?;
    let dirs: Vec<String> = fs::read_dir(root.join("spec/unicode")).map_err(|e| e.to_string())?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    if dirs.len() != 1 {
        return Err(format!("spec/unicode holds {} versions, expected one", dirs.len()));
    }
    editions.iter().find(|r| r["unicode"] == dirs[0]).map(|r| format!("{}L", r["yyyymm"])).ok_or(format!("headers/iso10646.tsv: no row for Unicode {}", dirs[0]))
}
