//! Shape and coverage check of the call-site and exception-specification tables (Appendix C.13, slice 4 of
//! plan/headers.md).
//!
//! `spec/abi/callsites.tsv` gives, per function (or C.9 macro) and mode, the symbol a compiled program references
//! and the configurations it does so in; `spec/abi/noexcept.tsv` gives each `prototypes.tsv` function's C++17
//! exception specification and variadic form and its C++98 `throw()`. Both are written by
//! `spec/tools/callsite_probe.py` on the reference system. The check (part of `cargo xtask static`) fails when a
//! table's shape is wrong (columns, modes, values, a header outside the public inventory, a configuration list
//! not in the probe's spelling over the mode's configurations, unsorted or repeated rows, one configuration
//! given two symbols) or when a `__isoc*` or `*_chk` export of `versions.tsv` is reached by no row and not
//! exempted with a reason, or a `prototypes.tsv` function has no `noexcept.tsv` row.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::visibility::{configs_form, is_ident, public_headers, read};

const CALLSITES: &str = "spec/abi/callsites.tsv";
const NOEXCEPT: &str = "spec/abi/noexcept.tsv";
const SITES_HEAD: &str = "name\theader\tmode\tconfigs\tsymbol";
const NX_HEAD: &str = "name\theader\tcxx17\tvariadic\tcxx98";
/// The suffixes the probe adds to each fortify configuration in the call modes (`spec/tools/callsite_probe.py`).
const OVERLAYS: [&str; 3] = ["", "+gnu", "+gnu64"];
/// The C.9 macros probed in mode `macro`.
const MACROS: [&str; 3] = ["FD_CLR", "FD_ISSET", "FD_SET"];
/// Call-site exports no probed (function, configuration) reaches, with the reason.
const EXEMPT: [(&str, &str); 2] = [
    ("__gets_chk", "`gets` has no prototypes.tsv row: the reference headers declare it in no probed configuration"),
    ("__vprintf_chk", "the fortified `vprintf` call references `__vfprintf_chk` and `stdout` instead (callsites.tsv)"),
];

/// The configuration ids of each mode: `addr` the non-fortify configurations, `call` and `macro` each fortify
/// configuration followed by its overlays.
fn universes(ids: &[String]) -> (Vec<String>, Vec<String>) {
    let addr = ids.iter().filter(|i| !i.starts_with("fortify")).cloned().collect();
    let call = ids.iter().filter(|i| i.starts_with("fortify")).flat_map(|i| OVERLAYS.iter().map(move |o| format!("{i}{o}"))).collect();
    (addr, call)
}

/// The set a configuration list names over `ids`, or the problem with it.
fn parse_set(cfgs: &str, ids: &[String]) -> Result<Vec<bool>, String> {
    if cfgs == "*" {
        return Ok(vec![true; ids.len()]);
    }
    let (negated, list) = cfgs.strip_prefix('!').map_or((false, cfgs), |l| (true, l));
    let mut picked = vec![false; ids.len()];
    let mut last: Option<usize> = None;
    for c in list.split(',') {
        match ids.iter().position(|i| i == c) {
            Some(p) if last.is_none_or(|l| p > l) => {
                last = Some(p);
                picked[p] = true;
            }
            Some(_) => return Err(format!("configuration `{c}` out of order or repeated")),
            None => return Err(format!("unknown configuration `{c}` for this mode")),
        }
    }
    let set: Vec<bool> = picked.iter().map(|&p| p != negated).collect();
    match configs_form(ids, &set) {
        None => Err("reached in no configuration; drop the row".to_string()),
        Some(form) if form != cfgs => Err(format!("configurations `{cfgs}` must be written `{form}`")),
        Some(_) => Ok(set),
    }
}

fn symbol_ok(sym: &str) -> bool {
    if matches!(sym, "-" | "!error" | "local") {
        return true;
    }
    let parts: Vec<&str> = sym.split(',').collect();
    parts.iter().all(|p| is_ident(p) || *p == "local") && parts.windows(2).all(|w| w[0] < w[1])
}

