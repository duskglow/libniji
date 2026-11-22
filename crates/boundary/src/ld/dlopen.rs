//! `ld.so`'s `dlopen` (phase 14 DL-16a `DL-DLOPEN-LOAD`; plan/ph14/dl-16-dlopen.md §1, §3, §4;
//! D-27.34). The C library's `dlopen` reaches [`entry`] through `StartInfo::dlopen` with the name,
//! the mode and its caller's return address.
//!
//! * **State.** One [`Loader`] in a private anonymous mapping made at the first call (nothing at
//!   start: `link` only records a few words per object). It holds every object as the relocator
//!   sees it (the start-time objects read again from their memory images, as `link` read them),
//!   their lookup views, the link-map nodes (the public `struct link_map` prefix first), the global
//!   scope, and two arenas (paths, dependency lists) that grow by generations never moved
//!   ([`Arena`], DL-PROCESS-OBJECTS). A new set is linked in with one Release
//!   store; `dlclose` ([`close_entry`], DL-17) unlinks what it unloads.
//! * **Locks.** The writer lock [`CALL`], re-entrant for its owner (a constructor may call
//!   `dlopen`), held for the whole call; inside it [`STATE`] while the call changes the loader's
//!   state, which `fork` waits for (DL-16c). The `&mut Loader` borrow ends before any constructor
//!   runs. Readers take neither (`super::readers`); the lock order is written at [`CALL`].
//! * **A new set.** The name is found (the caller's object as requester), every object of the set
//!   is opened, checked ([`niji_ld::dlopen::check`]), identified by (`st_dev`, `st_ino`) and
//!   mapped; versions are checked; pass 1 of the relocation resolves every reference of the whole
//!   set without running foreign code; only then pass 2 runs the IFUNC resolvers (ruling #5125), the
//!   RELRO pages are protected, the set is published, and the constructors run in
//!   `init::init_order`. Any failure before publication unmaps every new object and leaves the
//!   loader as it was, with one dlerror text set.
//!
//! * **Unloading** (DL-17, design in handoff/DL-DLCLOSE.md; probe `== close`). `dlclose` drops one
//!   open; an object stays while it is initial, `nodelete`, open, pinned by a pending C++
//!   `thread_local` destructor ([`pin_entry`]), or reachable from such an object through its
//!   dependencies and its relocations' bindings ([`live_set`]). The others run their
//!   finalisers (the reverse of the constructor log; `DT_FINI_ARRAY` backwards, then `DT_FINI`)
//!   holding `CALL` only, still published; then they are unpublished under `STATE` and unmapped once
//!   every reader that may have seen them has left ([`reclaim`]); a slot is reused after that.
//!
//! Not here yet (plan §6): lazy binding (DL-20).

use super::{array_words, c_string, dyn_string, env_entries, image_view, own_object, read_auxv, read_cache, report_nonshared, ProcImage, SysMapper};
use super::table::{Arena, Gens, Grown, Records, Spill, Table, Wide, ZeroValid, INLINE};
use super::{AT_FDCWD, AUXV_WORDS_MAX, INITIAL_TAIL, MAP_ANONYMOUS_I32, MAP_PRIVATE_I32, START_INTERP, START_N, START_ARGS, START_ORIGIN, START_PAGE, START_RUN_PATH};
use crate::{conv, dlerr, raw, tb};
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize};
use niji_kernel::auxv::Auxv;
use niji_kernel::uapi::{O_CLOEXEC, O_RDONLY, SEEK_END};
use niji_ld::arch::{Arch, Native};
use niji_ld::cache::Cache;
use niji_ld::dlopen::{self as dl, Mode, Refusal};
use niji_ld::elf::{pt, Elf, STB_GNU_UNIQUE};
use niji_ld::reloc::{self, Obj, Objects, RelocError, VersionError};
use niji_ld::search::{self, Ctx, PathBuf, Scope};
use niji_ld::symbols::{self, Found as SymFound, LookupError, Object, Request, UniqueRegistry, Want};
use niji_ld::tls::{self as ltls, Module, Placed};
use niji_ld::{init, map, start};

/// The object tables' entries in place (their generation 0: the link-map nodes in `ld.so`'s data,
/// the object records, the readers' table, the lock-held lists and the stack scratch). Past it each
/// table grows by generations ([`OBJ_GENS`], DL-PROCESS-OBJECTS part 2, audit HL-3): memory is the
/// only bound on the objects a process holds. The start-time objects stay below it (the start
/// writes their nodes before any loader exists).
pub(super) const CAP: usize = 512;
/// Generations of the object tables past [`CAP`]: indices fit 32 bits (`CAP << OBJ_GENS` = 2^32,
/// past the mappings the kernel lets a process hold: every object takes two or more).
pub(super) const OBJ_GENS: usize = 23;
/// An object index as stored (dependency lists, the constructor log, roots); indices fit 32 bits.
fn ix32(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}
/// A stored index as an index.
fn ix(i: u32) -> usize {
    usize::try_from(i).unwrap_or(usize::MAX)
}
/// An index of a new object relative to its set's first, for `init::init_order`'s edges.
#[derive(Clone, Copy)]
struct Rel(u32);
impl From<Rel> for usize {
    fn from(r: Rel) -> usize {
        ix(r.0)
    }
}
/// One `dlopen` adds any number of new objects (DL-DLOPEN-NEW-OBJECTS): its per-object tables
/// ([`Binds`], [`Ctors`], the pass-2 marks, the dependency edges and the init walk, the new TLS
/// modules) hold their first [`INLINE`] entries in place and spill to a mapping beyond that,
/// reserved before anything is published; the only failure left is that mapping's,
/// `<file>: cannot allocate memory`.
const NO_MEMORY: &[u8] = b": cannot allocate memory";
/// [`Found::Full`]'s text when an arena cannot grow (DL-PROCESS-OBJECTS: memory is the only bound).
const NO_ROOM: &[u8] = b"cannot allocate memory";
/// Dependency edges between one call's new objects held in place (a set of a few dozen objects
/// has a few hundred).
const EDGES_INLINE: usize = 256;
/// The path arena's bytes in place (its generation 0, in the loader's mapping); later generations
/// double, each mapped at its first use and never moved: the paths of every object the process holds
/// (DL-PROCESS-OBJECTS, audit HL-5). `l_name`, `dlpi_name` and `dli_fname` point into it.
const NAMES: usize = 128 * 1024;
/// Generations of the path arena past the one in place (its offsets fit 32 bits: 4 GiB of paths,
/// past the mappings the kernel lets a process hold for that many objects).
const NAMES_GENS: usize = 15;
type Names = Arena<u8, NAMES, NAMES_GENS>;
/// The dependency-list arena's entries in place (generation 0), then generations as for the paths:
/// every object's dependencies and recorded bindings, however many.
const DEPS: usize = 16 * 1024;
const DEPS_GENS: usize = 18;
type Deps = Arena<u32, DEPS, DEPS_GENS>;
/// The unique-symbol table: (name, version) keys and the bytes of their strings, each in its own
/// mapping that grows as needed (no count of unique names is too many; C++ libraries define
/// hundreds). Only a failed mapping makes the binding that needed the entry fail, with a text that
/// says memory ran out.
type Unique = reloc::GrowTable<MapStack<reloc::UniqueEntry>, MapStack<u8>>;

/// A [`reloc::Stack`] in an anonymous private mapping of its own, moved and grown by `mremap` (the
/// loader runs under its lock and calls no allocator). All zero is the empty stack, as the zeroed
/// `Loader` holds it. The mapping is never released: a namespace's table lives as long as it.
#[derive(Debug)]
pub(super) struct MapStack<T> {
    at: usize,
    len: usize,
    cap: usize,
    _t: core::marker::PhantomData<T>,
}

impl<T: Copy> MapStack<T> {
    /// Room for `extra` more items; `false` when the mapping cannot be made or grown.
    fn reserve(&mut self, extra: usize) -> bool {
        let size = size_of::<T>().max(1);
        let Some(need) = self.len.checked_add(extra) else { return false };
        if need <= self.cap {
            return true;
        }
        let want = need.max(self.cap.saturating_mul(2));
        let Some(bytes) = want.checked_mul(size).and_then(|b| b.checked_next_multiple_of(4096)) else { return false };
        let flags = MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32;
        let at = if self.at == 0 {
            raw::mmap(0, bytes, map::PROT_READ | map::PROT_WRITE, flags, -1, 0)
        } else {
            // The old range is referenced by nothing: `items` borrows end before `&mut self` here.
            raw::mremap(self.at, self.cap.wrapping_mul(size), bytes)
        };
        let Ok(at) = at else { return false };
        self.at = at;
        self.cap = bytes.checked_div(size).unwrap_or(0);
        true
    }
}

impl<T: Copy> reloc::Stack<T> for MapStack<T> {
    fn items(&self) -> &[T] {
        if self.at == 0 {
            return &[];
        }
        // UNSAFE(slice): the written prefix of the stack's own mapping.
        #[allow(unsafe_code)]
        // SAFETY: `at` is a private read-write mapping of `cap * size_of::<T>()` bytes or more, page
        // aligned (so aligned for T), made by `reserve` and released by nothing; its first `len <= cap`
        // items were written by `try_extend` as valid `T`s; the borrow of `self` keeps `reserve` (the
        // only mover) from running while the slice lives.
        unsafe {
            core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<T>(self.at), self.len)
        }
    }

    fn try_extend(&mut self, more: &[T]) -> bool {
        if more.is_empty() {
            return true;
        }
        if !self.reserve(more.len()) {
            return false;
        }
        let end = self.len.wrapping_add(more.len());
        let dst = core::ptr::with_exposed_provenance_mut::<T>(self.at).wrapping_add(self.len);
        // UNSAFE(slice): the reserved tail of the stack's own mapping, written once.
        #[allow(unsafe_code)]
        // SAFETY: `reserve` made room for `cap >= end` items at the page-aligned `at`, so `dst` is
        // aligned and `more.len()` items from it are inside the mapping; they are past every slice
        // `items` handed out (none is live: this borrows `&mut self`) and `more` is not in it. A
        // plain copy of `Copy` items, so the old bytes need not be valid `T`s.
        unsafe {
            core::ptr::copy_nonoverlapping(more.as_ptr(), dst, more.len());
        }
        self.len = end;
        true
    }

    fn truncate(&mut self, n: usize) {
        self.len = self.len.min(n);
    }
}
/// `SHN_ABS` (ELF gABI): an absolute symbol, not moved by the load bias.
const SHN_ABS: u16 = 0xfff1;

/// The object records (`Obj` or `Object`) as two copies (DL-PROCESS-OBJECTS part 2): the published
/// ones in never-moved generations (generation 0 the array in place), which [`View`] reads without
/// a lock, and, once the process holds more than [`CAP`] objects, a contiguous lock-held copy of
/// every record, which the relocator and the lookups take as one slice. Within [`CAP`] the array in
/// place is both, as before. All-zero is the empty table.
#[repr(C)]
struct Recs<T: Copy> {
    published: Grown<MaybeUninit<T>, CAP, OBJ_GENS>,
    wide: Records<T>,
}

impl<T: Copy> Recs<T> {
    /// Records `0..k`; every caller passes a `k` whose records are written (the loaded objects and
    /// the attempt's new ones, written before `k` is advanced). Empty when they are not.
    fn upto(&self, k: usize) -> &[T] {
        if k > CAP {
            return self.wide.slice(k).unwrap_or(&[]);
        }
        let k = k.min(CAP);
        // UNSAFE(slice): the initialised prefix of a MaybeUninit array.
        #[allow(unsafe_code)]
        // SAFETY: as documented above: `k <= CAP` records of the array in place are written.
        unsafe {
            core::slice::from_raw_parts(self.published.inline().as_ptr().cast::<T>(), k)
        }
    }

    /// Room for records `0..len`: the published generations, and past [`CAP`] the lock-held copy
    /// (filled with the `written` records the first time); `false` when a mapping cannot be made.
    fn reserve(&mut self, len: usize, written: usize) -> bool {
        if len <= CAP {
            return true;
        }
        if !self.published.reserve(len) || !self.wide.reserve(len) {
            return false;
        }
        for i in self.wide.len()..written.min(CAP) {
            let Some(&r) = self.upto(written.min(CAP)).get(i) else { return false };
            if !self.wide.set(i, r) {
                return false;
            }
        }
        true
    }

    /// Write record `i` in both copies (the room reserved).
    fn set(&mut self, i: usize, v: T) {
        if let Some(s) = self.published.get_mut(i) {
            *s = MaybeUninit::new(v);
        }
        if i >= CAP || self.wide.len() > 0 {
            let _ = self.wide.set(i, v);
        }
    }

    /// The published record `i`'s address in the table at `at`, read without a lock.
    #[inline(always)]
    fn addr_in(at: usize, i: usize) -> usize {
        Grown::<MaybeUninit<T>, CAP, OBJ_GENS>::addr_in(at.wrapping_add(core::mem::offset_of!(Recs<T>, published)), i)
    }
}

/// `objs[..k]` of the loader's object records (a field borrow, so another field may be borrowed
/// mutably beside it).
fn objs_of<'l>(objs: &'l Recs<Obj<'static>>, k: usize) -> &'l [Obj<'static>] {
    objs.upto(k)
}

/// `syms[..k]`, as [`objs_of`].
fn syms_of<'l>(syms: &'l Recs<Object<'static>>, k: usize) -> &'l [Object<'static>] {
    syms.upto(k)
}

/// The unique table as one lookup sees it: an entry recorded by this loader, or else the start's
/// binding, which is the first unique definition in the start's global scope (the start bound every
/// reference in that order); it is not recorded, so nothing is copied at the first `dl*` call.
struct Uniq<'a> {
    table: &'a mut Unique,
    syms: &'a [Object<'static>],
    start: &'a [usize],
    /// A `dlopen`'s record of its relocations' bindings (DL-17); `None` for a lookup (`dlsym`).
    binds: Option<&'a mut Binds>,
}

/// The bindings one `dlopen`'s relocations made (DL-17): per new object (index - `base`), a row of
/// `width` words with a bit per object it bound to (`CAP` bits while the set ends within [`CAP`]).
struct Binds {
    base: usize,
    width: usize,
    bits: Table<u64, { INLINE * CAP / 64 }>,
}

impl Binds {
    /// Word `obj / 64` of row `by`.
    fn word(&self, by: usize, obj: usize) -> Option<usize> {
        let row = by.checked_sub(self.base)?;
        let w = obj.checked_div(64).filter(|&w| w < self.width)?;
        row.checked_mul(self.width)?.checked_add(w)
    }

    fn has(&self, by: usize, obj: usize) -> bool {
        let Some(&w) = self.word(by, obj).and_then(|at| self.bits.get(at)) else { return false };
        w & (1u64 << (obj % 64)) != 0
    }
}

impl UniqueRegistry for Uniq<'_> {
    fn find(&self, name: &[u8], version: &[u8]) -> Option<SymFound> {
        self.table.find(name, version).or_else(|| {
            let want = if version.is_empty() { Want::Default } else { Want::Version(version) };
            let f = symbols::lookup(self.syms, symbols::Scope::global(self.start), &Request::new(name, want), &mut symbols::ScopeOrder).ok()??;
            (f.def.st.bind() == STB_GNU_UNIQUE).then_some(f)
        })
    }
    fn record(&mut self, name: &[u8], version: &[u8], found: SymFound) -> Result<(), LookupError> {
        self.table.record(name, version, found)
    }
    fn bound(&mut self, by: usize, found: &SymFound) {
        let Some(b) = self.binds.as_deref_mut() else { return };
        if found.obj != by {
            if let Some(w) = b.word(by, found.obj).and_then(|at| b.bits.get_mut(at)) {
                *w |= 1u64 << (found.obj % 64);
            }
        }
    }
}

const DT_SONAME: u64 = 14;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;
const S_IFMT: u32 = 0o170_000;
const S_IFDIR: u32 = 0o040_000;
const MAP_NORESERVE_I32: i32 = 0x4000;
const FUTEX_WAIT_PRIVATE: i64 = 128;
const FUTEX_WAKE_PRIVATE: i64 = 129;

/// One object's link map: `<link.h>`'s public `struct link_map` prefix, then libniji's own fields.
/// All-zero is a valid value (the loader's mapping starts zeroed).
#[repr(C)]
struct Node {
    /// `l_addr`: the load bias.
    l_addr: usize,
    /// `l_name`: the NUL-terminated name (`""` for the program).
    l_name: usize,
    /// `l_ld`: the run-time address of the dynamic section.
    l_ld: usize,
    /// `l_next`: the next node, published with Release.
    l_next: AtomicUsize,
    /// `l_prev`.
    l_prev: usize,
    /// The file's identity (start-time program: `u64::MAX`).
    dev: u64,
    ino: u64,
    /// The mapped range [`lo`, `hi`): which object a caller address is in.
    lo: usize,
    hi: usize,
    /// Direct `dlopen` calls holding it (saturating, DL-17 reads it).
    opens: u32,
    /// Pending C++ `thread_local` destructors of any thread registered with an address in it
    /// ([`pin_entry`], spec s1_2_c_6): it stays loaded while any is left.
    pins: u32,
    /// Never unloaded (initial, `RTLD_NODELETE`, `DF_1_NODELETE`, or `opens` or `pins` saturated).
    nodelete: bool,
    /// In the global scope.
    global: bool,
    /// Its dependencies (indices), a span of the dependency arena.
    deps: (u32, u32),
    /// Its path (a span of the path arena, NUL after it): `$ORIGIN` and messages.
    path: (u32, u32),
    /// Its finalisation words: `DT_FINI_ARRAY` address and length, `DT_FINI`.
    fini: [u64; 3],
    /// A `dlopen`ed object's read-only file mapping (address, length), which its `Obj` reads.
    file: (usize, usize),
    /// A `dlopen`ed object's reservation (base, span): what DL-17 (and a failed attempt) unmaps.
    reserve: (u64, u64),
    /// A `dlopen`ed object: the root of the call that loaded it, whose local scope it was relocated
    /// with (`RTLD_NEXT` and `RTLD_DEFAULT` from it search that scope, probe P-5); itself once that
    /// root is unloaded.
    root: u32,
    /// The objects its relocations bound to outside itself (indices), a span of the dependency
    /// arena: they stay loaded while it does (DL-17, probe `== close` C-3).
    binds: (u32, u32),
    /// `dlclose` (or the exit) is running its finalisers (DL-17).
    finalising: bool,
    /// Unloaded (DL-17): out of the chain, the scopes and the readers' table; its memory is
    /// unmapped and the slot reusable once `freed`.
    gone: bool,
    freed: bool,
}

