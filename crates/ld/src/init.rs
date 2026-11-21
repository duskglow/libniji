//! Constructor and destructor order (phase 14 DL-11, plan/ph14/design.md §9, D-27.19). Pure logic
//! over object indices and dynamic arrays; the boundary (`boundary/src/ld.rs`) reads the arrays and
//! makes the calls.
//!
//! * [`init_order`]: the order objects are initialised in. Objects are indexed in load order (0 is
//!   the root: the program at start, the object named to `dlopen` later); `edges` are the
//!   `DT_NEEDED` links `(dependent, dependency)` in the order each object names them. The walk is
//!   depth first over the objects in *reverse* load order, each object placed after the objects it
//!   needs (post-order); a dependency already being walked (a cycle) is passed over; the root is never
//!   entered from another object and comes last. Probe-settled (tools/probe/dl/order.sh against the
//!   reference loader: a diamond, ties between siblings, a dependency loaded through a later sibling,
//!   two- and three-object cycles at start; DL-0b's record for a cycle opened by `dlopen`).
//! * [`fini_order`]: the reverse of [`init_order`] over every object loaded at the time (DL-0b's
//!   exit record: an object opened later is placed by its dependencies, not by when it was opened).
//! * [`hooks`]: an object's `DT_PREINIT_ARRAY` (the program's only: a shared object's is ignored, as
//!   by the reference), `DT_INIT`, `DT_INIT_ARRAY`, `DT_FINI_ARRAY`, `DT_FINI`.
//! * [`init_calls`] (`DT_INIT`, then `DT_INIT_ARRAY` in order) and [`fini_calls`] (`DT_FINI_ARRAY`
//!   in reverse, then `DT_FINI`): the functions to call for one object (ELF gABI "Initialization
//!   and Termination Functions").

use crate::elf::{dt, Dynamic};

/// Why an order cannot be computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderError {
    /// More objects than the caller's storage holds (`out` or `work` shorter than `n`; for
    /// [`exit_order`], more than [`EXIT_MAX`] units).
    TooMany,
    /// An edge names an object outside `0..n`.
    BadEdge,
}

/// Walk state of one object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mark {
    #[default]
    New,
    Active,
    Done,
}

/// The working storage of one [`init_order`] walk, one entry per object (the caller's, so the
/// number of objects has no bound but the caller's memory, DL-STARTUP-OBJECTS): the object's walk
/// state and one frame of the walk's stack, (object, index of the next edge to look at).
#[derive(Clone, Copy, Debug, Default)]
pub struct Walk {
    mark: Mark,
    frame: (usize, usize),
}

/// The initialisation order of objects `0..n` (see the module documentation) into `out`; returns
/// the number of entries (`n`). `edges` are `(dependent, dependency)` indices of any width;
/// `work` is the walk's storage (its contents on entry do not matter).
///
/// # Errors
///
/// [`OrderError`] for `out` or `work` shorter than `n` or an edge outside `0..n`.
pub fn init_order<E: Copy + Into<usize>>(n: usize, edges: &[(E, E)], out: &mut [usize], work: &mut [Walk]) -> Result<usize, OrderError> {
    let (Some(out), Some(work)) = (out.get_mut(..n), work.get_mut(..n)) else { return Err(OrderError::TooMany) };
    if edges.iter().any(|&(a, b)| a.into() >= n || b.into() >= n) {
        return Err(OrderError::BadEdge);
    }
    if n == 0 {
        return Ok(0);
    }
    work.fill(Walk::default());
    // The root is held back: never entered through an edge, placed last.
    if let Some(w) = work.first_mut() {
        w.mark = Mark::Active;
    }
    let mut len = 0usize;
    let mut emit = |v: usize, out: &mut [usize]| {
        if let Some(slot) = out.get_mut(len) {
            *slot = v;
            len = len.saturating_add(1);
        }
    };
    for start in (1..n).rev() {
        if work.get(start).map(|w| w.mark) != Some(Mark::New) {
            continue;
        }
        let mut depth = 0usize;
        push(work, &mut depth, start);
        while let Some((v, from)) = depth.checked_sub(1).and_then(|t| work.get(t)).map(|w| w.frame) {
            let next = edges.iter().enumerate().skip(from).find(|(_, e)| e.0.into() == v);
            match next {
                Some((k, &(_, w))) => {
                    if let Some(top) = depth.checked_sub(1).and_then(|t| work.get_mut(t)) {
                        top.frame.1 = k.saturating_add(1);
                    }
                    let w = w.into();
                    if work.get(w).map(|x| x.mark) == Some(Mark::New) {
                        push(work, &mut depth, w);
                    }
                }
                None => {
                    depth = depth.saturating_sub(1);
                    if let Some(x) = work.get_mut(v) {
                        x.mark = Mark::Done;
                    }
                    emit(v, out);
                }
            }
        }
    }
    emit(0, out);
    Ok(len)
}

