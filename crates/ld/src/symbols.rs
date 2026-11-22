//! Symbol lookup (spec §27.1, phase 14 DL-9, plan/ph14/design.md §5): the GNU and SysV hash
//! functions, lookup in one object through either hash table, search scopes, version matching,
//! `STB_GNU_UNIQUE` and the weak rules. The rules are decision D-27.11 (plan/07-decisions/s26-29.md);
//! the version rules were settled by black-box probes of the reference loader
//! (spec/behaviour/dynlink.md, "Symbol binding").
//!
//! Pure logic over the [`crate::elf`] reader: no I/O, no allocation, no process. Every index from an
//! object goes through the reader's checked accessors, so a malformed table gives a [`LookupError`],
//! never a panic or a read outside the object. Nothing here is architecture-specific (design §0a).
//!
//! Version names are compared as bytes; no version node is special (`NIJI_PRIVATE` and any
//! other name are ordinary nodes, ruling R14-12).

use core::cell::{Cell, OnceCell};

use crate::elf::{
    Dynamic, ElfError, GnuHash, Hashes, Sym, Symtab, SysvHash, Tags, Versions, Versym, SHN_UNDEF, STB_GLOBAL, STB_GNU_UNIQUE, STB_WEAK,
    STT_COMMON, STT_FUNC, STT_GNU_IFUNC, STT_NOTYPE, STT_OBJECT, STT_TLS, STV_HIDDEN, STV_INTERNAL, VERSYM_HIDDEN,
};

/// `VER_NDX_LOCAL`: the symbol is local to its object and never binds.
pub const VER_NDX_LOCAL: u16 = 0;
/// `VER_NDX_GLOBAL`: the symbol has the object's base version (it is unversioned).
pub const VER_NDX_GLOBAL: u16 = 1;

/// The SysV ELF hash of `name` (gABI "Hash Table" section).
#[must_use]
pub fn sysv_hash(name: &[u8]) -> u32 {
    let mut h: u32 = 0;
    for &c in name {
        h = h.wrapping_shl(4).wrapping_add(u32::from(c));
        let g = h & 0xf000_0000;
        h ^= g.wrapping_shr(24);
        h &= !g;
    }
    h
}

/// The GNU hash of `name`: `h = h * 33 + c` from 5381, modulo 2^32.
#[must_use]
pub fn gnu_hash(name: &[u8]) -> u32 {
    name.iter().fold(5381u32, |h, &c| h.wrapping_mul(33).wrapping_add(u32::from(c)))
}

/// Why a lookup failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookupError {
    /// A table of the object is malformed (the reader's error, with the index it was about).
    Elf(ElfError),
    /// The object has no `DT_SYMTAB`.
    NoSymtab,
    /// The object has no hash table of the style asked for (or none at all).
    NoHashTable,
    /// Symbol `sym` carries version index `index`, which no version definition (or, for a
    /// reference, no version requirement) of its object gives.
    UnknownVersion {
        /// The symbol index.
        sym: u32,
        /// The version index from `versym`.
        index: u16,
    },
    /// A scope names an object index that the object list does not have.
    NoObject(usize),
    /// A strong reference has no definition in its scope.
    Undefined,
    /// The unique-symbol registry could not record a definition (allocation failed).
    NoMemory,
}

/// The diagnostic for [`LookupError::NoMemory`] (after `<object>: `): it names what ran out, since
/// the tables themselves are sound.
pub const UNIQUE_NO_MEMORY: &[u8] = b"cannot allocate memory for the unique symbol table\n";

impl From<ElfError> for LookupError {
    fn from(e: ElfError) -> Self {
        LookupError::Elf(e)
    }
}

/// Which hash table a lookup walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashStyle {
    /// `DT_GNU_HASH` (Bloom filter, buckets, chains).
    Gnu,
    /// `DT_HASH`.
    Sysv,
}

