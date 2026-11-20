//! Eager relocation of loaded objects (phase 14 DL-10; plan/ph14/design.md §6, L14-6, L14-8; D-27.17).
//!
//! Pure logic: the objects are seen through the ELF reader (their files) and their load bias; every
//! store, load, resolver call and protection change goes through the [`Image`] trait, which the
//! boundary implements on the process's memory and the tests on a model. The order (L14-8):
//!
//! 1. [`check_versions`] for every object before anything is relocated (D-27.11, ruling #3655): a
//!    versioned reference to an object without version tables stops the load.
//! 2. [`relocate_all`]: objects in reverse load order (dependencies before the objects that need
//!    them, the program last). Inside one object: RELR, then RELA, then `JMPREL`; `IRELATIVE` entries
//!    and references bound to `STT_GNU_IFUNC` definitions are deferred to a second pass over the same
//!    tables, so a resolver runs only once its object and every object after it in load order are
//!    relocated (threat rule 6). `COPY` (program only) copies from a provider relocated before.
//! 3. [`relro_ranges`]: the caller makes each object's RELRO pages read-only after all of the above.
//!
//! A text-relocated object (`DT_TEXTREL` or `DF_TEXTREL`) has its read-only segments made writable
//! and not executable for the first pass and restored before the second (never W+X, L14-6); a
//! deferred store into such a segment is refused. Relocation code matches on [`RelocKind`] only.

use crate::arch::{ApplyError, Arch, Operands, RelocKind, TlsOperands, ELFDATA2LSB};
use crate::elf::{dt, pt, Dynamic, Elf, ElfError, ProgramHeader, Rela, STB_LOCAL, STB_WEAK, STT_GNU_IFUNC, STT_TLS, STV_DEFAULT, STV_PROTECTED, SHN_UNDEF};
use crate::map::{PROT_EXEC, PROT_READ, PROT_WRITE};
use crate::symbols::{self, Binding, Found, LookupError, Object, Request, Scope, UniqueRegistry, VersionMemo};

/// `SHN_ABS` (gABI): an absolute symbol, whose value the load bias does not move.
const SHN_ABS: u16 = 0xfff1;
/// `VER_FLG_WEAK` (GNU symbol-versioning document): a version whose absence is not fatal.
const VER_FLG_WEAK: u16 = 2;
/// `PF_X`, `PF_W`, `PF_R` (ELF gABI).
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

/// One loaded object as the relocator sees it.
#[derive(Clone, Copy, Debug)]
pub struct Obj<'a> {
    /// The object's file, through the reader.
    pub elf: Elf<'a>,
    /// Its dynamic section.
    pub dynm: Dynamic<'a>,
    /// Run-time address minus link-time address.
    pub bias: u64,
    /// The name it was loaded by (its `DT_NEEDED` string, or the program's path): messages, and the
    /// `vn_file` of a version need when the object has no `DT_SONAME`.
    pub name: &'a [u8],
    /// Where its static TLS block is ([`crate::tls::layout`]); `None` when it has no `PT_TLS`.
    pub tls: Option<crate::tls::Placed>,
}

/// The objects of one load, in load order (index 0 is the program), with their lookup views and the
/// global scope (indices in lookup order).
#[derive(Clone, Copy, Debug)]
pub struct Objects<'s, 'a> {
    /// The objects.
    pub objs: &'s [Obj<'a>],
    /// `syms[i]` is the lookup view of `objs[i]`.
    pub syms: &'s [Object<'a>],
    /// The global scope.
    pub global: &'s [usize],
}

/// The process memory the relocator writes. Addresses are run-time addresses; the relocator has
/// checked each range against the object's segments before a call.
pub trait Image {
    /// Store `bytes` at `at` (inside a segment that is writable at that moment).
    fn store(&mut self, at: u64, bytes: &[u8]);
    /// Read `out.len()` bytes at `at` (inside a mapped segment).
    fn load(&mut self, at: u64, out: &mut [u8]);
    /// Copy `len` bytes from `from` (a provider's segment) to `to` (the program's writable segment).
    fn copy(&mut self, to: u64, from: u64, len: u64);
    /// Call the IFUNC resolver at `resolver` and return its result.
    fn call(&mut self, resolver: u64) -> u64;
    /// Change the protection of `len` bytes at `at` (page multiples) to `prot`.
    #[allow(clippy::result_unit_err)]
    fn protect(&mut self, at: u64, len: u64, prot: i32) -> Result<(), ()>;
    /// The run-time address of the TLS descriptor function for a block in the static area (the
    /// loader's `niji_machine_ld_tlsdesc_static`, DL-15); `None` (the default): this image has
    /// none, and a TLS descriptor relocation is refused ([`RelocError::Tls`]).
    fn tlsdesc_static(&mut self) -> Option<u64> {
        None
    }
    /// Report a warning that does not stop the load: `parts` make one line. Nothing by default.
    fn warn(&mut self, _parts: &[&[u8]]) {}
}

/// Why an object could not be relocated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocError<'a> {
    /// A relocation type the architecture's table does not handle (the raw `r_type`).
    Unsupported(u32),
    /// A TLS descriptor relocation (the raw `r_type`) in an image without a descriptor function
    /// ([`Image::tlsdesc_static`]).
    Tls(u32),
    /// A TLS relocation whose defining object has no TLS block (the field's offset).
    TlsModule(u64),
    /// A text-field relocation in an object without `DT_TEXTREL` (design §6).
    TextRel(u32),
    /// A strong reference without a definition (the symbol's name).
    Undefined(&'a [u8]),
    /// The program's canonical PLT entry for a function (the symbol's name) whose definition is
    /// protected in an object marked `GNU_PROPERTY_1_NEEDED_INDIRECT_EXTERN_ACCESS`: that object takes
    /// its own address as the function's, so the two addresses would differ.
    IndirectExternAccess(&'a [u8]),
    /// A field outside the object's writable segments, or a copy source outside its provider.
    Outside(u64),
    /// The value does not fit the field.
    Overflow(u64),
    /// A `COPY` relocation outside the program, or one whose definition is the program's own.
    Copy(u64),
    /// The symbol tables are malformed.
    Lookup(LookupError),
    /// The relocation tables are malformed.
    Elf(ElfError),
    /// A protection change for a text-relocated object failed.
    Protect,
}

/// A relocation failure and the object (index into the load) it happened in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure<'a> {
    /// The object.
    pub obj: usize,
    /// What went wrong.
    pub error: RelocError<'a>,
}