/// Problems of the two tables. `ids` are ftm-configs.tsv's ids, `functions` the prototypes.tsv names, `exports`
/// the `__isoc*` and `*_chk` names exported at a default version.
fn check_tables(sites: &str, nx: &str, ids: &[String], public: &BTreeSet<String>, functions: &BTreeSet<String>, exports: &BTreeSet<String>) -> Vec<String> {
    let mut errors = Vec::new();
    let (addr, call) = universes(ids);
    let mut lines = sites.lines();
    if lines.next() != Some(SITES_HEAD) {
        errors.push(format!("{CALLSITES}: the first line must be `{}`", SITES_HEAD.replace('\t', "<TAB>")));
    }
    let mut prev: Option<(String, String, String)> = None;
    let mut taken: BTreeMap<(String, String), Vec<bool>> = BTreeMap::new();
    let mut reached: BTreeSet<String> = BTreeSet::new();
    for (n, line) in lines.enumerate() {
        let at = format!("{CALLSITES}:{}", n + 2);
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 5 {
            errors.push(format!("{at}: {} fields, expected 5", f.len()));
            continue;
        }
        let (name, header, mode, cfgs, sym) = (f[0], f[1], f[2], f[3], f[4]);
        let known = if mode == "macro" { MACROS.contains(&name) } else { functions.contains(name) };
        if !known {
            errors.push(format!("{at}: `{name}` is not a {} name", if mode == "macro" { "probed macro" } else { "prototypes.tsv" }));
        }
        if !public.contains(header) {
            errors.push(format!("{at}: `{header}` is not a public header"));
        }
        let universe = match mode {
            "addr" => &addr,
            "call" | "macro" => &call,
            _ => {
                errors.push(format!("{at}: unknown mode `{mode}`"));
                continue;
            }
        };
        if !symbol_ok(sym) {
            errors.push(format!("{at}: symbol `{sym}` is not `-`, `!error`, or sorted comma-joined names"));
        }
        match parse_set(cfgs, universe) {
            Err(e) => errors.push(format!("{at}: {e}")),
            Ok(set) => {
                let seen = taken.entry((name.to_string(), mode.to_string())).or_insert_with(|| vec![false; set.len()]);
                if seen.iter().zip(&set).any(|(a, b)| *a && *b) {
                    errors.push(format!("{at}: a configuration of `{name}` ({mode}) already has a symbol"));
                }
                seen.iter_mut().zip(&set).for_each(|(a, b)| *a |= *b);
            }
        }
        let key = (name.to_string(), mode.to_string(), sym.to_string());
        if prev.as_ref().is_some_and(|p| *p >= key) {
            errors.push(format!("{at}: row out of order or repeated (sorted by name, mode, symbol)"));
        }
        prev = Some(key);
        reached.extend(sym.split(',').map(str::to_string));
    }
    for e in exports {
        if !reached.contains(e) && !EXEMPT.iter().any(|(x, _)| x == e) {
            errors.push(format!("{CALLSITES}: export `{e}` is reached by no (function, configuration)"));
        }
    }
    let mut lines = nx.lines();
    if lines.next() != Some(NX_HEAD) {
        errors.push(format!("{NOEXCEPT}: the first line must be `{}`", NX_HEAD.replace('\t', "<TAB>")));
    }
    let mut rows: Vec<String> = Vec::new();
    for (n, line) in lines.enumerate() {
        let at = format!("{NOEXCEPT}:{}", n + 2);
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 5 {
            errors.push(format!("{at}: {} fields, expected 5", f.len()));
            continue;
        }
        let (name, header, c17, var, c98) = (f[0], f[1], f[2], f[3], f[4]);
        if header != "-" && !public.contains(header) {
            errors.push(format!("{at}: `{header}` is not a public header"));
        }
        if !["noexcept", "none", "error", "-"].contains(&c17) || !["throw()", "none", "unknown", "error", "-"].contains(&c98) {
            errors.push(format!("{at}: cxx17 `{c17}` or cxx98 `{c98}` is not a known value"));
        }
        let typed = c17 == "noexcept" || c17 == "none";
        if (typed && var != "yes" && var != "no") || (!typed && var != "-") {
            errors.push(format!("{at}: variadic `{var}` does not fit cxx17 `{c17}`"));
        }
        if header == "-" && (c17 != "-" || c98 != "-") {
            errors.push(format!("{at}: a function in no header has no C++ result"));
        }
        if rows.last().is_some_and(|p| p.as_str() >= name) {
            errors.push(format!("{at}: row out of order or repeated"));
        }
        rows.push(name.to_string());
    }
    let have: BTreeSet<String> = rows.into_iter().collect();
    for f in functions.difference(&have) {
        errors.push(format!("{NOEXCEPT}: prototypes.tsv function `{f}` has no row"));
    }
    for f in have.difference(functions) {
        errors.push(format!("{NOEXCEPT}: `{f}` is no prototypes.tsv function"));
    }
    errors
}

