//! LAYOUT-BYTE-BUDGETS (owner ruling R2 of `plan/layout-budget-rethink.md`, D-X.136): the per-story gate on the
//! start-up layout, in bytes and windows read from the link map of the static probe (`page_faults.c` linked as
//! `crates/core/tests/hot_text_products.rs` links it). No program runs: the verdict is a function of the archive, the
//! build's signatures (`hot-text.signatures`) and the probe's link.
//!
//! * **Bins.** Every window bin (`.text.sorted.h9<bin>...`, [`crate::bins::section`]) is measured from its first
//!   member's start to its last member's end. A bin after the head bin must start on a window boundary (else the plan
//!   was not applied), the head bin no more than `head-room` bytes into its window. Budget: the window (64 KiB; the
//!   head bin less its room), or as many windows as its members' bytes need. The planner re-plans every build and
//!   fills a bin only to its capacity, so a bin over budget means the link disagrees with the plan; the per-bin
//!   lines are the headroom report.
//! * **Windows per mode.** What growth moves under bins is which windows a `page_faults` mode touches: a group that
//!   outgrows its bin is placed in another one. A mode's count is the windows its sections span, less the windows its
//!   warm-up (`<mode>.pre`) spans (faulted before its phase). It must equal its budget for the profile
//!   (`windows-<profile>=mode:n,...`): over is a regression, under is a level to ratchet (ALWAYS FORWARD). A red
//!   names, for each window of the mode, the bin, its bytes, budget and headroom, its largest members the mode runs,
//!   and against a base plan (`NIJI_LAYOUT_BASE_BINS`: next's `layout-bins.tsv`) the members new or grown, in bytes.
//! * **Data bins.** The start-up read-only cluster (`.rodata1`) within `rodata1-windows` windows from its window's
//!   start; the written start-up cluster (`.data1`) within `data1-pages-<profile>` pages from its page's start.
//!   And every output section holding start-up data (D-X.138: the objects STARTUP-DATA-WINDOWS counts, `.got`,
//!   RELRO, `.data`, `.bss` and the rest) is a bin of its own: from the page (written) or window (read-only)
//!   boundary below its first start-up object to the end of its last, within `data-bins-<profile>` units. A section
//!   pushed across a boundary by what is linked ahead of it is red here in bytes, as it costs a page there.

use std::collections::{BTreeMap, BTreeSet};

use crate::bins::WARM_UP;

/// The environment variable naming a base plan (another revision's `layout-bins.tsv`) for the member diff.
pub const BASE_ENV: &str = "NIJI_LAYOUT_BASE_BINS";
/// Members named per bin in a red verdict, largest first.
const LARGEST: usize = 5;
/// Members named per bin as new or grown against the base, largest growth first.
const GROWN: usize = 10;

/// A linked input or output section: `[start, end)` and its name.
#[derive(Clone, Debug, PartialEq)]
pub struct Linked {
    pub start: u64,
    pub end: u64,
    pub name: String,
}

/// `.text.sorted.h9<bin, 2 digits><0 member | 1 filler><place, 5 digits>.<f>` -> (bin, filler, f).
pub fn bin_of(name: &str) -> Option<(usize, bool, &str)> {
    let r = name.strip_prefix(".text.sorted.h9")?;
    let b = r.as_bytes();
    if b.len() < 10 || !b[..8].iter().all(u8::is_ascii_digit) || b[8] != b'.' {
        return None;
    }
    let filler = match b[2] {
        b'0' => false,
        b'1' => true,
        _ => return None,
    };
    Some((r[..2].parse().ok()?, filler, &r[9..]))
}

/// One member of a linked bin: its build-stable key, bytes and the counted modes that run it.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub key: String,
    pub bytes: u64,
    pub modes: BTreeSet<String>,
}

/// A bin as linked: the members' `[start, end)` and the members in link order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkedBin {
    pub start: u64,
    pub end: u64,
    pub members: Vec<Member>,
}

impl LinkedBin {
    pub fn bytes(&self) -> u64 {
        self.end - self.start
    }

    /// The bin's budget: whole windows for its members' bytes (at least one), less `room` (the head bin's).
    pub fn budget(&self, window: u64, room: u64) -> u64 {
        let sum: u64 = self.members.iter().map(|m| m.bytes).sum();
        (sum + room).div_ceil(window).max(1) * window - room
    }

    fn modes(&self) -> BTreeSet<String> {
        self.members.iter().flat_map(|m| m.modes.iter().cloned()).collect()
    }
}

/// The link read for the gate: the bins, and each counted mode's windows (window numbers, address / window).
#[derive(Debug, Default, PartialEq)]
pub struct Analysis {
    pub bins: BTreeMap<usize, LinkedBin>,
    pub windows: BTreeMap<String, BTreeSet<u64>>,
}

