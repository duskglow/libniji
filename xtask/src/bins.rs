//! LAYOUT-WINDOW-BINS (plan/layout-budget-rethink.md section 3, D-X.103): the window-bin plan of the hot
//! text group. `cargo xtask so` applies the plan (slice 2: section names and alignments, [`Plan::placements`]),
//! writes it to `target/product/<profile>/layout-bins.tsv` and prints its summary.
//!
//! Every grouped section has a **signature**: the `page_faults.c` modes whose trace runs it
//! (`hot-text.signatures`, written by `tools/hottext/regen.sh`). A **bin** holds at most [`BIN`] bytes and,
//! once the plan is applied, starts on a [`BIN`] boundary, so a mode's text count over the group is the
//! number of windows of the bins it touches: a function of the plan alone, which growth inside a bin that
//! still fits, or growth anywhere else, cannot move. The cost of a plan is the sum over modes of the windows
//! each one touches, which is the sum over bins of (modes touching the bin x the bin's windows).
//!
//! The planner takes the signature groups from the widest signature down (first touch breaks ties), and puts
//! each group whole into the existing bin it adds the fewest modes to (the first such bin), or else into
//! fresh bins, split only where a bin is full. A group whose signature is inside a bin's modes costs
//! nothing there, so nested signatures share bins. The gap between a bin's end and the next boundary is
//! filled best-fit (largest first) from cold sections of the members every program links; what is left is
//! the residual fill. [`Plan::bound`] is a lower bound per mode (its bytes over [`BIN`], rounded up), so
//! the plan's distance from optimal is printed, not assumed.
//!
//! LAYOUT-WINDOW-BINS-5: the last bin is filled to its boundary too, and the start file ends the group there
//! ([`GROUP_END`]), so a program's own text after the group (the probe's `main`, compiled into `.text`)
//! starts on a window boundary instead of wherever the last bin happens to end: a `main` across that
//! boundary cost every mode a second fault for it (start-up 12 against 11, fast products).
//!
//! LAYOUT-WINDOW-BINS-4: a phase mode counts only the faults after its warm-up, so its signatures carry the
//! warm-up as a mode of its own ([`WARM_UP`]); the planner groups by the modes that count a section
//! ([`counted`]) and charges a phase mode nothing in a bin its warm-up runs in, so the measured phase's
//! sections join the bins the warm-up faulted when they have room. (Placing them there ahead of their
//! turn was tried in LAYOUT-WINDOW-BINS-4: it pushed start-up and printf text out of the head bins.)

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// The fault-around window of the reference hosts (64 KiB), the bin capacity and alignment.
pub const BIN: u64 = 0x10000;
/// The generated signatures list, next to `hot-text.list` (one of [`crate::hotlists::GENERATED`]).
pub const SIGNATURES: &str = "hot-text.signatures";
/// The plan `so` writes next to the lists (never committed).
pub const PLAN: &str = "layout-bins.tsv";
/// The mode every program runs (start-up): its sections' members are linked by every program.
pub const STARTUP_MODE: &str = "true";

/// One grouped section: its name in the archive, bytes, alignment, list position and signature.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub name: String,
    pub bytes: u64,
    pub align: u64,
    pub pos: usize,
    pub sig: BTreeSet<String>,
}

/// One cold section a bin's gap may take: name, bytes, alignment.
#[derive(Clone, Debug, PartialEq)]
pub struct Cold {
    pub name: String,
    pub bytes: u64,
    pub align: u64,
}

/// One bin: its members (indices into the items, list order) and fillers (indices into the cold pool,
/// in placement order), the members' extent, the end after the fillers, the modes that touch it, and
/// whether what follows it (the next bin, or after the last one the program's text, [`GROUP_END`]) starts
/// on a boundary (a gap to fill; every bin of a plan has one since LAYOUT-WINDOW-BINS-5).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bin {
    pub members: Vec<usize>,
    pub fillers: Vec<usize>,
    pub used: u64,
    pub end: u64,
    pub modes: BTreeSet<String>,
    pub gap: bool,
    /// Where the fillers must end: the next boundary (less [`HEAD_ROOM`] for the head bin); 0 without a gap.
    pub limit: u64,
}

impl Bin {
    /// The windows the bin's members span from a [`BIN`] boundary (at least one).
    pub fn windows(&self) -> u64 {
        self.used.div_ceil(BIN).max(1)
    }