/// `cargo xtask static` entry.
pub fn check(root: &Path) -> Result<(), String> {
    let configs = read(root, "spec/abi/ftm-configs.tsv")?;
    let ids: Vec<String> = configs.lines().skip(1).filter_map(|l| l.split('\t').next()).map(str::to_string).collect();
    let functions: BTreeSet<String> = read(root, "spec/abi/prototypes.tsv")?.lines().skip(1).filter_map(|l| l.split('\t').next()).map(str::to_string).collect();
    let exports: BTreeSet<String> = read(root, "spec/abi/versions.tsv")?
        .lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() > 3 && f[0] == "libc.so.6" && f[3] == "default" && (f[1].starts_with("__isoc") || f[1].ends_with("_chk"))).then(|| f[1].to_string())
        })
        .collect();
    let errors = check_tables(&read(root, CALLSITES)?, &read(root, NOEXCEPT)?, &ids, &public_headers(root)?, &functions, &exports);
    if errors.is_empty() { Ok(()) } else { Err(format!("callsites: {} problem(s):\n{}", errors.len(), errors.join("\n"))) }
}

#[cfg(test)]
mod tests {
    //! no spec clause: Appendix C.13's table format (slice 4 of plan/headers.md); each test breaks one rule.
    use super::*;

    const SITES: &str = "name\theader\tmode\tconfigs\tsymbol\nFD_SET\tsys/select.h\tmacro\t*\t__fdelt_chk\nscanf\tstdio.h\taddr\tc2x\t__isoc23_scanf\nscanf\tstdio.h\taddr\t!c2x\t__isoc99_scanf\nscanf\tstdio.h\tcall\t*\t__isoc99_scanf\nstrlen\tstring.h\taddr\tcxx98\t!error\nstrlen\tstring.h\taddr\tc89,c2x\tstrlen\nstrlen\tstring.h\tcall\tfortify1\t-\n";
    const NX: &str = "name\theader\tcxx17\tvariadic\tcxx98\nscanf\tstdio.h\tnone\tyes\tnone\nstrlen\tstring.h\terror\t-\tthrow()\nzork\t-\t-\t-\t-\n";

    fn ids() -> Vec<String> {
        ["c89", "c2x", "fortify1", "cxx98"].iter().map(|s| s.to_string()).collect()
    }

    fn set(xs: &[&str]) -> BTreeSet<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    fn errs(sites: &str, nx: &str, exports: &[&str]) -> Vec<String> {
        check_tables(sites, nx, &ids(), &set(&["stdio.h", "string.h", "sys/select.h"]), &set(&["scanf", "strlen", "zork"]), &set(exports))
    }

    fn fails(sites: &str, nx: &str, needle: &str) {
        let e = errs(sites, nx, &[]);
        assert!(e.iter().any(|x| x.contains(needle)), "expected `{needle}` in {e:?}");
    }

    #[test]
    fn good_tables_pass() {
        assert_eq!(errs(SITES, NX, &["__isoc23_scanf", "__isoc99_scanf", "__fdelt_chk", "__gets_chk"]), Vec::<String>::new());
    }

    #[test]
    fn universes_per_mode() {
        let (a, c) = universes(&ids());
        assert_eq!(a, ["c89", "c2x", "cxx98"]);
        assert_eq!(c, ["fortify1", "fortify1+gnu", "fortify1+gnu64"]);
    }