/// Which definitions a request accepts (D-27.11; spec/behaviour/dynlink.md "Symbol binding").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want<'n> {
    /// A relocation's reference without a version (its `versym` index is 0 or 1, or its object has no
    /// `versym`): the definition with the lowest version index, hidden or not, so the base definition
    /// first, then the oldest version.
    Oldest,
    /// A lookup by name alone (`dlsym`): a base or default (not hidden) definition only.
    Default,
    /// A named version (a reference's `vna_other` resolved to its name, or `dlvsym`): the definition of
    /// exactly that version, hidden or not. A base (index 1) definition never matches a name, and
    /// nothing in an object without `versym` does.
    Version(&'n [u8]),
    /// A relocation's reference with a version (its `versym` index names one): the definition of exactly
    /// that version, hidden or not, or else a base (index 1) definition, so an unversioned definition
    /// earlier in the scope (the program's, a preloaded object's) interposes on a versioned reference
    /// (probe-settled; spec/behaviour/dynlink.md "Symbol binding"). In an object without `versym` every
    /// definition matches.
    Reference(&'n [u8]),
}

/// One symbol request: a name and the definitions it accepts. The hashes are computed once per request
/// (the SysV one only when an object without a GNU table is searched).
#[derive(Clone, Debug)]
pub struct Request<'n> {
    name: &'n [u8],
    want: Want<'n>,
    gnu: u32,
    sysv: OnceCell<u32>,
    /// The version names found so far by the pass this request belongs to ([`VersionMemo`]).
    memo: Option<&'n VersionMemo>,
    /// The name holds no NUL, so a candidate's name is compared in place ([`Dynamic::string_is`]).
    plain: bool,
    /// For a [`Want::Reference`] whose name is its own object's definition of a version index (no
    /// requirement gives the index): that object's string table address and the index. A definition
    /// in that same object with that index has exactly that version name, so it matches without
    /// finding and comparing the name again (LD-LOOKUP-COST).
    origin: Option<(usize, u16)>,
}

impl<'n> Request<'n> {
    /// A request for `name` accepting `want`.
    #[must_use]
    pub fn new(name: &'n [u8], want: Want<'n>) -> Self {
        Request { name, want, gnu: gnu_hash(name), sysv: OnceCell::new(), memo: None, plain: !name.contains(&0), origin: None }
    }

    /// A request for `name`, a string of an object's string table (so without a NUL), whose GNU hash
    /// is `gnu` ([`Dynamic::hashed_string`]), accepting `want`, and the `origin` of its version name
    /// ([`Object::reference_want_origin`]).
    #[must_use]
    pub fn of_table(name: &'n [u8], gnu: u32, want: Want<'n>, origin: Option<(usize, u16)>) -> Self {
        Request { name, want, gnu, sysv: OnceCell::new(), memo: None, plain: true, origin }
    }

    /// This request, finding its definitions' version names through `memo` ([`VersionMemo`]).
    #[must_use]
    pub fn with_memo(mut self, memo: &'n VersionMemo) -> Self {
        self.memo = Some(memo);
        self
    }
    /// The name.
    #[must_use]
    pub fn name(&self) -> &'n [u8] {
        self.name
    }
    /// What the request accepts.
    #[must_use]
    pub fn want(&self) -> Want<'n> {
        self.want
    }
    fn sysv(&self) -> u32 {
        *self.sysv.get_or_init(|| sysv_hash(self.name))
    }
}

/// Slots of a [`VersionMemo`] (a power of two).
const MEMO_SLOTS: usize = 64;

/// [`MemoSlot`] key bits: a reference's name (else a definition's).
const MEMO_REF: u32 = 1 << 16;
/// [`MemoSlot`] key bits: the name is one of the object's own definitions (else a requirement's).
const MEMO_OWN: u32 = 1 << 17;
/// [`MemoSlot`] key bits: the name's length, from this bit (13 bits; a longer name is not remembered).
const MEMO_LEN_SHIFT: u32 = 18;
/// [`MemoSlot`] key bits: the slot is in use.
const MEMO_USED: u32 = 1 << 31;
/// The [`MemoSlot`] key bits a probe compares: index, walk and use.
const MEMO_MATCH: u32 = 0xffff | MEMO_REF | MEMO_USED;

/// One remembered version name, 16 bytes: the object's string table (its address: the object's
/// identity for the pass), the key (the index; [`MEMO_REF`]: which walk found it; [`MEMO_OWN`]:
/// whose name it is; the length; [`MEMO_USED`]) and where the name starts in that string table.
#[derive(Clone, Copy, Debug, Default)]
struct MemoSlot {
    strtab: usize,
    key: u32,
    off: u32,
}