    /// The bytes from the end to [`Bin::limit`] that no filler took (none without a gap).
    pub fn residual(&self) -> u64 {
        if self.gap { self.limit.saturating_sub(self.end) } else { 0 }
    }
}

/// A plan: the bins in link order.
#[derive(Debug, PartialEq)]
pub struct Plan {
    pub bins: Vec<Bin>,
}

/// Where a section of `bytes` and `align` ends when laid out from offset `off`.
fn place(off: u64, bytes: u64, align: u64) -> u64 {
    off.next_multiple_of(align.max(1)) + bytes
}

/// The extent of `idx` (indices into `items`) laid out in list order from a [`BIN`] boundary.
fn extent(items: &[Item], idx: &[usize]) -> u64 {
    let mut v = idx.to_vec();
    v.sort_by_key(|&i| (items[i].pos, i));
    v.iter().fold(0, |off, &i| place(off, items[i].bytes, items[i].align))
}

/// One section's place under an applied plan ([`Plan::placements`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub bin: usize,
    pub filler: bool,
    /// Index into the items (a member) or the cold pool (a filler).
    pub index: usize,
    /// The list position (a member) or the filler's order in its bin.
    pub place: usize,
    /// The alignment to give the section, 0 to keep its own.
    pub align: u64,
}

/// Whether the head bin (the start-up bin, first in the group) starts on a [`BIN`] boundary too
/// (variant i of plan/layout-budget-rethink.md section 3). No: a static program's first text fault maps
/// the 64 KiB from the start of its executable segment, which the group follows by a few bytes (crt1,
/// `.init`, the PLTs), so the head bin is in that first window as long as it fits; anchoring it at the
/// next boundary would put it half in that window and half in the next.
pub const HEAD_ANCHOR: bool = false;

/// The bytes the head bin leaves free in its window when it is not anchored ([`HEAD_ANCHOR`]): room for
/// what a program links ahead of the group (the static probe: 64 bytes), so the head bin and its fillers
/// end inside the first window and the next bin's boundary is the one after it. A program with more
/// ahead of the group (its own `.text.startup`) may see the head bin in two windows: a count fixed by
/// that program and the start file, not by the bins' contents.
pub const HEAD_ROOM: u64 = if HEAD_ANCHOR { 0 } else { 0x400 };

/// The bytes bin `n` may hold: [`BIN`], less [`HEAD_ROOM`] for the head bin.
fn capacity(n: usize) -> u64 {
    if n == 0 { BIN - HEAD_ROOM } else { BIN }
}

/// The highest bin number a section name can carry ([`section`]: two digits).
pub const MAX_BINS: usize = 100;

/// The name of function `f`'s section placed in `bin` (member or filler) at `place`:
/// `.text.sorted.h9<bin, 2 digits><0 member | 1 filler><place, 5 digits>.<f>`. GNU ld's
/// `SORT(.text.sorted.*)` orders them by bin, members before fillers, then by place; the leading 9
/// sorts every bin after a five-digit list position.
pub fn section(bin: usize, filler: bool, place: usize, f: &str) -> String {
    format!(".text.sorted.h9{bin:02}{}{place:05}.{f}", u8::from(filler))
}

/// The empty, [`BIN`]-aligned section the start file (`csys/src/machine/entry.S`) carries to end the
/// group (LAYOUT-WINDOW-BINS-5): its name sorts after every [`section`] name (`z` after any digit), so GNU
/// ld places it after the last bin and starts the program's own `.text` on the next window boundary.
pub const GROUP_END: &str = ".text.sorted.h9z.niji_group_end";

/// The suffix of a phase mode's warm-up mode in the signatures (LAYOUT-WINDOW-BINS-4). A **phase mode** of
/// `page_faults.c` (one that prints `f1 - f0`) counts only the faults of its measured phase, after its
/// warm-up; `tools/hottext/regen.sh` writes the names the warm-up runs (up to the first `getrusage`, the
/// `minflt()` at f0) as the mode `<mode>.pre`. Every window a warm-up section is in has faulted before the
/// phase starts, so the phase's own sections cost nothing there.
pub const WARM_UP: &str = ".pre";

/// Mode `m`'s warm-up mode.
fn warm_up(m: &str) -> String {
    format!("{m}{WARM_UP}")
}

/// The counted modes of signature `sig`: the modes whose fault count a section of it is in. A warm-up mode
/// counts nothing, and a phase mode whose warm-up runs the section too does not count it.
pub fn counted(sig: &BTreeSet<String>) -> BTreeSet<String> {
    sig.iter().filter(|m| !m.ends_with(WARM_UP) && !sig.contains(&warm_up(m))).cloned().collect()
}