impl<'a> RelocError<'a> {
    /// The diagnostic text (after `ld.so: <object>: `), without the symbol name of
    /// [`RelocError::Undefined`] and [`RelocError::IndirectExternAccess`] (the caller prints it after
    /// this text).
    pub fn message(self) -> &'static [u8] {
        match self {
            RelocError::Unsupported(_) => b"unsupported relocation type\n",
            RelocError::Tls(_) => b"TLS descriptor relocations are not available in this build\n",
            RelocError::TlsModule(_) => b"a thread-local storage relocation refers to an object without a TLS segment\n",
            RelocError::TextRel(_) => b"relocation of a text field in an object not marked DT_TEXTREL\n",
            RelocError::Undefined(_) => b"undefined symbol: ",
            RelocError::IndirectExternAccess(_) => b"the program's PLT address of a protected function of an object that needs indirect extern access: ",
            RelocError::Outside(_) => b"relocation outside the object's writable segments\n",
            RelocError::Overflow(_) => b"relocated value does not fit its field\n",
            RelocError::Copy(_) => b"COPY relocation outside the program or without a provider\n",
            RelocError::Lookup(LookupError::NoMemory) => crate::symbols::UNIQUE_NO_MEMORY,
            RelocError::Lookup(_) => b"malformed symbol tables\n",
            RelocError::Elf(_) => b"malformed relocation tables\n",
            RelocError::Protect => b"cannot change the protection of a text-relocated segment\n",
        }
    }
}

/// The page range \[start, end) covering link-time range \[from, from + len), in run-time addresses.
fn pages(bias: u64, from: u64, len: u64, page: u64) -> (u64, u64) {
    let mask = !page.wrapping_sub(1);
    let start = bias.wrapping_add(from) & mask;
    let end = bias.wrapping_add(from).wrapping_add(len).wrapping_add(page.wrapping_sub(1)) & mask;
    (start, end)
}

/// The `mmap` protection of a segment's `p_flags`.
fn prot_of(flags: u32) -> i32 {
    let bit = |f: u32, p: i32| if flags & f != 0 { p } else { 0 };
    bit(PF_R, PROT_READ) | bit(PF_W, PROT_WRITE) | bit(PF_X, PROT_EXEC)
}

/// Whether \[at, at + len) (link-time) lies inside one `PT_LOAD`'s memory, writable (or any, when
/// `any`).
fn inside(elf: &Elf<'_>, at: u64, len: u64, any: bool) -> bool {
    let Some(end) = at.checked_add(len) else { return false };
    elf.program_headers().any(|p| {
        p.p_type == pt::LOAD && (any || p.flags & PF_W != 0) && p.vaddr <= at && p.vaddr.checked_add(p.memsz).is_some_and(|e| end <= e)
    })
}

/// The most `PT_LOAD` ranges [`Loads`] holds; an object with more checks ranges through the table.
const LOADS_MAX: usize = 8;

/// The `PT_LOAD` ranges of one object (link-time start, end, writable), read once per relocation
/// pass so that the check every entry makes ([`Loads::inside`]) does not decode the program
/// header table again (DYN-START-PARITY: that walk was a quarter of a start's instructions). A
/// segment whose end overflows is left out: [`inside`] never accepts a range in one.
struct Loads {
    seg: [(u64, u64, bool); LOADS_MAX],
    n: usize,
    /// More `PT_LOAD`s than fit: [`Loads::inside`] walks the table as [`inside`] does.
    spill: bool,
    /// The segment the last accepted range lay in: tried first, as a table's entries run through
    /// one segment in turn.
    hit: core::cell::Cell<usize>,
}

impl Loads {
    fn of(elf: &Elf<'_>) -> Self {
        let mut l = Loads { seg: [(0, 0, false); LOADS_MAX], n: 0, spill: false, hit: core::cell::Cell::new(0) };
        for p in elf.program_headers().filter(|p| p.p_type == pt::LOAD) {
            let Some(end) = p.vaddr.checked_add(p.memsz) else { continue };
            match l.seg.get_mut(l.n) {
                Some(s) => *s = (p.vaddr, end, p.flags & PF_W != 0),
                None => l.spill = true,
            }
            l.n = l.n.saturating_add(1);
        }
        l
    }

    /// The words of the segment the last accepted range lay in, `(first, last)` start addresses
    /// (link-time), when that segment is writable (or any, when `any`); an empty window (`first >
    /// last`) otherwise. A word at `at` with `first <= at <= last` lies in a `PT_LOAD` [`inside`]
    /// accepts; any other answers only "not there", and [`Loads::inside`] then decides. Returned by
    /// value, so a pass keeps it in registers across its stores (DYN-START-LINK-SHAPE: reading it
    /// through the table on every entry was most of the `RELATIVE` loop).
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn window(&self, any: bool) -> (u64, u64) {
        match self.seg.get(..self.n).and_then(|s| s.get(self.hit.get())) {
            Some(&(lo, hi, w)) if any || w => hi.checked_sub(8).map_or((1, 0), |last| (lo, last)),
            _ => (1, 0),
        }
    }

    /// [`inside`] for the object these ranges were read from.
    fn inside(&self, elf: &Elf<'_>, at: u64, len: u64, any: bool) -> bool {
        if self.spill {
            return inside(elf, at, len, any);
        }
        let Some(end) = at.checked_add(len) else { return false };
        let fits = |&(lo, hi, w): &(u64, u64, bool)| (any || w) && lo <= at && end <= hi;
        let segs = self.seg.get(..self.n).unwrap_or(&[]);
        if segs.get(self.hit.get()).is_some_and(fits) {
            return true;
        }
        match segs.iter().position(fits) {
            Some(k) => {
                self.hit.set(k);
                true
            }
            None => false,
        }
    }
}

/// Whether the object asks for text relocations.
fn textrel(d: &Dynamic<'_>) -> bool {
    d.get(dt::TEXTREL).is_some() || d.get(dt::FLAGS).is_some_and(|f| f & dt::DF_TEXTREL != 0)
}

/// What a symbol reference binds to.
#[derive(Clone, Copy, Debug)]
struct Target {
    /// S: the run-time value (the resolver's address for an IFUNC).
    s: u64,
    /// The definition is an `STT_GNU_IFUNC`: S is its resolver.
    ifunc: bool,
    /// The defining object (`None` for an undefined weak reference or symbol 0).
    obj: Option<usize>,
    /// The definition's size.
    size: u64,
}

/// The run-time value of definition `st` in object `o`.
fn value_of(set: &Objects<'_, '_>, o: usize, st: &crate::elf::Sym) -> u64 {
    // A TLS symbol's value is an offset in its module's block: the load bias does not move it.
    let bias = if st.shndx == SHN_ABS || st.kind() == STT_TLS { 0 } else { set.objs.get(o).map_or(0, |x| x.bias) };
    bias.wrapping_add(st.value)
}

/// `GNU_PROPERTY_1_NEEDED` (Linux extensions to the gABI, "Program Properties": a 32-bit property
/// whose bits are OR-ed across inputs) and its bit `GNU_PROPERTY_1_NEEDED_INDIRECT_EXTERN_ACCESS`: the
/// object reaches external symbols only indirectly, so its protected functions' addresses are its own.
const GNU_PROPERTY_1_NEEDED: u32 = 0xb000_8000;
const NEEDED_INDIRECT_EXTERN_ACCESS: u32 = 1;