/// Read the bins and the modes' windows from the linked sections. `key` maps a function name to its signature key
/// ([`crate::hottext::key`]); a member without a signature is an error (the trace and the link disagree).
pub fn analyse(secs: &[Linked], sigs: &BTreeMap<String, BTreeSet<String>>, key: impl Fn(&str) -> String, window: u64) -> Result<Analysis, String> {
    let mut a = Analysis::default();
    let mut touched: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    let mut unsigned = Vec::new();
    for s in secs {
        let Some((n, false, f)) = bin_of(&s.name) else { continue };
        let k = key(f);
        let Some(sig) = sigs.get(&k) else {
            unsigned.push(k);
            continue;
        };
        for m in sig {
            touched.entry(m.clone()).or_default().extend(s.start / window..=(s.end.max(s.start + 1) - 1) / window);
        }
        let b = a.bins.entry(n).or_insert_with(|| LinkedBin { start: s.start, end: s.end, members: Vec::new() });
        b.start = b.start.min(s.start);
        b.end = b.end.max(s.end);
        b.members.push(Member { key: k, bytes: s.end - s.start, modes: crate::bins::counted(sig) });
    }
    if !unsigned.is_empty() {
        return Err(format!("{} bin members have no signature (the trace and the link disagree): {}", unsigned.len(), unsigned.join(" ")));
    }
    if a.bins.is_empty() {
        return Err("no window-bin section in the link map (the plan was not applied?)".into());
    }
    for (m, w) in &touched {
        if m.ends_with(WARM_UP) {
            continue;
        }
        let pre = touched.get(&format!("{m}{WARM_UP}"));
        a.windows.insert(m.clone(), w.iter().copied().filter(|x| !pre.is_some_and(|p| p.contains(x))).collect());
    }
    Ok(a)
}

/// `mode:n,mode:n` -> mode -> windows.
pub fn parse_budgets(text: &str) -> Result<BTreeMap<String, u64>, String> {
    let mut out = BTreeMap::new();
    for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (m, n) = part.split_once(':').ok_or_else(|| format!("`{part}` is not mode:windows"))?;
        let n = n.parse().map_err(|_| format!("`{part}`: {n} is not a number"))?;
        if out.insert(m.to_string(), n).is_some() {
            return Err(format!("mode {m} budgeted twice"));
        }
    }
    Ok(out)
}

/// A base plan (`layout-bins.tsv`: `<bin> TAB <section> TAB <bytes> TAB <signature>`) -> member key -> bytes
/// (fillers left out). `key` maps a section name to its key.
pub fn parse_base(tsv: &str, key: impl Fn(&str) -> String) -> Result<BTreeMap<String, u64>, String> {
    let mut out = BTreeMap::new();
    for (n, l) in tsv.lines().enumerate().filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#')) {
        let f: Vec<&str> = l.split('\t').collect();
        let [_, sec, bytes, sig] = f.as_slice() else {
            return Err(format!("line {}: not four tab-separated columns: {l}", n + 1));
        };
        let bytes: u64 = bytes.parse().map_err(|_| format!("line {}: bytes `{bytes}` is not a number", n + 1))?;
        if *sig != "cold" {
            *out.entry(key(sec)).or_insert(0) += bytes;
        }
    }
    Ok(out)
}

/// What the gate is given besides the link.
pub struct Budgets {
    pub window: u64,
    pub page: u64,
    pub head_room: u64,
    pub profile: String,
    pub modes: BTreeMap<String, u64>,
    /// `.rodata1` and its budget in windows.
    pub rodata1: Option<(Linked, u64)>,
    /// `.data1` and its budget in pages.
    pub data1: Option<(Linked, u64)>,
}

fn fmt_members<'a>(it: impl Iterator<Item = &'a Member>, n: usize) -> String {
    let mut v: Vec<&Member> = it.collect();
    v.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.key.cmp(&b.key)));
    let s: Vec<String> = v.iter().take(n).map(|m| format!("{} {}", m.key, m.bytes)).collect();
    if s.is_empty() { "none".into() } else { s.join(", ") }
}