/// Push `v` on the walk's stack and mark it active. The stack never holds more than `n` frames:
/// each object is pushed once (it leaves state [`Mark::New`] here).
fn push(work: &mut [Walk], depth: &mut usize, v: usize) {
    if let Some(slot) = work.get_mut(*depth) {
        slot.frame = (v, 0);
        *depth = depth.saturating_add(1);
    }
    if let Some(x) = work.get_mut(v) {
        x.mark = Mark::Active;
    }
}

/// The finalisation order of objects `0..n`: [`init_order`] reversed.
///
/// # Errors
///
/// As [`init_order`].
pub fn fini_order<E: Copy + Into<usize>>(n: usize, edges: &[(E, E)], out: &mut [usize], work: &mut [Walk]) -> Result<usize, OrderError> {
    let len = init_order(n, edges, out, work)?;
    if let Some(s) = out.get_mut(..len) {
        s.reverse();
    }
    Ok(len)
}

/// An array of function addresses in an object: run-time address and number of entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Array {
    /// Run-time address of the first entry.
    pub addr: u64,
    /// Number of 8-byte entries.
    pub len: u64,
}

/// An object's initialisation and termination functions (run-time addresses; 0 = none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hooks {
    /// `DT_PREINIT_ARRAY`: the program's only.
    pub preinit: Array,
    /// `DT_INIT`.
    pub init: u64,
    /// `DT_INIT_ARRAY`.
    pub init_array: Array,
    /// `DT_FINI_ARRAY`.
    pub fini_array: Array,
    /// `DT_FINI`.
    pub fini: u64,
}

/// The hooks of the object with dynamic array `d` and load bias `bias`; `program` says whether it
/// is the program (only the program's `DT_PREINIT_ARRAY` counts).
#[must_use]
pub fn hooks(d: &Dynamic<'_>, bias: u64, program: bool) -> Hooks {
    let at = |tag| d.get(tag).filter(|&v| v != 0).map_or(0, |v| v.wrapping_add(bias));
    let array = |addr, size| {
        let a = at(addr);
        if a == 0 {
            return Array::default();
        }
        Array { addr: a, len: d.get(size).unwrap_or(0) / 8 }
    };
    Hooks {
        preinit: if program { array(dt::PREINIT_ARRAY, dt::PREINIT_ARRAYSZ) } else { Array::default() },
        init: at(dt::INIT),
        init_array: array(dt::INIT_ARRAY, dt::INIT_ARRAYSZ),
        fini_array: array(dt::FINI_ARRAY, dt::FINI_ARRAYSZ),
        fini: at(dt::FINI),
    }
}

/// Whether an array entry names a function: 0 and all-ones are left out (D-27.19).
fn callable(f: u64) -> bool {
    f != 0 && f != u64::MAX
}