/// Whether `elf` is marked `GNU_PROPERTY_1_NEEDED_INDIRECT_EXTERN_ACCESS` (no or malformed notes:
/// not marked).
pub fn needs_indirect_extern_access<A: Arch>(elf: &Elf<'_>) -> bool {
    let Some(ph) = elf.find(pt::GNU_PROPERTY) else { return false };
    let Ok(notes) = elf.notes(&ph) else { return false };
    let word = |d: &[u8]| {
        let b: [u8; 4] = d.get(..4)?.try_into().ok()?;
        Some(if A::EI_DATA == ELFDATA2LSB { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
    };
    notes
        .map_while(Result::ok)
        .flat_map(|n| n.properties().map_while(Result::ok))
        .any(|p| p.pr_type == GNU_PROPERTY_1_NEEDED && word(p.data).is_some_and(|w| w & NEEDED_INDIRECT_EXTERN_ACCESS != 0))
}

/// The program's `JUMP_SLOT` `sym` of a canonical PLT entry bound to `t`, a protected definition in
/// another object: that object's own references to the function bind its own address, not the
/// entry. Refused when the object is marked as needing indirect extern access (it was built on the
/// promise that no program takes the function's address directly); otherwise a warning, and the
/// load goes on.
#[cold]
#[inline(never)]
fn protected_canonical<'a, A: Arch, I: Image>(set: &Objects<'_, 'a>, i: usize, sym: u32, t: Target, img: &mut I) -> Result<(), RelocError<'a>> {
    let me = set.syms.get(i).ok_or(RelocError::Lookup(LookupError::NoObject(i)))?;
    let st = me.symtab().get(sym).map_err(RelocError::Elf)?;
    let name = me.dynamic().string(u64::from(st.name)).map_err(RelocError::Elf)?;
    let Some(def) = t.obj.and_then(|d| set.objs.get(d)) else { return Ok(()) };
    if needs_indirect_extern_access::<A>(&def.elf) {
        return Err(RelocError::IndirectExternAccess(name));
    }
    let prog = set.objs.get(i).map_or(&b""[..], |o| o.name);
    img.warn(&[prog, b": warning: the program's address of protected function `", name, b"' differs from its address in ", def.name, b"\n"]);
    Ok(())
}

/// The `JUMP_SLOT`s of object `i` after its plain pass `before` ([`pass1`], or the deferred loop
/// of [`relocate_deferred`] when `deferred`), when the program has canonical PLT entries
/// ([`Object::has_canonical_plt`]): the plain lookup binds a slot of such a function to the
/// program's entry, which is itself the slot's caller (gABI "Symbol Values"), so [`slot`] binds it
/// again. Only this one test is added to the plain path; the rest stays out of it. Whether any
/// entry was deferred.
#[cold]
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn canonical_slots<'a, A: Arch, I: Image>(set: &Objects<'_, 'a>, i: usize, before: Result<bool, RelocError<'a>>, deferred: bool, text: bool, img: &mut I, unique: &mut dyn UniqueRegistry) -> Result<bool, RelocError<'a>> {
    let mut any = before?;
    let o = set.objs.get(i).ok_or(RelocError::Lookup(LookupError::NoObject(i)))?;
    // Where a field can point after the plain pass to hold a canonical entry: the program's span
    // (lowest to highest `PT_LOAD` address, run time), read once here, not once per slot.
    let prog = set.objs.first().ok_or(RelocError::Lookup(LookupError::NoObject(0)))?;
    let loads = || prog.elf.program_headers().filter(|p| p.p_type == pt::LOAD);
    let lo = loads().map(|p| p.vaddr).min().unwrap_or(0);
    let hi = loads().filter_map(|p| p.vaddr.checked_add(p.memsz)).max().unwrap_or(0);
    let span = (prog.bias.wrapping_add(lo), hi.saturating_sub(lo));
    // The RELA table's `DT_RELACOUNT` prefix holds only `RELATIVE` entries: no `JUMP_SLOT` there.
    let prefix = usize::try_from(o.dynm.get(dt::RELACOUNT).unwrap_or(0)).unwrap_or(usize::MAX);
    for (table, skip) in [(o.dynm.rela::<A>(), prefix), (o.dynm.jmprel::<A>(), 0)] {
        // `Skip` reaches `Relas::nth`, which jumps over the prefix (a `flatten` would decode it).
        let Some(table) = table.map_err(RelocError::Elf)? else { continue };
        for r in table.skip(skip) {
            if r.kind == RelocKind::JumpSlot {
                any |= slot::<A, I>(set, i, o, &r, span, deferred, text, img, unique)?;
            }
        }
    }
    Ok(any)
}

/// The definition `JUMP_SLOT` symbol `sym` of object `i` binds when the plain lookup answered with
/// the program's canonical PLT entry (the only undefined symbol a lookup returns): the lookup again
/// without the program. `None` when the plain answer stands. With the target, whether it is the
/// protected definition, in another object, of the program's own canonical entry
/// ([`protected_canonical`]).
fn slot_target<'a>(set: &Objects<'_, 'a>, i: usize, sym: u32, unique: &mut dyn UniqueRegistry) -> Result<Option<(Target, bool)>, RelocError<'a>> {
    let me = set.syms.get(i).ok_or(RelocError::Lookup(LookupError::NoObject(i)))?;
    let st = me.symtab().get(sym).map_err(RelocError::Elf)?;
    if sym == 0 || (st.shndx != SHN_UNDEF && (st.bind() == STB_LOCAL || st.visibility() != STV_DEFAULT)) {
        return Ok(None);
    }
    let name = me.dynamic().string(u64::from(st.name)).map_err(RelocError::Elf)?;
    let req = Request::new(name, me.reference_want(sym).map_err(RelocError::Lookup)?);
    let mut scope = Scope::global(set.global);
    let mut again = false;
    loop {
        match symbols::resolve(set.syms, scope, &req, st.bind() == STB_WEAK, unique) {
            Ok(Binding::Defined(Found { obj, def })) if !again && def.st.shndx == SHN_UNDEF => {
                scope = scope.excluding(obj);
                again = true;
            }
            _ if !again => return Ok(None),
            Ok(Binding::Defined(f @ Found { obj, def })) => {
                unique.bound(i, &f);
                let protected = obj != i && def.st.visibility() == STV_PROTECTED && me.has_canonical_plt() && symbols::canonical_plt(&st);
                return Ok(Some((Target { s: value_of(set, obj, &def.st), ifunc: def.st.kind() == STT_GNU_IFUNC, obj: Some(obj), size: def.st.size }, protected)));
            }
            Ok(Binding::WeakUndefined) => return Ok(Some((Target { s: 0, ifunc: false, obj: None, size: 0 }, false))),
            Err(LookupError::Undefined) => return Err(RelocError::Undefined(name)),
            Err(e) => return Err(RelocError::Lookup(e)),
        }
    }
}