/// The counted modes of `sig` that a bin whose sections run `modes` would newly charge: those that run
/// nothing there yet. A bin its warm-up runs in charges a phase mode nothing, and needs no test of its own
/// here: the warm-up is a prefix of the mode's trace, so every signature holding `<m>.pre` holds `m`.
fn added(sig: &BTreeSet<String>, modes: &BTreeSet<String>) -> usize {
    sig.difference(modes).count()
}

/// Whether bin `n` can take the sections `rest` besides its members.
fn fits(items: &[Item], b: &Bin, n: usize, rest: &[usize]) -> bool {
    extent(items, &[b.members.as_slice(), rest].concat()) <= capacity(n)
}

/// Moves `rest` into bin `b`, whose modes then include every mode of their signatures (warm-ups too).
fn join(items: &[Item], b: &mut Bin, rest: &mut Vec<usize>) {
    for &i in rest.iter() {
        b.modes.extend(items[i].sig.iter().cloned());
    }
    b.members.append(rest);
}

/// Places the group `rest` of counted signature `sig`: whole into the existing bin it charges the fewest
/// modes (the first such), or else into fresh bins, the longest prefix (list order) that fits each.
fn put(items: &[Item], bins: &mut Vec<Bin>, sig: &BTreeSet<String>, mut rest: Vec<usize>) {
    while !rest.is_empty() {
        let best = bins
            .iter()
            .enumerate()
            .filter(|(n, b)| fits(items, b, *n, &rest))
            .min_by_key(|(n, b)| (added(sig, &b.modes), *n))
            .map(|(n, _)| n);
        if let Some(n) = best {
            join(items, &mut bins[n], &mut rest);
            return;
        }
        let mut off = 0;
        let mut take = 0;
        let cap = capacity(bins.len());
        while take < rest.len() {
            let next = place(off, items[rest[take]].bytes, items[rest[take]].align);
            if next > cap && take > 0 {
                break;
            }
            off = next;
            take += 1;
        }
        let mut b = Bin::default();
        join(items, &mut b, &mut rest.drain(..take).collect());
        bins.push(b);
    }
}

/// The window-bin plan of `items` with fillers from `cold` (see the module comment). Groups are the
/// sections of one counted signature ([`counted`], LAYOUT-WINDOW-BINS-4): a warm-up section no counted
/// mode measures waits with the unmeasured ones, and a measured phase's sections stay one group whatever
/// other warm-ups run them. The start-up groups go first (the head bin), then the widest.
pub fn plan(items: &[Item], cold: &[Cold]) -> Plan {
    let mut groups: BTreeMap<BTreeSet<String>, Vec<usize>> = BTreeMap::new();
    for (i, it) in items.iter().enumerate() {
        groups.entry(counted(&it.sig)).or_default().push(i);
    }
    let mut order: Vec<(BTreeSet<String>, Vec<usize>)> = groups
        .into_iter()
        .map(|(s, mut v)| {
            v.sort_by_key(|&i| (items[i].pos, i));
            (s, v)
        })
        .collect();
    order.sort_by_key(|(s, v)| (!s.contains(STARTUP_MODE), Reverse(s.len()), items[v[0]].pos));
    let mut bins: Vec<Bin> = Vec::new();
    for (sig, rest) in order {
        put(items, &mut bins, &sig, rest);
    }
    for b in &mut bins {
        b.members.sort_by_key(|&i| (items[i].pos, i));
        b.used = extent(items, &b.members);
        b.end = b.used;
    }
    // Fillers: best fit, largest first, into every gap, the last bin's included: the start file aligns
    // what follows the group to the next boundary ([`GROUP_END`]), so that gap is padding unless filled.
    let mut pool: Vec<usize> = (0..cold.len()).filter(|&c| cold[c].bytes > 0 && cold[c].align <= BIN).collect();
    pool.sort_by(|&a, &b| cold[b].bytes.cmp(&cold[a].bytes).then_with(|| cold[a].name.cmp(&cold[b].name)));
    let mut taken = vec![false; cold.len()];
    for (n, b) in bins.iter_mut().enumerate() {
        b.gap = true;
        let limit = b.used.max(1).next_multiple_of(BIN) - (BIN - capacity(n));
        b.limit = limit;
        while let Some(&c) = pool.iter().find(|&&c| !taken[c] && place(b.end, cold[c].bytes, cold[c].align) <= limit) {
            taken[c] = true;
            b.end = place(b.end, cold[c].bytes, cold[c].align);
            b.fillers.push(c);
        }
    }
    Plan { bins }
}