/// A [`MemoSlot`]'s key bits a probe compares, and its slot, for `index` of the object whose string
/// table is at `base`, as a reference's name or a definition's.
#[inline]
fn memo_key(index: u16, reference: bool, base: usize) -> (u32, usize) {
    let key = u32::from(index) | if reference { MEMO_REF } else { 0 } | MEMO_USED;
    let at = (usize::from(index) ^ usize::from(reference).wrapping_mul(MEMO_SLOTS / 2) ^ (base >> 4) ^ (base >> 10)) & (MEMO_SLOTS - 1);
    (key, at)
}

/// The version names one relocation pass has found, by object and version index, so the walks of
/// the version tables run once per name and not once per symbolic relocation (DYN-START-SPEED:
/// they were a sixth of a dynamic start). A direct-mapped cache on the pass's stack: a slot holds an
/// offset in the object's own string table, so it has no lifetime, and a miss walks the tables as
/// before. Answers are the walks' answers; only found names are remembered. 64 slots (1 KiB): a
/// program needing a few dozen versions of the C library (each a reference and a definition key)
/// thrashed 16 (LD-LOOKUP-COST).
#[derive(Debug)]
pub struct VersionMemo {
    slots: [Cell<MemoSlot>; MEMO_SLOTS],
}

impl Default for VersionMemo {
    fn default() -> Self {
        VersionMemo { slots: [const { Cell::new(MemoSlot { strtab: 0, key: 0, off: 0 }) }; MEMO_SLOTS] }
    }
}

/// A definition found in one object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Def {
    /// The symbol index in the defining object.
    pub sym: u32,
    /// The symbol.
    pub st: Sym,
    /// Its version index (1, the base, when the object has no `versym`).
    pub version: u16,
    /// Whether that version is hidden (not the default).
    pub hidden: bool,
}

/// The hash table an [`Object`] walks. The table's mark (`GnuHash::mark`, `SysvHash::mark`) says
/// this is the program's view and the program has canonical PLT entries ([`Object::mark_program`]): it
/// sits in the table's tail padding, so neither this enum nor [`Object`] grows (a fourth variant
/// carrying a GNU table would: the enum's spare discriminant values only fit a SysV-sized payload).
#[derive(Clone, Copy, Debug)]
enum Hash<'a> {
    Gnu(GnuHash<'a>),
    Sysv(SysvHash<'a>),
}

/// The lookup view of one loaded object: its symbol table, one hash table and its version tables.
#[derive(Clone, Copy, Debug)]
pub struct Object<'a> {
    dynm: Dynamic<'a>,
    symtab: Symtab<'a>,
    hash: Hash<'a>,
    versym: Option<Versym<'a>>,
    /// The version tables when `versym` is present ([`Dynamic::versions`]).
    vers: Option<Versions<'a>>,
}

// The loader keeps its views in an inline table on the start-up stack and passes `Scope` by value
// on every lookup: neither may grow (DL-CANONICAL-PLT: a field here cost a stack page).
#[cfg(target_pointer_width = "64")]
const _: () = assert!(core::mem::size_of::<Object<'static>>() == 0x128 && core::mem::size_of::<Scope<'static>>() == 0x30);

/// Whether `s` is a definition another object may bind: defined, a global, weak or unique binding, a
/// data, code, TLS or IFUNC type, and not hidden or internal.
fn exported(s: &Sym) -> bool {
    s.shndx != SHN_UNDEF
        && matches!(s.bind(), STB_GLOBAL | STB_WEAK | STB_GNU_UNIQUE)
        && matches!(s.kind(), STT_NOTYPE | STT_OBJECT | STT_FUNC | STT_COMMON | STT_TLS | STT_GNU_IFUNC)
        && !matches!(s.visibility(), STV_HIDDEN | STV_INTERNAL)
}

/// Whether the program's symbol `s` is a canonical PLT entry: an undefined global or weak function
/// with a non-zero value, which is the address of the program's PLT entry for it. The gABI ("Symbol
/// Values") makes that entry the function's address for every reference to the address in the
/// process; only the calls through the entry's own slot (`JUMP_SLOT`) go to the definition.
#[must_use]
pub fn canonical_plt(s: &Sym) -> bool {
    s.shndx == SHN_UNDEF && s.value != 0 && s.kind() == STT_FUNC && matches!(s.bind(), STB_GLOBAL | STB_WEAK)
}