/// The loader's state (module documentation).
#[repr(C)]
struct Loader {
    /// Objects loaded (`objs[..n]`, `syms[..n]` initialised).
    n: usize,
    /// Entries of `global`.
    n_global: usize,
    /// The start-time objects (`ld.so` included when it joined at adoption), and how many of them
    /// are global: the start's own global scope, `global[..n_start_global]`.
    n_start: usize,
    n_start_global: usize,
    /// The namespace's `STB_GNU_UNIQUE` table (plan §2): the first definition bound for a (name,
    /// version) is every later binding's; `unique_mark` is its length when a `dlopen` began.
    unique: Unique,
    unique_mark: usize,
    /// `dlopen`ed objects whose constructors started, in that order (destructors run backwards).
    n_log: AtomicUsize,
    log: Spill<u32, CAP>,
    global: Spill<usize, CAP>,
    /// The link-map nodes: the first [`CAP`] in [`NODES`] (DL-23), then the directory of their
    /// later generations.
    nodes: Nodes,
    objs: Recs<Obj<'static>>,
    syms: Recs<Object<'static>>,
    /// The readers' table's generations past its static [`CAP`] entries (`super::readers`).
    readers: Gens<CAP, OBJ_GENS>,
    /// Objects every table and the scratch of `dlclose` and the exit have room for ([`grow_to`];
    /// below [`CAP`]: [`CAP`]).
    room: usize,
    /// [`Scratch`]'s address once the count passed [`CAP`] (0: not made).
    scratch: usize,
    /// Finalisers of a `dlclose` or the exit past a [`Finis`]'s [`CAP`] in place: entries
    /// `0..fin_top` belong to the calls in progress (nested by a destructor's `dlclose`, last in
    /// first out). Never moved: [`Finis::run`] reads them without the loader.
    fin: Grown<Fin, CAP, OBJ_GENS>,
    fin_top: usize,
    /// The dependency lists (`Node::deps`, `Node::binds`) and the paths (`Node::path`): spans of
    /// arenas that grow by generations never moved ([`Arena`]); their tops are the attempt marks.
    deps: Deps,
    names: Names,
    /// The static surplus's places for `dlopen`ed modules (DL-16d) and their count when the
    /// current `load_set` began ([`unwind`] takes back the later ones).
    surplus: ltls::Surplus<super::Places>,
    tls_mark: usize,
    /// `__niji_thread_walk`'s address once looked up (0: not yet).
    thread_walk: usize,
    /// `__niji_owner_unload`'s address once looked up (0: not yet).
    owner_unload: usize,
    /// The TLS module IDs of `dlopen`ed modules (DL-17: an unloaded module's ID is reused).
    ids: ltls::ModuleIds,
    /// The free bits of module IDs past the ones `ids` holds (DL-PROCESS-OBJECTS part 3), grown by
    /// the `dlopen` that takes such an ID ([`tls_room_past`]).
    ids_more: Wide<u64>,
    /// Unloaded objects still mapped: (index, whether the reader parity flipped since it was
    /// unpublished; [`reclaim`]).
    n_retired: usize,
    retired: Spill<(u32, bool), CAP>,
    /// Freed slots below `n` (unloaded, unmapped, their grace period over): a new set that fits in
    /// a run of them goes there instead of the top ([`settle`]).
    holes: usize,
    /// A set went below the top at least once: index order is no longer load order, so the chain's
    /// tail is found by walking it ([`chain_tail`]).
    reordered: bool,
    /// The program's `$ORIGIN` (DL-RUNPATH, [`program_origin`]): `origin[..origin_len]`, published
    /// once (`origin_len` 0: not known); `origin_tried` once looked up.
    origin_tried: bool,
    origin_len: AtomicUsize,
    origin: [u8; search::PATH_MAX],
}

/// One finaliser entry of a [`Finis`] past its [`CAP`] in place.
#[derive(Clone, Copy)]
#[repr(C)]
struct Fin {
    call: [u64; 3],
    range: (usize, usize),
    idx: u32,
}
impl ZeroValid for Fin {}

/// The scratch of the calls that run past [`CAP`] objects (`dlclose`'s live set and unload list, the
/// exit's order, a new set's scope, `dlsym`'s scope), in a mapping of its own made by the `dlopen`
/// that first passes [`CAP`] and sized by every `dlopen` that grows the count ([`grow_to`]), so
/// `dlclose` and the exit never allocate. Within [`CAP`] the stack arrays serve, as before. Taken
/// only by the lock holder, once per call ([`scratch`]).
struct Scratch {
    keep: Wide<bool>,
    stack: Wide<u32>,
    set: Wide<bool>,
    out: Wide<u32>,
    local: Wide<u32>,
    scope: Wide<usize>,
    seen: Wide<bool>,
    pos: Wide<u32>,
    unit: Wide<u32>,
    rank: Wide<u32>,
    logged: Wide<u32>,
    order: Wide<u32>,
    walk: Wide<init::ExitWalk>,
}

impl Scratch {
    fn reserve(&mut self, n: usize) -> bool {
        self.keep.reserve(n) && self.stack.reserve(n) && self.set.reserve(n) && self.out.reserve(n) && self.local.reserve(n) && self.scope.reserve(n) && self.seen.reserve(n) && self.pos.reserve(n) && self.unit.reserve(n) && self.rank.reserve(n) && self.logged.reserve(n) && self.order.reserve(n) && self.walk.reserve(n)
    }
}

/// The loader's [`Scratch`] (`None` while the count never passed [`CAP`]). The caller holds the
/// lock and takes it once per call, before any foreign code runs and with no other borrow of it
/// live, as [`loader`] does.
fn scratch(l: &Loader) -> Option<&'static mut Scratch> {
    if l.scratch == 0 {
        return None;
    }
    // UNSAFE(slice): the loader's scratch in its own mapping.
    #[allow(unsafe_code)]
    // SAFETY: `l.scratch` is a private read-write mapping of `size_of::<Scratch>()` bytes, page
    // aligned, made by `grow_to` and never unmapped; all-zero is a valid `Scratch` (empty buffers).
    // Only the lock holder calls this, once per call with no borrow of it surviving the call, and
    // no foreign code runs while it is borrowed.
    Some(unsafe { &mut *core::ptr::with_exposed_provenance_mut::<Scratch>(l.scratch) })
}

/// `n` entries of scratch filled with `fill`: `inline` (the stack array within [`CAP`]) when it holds
/// them, else the loader's reserved buffer; empty when neither has the room (not reached: every
/// `dlopen` that grows the count reserves it first).
fn room<'a, T: ZeroValid>(inline: &'a mut [T], wide: Option<&'a mut Wide<T>>, n: usize, fill: T) -> &'a mut [T] {
    let s = if n <= inline.len() { inline.get_mut(..n) } else { wide.and_then(|w| w.slice_mut(n)) };
    let s = s.unwrap_or(&mut []);
    s.fill(fill);
    s
}

/// The loader's address (0: not made yet).
static LOADER: AtomicUsize = AtomicUsize::new(0);

/// Words of [`NODES`].
const NODE_WORDS: usize = CAP * size_of::<Node>() / 8;
const _: () = assert!(size_of::<Node>() % 8 == 0 && core::mem::align_of::<Node>() <= 64);

/// Entries of `ld.so`'s table of thread-local modules ([`thread_mods_head`]) held before the nodes.
pub(super) const MODS_HEAD: usize = 16;
const _: () = assert!(MODS_HEAD * 5 * 8 % 64 == 0);

/// The storage of every link-map node (DL-23): in `ld.so`'s own zero-filled data rather than in the
/// loader's mapping, so the start fills the start-time objects' public `struct link_map` prefix
/// ([`start_node`]) for the debugger rendezvous before any `dlopen`, and a node's address (an
/// object's handle and link map) is the same from the start to the exit. Only the pages the start
/// writes are touched (a node is under 200 bytes). The first [`MODS_HEAD`] entries of the
/// thread-local module table come first: the build places this table first in `ld.so`'s `.lbss`,
/// on the page the end of `.bss` already shares, so a start writes its first nodes and its first
/// thread-local modules there, not on new pages.
#[repr(C, align(64))]
struct NodeSpace {
    mods: [[AtomicU64; 5]; MODS_HEAD],
    nodes: [AtomicU64; NODE_WORDS],
}

static NODES: NodeSpace = NodeSpace { mods: [const { [const { AtomicU64::new(0) }; 5] }; MODS_HEAD], nodes: [const { AtomicU64::new(0) }; NODE_WORDS] };

/// Node 0's address (64-byte aligned); node `i` is `size_of::<Node>()` bytes further per index.
fn nodes_base() -> usize {
    core::ptr::from_ref(&NODES.nodes).expose_provenance()
}

/// The first [`MODS_HEAD`] entries of `ld.so`'s table of thread-local modules (`super::thread_mod`).
pub(super) fn thread_mods_head() -> &'static [[AtomicU64; 5]; MODS_HEAD] {
    &NODES.mods
}

// A node is 176 bytes, as before the tables grew: generation 0 is the same bytes.
const _: () = assert!(size_of::<Node>() == 176);

/// The nodes in [`NODES`] (generation 0), for [`Nodes`] and the start.
fn gen0_nodes() -> &'static [Node; CAP] {
    // UNSAFE(slice): the link-map nodes in `ld.so`'s data.
    #[allow(unsafe_code)]
    // SAFETY: NODES.nodes is a zero-filled, 64-byte aligned array of CAP Nodes (all-zero is a valid
    // Node; its words are atomics, readable through a shared reference). Reached only by the start
    // (one thread, before any `Loader` exists) and through the one live `&Loader` ([`Nodes`]);
    // readers read published fields by address.
    unsafe { &*core::ptr::with_exposed_provenance::<[Node; CAP]>(nodes_base()) }
}

/// [`gen0_nodes`] for their one writer.
fn gen0_nodes_mut() -> &'static mut [Node; CAP] {
    // UNSAFE(slice): the link-map nodes in `ld.so`'s data, for their one writer.
    #[allow(unsafe_code)]
    // SAFETY: as in `gen0_nodes`; reached only by the start (one thread, before any `Loader`
    // exists) and through the one live `&mut Loader` ([`Nodes`]), so no other reference to the
    // nodes is live (readers hold none: they read by address).
    unsafe { &mut *core::ptr::with_exposed_provenance_mut::<[Node; CAP]>(nodes_base()) }
}

/// The loader's view of the link-map nodes (DL-PROCESS-OBJECTS part 2): the first [`CAP`] in
/// [`NODES`], then generations never moved ([`Gens`]), so a node's address (a handle, a link map)
/// stays valid for the process. A field of [`Loader`], so the `&mut Loader` holder (one at a time,
/// under the lock) is their one writer, as when they were the loader's own bytes. All-zero is the
/// view with no later generation.
#[repr(C)]
struct Nodes {
    gens: Gens<CAP, OBJ_GENS>,
}

impl Nodes {
    /// Node `i`'s address, 0 when its generation is not mapped.
    fn addr(&self, i: usize) -> usize {
        self.gens.at(nodes_base(), size_of::<Node>(), i)
    }

    /// The index of the node at `addr`.
    fn index_of(&self, addr: usize) -> Option<usize> {
        self.gens.index(nodes_base(), size_of::<Node>(), addr)
    }

    /// Room for nodes `0..len`.
    fn reserve(&self, len: usize) -> bool {
        len <= CAP || self.gens.reserve(size_of::<Node>(), len)
    }

    fn get(&self, i: usize) -> Option<&Node> {
        if i < CAP {
            return gen0_nodes().get(i);
        }
        let at = self.addr(i);
        if at == 0 {
            return None;
        }
        // UNSAFE(slice): one node of a mapped generation.
        #[allow(unsafe_code)]
        // SAFETY: `at` is node `i` of a generation `reserve` mapped (read-write, page aligned, never
        // unmapped; a node is 176 bytes, so every one is 8-byte aligned); all-zero is a valid Node;
        // the borrow of the loader keeps its one writer away; readers read by address.
        Some(unsafe { &*core::ptr::with_exposed_provenance::<Node>(at) })
    }

    fn get_mut(&mut self, i: usize) -> Option<&mut Node> {
        if i < CAP {
            return gen0_nodes_mut().get_mut(i);
        }
        let at = self.addr(i);
        if at == 0 {
            return None;
        }
        // UNSAFE(slice): one node of a mapped generation, for its one writer.
        #[allow(unsafe_code)]
        // SAFETY: as in `get`; `&mut self` is reached only through the one `&mut Loader`, so no other
        // reference to the node is live (readers hold none: they read by address).
        Some(unsafe { &mut *core::ptr::with_exposed_provenance_mut::<Node>(at) })
    }

    /// Nodes `0..n` in order (as far as they are mapped).
    fn upto(&self, n: usize) -> impl Iterator<Item = &Node> + '_ {
        (0..n).map_while(|i| self.get(i))
    }
}

impl core::ops::Index<usize> for Nodes {
    type Output = Node;
    // Past every mapped node: the array's bounds check, as when the nodes were one array.
    #[allow(clippy::indexing_slicing)]
    fn index(&self, i: usize) -> &Node {
        match self.get(i) {
            Some(n) => n,
            None => &gen0_nodes()[i],
        }
    }
}

impl core::ops::IndexMut<usize> for Nodes {
    // As `index`.
    #[allow(clippy::indexing_slicing)]
    fn index_mut(&mut self, i: usize) -> &mut Node {
        if i < CAP || self.addr(i) == 0 {
            return &mut gen0_nodes_mut()[i];
        }
        match self.get_mut(i) {
            Some(n) => n,
            None => &mut gen0_nodes_mut()[i],
        }
    }
}

/// The name the start gave start-time object `k`'s link map ([`start_node`]), 0 when it linked none.
pub(super) fn start_name(k: usize) -> usize {
    if k >= CAP {
        return 0;
    }
    let at = nodes_base().wrapping_add(k.wrapping_mul(size_of::<Node>())).wrapping_add(core::mem::offset_of!(Node, l_name));
    // UNSAFE(slice): one word of a start-time node in `ld.so`'s data.
    #[allow(unsafe_code)]
    // SAFETY: `k < CAP`: the word is inside NODES.nodes (aligned: a usize field of a Node). The start wrote
    // it before any reader or `dlopen` ran, and nothing writes it again (`adopt_start` keeps a
    // linked node's prefix; slots are reused only past the start-time objects).
    unsafe { core::ptr::with_exposed_provenance::<usize>(at).read() }
}

/// The start's link map of start-time object `k` (DL-23; [`super::link`], before any `dlopen`, one
/// thread): its public prefix, with `l_prev` = `prev` (the previous link map's address, 0 for the
/// program's) and no `l_next` yet ([`link_next`]). `name` is a NUL-terminated string kept for the
/// process's lifetime. The node's address, or 0 when `k` is past the nodes ([`CAP`]).
pub(super) fn start_node(k: usize, l_addr: usize, name: usize, l_ld: usize, prev: usize) -> usize {
    let Some(node) = gen0_nodes_mut().get_mut(k) else { return 0 };
    (node.l_addr, node.l_name, node.l_ld, node.l_prev) = (l_addr, name, l_ld, prev);
    node.l_next.store(0, Relaxed);
    core::ptr::from_ref(&*node).expose_provenance()
}

/// Link `next` after start-time node `k` (DL-23, [`super::link`], one thread).
pub(super) fn link_next(k: usize, next: usize) {
    if let Some(p) = gen0_nodes().get(k) {
        p.l_next.store(next, Release);
    }
}

/// A futex mutex re-entrant for its owner: `word` 0 free, 1 held, 2 held with waiters; the
/// holder's thread pointer and depth are written by the holder only.
struct Reentrant {
    word: AtomicU32,
    owner: AtomicUsize,
    depth: AtomicUsize,
}

impl Reentrant {
    const fn new() -> Reentrant {
        Reentrant { word: AtomicU32::new(0), owner: AtomicUsize::new(0), depth: AtomicUsize::new(0) }
    }

    fn futex(&self, op: i64, val: u32) {
        let addr = self.word.as_ptr().cast::<c_void>();
        let _ = crate::ffi_sys::niji_sys_futex(addr, op, i64::from(val), core::ptr::null(), core::ptr::null_mut(), 0);
    }

    fn lock(&self) {
        let me = tb::tp();
        if self.owner.load(Relaxed) == me {
            self.depth.store(self.depth.load(Relaxed).saturating_add(1), Relaxed);
            return;
        }
        if let Err(mut c) = self.word.compare_exchange(0, 1, Acquire, Relaxed) {
            if c != 2 {
                c = self.word.swap(2, Acquire);
            }
            while c != 0 {
                self.futex(FUTEX_WAIT_PRIVATE, 2);
                c = self.word.swap(2, Acquire);
            }
        }
        self.owner.store(me, Relaxed);
        self.depth.store(1, Relaxed);
    }