/// `JUMP_SLOT` `r` of object `i` in pass `deferred` after the plain pass: only a field that points
/// into the program can hold a canonical entry, so any other is left as written; else bound by
/// [`slot_target`] and written (or the IFUNC entry deferred) as [`one`] does for its symbol entries.
#[allow(clippy::too_many_arguments)]
fn slot<'a, A: Arch, I: Image>(set: &Objects<'_, 'a>, i: usize, o: &Obj<'a>, r: &Rela, span: (u64, u64), deferred: bool, text: bool, img: &mut I, unique: &mut dyn UniqueRegistry) -> Result<bool, RelocError<'a>> {
    let p = o.bias.wrapping_add(r.offset);
    let mut w = [0u8; 8];
    img.load(p, &mut w);
    let v = if A::EI_DATA == ELFDATA2LSB { u64::from_le_bytes(w) } else { u64::from_be_bytes(w) };
    if v.wrapping_sub(span.0) >= span.1 {
        return Ok(false);
    }
    let Some((t, protected)) = slot_target(set, i, r.sym, unique)? else { return Ok(false) };
    if protected && !deferred {
        protected_canonical::<A, I>(set, i, r.sym, t, img)?;
    }
    let put = |img: &mut I, s: u64, any: bool| -> Result<(), RelocError<'a>> {
        let f = match A::apply(r.kind, Operands { s, a: r.addend, b: o.bias, p }) {
            Ok(f) => f,
            Err(ApplyError::Overflow) => return Err(RelocError::Overflow(r.offset)),
            Err(ApplyError::NoArithmetic) => return Err(RelocError::Unsupported(r.r_type)),
        };
        let Some(f) = f else { return Ok(()) };
        let w = usize::from(f.width);
        if !inside(&o.elf, r.offset, u64::from(f.width), any) {
            return Err(RelocError::Outside(r.offset));
        }
        let (le, be) = (f.value.to_le_bytes(), f.value.to_be_bytes());
        let bytes = if A::EI_DATA == ELFDATA2LSB { le.get(..w) } else { be.get(8usize.saturating_sub(w)..) };
        img.store(p, bytes.ok_or(RelocError::Overflow(r.offset))?);
        Ok(())
    };
    match (t.ifunc, deferred) {
        (false, false) => put(img, t.s, text).map(|()| false),
        (false, true) => Ok(false),
        (true, false) => inside(&o.elf, r.offset, 8, false).then_some(true).ok_or(RelocError::Outside(r.offset)),
        (true, true) => {
            let s = img.call(t.s);
            put(img, s, false).map(|()| true)
        }
    }
}

/// Resolve symbol `sym` of object `i`. A defined local, or a defined symbol whose visibility is not
/// default, binds in its own object; every other reference is looked up in the global scope (without
/// the program for a `COPY`).
fn target<'a>(set: &Objects<'_, 'a>, i: usize, sym: u32, copy: bool, unique: &mut dyn UniqueRegistry, memo: &VersionMemo) -> Result<Target, RelocError<'a>> {
    if sym == 0 {
        return Ok(Target { s: 0, ifunc: false, obj: None, size: 0 });
    }
    let me = set.syms.get(i).ok_or(RelocError::Lookup(LookupError::NoObject(i)))?;
    let st = me.symtab().get(sym).map_err(RelocError::Elf)?;
    if !copy && st.shndx != SHN_UNDEF && (st.bind() == STB_LOCAL || st.visibility() != STV_DEFAULT) {
        return Ok(Target { s: value_of(set, i, &st), ifunc: st.kind() == STT_GNU_IFUNC, obj: Some(i), size: st.size });
    }
    let dynm = set.objs.get(i).map(|o| o.dynm).ok_or(RelocError::Lookup(LookupError::NoObject(i)))?;
    let name = dynm.string(u64::from(st.name)).map_err(RelocError::Elf)?;
    let want = me.reference_want_with(sym, Some(memo)).map_err(RelocError::Lookup)?;
    let mut scope = Scope::global(set.global);
    if copy {
        scope = scope.excluding(0);
    }
    match symbols::resolve(set.syms, scope, &Request::new(name, want).with_memo(memo), st.bind() == STB_WEAK, unique) {
        Ok(Binding::Defined(f @ Found { obj, def })) => {
            unique.bound(i, &f);
            Ok(Target { s: value_of(set, obj, &def.st), ifunc: def.st.kind() == STT_GNU_IFUNC, obj: Some(obj), size: def.st.size })
        }
        Ok(Binding::WeakUndefined) => Ok(Target { s: 0, ifunc: false, obj: None, size: 0 }),
        Err(LookupError::Undefined) => Err(RelocError::Undefined(name)),
        Err(e) => Err(RelocError::Lookup(e)),
    }
}

/// The field `r` (of an object loaded at `bias`) gives for symbol value `s`, as type `kind`.
// Inlined into each pass: one call per relocation otherwise.
#[allow(clippy::inline_always)]
#[inline(always)]
fn field<'a, A: Arch>(kind: RelocKind, s: u64, bias: u64, r: &Rela) -> Result<Option<crate::arch::Field>, RelocError<'a>> {
    match A::apply(kind, Operands { s, a: r.addend, b: bias, p: bias.wrapping_add(r.offset) }) {
        Ok(f) => Ok(f),
        Err(ApplyError::Overflow) => Err(RelocError::Overflow(r.offset)),
        Err(ApplyError::NoArithmetic) => Err(RelocError::Unsupported(r.r_type)),
    }
}

/// Stores field `f` at `r`'s place in object `o`, which must lie in one of its writable segments
/// (any segment when `any`). A whole word is stored as one fixed-size write.
#[allow(clippy::inline_always)]
#[inline(always)]
fn put<'a, A: Arch, I: Image>(img: &mut I, loads: &Loads, o: &Obj<'a>, r: &Rela, f: Option<crate::arch::Field>, any: bool) -> Result<(), RelocError<'a>> {
    let Some(f) = f else { return Ok(()) };
    if !loads.inside(&o.elf, r.offset, u64::from(f.width), any) {
        return Err(RelocError::Outside(r.offset));
    }
    let p = o.bias.wrapping_add(r.offset);
    let (le, be) = (f.value.to_le_bytes(), f.value.to_be_bytes());
    let word = if A::EI_DATA == ELFDATA2LSB { le } else { be };
    let w = usize::from(f.width);
    if w == 8 {
        img.store(p, &word);
        return Ok(());
    }
    let bytes = if A::EI_DATA == ELFDATA2LSB { le.get(..w) } else { be.get(8usize.saturating_sub(w)..) };
    img.store(p, bytes.ok_or(RelocError::Overflow(r.offset))?);
    Ok(())
}

