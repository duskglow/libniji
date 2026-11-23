//! Slice 16 (plan/headers.md section 4: ELF, dynamic linking, debugging): the markers of `gnu/lib-names.h` and
//! `sys/platform/x86.h`. Listed by `xtask/build.rs` (plan/headers.md section 4, "Adding a slice").
//!
//! * `@@sonames`: one `<STEM>_SO` string macro per shared object `versions.tsv` and `empty-versions.tsv` name
//!   (`libc.so.6` is `LIBC_SO`, `ld-linux-x86-64.so.2` is `LD_LINUX_X86_64_SO`: the part before `.so`, upper case, `-` as `_`).
//! * `@@x86features`: `__NIJI_X86_P_<name>` and `__NIJI_X86_A_<name>` for each row of `spec/abi/x86-features.tsv`
//!   (what `CPU_FEATURE_PRESENT(name)` and `CPU_FEATURE_ACTIVE(name)` expand to, with the template's `__NIJI_X86_TEST`).
use std::collections::BTreeSet;

use super::super::{tsv, Gen, Marker, Slice};

pub(super) const SLICE: Slice = Slice {
    markers: &[Marker { name: "@@sonames", run: sonames }, Marker { name: "@@x86features", run: x86features }],
    inline: &[],
    // The audit entry points of link.h are headers/providers.tsv rows (an audit library defines them), not entries here.
    linked_elsewhere: &[],
};

fn sonames(g: &mut Gen<'_>, _args: &[&str]) {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for f in ["spec/abi/versions.tsv", "spec/abi/empty-versions.tsv"] {
        match tsv(g.root, f) {
            Ok(rows) => names.extend(rows.into_iter().map(|r| r["library"].clone())),
            Err(e) => {
                g.errors.push(format!("{}: @@sonames: {e}", g.header));
                return;
            }
        }
    }
    for so in &names {
        let stem = so.split(".so").next().unwrap_or(so).to_uppercase().replace('-', "_");
        g.out.push(format!("#define {stem}_SO \"{so}\""));
    }
}

fn x86features(g: &mut Gen<'_>, _args: &[&str]) {
    const FILE: &str = "spec/abi/x86-features.tsv";
    let rows = match tsv(g.root, FILE) {
        Ok(r) => r,
        Err(e) => {
            g.errors.push(format!("{}: @@x86features: {e}", g.header));
            return;
        }
    };
    for r in rows {
        let Some(reg) = ["eax", "ebx", "ecx", "edx"].iter().position(|x| *x == r["reg"]) else {
            g.errors.push(format!("{}: {FILE}: register {}", g.header, r["reg"]));
            return;
        };
        let (n, idx, bit) = (&r["name"], &r["index"], &r["bit"]);
        g.out.push(format!("#define __NIJI_X86_P_{n} __NIJI_X86_TEST (cpuid_array, {idx}, {reg}, {bit})"));
        let a = if r["active"] == "yes" { format!("__NIJI_X86_TEST (active_array, {idx}, {reg}, {bit})") } else { "((_Bool) 0)".to_string() };
        g.out.push(format!("#define __NIJI_X86_A_{n} {a}"));
    }
}