    fn unlock(&self) {
        let d = self.depth.load(Relaxed).saturating_sub(1);
        self.depth.store(d, Relaxed);
        if d != 0 {
            return;
        }
        self.owner.store(0, Relaxed);
        if self.word.swap(0, Release) == 2 {
            self.futex(FUTEX_WAKE_PRIVATE, 1);
        }
    }

    /// Whether the calling thread holds the lock (at any depth).
    fn held_by_caller(&self) -> bool {
        self.owner.load(Relaxed) == tb::tp()
    }

    /// In a `fork` child, whose only thread is the caller: free the lock when another thread (one
    /// that does not exist here) held it; the caller's own hold stays.
    fn reset_in_child(&self) {
        if self.owner.load(Relaxed) != tb::tp() {
            self.owner.store(0, Relaxed);
            self.depth.store(0, Relaxed);
            self.word.store(0, Relaxed);
        }
    }
}

// Lock order (plan/ph14/dl-16-dlopen.md §4, one rule for every story): the loader's writer lock
// [`CALL`] -> its state lock [`STATE`] -> DL-15's thread-list lock (`thread::list_lock`) -> the
// loader arena's lock (DL-19's dynamic blocks; the dlerror records take none) -> the C library's
// internal locks (heap, stdio, ...). A constructor runs holding `CALL` only, so it may call
// `malloc`, stdio, `pthread_create` and the loader itself (both locks are re-entrant); `fork`'s
// prepare step takes `STATE` before every library lock (`crate::fork::lock_all`). Readers take no
// lock (`super::readers`). Checked by crates/ld/tests/dlopen_concurrent.rs (constructors that call
// into the library and the loader; `fork` during another thread's `dlopen`).

/// The writer lock: held for a whole `dlopen` call, constructors included, so `dlopen`s run one at
/// a time (probe P-8: a second thread's `dlopen` waits for the first's constructors).
static CALL: Reentrant = Reentrant::new();
/// The state lock: held (inside `CALL`) only while a call changes the loader's state, which runs
/// no foreign code but IFUNC resolvers. `fork` takes it, so a child never sees half a change, and
/// never waits for a constructor (probe P-9: `fork` returns while another thread's constructor
/// runs, and the child can `dlopen`).
static STATE: Reentrant = Reentrant::new();

pub(super) mod dladdr;
pub(super) mod dlsym;

/// One reader's view of the published objects (plan §4 `read_section`, D-27.1, D-27.30). It reads
/// only what publication made final: the chain (`l_next`, Acquire), and each published node's
/// range, names and path, object and symbol tables, all written before the Release store that
/// linked them and not written again until the slot is reused (a failed attempt rewrites only
/// unpublished indices). `dlclose` ([`unload`]) unlinks a node by storing its predecessor's
/// `l_next` and leaves the node's own `l_next`, range and path as they were, so a reader standing
/// on it walks on to the live chain; the slot is reused only after every reader counted before
/// the unlink has left ([`reclaim`]). It never forms a reference to the [`Loader`] or a [`Node`],
/// whose other fields (`opens`, `global`, `gone`) the writer may change under the lock.
pub(super) struct View {
    at: usize,
}

/// Run `f` over the published objects without the writer lock (D-27.1), counted as a reader in
/// [`readers::read_section`](super::readers::read_section) (DL-17): nothing it may reach is
/// unmapped or its slot reused before it leaves. `f` must not wait for another thread. The process's
/// first loader call builds the loader's state under [`STATE`] (the lock every state change holds;
/// taking it without [`CALL`] keeps the lock order, as `fork` does); `None` when it cannot be
/// built, or when the calling thread holds [`STATE`] (a signal handler that interrupted the first
/// `dlopen` while it builds the state), whose re-entrant hold must not build a second one.
pub(super) fn read_section<R>(f: impl FnOnce(&View) -> R) -> Option<R> {
    let mut at = LOADER.load(Acquire);
    if at == 0 {
        if STATE.owner.load(Relaxed) == tb::tp() {
            return None;
        }
        STATE.lock();
        let made = loader().is_some();
        STATE.unlock();
        at = LOADER.load(Acquire);
        if !made || at == 0 {
            return None;
        }
    }
    Some(super::readers::read_section(|_| f(&View { at })))
}

/// [`View::find`] for a caller already inside a read section (`dl_iterate_phdr` in chain order):
/// `None` also when no loader state exists (nothing went below the top then).
pub(super) fn chain_find(pred: impl FnMut(usize) -> bool) -> Option<usize> {
    let at = LOADER.load(Acquire);
    if at == 0 {
        return None;
    }
    View { at }.find(pred)
}

impl View {
    /// Node `i`'s address (the handle and link map of object `i`): below [`CAP`] in [`NODES`], else
    /// one directory load (0: not mapped).
    #[inline(always)]
    pub(super) fn node_addr(&self, i: usize) -> usize {
        Gens::<CAP, OBJ_GENS>::at_in(self.node_dir(), nodes_base(), size_of::<Node>(), i)
    }

    /// Node `i`'s address, `None` when its generation is not mapped (within [`CAP`], no test).
    #[inline(always)]
    fn node_of(&self, i: usize) -> Option<usize> {
        if i < CAP {
            return Some(nodes_base().wrapping_add(i.wrapping_mul(size_of::<Node>())));
        }
        Some(self.node_addr(i)).filter(|&a| a != 0)
    }

    /// The address of the nodes' directory of later generations.
    fn node_dir(&self) -> usize {
        self.at.wrapping_add(core::mem::offset_of!(Loader, nodes))
    }

    /// The index of the node at `addr` outside generation 0, by the directory.
    #[inline(never)]
    fn index_past(&self, addr: usize) -> Option<usize> {
        Gens::<CAP, OBJ_GENS>::index_in(self.node_dir(), nodes_base(), size_of::<Node>(), addr)
    }

    /// The program's `$ORIGIN` once published ([`look_up_origin`]), else `None`.
    pub(super) fn program_origin(&self) -> Option<&'static [u8]> {
        let len_at = self.at.wrapping_add(core::mem::offset_of!(Loader, origin_len));
        // UNSAFE(slice): the published origin length in the loader's own mapping.
        #[allow(unsafe_code)]
        // SAFETY: `self.at` is the loader's mapping, never unmapped; the field is an aligned AtomicUsize.
        let len = unsafe { &*core::ptr::with_exposed_provenance::<AtomicUsize>(len_at) }.load(Acquire);
        if len == 0 {
            return None;
        }
        let at = self.at.wrapping_add(core::mem::offset_of!(Loader, origin));
        // UNSAFE(slice): the published origin bytes in the loader's own mapping.
        #[allow(unsafe_code)]
        // SAFETY: the first `len` bytes were written before `origin_len` was stored (Release, read
        // above with Acquire) and are never written again; the mapping is never unmapped.
        Some(unsafe { conv::bytes_at(at, len) })
    }

    /// A published, final field of node `i` (`off` from [`core::mem::offset_of`]).
    fn field<T: Copy + Default>(&self, i: usize, off: usize) -> T {
        let Some(node) = self.node_of(i) else { return T::default() };
        let at = node.wrapping_add(off);
        // UNSAFE(slice): one field of a published node in the loader's own mapping.
        #[allow(unsafe_code)]
        // SAFETY: `i` came from the published chain: the node is in `ld.so`'s data or a generation
        // mapped before its publication, never unmapped (0 is no node: refused); `off` is a field's offset (aligned for `T`); the field was written before
        // the node's publication and not again (`unload` keeps it) until `reclaim` reuses the slot
        // after this counted reader has left, so this read does not race with the writer.
        unsafe { core::ptr::with_exposed_provenance::<T>(at).read() }
    }

    /// The next published node after `i` (0: none); the vDSO's link map, which is no node, is
    /// stepped over.
    fn next(&self, i: usize) -> usize {
        let Some(node) = self.node_of(i) else { return 0 };
        let at = node.wrapping_add(core::mem::offset_of!(Node, l_next));
        // UNSAFE(slice): the atomic `l_next` word of a published node.
        #[allow(unsafe_code)]
        // SAFETY: as in `field`; an atomic may be read while the writer stores it.
        let next = unsafe { &*core::ptr::with_exposed_provenance::<AtomicUsize>(at) };
        super::debug::past_vdso(next.load(Acquire))
    }

    /// The first published object, in chain order, for which `pred` holds.
    pub(super) fn find(&self, mut pred: impl FnMut(usize) -> bool) -> Option<usize> {
        let first = nodes_base();
        let mut i = 0usize;
        // Generation 0 as before: the index is bounded by CAP where it is decoded, so the node reads
        // in `pred` and `next` need no test of their own; the first node past it continues in
        // `find_past`. Every published object once (a chain longer than the indices is no chain
        // of ours).
        for _ in 0..CAP {
            if pred(i) {
                return Some(i);
            }
            let next = self.next(i);
            if next == 0 {
                return None;
            }
            let k = next.wrapping_sub(first).checked_div(size_of::<Node>())?;
            if next < first || k >= CAP {
                return self.find_past(next, &mut pred);
            }
            // `k < CAP` already; the mask (CAP is a power of two) is what lets the compiler drop
            // the node reads' own `i < CAP` tests (it folds the test above into one on the byte
            // offset and loses the index's bound): one `and` a step instead of a compare and a merge.
            const { assert!(CAP.is_power_of_two()) };
            i = k & CAP.wrapping_sub(1);
        }
        None
    }

    /// [`View::find`] from the node at `next` on, once the walk left generation 0.
    #[cold]
    #[inline(never)]
    fn find_past(&self, mut next: usize, pred: &mut impl FnMut(usize) -> bool) -> Option<usize> {
        let first = nodes_base();
        // A chain of distinct nodes holds at most the nodes the mapped generations have: a longer
        // walk decoded a wrong node and ends here, not after 2^32 steps. A generation mapped during
        // the walk raises the bound before it ends (its nodes may be on the chain now).
        // Counted from here: `find` hands over at the first node outside generation 0, and the
        // chain may come back to generation 0 after it.
        let mut bound = Gens::<CAP, OBJ_GENS>::mapped_in(self.node_dir());
        let mut steps = 0usize;
        loop {
            if steps >= bound {
                let now = Gens::<CAP, OBJ_GENS>::mapped_in(self.node_dir());
                if now <= bound {
                    return None;
                }
                bound = now;
            }
            steps = steps.wrapping_add(1);
            let off = next.wrapping_sub(first);
            let i = if next >= first && off < CAP.wrapping_mul(size_of::<Node>()) {
                off.checked_div(size_of::<Node>())?
            } else {
                self.index_past(next)?
            };
            if pred(i) {
                return Some(i);
            }
            next = self.next(i);
            if next == 0 {
                return None;
            }
        }
    }

    /// Object `i`'s mapped range [lo, hi).
    pub(super) fn range(&self, i: usize) -> (usize, usize) {
        (self.field(i, core::mem::offset_of!(Node, lo)), self.field(i, core::mem::offset_of!(Node, hi)))
    }

    /// `l_name` of object `i`.
    pub(super) fn l_name(&self, i: usize) -> usize {
        self.field(i, core::mem::offset_of!(Node, l_name))
    }

    /// Object `i`'s path (NUL-terminated in the path arena) as an address and a length; (0, 0)
    /// when unknown.
    pub(super) fn path(&self, i: usize) -> (usize, usize) {
        let (a, b): (u32, u32) = self.field(i, core::mem::offset_of!(Node, path));
        if b == 0 {
            return (0, 0);
        }
        let names = self.at.wrapping_add(core::mem::offset_of!(Loader, names));
        let at = Names::addr_in(names, usize::try_from(a).unwrap_or(usize::MAX));
        if at == 0 {
            return (0, 0);
        }
        (at, usize::try_from(b).unwrap_or(0))
    }

    /// Object `i` as the relocator saw it.
    pub(super) fn obj(&self, i: usize) -> Option<&Obj<'static>> {
        let at = Recs::<Obj<'static>>::addr_in(self.at.wrapping_add(core::mem::offset_of!(Loader, objs)), i);
        if at == 0 {
            return None;
        }
        // UNSAFE(slice): a published object record in the loader's mapping or a generation of it.
        #[allow(unsafe_code)]
        // SAFETY: `i` came from the published chain, so record `i` (in place below CAP, else in a
        // generation mapped before it was written, never moved or unmapped) was initialised before
        // the Release store that published it and is not written again until the slot is reused,
        // after this counted reader has left (`reclaim`), and the mapping outlives the view.
        Some(unsafe { &*core::ptr::with_exposed_provenance::<Obj<'static>>(at) })
    }

    /// Object `i`'s symbol tables.
    pub(super) fn syms(&self, i: usize) -> Option<&Object<'static>> {
        let at = Recs::<Object<'static>>::addr_in(self.at.wrapping_add(core::mem::offset_of!(Loader, syms)), i);
        if at == 0 {
            return None;
        }
        // UNSAFE(slice): a published symbol-table record in the loader's mapping or a generation.
        #[allow(unsafe_code)]
        // SAFETY: as in `obj`.
        Some(unsafe { &*core::ptr::with_exposed_provenance::<Object<'static>>(at) })
    }
}

/// The address of [`entry`], for `StartInfo::dlopen`.
pub(super) fn entry_address() -> usize {
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    (entry as *const ()).expose_provenance()
}

/// The address of [`fork_step`], for `StartInfo::dl_fork`.
pub(super) fn fork_address() -> usize {
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    (fork_step as *const ()).expose_provenance()
}

/// `fork`'s loader step (`crate::fork::lock_all`/`unlock_all`, while other threads exist): 0 takes
/// [`STATE`] before the clone, 1 (parent) releases it, 2 (child) releases it and frees [`CALL`]
/// when a thread that is gone held it, and clears the gone threads' reader counts
/// (`readers::after_fork_child`). The child keeps every published object, those whose
/// constructors that thread was running included (probe P-9: listed, `RTLD_NOLOAD` finds them).
extern "C" fn fork_step(step: usize) -> usize {
    match step {
        0 => STATE.lock(),
        1 => STATE.unlock(),
        _ => {
            STATE.unlock();
            CALL.reset_in_child();
            super::readers::after_fork_child();
        }
    }
    0
}

/// `dlopen(file, mode)` called from `caller` (a return address in the calling object): the handle,
/// or 0 with the calling thread's dlerror text set (none for an `RTLD_NOLOAD` miss).
extern "C" fn entry(file: usize, mode: i32, caller: usize) -> usize {
    room_wait();
    CALL.lock();
    STATE.lock();
    let (handle, ctors) = open_locked(file, mode, caller);
    STATE.unlock();
    // The loader borrow has ended: constructors may call back into the loader.
    ctors.run();
    CALL.unlock();
    handle
}

/// The address of node `i` (its link map) once the loader's state exists, else 0; no reference to
/// the state is made (readers call this while a `dlopen` may hold `&mut Loader`).
pub(super) fn node_address(i: usize) -> usize {
    let at = LOADER.load(Acquire);
    if at == 0 {
        return 0;
    }
    View { at }.node_addr(i)
}

/// Entry `i` of the readers' table past its static [`CAP`] (`super::readers`): its address in a
/// generation mapped (and in the loader's directory) before the entry was staged; 0 when there is
/// none. Read without a lock and without a reference to the loader.
pub(super) fn reader_entry(i: usize) -> usize {
    let at = LOADER.load(Acquire);
    if at == 0 || i < CAP {
        return 0;
    }
    Gens::<CAP, OBJ_GENS>::at_in(at.wrapping_add(core::mem::offset_of!(Loader, readers)), 0, super::readers::ENTRY_BYTES, i)
}

/// The loader, made (with the start-time objects) at the first call. The caller holds the lock
/// and drops the borrow before any foreign code runs, so no two `&mut Loader` are ever live.
fn loader() -> Option<&'static mut Loader> {
    let mut at = LOADER.load(Acquire);
    let fresh = at == 0;
    if fresh {
        let flags = MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32 | MAP_NORESERVE_I32;
        at = raw::mmap(0, size_of::<Loader>(), map::PROT_READ | map::PROT_WRITE, flags, -1, 0).ok()?;
    }
    // UNSAFE(slice): the loader's state in its own mapping.
    #[allow(unsafe_code)]
    // SAFETY: `at` is a private read-write mapping of size_of::<Loader>() bytes, page-aligned, made
    // above or by an earlier call and never unmapped. A zeroed Loader is valid (integers, atomics,
    // bools, MaybeUninit). Only the lock holder calls this, and every caller drops the borrow before
    // foreign code (a constructor that may re-enter) runs, so this is the only live reference.
    let l = unsafe { &mut *core::ptr::with_exposed_provenance_mut::<Loader>(at) };
    if fresh {
        if !adopt_start(l) {
            let _ = raw::munmap(at, size_of::<Loader>());
            return None;
        }
        if origin_listed(l) {
            look_up_origin(l);
        }
        dladdr::keep_start_paths(l);
        LOADER.store(at, Release);
    }
    Some(l)
}

impl Loader {
    fn node_addr(&self, i: usize) -> usize {
        self.nodes.addr(i)
    }

    /// Room in every object table for objects `0..len` and, past [`CAP`], the scratch of the calls
    /// that must not allocate (DL-PROCESS-OBJECTS part 2): called before object `len - 1` is written
    /// (`written` records are). `false` when a mapping cannot be made (the call fails with
    /// `cannot allocate memory`; what was mapped stays, as every generation does).
    fn grow_to(&mut self, len: usize, written: usize) -> bool {
        if len <= self.room.max(CAP) {
            return true;
        }
        grow_cold(self, len, written)
    }

    /// Copy `s` and a NUL into the path arena: its span; `None` only when the arena cannot grow (no
    /// memory).
    fn keep(&mut self, s: &[u8]) -> Option<(u32, u32)> {
        let start = self.names.append(s, 0)?;
        Some((u32::try_from(start).ok()?, u32::try_from(s.len()).ok()?))
    }