impl Plan {
    /// Mode -> the windows it is charged under the plan: each bin it touches counts its windows, except,
    /// for a phase mode, a bin its warm-up touches too ([`WARM_UP`]: those faulted before the phase).
    /// Warm-up modes are not listed.
    pub fn windows(&self) -> BTreeMap<String, u64> {
        let mut out = BTreeMap::new();
        for b in &self.bins {
            for m in b.modes.iter().filter(|m| !m.ends_with(WARM_UP)) {
                let w = if b.modes.contains(&warm_up(m)) { 0 } else { b.windows() };
                *out.entry(m.clone()).or_insert(0) += w;
            }
        }
        out
    }

    /// Where the plan puts each section, in link order: (bin, member or filler, index into the items or
    /// the cold pool, place in the bin, alignment to set or 0 to keep its own). The place is the list
    /// position for a member and the filler's order for a filler, so [`section`] names sort into
    /// exactly this order. The first member with bytes of every bin that follows a gap (and of the head
    /// bin when `anchor`) is aligned to [`BIN`]: that bin then starts on a boundary wherever the
    /// program puts the group, so no byte outside it moves its windows.
    pub fn placements(&self, items: &[Item], anchor: bool) -> Vec<Placement> {
        let mut out = Vec::new();
        for (n, b) in self.bins.iter().enumerate() {
            let aligned = if n == 0 { anchor } else { self.bins[n - 1].gap };
            let head = if aligned { b.members.iter().copied().find(|&i| items[i].bytes > 0) } else { None };
            for &i in &b.members {
                let align = if Some(i) == head && items[i].align < BIN { BIN } else { 0 };
                out.push(Placement { bin: n, filler: false, index: i, place: items[i].pos, align });
            }
            for (k, &c) in b.fillers.iter().enumerate() {
                out.push(Placement { bin: n, filler: true, index: c, place: k, align: 0 });
            }
        }
        out
    }

    /// Mode -> a lower bound of its windows over the group: the bytes of its sections over [`BIN`],
    /// rounded up (alignment padding left out, so it never exceeds the optimum). A phase mode's bound is
    /// 0: its phase's sections may all share windows its warm-up faulted.
    pub fn bound(items: &[Item]) -> BTreeMap<String, u64> {
        let mut bytes: BTreeMap<String, u64> = BTreeMap::new();
        for it in items {
            for m in it.sig.iter().filter(|m| !m.ends_with(WARM_UP)) {
                *bytes.entry(m.clone()).or_insert(0) += it.bytes;
            }
        }
        let phase: BTreeSet<&str> = items.iter().flat_map(|it| it.sig.iter()).filter_map(|m| m.strip_suffix(WARM_UP)).collect();
        bytes.into_iter().map(|(m, b)| (m.clone(), if phase.contains(m.as_str()) { 0 } else { b.div_ceil(BIN) })).collect()
    }

    /// The plan as `layout-bins.tsv`: `<bin> TAB <section> TAB <bytes> TAB <signature>`, members then
    /// fillers per bin (signature `-` for a member no counted mode runs, `cold` for a filler).
    pub fn tsv(&self, items: &[Item], cold: &[Cold]) -> String {
        let mut out = String::from(
            "# Written by cargo xtask so (LAYOUT-WINDOW-BINS, D-X.103): the window-bin plan of the hot group, as applied.\n\
             # <bin> TAB <section> TAB <bytes> TAB <signature: page_faults modes, comma-separated; - none; cold = a filler>\n",
        );
        for (n, b) in self.bins.iter().enumerate() {
            for &i in &b.members {
                let sig = if items[i].sig.is_empty() { "-".to_string() } else { items[i].sig.iter().cloned().collect::<Vec<_>>().join(",") };
                let _ = writeln!(out, "{n}\t{}\t{}\t{sig}", items[i].name, items[i].bytes);
            }
            for &c in &b.fillers {
                let _ = writeln!(out, "{n}\t{}\t{}\tcold", cold[c].name, cold[c].bytes);
            }
        }
        out
    }

