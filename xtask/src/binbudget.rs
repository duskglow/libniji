//! LAYOUT-BYTE-BUDGETS (owner ruling R2 of `plan/layout-budget-rethink.md`, D-X.136): the per-story gate on the
//! start-up layout, in bytes and windows read from the link map of the static probe (`page_faults.c` linked as
//! `crates/core/tests/hot_text_products.rs` links it). No program runs: the verdict is a function of the archive, the
//! build's signatures (`hot-text.signatures`) and the probe's link.
//!
//! * **Bins.** Every window bin (`.text.sorted.h9<bin>...`, [`crate::bins::section`]) is measured from its first
//!   member's start to its last member's end. A bin after the head bin must start on a window boundary (else the plan
//!   was not applied), the head bin no more than `head-room` bytes into its window. Budget: the window ([`BIN`]; the
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

use std::collections::{BTreeMap, BTreeSet};

use crate::bins::{BIN, WARM_UP};

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

#[cfg(test)]
mod tests {
    use super::*;

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
        sigs(&[("start", "true,locale,strings.pre"), ("setlocale", "locale"), ("newlocale", "locale"), ("strlen", "strings,strings.pre")])
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
}