/// One relocation of object `i` in pass `deferred` (false: everything but the deferred entries;
/// true: only them). Returns whether the entry is deferred (pass 1) or was applied (pass 2).
#[allow(clippy::too_many_arguments)]
fn one<'a, A: Arch, I: Image>(set: &Objects<'_, 'a>, i: usize, loads: &Loads, r: &Rela, deferred: bool, text: bool, img: &mut I, unique: &mut dyn UniqueRegistry, memo: &VersionMemo) -> Result<bool, RelocError<'a>> {
    let o = set.objs.get(i).ok_or(RelocError::Lookup(LookupError::NoObject(i)))?;
    let p = o.bias.wrapping_add(r.offset);
    let field = |kind, s| field::<A>(kind, s, o.bias, r);
    // Text fields may be written in pass 1 only (the segment is writable then).
    let put = |img: &mut I, f: Option<crate::arch::Field>, any: bool| put::<A, I>(img, loads, o, r, f, any);
    match r.kind {
        RelocKind::None => Ok(false),
        RelocKind::Relative if !deferred => put(img, field(r.kind, 0)?, text).map(|()| false),
        RelocKind::Relative => Ok(false),
        // Pass 1 checks the field a deferred entry will write, so pass 2 meets no new error.
        RelocKind::Irelative if !deferred => loads.inside(&o.elf, r.offset, 8, false).then_some(true).ok_or(RelocError::Outside(r.offset)),
        RelocKind::Irelative => {
            let resolver = field(r.kind, 0)?.map_or(0, |f| f.value);
            let v = img.call(resolver);
            put(img, Some(crate::arch::Field { value: v, width: 8 }), false).map(|()| true)
        }
        RelocKind::Abs64 | RelocKind::GlobDat | RelocKind::JumpSlot | RelocKind::TextRel { .. } => {
            if matches!(r.kind, RelocKind::TextRel { .. }) && !text {
                return Err(RelocError::TextRel(r.r_type));
            }
            let t = target(set, i, r.sym, false, unique, memo)?;
            match (t.ifunc, deferred) {
                (false, false) => put(img, field(r.kind, t.s)?, text).map(|()| false),
                (false, true) => Ok(false),
                (true, false) => loads.inside(&o.elf, r.offset, 8, false).then_some(true).ok_or(RelocError::Outside(r.offset)),
                (true, true) => {
                    let s = img.call(t.s);
                    put(img, field(r.kind, s)?, false).map(|()| true)
                }
            }
        }
        RelocKind::Copy if deferred => Ok(false),
        RelocKind::Copy => {
            let me = set.syms.get(i).ok_or(RelocError::Lookup(LookupError::NoObject(i)))?;
            let size = me.symtab().get(r.sym).map_err(RelocError::Elf)?.size;
            let t = target(set, i, r.sym, true, unique, memo)?;
            let src = t.obj.filter(|&d| i == 0 && d != 0).and_then(|d| set.objs.get(d)).ok_or(RelocError::Copy(r.offset))?;
            // The program's size; a smaller definition copies only what it has.
            let len = size.min(t.size);
            if !inside(&src.elf, t.s.wrapping_sub(src.bias), len, true) || !loads.inside(&o.elf, r.offset, size, false) {
                return Err(RelocError::Outside(r.offset));
            }
            img.copy(p, t.s, len);
            Ok(false)
        }
        RelocKind::DtpMod | RelocKind::DtpOff | RelocKind::TpOff | RelocKind::TlsDesc if deferred => Ok(false),
        RelocKind::DtpMod | RelocKind::DtpOff | RelocKind::TpOff | RelocKind::TlsDesc => {
            // No symbol: the object's own module (local-dynamic and initial-exec on a local symbol).
            let (s, def) = if r.sym == 0 {
                (0, Some(i))
            } else {
                let t = target(set, i, r.sym, false, unique, memo)?;
                (t.s, t.obj)
            };
            let placed = def.and_then(|d| set.objs.get(d)).and_then(|d| d.tls).ok_or(RelocError::TlsModule(r.offset))?;
            let ops = TlsOperands { s, a: r.addend, module: placed.module, tp_offset: placed.tp_offset };
            if r.kind != RelocKind::TlsDesc {
                let f = A::apply_tls(r.kind, ops).map_err(|_| RelocError::Unsupported(r.r_type))?;
                return put(img, Some(f), text).map(|()| false);
            }
            // A descriptor (DL-15): every initially loaded module's block is in the static area, so
            // its function is the image's static one and its argument the TP offset.
            let resolver = img.tlsdesc_static().ok_or(RelocError::Tls(r.r_type))?;
            let words = A::tls_descriptor(resolver, ops).map_err(|_| RelocError::Unsupported(r.r_type))?;
            if !loads.inside(&o.elf, r.offset, 16, text) {
                return Err(RelocError::Outside(r.offset));
            }
            let mut bytes = [0u8; 16];
            for (b, w) in bytes.chunks_exact_mut(8).zip(words) {
                b.copy_from_slice(&if A::EI_DATA == ELFDATA2LSB { w.to_le_bytes() } else { w.to_be_bytes() });
            }
            img.store(p, &bytes);
            Ok(false)
        }
        RelocKind::Unsupported(n) => Err(RelocError::Unsupported(n)),
    }
}

/// Make the non-writable `PT_LOAD`s of a text-relocated object writable and not executable (`open`)
/// or give them back their own protection.
fn text_segments<I: Image>(o: &Obj<'_>, img: &mut I, page: u64, open: bool) -> Result<(), ()> {
    for ph in o.elf.program_headers().filter(|p: &ProgramHeader| p.p_type == pt::LOAD && p.memsz != 0 && p.flags & PF_W == 0) {
        let (start, end) = pages(o.bias, ph.vaddr, ph.memsz, page);
        let prot = if open { PROT_READ | PROT_WRITE } else { prot_of(ph.flags) };
        img.protect(start, end.wrapping_sub(start), prot)?;
    }
    Ok(())
}

/// Relocate object `i` of the load `set` (design §6 order inside one object). `page` is the page size.
///
/// # Errors
///
/// The first [`RelocError`] met.
pub fn relocate<'a, A: Arch, I: Image>(set: &Objects<'_, 'a>, i: usize, img: &mut I, page: u64, unique: &mut dyn UniqueRegistry) -> Result<(), RelocError<'a>> {
    if relocate_first::<A, I>(set, i, img, page, unique)? {
        relocate_deferred::<A, I>(set, i, img, unique)?;
    }
    Ok(())
}