    #[test]
    fn sites_shape() {
        fails(&SITES.replace("name\theader\tmode", "name\thdr\tmode"), NX, "first line");
        fails(&SITES.replace("\tmacro\t*\t__fdelt_chk", "\tmacro\t*"), NX, "fields, expected 5");
        fails(&SITES.replace("FD_SET\t", "FD_ZERO\t"), NX, "not a probed macro");
        fails(&SITES.replace("scanf\tstdio.h\tcall", "sscanf\tstdio.h\tcall"), NX, "not a prototypes.tsv");
        fails(&SITES.replace("strlen\tstring.h\tcall", "strlen\tbits/string.h\tcall"), NX, "not a public header");
        fails(&SITES.replace("\tcall\t*", "\tcalls\t*"), NX, "unknown mode");
        fails(&SITES.replace("\tfortify1\t-", "\tfortify1\tb,a"), NX, "symbol `b,a`");
        fails(&SITES.replace("\tfortify1\t-", "\tfortify1\tx y"), NX, "symbol `x y`");
    }

    #[test]
    fn sites_configs() {
        fails(&SITES.replace("\tcall\t*", "\tcall\tc89"), NX, "unknown configuration `c89`");
        fails(&SITES.replace("\taddr\tc89,c2x", "\taddr\tfortify1"), NX, "unknown configuration `fortify1`");
        fails(&SITES.replace("\taddr\tc89,c2x", "\taddr\tc2x,c89"), NX, "out of order");
        fails(&SITES.replace("\taddr\tc89,c2x", "\taddr\t!cxx98"), NX, "must be written `c89,c2x`");
        fails(&SITES.replace("\tcall\tfortify1\t", "\tcall\tfortify1,fortify1+gnu,fortify1+gnu64\t"), NX, "must be written `*`");
        fails(&SITES.replace("\taddr\tc89,c2x", "\taddr\t!c89,c2x,cxx98"), NX, "drop the row");
        fails(&SITES.replace("\taddr\tc2x\t", "\taddr\t!cxx98\t"), NX, "already has a symbol");
    }

    #[test]
    fn sites_order_and_coverage() {
        fails(&SITES.replace("scanf\tstdio.h\taddr\tc2x\t__isoc23_scanf\nscanf\tstdio.h\taddr\t!c2x\t__isoc99_scanf\n", "scanf\tstdio.h\taddr\t!c2x\t__isoc99_scanf\nscanf\tstdio.h\taddr\tc2x\t__isoc23_scanf\n"), NX, "out of order");
        let e = errs(SITES, NX, &["__isoc23_scanf", "__isoc99_scanf", "__fdelt_chk", "__memcpy_chk"]);
        assert!(e.iter().any(|x| x.contains("`__memcpy_chk` is reached by no")), "{e:?}");
        // A symbol list reaches each of its names.
        assert_eq!(errs(&SITES.replace("\tfortify1\t-", "\tfortify1\t__memcpy_chk,stdout"), NX, &["__memcpy_chk"]), Vec::<String>::new());
    }

    #[test]
    fn noexcept_rows() {
        fails(SITES, &NX.replace("\tcxx17\t", "\tc++17\t"), "first line");
        fails(SITES, &NX.replace("\tnone\tyes\tnone", "\tnone\tyes"), "fields, expected 5");
        fails(SITES, &NX.replace("\tnone\tyes\tnone", "\tnothrow\tyes\tnone"), "not a known value");
        fails(SITES, &NX.replace("\tnone\tyes\tnone", "\tnone\tyes\tthrow"), "not a known value");
        fails(SITES, &NX.replace("\tnone\tyes\tnone", "\tnone\t-\tnone"), "does not fit");
        fails(SITES, &NX.replace("\terror\t-\t", "\terror\tno\t"), "does not fit");
        fails(SITES, &NX.replace("zork\t-\t-\t-\t-", "zork\t-\t-\t-\tnone"), "no C++ result");
        fails(SITES, &NX.replace("scanf\tstdio.h", "scanf\tbits/stdio.h"), "not a public header");
        fails(SITES, &NX.replace("zork\t-\t-\t-\t-\n", ""), "`zork` has no row");
        fails(SITES, &NX.replace("zork\t", "zzz\t"), "`zzz` is no prototypes.tsv");
        fails(SITES, &NX.replace("scanf\tstdio.h\tnone\tyes\tnone\nstrlen\tstring.h\terror\t-\tthrow()\n", "strlen\tstring.h\terror\t-\tthrow()\nscanf\tstdio.h\tnone\tyes\tnone\n"), "out of order");
    }
}