    /// The bytes of path span `(a, b)`, `None` when it is no written span.
    fn path_span(&self, (a, b): (u32, u32)) -> Option<&[u8]> {
        self.names.get(usize::try_from(a).ok()?, usize::try_from(b).ok()?)
    }

    fn path(&self, i: usize) -> &[u8] {
        let Some(&Node { path, .. }) = self.nodes.get(i) else { return b"" };
        self.path_span(path).unwrap_or(b"")
    }

    /// The entries of dependency-arena span `(a, b)`.
    fn dep_span(&self, (a, b): (u32, u32)) -> &[u32] {
        usize::try_from(a).ok().zip(usize::try_from(b).ok()).and_then(|(a, b)| self.deps.get(a, b)).unwrap_or(&[])
    }

    fn deps_of(&self, i: usize) -> &[u32] {
        self.nodes.get(i).map_or(&[], |nd| self.dep_span(nd.deps))
    }

    fn binds_of(&self, i: usize) -> &[u32] {
        self.nodes.get(i).map_or(&[], |nd| self.dep_span(nd.binds))
    }

    /// Whether object `j` is unloaded (its memory may be gone: never read its `Obj`).
    fn gone(&self, j: usize) -> bool {
        self.nodes.get(j).is_none_or(|nd| nd.gone)
    }

    /// The index of the node at `addr`.
    fn index_of(&self, addr: usize) -> Option<usize> {
        self.nodes.index_of(addr)
    }

    /// The global scope.
    fn global(&self) -> &[usize] {
        self.global.slice(self.n_global).unwrap_or(&[])
    }

    /// The constructor log.
    fn log(&self) -> &[u32] {
        self.log.slice(self.n_log.load(Relaxed)).unwrap_or(&[])
    }

    /// The object whose `DT_SONAME` is `name`, among the first `k`.
    fn by_soname(&self, name: &[u8], k: usize) -> Option<usize> {
        let objs = self.objs_upto(k);
        objs.iter().enumerate().position(|(j, o)| self.nodes.get(j).is_some_and(|nd| !nd.gone) && soname(o) == Some(name))
    }

    /// An object without `DT_SONAME` loaded by the bare name `name` (a `DT_NEEDED` entry or a
    /// `dlopen` name; probed: the reference matches that name too).
    fn by_given(&self, name: &[u8], k: usize) -> Option<usize> {
        let objs = self.objs_upto(k);
        objs.iter().enumerate().position(|(j, o)| self.nodes.get(j).is_some_and(|nd| !nd.gone) && soname(o).is_none() && o.name == name)
    }

    /// `objs[..k]` (`k` may pass `n` by the objects of the attempt in progress).
    fn objs_upto(&self, k: usize) -> &[Obj<'static>] {
        objs_of(&self.objs, k)
    }

}

fn soname(o: &Obj<'static>) -> Option<&'static [u8]> {
    dyn_string(&o.dynm, DT_SONAME)
}

/// The mapped range of `o`: lowest and highest run-time address of its `PT_LOAD`s.
fn range(o: &Obj<'_>) -> (usize, usize) {
    let loads = || o.elf.program_headers().filter(|p| p.p_type == pt::LOAD);
    let lo = loads().map(|p| p.vaddr).min().unwrap_or(0);
    let hi = loads().map(|p| p.vaddr.saturating_add(p.memsz)).max().unwrap_or(0);
    let at = |v: u64| usize::try_from(o.bias.wrapping_add(v)).unwrap_or(0);
    (at(lo), at(hi))
}

/// Fill node `i` of `o`, not yet linked.
fn describe(l: &mut Loader, i: usize, o: &Obj<'static>, name: usize, id: (u64, u64), prefix: bool) {
    let (lo, hi) = range(o);
    let dynv = o.elf.find(pt::DYNAMIC).map_or(0, |p| p.vaddr);
    let prev = if i == 0 { 0 } else { l.node_addr(i.wrapping_sub(1)) };
    if let Some(node) = l.nodes.get_mut(i) {
        // A start-time node the start linked (DL-23) keeps its public prefix: the chain a debugger
        // or a program may be walking is never cut.
        if prefix {
            node.l_addr = usize::try_from(o.bias).unwrap_or(0);
            node.l_name = name;
            node.l_ld = usize::try_from(o.bias.wrapping_add(dynv)).unwrap_or(0);
            node.l_next.store(0, Relaxed);
            node.l_prev = prev;
        }
        (node.dev, node.ino) = id;
        (node.lo, node.hi) = (lo, hi);
    }
}

/// Read the start-time objects again from their memory images ([`image_view`], as `link` did) and
/// make them the first nodes, all global, never unloaded. `ld.so` itself joins (global, last) when
/// no object named it at start. False when `link` did not run (no records).
// Indices stay below CAP (checked).
#[allow(clippy::indexing_slicing)]
fn adopt_start(l: &mut Loader) -> bool {
    let n = START_N.load(Acquire);
    if n == 0 || n >= CAP {
        return false;
    }
    let mut interp = START_INTERP.load(Relaxed);
    let total = if interp < n { n } else { n.saturating_add(1) };
    for k in 0..total {
        // The views are kept for the process's lifetime: they are the loader's copies.
        let mut views = [(0usize, 0usize); 3];
        let w = super::start_obj(k);
        let o = if k == interp || k == n {
            interp = k;
            own_object(Native::INTERP.as_bytes(), &mut views)
        } else {
            let at = |v: u64| usize::try_from(v).unwrap_or(0);
            let prog = start::Program { phdr: at(w[0]), phnum: at(w[1]), entry: 0 };
            let (head, view) = image_view(&prog, at(w[2]), b"", &mut views, true);
            let Ok(elf) = Elf::from_image::<Native>(head, view) else { return false };
            let Ok(Some(dynm)) = elf.dynamic() else { return false };
            // Its static block, so a `dlopen`ed object's TLS relocation against one of its
            // symbols finds the place (DL-16d).
            let tls = elf.find(pt::TLS).and_then(|ph| super::initial_place(w[2].wrapping_add(ph.vaddr)));
            let mut o = Obj { elf, dynm, bias: w[2], name: b"", tls };
            o.name = if k == 0 { b"" } else { soname(&o).unwrap_or(b"") };
            o
        };
        let Ok(mut s) = Object::new(o.dynm) else { return false };
        // The program's canonical PLT entries define their functions for later objects too.
        if k == 0 {
            s.mark_program();
        }
        l.objs.set(k, o);
        l.syms.set(k, s);
        // `l_name` when the start linked none: the program's is "", another object's its DT_SONAME (NUL-terminated in the
        // string table the view holds).
        // A node the start linked keeps its name (the path the start found the object by).
        let linked = l.nodes[k].l_ld != 0;
        let name = if linked {
            l.nodes[k].l_name
        } else if k == 0 {
            c"".as_ptr().expose_provenance()
        } else {
            o.name.as_ptr().expose_provenance()
        };
        let id = if k == interp { (u64::MAX, u64::MAX) } else { (w[3], w[4]) };
        describe(l, k, &o, name, id, !linked);
        l.nodes[k].nodelete = true;
        // At start every object was in the global scope. An `ld.so` that joins now joins it last,
        // where the C library naming it would put it (D-27.65), so `dlsym`'s
        // `RTLD_DEFAULT`/`RTLD_NEXT` find its exports (`_dl_get_tls_static_info`, `__tls_get_addr`).
        l.nodes[k].global = true;
        if let Some(g) = l.global.get_mut(l.n_global) {
            *g = k;
        }
        l.n_global = l.n_global.saturating_add(1);
        if k > 0 && !linked {
            let me = l.node_addr(k);
            l.nodes[k.wrapping_sub(1)].l_next.store(me, Relaxed);
        }
        l.n = k.saturating_add(1);
    }
    l.n_start = total;
    l.n_start_global = l.n_global;
    adopt_tls(l, n, interp) && adopt_deps(l, total)
}

/// The start's static TLS placement of each start-time object, computed again from the same
/// segments in the same order as `link` placed them (`ltls::layout` is deterministic), so `dlsym` of
/// an `STT_TLS` symbol finds the calling thread's block (plan §2).
#[allow(clippy::indexing_slicing)] // both tables hold n entries (reserved)
fn adopt_tls(l: &mut Loader, n: usize, interp: usize) -> bool {
    let mut mods: Table<Option<Module>, INLINE> = Table::new(None);
    let mut placed: Table<Option<Placed>, INLINE> = Table::new(None);
    let Some(objs) = l.objs_upto(n).get(..n) else { return false };
    if mods.reserve(n).is_err() || placed.reserve(n).is_err() {
        return false;
    }
    for (k, o) in objs.iter().enumerate() {
        if k != interp {
            mods[k] = o.elf.find(pt::TLS).map(|ph| Module::of(&ph));
        }
    }
    if ltls::layout::<Native>(&mods[..n], &mut placed[..n], INITIAL_TAIL, ltls::SURPLUS).is_err() {
        return false;
    }
    for (k, p) in placed[..n].iter().enumerate() {
        let o = Obj { tls: *p, ..l.objs_upto(n)[k] };
        l.objs.set(k, o);
    }
    true
}

/// Each start-time object's dependencies (its `DT_NEEDED` entries in order), for its local scope (a
/// handle's `dlsym`, `RTLD_GLOBAL` promotion): a name matches a start-time object's `DT_SONAME`, a
/// name with a `/` the object at that path by (`st_dev`, `st_ino`). A name that matches neither
/// (a start-time object without `DT_SONAME` found by a bare name) is left out.
/// False only when the dependency arena cannot grow (no memory).
#[allow(clippy::indexing_slicing)] // q < total < CAP
fn adopt_deps(l: &mut Loader, total: usize) -> bool {
    for q in 0..total {
        let o = l.objs_upto(total)[q];
        // Room for every entry first: one list is one span.
        let Some(start) = l.deps.open(o.dynm.needed().count()) else { return false };
        for needed in o.dynm.needed().map_while(Result::ok) {
            let j = l.by_soname(needed, total).or_else(|| {
                if !needed.contains(&b'/') {
                    return None;
                }
                let id = file_id(needed)?;
                l.nodes.upto(total).position(|nd| (nd.dev, nd.ino) == id)
            });
            let Some(j) = j else { continue };
            if !l.deps.push(ix32(j)) {
                return false;
            }
        }
        l.nodes[q].deps = (u32::try_from(start).unwrap_or(0), u32::try_from(l.deps.top().saturating_sub(start)).unwrap_or(0));
    }
    true
}

/// The (`st_dev`, `st_ino`) of the file at `path` (no NUL in it), or `None`.
fn file_id(path: &[u8]) -> Option<(u64, u64)> {
    let mut buf = PathBuf::default();
    if !buf.push(path) {
        return None;
    }
    let c = buf.c_str()?;
    let fd = crate::ffi_sys::niji_sys_openat(AT_FDCWD, core::ptr::with_exposed_provenance(c.as_ptr().expose_provenance()), i64::from(O_RDONLY | O_CLOEXEC), 0);
    let _ = i32::try_from(fd).ok().filter(|&f| f >= 0)?;
    let mut st = niji_kernel::uapi::Stat::default();
    let ok = crate::ffi_sys::niji_sys_fstat(fd, core::ptr::from_mut(&mut st).cast::<c_void>()) == 0;
    let _ = crate::ffi_sys::niji_sys_close(fd);
    ok.then_some((st.dev, st.ino))
}

/// The start's `argc`, `argv`, `envp`, auxiliary vector (`i` = 0..4) as `run_init` recorded them.
fn start_arg(i: usize) -> usize {
    START_ARGS.get(i).map_or(0, |w| w.load(Relaxed))
}

/// The constructors of one call's new set, in order, run after the loader borrow ends.
struct Ctors {
    calls: Table<(u64, u64, u64), INLINE>,
}

impl Ctors {
    /// No constructors (nothing written, nothing mapped).
    #[inline(always)]
    fn none() -> Ctors {
        Ctors { calls: Table::new((0, 0, 0)) }
    }

    /// Run each object's `DT_INIT` then `DT_INIT_ARRAY` with the initial `(argc, argv, envp)`.
    fn run(&self) {
        let [a, b, c] = [0, 1, 2].map(|i| i64::from_ne_bytes(u64::try_from(start_arg(i)).unwrap_or(0).to_ne_bytes()));
        for &(f, addr, len) in self.calls.iter() {
            for f in init::init_calls(f, array_words(init::Array { addr, len })) {
                let _ = crate::ffi::niji_machine_call3(usize::try_from(f).unwrap_or(0), a, b, c);
            }
        }
    }
}

/// Run the exit's finalisers once `dlopen` was used (from `fini_all`; false, nothing run, when it
/// never was): the start-time objects (`start`: each one's node and finalisation words, in
/// [`init::fini_order`], the program first) and the `dlopen`ed objects still loaded whose
/// constructors ran, as one sequence (DL-EXIT-ORDER-PROBE, [`init::exit_order`]): the earliest
/// loaded first, each object after every object that needs it (`DT_NEEDED`, or a binding its
/// relocation made), a dependency cycle as one group, the last initialised first. Each `dlopen`ed
/// object is marked finalising first, so a `dlclose` that a destructor or a later exit step makes
/// never runs it again.
pub(super) fn fini_exit(start: &[(usize, [u64; 3])]) -> bool {
    if LOADER.load(Acquire) == 0 {
        return false;
    }
    fini_exit_loaded(start)
}

/// [`fini_exit`] once a `dlopen` loader exists. Out of line (ICONV-CJK-STARTUP-2): its frame holds
/// [`Finis`] and [`exit_set`]'s tables (more than 24 KiB at [`CAP`] entries), and inlined into the
/// interpreter's `fini_all` the stack probes of that frame touched four more stack pages at every
/// exit, `dlopen` used or not (Zen 1 host, empty main: 4 of the 5 page faults start-up gained).
#[cold]
#[inline(never)]
fn fini_exit_loaded(start: &[(usize, [u64; 3])]) -> bool {
    let mut f = Finis::NONE;
    STATE.lock();
    let Some(l) = loader() else {
        STATE.unlock();
        return false;
    };
    exit_set(l, start, &mut f);
    STATE.unlock();
    f.run();
    true
}

/// Fill `f` with the exit's sequence ([`fini_exit`]). Units: `0..m` the start-time objects in
/// `start`'s order, chained (each before the next, so their own order holds); then the `dlopen`ed
/// objects in the constructor log, not unloaded or finalising, in load order (by index until a set
/// went below the top, then along the chain). A `dlopen`ed unit runs before each unit it needs or
/// bound to.
fn exit_set(l: &mut Loader, start: &[(usize, [u64; 3])], f: &mut Finis) {
    const NONE: u32 = u32::MAX;
    let as32 = ix32;
    let m = start.len().min(CAP);
    // Within CAP objects the stack arrays, else the scratch the growing `dlopen` reserved.
    let s = scratch(l);
    let (s_pos, s_unit, s_rank, s_logged, s_order, s_walk) = match s {
        Some(s) => (Some(&mut s.pos), Some(&mut s.unit), Some(&mut s.rank), Some(&mut s.logged), Some(&mut s.order), Some(&mut s.walk)),
        None => (None, None, None, None, None, None),
    };
    let total = l.n.max(m);
    let (mut pos0, mut unit0, mut rank0, mut logged0, mut out0) = ([NONE; CAP], [NONE; CAP], [0u32; CAP], [NONE; CAP], [0u32; CAP]);
    let mut walk0 = [init::ExitWalk::default(); CAP];
    // `pos[node]`: its unit; `unit[u]`: a dlopen unit's node; `rank[u]`: higher = initialised later.
    let pos = room(&mut pos0, s_pos, total, NONE);
    let unit = room(&mut unit0, s_unit, total, NONE);
    let rank = room(&mut rank0, s_rank, total, 0);
    let logged = room(&mut logged0, s_logged, total, NONE);
    let out = room(&mut out0, s_order, total, 0);
    let walk = room(&mut walk0, s_walk, total, init::ExitWalk::default());
    for (u, &(i, _)) in start.iter().take(m).enumerate() {
        if let (Some(p), Some(r)) = (pos.get_mut(i), rank.get_mut(u)) {
            *p = as32(u);
            *r = as32(m.saturating_sub(u));
        }
    }
    let n_log = l.log().len();
    for (k, &i) in l.log().iter().enumerate() {
        if let Some(x) = logged.get_mut(ix(i)) {
            *x = as32(k);
        }
    }
    let mut n = m;
    // Load order: by index, or along the chain once a set went below the top ([`settle`]).
    let mut cur = if l.reordered { chain_next(l, 0) } else { Some(l.n_start).filter(|&i| i < l.n) };
    for _ in 0..l.n {
        let Some(i) = cur else { break };
        cur = if l.reordered { chain_next(l, i) } else { Some(i.wrapping_add(1)).filter(|&j| j < l.n) };
        let k = logged.get(i).copied().unwrap_or(NONE);
        if k == NONE || l.nodes.get(i).is_none_or(|nd| nd.gone || nd.finalising) {
            continue;
        }
        let (Some(p), Some(x), Some(r)) = (pos.get_mut(i), unit.get_mut(n), rank.get_mut(n)) else { break };
        *p = as32(n);
        *x = as32(i);
        *r = as32(m.saturating_add(1).saturating_add(ix(k)));
        n = n.saturating_add(1);
    }
    let lr: &Loader = l;
    let succ = |v: usize, e: usize| -> Option<usize> {
        if v < m {
            let w = v.saturating_add(1);
            return (e == 0 && w < m).then_some(w);
        }
        let i = ix(*unit.get(v)?);
        let targets = lr.deps_of(i).iter().chain(lr.binds_of(i)).map(|&j| pos.get(ix(j)).copied().unwrap_or(NONE));
        targets.filter(|&p| p != NONE && ix(p) != v).nth(e).map(ix)
    };
    let len = init::exit_order(n, succ, rank.get(..n).unwrap_or(&[]), out, walk).unwrap_or(0);
    let order = out.get(..len).unwrap_or(&[]);
    if order.is_empty() && n > 0 {
        // Not reached (the scratch has room for every unit, every edge is a unit): the `dlopen`ed
        // objects the last initialised first, then the start-time ones.
        for k in (0..n_log).rev() {
            let j = l.log().get(k).map_or(usize::MAX, |&i| ix(i));
            f.add(l, j);
        }
        start.iter().take(m).for_each(|&(i, w)| f.push(l, i, w));
        return;
    }
    for &u in order {
        let u = ix(u);
        match start.get(u) {
            Some(&(i, w)) if u < m => f.push(l, i, w),
            _ => f.add(l, unit.get(u).map_or(usize::MAX, |&i| ix(i))),
        }
    }
}

/// The finalisers of the objects one `dlclose` (or the exit) unloads, in the order they run.
struct Finis {
    calls: [[u64; 3]; CAP],
    idx: [u32; CAP],
    len: usize,
    /// Entries past [`CAP`]: from `from` in the loader's never-moved `fin` store, at `store`.
    from: usize,
    store: usize,
    /// Each object's mapped range [lo, hi), for the C library's `__niji_owner_unload`.
    ranges: [(usize, usize); CAP],
    /// `__niji_owner_unload`'s address when this is a `dlclose` or a last-pin unload (0: at exit, or
    /// no C library has it).
    owner_unload: usize,
}

impl Finis {
    const NONE: Finis = Finis { calls: [[0; 3]; CAP], idx: [0; CAP], len: 0, from: 0, store: 0, ranges: [(0, 0); CAP], owner_unload: 0 };

    /// Add object `i` (loaded, not finalising yet) and mark it finalising.
    fn add(&mut self, l: &mut Loader, i: usize) {
        let Some(node) = l.nodes.get(i) else { return };
        if node.gone || node.finalising || i >= l.n {
            return;
        }
        let e = Fin { call: node.fini, range: (node.lo, node.hi), idx: ix32(i) };
        if self.put(l, e) {
            if let Some(node) = l.nodes.get_mut(i) {
                node.finalising = true;
            }
        }
    }

    /// Add a start-time object's finalisation words (`i`: its node; never unloaded).
    fn push(&mut self, l: &mut Loader, i: usize, words: [u64; 3]) {
        let _ = self.put(l, Fin { call: words, range: (0, 0), idx: ix32(i) });
    }

    /// Append `e`: in place within [`CAP`], else in the loader's `fin` store (reserved for every
    /// object by the `dlopen` that grew the count, so this never allocates); `false` when neither
    /// has room (not reached).
    fn put(&mut self, l: &mut Loader, e: Fin) -> bool {
        if let (Some(c), Some(x), Some(r)) = (self.calls.get_mut(self.len), self.idx.get_mut(self.len), self.ranges.get_mut(self.len)) {
            (*c, *x, *r) = (e.call, e.idx, e.range);
        } else {
            if self.len == CAP {
                (self.from, self.store) = (l.fin_top, core::ptr::from_ref(&l.fin).expose_provenance());
            }
            let at = self.from.saturating_add(self.len.saturating_sub(CAP));
            let Some(slot) = l.fin.get_mut(at) else { return false };
            *slot = e;
            l.fin_top = at.saturating_add(1);
        }
        self.len = self.len.saturating_add(1);
        true
    }

    /// Entry `j` (below `len`): in place, else read from the `fin` store by address (no loader
    /// borrow: [`Finis::run`] holds none).
    fn entry(&self, j: usize) -> Option<Fin> {
        if j >= self.len {
            return None;
        }
        if let (Some(&call), Some(&idx), Some(&range)) = (self.calls.get(j), self.idx.get(j), self.ranges.get(j)) {
            return Some(Fin { call, range, idx });
        }
        let at = Grown::<Fin, CAP, OBJ_GENS>::addr_in(self.store, self.from.saturating_add(j.saturating_sub(CAP)));
        if at == 0 {
            return None;
        }
        // UNSAFE(slice): one finaliser entry of this call's part of the loader's `fin` store.
        #[allow(unsafe_code)]
        // SAFETY: `put` wrote entry `from + (j - CAP)` of the store (in the loader's mapping or a
        // generation of it, never moved or unmapped) before `len` passed `j`; nothing writes it again
        // while this call is in progress (a nested call's entries lie above it), and it is read by
        // value.
        Some(unsafe { core::ptr::with_exposed_provenance::<Fin>(at).read() })
    }

    /// The objects in the set, in order.
    fn set(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.len).filter_map(|j| self.entry(j)).map(|e| ix(e.idx))
    }

    /// Give this call's part of the `fin` store back (its nested calls have ended).
    fn release(&self, l: &mut Loader) {
        if self.len > CAP {
            l.fin_top = l.fin_top.min(self.from);
        }
    }

    /// Each object's `DT_FINI_ARRAY` in reverse, then its `DT_FINI` (probe `== order`); then, for a
    /// `dlclose`, the C library's step for the opted-in registrations the object owns or whose
    /// handlers it holds (D-27.29), while its code is still mapped.
    fn run(&self) {
        let w = |v: usize| i64::from_ne_bytes(u64::try_from(v).unwrap_or(0).to_ne_bytes());
        for Fin { call: [addr, len, fini], range: (lo, hi), .. } in (0..self.len).filter_map(|j| self.entry(j)) {
            for f in init::fini_calls(fini, array_words(init::Array { addr, len })) {
                let _ = crate::ffi::niji_machine_call0(usize::try_from(f).unwrap_or(0));
            }
            if self.owner_unload != 0 {
                let _ = crate::ffi::niji_machine_call3(self.owner_unload, w(lo), w(hi), 0);
            }
        }
    }
}

