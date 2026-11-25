//! Slice 15 (plan/headers.md section 4: administration, devices, legacy). Listed by `xtask/build.rs`.
//!
//! * `@@replace-macros`: before `@@emit`, an `#undef` of each name this header declares as an enumerator of an
//!   enumeration without a name (enumerators.tsv `anon FIRST`) and as a macro naming that enumerator itself
//!   (enum-macros.tsv `MS_RDONLY MS_RDONLY`), under the enumerator's condition. A kernel header the template
//!   includes first defines those names as macros of their values (`<linux/mount.h>`'s `MS_*`); `@@emit` then
//!   declares the enumeration with every member and the macros naming them, so the names have the enumeration's
//!   type in C++ in either include order: a kernel header included after this one is not read again (its include
//!   guard). HEADERS-ENUM-FOLLOWUPS.
use super::super::{Gen, Marker, Slice};

pub(super) const SLICE: Slice = Slice { markers: &[Marker { name: "@@replace-macros", run: replace_macros }], inline: &[], linked_elsewhere: &[] };

fn replace_macros(g: &mut Gen<'_>, _args: &[&str]) {
    let t = g.t;
    let names: Vec<String> = t
        .header_rows(&g.header)
        .filter(|r| r["kind"] == "enumerator")
        .map(|r| r["name"].clone())
        .filter(|n| g.anon_enum(n).is_some() && t.enum_macros.get(n).is_some_and(|(e, _)| e == n))
        .collect();
    if names.is_empty() {
        g.errors.push(format!("{}: @@replace-macros: no enumerator of an unnamed enumeration names itself as a macro here", g.header));
        return;
    }
    let mut open: Option<String> = None;
    for n in names {
        let c = g.cond(&n, Some("enumerator"));
        if c == "0" {
            continue;
        }
        if open.as_ref() != Some(&c) {
            if open.is_some() {
                g.out.push("#endif".into());
            }
            g.out.push(format!("#if {c}"));
            open = Some(c);
        }
        g.out.push(format!("# undef {n}"));
    }
    if open.is_some() {
        g.out.push("#endif".into());
    }
}