    /// The summary `so` prints: one line per bin, then one per mode (planned windows and the bound).
    pub fn summary(&self, items: &[Item], cold: &[Cold]) -> Vec<String> {
        let mut out = Vec::new();
        let (mut fill, mut residual) = (0, 0);
        for (n, b) in self.bins.iter().enumerate() {
            let filled: u64 = b.fillers.iter().map(|&c| cold[c].bytes).sum();
            fill += filled;
            residual += b.residual();
            out.push(format!(
                "so: window bin {n}: {} sections, {} bytes ({} windows), {} fillers {filled} bytes, residual {} bytes, modes {}",
                b.members.len(),
                b.used,
                b.windows(),
                b.fillers.len(),
                b.residual(),
                b.modes.iter().cloned().collect::<Vec<_>>().join(",")
            ));
        }
        let bound = Plan::bound(items);
        let planned = self.windows();
        let text: Vec<String> = planned.iter().map(|(m, w)| format!("{m} {w}/{}", bound.get(m).copied().unwrap_or(0))).collect();
        out.push(format!(
            "so: window bins (applied): {} bins, {} sections, fillers {fill} bytes, residual {residual} bytes; windows/bound per mode: {}",
            self.bins.len(),
            items.len(),
            text.join(" ")
        ));
        out
    }
}