/// The address of [`close_entry`], for `StartInfo::dlclose`.
pub(super) fn close_address() -> usize {
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    (close_entry as *const ()).expose_provenance()
}

/// `dlclose(handle)` (DL-17; module documentation): 0, or -1 with the text `dlclose: invalid
/// handle` for NULL, a pointer that is not a handle, or a handle with no open left (pending
/// D-27.32, ruling #5006). A success clears a pending text (probe `== close` C-2).
extern "C" fn close_entry(handle: usize) -> i64 {
    room_wait();
    CALL.lock();
    STATE.lock();
    let mut f = Finis::NONE;
    let r = close_begin(handle, &mut f);
    STATE.unlock();
    if r.is_ok() && f.len > 0 {
        // The loader borrow has ended: a destructor may call `dlopen` or `dlclose` (C-8).
        f.run();
        STATE.lock();
        close_end(&f);
        STATE.unlock();
    }
    CALL.unlock();
    match r {
        Ok(()) => {
            dlerr::clear();
            0
        }
        Err(Fail) => -1,
    }
}

/// The address of [`pin_entry`], for `StartInfo::dl_pin`.
pub(super) fn pin_address() -> usize {
    #[allow(clippy::as_conversions)] // a function's address has no conversion other than `as`
    (pin_entry as *const ()).expose_provenance()
}

/// A C++ `thread_local` destructor's hold on its object (spec s1_2_c_6: `__cxa_thread_atexit_impl`
/// keeps `dso` loaded until the destructor has run; DL-TLS-DTOR-PIN). `(0, dso)`: the `dlopen`ed
/// object holding the address `dso` gains a pin, and the result is its token (its index plus one);
/// 0 when `dso` is in no such object (the program, an initial object, `ld.so` or no object need
/// none). `(1, token)`: the destructor has run; drop that pin, and when it was the object's last and
/// no open holds it, unload what is no longer live as `dlclose` does (its finalisers run now).
/// A pinned object's slot is never reused, so a token names its object until it is dropped.
extern "C" fn pin_entry(op: usize, arg: usize) -> usize {
    if op == 0 {
        return pin(arg);
    }
    unpin(arg);
    0
}

fn pin(dso: usize) -> usize {
    if dso == 0 || LOADER.load(Acquire) == 0 {
        return 0;
    }
    STATE.lock();
    let mut token = 0;
    if let Some(l) = loader() {
        let at = l.nodes.upto(l.n).position(|nd| !nd.gone && (nd.lo..nd.hi).contains(&dso));
        if let Some((i, nd)) = at.filter(|&i| i >= l.n_start).and_then(|i| Some((i, l.nodes.get_mut(i)?))) {
            nd.pins = nd.pins.saturating_add(1);
            // A saturated count can no longer be dropped exactly: the object stays for good.
            nd.nodelete |= nd.pins == u32::MAX;
            token = i.saturating_add(1);
        }
    }
    STATE.unlock();
    token
}

fn unpin(token: usize) {
    let Some(i) = token.checked_sub(1) else { return };
    room_wait();
    CALL.lock();
    STATE.lock();
    let mut f = Finis::NONE;
    if let Some(l) = loader() {
        if let Some(nd) = l.nodes.get_mut(i).filter(|nd| !nd.gone && nd.pins > 0) {
            nd.pins = nd.pins.saturating_sub(1);
            if nd.pins == 0 && nd.opens == 0 && !nd.nodelete {
                collect(l, &mut f);
            }
        }
    }
    STATE.unlock();
    if f.len > 0 {
        // As in `close_entry`: a destructor may call `dlopen` or `dlclose`.
        f.run();
        STATE.lock();
        close_end(&f);
        STATE.unlock();
    }
    CALL.unlock();
}

/// Drop one open of `handle`; fill `f` with what is to be unloaded (marked finalising).
fn close_begin(handle: usize, f: &mut Finis) -> Result<(), Fail> {
    let invalid = || fail_parts(&[b"dlclose: invalid handle"]);
    let Some(l) = loader() else { return Err(invalid()) };
    reclaim(l);
    let Some(i) = dlsym::handle_index(l, handle) else { return Err(invalid()) };
    let start = i < l.n_start;
    let Some(node) = l.nodes.get_mut(i) else { return Err(invalid()) };
    if start {
        // The program and its start-time objects are never unloaded (C-9: 0).
        node.opens = node.opens.saturating_sub(1);
        return Ok(());
    }
    if node.opens == 0 {
        return Err(invalid());
    }
    node.opens = node.opens.saturating_sub(1);
    if node.opens > 0 || node.nodelete {
        return Ok(());
    }
    collect(l, f);
    Ok(())
}

/// Fill `f` with every object no longer live, in finalisation order (marked finalising).
fn collect(l: &mut Loader, f: &mut Finis) {
    let s = scratch(l);
    let (s_keep, s_stack) = match s {
        Some(s) => (Some(&mut s.keep), Some(&mut s.stack)),
        None => (None, None),
    };
    let (mut keep0, mut stack0) = ([false; CAP], [0u32; CAP]);
    let keep = room(&mut keep0, s_keep, l.n, false);
    let stack = room(&mut stack0, s_stack, l.n, 0);
    live_set(l, &[], keep, stack);
    let n = l.log().len();
    for k in (0..n).rev() {
        let j = l.log().get(k).map_or(usize::MAX, |&j| ix(j));
        if !keep.get(j).copied().unwrap_or(true) {
            f.add(l, j);
        }
    }
    // A published object the log could not hold: after the logged ones.
    for j in (0..l.n).rev() {
        if !keep.get(j).copied().unwrap_or(true) {
            f.add(l, j);
        }
    }
    if f.len > 0 {
        f.owner_unload = owner_unload(l);
    }
}

/// Which objects stay (plan §1): every object that is initial, `nodelete`, open, pinned, or finalising in
/// an outer call (`set`'s own excepted), and every object reachable from one through its
/// dependencies and its relocations' bindings.
/// `keep` and `stack` hold `l.n` entries (`keep` all false).
fn live_set(l: &Loader, set: &[bool], keep: &mut [bool], stack: &mut [u32]) {
    let mut sp = 0usize;
    let push = |keep: &mut [bool], stack: &mut [u32], sp: &mut usize, i: usize| {
        if let (Some(k), Some(s)) = (keep.get_mut(i), stack.get_mut(*sp)) {
            if !*k {
                *k = true;
                *s = ix32(i);
                *sp = sp.saturating_add(1);
            }
        }
    };
    for (i, nd) in l.nodes.upto(l.n).enumerate() {
        let mine = set.get(i).copied().unwrap_or(false);
        if !nd.gone && (i < l.n_start || nd.nodelete || nd.opens > 0 || nd.pins > 0 || (nd.finalising && !mine)) {
            push(keep, stack, &mut sp, i);
        }
    }
    while sp > 0 {
        sp = sp.saturating_sub(1);
        let i = stack.get(sp).map_or(0, |&i| ix(i));
        for &d in l.deps_of(i).iter().chain(l.binds_of(i)) {
            let d = ix(d);
            if !l.gone(d) {
                push(keep, stack, &mut sp, d);
            }
        }
    }
}

/// After the finalisers of `f` ran: unpublish and retire every object of the set that is still not
/// reachable (one a destructor opened again stays, its destructors run), then [`reclaim`].
fn close_end(f: &Finis) {
    let Some(l) = loader() else { return };
    f.release(l);
    let s = scratch(l);
    let (s_set, s_keep, s_stack, s_out) = match s {
        Some(s) => (Some(&mut s.set), Some(&mut s.keep), Some(&mut s.stack), Some(&mut s.out)),
        None => (None, None, None, None),
    };
    let (mut set0, mut keep0, mut stack0, mut out0) = ([false; CAP], [false; CAP], [0u32; CAP], [0u32; CAP]);
    let set = room(&mut set0, s_set, l.n, false);
    for j in f.set() {
        if let Some(b) = set.get_mut(j) {
            *b = true;
        }
    }
    let keep = room(&mut keep0, s_keep, l.n, false);
    live_set(l, set, keep, room(&mut stack0, s_stack, l.n, 0));
    let mut tls = super::NewTls::none();
    let mut generation = super::tls_generation();
    let out = room(&mut out0, s_out, l.n, 0);
    let mut n_out = 0usize;
    for j in f.set() {
        if let Some(nd) = l.nodes.get_mut(j) {
            nd.finalising = false;
        }
        if keep.get(j).copied().unwrap_or(true) {
            continue;
        }
        if n_out == 0 {
            super::debug::change(niji_ld::debug::State::Delete);
        }
        if let Some(id) = unload(l, j) {
            // Words other than the ID are not read for a release. A batch the table cannot grow
            // for is released at once, as a `dlclose` of its own would (the storage it filled is
            // kept, so the next push has room): any number of modules goes, with no memory.
            let m = [0, 0, 0, id, 0];
            if tls.mods.push(m).is_err() {
                super::release_tls(thread_walk(l), &mut tls, generation);
                tls.mods.clear();
                let _ = tls.mods.push(m);
            }
            generation = l.ids.release(id, generation, l.ids_more.all_mut());
        }
        if let Some(o) = out.get_mut(n_out) {
            *o = ix32(j);
            n_out = n_out.saturating_add(1);
        }
    }
    if n_out == 0 {
        return;
    }
    super::debug::change(niji_ld::debug::State::Consistent);
    // A survivor relocated with an unloaded root's scope now searches its own (`dlsym`'s
    // `RTLD_DEFAULT`/`RTLD_NEXT`).
    for i in l.n_start..l.n {
        let r = l.nodes.get(i).map_or(0, |nd| ix(nd.root));
        if !l.gone(i) && l.gone(r) {
            if let Some(nd) = l.nodes.get_mut(i) {
                nd.root = ix32(i);
            }
        }
    }
    if !tls.mods.is_empty() {
        super::release_tls(thread_walk(l), &mut tls, generation);
    }
    for &j in out.get(..n_out).unwrap_or(&[]) {
        // Never short: `grow_to` gave the list room for every object.
        if let Some(r) = l.retired.get_mut(l.n_retired) {
            *r = (j, false);
            l.n_retired = l.n_retired.saturating_add(1);
        }
    }
    reclaim(l);
}

/// Unpublish object `j`: the readers' table (and `dlpi_subs`), the chain, the global scope, the
/// constructor log, its static TLS place; its TLS module ID when it had one.
fn unload(l: &mut Loader, j: usize) -> Option<u64> {
    super::readers::unpublish(j);
    let (prev, next) = l.nodes.get(j).map_or((0, 0), |nd| (nd.l_prev, nd.l_next.load(Relaxed)));
    if let Some(p) = l.index_of(prev).and_then(|p| l.nodes.get(p)) {
        p.l_next.store(next, Release);
    }
    if let Some(nx) = l.index_of(next).and_then(|x| l.nodes.get_mut(x)) {
        nx.l_prev = prev;
    }
    let mut w = 0usize;
    for r in 0..l.n_global {
        let g = l.global.get(r).copied().unwrap_or(j);
        if g != j {
            if let Some(slot) = l.global.get_mut(w) {
                *slot = g;
            }
            w = w.saturating_add(1);
        }
    }
    l.n_global = w;
    let n = l.log().len();
    let mut w = 0usize;
    for r in 0..n {
        let g = l.log.get(r).copied().unwrap_or(0);
        if ix(g) != j {
            if let Some(slot) = l.log.get_mut(w) {
                *slot = g;
            }
            w = w.saturating_add(1);
        }
    }
    l.n_log.store(w, Release);
    let tls = l.objs_upto(l.n).get(j).and_then(|o| o.tls);
    // The node's own `l_next`, range and path stay as they were until the slot is reused: a
    // `dladdr` reader standing on it ([`View`]) walks on to the live chain. Lookups under the lock
    // skip it by `gone`.
    if let Some(nd) = l.nodes.get_mut(j) {
        nd.gone = true;
        nd.global = false;
        nd.opens = 0;
        nd.pins = 0;
        (nd.dev, nd.ino) = (u64::MAX, u64::MAX.wrapping_sub(1));
    }
    let p = tls?;
    let _ = l.surplus.free(p.tp_offset);
    Some(p.module)
}