impl<'a> Object<'a> {
    /// The object's lookup view, walking the GNU hash table when there is one, else the SysV table.
    ///
    /// # Errors
    ///
    /// [`LookupError::NoSymtab`], [`LookupError::NoHashTable`], or the reader's errors for the symbol,
    /// hash and version tables.
    pub fn new(dynm: Dynamic<'a>) -> Result<Self, LookupError> {
        // One pass over the dynamic entries, and each hash table read once, for every table below
        // (LD-OBJECT-ONCE: each request walked the entries again, and the GNU table was read six
        // times). The answers and the order of the errors are those of the requests one by one.
        let t = dynm.tags();
        let h = dynm.hashes(&t);
        let style = if h.gnu?.is_some() { HashStyle::Gnu } else { HashStyle::Sysv };
        Self::build(dynm, style, &t, &h)
    }

    /// The object's lookup view, walking the hash table of style `style`.
    ///
    /// # Errors
    ///
    /// As [`Object::new`]; [`LookupError::NoHashTable`] when the object has no table of that style.
    pub fn with_style(dynm: Dynamic<'a>, style: HashStyle) -> Result<Self, LookupError> {
        let t = dynm.tags();
        Self::build(dynm, style, &t, &dynm.hashes(&t))
    }

    /// [`Object::with_style`] from the object's tags `t` and hash tables `h`.
    fn build(dynm: Dynamic<'a>, style: HashStyle, t: &Tags, h: &Hashes<'a>) -> Result<Self, LookupError> {
        // Counted once for both tables (the count reads every hash bucket; DYN-START-PARITY).
        let count = h.symbol_count();
        let symtab = dynm.symtab_with(t, h, count)?.ok_or(LookupError::NoSymtab)?;
        let hash = match style {
            HashStyle::Gnu => Hash::Gnu(h.gnu?.ok_or(LookupError::NoHashTable)?),
            HashStyle::Sysv => Hash::Sysv(h.sysv?.ok_or(LookupError::NoHashTable)?),
        };
        // The version tables, walked whole for `versym`'s checks and kept so a lookup by index does
        // not find them again (DYN-START-PARITY); without `DT_VERSYM` the walks below find them
        // each time.
        let (versym, vers) = match dynm.versym_with_versions(t, count)? {
            Some((v, vers)) => (Some(v), vers),
            None => (None, None),
        };
        Ok(Object { dynm, symtab, hash, versym, vers })
    }

    /// Makes this view the program's: its canonical PLT entries ([`canonical_plt`]) define their
    /// functions for every lookup. Only the program has them (gABI "Symbol Values": the executable
    /// file's PLT); a shared object's undefined symbols stay references whatever their value. A
    /// `JUMP_SLOT` must not bind such an entry (the entry is itself the slot's caller): the relocator
    /// takes those through its own path when [`Object::has_canonical_plt`] says there are entries.
    ///
    /// The entries are found once here, among the symbols the hash table covers (a GNU table leaves
    /// out the symbols below `symoffset`, which a linker gives only to plain references); a program
    /// without one keeps the plain view. In place: only the table's mark byte is written (taking and
    /// returning the view by value copied all of it).
    #[inline(never)]
    pub fn mark_program(&mut self) {
        let first = match &self.hash {
            Hash::Gnu(g) => g.symoffset(),
            Hash::Sysv(_) => 1,
        };
        let count = self.symtab.count().unwrap_or(0);
        let canonical = (first..count).any(|i| self.symtab.get(i).is_ok_and(|s| canonical_plt(&s)));
        match &mut self.hash {
            Hash::Gnu(g) => g.set_mark(canonical),
            Hash::Sysv(s) => s.set_mark(canonical),
        }
    }

    /// Whether this is the program's view and the program has canonical PLT entries
    /// ([`Object::mark_program`]).
    #[must_use]
    pub fn has_canonical_plt(&self) -> bool {
        match self.hash {
            Hash::Gnu(g) => g.mark(),
            Hash::Sysv(s) => s.mark(),
        }
    }

    /// Whether symbol `i`, a hash candidate that is not an exported definition, is one of the
    /// program's canonical PLT entries ([`Object::mark_program`]). Cold and out of line: a lookup reaches
    /// it only for such a candidate, so the flag's read (a match on the hash table) stays off the
    /// walk. It takes the index and reads the symbol again, so the caller need not keep the symbol
    /// in memory to pass it by reference on every candidate.
    #[cold]
    #[inline(never)]
    fn canonical_entry(&self, i: u32) -> bool {
        self.has_canonical_plt() && self.symtab.get(i).is_ok_and(|s| canonical_plt(&s))
    }