/// The lines of [`SIGNATURES`]: `<key> TAB <modes>` (comma-separated, `-` for none); `#` comments and blank
/// lines skipped. Errors name the line.
pub fn parse_signatures(text: &str) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let mut out = BTreeMap::new();
    for (n, l) in text.lines().enumerate() {
        if l.trim().is_empty() || l.starts_with('#') {
            continue;
        }
        let bad = || format!("{SIGNATURES} line {}: {l:?}: want <key>\\t<modes, comma-separated, or ->", n + 1);
        let [k, m] = l.split('\t').collect::<Vec<_>>()[..] else { return Err(bad()) };
        if k.is_empty() || m.is_empty() {
            return Err(bad());
        }
        let modes: BTreeSet<String> = if m == "-" { BTreeSet::new() } else { m.split(',').map(str::to_string).collect() };
        if modes.iter().any(|x| x.is_empty() || x == "-") {
            return Err(bad());
        }
        out.insert(k.to_string(), modes);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{parse_signatures, plan, Cold, Item, Plan, BIN, GROUP_END, HEAD_ROOM, section, MAX_BINS};
    use std::collections::BTreeSet;

    fn item(name: &str, bytes: u64, align: u64, pos: usize, sig: &[&str]) -> Item {
        Item { name: name.into(), bytes, align, pos, sig: sig.iter().map(|s| s.to_string()).collect() }
    }

    fn cold(name: &str, bytes: u64, align: u64) -> Cold {
        Cold { name: name.into(), bytes, align }
    }

    fn names(p: &Plan, items: &[Item]) -> Vec<Vec<String>> {
        p.bins.iter().map(|b| b.members.iter().map(|&i| items[i].name.clone()).collect()).collect()
    }

    #[test]
    fn nested_signatures_share_a_bin() {
        // LAYOUT-WINDOW-BINS (no spec clause: build tooling, D-X.103): a section whose modes are inside a
        // bin's modes joins that bin (no mode touches another window), whatever the list order; a
        // disjoint signature that no longer fits goes to its own bin.
        let items = [
            item("a_only", 0x1000, 1, 0, &["a"]),
            item("all", 0x4000, 1, 1, &["a", "b", "true"]),
            item("b_only", 0x1000, 1, 2, &["b"]),
            item("c_big", 0xc000, 1, 3, &["c"]),
            item("c_more", 0x2000, 1, 4, &["c"]),
        ];
        let p = plan(&items, &[]);
        assert_eq!(names(&p, &items), [vec!["a_only", "all", "b_only"], vec!["c_big", "c_more"]]);
        let w = p.windows();
        assert_eq!((w["a"], w["b"], w["c"], w["true"]), (1, 1, 1, 1));
        assert_eq!(Plan::bound(&items)["c"], 1);
    }

    #[test]
    fn a_group_splits_only_where_a_bin_is_full() {
        // LAYOUT-WINDOW-BINS (no spec clause: build tooling, D-X.103): three 0x6000-byte sections of one
        // signature take two bins (two, then one), not three; a later narrower group fills the second
        // bin's room instead of opening a third. Every mode then meets its lower bound.
        let items = [
            item("s0", 0x6000, 1, 0, &["a", "b"]),
            item("s1", 0x6000, 1, 1, &["a", "b"]),
            item("s2", 0x6000, 1, 2, &["a", "b"]),
            item("n0", 0x8000, 1, 3, &["a"]),
        ];
        let p = plan(&items, &[]);
        assert_eq!(names(&p, &items), [vec!["s0", "s1"], vec!["s2", "n0"]]);
        assert_eq!(p.windows(), Plan::bound(&items));
        assert_eq!(p.bins.iter().map(|b| b.used).collect::<Vec<_>>(), [0xc000, 0xe000]);
    }

    #[test]
    fn alignment_counts_in_a_bin() {
        // LAYOUT-WINDOW-BINS (no spec clause: build tooling, D-X.103): a bin's extent includes the
        // padding its members' alignments ask for, so a pair whose bytes fit but whose aligned extent
        // does not goes to two bins; the extent follows list order, not the order of the input.
        let items = [item("b", 0x7fff, 0x1000, 1, &["a"]), item("a", 0x8001, 1, 0, &["a"])];
        let p = plan(&items, &[]);
        assert_eq!(p.bins.len(), 2, "{:?}", p.bins);
        let fit = [item("b", 0x7fff, 0x1000, 1, &["a"]), item("a", 0x7000, 1, 0, &["a"])];
        let p = plan(&fit, &[]);
        assert_eq!(p.bins.len(), 1);
        assert_eq!(p.bins[0].used, 0x7000 + 0x7fff);
        assert_eq!(p.bins[0].members, [1, 0]);
    }

    #[test]
    fn fillers_take_the_gap_best_fit_and_never_twice() {
        // LAYOUT-WINDOW-BINS (no spec clause: build tooling, D-X.103): the gap to the next boundary is
        // filled largest-first with cold sections that fit (alignment included); a cold section fills
        // one gap only. The last bin fills its gap too (LAYOUT-WINDOW-BINS-5: the group ends on a boundary).
        let items = [item("x", 0xec00, 1, 0, &["a"]), item("y", 0xf800, 1, 1, &["b"]), item("z", 0x1100, 1, 2, &["c"])];
        let pool = [cold("big", 0x2000, 1), cold("m", 0x800, 1), cold("s", 0x680, 1), cold("t", 0x100, 0x100), cold("u", 0x80, 1), cold("z0", 0, 1)];
        let p = plan(&items, &pool);
        assert_eq!(p.bins.len(), 3);
        assert_eq!(p.bins[0].fillers, [1, 2, 3]);
        assert_eq!(p.bins[0].end, BIN - HEAD_ROOM, "t aligned to 0x100 after m and s; the head bin leaves HEAD_ROOM");
        assert_eq!(p.bins[0].residual(), 0);
        assert_eq!(p.bins[1].fillers, [4], "m would fit bin 1 exactly but already fills bin 0");
        assert_eq!(p.bins[1].residual(), 0x800 - 0x80);
        assert_eq!((p.bins[2].fillers.as_slice(), p.bins[2].limit), (&[0][..], BIN), "big did not fit bins 0 and 1");
        let tsv = p.tsv(&items, &pool);
        assert!(tsv.contains("0\tm\t2048\tcold\n") && tsv.contains("2\tz\t4352\tc\n"), "{tsv}");
    }

    #[test]
    fn every_bin_fills_to_its_boundary_and_the_group_end_sorts_last() {
        // LAYOUT-WINDOW-BINS-5 (no spec clause: build tooling, D-X.103): sections no counted mode runs come
        // after the counted bins; every bin, theirs and the last included, has a gap to its boundary that
        // fillers take, because the start file's GROUP_END starts what follows the group on the next
        // boundary (a probe `main` after an unfilled last bin spanned two windows). GROUP_END sorts after
        // every bin section name, so GNU ld places it last.
        let items = [item("x", 0xf000, 1, 0, &["a"]), item("u0", 0x9000, 1, 1, &[]), item("u1", 0x9000, 1, 2, &[])];
        let pool = [cold("c", 0x800, 1), cold("d", 0x6000, 1)];
        let p = plan(&items, &pool);
        assert_eq!(names(&p, &items), [vec!["x"], vec!["u0"], vec!["u1"]]);
        assert!(p.bins.iter().enumerate().all(|(n, b)| b.gap && b.limit == BIN - if n == 0 { HEAD_ROOM } else { 0 }), "{:?}", p.bins);
        let filled: Vec<&[usize]> = p.bins.iter().map(|b| b.fillers.as_slice()).collect();
        assert_eq!(filled, [&[0][..], &[1][..], &[][..]], "the trailing bins take fillers too");
        assert_eq!(p.bins[2].residual(), BIN - 0x9000);
        for s in [section(0, false, 0, "a"), section(MAX_BINS - 1, true, 99_999, "zzz"), ".text.sorted.h99999.zzz".to_string()] {
            assert!(GROUP_END > s.as_str(), "{GROUP_END} sorts before {s}");
        }
        assert!(GROUP_END.starts_with(".text.sorted."));
    }

    #[test]
    fn the_plan_does_not_depend_on_input_order() {
        // LAYOUT-WINDOW-BINS (no spec clause: build tooling, D-X.103): the plan is a function of the
        // sections (signature, list position, size), not of the order the archive lists them in.
        let items = [
            item("p0", 0x9000, 1, 0, &["a", "b", "true"]),
            item("p1", 0x9000, 1, 1, &["a"]),
            item("p2", 0x3000, 1, 2, &["b"]),
            item("p3", 0x5000, 1, 3, &[]),
            item("p4", 0x2000, 1, 4, &["a", "b", "true"]),
        ];
        let mut rev = items.to_vec();
        rev.reverse();
        let (a, b) = (plan(&items, &[]), plan(&rev, &[]));
        assert_eq!(names(&a, &items), names(&b, &rev));
        assert_eq!(names(&a, &items), [vec!["p0", "p2", "p4"], vec!["p1", "p3"]]);
    }

    #[test]
    fn placements_name_sections_in_link_order_and_align_each_bin_after_a_gap() {
        // LAYOUT-WINDOW-BINS slice 2 (no spec clause: build tooling, D-X.103): the names sort (as GNU ld's
        // SORT(.text.sorted.*) does) into bin order, members by list position, then the bin's fillers; the
        // first member with bytes of every bin after a gap asks for a 64 KiB start, the head bin's does not
        // (HEAD_ANCHOR); a bin no counted mode touches gets it too (LAYOUT-WINDOW-BINS-5: every bin has a gap).
        let items = [
            item("h0", 0, 1, 0, &["a", "b", "true"]),
            item("h1", 0x8000, 16, 1, &["a", "b", "true"]),
            item("a0", 0, 1, 2, &["a"]),
            item("a1", 0xa000, 64, 3, &["a"]),
            item("b0", 0xb000, 1, 4, &["b"]),
            item("u0", 0xf000, 1, 5, &[]),
        ];
        let pool = [cold("f0", 0x800, 1), cold("f1", 0x400, 1)];
        let p = plan(&items, &pool);
        assert_eq!(names(&p, &items), [vec!["h0", "h1"], vec!["a0", "a1"], vec!["b0"], vec!["u0"]]);
        let pl = p.placements(&items, false);
        let named: Vec<String> = pl.iter().map(|x| section(x.bin, x.filler, x.place, if x.filler { &pool[x.index].name } else { &items[x.index].name })).collect();
        let mut sorted = named.clone();
        sorted.sort();
        assert_eq!(named, sorted, "the plan's order is the name order");
        assert_eq!(named[0], ".text.sorted.h900000000.h0");
        assert!(named.contains(&".text.sorted.h900100000.f0".to_string()) && named.contains(&".text.sorted.h900100001.f1".to_string()), "{named:?}");
        let aligned: Vec<(&str, u64)> = pl.iter().filter(|x| x.align > 0).map(|x| (items[x.index].name.as_str(), x.align)).collect();
        assert_eq!(aligned, [("a1", BIN), ("b0", BIN), ("u0", BIN)], "a zero-byte member never carries the alignment");
        assert!(p.placements(&items, true).iter().any(|x| x.index == 1 && !x.filler && x.align == BIN), "an anchored head bin");
        assert!(p.bins.len() <= MAX_BINS);
    }

    #[test]
    fn the_head_bin_leaves_room_for_what_precedes_the_group() {
        // LAYOUT-WINDOW-BINS slice 2 (no spec clause: build tooling, D-X.103): an unanchored head bin
        // holds at most BIN - HEAD_ROOM bytes, so it ends inside the first window of a program whose
        // start file and PLTs precede the group; a group that fits a later bin still fills that one whole.
        let items = [item("t0", BIN - HEAD_ROOM, 1, 0, &["true"]), item("t1", 0x10, 1, 1, &["true"]), item("x", BIN, 1, 2, &["a"])];
        let p = plan(&items, &[]);
        assert_eq!(names(&p, &items), [vec!["t0"], vec!["t1"], vec!["x"]]);
        assert_eq!(p.bins[0].limit, BIN - HEAD_ROOM);
    }

    #[test]
    fn a_measured_phase_shares_its_warm_up_windows() {
        // LAYOUT-WINDOW-BINS-4 (no spec clause: build tooling, D-X.103): phase mode `s` counts only what
        // runs after its warm-up (`s.pre`). The warm-up's own section `r` is measured by no mode, so it
        // waits; the phase's section `p` joins the head bin, which the warm-up (start-up) runs in, and
        // `s` is charged no window. Read as whole-process traces (no `s.pre`), `r` and `p` are one group
        // of `s` that no longer fits the head bin: two windows for `s`.
        let items = [
            item("t", 0x8000, 1, 0, &["true", "a", "s", "s.pre"]),
            item("r", 0x7000, 1, 1, &["s", "s.pre"]),
            item("p", 0x2000, 1, 2, &["s"]),
            item("w", 0x6000, 1, 3, &["a"]),
        ];
        let p = plan(&items, &[]);
        assert_eq!(names(&p, &items), [vec!["t", "p"], vec!["r", "w"]]);
        let w = p.windows();
        assert_eq!((w["s"], w["a"], w["true"]), (0, 2, 1), "{w:?}");
        assert!(!w.contains_key("s.pre"), "a warm-up mode is not charged");
        assert_eq!(Plan::bound(&items)["s"], 0);
        let plain: Vec<Item> = items.iter().map(|it| Item { sig: it.sig.iter().filter(|m| *m != "s.pre").cloned().collect(), ..it.clone() }).collect();
        let q = plan(&plain, &[]);
        assert_eq!(names(&q, &plain), [vec!["t", "w"], vec!["r", "p"]]);
        assert_eq!(q.windows()["s"], 2);
    }

    #[test]
    fn the_start_up_group_takes_the_head_bin_before_wider_groups() {
        // LAYOUT-WINDOW-BINS-4 (no spec clause: build tooling, D-X.103): with phase modes left out of the
        // start-up sections' counted modes, another group can be wider than start-up; start-up still
        // takes the head bin, so `true` stays in one window.
        let items = [item("w", 0x2000, 1, 0, &["a", "b"]), item("t", 0xf000, 1, 1, &["true"])];
        let p = plan(&items, &[]);
        assert_eq!(names(&p, &items), [vec!["t"], vec!["w"]]);
        assert_eq!(p.windows()["true"], 1);
    }

    #[test]
    fn a_phase_split_by_its_warm_up_is_one_group() {
        // LAYOUT-WINDOW-BINS-4 (no spec clause: build tooling, D-X.103): sections the phase of `f` runs
        // are one group whatever else runs them in a warm-up (`f2.pre`), so they share one fresh bin,
        // and a section of the warm-up only is charged to no mode.
        let items = [
            item("t", 0xf000, 1, 0, &["true", "f2", "f2.pre", "f", "f.pre"]),
            item("x", 0x6000, 1, 1, &["f", "f2", "f2.pre"]),
            item("g", 0x9000, 1, 2, &["g"]),
            item("y", 0x8000, 1, 3, &["f"]),
            item("r", 0x800, 1, 4, &["f2", "f2.pre"]),
        ];
        let p = plan(&items, &[]);
        assert_eq!(names(&p, &items), [vec!["t", "r"], vec!["x", "y"], vec!["g"]]);
        let w = p.windows();
        assert_eq!((w["f"], w["f2"], w["g"]), (1, 0, 1), "{w:?}");
        assert_eq!(super::counted(&items[4].sig), BTreeSet::new());
    }

    #[test]
    fn signatures_parse_and_refuse_bad_lines() {
        // LAYOUT-WINDOW-BINS (no spec clause: build tooling, D-X.103): `-` is the empty signature; a line
        // without exactly two fields, an empty mode or `-` among modes is refused by line number.
        let s = parse_signatures("# c\nk1\ttrue,locale\nk2\t-\n").unwrap();
        assert_eq!(s["k1"], ["locale", "true"].iter().map(|x| x.to_string()).collect::<BTreeSet<_>>());
        assert!(s["k2"].is_empty());
        for bad in ["k1\n", "k1\ta\tb\n", "k1\ta,,b\n", "k1\ta,-\n", "\ta\n"] {
            let e = parse_signatures(bad).unwrap_err();
            assert!(e.contains("line 1"), "{bad:?}: {e}");
        }
        assert_eq!(BIN, 0x10000);
    }
}