/// Unmap every retired object no reader can still see, then give the slots at the end of the table
/// back: `n` and the arenas shrink over freed slots. Never waits. Two steps per object: the parity
/// flips once the readers of the one before have left ([`readers::drained`](super::readers::drained)),
/// and the object is unmapped when the readers counted before that flip have left in turn. With no
/// reader inside, both steps happen at once; otherwise at a later `dlopen`/`dlclose`.
fn reclaim(l: &mut Loader) {
    for _ in 0..2 {
        if !super::readers::drained() {
            break;
        }
        let (mut w, mut waiting) = (0usize, false);
        for r in 0..l.n_retired {
            let (j, flipped) = l.retired.get(r).copied().unwrap_or((0, false));
            if flipped {
                if let Some(nd) = l.nodes.get_mut(ix(j)) {
                    map::Mapper::unmap(&mut SysMapper { fd: -1 }, nd.reserve.0, nd.reserve.1);
                    let _ = raw::munmap(nd.file.0, nd.file.1);
                    nd.freed = true;
                    l.holes = l.holes.saturating_add(1);
                }
                continue;
            }
            if let Some(slot) = l.retired.get_mut(w) {
                *slot = (j, true);
            }
            (w, waiting) = (w.saturating_add(1), true);
        }
        l.n_retired = w;
        if !waiting {
            break;
        }
        super::readers::flip();
    }
    let mut n = l.n;
    while n > l.n_start && l.nodes.get(n.wrapping_sub(1)).is_some_and(|nd| nd.gone && nd.freed) {
        n = n.wrapping_sub(1);
    }
    if n < l.n {
        l.holes = l.holes.saturating_sub(l.n.wrapping_sub(n));
        l.n = n;
        let end = |(a, b): (u32, u32)| usize::try_from(a).unwrap_or(0).saturating_add(usize::try_from(b).unwrap_or(0));
        let deps_top = l.nodes.upto(n).filter(|nd| !nd.gone).map(|nd| end(nd.deps).max(end(nd.binds))).max().unwrap_or(0);
        // A path stays until its slot is freed: an unloaded object in its grace period may still
        // be named to a reader standing on it (`l_name`, `dlpi_name`, `dli_fname`).
        let names_top = l.nodes.upto(n).filter(|nd| !nd.freed && nd.path.1 > 0).map(|nd| end(nd.path).saturating_add(1)).max().unwrap_or(0);
        l.deps.set_top(deps_top);
        l.names.set_top(names_top);
    }
    RETIRED.store(l.n_retired, Relaxed);
    TOP.store(l.n, Relaxed);
}

/// Retired objects past which a `dlopen` or `dlclose` first waits for the readers to drain
/// (ruling #5651): a quarter of the tables in place, a count (DL-PROCESS-OBJECTS part 2: the
/// tables grow, so it bounds the memory that retired objects hold, not a table). A reader preempted
/// inside `dl_iterate_phdr` stalls every unmap, so without the wait a loop of opens and closes
/// would grow the tables without end.
const RETIRED_WAIT: usize = 128;
/// `Loader::n_retired` and `Loader::n` as [`reclaim`] last left them, read without the locks by
/// [`room_wait`].
static RETIRED: AtomicUsize = AtomicUsize::new(0);
static TOP: AtomicUsize = AtomicUsize::new(0);

/// Whether a `dlopen`/`dlclose` must make room first: more than [`RETIRED_WAIT`] objects retired,
/// or any retired while the table's top is past three quarters. The second case is readers that
/// are always inside some walk: each reclaim then frees only the objects retired before the
/// previous flip while the newest stay at the top, and the freed slots below become holes; a set
/// goes into the first run of them that holds it ([`settle`]), so the top climbs only when no run
/// does (sets larger than the holes), by every epoch's opens with few retired.
fn room_short() -> bool {
    let r = RETIRED.load(Relaxed);
    r > RETIRED_WAIT || (r > 0 && TOP.load(Relaxed) > CAP.saturating_sub(RETIRED_WAIT) && TOP.load(Relaxed) <= CAP)
}
/// The poll interval of [`room_wait`] (`CLOCK_MONOTONIC`, nanoseconds).
const ROOM_POLL_NS: i64 = 50_000;
const CLOCK_MONOTONIC: i64 = 1;

/// At the entry of `dlopen` and `dlclose`, holding no loader lock: while [`room_short`], reclaim,
/// and wait (poll) for the readers to drain when that was not enough. Never waits when the calling thread holds [`CALL`] (a constructor or
/// destructor: a reader may be blocked on that lock) or is inside a read section (it may be the
/// reader); the call then goes on and grows the tables past their first generation (failing
/// only when no memory is left for them). Each drain lets [`reclaim`] unmap what was flipped before it, so the loop
/// ends once the readers present now have left.
fn room_wait() {
    while room_short() {
        if CALL.held_by_caller() || super::readers::inside() {
            return;
        }
        CALL.lock();
        STATE.lock();
        if let Some(l) = loader() {
            reclaim(l);
        }
        STATE.unlock();
        CALL.unlock();
        if !room_short() {
            return;
        }
        while !super::readers::drained() {
            let ts = [0i64, ROOM_POLL_NS];
            let _ = crate::ffi_sys::niji_sys_clock_nanosleep(CLOCK_MONOTONIC, 0, ts.as_ptr().cast::<c_void>(), core::ptr::null_mut());
        }
    }
}

/// Record, for each new object `base..k`, the objects its relocations bound to that its own
/// dependencies do not list (a start-time object never unloads: left out). When the arena cannot
/// grow (no memory) the definer becomes `nodelete` instead (it then stays, never unmapped under its
/// user), so this never fails the call.
/// Objects past `k` up to `end` are loaded ones above a set that went below the top.
fn record_binds(l: &mut Loader, base: usize, k: usize, end: usize, b: &Binds) {
    for i in base..k {
        let wanted = |l: &Loader, j: usize| b.has(i, j) && !l.deps_of(i).contains(&ix32(j));
        // Room for the whole list first: one list is one span. Without it the list holds what the
        // rest of its generation takes.
        let most = (l.n_start..end).filter(|&j| wanted(l, j)).count();
        let start = l.deps.open(most).unwrap_or_else(|| l.deps.top());
        for j in l.n_start..end {
            if !wanted(l, j) {
                continue;
            }
            if !l.deps.push(ix32(j)) {
                if let Some(nd) = l.nodes.get_mut(j) {
                    nd.nodelete = true;
                }
            }
        }
        if let Some(nd) = l.nodes.get_mut(i) {
            nd.binds = (u32::try_from(start).unwrap_or(0), u32::try_from(l.deps.top().saturating_sub(start)).unwrap_or(0));
        }
    }
}

/// A failed call whose dlerror text is set (always before the attempt is unwound: the text's parts
/// may lie in the attempt's mappings and arena spans).
struct Fail;

/// What the search for one name found.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Found {
    /// Not found (or, under `RTLD_NOLOAD`, not loaded).
    Missing,
    /// An object already in the loader (or earlier in this attempt).
    Loaded(usize),
    /// A new object, now at index `k` of the attempt.
    New(usize),
    /// A candidate refused: the reason and the path it was found at (a span of the path arena).
    Refused(Refusal, (u32, u32)),
    /// Out of room (objects, paths or dependency entries) or a mapping failure: libniji's own text.
    Full(&'static [u8]),
}

/// The search context of one call: the start's environment, auxiliary vector and the cache.
struct Env {
    secure: bool,
    library_path: Option<&'static [u8]>,
    origin_path: Option<&'static [u8]>,
    features: niji_rt::cpu::Features,
    cache: Option<(&'static [u8], Cache<'static>)>,
}

impl Env {
    fn read() -> Env {
        Env::read_with(true)
    }

    /// The environment without the cache (a caller that lists directories, never opens a name).
    fn read_paths() -> Env {
        Env::read_with(false)
    }

    fn read_with(with_cache: bool) -> Env {
        let (envp, auxv) = (start_arg(2), start_arg(3));
        let mut copy = [0u64; AUXV_WORDS_MAX];
        let words = if auxv == 0 { 0 } else { read_auxv(auxv, &mut copy).unwrap_or(0) };
        let aux = Auxv::from_words(copy.get(..words).unwrap_or(&[]));
        let secure = start::secure(&aux);
        let ld = start::ld_env(env_entries(envp));
        let features = Native::features(&aux, &crate::sys::CSys);
        // `ld.so --library-path` replaces LD_LIBRARY_PATH and `--inhibit-cache` the cache, as at start.
        // (`read_cache` reads none after `--inhibit-cache`.)
        let (option_path, _) = super::state::program_options();
        let cache = if with_cache { read_cache() } else { None };
        Env { secure, library_path: option_path.or(ld.library_path), origin_path: ld.origin_path, features, cache }
    }

    fn ctx(&self) -> Ctx<'_> {
        Ctx {
            secure: self.secure,
            library_path: if self.secure { None } else { self.library_path },
            origin_path: if self.secure { None } else { self.origin_path },
            platform: Native::platform(&self.features),
            hwcaps: Native::hwcaps(&self.features),
            cache: self.cache.as_ref().map(|(_, c)| c).into(),
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        if let Some((f, _)) = self.cache {
            let _ = raw::munmap(f.as_ptr().expose_provenance(), f.len());
        }
    }
}

/// The search scope of object `i`: its `DT_RPATH`, `DT_RUNPATH` and `$ORIGIN` (the directory of
/// the path it was opened by; unknown for a start-time object without one).
fn scope_of(l: &Loader, i: usize) -> Scope<'static> {
    let Some(o) = l.objs_upto(l.n.max(i.saturating_add(1))).get(i).copied() else { return Scope::default() };
    let origin = if i == 0 {
        // The program's has no path of its own ([`look_up_origin`]).
        l.origin.get(..l.origin_len.load(Relaxed)).filter(|d| !d.is_empty())
    } else {
        let path = l.path(i);
        path.iter().rposition(|&b| b == b'/').and_then(|p| path.get(..p.max(1)))
    };
    // UNSAFE(slice): a span of the loader's path arena, which rewinds only past freed objects, or
    // the program's published origin.
    #[allow(unsafe_code)]
    // SAFETY: the bytes stay in the loader's mapping for the process's lifetime and a written span
    // is never rewritten while it is in use: a failed attempt rewinds only past its own spans, and
    // `reclaim` only past the spans of objects already unmapped; the origin is written once.
    let origin: Option<&'static [u8]> = origin.map(|s| unsafe { conv::bytes_at(s.as_ptr().expose_provenance(), s.len()) });
    Scope { rpath: dyn_string(&o.dynm, DT_RPATH), runpath: dyn_string(&o.dynm, DT_RUNPATH), origin }
}

/// Whether a search can expand the program's `$ORIGIN`: a token in its `DT_RPATH` or `DT_RUNPATH`.
fn origin_listed(l: &Loader) -> bool {
    let Some(o) = l.objs_upto(l.n).first() else { return false };
    [DT_RPATH, DT_RUNPATH].into_iter().any(|t| dyn_string(&o.dynm, t).is_some_and(|s| s.contains(&b'$')))
}

/// Look up the program's `$ORIGIN` once (DL-RUNPATH): the start's when it looked it up, else by
/// the start's rule now ([`super::link::program_origin`]), so `dlopen`, `RTLD_DI_SERINFO` and
/// `RTLD_DI_ORIGIN` expand it as the start did. Cold: only for a search that can expand it.
#[cold]
#[inline(never)]
fn look_up_origin(l: &mut Loader) {
    if l.origin_tried {
        return;
    }
    l.origin_tried = true;
    let mut out = PathBuf::default();
    let len = START_ORIGIN[1].load(Acquire);
    if len != 0 {
        // UNSAFE(slice): the start's copy of the program's origin in its path store.
        #[allow(unsafe_code)]
        // SAFETY: the start kept the bytes in its path store, never released or rewritten, and
        // published their length (Release) after writing them.
        let kept = unsafe { conv::bytes_at(START_ORIGIN[0].load(Relaxed), len) };
        if !out.push(kept) {
            return;
        }
    } else if !super::link::program_origin(START_RUN_PATH.load(Relaxed), &mut out) {
        return;
    }
    let dir = out.as_bytes();
    if let Some(d) = l.origin.get_mut(..dir.len()) {
        d.copy_from_slice(dir);
        l.origin_len.store(dir.len(), Release);
    }
}

/// Open the candidate `path` for the attempt whose next index is `k`; see [`Found`].
// The arena spans and indices are checked before use.
#[allow(clippy::indexing_slicing)]
fn candidate(l: &mut Loader, path: &[u8], given: &'static [u8], k: usize, searched: bool, noload: bool, page: u64) -> Option<Found> {
    let cpath = path.as_ptr().expose_provenance();
    let fd = crate::ffi_sys::niji_sys_openat(AT_FDCWD, core::ptr::with_exposed_provenance(cpath), i64::from(O_RDONLY | O_CLOEXEC), 0);
    let fd32 = i32::try_from(fd).ok().filter(|&f| f >= 0)?;
    let close = || {
        let _ = crate::ffi_sys::niji_sys_close(fd);
    };
    let shown = path.strip_suffix(b"\0").unwrap_or(path);
    let mut st = niji_kernel::uapi::Stat::default();
    if crate::ffi_sys::niji_sys_fstat(fd, core::ptr::from_mut(&mut st).cast::<c_void>()) != 0 {
        close();
        return None;
    }
    if let Some(j) = l.nodes.upto(k).position(|nd| !nd.gone && (nd.dev, nd.ino) == (st.dev, st.ino)) {
        close();
        return Some(Found::Loaded(j));
    }
    if noload {
        close();
        return Some(Found::Missing);
    }
    let refused = |l: &mut Loader, r: Refusal| Some(l.keep(shown).map_or(Found::Full(NO_ROOM), |s| Found::Refused(r, s)));
    if st.mode & S_IFMT == S_IFDIR {
        close();
        return refused(l, Refusal::Directory);
    }
    let size = crate::ffi_sys::niji_sys_lseek(fd, 0, i64::from(SEEK_END));
    let Some(len) = usize::try_from(size).ok().filter(|&s| s >= niji_ld::elf::EHDR_SIZE) else {
        close();
        return refused(l, Refusal::TooShort);
    };
    let Ok(at) = super::map_view(fd32, len) else {
        close();
        return Some(Found::Full(b"cannot read file data"));
    };
    // UNSAFE(slice): the candidate's file, mapped read-only above.
    #[allow(unsafe_code)]
    // SAFETY: `len` bytes from `at` are a private read-only mapping of the file. It is unmapped on
    // every path that drops the object (refusal, failed attempt) after the last use of the slice,
    // and otherwise kept for the process's lifetime (the loaded object's `Obj` reads it).
    let file: &'static [u8] = unsafe { conv::bytes_at(at, len) };
    let drop_file = || {
        let _ = raw::munmap(at, len);
        close();
    };
    // COMPAT-19: an object asking for an executable stack, accepted under the start-time tunable.
    let mut exec_stack = false;
    let elf = match dl::check::<Native>(file, page) {
        Ok(e) => e,
        Err(Refusal::ExecutableStack) if exec_stack_accepted() => match dl::check_accepting_exec_stack::<Native>(file, page) {
            Ok(e) => {
                exec_stack = true;
                e
            }
            Err(r) => {
                drop_file();
                return refused(l, r);
            }
        },
        Err(r) if r.skipped(searched) => {
            drop_file();
            return None;
        }
        Err(r) => {
            drop_file();
            return refused(l, r);
        }
    };
    let Ok(Some(dynm)) = elf.dynamic() else {
        drop_file();
        return refused(l, Refusal::NoDynamic);
    };
    // `ld.so` by any path is the start-time `ld.so` (ruling #5126).
    if dyn_string(&dynm, DT_SONAME) == Some(Native::INTERP.as_bytes()) {
        drop_file();
        return Some(l.by_soname(Native::INTERP.as_bytes(), k).map_or(Found::Missing, Found::Loaded));
    }
    // Room in every object table for object `k` (DL-PROCESS-OBJECTS part 2: memory is the bound).
    if !l.grow_to(k.saturating_add(1), k) {
        drop_file();
        return Some(Found::Full(NO_ROOM));
    }
    let loaded = if exec_stack {
        map::load_accepting_exec_stack::<Native, _>(&mut SysMapper { fd: fd32 }, file, page)
    } else {
        map::load::<Native, _>(&mut SysMapper { fd: fd32 }, file, page)
    };
    close();
    let loaded = match loaded {
        Ok(x) => x,
        Err(e) => {
            let _ = raw::munmap(at, len);
            return refused(l, dl::of_map_error(e));
        }
    };
    if exec_stack {
        // Before the object's relocation and constructors, which may need the executable stack;
        // kept if the set later fails (the reference's stacks stay executable too).
        super::exec_main_stack(start_arg(3), START_PAGE.load(Relaxed));
        call_stack_exec(l);
    }
    let Ok(sym) = Object::new(dynm) else {
        loaded.unmap(&mut SysMapper { fd: -1 });
        let _ = raw::munmap(at, len);
        super::readers::count_unmapped(1);
        return refused(l, Refusal::Malformed);
    };
    let Some(span) = l.keep(shown) else {
        loaded.unmap(&mut SysMapper { fd: -1 });
        let _ = raw::munmap(at, len);
        super::readers::count_unmapped(1);
        return Some(Found::Full(NO_ROOM));
    };
    let o = Obj { elf, dynm, bias: loaded.bias, name: given, tls: None };
    report_nonshared(&elf, shown);
    l.objs.set(k, o);
    l.syms.set(k, sym);
    let name = l.names.addr(usize::try_from(span.0).unwrap_or(usize::MAX));
    describe(l, k, &o, name, (st.dev, st.ino), true);
    let node = &mut l.nodes[k];
    node.path = span;
    node.deps = (0, 0);
    node.opens = 0;
    node.pins = 0;
    node.global = false;
    node.nodelete = dynm.get(dl::DT_FLAGS_1).is_some_and(|f| f & dl::DF_1_NODELETE != 0);
    node.fini = [0; 3];
    node.file = (at, len);
    node.reserve = (loaded.base, loaded.span);
    node.binds = (0, 0);
    (node.finalising, node.gone, node.freed) = (false, false, false);
    // Its reader entry, invisible until the set is published.
    super::readers::stage(k, node.l_addr, name, usize::try_from(loaded.phdr).unwrap_or(0), usize::from(loaded.phnum));
    Some(Found::New(k))
}