/// Pass 1 of object `i` (DL-16a two-pass relocation, plan/ph14/dl-16-dlopen.md §2): every symbol
/// reference resolved, every field outside the IFUNC entries written, and each IFUNC entry's field
/// checked; no foreign code runs. Whether any entry was deferred to [`relocate_deferred`].
///
/// # Errors
///
/// The first [`RelocError`] met.
pub fn relocate_first<'a, A: Arch, I: Image>(set: &Objects<'_, 'a>, i: usize, img: &mut I, page: u64, unique: &mut dyn UniqueRegistry) -> Result<bool, RelocError<'a>> {
    let o = *set.objs.get(i).ok_or(RelocError::Lookup(LookupError::NoObject(i)))?;
    let text = textrel(&o.dynm);
    if text {
        text_segments(&o, img, page, true).map_err(|()| RelocError::Protect)?;
    }
    let first = pass1::<A, I>(set, i, &o, text, img, unique);
    let first = if set.syms.first().is_some_and(Object::has_canonical_plt) { canonical_slots::<A, I>(set, i, first, false, text, img, unique) } else { first };
    if text {
        // Restored before any resolver can run, also on failure.
        text_segments(&o, img, page, false).map_err(|()| RelocError::Protect)?;
    }
    first
}

/// Pass 2 of object `i`: the IFUNC entries [`relocate_first`] deferred, each calling its resolver.
/// After a successful pass 1 of the same set it meets no new error (pass 1 resolved the same
/// symbols and checked the same fields).
///
/// # Errors
///
/// The first [`RelocError`] met.
pub fn relocate_deferred<'a, A: Arch, I: Image>(set: &Objects<'_, 'a>, i: usize, img: &mut I, unique: &mut dyn UniqueRegistry) -> Result<(), RelocError<'a>> {
    let o = *set.objs.get(i).ok_or(RelocError::Lookup(LookupError::NoObject(i)))?;
    let text = textrel(&o.dynm);
    let loads = Loads::of(&o.elf);
    let memo = VersionMemo::default();
    for table in [o.dynm.rela::<A>(), o.dynm.jmprel::<A>()] {
        for r in table.map_err(RelocError::Elf)?.into_iter().flatten() {
            one::<A, I>(set, i, &loads, &r, true, text, img, unique, &memo)?;
        }
    }
    if set.syms.first().is_some_and(Object::has_canonical_plt) {
        canonical_slots::<A, I>(set, i, Ok(false), true, text, img, unique)?;
    }
    Ok(())
}

/// One `RELATIVE` entry of the RELA table's `DT_RELACOUNT` prefix ([`Relas::relative_prefix`]):
/// B + A (the psABI's calculation, as the RELR words above), stored at `at` in object `o`, which must
/// lie in a writable segment (any segment when `any`). `window` is the caller's copy of
/// [`Loads::window`]: a word inside it is accepted at once; on a miss the general check decides and
/// `window` follows the segment it found.
#[allow(clippy::inline_always)]
#[inline(always)]
fn relative<'a, A: Arch, I: Image>(img: &mut I, loads: &Loads, o: &Obj<'a>, window: &mut (u64, u64), at: u64, addend: i64, any: bool) -> Result<(), RelocError<'a>> {
    if !(window.0 <= at && at <= window.1) {
        if !loads.inside(&o.elf, at, 8, any) {
            return Err(RelocError::Outside(at));
        }
        *window = loads.window(any);
    }
    let v = o.bias.wrapping_add(u64::from_ne_bytes(addend.to_ne_bytes()));
    img.store(o.bias.wrapping_add(at), &if A::EI_DATA == ELFDATA2LSB { v.to_le_bytes() } else { v.to_be_bytes() });
    Ok(())
}

/// Pass 1 of object `o` (index `i`): RELR, RELA, `JMPREL`. Whether any entry was deferred. Kept
/// out of [`relocate_first`], which then stays small.
#[inline(never)]
fn pass1<'a, A: Arch, I: Image>(set: &Objects<'_, 'a>, i: usize, o: &Obj<'a>, text: bool, img: &mut I, unique: &mut dyn UniqueRegistry) -> Result<bool, RelocError<'a>> {
    let loads = Loads::of(&o.elf);
    for at in o.dynm.relr().map_err(RelocError::Elf)?.into_iter().flatten() {
        let at = at.map_err(RelocError::Elf)?;
        if !loads.inside(&o.elf, at, 8, text) {
            return Err(RelocError::Outside(at));
        }
        let p = o.bias.wrapping_add(at);
        let mut w = [0u8; 8];
        img.load(p, &mut w);
        let v = if A::EI_DATA == ELFDATA2LSB { u64::from_le_bytes(w) } else { u64::from_be_bytes(w) }.wrapping_add(o.bias);
        img.store(p, &if A::EI_DATA == ELFDATA2LSB { v.to_le_bytes() } else { v.to_be_bytes() });
    }
    let mut deferred = false;
    let memo = VersionMemo::default();
    // The RELA table's `DT_RELACOUNT` prefix: `RELATIVE` entries, most of a shared object's.
    let prefix = usize::try_from(o.dynm.get(dt::RELACOUNT).unwrap_or(0)).unwrap_or(usize::MAX);
    for (k, table) in [o.dynm.rela::<A>(), o.dynm.jmprel::<A>()].into_iter().enumerate() {
        // A plain loop over the entries: flattening the `Option` called out of line per entry.
        let Some(mut rs) = table.map_err(RelocError::Elf)? else { continue };
        if k == 0 && prefix != 0 {
            let mut window = loads.window(text);
            rs.relative_prefix(prefix, |at, a| relative::<A, I>(img, &loads, o, &mut window, at, a, text))?;
        }
        for r in rs {
            // B + A, most of a shared object's entries: written here as `one` would, without its
            // object lookup and dispatch.
            if r.kind == RelocKind::Relative {
                put::<A, I>(img, &loads, o, &r, field::<A>(r.kind, 0, o.bias, &r)?, text)?;
                continue;
            }
            deferred |= one::<A, I>(set, i, &loads, &r, false, text, img, unique, &memo)?;
        }
    }
    Ok(deferred)
}

/// Relocate every object of `set`, in reverse load order (the program last; L14-8).
///
/// # Errors
///
/// The first failure, with the object it happened in.
pub fn relocate_all<'a, A: Arch, I: Image>(set: &Objects<'_, 'a>, img: &mut I, page: u64, unique: &mut dyn UniqueRegistry) -> Result<(), Failure<'a>> {
    for i in (0..set.objs.len()).rev() {
        relocate::<A, I>(set, i, img, page, unique).map_err(|error| Failure { obj: i, error })?;
    }
    Ok(())
}

/// The RELRO pages of object `o` at run time (start, length), whole pages inside `PT_GNU_RELRO`.
pub fn relro_range(o: &Obj<'_>, page: u64) -> Option<(u64, u64)> {
    let r = o.elf.find(pt::GNU_RELRO)?;
    let mask = !page.wrapping_sub(1);
    let start = o.bias.wrapping_add(r.vaddr) & mask;
    let end = o.bias.wrapping_add(r.vaddr).wrapping_add(r.memsz) & mask;
    (end > start).then(|| (start, end.wrapping_sub(start)))
}

