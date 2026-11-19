//! Slice 12 (plan/headers.md section 4: network core and name services): the markers of the sixteen network headers.
//! Listed by `xtask/build.rs` (plan/headers.md section 4, "Adding a slice").
//!
//! * `@@needs H ...`: the headers H (slices not yet generated) supply the names `visibility.tsv` lists for them; the
//!   include is `__has_include`-guarded while H has no template.
//! * `@@alias NAME OF`: `#define NAME OF`, spelled as the macro it equals (linux/in.h spells `IP_RECVRETOPTS` as `IP_RETOPTS`,
//!   so a later kernel definition repeats the replacement list token for token, C 6.10.3p2); refused when `macros.tsv`
//!   gives the two different types or values. The template also names NAME in `@@owns-macro`.
//! * `@@needs-if NAME H`: `#include <H>` where NAME is visible; H supplies the names `visibility.tsv` lists for it.
use super::super::{mask_of, Gen, Marker, Slice};

pub(super) const SLICE: Slice = Slice {
    markers: &[Marker { name: "@@needs", run: needs }, Marker { name: "@@needs-if", run: needs_if }, Marker { name: "@@alias", run: alias }],
    inline: &[],
    linked_elsewhere: &[],
};

/// The names `visibility.tsv` lists for `header` count as covered here, in the configurations of their rows.
fn supply(g: &mut Gen<'_>, header: &str) {
    for r in g.t.vis.iter().filter(|r| r["header"] == header) {
        *g.covered.entry((r["name"].clone(), r["kind"].clone())).or_insert(0) |= mask_of(&g.t.lv, &r["configs"]).unwrap_or(u64::MAX);
    }
}

fn needs(g: &mut Gen<'_>, args: &[&str]) {
    for n in args {
        supply(g, n);
        if g.root.join("headers").join(format!("{n}.in")).is_file() {
            g.out.push(format!("#include <{n}>"));
        } else {
            g.out.extend([format!("#if __has_include(<{n}>)"), format!("# include <{n}>"), "#endif".into()]);
        }
    }
}

fn needs_if(g: &mut Gen<'_>, args: &[&str]) {
    let (n, h) = (args.first().copied().unwrap_or(""), args.get(1).copied().unwrap_or(""));
    supply(g, h);
    let c = g.cond(n, None);
    g.out.extend([format!("#if {c}"), format!("# include <{h}>"), "#endif".into()]);
}

fn alias(g: &mut Gen<'_>, args: &[&str]) {
    let (n, of) = (args.first().copied().unwrap_or(""), args.get(1).copied().unwrap_or(""));
    if g.t.macros.get(n).is_none() || g.t.macros.get(n) != g.t.macros.get(of) {
        g.errors.push(format!("headers/{}.in: @@alias {n} {of}: macros.tsv gives the two different types or values", g.header));
    }
    g.out.extend([format!("#ifndef {n}"), format!("# define {n} {of}"), "#endif".into()]);
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    // no spec clause: plan/headers.md section 3 (P1 member layouts and constants, P5 hygiene of the sixteen slice 12
    // headers, P8 with the build host's kernel UAPI headers in the libc-first order, which header_check.py skips where
    // the host has none; P10 against the build host's own headers, run only where that input exists, and the skip is
    // printed).
    #[test]
    fn network_headers_pass_header_check() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let run = |script: &str, args: &[&str]| {
            let out = std::process::Command::new("python3").arg("-I").arg(root.join(script)).args(args).output().expect("python3");
            assert!(out.status.success(), "{script} {args:?}:\n{}", String::from_utf8_lossy(&out.stdout));
        };
        run("spec/tools/header_check.py", &[]);
        if Path::new("/usr/include/netinet/in.h").is_file() {
            run("spec/tools/header_checks/s12.py", &[]);
        } else {
            println!("skip P10: the build host has no reference <netinet/in.h>");
        }
    }
}