/// Find `name` (needed by object `by`, or by the caller for the root) for the attempt whose next
/// index is `k`: a bare name first matches a loaded object's `DT_SONAME` (P-2), then the search runs
/// with `chain` as requesters.
fn find(l: &mut Loader, name: &'static [u8], chain: &[Scope<'_>], env: &Env, k: usize, noload: bool, page: u64) -> Found {
    let bare = dl::bare(name);
    if bare {
        if let Some(j) = l.by_soname(name, k).or_else(|| l.by_given(name, k)) {
            return Found::Loaded(j);
        }
    } else if let Some(j) = l.by_soname(Native::INTERP.as_bytes(), k).filter(|&j| l.nodes.get(j).is_some_and(|nd| nd.l_name != 0 && c_string(nd.l_name) == name)) {
        // `ld.so` by the path its link map is named by (its `PT_INTERP` path), RTLD_NOLOAD included
        // (probed: another path to its file is not it).
        return Found::Loaded(j);
    }
    let ctx = env.ctx();
    let mut out = Found::Missing;
    search::search::<Native>(name, chain, &ctx, &mut |p: &mut PathBuf| {
        let Some(c) = p.c_str() else { return false };
        match candidate(l, c, name, k, bare, noload, page) {
            Some(f) => {
                out = f;
                true
            }
            None => false,
        }
    });
    out
}

/// One `dlopen` under the lock: the handle (0 on failure) and the constructors to run.
fn open_locked(file: usize, mode: i32, caller: usize) -> (usize, Ctors) {
    let given: &'static [u8] = if file == 0 { b"" } else { c_string(file) };
    let m = match dl::mode(mode) {
        Ok(m) => m,
        Err(e) => {
            if e.prefixed() {
                dlerr::set(&[given, b": ", e.text()]);
            } else {
                dlerr::set(&[e.text()]);
            }
            return (0, Ctors::none());
        }
    };
    let Some(l) = loader() else {
        dlerr::set(&[given, b": cannot set up the dynamic loader's state"]);
        return (0, Ctors::none());
    };
    reclaim(l);
    if given.is_empty() {
        reopen(l, 0, m);
        dlerr::clear();
        return (l.node_addr(0), Ctors::none());
    }
    match load_set(l, given, m, caller) {
        // Also an `RTLD_NOLOAD` miss (handle 0): it clears the pending text (tools/probe/dl/gaps).
        Ok((h, ctors)) => {
            dlerr::clear();
            (h, ctors)
        }
        Err(Fail) => (0, Ctors::none()),
    }
}

/// One more `dlopen` of loaded object `i`: count, `RTLD_NODELETE`, `RTLD_GLOBAL` promotion.
fn reopen(l: &mut Loader, i: usize, m: Mode) {
    let promote = m.global && l.nodes.get(i).is_some_and(|n| !n.global);
    if let Some(node) = l.nodes.get_mut(i) {
        let (opens, pin) = dl::reopen(node.opens);
        node.opens = opens;
        node.nodelete |= pin || m.nodelete;
    }
    if promote {
        let mut local0 = [0u32; CAP];
        let local = room(&mut local0, scratch(l).map(|s| &mut s.local), l.n, 0);
        let len = local_scope(l, i, local);
        make_global(l, local.get(..len).unwrap_or(&[]));
    }
}

/// The breadth-first local scope of `root` (itself first) into `out` (room for every object): its
/// length.
// `len` stays at most `out.len()`: every index is below `l.n` and appears once.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
fn local_scope(l: &Loader, root: usize, out: &mut [u32]) -> usize {
    let Ok(r) = u32::try_from(root) else { return 0 };
    let Some(first) = out.first_mut() else { return 0 };
    *first = r;
    let mut len = 1usize;
    let mut q = 0usize;
    while q < len {
        let deps = l.deps_of(ix(out[q]));
        for &d in deps {
            if len < out.len() && !out[..len].contains(&d) {
                out[len] = d;
                len += 1;
            }
        }
        q += 1;
    }
    len
}

/// Append `set`'s objects that are not global yet to the global scope (`RTLD_GLOBAL`).
fn make_global(l: &mut Loader, set: &[u32]) {
    for &i in set {
        let i = ix(i);
        let Some(node) = l.nodes.get_mut(i) else { continue };
        if node.global {
            continue;
        }
        node.global = true;
        // Never short: `grow_to` gave the scope room for every object.
        if let Some(slot) = l.global.get_mut(l.n_global) {
            *slot = i;
            l.n_global = l.n_global.saturating_add(1);
        }
    }
}

/// Unmap the objects `base..k` of a failed attempt and rewind the arenas and the unique table.
fn unwind(l: &mut Loader, base: usize, k: usize, names_top: usize, deps_top: usize) {
    for node in (base..k).filter_map(|i| l.nodes.get(i)) {
        map::Mapper::unmap(&mut SysMapper { fd: -1 }, node.reserve.0, node.reserve.1);
        let _ = raw::munmap(node.file.0, node.file.1);
    }
    super::readers::count_unmapped(k.saturating_sub(base));
    for i in (base..k).rev() {
        if let Some(p) = l.objs_upto(k).get(i).and_then(|o| o.tls) {
            l.ids.put_back(p.module, l.ids_more.all_mut());
        }
    }
    l.names.set_top(names_top);
    l.deps.set_top(deps_top);
    l.unique.truncate(l.unique_mark);
    l.surplus.rollback(l.tls_mark);
    if base < l.n {
        // A set [`settle`] moved below the top: its slots are freed slots again.
        for i in base..k {
            if let Some(nd) = l.nodes.get_mut(i) {
                free_slot(nd);
            }
        }
        l.holes = l.holes.saturating_add(k.saturating_sub(base));
    }
}

/// Mark `nd` an unused slot: unloaded and freed, matching no file.
fn free_slot(nd: &mut Node) {
    (nd.gone, nd.freed, nd.opens, nd.pins, nd.global) = (true, true, 0, 0, false);
    (nd.dev, nd.ino) = (u64::MAX, u64::MAX.wrapping_sub(1));
}

/// Move the new set `t..k`, written at the top and not yet relocated or published, into the first
/// run of freed slots below the top that holds it: its base there, or `None` (nothing changed) when
/// no run does. Freed slots are past their grace period, so no reader can be on them, and the set's
/// nodes, records and reader entries are not visible yet. The set's paths stay where they are; its
/// dependency lists are renumbered. The top slots it leaves are unused again (at and past `n`).
#[cold]
#[inline(never)]
fn settle(l: &mut Loader, t: usize, k: usize) -> Option<usize> {
    let s = k.checked_sub(t).filter(|&s| s > 0)?;
    let mut run = 0usize;
    let mut h = None;
    for i in l.n_start..t {
        run = if l.nodes.get(i).is_some_and(|nd| nd.gone && nd.freed) { run.saturating_add(1) } else { 0 };
        if run == s {
            h = Some(i.wrapping_add(1).wrapping_sub(s));
            break;
        }
    }
    let h = h?;
    // Everything the move reads is there before anything moves: it never stops half way.
    let whole = |i: usize| {
        let deps = l.nodes.get(i).map(|nd| nd.deps);
        l.objs_upto(k).get(i).is_some() && syms_of(&l.syms, k).get(i).is_some() && deps.is_some_and(|d| d.1 == 0 || l.dep_span(d).len() == ix(d.1))
    };
    if !(t..k).all(whole) || !(h..h.wrapping_add(s)).all(|i| l.nodes.get(i).is_some()) {
        return None;
    }
    let to = |j: u32| -> u32 {
        let j = ix(j);
        if (t..k).contains(&j) { ix32(j.wrapping_sub(t).wrapping_add(h)) } else { ix32(j) }
    };
    for m in 0..s {
        let (src, dst) = (t.wrapping_add(m), h.wrapping_add(m));
        let (Some(o), Some(sy)) = (l.objs_upto(k).get(src).copied(), syms_of(&l.syms, k).get(src).copied()) else { continue };
        let Some(nd) = l.nodes.get(src) else { continue };
        let w = (nd.l_addr, nd.l_name, nd.l_ld, nd.dev, nd.ino, nd.lo, nd.hi, nd.nodelete, nd.deps, nd.path, nd.file, nd.reserve);
        // Checked above: the span is inside one mapped generation.
        let _ = l.deps.rewrite(ix(w.8 .0), ix(w.8 .1), to);
        l.objs.set(dst, o);
        l.syms.set(dst, sy);
        let prev = l.node_addr(dst.wrapping_sub(1));
        let Some(d) = l.nodes.get_mut(dst) else { continue };
        (d.l_addr, d.l_name, d.l_ld, d.dev, d.ino, d.lo, d.hi, d.nodelete, d.deps, d.path, d.file, d.reserve) = w;
        d.l_next.store(0, Relaxed);
        d.l_prev = prev;
        (d.opens, d.pins, d.global, d.fini, d.binds, d.root) = (0, 0, false, [0; 3], (0, 0), 0);
        (d.finalising, d.gone, d.freed) = (false, false, false);
        super::readers::restage(src, dst);
        if let Some(nd) = l.nodes.get_mut(src) {
            free_slot(nd);
        }
    }
    l.holes = l.holes.saturating_sub(s);
    l.reordered = true;
    super::readers::reorder();
    Some(h)
}

/// The chain's last node once sets went below the top ([`settle`]): the walk from the program
/// along `l_next` (the live objects, each once).
#[cold]
#[inline(never)]
fn chain_tail(l: &Loader) -> usize {
    let mut i = 0usize;
    for _ in 0..l.n {
        match chain_next(l, i) {
            Some(j) => i = j,
            None => break,
        }
    }
    i
}

/// The node after node `i` in the chain (the vDSO's link map stepped over), under the lock.
fn chain_next(l: &Loader, i: usize) -> Option<usize> {
    let next = l.nodes.get(i).map_or(0, |nd| super::debug::past_vdso(nd.l_next.load(Relaxed)));
    if next == 0 { None } else { l.index_of(next) }
}

/// Pin the definers of the unique-table entries recorded since mark `from`: an object whose
/// `STB_GNU_UNIQUE` definition was bound is never unloaded (plan §1, design §5).
fn pin_unique(l: &mut Loader, from: usize) {
    let mut owners = [0u32; 64];
    let mut from = from;
    loop {
        let mut n = 0usize;
        for (slot, o) in owners.iter_mut().zip(l.unique.owners(from)) {
            *slot = ix32(o);
            n = n.saturating_add(1);
        }
        for &o in owners.get(..n).unwrap_or(&[]) {
            if let Some(node) = l.nodes.get_mut(ix(o)) {
                node.nodelete = true;
            }
        }
        if n < owners.len() {
            return;
        }
        from = from.saturating_add(n);
    }
}

/// Give each new object (`base..k`) with `PT_TLS` a module ID and a place in the static surplus
/// (DL-16d, plan/ph14/dl-16-dlopen.md §5), recorded in its `Obj` for relocation and in `new` for
/// [`super::publish_tls`]; `new.mods` stays empty for a set without TLS. Nothing is written to any
/// thread yet. A block that has no place fails with `<path>: cannot allocate memory in static TLS
/// block` (dynamic blocks are DL-19's), as does a C library without the thread walk.
fn place_tls(l: &mut Loader, base: usize, k: usize, new: &mut super::NewTls) -> Result<(), Fail> {
    let (lo, hi, tp_align) = super::surplus();
    for i in base..k {
        let Some(mut o) = l.objs_upto(k).get(i).copied() else { continue };
        let Some(ph) = o.elf.find(pt::TLS) else { continue };
        let m = Module::of(&ph);
        let refused = |l: &Loader| fail_parts(&[l_path_static(l, i), b": ", Refusal::StaticTls.text()]);
        if !ltls::image_in_load(&o.elf, &m) {
            return Err(fail_parts(&[l_path_static(l, i), b": ", Refusal::Malformed.text()]));
        }
        if thread_walk(l) == 0 {
            return Err(refused(l));
        }
        let floor = super::module_floor();
        if (l.surplus.is_full() || l.ids.top(floor) >= MODS_IN_PLACE) && !tls_room_past(l, floor) {
            return Err(fail_parts(&[l_path_static(l, i), NO_MEMORY]));
        }
        let Some(off) = l.surplus.place::<Native>(lo, hi, tp_align, &m) else { return Err(refused(l)) };
        let Some(id) = l.ids.take(floor, l.ids_more.all_mut()) else {
            let _ = l.surplus.free(off);
            return Err(refused(l));
        };
        if new.mods.push([o.bias.wrapping_add(m.vaddr), m.filesz, m.memsz, id, u64::from_ne_bytes(off.to_ne_bytes())]).is_err() {
            l.ids.put_back(id, l.ids_more.all_mut());
            let _ = l.surplus.free(off);
            return Err(fail_parts(&[l_path_static(l, i), NO_MEMORY]));
        }
        o.tls = Some(Placed { module: id, tp_offset: off });
        l.objs.set(i, o);
    }
    Ok(())
}

/// The module IDs whose free bits and table entries are in place.
#[allow(clippy::as_conversions)] // const context: `try_from` is not const; 512 fits
const MODS_IN_PLACE: u64 = ltls::MODULES_MAX as u64;

/// [`place_tls`] past the room in place (DL-PROCESS-OBJECTS part 3, rare): a larger store of
/// surplus places when it is full, then, for the next new module ID, the word of its free bit and
/// its entry of the module table every new thread copies. `false` when a mapping cannot be made
/// (the store and words grown so far stay, as every table's generations do).
#[cold]
#[inline(never)]
fn tls_room_past(l: &mut Loader, floor: u64) -> bool {
    if l.surplus.is_full() && !l.surplus.store_mut().grow() {
        return false;
    }
    let next = l.ids.top(floor).saturating_add(1);
    l.ids_more.reserve(ltls::ModuleIds::words_for(next)) && super::reserve_thread_mods(next)
}

/// [`Loader::grow_to`] past the room the tables have (more than [`CAP`] objects, rare): every
/// table's generations, the lock-held lists, the records' contiguous copies, the readers' table,
/// the `fin` store and the [`Scratch`] (made now the first time), each for `len` objects. `room`
/// moves only when all of them have it.
#[cold]
#[inline(never)]
fn grow_cold(l: &mut Loader, len: usize, written: usize) -> bool {
    if len > Gens::<CAP, OBJ_GENS>::MOST {
        return false;
    }
    let tables = l.nodes.reserve(len)
        && l.objs.reserve(len, written)
        && l.syms.reserve(len, written)
        && l.readers.reserve(super::readers::ENTRY_BYTES, len)
        && l.fin.reserve(len)
        && l.global.reserve(len)
        && l.log.reserve(len)
        && l.retired.reserve(len);
    if !tables {
        return false;
    }
    if l.scratch == 0 {
        let flags = MAP_PRIVATE_I32 | MAP_ANONYMOUS_I32 | MAP_NORESERVE_I32;
        let Ok(at) = raw::mmap(0, size_of::<Scratch>(), map::PROT_READ | map::PROT_WRITE, flags, -1, 0) else { return false };
        l.scratch = at;
    }
    if !scratch(l).is_some_and(|s| s.reserve(len)) {
        return false;
    }
    l.room = len;
    true
}

/// The C library's `__niji_thread_walk` (a start object's, at `NIJI_PRIVATE`), looked up once
/// by the first `dlopen` that loads TLS; 0 when no start object has it.
fn thread_walk(l: &mut Loader) -> usize {
    if l.thread_walk == 0 {
        let req = Request::new(start::private::THREAD_WALK, Want::Version(start::private::NODE));
        let n = START_N.load(Acquire).min(l.n);
        let found = l.objs_upto(n).iter().zip(syms_of(&l.syms, n)).find_map(|(o, s)| s.lookup(&req).ok().flatten().map(|d| o.bias.wrapping_add(d.st.value)));
        l.thread_walk = found.and_then(|a| usize::try_from(a).ok()).unwrap_or(0);
    }
    l.thread_walk
}

/// COMPAT-19: whether the start-time `GLIBC_TUNABLES` (the environment and auxiliary vector the
/// process started with, as the search reads them) set `niji.compat.execstack=1`; never in a
/// secure process. Reached only for an object asking for an executable stack.
#[cold]
#[inline(never)]
fn exec_stack_accepted() -> bool {
    let (envp, auxv) = (start_arg(2), start_arg(3));
    let mut copy = [0u64; AUXV_WORDS_MAX];
    let words = if auxv == 0 { 0 } else { read_auxv(auxv, &mut copy).unwrap_or(0) };
    let secure = start::secure(&Auxv::from_words(copy.get(..words).unwrap_or(&[])));
    start::ld_env(env_entries(envp)).tunables.is_some_and(|t| niji::config::Tunables::parse(t, secure).exec_stack == Some(true))
}