    /// The dynamic section the view reads.
    #[must_use]
    pub fn dynamic(&self) -> Dynamic<'a> {
        self.dynm
    }

    /// The symbol table.
    #[must_use]
    pub fn symtab(&self) -> Symtab<'a> {
        self.symtab
    }

    /// The name of version index `index` in this object's definitions (`None` for 0 and 1, which name no
    /// version, and for an index no definition gives).
    ///
    /// # Errors
    ///
    /// The reader's errors for the version definitions.
    pub fn version_name(&self, index: u16) -> Result<Option<&'a [u8]>, LookupError> {
        if index <= VER_NDX_GLOBAL {
            return Ok(None);
        }
        if let Some(v) = self.vers {
            return Ok(v.def_name(&self.dynm, index)?);
        }
        let Some(defs) = self.dynm.verdef()? else { return Ok(None) };
        for d in defs {
            let d = d?;
            if d.ndx & !VERSYM_HIDDEN == index {
                return Ok(d.names().next().transpose()?.map(|a| a.name));
            }
        }
        Ok(None)
    }

    /// What this object's reference through symbol `sym` accepts: [`Want::Oldest`] without a version,
    /// else [`Want::Reference`] to the version its `versym` index names (a requirement's `vna_other`, or
    /// one of the object's own definitions).
    ///
    /// # Errors
    ///
    /// The reader's errors, or [`LookupError::UnknownVersion`] for an index neither table gives.
    pub fn reference_want(&self, sym: u32) -> Result<Want<'a>, LookupError> {
        self.reference_want_with(sym, None)
    }

    /// [`Object::reference_want`], finding the name through `memo` when there is one.
    ///
    /// # Errors
    ///
    /// As [`Object::reference_want`].
    pub fn reference_want_with(&self, sym: u32, memo: Option<&VersionMemo>) -> Result<Want<'a>, LookupError> {
        let Some(vs) = self.versym else { return Ok(Want::Oldest) };
        let index = vs.get(sym)?.index;
        if index <= VER_NDX_GLOBAL {
            return Ok(Want::Oldest);
        }
        self.memo_name(memo, index, true)?.map(Want::Reference).ok_or(LookupError::UnknownVersion { sym, index })
    }

    /// [`Object::reference_want_with`], and the request's origin ([`Request::of_table`]): this
    /// object's string table address and the version index, when the name is one of this object's
    /// own definitions (no requirement gives the index), else `None`.
    ///
    /// # Errors
    ///
    /// As [`Object::reference_want`].
    pub fn reference_want_origin(&self, sym: u32, memo: &VersionMemo) -> Result<(Want<'a>, Option<(usize, u16)>), LookupError> {
        let Some(vs) = self.versym else { return Ok((Want::Oldest, None)) };
        let index = vs.get(sym)?.index;
        if index <= VER_NDX_GLOBAL {
            return Ok((Want::Oldest, None));
        }
        // The reference side answers from this object's own definitions only when no requirement
        // gives the index; a definition of this object with this index then has exactly that name
        // (the definition side asks its own definitions first).
        let (name, own) = self.memo_name_from(Some(memo), index, true)?.ok_or(LookupError::UnknownVersion { sym, index })?;
        let origin = if own { self.dynm.strtab().map(|t| (t.as_ptr().addr(), index)) } else { None };
        Ok((Want::Reference(name), origin))
    }

    /// The version name a reference with index `index` names: a requirement's, else one of the
    /// object's own definitions (`true`: the name is the definition's).
    fn reference_name(&self, index: u16) -> Result<Option<(&'a [u8], bool)>, LookupError> {
        if let Some(name) = self.needed_version_name(index)? {
            return Ok(Some((name, false)));
        }
        Ok(self.version_name(index)?.map(|n| (n, true)))
    }

    /// [`Object::reference_name`] (`reference`) or [`Object::def_version_name`] of `index`, through
    /// `memo` ([`VersionMemo`]) when there is one.
    fn memo_name(&self, memo: Option<&VersionMemo>, index: u16, reference: bool) -> Result<Option<&'a [u8]>, LookupError> {
        Ok(self.memo_name_from(memo, index, reference)?.map(|(n, _)| n))
    }

    /// [`Object::memo_name`], and whether the name is one of this object's own definitions (else a
    /// requirement's).
    #[inline]
    fn memo_name_from(&self, memo: Option<&VersionMemo>, index: u16, reference: bool) -> Result<Option<(&'a [u8], bool)>, LookupError> {
        // The hit, inline in each caller; a miss (once per name and pass) walks the tables in
        // `memo_fill` (LD-LOOKUP-COST).
        if let (Some(memo), Some(tab)) = (memo, self.dynm.strtab()) {
            let base = tab.as_ptr().addr();
            let (key, at) = memo_key(index, reference, base);
            if let Some(s) = memo.slots.get(at).map(Cell::get) {
                if s.key & MEMO_MATCH == key && s.strtab == base {
                    let off = usize::try_from(s.off).unwrap_or(usize::MAX);
                    let len = usize::try_from(s.key >> MEMO_LEN_SHIFT).unwrap_or(0) & 0x1fff;
                    if let Some(name) = off.checked_add(len).and_then(|end| tab.get(off..end)) {
                        return Ok(Some((name, s.key & MEMO_OWN != 0)));
                    }
                }
            }
        }
        self.memo_fill(memo, index, reference)
    }

    /// [`Object::memo_name_from`] by the walks, remembering a found name in `memo` when there is one.
    #[cold]
    #[inline(never)]
    fn memo_fill(&self, memo: Option<&VersionMemo>, index: u16, reference: bool) -> Result<Option<(&'a [u8], bool)>, LookupError> {
        let found = if reference { self.reference_name(index)? } else { self.def_version_name_from(index)? };
        let (Some(memo), Some(tab), Some((n, own))) = (memo, self.dynm.strtab(), found) else { return Ok(found) };
        let base = tab.as_ptr().addr();
        let (key, at) = memo_key(index, reference, base);
        let off = n.as_ptr().addr().wrapping_sub(base);
        if let (Some(slot), true) = (memo.slots.get(at), off.checked_add(n.len()).is_some_and(|end| end <= tab.len())) {
            if let (Ok(off), Ok(len @ 0..0x2000)) = (u32::try_from(off), u32::try_from(n.len())) {
                slot.set(MemoSlot { strtab: base, key: key | if own { MEMO_OWN } else { 0 } | len << MEMO_LEN_SHIFT, off });
            }
        }
        Ok(found)
    }

    /// The name of version index `index` among this object's requirements (the `vna_other` of a
    /// `DT_VERNEED` entry), `None` when no requirement has it.
    fn needed_version_name(&self, index: u16) -> Result<Option<&'a [u8]>, LookupError> {
        if let Some(v) = self.vers {
            return Ok(v.need_name(&self.dynm, index)?);
        }
        if let Some(needs) = self.dynm.verneed()? {
            for n in needs {
                for a in n?.auxes() {
                    let a = a?;
                    if a.other & !VERSYM_HIDDEN == index {
                        return Ok(Some(a.name));
                    }
                }
            }
        }
        Ok(None)
    }

    /// The version name of a definition's `versym` index: one of the object's own definitions, else
    /// a requirement. A program's `COPY` relocation defines the copied symbol in the program with the
    /// index of the version it needs from the provider, and the provider's own references bind to
    /// that copy by that version (index space shared by `DT_VERDEF` and `DT_VERNEED`).
    fn def_version_name(&self, index: u16) -> Result<Option<&'a [u8]>, LookupError> {
        Ok(self.def_version_name_from(index)?.map(|(n, _)| n))
    }

    /// [`Object::def_version_name`], and whether the name is one of this object's own definitions
    /// (else a requirement's).
    fn def_version_name_from(&self, index: u16) -> Result<Option<(&'a [u8], bool)>, LookupError> {
        match self.version_name(index)? {
            Some(name) => Ok(Some((name, true))),
            None if index > VER_NDX_GLOBAL => Ok(self.needed_version_name(index)?.map(|n| (n, false))),
            None => Ok(None),
        }
    }

    /// The candidates for `req` in the hash chain, as symbol indices (one error ends the walk).
    fn for_each_candidate(&self, req: &Request<'_>, mut f: impl FnMut(u32) -> Result<bool, LookupError>) -> Result<(), LookupError> {
        match self.hash {
            Hash::Gnu(g) => g.candidates(req.gnu, f)?,
            Hash::Sysv(s) => {
                for i in s.chain(req.sysv()) {
                    if f(i?)? {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    /// The definition of `req` in this object, by the rules of [`Want`].
    ///
    /// # Errors
    ///
    /// The reader's errors for the hash chain, symbol, string and version tables, and
    /// [`LookupError::UnknownVersion`] for a definition whose version index no definition gives.
    pub fn lookup(&self, req: &Request<'_>) -> Result<Option<Def>, LookupError> {
        let mut best: Option<Def> = None;
        self.for_each_candidate(req, |i| {
            let st = self.symtab.get(i)?;
            if !(exported(&st) || self.canonical_entry(i)) {
                return Ok(false);
            }
            let same = if req.plain { self.dynm.string_is(st.name, req.name)? } else { self.dynm.string(u64::from(st.name))? == req.name };
            if !same {
                return Ok(false);
            }
            let (version, hidden) = match self.versym {
                None => (VER_NDX_GLOBAL, false),
                Some(vs) => {
                    let v = vs.get(i)?;
                    (v.index, v.hidden)
                }
            };
            if version == VER_NDX_LOCAL {
                return Ok(false);
            }
            let def = Def { sym: i, st, version, hidden };
            match req.want {
                Want::Oldest => {
                    if best.is_none_or(|b| version < b.version) {
                        best = Some(def);
                    }
                    Ok(false)
                }
                Want::Default => {
                    let ok = version == VER_NDX_GLOBAL || !hidden;
                    if ok {
                        best = Some(def);
                    }
                    Ok(ok)
                }
                Want::Version(name) => {
                    // An object without `versym` defines no version (each definition reads as
                    // base, index 1), so a named version never matches there (`dlvsym`:
                    // spec/behaviour/dynlink.md, "dlvsym missing version ... also for an object
                    // without versions"). Relocations use `Reference`, which stays permissive
                    // (D-27.11).
                    let ok = version > VER_NDX_GLOBAL
                        && self.def_version_name(version)?.ok_or(LookupError::UnknownVersion { sym: i, index: version })? == name;
                    if ok {
                        best = Some(def);
                    }
                    Ok(ok)
                }
                Want::Reference(name) => {
                    let mine = |&(tab, index): &(usize, u16)| index == version && self.dynm.strtab().is_some_and(|t| t.as_ptr().addr() == tab);
                    let exact = self.versym.is_none()
                        || (version > VER_NDX_GLOBAL && req.origin.as_ref().is_some_and(mine))
                        || (version > VER_NDX_GLOBAL
                            && self.memo_name(req.memo, version, false)?.ok_or(LookupError::UnknownVersion { sym: i, index: version })? == name);
                    if exact {
                        best = Some(def);
                    } else if version == VER_NDX_GLOBAL && best.is_none() {
                        // A base definition matches, but an exact one later in the chain wins.
                        best = Some(def);
                    }
                    Ok(exact)
                }
            }
        })?;
        Ok(best)
    }
}

/// An ordered list of objects to search (indices into the namespace's object list): `first`, then
/// `second`, leaving out `exclude` (design §5).
#[derive(Clone, Copy, Debug, Default)]
pub struct Scope<'s> {
    first: &'s [usize],
    second: &'s [usize],
    exclude: Option<usize>,
}

impl<'s> Scope<'s> {
    /// The global scope alone: the main program, its dependencies in load order and the `RTLD_GLOBAL`
    /// objects in order (`RTLD_DEFAULT`, and the relocations of objects in the global scope).
    #[must_use]
    pub fn global(global: &'s [usize]) -> Self {
        Scope { first: global, second: &[], exclude: None }
    }

    /// The scope of an object loaded by `dlopen`: the global scope, then its local scope (the object's
    /// dependency tree, `local`); with `RTLD_DEEPBIND` the local scope comes first.
    #[must_use]
    pub fn group(global: &'s [usize], local: &'s [usize], deepbind: bool) -> Self {
        if deepbind { Scope { first: local, second: global, exclude: None } } else { Scope { first: global, second: local, exclude: None } }
    }

    /// `RTLD_NEXT` from `caller`: the objects after it in the global scope, or, for an object only in a
    /// local scope, the objects after it there; `None` when `caller` is in neither.
    #[must_use]
    pub fn next(global: &'s [usize], local: &'s [usize], caller: usize) -> Option<Self> {
        let after = |l: &'s [usize]| l.iter().position(|&o| o == caller).and_then(|p| l.get(p.checked_add(1)?..));
        after(global).or_else(|| after(local)).map(|first| Scope { first, second: &[], exclude: None })
    }

    /// This scope without object `obj` (a COPY relocation's lookup leaves out the program itself).
    #[must_use]
    pub fn excluding(self, obj: usize) -> Self {
        Scope { exclude: Some(obj), ..self }
    }

    /// The object indices in search order.
    pub fn iter(&self) -> impl Iterator<Item = usize> + 's {
        let ex = self.exclude;
        self.first.iter().chain(self.second.iter()).copied().filter(move |&o| Some(o) != ex)
    }
}

/// A definition and the object (index into the object list) that holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    /// The defining object.
    pub obj: usize,
    /// The definition.
    pub def: Def,
}

/// The namespace's table of `STB_GNU_UNIQUE` definitions, keyed by name and version name (empty for
/// an unversioned definition). The first definition looked up wins for the namespace; the owner of the
/// table also marks that object as not unloadable (design §5).
pub trait UniqueRegistry {
    /// The definition recorded for (`name`, `version`), if any.
    fn find(&self, name: &[u8], version: &[u8]) -> Option<Found>;
    /// Records `found` for (`name`, `version`).
    ///
    /// # Errors
    ///
    /// [`LookupError::NoMemory`] when the table cannot grow.
    fn record(&mut self, name: &[u8], version: &[u8], found: Found) -> Result<(), LookupError>;
    /// A relocation of object `by` was bound to `found` (any binding, unique or not): `dlopen` keeps
    /// the definer loaded while `by` is (phase 14 DL-17, probe `== close` C-3). Nothing by default.
    fn bound(&mut self, _by: usize, _found: &Found) {}
}

/// The registry of a load whose every lookup searches one scope in one order (the start: each
/// relocation looks in the global scope, a `COPY` without the program). The first object in that
/// order that defines a unique name is its definition for every reference, so the order itself is
/// the table: nothing is recorded, and no count of unique names can exhaust it (an object with
/// hundreds of unique definitions, as C++ libraries have, loads like any other). `dlopen` later
/// finds the start's definition the same way, as the first in the start's global scope.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScopeOrder;

impl UniqueRegistry for ScopeOrder {
    fn find(&self, _: &[u8], _: &[u8]) -> Option<Found> {
        None
    }
    fn record(&mut self, _: &[u8], _: &[u8], _: Found) -> Result<(), LookupError> {
        Ok(())
    }
}

/// The first definition of `req` in `scope` (object indices into `objects`). The first object in scope
/// order that defines the name wins, whether its definition is weak or strong. An `STB_GNU_UNIQUE`
/// definition is replaced by the one `unique` holds for its name and version, or recorded there.
///
/// # Errors
///
/// [`LookupError::NoObject`] for a scope index past `objects`, each object's [`Object::lookup`] errors,
/// and [`LookupError::NoMemory`] from the registry.
pub fn lookup(objects: &[Object<'_>], scope: Scope<'_>, req: &Request<'_>, unique: &mut dyn UniqueRegistry) -> Result<Option<Found>, LookupError> {
    for o in scope.iter() {
        let obj = objects.get(o).ok_or(LookupError::NoObject(o))?;
        let Some(def) = obj.lookup(req)? else { continue };
        let found = Found { obj: o, def };
        if def.st.bind() == STB_GNU_UNIQUE {
            let version = obj.def_version_name(def.version)?.unwrap_or(&[]);
            if let Some(first) = unique.find(req.name, version) {
                return Ok(Some(first));
            }
            unique.record(req.name, version, found)?;
        }
        return Ok(Some(found));
    }
    Ok(None)
}

/// How a reference binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// To this definition.
    Defined(Found),
    /// A weak reference without a definition: its value is 0.
    WeakUndefined,
}

/// [`lookup`] for a reference: `weak` says the reference is `STB_WEAK`, which may stay undefined.
///
/// # Errors
///
/// [`LookupError::Undefined`] for a strong reference without a definition, and the [`lookup`] errors.
pub fn resolve(objects: &[Object<'_>], scope: Scope<'_>, req: &Request<'_>, weak: bool, unique: &mut dyn UniqueRegistry) -> Result<Binding, LookupError> {
    match lookup(objects, scope, req, unique)? {
        Some(f) => Ok(Binding::Defined(f)),
        None if weak => Ok(Binding::WeakUndefined),
        None => Err(LookupError::Undefined),
    }
}