/// Write `o`'s loaded dynamic array's [`crate::elf::RUNTIME_ADDRESS_TAGS`] entries as run-time
/// addresses: each such entry of the reader's view `o.dynm` (link-time values, read from the file or
/// from a copy taken before this) plus the load bias, so a second call stores the same words. Nothing
/// when the array is not in a writable segment ([`Elf::published_dynamic`]). After the object is
/// relocated, before its RELRO pages are protected (DL-DYNAMIC-RELOC).
pub fn publish_dynamic<I: Image>(o: &Obj<'_>, img: &mut I) {
    let Some(d) = o.elf.published_dynamic() else { return };
    let mut at = o.bias.wrapping_add(d.vaddr).wrapping_add(8);
    for e in o.dynm.entries() {
        if crate::elf::RUNTIME_ADDRESS_TAGS.contains(&e.tag) {
            img.store(at, &e.val.wrapping_add(o.bias).to_ne_bytes());
        }
        at = at.wrapping_add(16);
    }
}

/// The run-time address of the value word of `o`'s first dynamic entry tagged `tag` (`DT_DEBUG`
/// of the program, DL-23), when its loaded dynamic array is in a writable segment
/// ([`Elf::published_dynamic`], as [`publish_dynamic`] writes it); `None` otherwise.
#[must_use]
pub fn dynamic_word(o: &Obj<'_>, tag: u64) -> Option<u64> {
    let d = o.elf.published_dynamic()?;
    let k = o.dynm.entries().position(|e| e.tag == tag)?;
    let off = u64::try_from(k).ok()?.checked_mul(16)?.checked_add(8)?;
    Some(o.bias.wrapping_add(d.vaddr).wrapping_add(off))
}

/// Make every object's RELRO pages read-only (after [`relocate_all`]; L14-8).
///
/// # Errors
///
/// The index of the object whose protection change failed.
///
/// Segment by segment (D-27.25): [`relro_range`] meets each `PT_LOAD`'s pages in one piece
/// (`niji_rt::start::relro_piece`), so a `PT_GNU_RELRO` that spans two writable segments never
/// changes the reservation pages between them; one segment gives the one range as before.
pub fn protect_relro<I: Image>(objs: &[Obj<'_>], img: &mut I, page: u64) -> Result<(), usize> {
    let word = |v: u64| usize::try_from(v).unwrap_or(usize::MAX);
    for (i, o) in objs.iter().enumerate() {
        let Some((at, len)) = relro_range(o, page) else { continue };
        let (from, to) = (word(at), word(at.wrapping_add(len)));
        for s in o.elf.program_headers().filter(|s| s.p_type == pt::LOAD) {
            let piece = niji_rt::start::relro_piece(from, to, word(o.bias.wrapping_add(s.vaddr)), word(s.memsz), word(page));
            if let Some((a, n)) = piece {
                let (a, n) = (u64::try_from(a).map_err(|_| i)?, u64::try_from(n).map_err(|_| i)?);
                img.protect(a, n, PROT_READ).map_err(|()| i)?;
            }
        }
    }
    Ok(())
}

/// A failed load-time version check (D-27.11, ruling #3655).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionError<'a> {
    /// Object `by` needs `version` of `file`, and the loaded object for `file` has no version
    /// definitions at all.
    NoVersions {
        /// The needing object.
        by: usize,
        /// The needed file (`vn_file`).
        file: &'a [u8],
        /// The version name.
        version: &'a [u8],
    },
    /// Object `by` needs `version` of `file`, which the loaded object does not define.
    Missing {
        /// The needing object.
        by: usize,
        /// The needed file (`vn_file`).
        file: &'a [u8],
        /// The version name.
        version: &'a [u8],
    },
    /// The version tables of object `.0` are malformed.
    Elf(usize, ElfError),
}

/// The object of the load that `file` (a `vn_file`) names: its `DT_SONAME`, or the name it was
/// loaded by.
fn provider(objs: &[Obj<'_>], file: &[u8], live: &impl Fn(usize) -> bool) -> Option<usize> {
    objs.iter().enumerate().position(|(j, o)| live(j) && (o.dynm.get(dt::SONAME).and_then(|s| o.dynm.string(s).ok()) == Some(file) || o.name == file))
}

/// Check every `DT_VERNEED` entry of every object against the named object's `DT_VERDEF` (D-27.11):
/// a needed version (not `VER_FLG_WEAK`) of an object without version definitions, or one it does not
/// define, stops the load. A `vn_file` that names no loaded object is not checked here (the load of
/// that dependency has failed already).
///
/// # Errors
///
/// The first [`VersionError`], objects in load order.
pub fn check_versions<'a>(objs: &[Obj<'a>]) -> Result<(), VersionError<'a>> {
    check_versions_live(objs, |_| true)
}

/// [`check_versions`] over the objects `live` accepts (an unloaded object's slot is never read,
/// DL-17): as checkers and as providers.
///
/// # Errors
///
/// As [`check_versions`].
pub fn check_versions_live<'a>(objs: &[Obj<'a>], live: impl Fn(usize) -> bool) -> Result<(), VersionError<'a>> {
    for (by, o) in objs.iter().enumerate() {
        if !live(by) {
            continue;
        }
        let Some(needs) = o.dynm.verneed().map_err(|e| VersionError::Elf(by, e))? else { continue };
        for vn in needs {
            let vn = vn.map_err(|e| VersionError::Elf(by, e))?;
            let Some(p) = provider(objs, vn.file, &live).and_then(|j| objs.get(j)) else { continue };
            let defs = p.dynm.verdef().map_err(|e| VersionError::Elf(by, e))?;
            for aux in vn.auxes() {
                let aux = aux.map_err(|e| VersionError::Elf(by, e))?;
                if aux.flags & VER_FLG_WEAK != 0 {
                    continue;
                }
                let Some(defs) = defs.clone() else {
                    return Err(VersionError::NoVersions { by, file: vn.file, version: aux.name });
                };
                let mut found = false;
                for d in defs {
                    let d = d.map_err(|e| VersionError::Elf(by, e))?;
                    if d.names().next().and_then(Result::ok).is_some_and(|n| n.name == aux.name) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(VersionError::Missing { by, file: vn.file, version: aux.name });
                }
            }
        }
    }
    Ok(())
}

/// The `PT_GNU_PROPERTY` properties of `elf` this architecture knows (L14-12: parsed, then ignored by
/// the loader): the count of known property entries. Malformed notes end the walk (they are ignored
/// like the properties themselves).
pub fn known_properties<A: Arch>(elf: &Elf<'_>) -> usize {
    let Some(ph) = elf.find(pt::GNU_PROPERTY) else { return 0 };
    let Ok(notes) = elf.notes(&ph) else { return 0 };
    notes.map_while(Result::ok).flat_map(|n| n.properties().map_while(Result::ok)).filter(|p| A::PROPERTIES.iter().any(|&(t, _)| t == p.pr_type)).count()
}