/// The most objects [`exit_order`] covers (the loader's object table).
pub const EXIT_MAX: usize = 512;
/// The exit-time finalisation order (story DL-EXIT-ORDER-PROBE, probe tools/probe/dl/exitorder):
/// units `0..n` are given in sequence order (the start-time objects in [`fini_order`], then the
/// `dlopen`ed objects in load order); `succ(v, e)` is the `e`-th unit that `v` runs before (an
/// object before each object it needs; each start-time object before the next), `None` past the
/// last. Each set of units that must run before one another in a cycle runs as one group, its
/// members the last initialised first (`rank[i]`: higher = initialised later). Of the groups free
/// to run (nothing pending must run before them), the one with the earliest unit runs next. Writes
/// the units to `out` and returns `n`.
///
/// # Errors
///
/// [`OrderError::TooMany`] for more than [`EXIT_MAX`] units, or `out` or `rank` shorter than
/// `n`; [`OrderError::BadEdge`] for a successor outside `0..n`.
// Every index is below n <= EXIT_MAX (checked), counts stay below n or the number of successors.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
pub fn exit_order(n: usize, succ: impl Fn(usize, usize) -> Option<usize>, rank: &[u32], out: &mut [u16]) -> Result<usize, OrderError> {
    const NONE: u16 = u16::MAX;
    if n > EXIT_MAX || out.len() < n || rank.len() < n {
        return Err(OrderError::TooMany);
    }
    let unit = |v: usize| u16::try_from(v).unwrap_or(NONE);
    for v in 0..n {
        if (0..).map_while(|e| succ(v, e)).any(|w| w >= n) {
            return Err(OrderError::BadEdge);
        }
    }
    // Groups: strongly connected components (Tarjan, iterative), `comp[v]`.
    let (mut index, mut low, mut comp) = ([NONE; EXIT_MAX], [0u16; EXIT_MAX], [NONE; EXIT_MAX]);
    let (mut stack, mut on) = ([0u16; EXIT_MAX], [false; EXIT_MAX]);
    let (mut call, mut sp, mut next, mut ncomp) = ([(0u16, 0u16); EXIT_MAX], 0usize, 0u16, 0u16);
    for root in 0..n {
        if index[root] != NONE {
            continue;
        }
        call[0] = (unit(root), 0);
        let mut cp = 1usize;
        index[root] = next;
        low[root] = next;
        next += 1;
        stack[sp] = unit(root);
        sp += 1;
        on[root] = true;
        while cp > 0 {
            let (v, e) = call[cp - 1];
            let vu = usize::from(v);
            if let Some(wu) = succ(vu, usize::from(e)) {
                call[cp - 1].1 += 1;
                let w = unit(wu);
                if index[wu] == NONE {
                    index[wu] = next;
                    low[wu] = next;
                    next += 1;
                    stack[sp] = w;
                    sp += 1;
                    on[wu] = true;
                    call[cp] = (w, 0);
                    cp += 1;
                } else if on[wu] {
                    low[vu] = low[vu].min(index[wu]);
                }
                continue;
            }
            cp -= 1;
            if cp > 0 {
                let p = usize::from(call[cp - 1].0);
                low[p] = low[p].min(low[vu]);
            }
            if low[vu] == index[vu] {
                loop {
                    sp -= 1;
                    let w = usize::from(stack[sp]);
                    on[w] = false;
                    comp[w] = ncomp;
                    if w == vu {
                        break;
                    }
                }
                ncomp += 1;
            }
        }
    }
    // Each group's earliest unit and the pairs from other groups it waits for.
    let nc = usize::from(ncomp);
    let (mut first, mut waits, mut done) = ([NONE; EXIT_MAX], [0u16; EXIT_MAX], [false; EXIT_MAX]);
    for (v, &c) in comp.iter().enumerate().take(n) {
        let c = usize::from(c);
        first[c] = first[c].min(unit(v));
    }
    for a in 0..n {
        for b in (0..).map_while(|e| succ(a, e)) {
            if comp[a] != comp[b] {
                waits[usize::from(comp[b])] += 1;
            }
        }
    }
    let mut len = 0usize;
    for _ in 0..nc {
        // The free group with the earliest unit (a cycle-free graph of groups always has one).
        let Some(c) = (0..nc).filter(|&c| !done[c] && waits[c] == 0).min_by_key(|&c| first[c]) else { break };
        done[c] = true;
        let start = len;
        for (v, &cv) in comp.iter().enumerate().take(n) {
            if usize::from(cv) == c {
                out[len] = unit(v);
                len += 1;
            }
        }
        out[start..len].sort_unstable_by_key(|&v| core::cmp::Reverse(rank[usize::from(v)]));
        for &v in &out[start..len] {
            for w in (0..).map_while(|e| succ(usize::from(v), e)) {
                let cw = usize::from(comp[w]);
                if cw != c {
                    waits[cw] -= 1;
                }
            }
        }
    }
    Ok(len)
}

/// The functions to call to initialise one object: `DT_INIT` (`init`, 0 = none), then the entries
/// of its `DT_INIT_ARRAY` (`array`, already relocated) in order.
pub fn init_calls(init: u64, array: &[u64]) -> impl Iterator<Item = u64> + '_ {
    core::iter::once(init).chain(array.iter().copied()).filter(|&f| callable(f))
}

/// The functions to call to finalise one object: the entries of its `DT_FINI_ARRAY` (`array`) in
/// reverse, then `DT_FINI` (`fini`, 0 = none).
pub fn fini_calls(fini: u64, array: &[u64]) -> impl Iterator<Item = u64> + '_ {
    array.iter().rev().copied().chain(core::iter::once(fini)).filter(|&f| callable(f))
}