/// The members of `b` (those `mode` runs, when given) new or grown against `base`, largest growth first.
fn fmt_grown(b: &LinkedBin, mode: Option<&str>, base: Option<&BTreeMap<String, u64>>) -> String {
    let Some(base) = base else {
        return format!("no base plan to diff against (set {BASE_ENV} to next's target/product/<profile>/layout-bins.tsv)");
    };
    let mut v: Vec<(u64, String)> = b
        .members
        .iter()
        .filter(|m| mode.is_none_or(|x| m.modes.contains(x)))
        .filter_map(|m| match base.get(&m.key) {
            None => Some((m.bytes, format!("{} new {}", m.key, m.bytes))),
            Some(&was) if m.bytes > was => Some((m.bytes - was, format!("{} +{} ({was} -> {})", m.key, m.bytes - was, m.bytes))),
            _ => None,
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let total: u64 = v.iter().map(|x| x.0).sum();
    if v.is_empty() {
        return "no member new or grown against the base".into();
    }
    format!("{} members new or grown against the base, {total} bytes: {}", v.len(), v.iter().take(GROWN).map(|x| x.1.as_str()).collect::<Vec<_>>().join(", "))
}

/// The gate: `(report, broken)`. The report (one line per bin, the windows per mode, the data bins) is printed on
/// every build; `broken` names each failure with the bin, bytes, budget, headroom and members.
pub fn verdict(a: &Analysis, b: &Budgets, base: Option<&BTreeMap<String, u64>>) -> (Vec<String>, Vec<String>) {
    let (w, p) = (b.window, &b.profile);
    let mut report = Vec::new();
    let mut broken = Vec::new();
    let bin_line = |n: usize, lb: &LinkedBin| {
        let room = if n == 0 { b.head_room } else { 0 };
        let budget = lb.budget(w, room);
        let modes: Vec<String> = lb.modes().into_iter().collect();
        format!(
            "window bin {n}: {} bytes of {budget} (headroom {}), {} members, 0x{:x}-0x{:x}, modes {}",
            lb.bytes(),
            budget as i64 - lb.bytes() as i64,
            lb.members.len(),
            lb.start,
            lb.end,
            if modes.is_empty() { "-".into() } else { modes.join(",") }
        )
    };
    for (&n, lb) in &a.bins {
        report.push(bin_line(n, lb));
        let room = if n == 0 { b.head_room } else { 0 };
        let budget = lb.budget(w, room);
        let off = lb.start % w;
        if n == 0 && off > b.head_room {
            broken.push(format!("head bin starts {off} bytes into its {w}-byte window, more than head-room {}: what the probe links ahead of the group grew", b.head_room));
        }
        if n > 0 && off != 0 {
            broken.push(format!("window bin {n} starts at 0x{:x}, {off} bytes off a {w}-byte boundary: the plan was not applied", lb.start));
        }
        if lb.bytes() > budget {
            broken.push(format!(
                "window bin {n}: {} bytes > budget {budget} (headroom {}); largest members: {}; {}",
                lb.bytes(),
                budget as i64 - lb.bytes() as i64,
                fmt_members(lb.members.iter(), LARGEST),
                fmt_grown(lb, None, base)
            ));
        }
    }
    let mut per_mode = Vec::new();
    for (m, ws) in &a.windows {
        let n = ws.len() as u64;
        let Some(&budget) = b.modes.get(m) else {
            broken.push(format!("mode {m}: {n} windows and no budget in windows-{p}"));
            continue;
        };
        per_mode.push(format!("{m} {n}/{budget}"));
        if n > budget {
            broken.push(format!("mode {m}: {n} windows of {w} bytes > windows-{p} {budget}"));
            for x in ws {
                for (&k, lb) in a.bins.iter().filter(|(_, lb)| lb.start / w <= *x && *x <= (lb.end - 1) / w) {
                    broken.push(format!(
                        "  mode {m}: window 0x{:x}: {}; largest members {m} runs there: {}; {}",
                        x * w,
                        bin_line(k, lb),
                        fmt_members(lb.members.iter().filter(|mb| mb.modes.contains(m)), LARGEST),
                        fmt_grown(lb, Some(m), base)
                    ));
                }
            }
        } else if n < budget {
            broken.push(format!("mode {m}: {n} windows < windows-{p} {budget}: the level improved; tighten the budget to {n} (ALWAYS FORWARD)"));
        }
    }
    for m in b.modes.keys().filter(|m| !a.windows.contains_key(*m)) {
        broken.push(format!("mode {m}: budgeted in windows-{p} but no bin member runs it (renamed or dropped mode?)"));
    }
    report.push(format!("windows per mode ({p}, count/budget): {}", per_mode.join(" ")));
    for (name, sec, unit, what) in [(".rodata1", &b.rodata1, w, "windows"), (".data1", &b.data1, b.page, "pages")] {
        let Some((s, units)) = sec else {
            broken.push(format!("{name}: not in the probe's link (the start-up cluster is missing)"));
            continue;
        };
        let bytes = s.end - s.start / unit * unit;
        let budget = units * unit;
        report.push(format!("{name}: {bytes} bytes from its {unit}-byte boundary of {budget} ({units} {what}; headroom {})", budget as i64 - bytes as i64));
        if bytes > budget {
            broken.push(format!("{name}: {bytes} bytes from its {unit}-byte boundary > {budget} ({units} {what}) at 0x{:x}-0x{:x}", s.start, s.end));
        }
    }
    (report, broken)
}

/// A start-up data object of the probe's link (D-X.138): `[start, end)`, its name, the output section it lies in, and
/// whether that section is written (counted in pages) or read-only (counted in windows).
#[derive(Clone, Debug, PartialEq)]
pub struct DataObject {
    pub obj: Linked,
    pub section: Linked,
    pub rw: bool,
}

/// A data bin: the start-up objects of one output section, or of adjacent sections that share a unit joined with `+`
/// in the budget (`.data1+.bss`), each cut to its first unit (start-up touches the head of a large buffer, as
/// STARTUP-DATA-WINDOWS counts it). Its bytes run from the unit boundary below its first object to the end of its
/// last, so a section that straddles a boundary, or is pushed onto a unit of its own by the section ahead of it in
/// the same bin, costs bytes here as it costs a page or window there.
#[derive(Clone, Debug, PartialEq)]
pub struct DataBin {
    /// Its sections, in link order.
    pub sections: Vec<Linked>,
    pub rw: bool,
    pub unit: u64,
    pub objs: Vec<Linked>,
    /// Written and read-only sections in one bin (a budget error).
    pub mixed: bool,
}

impl DataBin {
    fn start(&self) -> u64 {
        self.objs.iter().map(|o| o.start).min().unwrap_or(0)
    }

    fn end(&self) -> u64 {
        self.objs.iter().map(|o| o.end).max().unwrap_or(0)
    }

    /// Bytes from the unit boundary below the first object to the end of the last. Objects an untouched unit apart (two
    /// large buffers far apart in `.lbss`) are separate runs: each run but the last is charged its whole units, so the
    /// bytes fit `n` units exactly when the runs touch at most `n`.
    pub fn bytes(&self) -> u64 {
        let u = self.unit;
        let mut objs: Vec<&Linked> = self.objs.iter().collect();
        objs.sort_by_key(|o| o.start);
        let mut runs: Vec<(u64, u64)> = Vec::new();
        for o in objs {
            match runs.last_mut() {
                Some(r) if o.start / u <= (r.1 - 1) / u + 1 => r.1 = r.1.max(o.end),
                _ => runs.push((o.start / u * u, o.end)),
            }
        }
        let n = runs.len();
        runs.iter().enumerate().map(|(i, r)| if i + 1 < n { (r.1 - r.0).div_ceil(u) * u } else { r.1 - r.0 }).sum()
    }

    /// The units (page or window numbers) the objects touch.
    pub fn touched(&self) -> BTreeSet<u64> {
        self.objs.iter().flat_map(|o| o.start / self.unit..=(o.end - 1) / self.unit).collect()
    }
}

/// The start-up data objects grouped into bins: the budget naming the object's section (alone or `+`-joined), else a
/// bin of the section's own name (which has no budget).
pub fn data_bins(objs: &[DataObject], window: u64, page: u64, budgets: &BTreeMap<String, u64>) -> BTreeMap<String, DataBin> {
    let mut out: BTreeMap<String, DataBin> = BTreeMap::new();
    for o in objs {
        let unit = if o.rw { page } else { window };
        let end = o.obj.end.min(o.obj.start + unit).max(o.obj.start + 1);
        let key = budgets.keys().find(|k| k.split('+').any(|s| s == o.section.name)).cloned().unwrap_or_else(|| o.section.name.clone());
        let bin = out.entry(key).or_insert_with(|| DataBin { sections: Vec::new(), rw: o.rw, unit, objs: Vec::new(), mixed: false });
        bin.mixed |= bin.rw != o.rw;
        if !bin.sections.contains(&o.section) {
            bin.sections.push(o.section.clone());
            bin.sections.sort_by_key(|s| s.start);
        }
        bin.objs.push(Linked { start: o.obj.start, end, name: o.obj.name.clone() });
    }
    out
}

/// The data-bin gate (D-X.138): `(report, broken)`. Every output section holding start-up data needs a budget in
/// `data-bins-<profile>` (`<section>[+<section>...]:<units>`, pages for written sections, windows for read-only ones)
/// and its bin's bytes must fit `units x unit`; a budget no start-up data lies in is red too (the data moved). The
/// count STARTUP-DATA-WINDOWS reports is at most the bins' units added up, so with sections that share a unit joined in
/// one bin, a green byte gate keeps the count within the sum.
pub fn data_verdict(bins: &BTreeMap<String, DataBin>, budgets: &BTreeMap<String, u64>, profile: &str) -> (Vec<String>, Vec<String>) {
    let mut report = Vec::new();
    let mut broken = Vec::new();
    let mut sums = [(0u64, 0u64); 2];
    for (name, b) in bins {
        let (class, what) = if b.rw { ("written", "pages") } else { ("read-only", "windows") };
        let (first, last) = (b.start(), b.end());
        let touched = b.touched();
        let Some(&units) = budgets.get(name) else {
            broken.push(format!(
                "data bin {name} ({class}): {} start-up objects at 0x{first:x}-0x{last:x} ({} {what}) and no budget in data-bins-{profile}: start-up data newly lies in this section",
                b.objs.len(),
                touched.len()
            ));
            continue;
        };
        if b.mixed {
            broken.push(format!("data bin {name}: joins written and read-only sections (a bin counts pages or windows, not both)"));
            continue;
        }
        let (bytes, budget) = (b.bytes(), units * b.unit);
        let secs: Vec<String> = b.sections.iter().map(|s| format!("{} 0x{:x}-0x{:x} ({} bytes into its unit)", s.name, s.start, s.end, s.start % b.unit)).collect();
        sums[b.rw as usize].0 += touched.len() as u64;
        sums[b.rw as usize].1 += units;
        report.push(format!(
            "data bin {name} ({class}): {bytes} bytes from its {}-byte boundary of {budget} ({units} {what}; headroom {}), {} start-up objects 0x{first:x}-0x{last:x}, {} {what} touched; {}",
            b.unit,
            budget as i64 - bytes as i64,
            b.objs.len(),
            touched.len(),
            secs.join(", ")
        ));
        if bytes > budget {
            let mut objs: Vec<&Linked> = b.objs.iter().collect();
            objs.sort_by(|x, y| (y.end - y.start).cmp(&(x.end - x.start)).then_with(|| x.name.cmp(&y.name)));
            let largest: Vec<String> = objs.iter().take(LARGEST).map(|o| format!("{} {} at 0x{:x}", o.name, o.end - o.start, o.start)).collect();
            let at: Vec<String> = touched.iter().map(|u| format!("0x{:x}", u * b.unit)).collect();
            broken.push(format!(
                "data bin {name} ({class}): {bytes} bytes from its {}-byte boundary > {budget} ({units} {what}; headroom {}): its start-up objects touch {} {what} ({}); sections {} (what is linked ahead of a section places it); largest objects: {}",
                b.unit,
                budget as i64 - bytes as i64,
                touched.len(),
                at.join(" "),
                secs.join(", "),
                largest.join(", ")
            ));
        }
    }
    for name in budgets.keys().filter(|n| !bins.contains_key(*n)) {
        broken.push(format!("data bin {name}: budgeted in data-bins-{profile} but no start-up object lies there (section renamed, or its start-up data moved?)"));
    }
    report.push(format!(
        "data bins ({profile}): written {} pages touched of {} budgeted, read-only {} windows touched of {} budgeted",
        sums[1].0, sums[1].1, sums[0].0, sums[0].1
    ));
    (report, broken)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bins::BIN;

    fn sec(start: u64, len: u64, name: &str) -> Linked {
        Linked { start, end: start + len, name: name.into() }
    }

    fn sigs(rows: &[(&str, &str)]) -> BTreeMap<String, BTreeSet<String>> {
        rows.iter().map(|(k, m)| (k.to_string(), m.split(',').filter(|x| !x.is_empty()).map(str::to_string).collect())).collect()
    }

    fn budgets(modes: &str) -> Budgets {
        Budgets {
            window: BIN,
            page: 4096,
            head_room: 0x400,
            profile: "fast".into(),
            modes: parse_budgets(modes).unwrap(),
            rodata1: Some((sec(0x50_0000, 40_000, ".rodata1"), 1)),
            data1: Some((sec(0x60_3000, 11_744, ".data1"), 3)),
        }
    }

    /// Head bin (start-up and locale), bin 1 (locale only), a phase mode whose warm-up runs bin 0.
    fn link() -> Vec<Linked> {
        vec![
            sec(0x41_0040, 0x8000, &crate::bins::section(0, false, 1, "start")),
            sec(0x41_8040, 0x7000, &crate::bins::section(0, false, 2, "setlocale")),
            sec(0x41_f040, 0x800, &crate::bins::section(0, true, 0, "cold_a")),
            sec(0x42_0000, 0x9000, &crate::bins::section(1, false, 3, "newlocale")),
            sec(0x42_9000, 0x100, &crate::bins::section(1, false, 4, "strlen")),
            sec(0x43_0000, 0, ".text.sorted.h9z.niji_group_end"),
        ]
    }

    fn sig_rows() -> BTreeMap<String, BTreeSet<String>> {
        sigs(&[("start", "true,locale,strings.pre"), ("setlocale", "locale"), ("newlocale", "locale"), ("strlen", "strings")])
    }

    #[test]
    fn bin_names_parse_and_the_group_end_is_no_bin() {
        // no spec clause: tooling (D-X.136).
        assert_eq!(bin_of(&crate::bins::section(7, false, 42, "f")), Some((7, false, "f")));
        assert_eq!(bin_of(&crate::bins::section(12, true, 3, "g.h")), Some((12, true, "g.h")));
        assert_eq!(bin_of(".text.sorted.h9z.niji_group_end"), None);
        assert_eq!(bin_of(".text.sorted.h00042.f"), None, "a list position, not a bin");
    }

    #[test]
    fn modes_count_their_windows_less_their_warm_up() {
        // no spec clause: tooling (D-X.136).
        let a = analyse(&link(), &sig_rows(), str::to_string, BIN).unwrap();
        let n: BTreeMap<&str, usize> = a.windows.iter().map(|(m, w)| (m.as_str(), w.len())).collect();
        assert_eq!(n, BTreeMap::from([("locale", 2), ("strings", 1), ("true", 1)]), "strings' bin 1 window only: its warm-up faulted bin 0");
        assert_eq!(a.bins[&0].bytes(), 0xf000, "fillers are not counted");
        assert_eq!(a.bins[&1].members.len(), 2);
        // The phase also runs `start` (bin 0), which its warm-up already faulted: that window is not counted again.
        let shared = sigs(&[("start", "true,strings,strings.pre"), ("setlocale", "locale"), ("newlocale", "locale"), ("strlen", "strings")]);
        let b = analyse(&link(), &shared, str::to_string, BIN).unwrap();
        assert_eq!(b.windows["strings"].len(), 1, "a window its warm-up faulted counts once, in the warm-up: {:?}", b.windows);
        assert!(analyse(&link(), &sigs(&[("start", "true")]), str::to_string, BIN).unwrap_err().contains("no signature"));
    }

    #[test]
    fn a_green_link_reports_every_bin_and_its_headroom() {
        // no spec clause: tooling (D-X.136).
        let a = analyse(&link(), &sig_rows(), str::to_string, BIN).unwrap();
        let (report, broken) = verdict(&a, &budgets("true:1,locale:2,strings:1"), None);
        assert!(broken.is_empty(), "{broken:?}");
        assert!(report[0].starts_with("window bin 0: 61440 bytes of 64512 (headroom 3072), 2 members"), "{report:?}");
        assert!(report[1].starts_with("window bin 1: 37120 bytes of 65536 (headroom 28416)"), "{report:?}");
        assert!(report.iter().any(|l| l == "windows per mode (fast, count/budget): locale 2/2 strings 1/1 true 1/1"), "{report:?}");
        assert!(report.iter().any(|l| l.starts_with(".data1: 11744 bytes from its 4096-byte boundary of 12288")), "{report:?}");
    }

    #[test]
    fn a_mode_over_budget_names_its_bins_members_and_growth() {
        // no spec clause: tooling (D-X.136).
        let a = analyse(&link(), &sig_rows(), str::to_string, BIN).unwrap();
        let base = BTreeMap::from([("start".to_string(), 0x8000), ("setlocale".to_string(), 0x6000)]);
        let (_, broken) = verdict(&a, &budgets("true:1,locale:1,strings:1"), Some(&base));
        assert_eq!(broken[0], "mode locale: 2 windows of 65536 bytes > windows-fast 1", "{broken:?}");
        let w1 = broken.iter().find(|l| l.contains("window 0x420000")).unwrap();
        assert!(w1.contains("largest members locale runs there: newlocale 36864") && w1.contains("newlocale new 36864"), "{w1}");
        let w0 = broken.iter().find(|l| l.contains("window 0x410000")).unwrap();
        assert!(w0.contains("1 members new or grown against the base, 4096 bytes: setlocale +4096 (24576 -> 28672)"), "{w0}");
        let (_, none) = verdict(&a, &budgets("true:1,locale:1,strings:1"), None);
        assert!(none.iter().any(|l| l.contains(BASE_ENV)), "without a base the verdict says how to get one");
    }

    #[test]
    fn a_level_under_budget_an_unbudgeted_mode_and_a_misplaced_bin_are_red() {
        // no spec clause: tooling (D-X.136).
        let a = analyse(&link(), &sig_rows(), str::to_string, BIN).unwrap();
        let (_, broken) = verdict(&a, &budgets("true:2,locale:2,strings:1,gone:1"), None);
        assert!(broken.iter().any(|l| l.contains("mode true: 1 windows < windows-fast 2")), "{broken:?}");
        assert!(broken.iter().any(|l| l.contains("mode gone: budgeted")), "{broken:?}");
        let (_, broken) = verdict(&a, &budgets("true:1,locale:2"), None);
        assert!(broken.iter().any(|l| l == "mode strings: 1 windows and no budget in windows-fast"), "{broken:?}");
        let mut l = link();
        l[3].start += 0x40;
        let (_, broken) = verdict(&analyse(&l, &sig_rows(), str::to_string, BIN).unwrap(), &budgets("true:1,locale:2,strings:1"), None);
        assert!(broken.iter().any(|x| x.contains("window bin 1 starts at 0x420040, 64 bytes off")), "{broken:?}");
    }

    #[test]
    fn bytes_over_a_bin_or_data_budget_are_red() {
        // no spec clause: tooling (D-X.136).
        let mut a = analyse(&link(), &sig_rows(), str::to_string, BIN).unwrap();
        a.bins.get_mut(&1).unwrap().end = 0x43_0001;
        let mut b = budgets("true:1,locale:2,strings:1");
        b.data1 = Some((sec(0x60_3000, 12_289, ".data1"), 3));
        b.rodata1 = Some((sec(0x50_8000, 0x8001, ".rodata1"), 1));
        let (_, broken) = verdict(&a, &b, None);
        assert!(broken.iter().any(|l| l.starts_with("window bin 1: 65537 bytes > budget 65536 (headroom -1); largest members: newlocale 36864")), "{broken:?}");
        assert!(broken.iter().any(|l| l.starts_with(".data1: 12289 bytes")), "{broken:?}");
        assert!(broken.iter().any(|l| l.starts_with(".rodata1: 65537 bytes")), "{broken:?}");
        b.data1 = None;
        assert!(verdict(&a, &b, None).1.iter().any(|l| l.contains(".data1: not in the probe's link")));
    }

    #[test]
    fn a_base_plan_parses_members_only() {
        // no spec clause: tooling (D-X.136).
        let t = "# c\n0\t.text.sorted.h00001.f\t100\ttrue\n0\t.text.c\t50\tcold\n1\t.text.sorted.h00002.g\t7\t-\n";
        let m = parse_base(t, str::to_string).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[".text.sorted.h00001.f"], 100);
        assert!(parse_base("0\tx\n", str::to_string).is_err());
        assert!(parse_budgets("a:1,a:2").is_err() && parse_budgets("a").is_err());
    }

    fn dobj(start: u64, len: u64, name: &str, section: &Linked, rw: bool) -> DataObject {
        DataObject { obj: sec(start, len, name), section: section.clone(), rw }
    }

    /// next's start-up data on the Skylake-SP host (fast): RELRO head, `.got` 272 bytes into its page, the heap
    /// reserve's head in `.lbss`, one read-only object in `.rodata1`; `got` places `.got` and its two start-up slots.
    fn data_link(got: Linked, slots: [u64; 2]) -> Vec<DataObject> {
        let relro = sec(0x86_0000, 0xb110, ".data.rel.ro");
        let lbss = sec(0x88_c000, 0xc_4418, ".lbss");
        let ro = sec(0x80_0000, 0x9400, ".rodata1");
        vec![
            dobj(0x86_0200, 0x120, "C_SET", &relro, true),
            dobj(0x86_10b8, 0x390, "text_only", &relro, true),
            dobj(slots[0], 1, "<unnamed .got byte>", &got, true),
            dobj(slots[1], 1, "<unnamed .got byte>", &got, true),
            dobj(0x92_0000, 0x1_0000, "RESERVE_MEM", &lbss, true),
            dobj(0x80_0100, 0x40, "NAMES", &ro, false),
        ]
    }

    #[test]
    fn a_written_section_straddling_a_page_is_red_in_bytes() {
        // no spec clause: tooling (D-X.138); THREAD-FORCED-UNWIND-2: the section ahead of .got shrank by 0x148 bytes and
        // .got crossed a page boundary (8 written pages counted while every byte bin passed).
        let budgets = parse_budgets(".data.rel.ro:2,.got:1,.lbss:1,.rodata1:1").unwrap();
        let next = data_link(sec(0x86_b110, 0xc8, ".got"), [0x86_b128, 0x86_b1d0]);
        let (report, broken) = data_verdict(&data_bins(&next, BIN, 4096, &budgets), &budgets, "fast");
        assert!(broken.is_empty(), "{broken:?}");
        assert!(report.iter().any(|l| l.starts_with("data bin .got (written): 465 bytes from its 4096-byte boundary of 4096 (1 pages; headroom 3631), 2 start-up objects")), "{report:?}");
        assert!(report.iter().any(|l| l.starts_with("data bin .lbss (written): 4096 bytes")), "a large object counts its first page: {report:?}");
        assert!(report.iter().any(|l| l.starts_with("data bin .rodata1 (read-only): 320 bytes from its 65536-byte boundary")), "{report:?}");
        assert!(report.iter().any(|l| l == "data bins (fast): written 4 pages touched of 4 budgeted, read-only 1 windows touched of 1 budgeted"), "{report:?}");
        let moved = data_link(sec(0x86_afc8, 0xc8, ".got"), [0x86_afe0, 0x86_b088]);
        let (_, broken) = data_verdict(&data_bins(&moved, BIN, 4096, &budgets), &budgets, "fast");
        assert_eq!(broken.len(), 1, "{broken:?}");
        assert!(
            broken[0].starts_with("data bin .got (written): 4233 bytes from its 4096-byte boundary > 4096 (1 pages; headroom -137): its start-up objects touch 2 pages (0x86a000 0x86b000); sections .got 0x86afc8-0x86b090 (4040 bytes into its unit)"),
            "{broken:?}"
        );
        let mut ro = next.clone();
        ro.push(dobj(0x81_0000, 8, "FAR", &sec(0x80_0000, 0x1_0010, ".rodata1"), false));
        let (_, broken) = data_verdict(&data_bins(&ro, BIN, 4096, &budgets), &budgets, "fast");
        assert!(broken.iter().any(|l| l.starts_with("data bin .rodata1 (read-only): 65544 bytes from its 65536-byte boundary > 65536 (1 windows")), "{broken:?}");
        // The release link: a second large buffer 0x4f000 bytes further into .lbss costs one page, not the span.
        let mut far = next.clone();
        far.push(dobj(0x96_f000, 0x1_0000, "OTHER", &sec(0x88_c000, 0xc_4418, ".lbss"), true));
        let two = parse_budgets(".data.rel.ro:2,.got:1,.lbss:2,.rodata1:1").unwrap();
        let (report, broken) = data_verdict(&data_bins(&far, BIN, 4096, &two), &two, "release");
        assert!(broken.is_empty(), "{broken:?}");
        assert!(report.iter().any(|l| l.starts_with("data bin .lbss (written): 8192 bytes from its 4096-byte boundary of 8192 (2 pages; headroom 0), 2 start-up objects")), "{report:?}");
        let (_, broken) = data_verdict(&data_bins(&far, BIN, 4096, &budgets), &budgets, "release");
        assert!(broken.iter().any(|l| l.starts_with("data bin .lbss (written): 8192 bytes from its 4096-byte boundary > 4096 (1 pages")), "{broken:?}");
    }

    #[test]
    fn sections_sharing_a_page_are_one_bin_so_a_pushed_section_is_red() {
        // no spec clause: tooling (D-X.138); next: .bss's start-up words share .data1's last page. As two bins (3 + 1
        // pages) .data1 could grow within its own bytes, push them onto a page of their own and stay green.
        let (data1, bss) = (sec(0x88_8000, 0x2df0, ".data1"), sec(0x88_ae00, 0xf60, ".bss"));
        let link = |at: u64| {
            let d = |a: u64, n: u64, name: &str| dobj(a, n, name, &data1, true);
            vec![d(0x88_8000, 0x40, "DETECTIONS"), d(0x88_9da8, 0xe0, "stderr"), d(0x88_adc8, 8, "__environ"), dobj(at, 8, "WORD", &bss, true)]
        };
        let budgets = parse_budgets(".data1+.bss:3").unwrap();
        let (report, broken) = data_verdict(&data_bins(&link(0x88_ae00), BIN, 4096, &budgets), &budgets, "fast");
        assert!(broken.is_empty(), "{broken:?}");
        assert!(report.iter().any(|l| l.starts_with("data bin .data1+.bss (written): 11784 bytes from its 4096-byte boundary of 12288 (3 pages; headroom 504), 4 start-up objects 0x888000-0x88ae08, 3 pages touched; .data1 0x888000")), "{report:?}");
        let (_, broken) = data_verdict(&data_bins(&link(0x88_b000), BIN, 4096, &budgets), &budgets, "fast");
        assert!(broken.iter().any(|l| l.starts_with("data bin .data1+.bss (written): 12296 bytes from its 4096-byte boundary > 12288 (3 pages; headroom -8): its start-up objects touch 4 pages")), "{broken:?}");
        let mixed = parse_budgets(".data1+.rodata1:3").unwrap();
        let mut objs = link(0x88_ae00);
        objs.push(dobj(0x80_0100, 8, "NAMES", &sec(0x80_0000, 0x9400, ".rodata1"), false));
        let (_, broken) = data_verdict(&data_bins(&objs, BIN, 4096, &mixed), &mixed, "fast");
        assert!(broken.iter().any(|l| l.starts_with("data bin .data1+.rodata1: joins written and read-only sections")), "{broken:?}");
    }

    #[test]
    fn every_section_holding_start_up_data_needs_a_budget_and_every_budget_its_data() {
        // no spec clause: tooling (D-X.138); a section left out of the budgets would let its pages go uncounted.
        let next = data_link(sec(0x86_b110, 0xc8, ".got"), [0x86_b128, 0x86_b1d0]);
        let check = |b: &str| {
            let b = parse_budgets(b).unwrap();
            data_verdict(&data_bins(&next, BIN, 4096, &b), &b, "fast").1
        };
        let broken = check(".data.rel.ro:2,.lbss:1,.rodata1:1");
        assert!(broken.iter().any(|l| l.starts_with("data bin .got (written): 2 start-up objects at 0x86b128-0x86b1d1 (1 pages) and no budget in data-bins-fast")), "{broken:?}");
        let broken = check(".data.rel.ro:2,.got:1,.lbss:1,.rodata1:1,.bss:1");
        assert_eq!(broken, ["data bin .bss: budgeted in data-bins-fast but no start-up object lies there (section renamed, or its start-up data moved?)"]);
        let broken = check(".data.rel.ro:1,.got:1,.lbss:1,.rodata1:1");
        assert!(broken.iter().any(|l| l.starts_with("data bin .data.rel.ro (written): 5192 bytes from its 4096-byte boundary > 4096 (1 pages; headroom -1096)")), "{broken:?}");
    }
}