/// A fixed-size [`UniqueRegistry`] for a load without an allocator: up to `N` (name, version) keys
/// stored in a `B`-byte arena. A full table answers [`LookupError::NoMemory`].
#[derive(Debug)]
pub struct UniqueTable<const N: usize, const B: usize> {
    keys: [(usize, usize, usize); N],
    found: [Option<Found>; N],
    n: usize,
    arena: [u8; B],
    used: usize,
}

impl<const N: usize, const B: usize> Default for UniqueTable<N, B> {
    fn default() -> Self {
        UniqueTable { keys: [(0, 0, 0); N], found: [None; N], n: 0, arena: [0; B], used: 0 }
    }
}

impl<const N: usize, const B: usize> UniqueTable<N, B> {
    /// Entries recorded so far: a mark that [`UniqueTable::truncate`] rolls back to.
    #[must_use]
    pub fn len(&self) -> usize {
        self.n
    }

    /// Whether nothing is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Forget the entries recorded after mark `n` (a failed `dlopen` leaves no trace, plan §3).
    pub fn truncate(&mut self, n: usize) {
        if n < self.n {
            self.used = self.keys.get(n).map_or(0, |k| k.0);
            self.n = n;
        }
    }

    /// The defining objects of the entries recorded after mark `from` (each is pinned: design §5).
    pub fn owners(&self, from: usize) -> impl Iterator<Item = usize> + '_ {
        self.found.get(from..self.n).unwrap_or(&[]).iter().filter_map(|f| f.map(|f| f.obj))
    }
}

impl<const N: usize, const B: usize> UniqueRegistry for UniqueTable<N, B> {
    fn find(&self, name: &[u8], version: &[u8]) -> Option<Found> {
        let key = |&(at, nl, vl): &(usize, usize, usize)| {
            let n = self.arena.get(at..at.checked_add(nl)?)?;
            let v = self.arena.get(at.checked_add(nl)?..at.checked_add(nl)?.checked_add(vl)?)?;
            Some((n, v))
        };
        self.keys.iter().zip(&self.found).take(self.n).find(|(k, _)| key(k) == Some((name, version))).and_then(|(_, f)| *f)
    }

    fn record(&mut self, name: &[u8], version: &[u8], found: Found) -> Result<(), LookupError> {
        let at = self.used;
        let end = at.checked_add(name.len()).and_then(|e| e.checked_add(version.len())).ok_or(LookupError::NoMemory)?;
        let mid = at.checked_add(name.len()).ok_or(LookupError::NoMemory)?;
        let slot = self.keys.get_mut(self.n).ok_or(LookupError::NoMemory)?;
        self.arena.get_mut(at..mid).ok_or(LookupError::NoMemory)?.copy_from_slice(name);
        self.arena.get_mut(mid..end).ok_or(LookupError::NoMemory)?.copy_from_slice(version);
        *slot = (at, name.len(), version.len());
        if let Some(s) = self.found.get_mut(self.n) {
            *s = Some(found);
        }
        self.n = self.n.wrapping_add(1);
        self.used = end;
        Ok(())
    }
}

/// An array a [`GrowTable`] keeps its entries or bytes in: it only grows at the end (and only
/// fallibly) or is cut back. The loader backs it with its own mappings; tests with a `Vec`.
pub trait Stack<T: Copy> {
    /// The items pushed so far.
    fn items(&self) -> &[T];
    /// Appends `more`; `false` (and nothing appended) when there is no memory for them.
    fn try_extend(&mut self, more: &[T]) -> bool;
    /// Keeps the first `n` items.
    fn truncate(&mut self, n: usize);
}

impl<T: Copy> Stack<T> for alloc::vec::Vec<T> {
    fn items(&self) -> &[T] {
        self
    }
    fn try_extend(&mut self, more: &[T]) -> bool {
        crate::ldconfig::extend(self, more).is_ok()
    }
    fn truncate(&mut self, n: usize) {
        alloc::vec::Vec::truncate(self, n);
    }
}

/// One [`GrowTable`] entry: the key's place in the byte stack and the definition bound for it.
#[derive(Clone, Copy, Debug)]
pub struct UniqueEntry {
    hash: u32,
    at: usize,
    name_len: usize,
    version_len: usize,
    found: Found,
}

/// A [`UniqueRegistry`] with no limit but memory: (name, version) keys in a byte stack `B`, entries in
/// `E`. A key's bytes are pushed before its entry, so an entry always names bytes that exist; a
/// stack that cannot grow answers [`LookupError::NoMemory`] and the table is left as it was.
#[derive(Debug, Default)]
pub struct GrowTable<E, B> {
    entries: E,
    bytes: B,
}

impl<E: Stack<UniqueEntry>, B: Stack<u8>> GrowTable<E, B> {
    /// A table over empty stacks `entries` and `bytes`.
    #[must_use]
    pub const fn new(entries: E, bytes: B) -> Self {
        GrowTable { entries, bytes }
    }

    /// Entries recorded so far: a mark that [`GrowTable::truncate`] rolls back to.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.items().len()
    }

    /// Whether nothing is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.items().is_empty()
    }

    /// Forget the entries recorded after mark `n` (a failed `dlopen` leaves no trace, plan §3).
    pub fn truncate(&mut self, n: usize) {
        if let Some(e) = self.entries.items().get(n) {
            let at = e.at;
            self.entries.truncate(n);
            self.bytes.truncate(at);
        }
    }

    /// The defining objects of the entries recorded after mark `from` (each is pinned: design §5).
    pub fn owners(&self, from: usize) -> impl Iterator<Item = usize> + '_ {
        self.entries.items().get(from..).unwrap_or(&[]).iter().map(|e| e.found.obj)
    }
}

impl<E: Stack<UniqueEntry>, B: Stack<u8>> UniqueRegistry for GrowTable<E, B> {
    fn find(&self, name: &[u8], version: &[u8]) -> Option<Found> {
        let hash = symbols::gnu_hash(name);
        let bytes = self.bytes.items();
        let key = |e: &UniqueEntry| -> Option<(&[u8], &[u8])> {
            let mid = e.at.checked_add(e.name_len)?;
            Some((bytes.get(e.at..mid)?, bytes.get(mid..mid.checked_add(e.version_len)?)?))
        };
        self.entries.items().iter().find(|e| e.hash == hash && e.name_len == name.len() && key(e) == Some((name, version))).map(|e| e.found)
    }

    fn record(&mut self, name: &[u8], version: &[u8], found: Found) -> Result<(), LookupError> {
        let at = self.bytes.items().len();
        if !self.bytes.try_extend(name) || !self.bytes.try_extend(version) {
            self.bytes.truncate(at);
            return Err(LookupError::NoMemory);
        }
        let e = UniqueEntry { hash: symbols::gnu_hash(name), at, name_len: name.len(), version_len: version.len(), found };
        if !self.entries.try_extend(&[e]) {
            self.bytes.truncate(at);
            return Err(LookupError::NoMemory);
        }
        Ok(())
    }
}
