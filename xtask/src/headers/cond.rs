//! Configuration conditions (plan/headers.md 2.3): the feature levels of each probed configuration, the smallest
//! condition over them that selects a set of configurations, and the condition of a name in the header that
//! shows it (visibility.tsv), ignoring the configurations that do not decide it.
use super::*;

/// The level columns of `headers/levels.tsv`, their C names in `features.h` and the thresholds the
/// condition search may test (an empty list: a 0/1 flag).
pub(crate) const LEVEL_COLS: &[(&str, &str, &[i64])] = &[
    ("isoc", "__NIJI_ISOC", &[1999, 2011, 2023]),
    ("posix", "__NIJI_POSIX", &[1, 2, 199309, 199506, 200112, 200809]),
    ("xsi", "__NIJI_XSI", &[400, 450, 500, 600, 700]),
    ("misc", "__NIJI_MISC", &[]),
    ("gnu", "__NIJI_GNU", &[]),
    ("lfs64", "__NIJI_LFS64", &[]),
    ("fob64", "__NIJI_FOB64", &[]),
    ("time64", "__NIJI_TIME64", &[]),
    ("dynstack", "__NIJI_DYNSTACK", &[]),
    ("fortify", "__NIJI_FORTIFY", &[1, 2, 3]),
    ("cxx", "__NIJI_CXX", &[1, 2, 3]),
];

pub(crate) struct Levels {
    pub ids: Vec<String>,
    pub values: Vec<Vec<i64>>,
    pub reject: u64,
}

pub(crate) fn levels(root: &Path) -> Result<Levels, String> {
    let configs = tsv(root, "spec/abi/ftm-configs.tsv")?;
    let rows = tsv(root, LEVELS)?;
    let ids: Vec<String> = configs.iter().map(|r| r["id"].clone()).collect();
    if ids.len() > 64 {
        return Err("more than 64 configurations".into());
    }
    let got: Vec<&String> = rows.iter().map(|r| &r["id"]).collect();
    if got != ids.iter().collect::<Vec<_>>() {
        return Err(format!("{LEVELS}: ids differ from spec/abi/ftm-configs.tsv (same ids, same order)"));
    }
    let mut values = Vec::new();
    let mut reject = 0u64;
    for (i, r) in rows.iter().enumerate() {
        let mut v = Vec::new();
        for (col, _, _) in LEVEL_COLS {
            let s = r.get(*col).ok_or(format!("{LEVELS}: no column {col}"))?;
            v.push(s.parse::<i64>().map_err(|_| format!("{LEVELS}: {} {col} = {s:?}", r["id"]))?);
        }
        if r.get("reject").map(|s| s == "1").unwrap_or(false) {
            reject |= 1 << i;
        }
        values.push(v);
    }
    Ok(Levels { ids, values, reject })
}

/// A literal: level column `col` compared with `thr` (flags: `thr` 0 means "is set"), maybe negated.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Lit {
    col: usize,
    thr: i64,
    neg: bool,
}

impl Lit {
    fn mask(&self, lv: &Levels) -> u64 {
        let all = (1u64 << lv.ids.len()) - 1;
        let m = lv.values.iter().enumerate().filter(|(_, v)| if self.thr == 0 { v[self.col] != 0 } else { v[self.col] >= self.thr }).fold(0u64, |m, (i, _)| m | 1 << i);
        if self.neg { !m & all } else { m }
    }
    fn render(&self) -> String {
        let name = LEVEL_COLS[self.col].1;
        match (self.thr, self.neg) {
            (0, false) => name.to_string(),
            (0, true) => format!("!{name}"),
            (t, false) => format!("{name} >= {t}"),
            (t, true) => format!("{name} < {t}"),
        }
    }
}

/// The C condition true in exactly the configurations of `target` among `care` (see the module text).
pub(crate) fn condition(lv: &Levels, target: u64, care: u64) -> Option<String> {
    let t = target & care;
    if t == care {
        return Some("1".into());
    }
    if t == 0 {
        return Some("0".into());
    }
    let mut lits = Vec::new();
    for (c, (_, _, thr)) in LEVEL_COLS.iter().enumerate() {
        let ts: Vec<i64> = if thr.is_empty() { vec![0] } else { thr.to_vec() };
        for th in ts {
            lits.push(Lit { col: c, thr: th, neg: false });
            lits.push(Lit { col: c, thr: th, neg: true });
        }
    }
    let mut terms: Vec<(Vec<Lit>, u64)> = lits.iter().map(|l| (vec![*l], l.mask(lv))).collect();
    for (i, a) in lits.iter().enumerate() {
        for b in &lits[i + 1..] {
            // Two columns, or a range of one column (`>= lo && < hi`, e.g. POSIX 1 up to 199506).
            if a.col != b.col || (a.thr < b.thr && !a.neg && b.neg) {
                terms.push((vec![*a, *b], a.mask(lv) & b.mask(lv)));
            }
        }
    }
    terms.retain(|(_, m)| m & t != 0 && m & care & !t == 0);
    let mut left = t;
    let mut chosen: Vec<&Vec<Lit>> = Vec::new();
    while left != 0 {
        // Ties go to the first term (level columns in table order, single literals first): stable output.
        let best = terms.iter().rev().max_by(|a, b| (a.1 & left).count_ones().cmp(&(b.1 & left).count_ones()).then(b.0.len().cmp(&a.0.len())))?;
        if best.1 & left == 0 {
            return None;
        }
        left &= !best.1;
        chosen.push(&best.0);
    }
    let parts: Vec<String> = chosen.iter().map(|ls| ls.iter().map(|l| l.render()).collect::<Vec<_>>().join(" && ")).collect();
    Some(if parts.len() == 1 { parts[0].clone() } else { parts.iter().map(|p| if p.contains("&&") { format!("({p})") } else { p.clone() }).collect::<Vec<_>>().join(" || ") })
}