/// COMPAT-19: call the C library's `__niji_stack_exec` (a start object's, at `NIJI_PRIVATE`) once
/// an object asking for an executable stack is accepted; nothing when no start object has it.
#[cold]
#[inline(never)]
fn call_stack_exec(l: &Loader) {
    let req = Request::new(start::private::STACK_EXEC, Want::Version(start::private::NODE));
    let n = START_N.load(Acquire).min(l.n);
    let found = l.objs_upto(n).iter().zip(syms_of(&l.syms, n)).find_map(|(o, s)| s.lookup(&req).ok().flatten().map(|d| o.bias.wrapping_add(d.st.value)));
    if let Some(f) = found.and_then(|a| usize::try_from(a).ok()) {
        let _ = crate::ffi::niji_machine_call3(f, 0, 0, 0);
    }
}

/// COMPAT-19 at start without libniji's C library (`run_init`): the program or a start-up
/// dependency asking for an executable stack was accepted; before any constructor, the later thread
/// stacks of a start object defining `__niji_stack_exec` are made executable (the loader made the
/// main stack executable).
#[cold]
#[inline(never)]
pub(super) fn stacks_executable_at_start() {
    CALL.lock();
    STATE.lock();
    if let Some(l) = loader() {
        call_stack_exec(l);
    }
    STATE.unlock();
    CALL.unlock();
}

/// The C library's `__niji_owner_unload` (a start object's, at `NIJI_PRIVATE`; D-27.29),
/// looked up once by the first `dlclose` that unloads an object; 0 when no start object has it.
fn owner_unload(l: &mut Loader) -> usize {
    if l.owner_unload == 0 {
        let req = Request::new(start::private::OWNER_UNLOAD, Want::Version(start::private::NODE));
        let n = START_N.load(Acquire).min(l.n);
        let found = l.objs_upto(n).iter().zip(syms_of(&l.syms, n)).find_map(|(o, s)| s.lookup(&req).ok().flatten().map(|d| o.bias.wrapping_add(d.st.value)));
        l.owner_unload = found.and_then(|a| usize::try_from(a).ok()).unwrap_or(0);
    }
    l.owner_unload
}

/// The text of a failed attempt that found `f` for `name`.
fn found_text<'a>(l: &'a Loader, name: &'a [u8], f: Found, out: &'a mut [&'a [u8]; 3]) -> &'a [&'a [u8]] {
    match f {
        Found::Missing => *out = [name, b": ", dl::FILE_MISSING],
        Found::Refused(r, span) => {
            let p = l.path_span(span).unwrap_or(name);
            *out = [p, b": ", r.text()];
        }
        Found::Full(t) => *out = [name, b": ", t],
        Found::Loaded(_) | Found::New(_) => return &[],
    }
    &out[..]
}

/// Load `given` and the objects it needs as one new set, or re-open it (module documentation).
// Indices below CAP are checked before each store; the per-object tables hold `k - base` entries.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::too_many_lines)]
fn load_set(l: &mut Loader, given: &'static [u8], m: Mode, caller: usize) -> Result<(usize, Ctors), Fail> {
    let page = START_PAGE.load(Relaxed);
    let mut base = l.n;
    let (names_top, deps_top) = (l.names.top(), l.deps.top());
    l.unique_mark = l.unique.len();
    l.tls_mark = l.surplus.len();
    let who = dlsym::caller_index(l, caller);
    let env = Env::read();
    // The program's `$ORIGIN` for its own `$ORIGIN` name or an LD_LIBRARY_PATH token (its lists'
    // tokens looked it up when the loader was made).
    let token = |s: Option<&[u8]>| s.is_some_and(|s| s.contains(&b'$'));
    if !l.origin_tried && ((who == 0 && token(Some(given))) || token(env.library_path)) {
        look_up_origin(l);
    }
    let chain = [scope_of(l, who), scope_of(l, 0)];
    let chain = if who == 0 { &chain[..1] } else { &chain[..] };
    let root = find(l, given, chain, &env, base, m.noload, page);
    let mut k = base;
    match root {
        Found::Loaded(i) => {
            reopen(l, i, m);
            return Ok((l.node_addr(i), Ctors::none()));
        }
        Found::Missing if m.noload => return Ok((0, Ctors::none())),
        Found::New(_) => k += 1,
        f => {
            let t = fail_text(l, given, f, who);
            unwind(l, base, base, names_top, deps_top);
            return Err(t);
        }
    }
    // Breadth-first over the new objects' DT_NEEDED (each the requester of its own).
    // Indices relative to `base`.
    let mut edges: Table<(Rel, Rel), EDGES_INLINE> = Table::new((Rel(0), Rel(0)));
    let mut q = base;
    while q < k {
        let o = l.objs_upto(k)[q];
        // Room for every entry first: one list is one span (the arena grows; only memory bounds it).
        let Some(dep_start) = l.deps.open(o.dynm.needed().count()) else {
            let t = fail_parts(&[given, NO_MEMORY]);
            unwind(l, base, k, names_top, deps_top);
            return Err(t);
        };
        for needed in o.dynm.needed() {
            let Ok(needed) = needed else {
                let t = fail_parts(&[l_path_static(l, q), b": ", Refusal::Malformed.text()]);
                unwind(l, base, k, names_top, deps_top);
                return Err(t);
            };
            let chain = [scope_of(l, q), scope_of(l, 0)];
            let f = if needed == Native::INTERP.as_bytes() {
                l.by_soname(needed, k).map_or_else(|| find(l, needed, &chain, &env, k, false, page), Found::Loaded)
            } else {
                find(l, needed, &chain, &env, k, false, page)
            };
            let j = match f {
                Found::Loaded(j) => j,
                Found::New(j) => {
                    k += 1;
                    j
                }
                Found::Missing => {
                    let t = fail_parts(&[needed, b": ", dl::FILE_MISSING]);
                    unwind(l, base, k, names_top, deps_top);
                    return Err(t);
                }
                f => {
                    let t = fail_text(l, needed, f, who);
                    unwind(l, base, k, names_top, deps_top);
                    return Err(t);
                }
            };
            let edge_full = j >= base && edges.push((Rel(ix32(q - base)), Rel(ix32(j - base)))).is_err();
            // The push stays in the room `open` found (one entry per `DT_NEEDED` entry at most).
            if edge_full || !l.deps.push(ix32(j)) {
                let t = fail_parts(&[given, NO_MEMORY]);
                unwind(l, base, k, names_top, deps_top);
                return Err(t);
            }
        }
        l.nodes[q].deps = (u32::try_from(dep_start).unwrap_or(0), u32::try_from(l.deps.top() - dep_start).unwrap_or(0));
        q += 1;
    }
    // The whole set is known: when freed slots below the top can hold it, it goes to the first run
    // that does (a process that closes objects out of order keeps its tables at what it holds).
    if l.holes >= k - base {
        if let Some(h) = settle(l, base, k) {
            (base, k) = (h, h + (k - base));
        }
    }
    // Every loaded object and the set: `end` is `k` unless the set went below the top.
    let end = k.max(l.n);
    // The set's per-object tables, sized now, while the attempt can still be unwound: from here on
    // only a relocation, version or TLS failure can stop it.
    let Some(mut set_tables) = SetTables::new(base, k - base, end) else {
        let t = fail_parts(&[given, NO_MEMORY]);
        unwind(l, base, k, names_top, deps_top);
        return Err(t);
    };
    // Versions, then pass 1 over the whole set: no foreign code yet.
    if let Err(e) = reloc::check_versions_live(l.objs_upto(end), |j| !l.gone(j)) {
        let t = version_text(l, end, e);
        unwind(l, base, k, names_top, deps_top);
        return Err(t);
    }
    // Static TLS places before relocation (its TPOFF / DTPMOD values need them, DL-16d).
    let mut new_tls = super::NewTls::none();
    if let Err(t) = place_tls(l, base, k, &mut new_tls) {
        unwind(l, base, k, names_top, deps_top);
        return Err(t);
    }
    // Within CAP objects the stack arrays, else the scratch `grow_to` reserved for every object.
    let (s_local, s_scope, s_seen) = match scratch(l) {
        Some(s) => (Some(&mut s.local), Some(&mut s.scope), Some(&mut s.seen)),
        None => (None, None, None),
    };
    let (mut local0, mut scope0, mut seen0) = ([0u32; CAP], [0usize; CAP], [false; CAP]);
    let local = room(&mut local0, s_local, end, 0);
    let llen = local_scope(l, base, local);
    // The new set's scope (plan §2): the global scope, then the root's local scope; with
    // `RTLD_DEEPBIND` the local scope first. Each object once.
    let scope = room(&mut scope0, s_scope, end, 0);
    let mut sn = 0usize;
    let seen = room(&mut seen0, s_seen, end, false);
    let global = l.global().iter().copied();
    let local_it = local[..llen].iter().map(|&i| ix(i));
    let mut push = |i: usize| {
        if i < seen.len() && !seen[i] && sn < scope.len() {
            seen[i] = true;
            scope[sn] = i;
            sn += 1;
        }
    };
    if m.deepbind {
        local_it.chain(global).for_each(&mut push);
    } else {
        global.chain(local_it).for_each(&mut push);
    }
    for i in base..k {
        l.nodes[i].root = ix32(base);
    }
    let start = l.global.slice(l.n_start_global).unwrap_or(&[]);
    let (objs, syms) = (objs_of(&l.objs, end), syms_of(&l.syms, end));
    let set = Objects { objs, syms, global: &scope[..sn] };
    let SetTables { binds, deferred, order, work, ctors } = &mut set_tables;
    let mut unique = Uniq { table: &mut l.unique, syms, start, binds: Some(binds) };
    for i in (base..k).rev() {
        match reloc::relocate_first::<Native, _>(&set, i, &mut ProcImage, page, &mut unique) {
            Ok(d) => deferred[i - base] = d,
            Err(e) => {
                let t = reloc_text(l, i, e);
                unwind(l, base, k, names_top, deps_top);
                return Err(t);
            }
        }
    }
    // Pass 2: the IFUNC resolvers, only now (ruling #5125).
    for i in (base..k).rev().filter(|&i| deferred[i - base]) {
        if let Err(e) = reloc::relocate_deferred::<Native, _>(&set, i, &mut ProcImage, &mut unique) {
            let t = reloc_text(l, i, e);
            unwind(l, base, k, names_top, deps_top);
            return Err(t);
        }
    }
    // Each new dynamic array's address entries as run-time addresses, before any reader or
    // constructor sees the object (DL-DYNAMIC-RELOC).
    for o in &objs[base..k] {
        reloc::publish_dynamic(o, &mut ProcImage);
    }
    if let Err(i) = reloc::protect_relro(&objs[base..k], &mut ProcImage, page) {
        let t = fail_parts(&[l_path_static(l, base + i), b": cannot apply additional memory protection after relocation"]);
        unwind(l, base, k, names_top, deps_top);
        return Err(t);
    }
    record_binds(l, base, k, end, binds);
    // Every thread gets the new modules' blocks, and later threads the table, before any code
    // can reach them (DL-16d; nothing below fails).
    if !new_tls.mods.is_empty() {
        super::publish_tls(l.thread_walk, &mut new_tls);
    }
    // Publication, after pass 2 and before the constructors (plan §4): one Release store links the
    // whole new set into the chain, one makes it visible to the readers (`dl_iterate_phdr`,
    // `_dl_find_object`) with `dlpi_adds`. The debugger rendezvous announces it around the change.
    super::debug::change(niji_ld::debug::State::Add);
    for i in base..k {
        let next = if i + 1 < k { l.node_addr(i + 1) } else { 0 };
        l.nodes[i].l_next.store(next, Relaxed);
    }
    // The chain's tail, after which the set goes (the chain is in load order, probe P-12): the last
    // object below the set that is not unloaded (the program at least), or, once a set went below
    // the top, the chain's own last node.
    let tail = if l.reordered { chain_tail(l) } else { (0..base).rev().find(|&i| !l.nodes[i].gone).unwrap_or(0) };
    l.n = end;
    let first = l.node_addr(base);
    l.nodes[base].l_prev = l.node_addr(tail);
    l.nodes[tail].l_next.store(first, Release);
    super::readers::publish(base, k);
    super::debug::change(niji_ld::debug::State::Consistent);
    l.nodes[base].opens = 1;
    l.nodes[base].nodelete |= m.nodelete;
    pin_unique(l, l.unique_mark);
    if m.global {
        make_global(l, &local[..llen]);
    }
    // Constructors of the new set, dependencies first (D-27.19).
    let len = init::init_order(k - base, &edges, order, work).unwrap_or(0);
    for &j in &order[..len] {
        let i = base + j;
        let o = l.objs_upto(k)[i];
        let h = init::hooks(&o.dynm, o.bias, false);
        l.nodes[i].fini = [h.fini_array.addr, h.fini_array.len, h.fini];
        // Never fails: `SetTables::new` gave the table room for every new object.
        let _ = ctors.calls.push((h.init, h.init_array.addr, h.init_array.len));
        let n = l.n_log.load(Relaxed);
        // Never short: `grow_to` gave the log room for every object.
        if let Some(slot) = l.log.get_mut(n) {
            *slot = ix32(i);
            l.n_log.store(n + 1, Release);
        }
    }
    Ok((l.node_addr(base), set_tables.ctors))
}

/// One `dlopen`'s tables with an entry per new object (index - `base`), all sized before the set
/// is relocated: the bindings its relocations make, its pass-2 marks, its init walk and its
/// constructors (filled in init order).
struct SetTables {
    binds: Binds,
    deferred: Table<bool, INLINE>,
    order: Table<usize, INLINE>,
    work: Table<init::Walk, INLINE>,
    ctors: Ctors,
}

impl SetTables {
    /// The tables for `n` new objects from index `base`; `None` when one cannot grow to `n`.
    fn new(base: usize, n: usize, end: usize) -> Option<Self> {
        // A bit per object up to `end` (every loaded object and the set), whole words (`CAP` bits
        // within `CAP` objects).
        let width = base.checked_add(n)?.max(end).max(CAP).div_ceil(64);
        let mut s = SetTables { binds: Binds { base, width, bits: Table::new(0) }, deferred: Table::new(false), order: Table::new(0), work: Table::new(init::Walk::default()), ctors: Ctors::none() };
        s.binds.bits.reserve(n.checked_mul(width)?).ok()?;
        s.deferred.reserve(n).ok()?;
        s.order.reserve(n).ok()?;
        s.work.reserve(n).ok()?;
        s.ctors.calls.reserve(n).ok()?;
        s.ctors.calls.clear();
        Some(s)
    }
}

/// Set the calling thread's dlerror text to `parts` (copied at once into its record).
fn fail_parts(parts: &[&[u8]]) -> Fail {
    dlerr::set(parts);
    Fail
}

/// The path of object `i` (its name when it has none) as a `'static` slice.
fn l_path_static(l: &Loader, i: usize) -> &'static [u8] {
    let p = l.path(i);
    let p = if p.is_empty() { l.objs_upto(l.n.max(i.saturating_add(1))).get(i).map_or(&b"?"[..], |o| o.name) } else { p };
    // UNSAFE(slice): bytes in the loader's mapping or an object's image, both kept for the process.
    #[allow(unsafe_code)]
    // SAFETY: the slice is copied into the dlerror record (`fail_parts`) before any arena rewind.
    unsafe {
        conv::bytes_at(p.as_ptr().expose_provenance(), p.len())
    }
}

fn fail_text(l: &Loader, name: &[u8], f: Found, who: usize) -> Fail {
    if let Found::Refused(Refusal::ExecutableStack, span) = f {
        // COMPAT-19 (ruling of 2026-11-20): the object, the `dlopen` caller and the ways in.
        let p = l.path_span(span).unwrap_or(name);
        // The program's own entry has an empty path: name it by `argv[0]`.
        let from = l_path_static(l, who);
        let argv0 = || Some(start_arg(1)).filter(|&v| v != 0).and_then(|v| usize::try_from(super::stack_word(v)).ok()).filter(|&a| a != 0).map_or(&b"the program"[..], c_string);
        let from = if from.is_empty() { argv0() } else { from };
        return fail_parts(&[p, b": ", Refusal::ExecutableStack.text(), b" (dlopen called from ", from, b"); ", dl::EXEC_STACK_WAYS]);
    }
    let mut out: [&[u8]; 3] = [b"", b"", b""];
    let parts = found_text(l, name, f, &mut out);
    fail_parts(parts)
}

fn version_text(l: &Loader, k: usize, e: VersionError<'_>) -> Fail {
    fn by_file<'a>(l: &'a Loader, k: usize, file: &'a [u8]) -> &'a [u8] {
        match l.objs_upto(k).iter().enumerate().position(|(j, o)| !l.gone(j) && (soname(o) == Some(file) || o.name == file)) {
            Some(j) if !l.path(j).is_empty() => l.path(j),
            _ => file,
        }
    }
    match e {
        VersionError::NoVersions { by, file, version } | VersionError::Missing { by, file, version } => {
            fail_parts(&[by_file(l, k, file), b": version `", version, b"' not found (required by ", l_path_static(l, by), b")"])
        }
        VersionError::Elf(by, _) => fail_parts(&[l_path_static(l, by), b": ", Refusal::Malformed.text()]),
    }
}

fn reloc_text(l: &Loader, i: usize, e: RelocError<'_>) -> Fail {
    let who = l_path_static(l, i);
    match e {
        RelocError::Undefined(sym) => fail_parts(&[who, b": undefined symbol: ", sym]),
        RelocError::IndirectExternAccess(sym) => {
            let msg = e.message();
            fail_parts(&[who, b": ", msg, sym])
        }
        e => {
            let msg = e.message();
            fail_parts(&[who, b": ", msg.strip_suffix(b"\n").unwrap_or(msg)])
        }
    }
}