/// `configs` of visibility.tsv (`*`, ids, or `!` and the ids it is not visible in) as a mask.
pub(crate) fn mask_of(lv: &Levels, configs: &str) -> Result<u64, String> {
    let all = (1u64 << lv.ids.len()) - 1;
    let (neg, list) = match configs.strip_prefix('!') {
        Some(rest) => (true, rest),
        None if configs == "*" => return Ok(all),
        None => (false, configs),
    };
    let mut m = 0u64;
    for id in list.split(',') {
        let i = lv.ids.iter().position(|x| x == id).ok_or(format!("unknown configuration {id:?}"))?;
        m |= 1 << i;
    }
    Ok(if neg { !m & all } else { m })
}

impl Gen<'_> {
    /// Whether the included headers supply (name, kind) in every configuration this header's row asks for and
    /// that matters (the ones `dont_care` leaves): otherwise this header emits it too, under its own condition.
    pub(super) fn is_covered(&self, name: &str, kind: &str) -> bool {
        let Some(&have) = self.covered.get(&(name.to_string(), kind.to_string())) else { return false };
        let Some(r) = self.t.vis_row(&self.header, name, kind) else { return true };
        let all = (1u64 << self.t.lv.ids.len()) - 1;
        let want = mask_of(&self.t.lv, &r["configs"]).unwrap_or(0) & all & !self.dont_care(name, kind);
        want & !have == 0
    }

    pub(super) fn rows(&self, name: &str) -> Vec<&BTreeMap<String, String>> {
        self.t.vis_idx(&self.header, name).iter().map(|&i| &self.t.vis[i]).collect()
    }

    /// The configurations that do not decide a condition: rejected ones, those where the header
    /// itself does not compile, and those where (name, kind) is visible without any header.
    pub(super) fn dont_care(&self, name: &str, kind: &str) -> u64 {
        let mut d = self.t.lv.reject | self.t.include_error.get(&self.header).copied().unwrap_or(0);
        for r in self.t.vis_idx("-", name).iter().map(|&i| &self.t.vis[i]).filter(|r| r["kind"] == kind) {
            d |= mask_of(&self.t.lv, &r["configs"]).unwrap_or(0);
        }
        d
    }

    pub(super) fn cond(&mut self, name: &str, kind: Option<&str>) -> String {
        let rows = self.rows(name);
        let Some(r) = rows.iter().find(|r| kind.map(|k| r["kind"] == k).unwrap_or(true)) else {
            self.errors.push(format!("{}: {name} has no visibility.tsv row", self.header));
            return "0".into();
        };
        let all = (1u64 << self.t.lv.ids.len()) - 1;
        let care = all & !self.dont_care(name, &r["kind"]);
        let target = match mask_of(&self.t.lv, &r["configs"]) {
            Ok(m) => m,
            Err(e) => {
                self.errors.push(format!("{}: {name}: {e}", self.header));
                return "0".into();
            }
        };
        let configs = r["configs"].clone();
        // The _FORTIFY_SOURCE configurations (-O2 probes) belong to slice 18 (fortify): when no condition over the
        // levels separates a name there, they stop deciding it, and the generated tests keep the C probe's other rows.
        let fortify = self.fortify_mask();
        match condition(&self.t.lv, target, care).or_else(|| condition(&self.t.lv, target & !fortify, care & !fortify)) {
            Some(c) => c,
            None => {
                self.errors.push(format!("{}: {name}: no condition over the levels selects {configs}", self.header));
                "0".into()
            }
        }
    }

    /// The condition of the tag `inner_kind inner` (a structure or union) in this header when the header shows it in
    /// fewer of the configurations that matter than `outer_kind outer` (an aggregate with a member of that type, or a
    /// typedef named for linkage through the tag), else None.
    pub(super) fn narrower_tag(&self, (outer_kind, outer): (&str, &str), (inner_kind, inner): (&str, &str)) -> Option<String> {
        let row = |k: &str, n: &str| self.t.vis_row(&self.header, n, k);
        let (o, i) = (row(outer_kind, outer)?, row(inner_kind, inner)?);
        let all = (1u64 << self.t.lv.ids.len()) - 1;
        let care = all & !self.dont_care(inner, inner_kind);
        let (om, im) = (mask_of(&self.t.lv, &o["configs"]).ok()?, mask_of(&self.t.lv, &i["configs"]).ok()?);
        if om & care & !im == 0 {
            return None;
        }
        let fortify = self.fortify_mask();
        condition(&self.t.lv, im, care).or_else(|| condition(&self.t.lv, im & !fortify, care & !fortify))
    }

    pub(super) fn fortify_mask(&self) -> u64 {
        let col = LEVEL_COLS.iter().position(|c| c.0 == "fortify").unwrap_or(0);
        self.t.lv.values.iter().enumerate().filter(|(_, v)| v[col] > 0).fold(0u64, |m, (i, _)| m | 1 << i)
    }
}
