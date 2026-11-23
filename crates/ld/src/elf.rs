//! ELF64 reader (IP-24, plan/ph14/design.md §2). Part 1 (DL-2): the ELF header, the program headers, the
//! dynamic section and notes. Part 2 (DL-3): the GNU and SysV hash tables (structure only; symbol lookup
//! is DL-9), the symbol version tables (`versym`, `verdef`, `verneed`) and the RELA and RELR iterators.
//!
//! The reader works on a byte slice (a file prefix, a whole file or a mapped image) and does no I/O.
//! Every offset, count and size from the object is checked with checked arithmetic against the slice
//! before use (threat-model rule 4): a malformed object gives an [`ElfError`], never a panic and never a
//! read outside the slice. Nothing here allocates. The error rules are decisions D-27.7 (part 1) and D-27.8
//! (part 2) in plan/07-decisions/s26-29.md.
//!
//! The ELF identity (class, byte order, machine) is checked only through [`Arch`], and relocation types
//! are seen only as [`RelocKind`] through [`Arch::classify`] (design §0a).

use core::marker::PhantomData;

use crate::arch::{Arch, RelocKind, ELFCLASS64, ELFDATA2MSB};

/// Size of the ELF64 file header.
pub const EHDR_SIZE: usize = 64;
/// Size of one ELF64 program header.
pub const PHDR_SIZE: usize = 56;
/// Size of one ELF64 dynamic entry.
pub const DYN_SIZE: usize = 16;
/// `e_phnum` value that means "the count is in section header 0" (`PN_XNUM`); refused.
pub const PN_XNUM: u16 = 0xffff;
/// Size of one ELF64 RELA entry (`r_offset`, `r_info`, `r_addend`).
pub const RELA_SIZE: usize = 24;
/// Size of one ELF64 RELR entry.
pub const RELR_SIZE: usize = 8;
/// Size of one symbol table entry (`Elf64_Sym`).
pub const SYM_SIZE: usize = 24;
/// Size of an `Elf64_Verdef`.
pub const VERDEF_SIZE: usize = 20;
/// Size of an `Elf64_Verdaux`.
pub const VERDAUX_SIZE: usize = 8;
/// Size of an `Elf64_Verneed`.
pub const VERNEED_SIZE: usize = 16;
/// Size of an `Elf64_Vernaux`.
pub const VERNAUX_SIZE: usize = 16;
/// The only `vd_version` / `vn_version` value (`VER_DEF_CURRENT`, `VER_NEED_CURRENT`).
pub const VER_CURRENT: u16 = 1;
/// `versym` bit that marks a hidden (non-default) version.
pub const VERSYM_HIDDEN: u16 = 0x8000;

/// Object file types (`e_type`) the reader accepts.
pub mod et {
    /// Executable file.
    pub const EXEC: u16 = 2;
    /// Shared object (or position-independent executable).
    pub const DYN: u16 = 3;
}

/// `EI_OSABI` values the reader accepts.
pub mod osabi {
    /// `ELFOSABI_NONE` / System V.
    pub const SYSV: u8 = 0;
    /// `ELFOSABI_GNU` (objects using GNU extensions such as `STT_GNU_IFUNC`).
    pub const GNU: u8 = 3;
}

/// Program header types (`p_type`).
pub mod pt {
    /// Loadable segment.
    pub const LOAD: u32 = 1;
    /// Dynamic section.
    pub const DYNAMIC: u32 = 2;
    /// Program interpreter path.
    pub const INTERP: u32 = 3;
    /// Notes.
    pub const NOTE: u32 = 4;
    /// The program header table itself.
    pub const PHDR: u32 = 6;
    /// Thread-local storage template.
    pub const TLS: u32 = 7;
    /// `.eh_frame_hdr`.
    pub const GNU_EH_FRAME: u32 = 0x6474_e550;
    /// Stack executability.
    pub const GNU_STACK: u32 = 0x6474_e551;
    /// Read-only after relocation.
    pub const GNU_RELRO: u32 = 0x6474_e552;
    /// `.note.gnu.property`.
    pub const GNU_PROPERTY: u32 = 0x6474_e553;
}

/// Dynamic tags (`d_tag`) the reader itself interprets.
pub mod dt {
    /// End of the dynamic array.
    pub const NULL: u64 = 0;
    /// Name of a needed library (string table offset).
    pub const NEEDED: u64 = 1;
    /// Size in bytes of the PLT relocations (`DT_JMPREL`).
    pub const PLTRELSZ: u64 = 2;
    /// Address of the PLT/GOT (processor-specific use; x86-64: `.got.plt`).
    pub const PLTGOT: u64 = 3;
    /// Address of the SysV hash table.
    pub const HASH: u64 = 4;
    /// Address of the dynamic string table.
    pub const STRTAB: u64 = 5;
    /// Address of the dynamic symbol table.
    pub const SYMTAB: u64 = 6;
    /// Size in bytes of one symbol table entry.
    pub const SYMENT: u64 = 11;
    /// Address of the RELA relocations.
    pub const RELA: u64 = 7;
    /// Size in bytes of the RELA relocations.
    pub const RELASZ: u64 = 8;
    /// Size of one RELA entry.
    pub const RELAENT: u64 = 9;
    /// The number of `R_*_RELATIVE` entries the RELA table starts with (GNU extension of the gABI's
    /// OS-specific tag range).
    pub const RELACOUNT: u64 = 0x6fff_fff9;
    /// Size of the dynamic string table in bytes.
    pub const STRSZ: u64 = 10;
    /// The object's soname (string table offset).
    pub const SONAME: u64 = 14;
    /// Library search path (string table offset).
    pub const RPATH: u64 = 15;
    /// Type of the PLT relocations (`DT_RELA` or `DT_REL`).
    pub const PLTREL: u64 = 20;
    /// The object has relocations against non-writable segments (gABI `DT_TEXTREL`).
    pub const TEXTREL: u64 = 22;
    /// Flag word (gABI `DT_FLAGS`; [`DF_TEXTREL`] is the text-relocation bit).
    pub const FLAGS: u64 = 30;
    /// `DF_TEXTREL` in `DT_FLAGS` (gABI).
    pub const DF_TEXTREL: u64 = 4;
    /// Address of the PLT relocations.
    pub const JMPREL: u64 = 23;
    /// Library search path (string table offset).
    pub const RUNPATH: u64 = 29;
    /// Size in bytes of the RELR relocations.
    pub const RELRSZ: u64 = 35;
    /// Address of the RELR relocations.
    pub const RELR: u64 = 36;
    /// Size of one RELR entry.
    pub const RELRENT: u64 = 37;
    /// Address of the initialisation function.
    pub const INIT: u64 = 12;
    /// Address of the termination function.
    pub const FINI: u64 = 13;
    /// Address of the array of initialisation functions.
    pub const INIT_ARRAY: u64 = 25;
    /// Address of the array of termination functions.
    pub const FINI_ARRAY: u64 = 26;
    /// Size in bytes of `DT_INIT_ARRAY`.
    pub const INIT_ARRAYSZ: u64 = 27;
    /// Size in bytes of `DT_FINI_ARRAY`.
    pub const FINI_ARRAYSZ: u64 = 28;
    /// Address of the array of pre-initialisation functions (the program's only).
    pub const PREINIT_ARRAY: u64 = 32;
    /// Size in bytes of `DT_PREINIT_ARRAY`.
    pub const PREINIT_ARRAYSZ: u64 = 33;
    /// Address of the GNU hash table.
    pub const GNU_HASH: u64 = 0x6fff_fef5;
    /// Address of the symbol version table (`versym`).
    pub const VERSYM: u64 = 0x6fff_fff0;
    /// Address of the version definitions.
    pub const VERDEF: u64 = 0x6fff_fffc;
    /// Number of version definitions.
    pub const VERDEFNUM: u64 = 0x6fff_fffd;
    /// Address of the version requirements.
    pub const VERNEED: u64 = 0x6fff_fffe;
    /// Number of version requirement entries (files).
    pub const VERNEEDNUM: u64 = 0x6fff_ffff;
}

/// Note types of the `GNU` owner.
pub mod nt {
    /// `NT_GNU_ABI_TAG`: OS and minimum kernel version.
    pub const GNU_ABI_TAG: u32 = 1;
    /// `NT_GNU_BUILD_ID`.
    pub const GNU_BUILD_ID: u32 = 3;
    /// `NT_GNU_PROPERTY_TYPE_0`: an array of program properties.
    pub const GNU_PROPERTY_TYPE_0: u32 = 5;
}

/// Which file range an out-of-range or overlap error is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The program header table.
    ProgramHeaders,
    /// The file range of the program header with this index.
    Segment(u16),
}

/// Which table a part-2 error is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    /// The SysV hash table (`DT_HASH`).
    SysvHash,
    /// The GNU hash table (`DT_GNU_HASH`).
    GnuHash,
    /// The symbol version table (`DT_VERSYM`).
    Versym,
    /// The version definitions (`DT_VERDEF`).
    Verdef,
    /// The version requirements (`DT_VERNEED`).
    Verneed,
    /// The RELA relocations (`DT_RELA`).
    Rela,
    /// The PLT relocations (`DT_JMPREL`).
    Jmprel,
    /// The RELR relocations (`DT_RELR`).
    Relr,
}

/// Why an object was refused. Every variant is a property of the bytes, so the same input always
/// gives the same error (decision D-27.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElfError {
    /// The slice is shorter than the ELF header.
    TooShort,
    /// The first four bytes are not `\x7fELF`.
    BadMagic,
    /// `EI_CLASS` is not the architecture's class (or the architecture's class is not 64-bit).
    WrongClass(u8),
    /// `EI_DATA` is not the architecture's byte order.
    WrongData(u8),
    /// `EI_VERSION` is not 1 (`EV_CURRENT`).
    BadIdentVersion(u8),
    /// `EI_OSABI` is neither System V nor GNU.
    UnsupportedOsAbi(u8),
    /// `e_type` is neither `ET_EXEC` nor `ET_DYN`.
    UnsupportedType(u16),
    /// `e_machine` is not the architecture's machine.
    WrongMachine(u16),
    /// `e_version` is not 1.
    BadVersion(u32),
    /// `e_ehsize` is not the ELF64 header size.
    BadEhsize(u16),
    /// `e_phentsize` is not the ELF64 program header size (while `e_phnum` is not 0).
    BadPhentsize(u16),
    /// `e_phnum` is `PN_XNUM` (the count would come from the section headers, which the reader does not use).
    PhnumExtended,
    /// A file range lies (partly) outside the slice, or its end overflows.
    OutOfRange(Part),
    /// A file range overlaps the ELF header or the program header table.
    OverlapsHeaders(Part),
    /// `p_offset + p_filesz` or `p_vaddr + p_memsz` overflows.
    SegmentOverflow(u16),
    /// A `PT_LOAD` segment has `p_filesz > p_memsz`.
    FileszExceedsMemsz(u16),
    /// `p_align` is neither 0 nor a power of two.
    BadAlign(u16),
    /// A second program header of a type that may occur once (`PT_DYNAMIC`, `PT_INTERP`).
    DuplicateSegment(u32),
    /// The `PT_INTERP` bytes are empty, do not end in NUL, or hold a NUL before the end.
    BadInterp,
    /// The `PT_DYNAMIC` size is not a multiple of the entry size.
    DynamicSize,
    /// No `DT_NULL` entry within the `PT_DYNAMIC` range.
    DynamicUnterminated,
    /// A second `DT_STRTAB` or `DT_STRSZ` entry.
    DuplicateDynamic(u64),
    /// A string-valued entry (or `DT_STRSZ`) without `DT_STRTAB`.
    MissingStrtab,
    /// `DT_STRTAB` without `DT_STRSZ`.
    MissingStrsz,
    /// An address range is not wholly inside the file-backed part of one `PT_LOAD` segment.
    AddressNotMapped(u64),
    /// A string offset is at or past the end of the string table.
    StringOutOfRange(u64),
    /// No NUL between a string offset and the end of the string table.
    StringUnterminated(u64),
    /// A `PT_NOTE` segment's `p_align` is not 0, 1, 2, 4 or 8.
    BadNoteAlign(u16),
    /// A note header, name or descriptor runs past the end of its segment (offset of the note in the segment).
    NoteTruncated(usize),
    /// A note name of nonzero size that does not end in NUL (offset of the note in the segment).
    BadNoteName(usize),
    /// A program property runs past the end of its note descriptor (offset of the property in the descriptor).
    PropertyTruncated(usize),
    /// A dynamic tag that another present tag requires is absent (the missing tag).
    MissingDynamic(u64),
    /// A dynamic entry has a value the reader does not accept (`DT_RELAENT` not 24, `DT_RELRENT` not 8,
    /// `DT_PLTREL` not `DT_RELA`); the tag.
    BadDynamicValue(u64),
    /// A table's size is not a multiple of its entry size.
    TableSize(Table),
    /// A hash table header the reader refuses (GNU: no Bloom words, or a Bloom shift of 64 or more).
    BadHashHeader(Table),
    /// A hash bucket or chain names a symbol index outside the table (the index).
    HashIndex(Table, u32),
    /// A SysV hash chain visits more entries than the table holds (a loop).
    HashChainLoop(u32),
    /// A version entry or auxiliary entry runs past the end of its segment (offset in the table).
    VersionTruncated(Table, usize),
    /// A version entry's revision is not 1.
    BadVersionRevision(Table, u16),
    /// A version chain ends (next offset 0) before its count (`DT_VERDEFNUM`, `DT_VERNEEDNUM`, `vd_cnt`,
    /// `vn_cnt`) is reached; the offset of the entry whose chain ended early.
    VersionCount(Table, usize),
    /// A `versym` entry names a version index that no definition or requirement has (symbol, index).
    VersionIndex(u32, u16),
    /// A symbol index at or past the symbol count.
    SymbolIndex(u32),
    /// A RELR bitmap entry with no address entry before it (entry index).
    RelrNoBase(usize),
    /// A RELR entry's addresses overflow the address space (entry index).
    RelrOverflow(usize),
}

/// Reads fixed-size integers from a slice in the object's byte order. `None` means out of range.
#[derive(Clone, Copy, Debug)]
struct Rd<'a> {
    b: &'a [u8],
    be: bool,
}

impl<'a> Rd<'a> {
    fn bytes<const N: usize>(&self, off: usize) -> Option<[u8; N]> {
        let end = off.checked_add(N)?;
        self.b.get(off..end)?.try_into().ok()
    }
    fn u16(&self, off: usize) -> Option<u16> {
        let a = self.bytes::<2>(off)?;
        Some(if self.be { u16::from_be_bytes(a) } else { u16::from_le_bytes(a) })
    }
    fn u32(&self, off: usize) -> Option<u32> {
        let a = self.bytes::<4>(off)?;
        Some(if self.be { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) })
    }
    fn u64(&self, off: usize) -> Option<u64> {
        let a = self.bytes::<8>(off)?;
        Some(if self.be { u64::from_be_bytes(a) } else { u64::from_le_bytes(a) })
    }
}

/// `b[off..off+len]` with every step checked (`None`: outside the slice or overflow).
fn sub(b: &[u8], off: u64, len: u64) -> Option<&[u8]> {
    let off = usize::try_from(off).ok()?;
    let len = usize::try_from(len).ok()?;
    b.get(off..off.checked_add(len)?)
}

/// Whether `[a, a+alen)` and `[b, b+blen)` overlap (empty ranges overlap nothing; ends saturate).
fn overlaps(a: u64, alen: u64, b: u64, blen: u64) -> bool {
    alen != 0 && blen != 0 && a < b.saturating_add(blen) && b < a.saturating_add(alen)
}

/// The ELF64 file header, decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// `EI_CLASS`.
    pub class: u8,
    /// `EI_DATA`.
    pub data: u8,
    /// `EI_VERSION`.
    pub ident_version: u8,
    /// `EI_OSABI`.
    pub osabi: u8,
    /// `EI_ABIVERSION`.
    pub abi_version: u8,
    /// `e_type`.
    pub e_type: u16,
    /// `e_machine`.
    pub machine: u16,
    /// `e_version`.
    pub version: u32,
    /// `e_entry`.
    pub entry: u64,
    /// `e_phoff`.
    pub phoff: u64,
    /// `e_shoff` (reported, not used or checked).
    pub shoff: u64,
    /// `e_flags`.
    pub flags: u32,
    /// `e_ehsize`.
    pub ehsize: u16,
    /// `e_phentsize`.
    pub phentsize: u16,
    /// `e_phnum`.
    pub phnum: u16,
    /// `e_shentsize` (reported, not used or checked).
    pub shentsize: u16,
    /// `e_shnum` (reported, not used or checked).
    pub shnum: u16,
    /// `e_shstrndx` (reported, not used or checked).
    pub shstrndx: u16,
}

/// One ELF64 program header, decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramHeader {
    /// Index in the program header table.
    pub index: u16,
    /// `p_type`.
    pub p_type: u32,
    /// `p_flags` (`PF_X` 1, `PF_W` 2, `PF_R` 4).
    pub flags: u32,
    /// `p_offset`.
    pub offset: u64,
    /// `p_vaddr`.
    pub vaddr: u64,
    /// `p_paddr`.
    pub paddr: u64,
    /// `p_filesz`.
    pub filesz: u64,
    /// `p_memsz`.
    pub memsz: u64,
    /// `p_align`.
    pub align: u64,
}

impl ProgramHeader {
    /// Header `index` from the 56 bytes of an `Elf64_Phdr` in byte order `be`: fixed offsets of an
    /// entry whose range was checked when the reader was made (LD-PHDR-ONCE: walks decode it often).
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn decode(e: &[u8; PHDR_SIZE], be: bool, index: u16) -> ProgramHeader {
        let w4 = |a: [u8; 4]| if be { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) };
        let w8 = |a: [u8; 8]| if be { u64::from_be_bytes(a) } else { u64::from_le_bytes(a) };
        ProgramHeader {
            index,
            p_type: w4([e[0], e[1], e[2], e[3]]),
            flags: w4([e[4], e[5], e[6], e[7]]),
            offset: w8([e[8], e[9], e[10], e[11], e[12], e[13], e[14], e[15]]),
            vaddr: w8([e[16], e[17], e[18], e[19], e[20], e[21], e[22], e[23]]),
            paddr: w8([e[24], e[25], e[26], e[27], e[28], e[29], e[30], e[31]]),
            filesz: w8([e[32], e[33], e[34], e[35], e[36], e[37], e[38], e[39]]),
            memsz: w8([e[40], e[41], e[42], e[43], e[44], e[45], e[46], e[47]]),
            align: w8([e[48], e[49], e[50], e[51], e[52], e[53], e[54], e[55]]),
        }
    }
}

/// A parsed object: the header and the program header table are checked; the contents of segments
/// are checked when they are read (the slice may be only a prefix of the file).
#[derive(Clone, Copy, Debug)]
pub struct Elf<'a> {
    /// The file (from [`Elf::parse`]), or the program header table (image mode).
    rd: Rd<'a>,
    header: Header,
    /// Image mode ([`Elf::from_image`]): the readable ranges by link-time address.
    spans: Option<&'a [Span<'a>]>,
}

/// `PF_W`, `PF_R` (ELF gABI).
const PF_W: u32 = 2;
const PF_R: u32 = 4;

/// The dynamic tags whose value is the link-time address of a table the reader reads.
const IMAGE_TABLES: [u64; 10] = [dt::STRTAB, dt::SYMTAB, dt::HASH, dt::GNU_HASH, dt::VERSYM, dt::VERDEF, dt::VERNEED, dt::RELA, dt::JMPREL, dt::RELR];

/// The table addresses an [`ImageTables`] keeps: one per [`IMAGE_TABLES`] tag and room for a few
/// repeated ones; beyond them the entries are walked.
const IMAGE_TABLE_SLOTS: usize = 16;

/// What [`Elf::image_span_kind`] reads of an object's dynamic entries ([`Elf::image_tables`]): whether
/// it is text-relocated, and the addresses of the tables the reader reads by address.
#[derive(Clone, Copy, Debug)]
pub struct ImageTables<'d> {
    dynamic: &'d [u8],
    be: bool,
    text: bool,
    /// The number of [`IMAGE_TABLES`] entries; past [`IMAGE_TABLE_SLOTS`] only `dynamic` has them.
    n: usize,
    at: [u64; IMAGE_TABLE_SLOTS],
}

/// The (tag, value) entries of the dynamic bytes `dynamic` up to its `DT_NULL` or its end.
fn image_entries(dynamic: &[u8], be: bool) -> impl Iterator<Item = (u64, u64)> + '_ {
    let rd = Rd { b: dynamic, be };
    (0..dynamic.len().checked_div(DYN_SIZE).unwrap_or(0))
        .map_while(move |i| {
            let off = i.checked_mul(DYN_SIZE)?;
            Some((rd.u64(off)?, rd.u64(off.checked_add(8)?)?))
        })
        .take_while(|&(tag, _)| tag != dt::NULL)
}

/// The dynamic tags whose entries a loaded object's dynamic array holds as run-time addresses
/// (load bias added) once the loader has relocated it (DL-DYNAMIC-RELOC, probe
/// `tools/probe/dl/dynreloc`): programs read these through `_DYNAMIC`, `dl_iterate_phdr` and `l_ld`
/// and use them as pointers. Every other entry (`DT_INIT`, the `*_ARRAY` tables, `DT_VERDEF`,
/// `DT_VERNEED`, `DT_TLSDESC_*`, the sizes and flags) keeps its link-time value.
pub const RUNTIME_ADDRESS_TAGS: [u64; 9] = [dt::PLTGOT, dt::HASH, dt::STRTAB, dt::SYMTAB, dt::RELA, dt::JMPREL, dt::RELR, dt::GNU_HASH, dt::VERSYM];

/// Whether a loaded object's dynamic array (link-time address `vaddr`, `size` bytes) holds the
/// [`RUNTIME_ADDRESS_TAGS`] entries as run-time addresses: when it lies wholly inside one of
/// `writable` (each writable `PT_LOAD` as link-time address and size in memory). The loader writes
/// nothing outside a writable segment, so an array anywhere else keeps its link-time values. The one
/// rule for the loader that writes the array and for every reader of the loaded array.
pub fn dynamic_published(vaddr: u64, size: u64, writable: impl IntoIterator<Item = (u64, u64)>) -> bool {
    let Some(end) = vaddr.checked_add(size) else { return false };
    writable.into_iter().any(|(v, m)| v <= vaddr && v.checked_add(m).is_some_and(|e| end <= e))
}

/// Add `delta` (wrapping) to the value of each [`RUNTIME_ADDRESS_TAGS`] entry of `bytes`, dynamic
/// entries in native byte order up to the first `DT_NULL` (a trailing partial entry is left alone).
/// With the load bias negated, this turns a copy of a published array back into link-time values.
pub fn shift_address_entries(bytes: &mut [u8], delta: u64) {
    for e in bytes.chunks_exact_mut(DYN_SIZE) {
        let (tag, val) = e.split_at_mut(8);
        let word = |b: &[u8]| <[u8; 8]>::try_from(b).map_or(0, u64::from_ne_bytes);
        let tag = word(tag);
        if tag == dt::NULL {
            break;
        }
        if RUNTIME_ADDRESS_TAGS.contains(&tag) {
            let v = word(val).wrapping_add(delta);
            val.copy_from_slice(&v.to_ne_bytes());
        }
    }
}

/// Image mode: `bytes` are the contents of link-time addresses `vaddr .. vaddr + bytes.len()` of a
/// mapped object (a slice of its memory, or a copy taken before relocation; D-27.18).
#[derive(Clone, Copy, Debug)]
pub struct Span<'a> {
    /// Link-time address of the first byte.
    pub vaddr: u64,
    /// The bytes.
    pub bytes: &'a [u8],
}

/// How the image-mode reader sees one `PT_LOAD` ([`Elf::image_span_kind`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanKind {
    /// Its file-backed bytes are sliced in place: they do not change while the object is loaded.
    Live,
    /// Its file-backed bytes are copied before the first relocation store.
    Copy,
    /// Not read (empty, or changing and holding no table).
    Unseen,
}

/// The span in `spans` holding all of `[vaddr, vaddr + len)`, as a slice of exactly that range.
fn span_bytes<'a>(spans: &[Span<'a>], vaddr: u64, len: u64) -> Option<&'a [u8]> {
    spans.iter().find_map(|s| sub(s.bytes, vaddr.checked_sub(s.vaddr)?, len))
}

impl<'a> Elf<'a> {
    /// Checks the ELF header against architecture `A` and every program header, without reading
    /// any segment's contents.
    ///
    /// # Errors
    ///
    /// The first rule the bytes break, in the order of decision D-27.7: identity, header fields, the
    /// program header table's range, then each program header in index order.
    pub fn parse<A: Arch>(bytes: &'a [u8]) -> Result<Self, ElfError> {
        let ident = bytes.get(..16).ok_or(ElfError::TooShort)?;
        if bytes.len() < EHDR_SIZE {
            return Err(ElfError::TooShort);
        }
        let (magic, class, data, ident_version, osabi, abi_version) = match ident {
            [m0, m1, m2, m3, c, d, v, o, a, ..] => ([*m0, *m1, *m2, *m3], *c, *d, *v, *o, *a),
            _ => return Err(ElfError::TooShort),
        };
        if magic != *b"\x7fELF" {
            return Err(ElfError::BadMagic);
        }
        if class != A::EI_CLASS || class != ELFCLASS64 {
            return Err(ElfError::WrongClass(class));
        }
        if data != A::EI_DATA {
            return Err(ElfError::WrongData(data));
        }
        if ident_version != 1 {
            return Err(ElfError::BadIdentVersion(ident_version));
        }
        if osabi != osabi::SYSV && osabi != osabi::GNU {
            return Err(ElfError::UnsupportedOsAbi(osabi));
        }
        let rd = Rd { b: bytes, be: data == ELFDATA2MSB };
        let h = read_header(&rd, [class, data, ident_version, osabi, abi_version]).ok_or(ElfError::TooShort)?;
        if h.e_type != et::EXEC && h.e_type != et::DYN {
            return Err(ElfError::UnsupportedType(h.e_type));
        }
        if h.machine != A::E_MACHINE {
            return Err(ElfError::WrongMachine(h.machine));
        }
        if h.version != 1 {
            return Err(ElfError::BadVersion(h.version));
        }
        if usize::from(h.ehsize) != EHDR_SIZE {
            return Err(ElfError::BadEhsize(h.ehsize));
        }
        if h.phnum == PN_XNUM {
            return Err(ElfError::PhnumExtended);
        }
        if h.phnum != 0 && usize::from(h.phentsize) != PHDR_SIZE {
            return Err(ElfError::BadPhentsize(h.phentsize));
        }
        let elf = Elf { rd, header: h, spans: None };
        let table = elf.table_len();
        if sub(bytes, h.phoff, table).is_none() {
            return Err(ElfError::OutOfRange(Part::ProgramHeaders));
        }
        if overlaps(h.phoff, table, 0, ehdr_len()) {
            return Err(ElfError::OverlapsHeaders(Part::ProgramHeaders));
        }
        elf.check_table()?;
        Ok(elf)
    }

    /// Image mode (ruling #4229, D-27.18): an object the kernel (or the loader) has already mapped,
    /// read from its memory instead of its file. `phdrs` holds its program header table (`AT_PHDR`,
    /// `AT_PHNUM` entries); `spans` give the bytes of the link-time address ranges the reader may
    /// read (see [`Elf::image_span_kind`]), searched in order. Every address the reader resolves must
    /// lie in the file-backed part of one `PT_LOAD` and wholly in one span; anything else is
    /// [`ElfError::AddressNotMapped`], as for a file.
    ///
    /// There is no file header in this mode: [`Elf::header`] returns `A`'s identity, `e_type`
    /// `ET_DYN`, `e_phoff` 0 and the table's `e_phnum`, nothing else. The program header checks are
    /// those of [`Elf::parse`] except the file-offset overlaps with the headers.
    ///
    /// # Errors
    ///
    /// [`ElfError::OutOfRange`] for a table that is empty, not whole entries or `PN_XNUM` entries or
    /// more, then each program header's error in index order.
    pub fn from_image<A: Arch>(phdrs: &'a [u8], spans: &'a [Span<'a>]) -> Result<Self, ElfError> {
        let bad = ElfError::OutOfRange(Part::ProgramHeaders);
        if phdrs.is_empty() || phdrs.len().checked_rem(PHDR_SIZE) != Some(0) {
            return Err(bad);
        }
        let phnum = u16::try_from(phdrs.len().checked_div(PHDR_SIZE).ok_or(bad)?).ok().filter(|&n| n < PN_XNUM).ok_or(bad)?;
        let header = Header {
            class: A::EI_CLASS,
            data: A::EI_DATA,
            ident_version: 1,
            osabi: osabi::SYSV,
            abi_version: 0,
            e_type: et::DYN,
            machine: A::E_MACHINE,
            version: 1,
            entry: 0,
            phoff: 0,
            shoff: 0,
            flags: 0,
            ehsize: u16::try_from(EHDR_SIZE).unwrap_or(0),
            phentsize: u16::try_from(PHDR_SIZE).unwrap_or(0),
            phnum,
            shentsize: 0,
            shnum: 0,
            shstrndx: 0,
        };
        let elf = Elf { rd: Rd { b: phdrs, be: A::EI_DATA == ELFDATA2MSB }, header, spans: Some(spans) };
        elf.check_table()?;
        Ok(elf)
    }

    /// Image mode: how the reader may see `PT_LOAD` `ph` of an object whose dynamic entries are
    /// `dynamic` (a copy of its `PT_DYNAMIC` bytes, up to its `DT_NULL` or the end). The rule
    /// (D-27.18): a segment whose bytes cannot change while the object is loaded and relocated is
    /// sliced in place; a segment that can change is never sliced in place (a shared slice alive
    /// across the relocator's stores would alias them), but copied before the first store when a
    /// table the reader reads by address starts in it, and not seen at all otherwise.
    #[must_use]
    pub fn image_span_kind(&self, ph: &ProgramHeader, dynamic: &[u8]) -> SpanKind {
        self.image_span_kind_with(ph, &self.image_tables(dynamic))
    }

    /// What [`Elf::image_span_kind`] reads of the dynamic entries `dynamic`, read once for all of an
    /// object's segments (LD-OBJECT-ONCE: each segment's kind, asked three times per segment, walked
    /// every entry).
    #[must_use]
    pub fn image_tables<'d>(&self, dynamic: &'d [u8]) -> ImageTables<'d> {
        let mut t = ImageTables { dynamic, be: self.rd.be, text: false, n: 0, at: [0; IMAGE_TABLE_SLOTS] };
        for (tag, val) in image_entries(dynamic, self.rd.be) {
            t.text |= tag == dt::TEXTREL || (tag == dt::FLAGS && val & dt::DF_TEXTREL != 0);
            if IMAGE_TABLES.contains(&tag) {
                if let Some(slot) = t.at.get_mut(t.n) {
                    *slot = val;
                }
                t.n = t.n.saturating_add(1);
            }
        }
        t
    }

    /// [`Elf::image_span_kind`] from the entries' [`Elf::image_tables`].
    #[must_use]
    pub fn image_span_kind_with(&self, ph: &ProgramHeader, tables: &ImageTables<'_>) -> SpanKind {
        let text = tables.text;
        let file_end = ph.vaddr.saturating_add(ph.filesz);
        let holds = |val: u64| ph.vaddr <= val && val < file_end;
        let holds_table = match tables.at.get(..tables.n) {
            Some(at) => at.iter().any(|&v| holds(v)),
            // More table entries than the slots (repeated tags): every entry, as they come.
            None => image_entries(tables.dynamic, tables.be).any(|(tag, val)| IMAGE_TABLES.contains(&tag) && holds(val)),
        };
        // The relocator stores into PF_W segments, and into every segment of a text-relocated object.
        let changes = |p: &ProgramHeader| p.p_type == pt::LOAD && (text || p.flags & PF_W != 0);
        let aliased = self.program_headers().any(|p| changes(&p) && overlaps(p.vaddr, p.memsz, ph.vaddr, ph.filesz));
        if ph.p_type != pt::LOAD || ph.filesz == 0 || ph.flags & PF_R == 0 {
            SpanKind::Unseen
        } else if !aliased {
            SpanKind::Live
        } else if holds_table {
            SpanKind::Copy
        } else {
            SpanKind::Unseen
        }
    }

    /// Every program header checked, at most one `PT_DYNAMIC` and one `PT_INTERP`.
    fn check_table(&self) -> Result<(), ElfError> {
        let mut seen_dynamic = false;
        let mut seen_interp = false;
        for ph in self.program_headers() {
            self.check_phdr(&ph)?;
            let seen = match ph.p_type {
                pt::DYNAMIC => Some(&mut seen_dynamic),
                pt::INTERP => Some(&mut seen_interp),
                _ => None,
            };
            if let Some(seen) = seen {
                if *seen {
                    return Err(ElfError::DuplicateSegment(ph.p_type));
                }
                *seen = true;
            }
        }
        Ok(())
    }

    /// The decoded ELF header.
    #[must_use]
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Length in bytes of the program header table (`e_phnum` entries).
    fn table_len(&self) -> u64 {
        // PHDR_SIZE * u16::MAX fits in u64; saturating anyway so no arithmetic can wrap.
        u64::from(self.header.phnum).saturating_mul(phdr_len())
    }

    /// The program header table as `e_phnum` whole entries. In both modes `rd` holds it at `e_phoff`
    /// (image mode: `rd` is the table and `e_phoff` is 0), and making the reader checked that range,
    /// so a walk takes the slice once and reads fixed offsets of its entries (LD-PHDR-ONCE: a start
    /// walks the tables hundreds of times). Empty only for a table of no entries.
    fn table(&self) -> impl Iterator<Item = &'a [u8; PHDR_SIZE]> + 'a {
        let t = sub(self.rd.b, self.header.phoff, self.table_len()).unwrap_or(&[]);
        t.chunks_exact(PHDR_SIZE).filter_map(<[u8]>::first_chunk::<PHDR_SIZE>)
    }

    fn read_phdr(&self, i: u16) -> Option<ProgramHeader> {
        self.table().nth(usize::from(i)).map(|e| ProgramHeader::decode(e, self.rd.be, i))
    }

    fn check_phdr(&self, ph: &ProgramHeader) -> Result<(), ElfError> {
        let i = ph.index;
        if ph.offset.checked_add(ph.filesz).is_none() || ph.vaddr.checked_add(ph.memsz).is_none() {
            return Err(ElfError::SegmentOverflow(i));
        }
        if ph.align > 1 && !ph.align.is_power_of_two() {
            return Err(ElfError::BadAlign(i));
        }
        if ph.p_type == pt::LOAD && ph.filesz > ph.memsz {
            return Err(ElfError::FileszExceedsMemsz(i));
        }
        if self.spans.is_none()
            && matches!(ph.p_type, pt::DYNAMIC | pt::INTERP | pt::NOTE)
            && (overlaps(ph.offset, ph.filesz, 0, ehdr_len())
                || overlaps(ph.offset, ph.filesz, self.header.phoff, self.table_len()))
        {
            return Err(ElfError::OverlapsHeaders(Part::Segment(i)));
        }
        Ok(())
    }

    /// The program header with index `i` (`None` past `e_phnum`).
    #[must_use]
    pub fn program_header(&self, i: u16) -> Option<ProgramHeader> {
        if i < self.header.phnum { self.read_phdr(i) } else { None }
    }

    /// The program headers in table order.
    pub fn program_headers(&self) -> impl Iterator<Item = ProgramHeader> + 'a {
        let be = self.rd.be;
        // `e_phnum` entries, fewer than `PN_XNUM`, so every index fits in a u16.
        self.table().zip(0u16..).map(move |(e, i)| ProgramHeader::decode(e, be, i))
    }

    /// The first program header of type `p_type`.
    #[must_use]
    pub fn find(&self, p_type: u32) -> Option<ProgramHeader> {
        self.program_headers().find(|p| p.p_type == p_type)
    }

    /// `PT_DYNAMIC` when, loaded, the object's dynamic array holds run-time addresses
    /// ([`dynamic_published`]); `None` without `PT_DYNAMIC` or when it keeps link-time values.
    #[must_use]
    pub fn published_dynamic(&self) -> Option<ProgramHeader> {
        let d = self.find(pt::DYNAMIC)?;
        let writable = self.program_headers().filter(|p| p.p_type == pt::LOAD && p.flags & PF_W != 0).map(|p| (p.vaddr, p.memsz));
        dynamic_published(d.vaddr, d.memsz, writable).then_some(d)
    }

    /// The file bytes of a segment (`p_offset .. p_offset + p_filesz`).
    ///
    /// # Errors
    ///
    /// [`ElfError::OutOfRange`] when the range is not wholly inside the slice.
    pub fn segment_bytes(&self, ph: &ProgramHeader) -> Result<&'a [u8], ElfError> {
        let found = match self.spans {
            Some(spans) => span_bytes(spans, ph.vaddr, ph.filesz),
            None => sub(self.rd.b, ph.offset, ph.filesz),
        };
        found.ok_or(ElfError::OutOfRange(Part::Segment(ph.index)))
    }

    /// The file bytes behind `len` bytes at virtual address `vaddr`: the range must lie wholly in the
    /// file-backed part (`p_filesz`) of one `PT_LOAD` segment.
    ///
    /// # Errors
    ///
    /// [`ElfError::AddressNotMapped`] when no segment holds the whole range, [`ElfError::OutOfRange`]
    /// when the segment holding it extends past the slice.
    pub fn bytes_at(&self, vaddr: u64, len: u64) -> Result<&'a [u8], ElfError> {
        let end = vaddr.checked_add(len).ok_or(ElfError::AddressNotMapped(vaddr))?;
        let ph = self
            .program_headers()
            .find(|p| p.p_type == pt::LOAD && p.vaddr <= vaddr && p.vaddr.checked_add(p.filesz).is_some_and(|e| end <= e))
            .ok_or(ElfError::AddressNotMapped(vaddr))?;
        if let Some(spans) = self.spans {
            return span_bytes(spans, vaddr, len).ok_or(ElfError::AddressNotMapped(vaddr));
        }
        let rel = vaddr.checked_sub(ph.vaddr).ok_or(ElfError::AddressNotMapped(vaddr))?;
        let seg = self.segment_bytes(&ph)?;
        sub(seg, rel, len).ok_or(ElfError::AddressNotMapped(vaddr))
    }

    /// The file bytes from virtual address `vaddr` to the end of the file-backed part of the `PT_LOAD`
    /// segment holding it (possibly empty), for tables whose length is found by walking them.
    ///
    /// # Errors
    ///
    /// As [`Elf::bytes_at`].
    pub fn bytes_from(&self, vaddr: u64) -> Result<&'a [u8], ElfError> {
        let ph = self
            .program_headers()
            .find(|p| p.p_type == pt::LOAD && p.vaddr <= vaddr && p.vaddr.checked_add(p.filesz).is_some_and(|e| vaddr <= e))
            .ok_or(ElfError::AddressNotMapped(vaddr))?;
        if let Some(spans) = self.spans {
            // The rest of the span holding `vaddr`, cut at the end of the segment's file-backed part.
            let left = ph.vaddr.saturating_add(ph.filesz).saturating_sub(vaddr);
            if left == 0 {
                return Ok(&[]);
            }
            let rest = spans.iter().find_map(|s| {
                let r = usize::try_from(vaddr.checked_sub(s.vaddr)?).ok()?;
                s.bytes.get(r..).filter(|b| !b.is_empty())
            });
            return rest.map(|r| r.get(..usize::try_from(left).unwrap_or(usize::MAX)).unwrap_or(r)).ok_or(ElfError::AddressNotMapped(vaddr));
        }
        let rel = vaddr.checked_sub(ph.vaddr).ok_or(ElfError::AddressNotMapped(vaddr))?;
        let seg = self.segment_bytes(&ph)?;
        usize::try_from(rel).ok().and_then(|r| seg.get(r..)).ok_or(ElfError::AddressNotMapped(vaddr))
    }

    /// The program interpreter path (without its NUL), if the object has `PT_INTERP`.
    ///
    /// # Errors
    ///
    /// [`ElfError::OutOfRange`] for bytes outside the slice, [`ElfError::BadInterp`] unless the bytes
    /// are a non-empty path followed by exactly one NUL at the end.
    pub fn interp(&self) -> Result<Option<&'a [u8]>, ElfError> {
        let Some(ph) = self.find(pt::INTERP) else { return Ok(None) };
        let b = self.segment_bytes(&ph)?;
        match b.split_last() {
            Some((0, path)) if !path.is_empty() && !path.contains(&0) => Ok(Some(path)),
            _ => Err(ElfError::BadInterp),
        }
    }

    /// The dynamic section, if the object has `PT_DYNAMIC`.
    ///
    /// # Errors
    ///
    /// [`ElfError::OutOfRange`], [`ElfError::DynamicSize`], [`ElfError::DynamicUnterminated`],
    /// [`ElfError::DuplicateDynamic`], [`ElfError::MissingStrtab`], [`ElfError::MissingStrsz`], or
    /// the [`Elf::bytes_at`] errors for the string table.
    pub fn dynamic(&self) -> Result<Option<Dynamic<'a>>, ElfError> {
        let Some(ph) = self.find(pt::DYNAMIC) else { return Ok(None) };
        let all = self.segment_bytes(&ph)?;
        if all.len().checked_rem(DYN_SIZE) != Some(0) {
            return Err(ElfError::DynamicSize);
        }
        let rd = Rd { b: all, be: self.rd.be };
        let (mut strtab, mut strsz, mut wants_strings) = (None, None, false);
        let mut count = 0usize;
        loop {
            let off = count.checked_mul(DYN_SIZE).ok_or(ElfError::DynamicUnterminated)?;
            let (Some(tag), Some(val)) = (rd.u64(off), off.checked_add(8).and_then(|o| rd.u64(o))) else {
                return Err(ElfError::DynamicUnterminated);
            };
            count = count.checked_add(1).ok_or(ElfError::DynamicUnterminated)?;
            match tag {
                dt::NULL => break,
                dt::STRTAB | dt::STRSZ => {
                    let slot = if tag == dt::STRTAB { &mut strtab } else { &mut strsz };
                    if slot.replace(val).is_some() {
                        return Err(ElfError::DuplicateDynamic(tag));
                    }
                }
                dt::NEEDED | dt::SONAME | dt::RPATH | dt::RUNPATH => wants_strings = true,
                _ => {}
            }
        }
        let strtab = match (strtab, strsz) {
            (Some(addr), Some(size)) => Some(self.bytes_at(addr, size)?),
            (Some(_), None) => return Err(ElfError::MissingStrsz),
            (None, Some(_)) => return Err(ElfError::MissingStrtab),
            (None, None) if wants_strings => return Err(ElfError::MissingStrtab),
            (None, None) => None,
        };
        let used = all.get(..count.checked_mul(DYN_SIZE).ok_or(ElfError::DynamicUnterminated)?).ok_or(ElfError::DynamicUnterminated)?;
        Ok(Some(Dynamic { rd: Rd { b: used, be: self.rd.be }, strtab, elf: *self }))
    }

    /// The notes of a `PT_NOTE` segment (or any segment laid out as notes, such as `PT_GNU_PROPERTY`).
    ///
    /// # Errors
    ///
    /// [`ElfError::OutOfRange`] for bytes outside the slice, [`ElfError::BadNoteAlign`] for an
    /// alignment other than 0, 1, 2, 4 or 8. Each note's own errors come from the iterator.
    pub fn notes(&self, ph: &ProgramHeader) -> Result<Notes<'a>, ElfError> {
        let pad = match ph.align {
            0..=4 => 4,
            8 => 8,
            _ => return Err(ElfError::BadNoteAlign(ph.index)),
        };
        Ok(Notes { rd: Rd { b: self.segment_bytes(ph)?, be: self.rd.be }, pad, off: 0, done: false })
    }
}

fn read_header(rd: &Rd<'_>, ident: [u8; 5]) -> Option<Header> {
    let [class, data, ident_version, osabi, abi_version] = ident;
    Some(Header {
        class,
        data,
        ident_version,
        osabi,
        abi_version,
        e_type: rd.u16(16)?,
        machine: rd.u16(18)?,
        version: rd.u32(20)?,
        entry: rd.u64(24)?,
        phoff: rd.u64(32)?,
        shoff: rd.u64(40)?,
        flags: rd.u32(48)?,
        ehsize: rd.u16(52)?,
        phentsize: rd.u16(54)?,
        phnum: rd.u16(56)?,
        shentsize: rd.u16(58)?,
        shnum: rd.u16(60)?,
        shstrndx: rd.u16(62)?,
    })
}

fn ehdr_len() -> u64 {
    u64::try_from(EHDR_SIZE).unwrap_or(u64::MAX)
}

fn phdr_len() -> u64 {
    u64::try_from(PHDR_SIZE).unwrap_or(u64::MAX)
}

/// One dynamic entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DynEntry {
    /// `d_tag`.
    pub tag: u64,
    /// `d_val` / `d_ptr`.
    pub val: u64,
}

/// The dynamic section: its entries up to and including the first `DT_NULL`, and the string table
/// (`DT_STRTAB`, `DT_STRSZ`) resolved through the `PT_LOAD` segments.
#[derive(Clone, Copy, Debug)]
pub struct Dynamic<'a> {
    rd: Rd<'a>,
    strtab: Option<&'a [u8]>,
    elf: Elf<'a>,
}

impl<'a> Dynamic<'a> {
    /// The number of entries, the terminating `DT_NULL` included.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rd.b.len().checked_div(DYN_SIZE).unwrap_or(0)
    }

    /// Never true: a checked dynamic section holds at least its `DT_NULL`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The entries in order, the terminating `DT_NULL` included.
    pub fn entries(&self) -> impl Iterator<Item = DynEntry> + 'a {
        let rd = self.rd;
        (0..self.len()).filter_map(move |i| {
            let off = i.checked_mul(DYN_SIZE)?;
            Some(DynEntry { tag: rd.u64(off)?, val: rd.u64(off.checked_add(8)?)? })
        })
    }

    /// The value of the first entry with tag `tag`.
    #[must_use]
    pub fn get(&self, tag: u64) -> Option<u64> {
        self.entries().find(|e| e.tag == tag).map(|e| e.val)
    }

    /// The string table (`None` when the object has no `DT_STRTAB`).
    #[must_use]
    pub fn strtab(&self) -> Option<&'a [u8]> {
        self.strtab
    }

    /// The NUL-terminated string at offset `off` of the string table, without its NUL.
    ///
    /// # Errors
    ///
    /// [`ElfError::MissingStrtab`], [`ElfError::StringOutOfRange`] (offset at or past `DT_STRSZ`),
    /// [`ElfError::StringUnterminated`] (no NUL before the end of the table).
    pub fn string(&self, off: u64) -> Result<&'a [u8], ElfError> {
        let tab = self.strtab.ok_or(ElfError::MissingStrtab)?;
        let start = usize::try_from(off).ok().filter(|&o| o < tab.len()).ok_or(ElfError::StringOutOfRange(off))?;
        let rest = tab.get(start..).ok_or(ElfError::StringOutOfRange(off))?;
        let n = rest.iter().position(|&c| c == 0).ok_or(ElfError::StringUnterminated(off))?;
        rest.get(..n).ok_or(ElfError::StringUnterminated(off))
    }

    /// [`Dynamic::string`] at `off` and the GNU hash of it (`symbols::gnu_hash`), found in one pass
    /// over its bytes (LD-LOOKUP-COST: a relocation's name was scanned for its end, then hashed).
    ///
    /// # Errors
    ///
    /// As [`Dynamic::string`].
    pub fn hashed_string(&self, off: u32) -> Result<(&'a [u8], u32), ElfError> {
        let off = u64::from(off);
        let tab = self.strtab.ok_or(ElfError::MissingStrtab)?;
        let rest = usize::try_from(off).ok().and_then(|o| tab.get(o..)).filter(|r| !r.is_empty()).ok_or(ElfError::StringOutOfRange(off))?;
        let mut h = 5381u32;
        // Eight bytes at a time while none of them is the NUL (a word holds a zero byte exactly when
        // `(w - 0x0101..01) & !w & 0x8080..80` is not zero), then byte by byte.
        let mut done = 0usize;
        while let Some(w) = rest.get(done..).and_then(<[u8]>::first_chunk::<8>) {
            let x = u64::from_le_bytes(*w);
            if x.wrapping_sub(0x0101_0101_0101_0101) & !x & 0x8080_8080_8080_8080 != 0 {
                break;
            }
            for &c in w {
                h = h.wrapping_mul(33).wrapping_add(u32::from(c));
            }
            done = done.wrapping_add(8);
        }
        for (n, &c) in rest.get(done..).unwrap_or(&[]).iter().enumerate() {
            if c == 0 {
                return rest.get(..done.wrapping_add(n)).map(|s| (s, h)).ok_or(ElfError::StringUnterminated(off));
            }
            h = h.wrapping_mul(33).wrapping_add(u32::from(c));
        }
        Err(ElfError::StringUnterminated(off))
    }

    /// Whether [`Dynamic::string`] at `off` is `name`, which holds no NUL (the caller's promise), with
    /// the same errors. A string table that ends in a NUL (as every linker writes one) terminates every
    /// string in range, so the bytes are compared in place, without a scan for the end first.
    ///
    /// # Errors
    ///
    /// As [`Dynamic::string`].
    #[inline]
    pub fn string_is(&self, off: u32, name: &[u8]) -> Result<bool, ElfError> {
        let tab = self.strtab.ok_or(ElfError::MissingStrtab)?;
        let start = usize::try_from(off).ok().filter(|&o| o < tab.len()).ok_or(ElfError::StringOutOfRange(u64::from(off)))?;
        if tab.last() != Some(&0) {
            return Ok(self.string(u64::from(off))? == name);
        }
        let n = name.len();
        Ok(start.checked_add(n).and_then(|end| tab.get(start..=end)).is_some_and(|s| s.last() == Some(&0) && s.get(..n) == Some(name)))
    }

    /// The `DT_NEEDED` names in order.
    pub fn needed(&self) -> impl Iterator<Item = Result<&'a [u8], ElfError>> + 'a {
        let d = *self;
        self.entries().filter(|e| e.tag == dt::NEEDED).map(move |e| d.string(e.val))
    }
}

/// One note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Note<'a> {
    /// `n_type`.
    pub n_type: u32,
    /// The owner name without its NUL (`GNU`).
    pub name: &'a [u8],
    /// The descriptor (`n_descsz` bytes, padding excluded).
    pub desc: &'a [u8],
    be: bool,
}

impl<'a> Note<'a> {
    /// The program properties of an `NT_GNU_PROPERTY_TYPE_0` note's descriptor (8-byte aligned
    /// entries of `pr_type`, `pr_datasz`, data). The iterator yields nothing for other notes.
    #[must_use]
    pub fn properties(&self) -> Properties<'a> {
        let ours = self.name == b"GNU" && self.n_type == nt::GNU_PROPERTY_TYPE_0;
        Properties { rd: Rd { b: if ours { self.desc } else { &[] }, be: self.be }, off: 0, done: false }
    }
}

/// Iterator over the notes of a segment; it yields one error and then stops.
#[derive(Clone, Debug)]
pub struct Notes<'a> {
    rd: Rd<'a>,
    pad: usize,
    off: usize,
    done: bool,
}

/// `n` rounded up to a multiple of `pad` (a power of two), checked.
fn round_up(n: usize, pad: usize) -> Option<usize> {
    let mask = pad.checked_sub(1)?;
    Some(n.checked_add(mask)? & !mask)
}

impl<'a> Notes<'a> {
    fn step(&mut self) -> Result<Note<'a>, ElfError> {
        let at = self.off;
        let bad = ElfError::NoteTruncated(at);
        let namesz = usize::try_from(self.rd.u32(at).ok_or(bad)?).map_err(|_| bad)?;
        let descsz = usize::try_from(self.rd.u32(at.checked_add(4).ok_or(bad)?).ok_or(bad)?).map_err(|_| bad)?;
        let n_type = self.rd.u32(at.checked_add(8).ok_or(bad)?).ok_or(bad)?;
        let name_at = at.checked_add(12).ok_or(bad)?;
        let name_full = self.rd.b.get(name_at..name_at.checked_add(namesz).ok_or(bad)?).ok_or(bad)?;
        let desc_at = round_up(name_at.checked_add(namesz).ok_or(bad)?, self.pad).ok_or(bad)?;
        let desc = self.rd.b.get(desc_at..desc_at.checked_add(descsz).ok_or(bad)?).ok_or(bad)?;
        let name = match name_full.split_last() {
            None => name_full,
            Some((0, n)) => n,
            Some(_) => return Err(ElfError::BadNoteName(at)),
        };
        let next = round_up(desc_at.checked_add(descsz).ok_or(bad)?, self.pad).ok_or(bad)?;
        // The padding after the last descriptor may be cut by the segment's end.
        self.off = next.min(self.rd.b.len());
        Ok(Note { n_type, name, desc, be: self.rd.be })
    }
}

impl<'a> Iterator for Notes<'a> {
    type Item = Result<Note<'a>, ElfError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.off >= self.rd.b.len() {
            return None;
        }
        let r = self.step();
        self.done = r.is_err();
        Some(r)
    }
}

/// One program property of an `NT_GNU_PROPERTY_TYPE_0` note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Property<'a> {
    /// `pr_type`.
    pub pr_type: u32,
    /// `pr_datasz` bytes of data.
    pub data: &'a [u8],
}

/// Iterator over program properties; it yields one error and then stops.
#[derive(Clone, Debug)]
pub struct Properties<'a> {
    rd: Rd<'a>,
    off: usize,
    done: bool,
}

impl<'a> Properties<'a> {
    fn step(&mut self) -> Result<Property<'a>, ElfError> {
        let at = self.off;
        let bad = ElfError::PropertyTruncated(at);
        let pr_type = self.rd.u32(at).ok_or(bad)?;
        let size = usize::try_from(self.rd.u32(at.checked_add(4).ok_or(bad)?).ok_or(bad)?).map_err(|_| bad)?;
        let data_at = at.checked_add(8).ok_or(bad)?;
        let data = self.rd.b.get(data_at..data_at.checked_add(size).ok_or(bad)?).ok_or(bad)?;
        let next = round_up(data_at.checked_add(size).ok_or(bad)?, 8).ok_or(bad)?;
        self.off = next.min(self.rd.b.len());
        Ok(Property { pr_type, data })
    }
}

impl<'a> Iterator for Properties<'a> {
    type Item = Result<Property<'a>, ElfError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.off >= self.rd.b.len() {
            return None;
        }
        let r = self.step();
        self.done = r.is_err();
        Some(r)
    }
}

// ---------------------------------------------------------------------------------------------------
// Part 2 (DL-3): hash tables, symbol versions, relocation iterators. Error rules: decision D-27.8.

/// `n * size` as a `u64` byte count, checked.
fn bytes_for(n: u64, size: usize) -> Option<u64> {
    n.checked_mul(u64::try_from(size).ok()?)
}

/// The tags an object's lookup view reads ([`Tags`]), each with its bit in [`Tags::seen`].
const VIEW_TAGS: [u64; 9] = [dt::HASH, dt::GNU_HASH, dt::SYMTAB, dt::SYMENT, dt::VERSYM, dt::VERDEF, dt::VERDEFNUM, dt::VERNEED, dt::VERNEEDNUM];

/// The values of the [`VIEW_TAGS`] in one dynamic section, found in one pass over its entries
/// (LD-OBJECT-ONCE: a lookup view asked for a dozen tags, and each request walked every entry). The
/// answers are [`Dynamic::once`]'s: the value of the only entry with the tag, `None` without one,
/// [`ElfError::DuplicateDynamic`] when two entries have it.
#[derive(Clone, Copy, Debug)]
pub struct Tags {
    val: [u64; VIEW_TAGS.len()],
    /// Bit `i`: an entry has tag `VIEW_TAGS[i]`.
    seen: u16,
    /// Bit `i`: two or more entries have it.
    dup: u16,
}

impl Tags {
    /// The tags of `d`.
    #[must_use]
    pub fn scan(d: &Dynamic<'_>) -> Tags {
        let mut t = Tags { val: [0; VIEW_TAGS.len()], seen: 0, dup: 0 };
        for e in d.entries() {
            let Some(i) = VIEW_TAGS.iter().position(|&v| v == e.tag) else { continue };
            let bit = 1u16.checked_shl(u32::try_from(i).unwrap_or(16)).unwrap_or(0);
            if t.seen & bit != 0 {
                t.dup |= bit;
            } else if let Some(v) = t.val.get_mut(i) {
                t.seen |= bit;
                *v = e.val;
            }
        }
        t
    }

    /// [`Dynamic::once`] for `tag`, one of the [`VIEW_TAGS`] (another tag is looked for in `d`).
    fn once(&self, d: &Dynamic<'_>, tag: u64) -> Result<Option<u64>, ElfError> {
        let Some(i) = VIEW_TAGS.iter().position(|&v| v == tag) else { return d.once(tag) };
        let bit = 1u16.checked_shl(u32::try_from(i).unwrap_or(16)).unwrap_or(0);
        if self.dup & bit != 0 {
            return Err(ElfError::DuplicateDynamic(tag));
        }
        Ok(if self.seen & bit != 0 { self.val.get(i).copied() } else { None })
    }
}

/// An object's hash tables, each read once: [`Dynamic::sysv_hash`] and [`Dynamic::gnu_hash`] as
/// they answered (errors included), for the requests that read them again.
#[derive(Clone, Copy, Debug)]
pub struct Hashes<'a> {
    pub(crate) sysv: Result<Option<SysvHash<'a>>, ElfError>,
    pub(crate) gnu: Result<Option<GnuHash<'a>>, ElfError>,
}

impl<'a> Hashes<'a> {
    /// [`Dynamic::symbol_count`] from these tables.
    pub(crate) fn symbol_count(&self) -> Result<Option<u32>, ElfError> {
        if let Some(h) = self.sysv? {
            return Ok(Some(h.nchain()));
        }
        self.gnu?.map(|g| g.symbol_count()).transpose()
    }
}

impl<'a> Dynamic<'a> {
    /// The value of tag `tag`, which may occur at most once.
    fn once(&self, tag: u64) -> Result<Option<u64>, ElfError> {
        let mut it = self.entries().filter(|e| e.tag == tag);
        let first = it.next().map(|e| e.val);
        if it.next().is_some() {
            return Err(ElfError::DuplicateDynamic(tag));
        }
        Ok(first)
    }

    /// The tags a lookup view reads, in one pass ([`Tags`]). Public, as [`Dynamic::hashes`],
    /// [`Tags::scan`] and `symbols::Object::build`: a crate-local function called from another code
    /// unit is emitted with a `.llvm.<hash>` suffix that changes with unrelated code (the installation
    /// prefix), and the names in the interpreter's hot-text trace must not.
    #[must_use]
    pub fn tags(&self) -> Tags {
        Tags::scan(self)
    }

    /// Both hash tables, each read once, from the tags `t`.
    #[must_use]
    pub fn hashes(&self, t: &Tags) -> Hashes<'a> {
        Hashes { sysv: self.sysv_hash_t(t), gnu: self.gnu_hash_t(t) }
    }

    /// An (address, size or count) pair of tags: both or neither.
    fn pair(&self, addr: u64, size: u64) -> Result<Option<(u64, u64)>, ElfError> {
        both(addr, size, self.once(addr)?, self.once(size)?)
    }

    /// [`Dynamic::pair`] from the tags `t`.
    fn pair_t(&self, t: &Tags, addr: u64, size: u64) -> Result<Option<(u64, u64)>, ElfError> {
        both(addr, size, t.once(self, addr)?, t.once(self, size)?)
    }
}

/// The values `a` and `s` of the tags `addr` and `size` as a pair ([`Dynamic::pair`]).
fn both(addr: u64, size: u64, a: Option<u64>, s: Option<u64>) -> Result<Option<(u64, u64)>, ElfError> {
    match (a, s) {
        (Some(a), Some(s)) => Ok(Some((a, s))),
        (None, None) => Ok(None),
        (Some(_), None) => Err(ElfError::MissingDynamic(size)),
        (None, Some(_)) => Err(ElfError::MissingDynamic(addr)),
    }
}

impl<'a> Dynamic<'a> {
    /// The SysV hash table (`DT_HASH`): `nbucket`, `nchain`, then the buckets and chains (32-bit words).
    ///
    /// # Errors
    ///
    /// [`ElfError::DuplicateDynamic`], or the [`Elf::bytes_at`] errors when the header or the
    /// `nbucket + nchain` words are not wholly mapped.
    pub fn sysv_hash(&self) -> Result<Option<SysvHash<'a>>, ElfError> {
        self.sysv_hash_at(self.once(dt::HASH)?)
    }

    /// [`Dynamic::sysv_hash`] from the tags `t`.
    fn sysv_hash_t(&self, t: &Tags) -> Result<Option<SysvHash<'a>>, ElfError> {
        self.sysv_hash_at(t.once(self, dt::HASH)?)
    }

    /// [`Dynamic::sysv_hash`] given its `DT_HASH` value.
    fn sysv_hash_at(&self, addr: Option<u64>) -> Result<Option<SysvHash<'a>>, ElfError> {
        let Some(addr) = addr else { return Ok(None) };
        let be = self.rd.be;
        let head = Rd { b: self.elf.bytes_at(addr, 8)?, be };
        let (Some(nbucket), Some(nchain)) = (head.u32(0), head.u32(4)) else { return Err(ElfError::AddressNotMapped(addr)) };
        let len = bytes_for(u64::from(nbucket).saturating_add(u64::from(nchain)), 4)
            .and_then(|n| n.checked_add(8))
            .ok_or(ElfError::AddressNotMapped(addr))?;
        let all = self.elf.bytes_at(addr, len)?;
        Ok(Some(SysvHash { rd: Rd { b: all, be }, nbucket, nchain, mark: 0 }))
    }

    /// The GNU hash table (`DT_GNU_HASH`): `nbuckets`, `symoffset`, `bloom_size`, `bloom_shift`, the
    /// 64-bit Bloom words, the buckets, then the chain, which runs to the end of the mapped bytes.
    ///
    /// # Errors
    ///
    /// [`ElfError::DuplicateDynamic`], [`ElfError::BadHashHeader`] (no Bloom words, or a shift of 64
    /// or more), or the [`Elf::bytes_at`] errors when the header, Bloom words or buckets are not mapped.
    pub fn gnu_hash(&self) -> Result<Option<GnuHash<'a>>, ElfError> {
        self.gnu_hash_at(self.once(dt::GNU_HASH)?)
    }

    /// [`Dynamic::gnu_hash`] from the tags `t`.
    fn gnu_hash_t(&self, t: &Tags) -> Result<Option<GnuHash<'a>>, ElfError> {
        self.gnu_hash_at(t.once(self, dt::GNU_HASH)?)
    }

    /// [`Dynamic::gnu_hash`] given its `DT_GNU_HASH` value.
    fn gnu_hash_at(&self, addr: Option<u64>) -> Result<Option<GnuHash<'a>>, ElfError> {
        let Some(addr) = addr else { return Ok(None) };
        let be = self.rd.be;
        let head = Rd { b: self.elf.bytes_at(addr, 16)?, be };
        let field = |o| head.u32(o).ok_or(ElfError::AddressNotMapped(addr));
        let (nbuckets, symoffset, bloom_size, bloom_shift) = (field(0)?, field(4)?, field(8)?, field(12)?);
        if bloom_size == 0 || bloom_shift >= 64 {
            return Err(ElfError::BadHashHeader(Table::GnuHash));
        }
        let unmapped = ElfError::AddressNotMapped(addr);
        let bloom_len = bytes_for(u64::from(bloom_size), 8).ok_or(unmapped)?;
        let bucket_len = bytes_for(u64::from(nbuckets), 4).ok_or(unmapped)?;
        let fixed = bloom_len.checked_add(bucket_len).and_then(|n| n.checked_add(16)).ok_or(unmapped)?;
        let all = self.elf.bytes_at(addr, fixed)?;
        let chain = self.elf.bytes_from(addr.checked_add(fixed).ok_or(unmapped)?)?;
        // One slice from the first Bloom word to the end of the chain's bytes, so the table (and
        // every lookup view holding one) is a third of the size of three slices. The chain is the
        // rest of the segment holding the buckets, so the three parts are contiguous; a table whose
        // chain lies elsewhere (overlapping or unsorted `PT_LOAD`s) is refused.
        let whole = self.elf.bytes_from(addr)?;
        let rest = whole.get(usize::try_from(fixed).map_err(|_| unmapped)?..).ok_or(unmapped)?;
        if rest.as_ptr() != chain.as_ptr() || rest.len() != chain.len() || whole.as_ptr() != all.as_ptr() {
            return Err(ElfError::BadHashHeader(Table::GnuHash));
        }
        let tab = whole.get(16..).ok_or(unmapped)?;
        let chain_at = u32::try_from(bloom_len.checked_add(bucket_len).ok_or(unmapped)?).map_err(|_| unmapped)?;
        Ok(Some(GnuHash { tab, be, bloom_size, chain_at, nbuckets, symoffset, bloom_shift, mark: 0 }))
    }

    /// The number of dynamic symbols: the SysV table's `nchain` when there is one (exact), else the
    /// symbols the GNU table covers (exact when the hashed symbols come last, as linkers place them; an
    /// object whose buckets are all empty gives `symoffset`, a lower bound); `None` without either.
    ///
    /// # Errors
    ///
    /// The errors of [`Dynamic::sysv_hash`], [`Dynamic::gnu_hash`] and [`GnuHash::symbol_count`].
    pub fn symbol_count(&self) -> Result<Option<u32>, ElfError> {
        self.hashes(&self.tags()).symbol_count()
    }

    /// The dynamic symbol table (`DT_SYMTAB`; `DT_SYMENT`, when present, must be 24). Its length is not
    /// recorded, so an entry is read wherever it lies in the mapped bytes, and an index at or past the
    /// [`Dynamic::symbol_count`] hint is refused.
    ///
    /// # Errors
    ///
    /// [`ElfError::DuplicateDynamic`], [`ElfError::BadDynamicValue`]`(DT_SYMENT)`, the [`Elf::bytes_from`]
    /// errors for the address and the [`Dynamic::symbol_count`] errors.
    pub fn symtab(&self) -> Result<Option<Symtab<'a>>, ElfError> {
        let t = self.tags();
        let h = self.hashes(&t);
        self.symtab_with(&t, &h, h.symbol_count())
    }

    /// [`Dynamic::symtab`] from the tags `t` and hash tables `h`, given the [`Dynamic::symbol_count`]
    /// result its caller already has (a lookup view reads the symbol and version tables together and
    /// counts once: the count reads every bucket). The count's error is met where
    /// [`Dynamic::symtab`] meets it, after the tags.
    pub(crate) fn symtab_with(&self, t: &Tags, h: &Hashes<'a>, count: Result<Option<u32>, ElfError>) -> Result<Option<Symtab<'a>>, ElfError> {
        let Some(addr) = t.once(self, dt::SYMTAB)? else { return Ok(None) };
        if let Some(e) = t.once(self, dt::SYMENT)? {
            if usize::try_from(e).ok() != Some(SYM_SIZE) {
                return Err(ElfError::BadDynamicValue(dt::SYMENT));
            }
        }
        let count = self.symtab_count(h, count?)?;
        let b = self.elf.bytes_from(addr)?;
        Ok(Some(Symtab { rd: Rd { b, be: self.rd.be }, count }))
    }

    /// The count [`Dynamic::symtab`] checks an index against: [`Dynamic::symbol_count`], except when
    /// that is only a lower bound (no SysV table and a GNU table whose buckets are all empty: the object
    /// hashes no symbol, and its unhashed symbols need not end at `symoffset`, as linkers emit for a
    /// program that exports nothing). Then it is raised to cover every symbol a RELA or `JMPREL` entry
    /// names, the only indices read past the hashed ones. Objects that hash a symbol pay nothing.
    fn symtab_count(&self, h: &Hashes<'a>, count: Option<u32>) -> Result<Option<u32>, ElfError> {
        let lower_bound = h.sysv?.is_none() && h.gnu?.is_some_and(|g| (0..g.nbuckets()).all(|i| g.bucket(i) == Some(0)));
        let Some(mut count) = count.filter(|_| lower_bound) else { return Ok(count) };
        for (addr, size) in [(dt::RELA, dt::RELASZ), (dt::JMPREL, dt::PLTRELSZ)] {
            let Some((at, len)) = self.pair(addr, size)? else { continue };
            let rd = Rd { b: self.elf.bytes_at(at, len)?, be: self.rd.be };
            // r_info is the second word of each entry; its high half is the symbol index.
            let mut off = 8usize;
            while let Some(info) = rd.u64(off) {
                count = count.max(u32::try_from(info >> 32).unwrap_or(u32::MAX).saturating_add(1));
                off = off.saturating_add(RELA_SIZE);
            }
        }
        Ok(Some(count))
    }

    /// The symbol version table (`DT_VERSYM`): one 16-bit entry per dynamic symbol. The table's length is
    /// not recorded, so an entry is read wherever it lies in the mapped bytes; [`Versym::count`] is the
    /// [`Dynamic::symbol_count`] hint. Every entry is checked against the highest version index the
    /// definitions and requirements give (walked here, names included).
    ///
    /// # Errors
    ///
    /// The [`Elf::bytes_from`] errors for the address, the [`Dynamic::symbol_count`] errors, and the
    /// errors of the version walks.
    pub fn versym(&self) -> Result<Option<Versym<'a>>, ElfError> {
        let t = self.tags();
        self.versym_with(&t, self.hashes(&t).symbol_count())
    }

    /// [`Dynamic::versym`] from the tags `t`, given the [`Dynamic::symbol_count`] result (see
    /// [`Dynamic::symtab_with`]).
    pub(crate) fn versym_with(&self, t: &Tags, count: Result<Option<u32>, ElfError>) -> Result<Option<Versym<'a>>, ElfError> {
        let Some(addr) = t.once(self, dt::VERSYM)? else { return Ok(None) };
        let count = count?;
        let tab = self.elf.bytes_from(addr)?;
        Ok(self.versym_versions(t, tab, count)?.map(|(v, _)| v))
    }

    /// [`Dynamic::versym`] from the tags `t` and the table's bytes `tab`, and the object's
    /// [`Dynamic::versions`] (`None` when a count does not fit 32 bits), read in the same walk
    /// (LD-OBJECT-ONCE: the lookup view found the two tables twice).
    pub(crate) fn versym_versions(&self, t: &Tags, tab: &'a [u8], count: Option<u32>) -> Result<Option<(Versym<'a>, Option<Versions<'a>>)>, ElfError> {
        let mut v = Some(Versions { def: &[], need: &[], ndef: 0, nneed: 0 });
        let mut max = 1u16;
        if let Some((addr, num)) = self.pair_t(t, dt::VERDEF, dt::VERDEFNUM)? {
            let b = self.elf.bytes_from(addr)?;
            max = max.max(self.check_verdefs(b, num)?);
            v = v.zip(u32::try_from(num).ok()).map(|(v, n)| Versions { def: b, ndef: n, ..v });
        }
        if let Some((addr, num)) = self.pair_t(t, dt::VERNEED, dt::VERNEEDNUM)? {
            let b = self.elf.bytes_from(addr)?;
            max = max.max(self.check_verneeds(b, num)?);
            v = v.zip(u32::try_from(num).ok()).map(|(v, n)| Versions { need: b, nneed: n, ..v });
        }
        Ok(Some((Versym { rd: Rd { b: tab, be: self.rd.be }, count, max }, v)))
    }

    /// [`Dynamic::versym_versions`] for the tags `t` and the count `count`; `None` without `DT_VERSYM`.
    ///
    /// # Errors
    ///
    /// As [`Dynamic::versym`].
    pub(crate) fn versym_with_versions(&self, t: &Tags, count: Result<Option<u32>, ElfError>) -> Result<Option<(Versym<'a>, Option<Versions<'a>>)>, ElfError> {
        let Some(addr) = t.once(self, dt::VERSYM)? else { return Ok(None) };
        let count = count?;
        let tab = self.elf.bytes_from(addr)?;
        self.versym_versions(t, tab, count)
    }

    /// Every check a walk of [`Verdefs`] and each definition's [`Verdef::names`] makes over `num`
    /// definitions in `b`, in the same order with the same errors, without making the entries; the
    /// highest index (hidden bit cleared) the definitions give, at least 1.
    fn check_verdefs(&self, b: &'a [u8], num: u64) -> Result<u16, ElfError> {
        let be = self.rd.be;
        let (h, w) = (|e: &[u8], o: usize| field16(e, o, be), |e: &[u8], o: usize| field32(e, o, be));
        let (mut at, mut left, mut max) = (0usize, num, 1u16);
        while left != 0 {
            let bad = ElfError::VersionTruncated(Table::Verdef, at);
            let e = b.get(at..).and_then(<[u8]>::first_chunk::<VERDEF_SIZE>).ok_or(bad)?;
            if h(e, 0) != VER_CURRENT {
                return Err(ElfError::BadVersionRevision(Table::Verdef, h(e, 0)));
            }
            let (ndx, cnt, aux, next) = (h(e, 4), h(e, 6), w(e, 12), w(e, 16));
            left = left.saturating_sub(1);
            let def = at;
            if left > 0 {
                if next == 0 {
                    return Err(ElfError::VersionCount(Table::Verdef, def));
                }
                at = advance(def, next).ok_or(bad)?;
            }
            let mut a = advance(def, aux);
            for k in (0..cnt).rev() {
                let n = a.ok_or(ElfError::VersionTruncated(Table::Verdef, def))?;
                let bad = ElfError::VersionTruncated(Table::Verdef, n);
                let e = b.get(n..).and_then(<[u8]>::first_chunk::<8>).ok_or(bad)?;
                if k > 0 {
                    if w(e, 4) == 0 {
                        return Err(ElfError::VersionCount(Table::Verdef, def));
                    }
                    a = Some(advance(n, w(e, 4)).ok_or(bad)?);
                }
                self.check_string(u64::from(w(e, 0)))?;
            }
            max = max.max(ndx & !VERSYM_HIDDEN);
        }
        Ok(max)
    }

    /// [`Dynamic::check_verdefs`] for [`Verneeds`] and each requirement's [`Verneed::auxes`]: the
    /// highest `vna_other` (hidden bit cleared), at least 1.
    fn check_verneeds(&self, b: &'a [u8], num: u64) -> Result<u16, ElfError> {
        let be = self.rd.be;
        let (h, w) = (|e: &[u8], o: usize| field16(e, o, be), |e: &[u8], o: usize| field32(e, o, be));
        let (mut at, mut left, mut max) = (0usize, num, 1u16);
        while left != 0 {
            let bad = ElfError::VersionTruncated(Table::Verneed, at);
            let e = b.get(at..).and_then(<[u8]>::first_chunk::<16>).ok_or(bad)?;
            if h(e, 0) != VER_CURRENT {
                return Err(ElfError::BadVersionRevision(Table::Verneed, h(e, 0)));
            }
            let (cnt, file, aux, next) = (h(e, 2), w(e, 4), w(e, 8), w(e, 12));
            left = left.saturating_sub(1);
            let need = at;
            if left > 0 {
                if next == 0 {
                    return Err(ElfError::VersionCount(Table::Verneed, need));
                }
                at = advance(need, next).ok_or(bad)?;
            }
            self.check_string(u64::from(file))?;
            let mut a = advance(need, aux);
            for k in (0..cnt).rev() {
                let n = a.ok_or(ElfError::VersionTruncated(Table::Verneed, need))?;
                let bad = ElfError::VersionTruncated(Table::Verneed, n);
                let e = b.get(n..).and_then(<[u8]>::first_chunk::<16>).ok_or(bad)?;
                if k > 0 {
                    if w(e, 12) == 0 {
                        return Err(ElfError::VersionCount(Table::Verneed, need));
                    }
                    a = Some(advance(n, w(e, 12)).ok_or(bad)?);
                }
                self.check_string(u64::from(w(e, 8)))?;
                max = max.max(h(e, 6) & !VERSYM_HIDDEN);
            }
        }
        Ok(max)
    }

    /// The checks of [`Dynamic::string`] at `off`, without finding the string's end when the table's
    /// last byte is a NUL (then every offset in it starts a terminated string).
    fn check_string(&self, off: u64) -> Result<(), ElfError> {
        let tab = self.strtab.ok_or(ElfError::MissingStrtab)?;
        if tab.last() == Some(&0) && usize::try_from(off).is_ok_and(|o| o < tab.len()) {
            return Ok(());
        }
        self.string(off).map(|_| ())
    }

    /// The version definitions (`DT_VERDEF`, `DT_VERDEFNUM` entries).
    ///
    /// # Errors
    ///
    /// [`ElfError::MissingDynamic`] unless both tags are present, the [`Elf::bytes_from`] errors for
    /// the address; each entry's own errors come from the iterator.
    pub fn verdef(&self) -> Result<Option<Verdefs<'a>>, ElfError> {
        let Some((addr, num)) = self.pair(dt::VERDEF, dt::VERDEFNUM)? else { return Ok(None) };
        let b = self.elf.bytes_from(addr)?;
        Ok(Some(Verdefs { dynm: *self, rd: Rd { b, be: self.rd.be }, off: 0, left: num, done: false }))
    }

    /// The version requirements (`DT_VERNEED`, `DT_VERNEEDNUM` entries).
    ///
    /// # Errors
    ///
    /// As [`Dynamic::verdef`].
    pub fn verneed(&self) -> Result<Option<Verneeds<'a>>, ElfError> {
        let Some((addr, num)) = self.pair(dt::VERNEED, dt::VERNEEDNUM)? else { return Ok(None) };
        let b = self.elf.bytes_from(addr)?;
        Ok(Some(Verneeds { dynm: *self, rd: Rd { b, be: self.rd.be }, off: 0, left: num, done: false }))
    }

    /// The version tables found once, for an object whose [`Dynamic::versym`] succeeded (that walk
    /// read every definition, requirement and name), so a lookup by index needs neither the dynamic
    /// entries nor the program headers again. `None` when a count does not fit 32 bits.
    ///
    /// # Errors
    ///
    /// As [`Dynamic::verdef`] and [`Dynamic::verneed`] (none after a successful `versym`).
    pub fn versions(&self) -> Result<Option<Versions<'a>>, ElfError> {
        let mut v = Versions { def: &[], need: &[], ndef: 0, nneed: 0 };
        if let Some((addr, num)) = self.pair(dt::VERDEF, dt::VERDEFNUM)? {
            let Ok(n) = u32::try_from(num) else { return Ok(None) };
            (v.def, v.ndef) = (self.elf.bytes_from(addr)?, n);
        }
        if let Some((addr, num)) = self.pair(dt::VERNEED, dt::VERNEEDNUM)? {
            let Ok(n) = u32::try_from(num) else { return Ok(None) };
            (v.need, v.nneed) = (self.elf.bytes_from(addr)?, n);
        }
        Ok(Some(v))
    }
}

/// The 16-bit field at `o` of the record `e` (0 past its end), byte order big-endian when `be`.
fn field16(e: &[u8], o: usize, be: bool) -> u16 {
    let b = e.get(o..).and_then(<[u8]>::first_chunk::<2>).copied().unwrap_or_default();
    if be { u16::from_be_bytes(b) } else { u16::from_le_bytes(b) }
}

/// The 32-bit field at `o` of the record `e` (0 past its end), as [`field16`].
fn field32(e: &[u8], o: usize, be: bool) -> u32 {
    let b = e.get(o..).and_then(<[u8]>::first_chunk::<4>).copied().unwrap_or_default();
    if be { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) }
}

/// An object's version definitions and requirements ([`Dynamic::versions`]), read by index with the
/// same answers as walking [`Verdefs`] and [`Verneeds`]: the first entry with the index wins.
#[derive(Clone, Copy, Debug)]
pub struct Versions<'a> {
    def: &'a [u8],
    need: &'a [u8],
    ndef: u32,
    nneed: u32,
}

impl<'a> Versions<'a> {
    /// The first name of the first definition whose `vd_ndx` (hidden bit cleared) is `index`
    /// (`None` when none has it, or when it has no names), from `dynm`'s string table.
    ///
    /// # Errors
    ///
    /// The string table's errors, or [`ElfError::VersionTruncated`] for an entry past the bytes.
    pub fn def_name(&self, dynm: &Dynamic<'a>, index: u16) -> Result<Option<&'a [u8]>, ElfError> {
        let at = if dynm.rd.be { self.def_entry::<true>(index) } else { self.def_entry::<false>(index) };
        match at {
            Some(Some(at)) => self.def_name_from(dynm, index, at),
            Some(None) => Ok(None),
            None => self.def_name_from(dynm, index, 0),
        }
    }

    /// Where the first definition with index `index` starts in the definitions' bytes
    /// (`Some(None)`: no definition has it), each entry read as one 20-byte record in the byte
    /// order `BE` (DYN-START-SPEED: the field-by-field walk, with its byte-order select and range
    /// check per field, was a sixth of a start). `None` when an entry is not whole or a link
    /// leaves the bytes: [`Versions::def_name_from`] then walks from the start and gives the same
    /// answer, errors included.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn def_entry<const BE: bool>(&self, index: u16) -> Option<Option<usize>> {
        let mut at = 0usize;
        let mut left = self.ndef;
        while left != 0 {
            let e = self.def.get(at..)?.first_chunk::<VERDEF_SIZE>()?;
            let ndx = if BE { u16::from_be_bytes([e[4], e[5]]) } else { u16::from_le_bytes([e[4], e[5]]) };
            if ndx & !VERSYM_HIDDEN == index {
                return Some(Some(at));
            }
            left = left.saturating_sub(1);
            let next = if BE { u32::from_be_bytes([e[16], e[17], e[18], e[19]]) } else { u32::from_le_bytes([e[16], e[17], e[18], e[19]]) };
            at = advance(at, next)?;
        }
        Some(None)
    }

    /// [`Versions::def_name`] walking the definitions from the one at `at`, field by field.
    fn def_name_from(&self, dynm: &Dynamic<'a>, index: u16, start: usize) -> Result<Option<&'a [u8]>, ElfError> {
        let rd = Rd { b: self.def, be: dynm.rd.be };
        // From an entry the scan found: its index matches, so the first round answers.
        let mut at = start;
        for k in 0..self.ndef {
            let bad = ElfError::VersionTruncated(Table::Verdef, at);
            let f16 = |o: usize| at.checked_add(o).and_then(|x| rd.u16(x)).ok_or(bad);
            if f16(4)? & !VERSYM_HIDDEN == index {
                if f16(6)? == 0 {
                    return Ok(None);
                }
                let aux = advance(at, rd.u32(at.checked_add(12).ok_or(bad)?).ok_or(bad)?).ok_or(bad)?;
                let bad = ElfError::VersionTruncated(Table::Verdef, aux);
                return dynm.string(u64::from(rd.u32(aux).ok_or(bad)?)).map(Some);
            }
            if k.saturating_add(1) < self.ndef {
                at = advance(at, rd.u32(at.checked_add(16).ok_or(bad)?).ok_or(bad)?).ok_or(bad)?;
            }
        }
        Ok(None)
    }

    /// The name of the first requirement whose `vna_other` (hidden bit cleared) is `index`
    /// (`None` when none has it), from `dynm`'s string table.
    ///
    /// # Errors
    ///
    /// The string table's errors, or [`ElfError::VersionTruncated`] for an entry past the bytes.
    pub fn need_name(&self, dynm: &Dynamic<'a>, index: u16) -> Result<Option<&'a [u8]>, ElfError> {
        let rd = Rd { b: self.need, be: dynm.rd.be };
        let mut at = 0usize;
        for k in 0..self.nneed {
            let bad = ElfError::VersionTruncated(Table::Verneed, at);
            let cnt = rd.u16(at.checked_add(2).ok_or(bad)?).ok_or(bad)?;
            let mut a = advance(at, rd.u32(at.checked_add(8).ok_or(bad)?).ok_or(bad)?).ok_or(bad)?;
            for j in 0..cnt {
                let bad = ElfError::VersionTruncated(Table::Verneed, a);
                if rd.u16(a.checked_add(6).ok_or(bad)?).ok_or(bad)? & !VERSYM_HIDDEN == index {
                    return dynm.string(u64::from(rd.u32(a.checked_add(8).ok_or(bad)?).ok_or(bad)?)).map(Some);
                }
                if j.saturating_add(1) < cnt {
                    a = advance(a, rd.u32(a.checked_add(12).ok_or(bad)?).ok_or(bad)?).ok_or(bad)?;
                }
            }
            if k.saturating_add(1) < self.nneed {
                at = advance(at, rd.u32(at.checked_add(12).ok_or(bad)?).ok_or(bad)?).ok_or(bad)?;
            }
        }
        Ok(None)
    }
}

impl<'a> Dynamic<'a> {

    /// A RELA table at `addr` of `size` bytes (entry size already checked).
    fn rela_table<A: Arch>(&self, table: Table, addr: u64, size: u64) -> Result<Relas<'a, A>, ElfError> {
        let ent = u64::try_from(RELA_SIZE).map_err(|_| ElfError::TableSize(table))?;
        if size.checked_rem(ent) != Some(0) {
            return Err(ElfError::TableSize(table));
        }
        Ok(Relas { rd: Rd { b: self.elf.bytes_at(addr, size)?, be: self.rd.be }, off: 0, arch: PhantomData })
    }

    /// The RELA relocations (`DT_RELA`, `DT_RELASZ`, `DT_RELAENT` = 24). The whole table is checked here,
    /// so the iterator cannot fail; each entry's type is classified by `A`.
    ///
    /// # Errors
    ///
    /// [`ElfError::MissingDynamic`], [`ElfError::BadDynamicValue`]`(DT_RELAENT)`, [`ElfError::TableSize`],
    /// or the [`Elf::bytes_at`] errors.
    pub fn rela<A: Arch>(&self) -> Result<Option<Relas<'a, A>>, ElfError> {
        let Some((addr, size)) = self.pair(dt::RELA, dt::RELASZ)? else { return Ok(None) };
        match self.once(dt::RELAENT)? {
            None => return Err(ElfError::MissingDynamic(dt::RELAENT)),
            Some(e) if usize::try_from(e).ok() != Some(RELA_SIZE) => return Err(ElfError::BadDynamicValue(dt::RELAENT)),
            Some(_) => {}
        }
        self.rela_table(Table::Rela, addr, size).map(Some)
    }

    /// The PLT relocations (`DT_JMPREL`, `DT_PLTRELSZ`; `DT_PLTREL` must be `DT_RELA`).
    ///
    /// # Errors
    ///
    /// [`ElfError::MissingDynamic`], [`ElfError::BadDynamicValue`]`(DT_PLTREL)`, [`ElfError::TableSize`],
    /// or the [`Elf::bytes_at`] errors.
    pub fn jmprel<A: Arch>(&self) -> Result<Option<Relas<'a, A>>, ElfError> {
        let Some((addr, size)) = self.pair(dt::JMPREL, dt::PLTRELSZ)? else { return Ok(None) };
        match self.once(dt::PLTREL)? {
            None => return Err(ElfError::MissingDynamic(dt::PLTREL)),
            Some(dt::RELA) => {}
            Some(_) => return Err(ElfError::BadDynamicValue(dt::PLTREL)),
        }
        self.rela_table(Table::Jmprel, addr, size).map(Some)
    }

    /// The RELR relocations (`DT_RELR`, `DT_RELRSZ`, `DT_RELRENT` = 8), decoded into the addresses they
    /// name.
    ///
    /// # Errors
    ///
    /// [`ElfError::MissingDynamic`], [`ElfError::BadDynamicValue`]`(DT_RELRENT)`, [`ElfError::TableSize`],
    /// or the [`Elf::bytes_at`] errors; each entry's own errors come from the iterator.
    pub fn relr(&self) -> Result<Option<Relrs<'a>>, ElfError> {
        let Some((addr, size)) = self.pair(dt::RELR, dt::RELRSZ)? else { return Ok(None) };
        match self.once(dt::RELRENT)? {
            None => return Err(ElfError::MissingDynamic(dt::RELRENT)),
            Some(e) if usize::try_from(e).ok() != Some(RELR_SIZE) => return Err(ElfError::BadDynamicValue(dt::RELRENT)),
            Some(_) => {}
        }
        let ent = u64::try_from(RELR_SIZE).map_err(|_| ElfError::TableSize(Table::Relr))?;
        if size.checked_rem(ent) != Some(0) {
            return Err(ElfError::TableSize(Table::Relr));
        }
        let b = self.elf.bytes_at(addr, size)?;
        Ok(Some(Relrs { rd: Rd { b, be: self.rd.be }, idx: 0, base: None, bits: 0, bit_base: 0, bit_idx: 0, done: false }))
    }
}

/// The SysV hash table: `nbucket` buckets and `nchain` chain words, one per dynamic symbol.
#[derive(Clone, Copy, Debug)]
pub struct SysvHash<'a> {
    rd: Rd<'a>,
    nbucket: u32,
    nchain: u32,
    /// [`GnuHash`]'s `mark`, for a SysV table (a `u8` for the same reason).
    mark: u8,
}

impl<'a> SysvHash<'a> {
    /// Sets the mark to `on` ([`GnuHash::set_mark`]).
    pub(crate) fn set_mark(&mut self, on: bool) {
        self.mark = u8::from(on);
    }

    /// The mark ([`GnuHash::mark`]).
    #[must_use]
    pub(crate) fn mark(&self) -> bool {
        self.mark != 0
    }

    /// The number of buckets.
    #[must_use]
    pub fn nbucket(&self) -> u32 {
        self.nbucket
    }

    /// The number of chain words, which is the number of dynamic symbols.
    #[must_use]
    pub fn nchain(&self) -> u32 {
        self.nchain
    }

    fn word(&self, i: u64) -> Option<u32> {
        self.rd.u32(usize::try_from(i.checked_mul(4)?.checked_add(8)?).ok()?)
    }

    /// Bucket `i` (`None` past `nbucket`).
    #[must_use]
    pub fn bucket(&self, i: u32) -> Option<u32> {
        if i < self.nbucket { self.word(u64::from(i)) } else { None }
    }

    /// The symbol indices of the chain for hash value `hash` (bucket `hash % nbucket`), in chain order.
    /// The iterator yields one error and then stops; a table without buckets yields nothing.
    #[must_use]
    pub fn chain(&self, hash: u32) -> SysvChain<'a> {
        let start = hash.checked_rem(self.nbucket).and_then(|b| self.bucket(b)).unwrap_or(0);
        SysvChain { h: *self, next: start, steps: 0, done: false }
    }
}

/// The symbol indices of one SysV hash chain.
#[derive(Clone, Debug)]
pub struct SysvChain<'a> {
    h: SysvHash<'a>,
    next: u32,
    steps: u32,
    done: bool,
}

impl Iterator for SysvChain<'_> {
    type Item = Result<u32, ElfError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.next == 0 {
            return None;
        }
        let i = self.next;
        let r = if i >= self.h.nchain {
            Err(ElfError::HashIndex(Table::SysvHash, i))
        } else if self.steps >= self.h.nchain {
            Err(ElfError::HashChainLoop(i))
        } else {
            self.steps = self.steps.saturating_add(1);
            match self.h.word(u64::from(self.h.nbucket).saturating_add(u64::from(i))) {
                Some(n) => {
                    self.next = n;
                    Ok(i)
                }
                None => Err(ElfError::HashIndex(Table::SysvHash, i)),
            }
        };
        self.done = r.is_err();
        Some(r)
    }
}

/// The GNU hash table. Symbols below `symoffset` are not hashed; the chain word of symbol `i`
/// (`i >= symoffset`) is its hash with bit 0 replaced by "last in chain".
#[derive(Clone, Copy, Debug)]
pub struct GnuHash<'a> {
    /// The Bloom words, the buckets, then the chain words to the end of the mapped bytes. The slice
    /// and its byte order are kept apart (not an `Rd`), so they pack with the counts below and the
    /// lookup view keeps its size with its version tables ([`Versions`]).
    tab: &'a [u8],
    be: bool,
    /// The number of Bloom words (at least 1).
    bloom_size: u32,
    /// Byte offset of the chain in `tab` (past the Bloom words and the buckets).
    chain_at: u32,
    nbuckets: u32,
    symoffset: u32,
    bloom_shift: u32,
    /// One bit the user of the table keeps with it (the lookup view's "the program has canonical
    /// PLT entries", `symbols::Object::mark_program`); false as read. It sits in the struct's tail
    /// padding, so the table and every view that holds one keep their size. A `u8` (0 or 1), not a
    /// `bool`: a `bool` has invalid values the compiler would take for the discriminant of an enum
    /// holding the table (`symbols::Hash`), moving it off the byte it uses without the mark and
    /// changing the symbol walk's code.
    mark: u8,
}

/// One entry of a GNU hash chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GnuEntry {
    /// The symbol index.
    pub sym: u32,
    /// The chain word: the symbol's hash, bit 0 set on the chain's last entry.
    pub hash: u32,
}

impl<'a> GnuHash<'a> {
    /// Sets the mark to `on`.
    pub(crate) fn set_mark(&mut self, on: bool) {
        self.mark = u8::from(on);
    }

    /// The mark: false as read, else what [`GnuHash::set_mark`] set.
    #[must_use]
    pub(crate) fn mark(&self) -> bool {
        self.mark != 0
    }

    /// The number of buckets.
    #[must_use]
    pub fn nbuckets(&self) -> u32 {
        self.nbuckets
    }

    /// The index of the first hashed symbol.
    #[must_use]
    pub fn symoffset(&self) -> u32 {
        self.symoffset
    }

    /// The number of 64-bit Bloom words (at least 1).
    #[must_use]
    pub fn bloom_size(&self) -> usize {
        usize::try_from(self.bloom_size).unwrap_or(0)
    }

    /// The second Bloom shift (below 64).
    #[must_use]
    pub fn bloom_shift(&self) -> u32 {
        self.bloom_shift
    }

    /// Bloom word `i` (`None` past the last).
    #[must_use]
    pub fn bloom_word(&self, i: usize) -> Option<u64> {
        if i >= self.bloom_size() {
            return None;
        }
        self.rd().u64(i.checked_mul(8)?)
    }

    fn rd(&self) -> Rd<'a> {
        Rd { b: self.tab, be: self.be }
    }

    /// Bucket `i`: the first symbol of its chain, 0 for an empty bucket (`None` past `nbuckets`).
    #[must_use]
    pub fn bucket(&self, i: u32) -> Option<u32> {
        if i >= self.nbuckets {
            return None;
        }
        let at = usize::try_from(i).ok()?.checked_mul(4)?.checked_add(self.bloom_size().checked_mul(8)?)?;
        self.rd().u32(at)
    }

    /// The chain word of symbol `sym`.
    fn chain_word(&self, sym: u32) -> Result<u32, ElfError> {
        let bad = ElfError::HashIndex(Table::GnuHash, sym);
        let rel = sym.checked_sub(self.symoffset).ok_or(bad)?;
        let at = usize::try_from(rel).map_err(|_| bad)?.checked_mul(4).and_then(|r| r.checked_add(usize::try_from(self.chain_at).ok()?)).ok_or(bad)?;
        self.rd().u32(at).ok_or(bad)
    }

    /// The chain for hash value `hash` (bucket `hash % nbuckets`), in order. The iterator yields one
    /// error and then stops; an empty bucket or a table without buckets yields nothing.
    #[must_use]
    pub fn chain(&self, hash: u32) -> GnuChain<'a> {
        let start = hash.checked_rem(self.nbuckets).and_then(|b| self.bucket(b)).unwrap_or(0);
        GnuChain { h: *self, next: start, done: start == 0 }
    }

    /// The symbols a lookup of the name hashing to `hash` must look at: nothing when the Bloom word
    /// rejects the hash, else each entry of its chain whose hash matches (bit 0 aside), in order, given
    /// to `f` until it answers true. The same entries and errors as testing the Bloom word, then
    /// [`GnuHash::chain`] filtered by hash: a Bloom word past the table is
    /// [`ElfError::BadHashHeader`], a chain word past the mapped bytes [`ElfError::HashIndex`]. One
    /// loop over the chain's bytes, without the iterator's state (LD-LOOKUP-COST: every object ahead
    /// of the definer in the scope pays this walk on every lookup).
    ///
    /// # Errors
    ///
    /// Those above, and the first error of `f`.
    #[inline]
    pub(crate) fn candidates<E: From<ElfError>>(&self, hash: u32, mut f: impl FnMut(u32) -> Result<bool, E>) -> Result<(), E> {
        let wi = usize::try_from(hash.wrapping_shr(6)).ok().and_then(|w| w.checked_rem(self.bloom_size())).unwrap_or(0);
        let word = self.bloom_word(wi).ok_or(ElfError::BadHashHeader(Table::GnuHash))?;
        let second = u64::from(hash).checked_shr(self.bloom_shift).unwrap_or(0) & 63;
        let mask = 1u64.wrapping_shl(hash & 63) | 1u64.wrapping_shl(u32::try_from(second).unwrap_or(0));
        if word & mask != mask {
            return Ok(());
        }
        let mut sym = hash.checked_rem(self.nbuckets).and_then(|b| self.bucket(b)).unwrap_or(0);
        if sym == 0 {
            return Ok(());
        }
        let chain = usize::try_from(self.chain_at).ok().and_then(|c| self.tab.get(c..)).unwrap_or(&[]);
        let Some(rel) = sym.checked_sub(self.symoffset) else { return Err(ElfError::HashIndex(Table::GnuHash, sym).into()) };
        let mut at = usize::try_from(rel).ok().and_then(|r| r.checked_mul(4)).ok_or(ElfError::HashIndex(Table::GnuHash, sym))?;
        loop {
            let Some(w) = chain.get(at..).and_then(<[u8]>::first_chunk::<4>) else { return Err(ElfError::HashIndex(Table::GnuHash, sym).into()) };
            let word = if self.be { u32::from_be_bytes(*w) } else { u32::from_le_bytes(*w) };
            let last = word & 1 == 1;
            if !last && sym == u32::MAX {
                return Err(ElfError::HashIndex(Table::GnuHash, sym).into());
            }
            if (word | 1) == (hash | 1) && f(sym)? {
                return Ok(());
            }
            if last {
                return Ok(());
            }
            sym = sym.wrapping_add(1);
            at = at.checked_add(4).ok_or(ElfError::HashIndex(Table::GnuHash, sym))?;
        }
    }

    /// The number of dynamic symbols the table covers: one past the end of the chain that starts at
    /// the highest bucket, or `symoffset` when every bucket is empty.
    ///
    /// # Errors
    ///
    /// [`ElfError::HashIndex`] for a bucket below `symoffset` or a chain that runs past the mapped bytes.
    pub fn symbol_count(&self) -> Result<u32, ElfError> {
        // The buckets as one slice, scanned in one branch-free pass for the highest bucket and the
        // lowest non-empty one (LD-OBJECT-ONCE: the per-bucket reads, each with its own range
        // checks, were a tenth of a dynamic start for the C library's table). A bucket below
        // `symoffset` is found again by the checked walk, which names the first one in order.
        let top = match self.bucket_bytes() {
            Some(b) if self.be => bucket_range::<true>(b, self.symoffset),
            Some(b) => bucket_range::<false>(b, self.symoffset),
            None => None,
        };
        let top = match top {
            Some(top) => top,
            None => self.bucket_top_checked()?,
        };
        if top == 0 {
            return Ok(self.symoffset);
        }
        let mut last = top;
        for e in (GnuChain { h: *self, next: top, done: false }) {
            last = e?.sym;
        }
        last.checked_add(1).ok_or(ElfError::HashIndex(Table::GnuHash, last))
    }

    /// The bucket words' bytes (`nbuckets` 4-byte words past the Bloom words); `None` when they do
    /// not fit `tab` (never for a table [`Dynamic::gnu_hash`] made).
    fn bucket_bytes(&self) -> Option<&'a [u8]> {
        let start = self.bloom_size().checked_mul(8)?;
        let len = usize::try_from(self.nbuckets).ok()?.checked_mul(4)?;
        self.tab.get(start..start.checked_add(len)?)
    }

    /// The highest bucket, bucket by bucket: the first bucket below `symoffset` (not empty) is
    /// [`ElfError::HashIndex`].
    #[cold]
    #[inline(never)]
    fn bucket_top_checked(&self) -> Result<u32, ElfError> {
        let mut top = 0u32;
        for i in 0..self.nbuckets {
            let b = self.bucket(i).ok_or(ElfError::HashIndex(Table::GnuHash, i))?;
            if b != 0 && b < self.symoffset {
                return Err(ElfError::HashIndex(Table::GnuHash, b));
            }
            top = top.max(b);
        }
        Ok(top)
    }
}

/// The highest of the bucket words in `b` (byte order `BE`), or `None` when a non-empty bucket lies
/// below `symoffset` (the caller's checked walk then names it). One pass with no branch per bucket:
/// the highest word, and the lowest non-empty one (an empty bucket, 0, wraps to the largest value
/// when one is subtracted, so it never is the lowest).
fn bucket_range<const BE: bool>(b: &[u8], symoffset: u32) -> Option<u32> {
    let (mut top, mut low) = (0u32, u32::MAX);
    for w in b.chunks_exact(4) {
        let w: [u8; 4] = w.try_into().unwrap_or_default();
        let v = if BE { u32::from_be_bytes(w) } else { u32::from_le_bytes(w) };
        top = top.max(v);
        low = low.min(v.wrapping_sub(1));
    }
    // `low` is (lowest non-empty bucket) - 1, or u32::MAX when every bucket is empty; a non-empty
    // bucket below `symoffset` is one with `low < symoffset - 1`.
    (low >= symoffset.saturating_sub(1)).then_some(top)
}

/// The entries of one GNU hash chain.
#[derive(Clone, Debug)]
pub struct GnuChain<'a> {
    h: GnuHash<'a>,
    next: u32,
    done: bool,
}

impl Iterator for GnuChain<'_> {
    type Item = Result<GnuEntry, ElfError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let sym = self.next;
        let r = self.h.chain_word(sym).and_then(|hash| {
            if hash & 1 == 1 {
                self.done = true;
            } else {
                self.next = sym.checked_add(1).ok_or(ElfError::HashIndex(Table::GnuHash, sym))?;
            }
            Ok(GnuEntry { sym, hash })
        });
        if r.is_err() {
            self.done = true;
        }
        Some(r)
    }
}

/// A `versym` entry: a version index and the hidden bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionIndex {
    /// The index: 0 local, 1 global (base), otherwise a `vd_ndx` or `vna_other`.
    pub index: u16,
    /// The symbol's version is hidden (not the default version).
    pub hidden: bool,
}

/// The symbol version table.
#[derive(Clone, Copy, Debug)]
pub struct Versym<'a> {
    rd: Rd<'a>,
    count: Option<u32>,
    max: u16,
}

impl Versym<'_> {
    /// The dynamic symbol count from the hash tables (see [`Dynamic::symbol_count`]), if known.
    #[must_use]
    pub fn count(&self) -> Option<u32> {
        self.count
    }

    /// The highest version index a definition or requirement gives (at least 1).
    #[must_use]
    pub fn max_index(&self) -> u16 {
        self.max
    }

    /// The version of symbol `sym`.
    ///
    /// # Errors
    ///
    /// [`ElfError::SymbolIndex`] for an entry past the mapped bytes, [`ElfError::VersionIndex`] for an
    /// index above [`Versym::max_index`].
    pub fn get(&self, sym: u32) -> Result<VersionIndex, ElfError> {
        let off = usize::try_from(sym).ok().and_then(|s| s.checked_mul(2));
        let raw = off.and_then(|o| self.rd.u16(o)).ok_or(ElfError::SymbolIndex(sym))?;
        let index = raw & !VERSYM_HIDDEN;
        if index > self.max {
            return Err(ElfError::VersionIndex(sym, raw));
        }
        Ok(VersionIndex { index, hidden: raw & VERSYM_HIDDEN != 0 })
    }
}

/// `STB_LOCAL`.
pub const STB_LOCAL: u8 = 0;
/// `STB_GLOBAL`.
pub const STB_GLOBAL: u8 = 1;
/// `STB_WEAK`.
pub const STB_WEAK: u8 = 2;
/// `STB_GNU_UNIQUE`: one definition per namespace.
pub const STB_GNU_UNIQUE: u8 = 10;
/// `STT_NOTYPE`.
pub const STT_NOTYPE: u8 = 0;
/// `STT_OBJECT`.
pub const STT_OBJECT: u8 = 1;
/// `STT_FUNC`.
pub const STT_FUNC: u8 = 2;
/// `STT_COMMON`.
pub const STT_COMMON: u8 = 5;
/// `STT_TLS`.
pub const STT_TLS: u8 = 6;
/// `STT_GNU_IFUNC`: the value is a resolver, called at bind time.
pub const STT_GNU_IFUNC: u8 = 10;
/// `STV_DEFAULT`.
pub const STV_DEFAULT: u8 = 0;
/// `STV_INTERNAL`.
pub const STV_INTERNAL: u8 = 1;
/// `STV_HIDDEN`.
pub const STV_HIDDEN: u8 = 2;
/// `STV_PROTECTED`.
pub const STV_PROTECTED: u8 = 3;
/// `SHN_UNDEF`: the symbol is not defined in this object.
pub const SHN_UNDEF: u16 = 0;

/// One dynamic symbol (`Elf64_Sym`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sym {
    /// `st_name`: offset of the name in the string table.
    pub name: u32,
    /// `st_info`: binding (high four bits) and type (low four bits).
    pub info: u8,
    /// `st_other`: the visibility in the low two bits.
    pub other: u8,
    /// `st_shndx`: [`SHN_UNDEF`] for an undefined symbol.
    pub shndx: u16,
    /// `st_value`.
    pub value: u64,
    /// `st_size`.
    pub size: u64,
}

impl Sym {
    /// The binding (`STB_*`).
    #[must_use]
    pub fn bind(&self) -> u8 {
        self.info >> 4
    }
    /// The type (`STT_*`).
    #[must_use]
    pub fn kind(&self) -> u8 {
        self.info & 0xf
    }
    /// The visibility (`STV_*`).
    #[must_use]
    pub fn visibility(&self) -> u8 {
        self.other & 3
    }
}

/// The dynamic symbol table.
#[derive(Clone, Copy, Debug)]
pub struct Symtab<'a> {
    rd: Rd<'a>,
    count: Option<u32>,
}

impl Symtab<'_> {
    /// The dynamic symbol count from the hash tables (see [`Dynamic::symbol_count`]), if known.
    #[must_use]
    pub fn count(&self) -> Option<u32> {
        self.count
    }

    /// Symbol `i`.
    ///
    /// # Errors
    ///
    /// [`ElfError::SymbolIndex`] for an index at or past [`Symtab::count`] or an entry past the mapped bytes.
    pub fn get(&self, i: u32) -> Result<Sym, ElfError> {
        let bad = ElfError::SymbolIndex(i);
        if self.count.is_some_and(|c| i >= c) {
            return Err(bad);
        }
        // One bounds check for the whole entry, then fixed-offset fields (LD-LOOKUP-COST: a lookup
        // reads a symbol per hash candidate and per relocation).
        let at = usize::try_from(i).ok().and_then(|n| n.checked_mul(SYM_SIZE)).ok_or(bad)?;
        let e = self.rd.b.get(at..).and_then(<[u8]>::first_chunk::<SYM_SIZE>).ok_or(bad)?;
        Ok(Sym::decode(e, self.rd.be))
    }

    /// The symbols, in index order, whose range in an object loaded at `bias` holds `addr`: `start =
    /// bias + st_value` (wrapping) with `start <= addr` and `addr - start < st_size`, or `addr ==
    /// start`. Each entry's value and size are read first and the whole symbol is decoded only when
    /// its range holds `addr` (`dladdr` scans every symbol of an object). The entries are those
    /// [`Symtab::get`] reads: below [`Symtab::count`] (none when it is unknown) and within the mapped
    /// bytes.
    pub fn holding(&self, bias: u64, addr: u64) -> impl Iterator<Item = (u32, Sym)> + '_ {
        let n = self.count.and_then(|c| usize::try_from(c).ok()).unwrap_or(0);
        let bytes = self.rd.b.get(..n.saturating_mul(SYM_SIZE).min(self.rd.b.len())).unwrap_or(&[]);
        let be = self.rd.be;
        let w8 = move |a: [u8; 8]| if be { u64::from_be_bytes(a) } else { u64::from_le_bytes(a) };
        bytes.chunks_exact(SYM_SIZE).zip(0u32..).filter_map(move |(e, k)| {
            let e = e.first_chunk::<SYM_SIZE>()?;
            let (v, s) = e.split_at(8).1.split_at(8);
            let start = bias.wrapping_add(w8(*v.first_chunk::<8>()?));
            let off = addr.checked_sub(start)?;
            if off >= w8(*s.first_chunk::<8>()?) && off != 0 {
                return None;
            }
            Some((k, Sym::decode(e, be)))
        })
    }
}

impl Sym {
    /// The symbol in the 24 bytes of an `Elf64_Sym` in byte order `be`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn decode(e: &[u8; SYM_SIZE], be: bool) -> Sym {
        let w4 = |a: [u8; 4]| if be { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) };
        let w8 = |a: [u8; 8]| if be { u64::from_be_bytes(a) } else { u64::from_le_bytes(a) };
        let shndx = if be { u16::from_be_bytes([e[6], e[7]]) } else { u16::from_le_bytes([e[6], e[7]]) };
        let (name, value, size) = (w4([e[0], e[1], e[2], e[3]]), w8([e[8], e[9], e[10], e[11], e[12], e[13], e[14], e[15]]), w8([e[16], e[17], e[18], e[19], e[20], e[21], e[22], e[23]]));
        Sym { name, info: e[4], other: e[5], shndx, value, size }
    }
}

/// `off + rel` as a new offset in a version table, checked (`None` on overflow).
fn advance(off: usize, rel: u32) -> Option<usize> {
    off.checked_add(usize::try_from(rel).ok()?)
}

/// One version definition (`Elf64_Verdef`).
#[derive(Clone, Copy, Debug)]
pub struct Verdef<'a> {
    /// Offset of the entry in the table.
    pub offset: usize,
    /// `vd_flags` (`VER_FLG_BASE` 1, `VER_FLG_WEAK` 2).
    pub flags: u16,
    /// `vd_ndx`: the version index `versym` entries use.
    pub ndx: u16,
    /// `vd_cnt`: the number of names (the version, then its parents).
    pub cnt: u16,
    /// `vd_hash`.
    pub hash: u32,
    aux: u32,
    dynm: Dynamic<'a>,
    rd: Rd<'a>,
}

impl<'a> Verdef<'a> {
    /// The names (`Elf64_Verdaux`): the version's own name, then its parents. Yields one error and stops.
    #[must_use]
    pub fn names(&self) -> Verdauxes<'a> {
        Verdauxes { dynm: self.dynm, rd: self.rd, at: self.offset, off: advance(self.offset, self.aux), left: self.cnt, done: false }
    }
}

/// The version definitions; yields one error and then stops.
#[derive(Clone, Debug)]
pub struct Verdefs<'a> {
    dynm: Dynamic<'a>,
    rd: Rd<'a>,
    off: usize,
    left: u64,
    done: bool,
}

impl<'a> Verdefs<'a> {
    fn step(&mut self) -> Result<Verdef<'a>, ElfError> {
        let at = self.off;
        let bad = ElfError::VersionTruncated(Table::Verdef, at);
        let f16 = |o: usize| self.rd.u16(at.checked_add(o)?);
        let f32 = |o: usize| self.rd.u32(at.checked_add(o)?);
        let version = f16(0).ok_or(bad)?;
        let (flags, ndx, cnt) = (f16(2).ok_or(bad)?, f16(4).ok_or(bad)?, f16(6).ok_or(bad)?);
        let (hash, aux, next) = (f32(8).ok_or(bad)?, f32(12).ok_or(bad)?, f32(16).ok_or(bad)?);
        if version != VER_CURRENT {
            return Err(ElfError::BadVersionRevision(Table::Verdef, version));
        }
        self.left = self.left.saturating_sub(1);
        if self.left > 0 {
            if next == 0 {
                return Err(ElfError::VersionCount(Table::Verdef, at));
            }
            self.off = advance(at, next).ok_or(bad)?;
        }
        Ok(Verdef { offset: at, flags, ndx, cnt, hash, aux, dynm: self.dynm, rd: self.rd })
    }
}

impl Verdefs<'_> {
    /// Whether a walk of these definitions from the start meets no error before it reaches one
    /// whose `vd_hash` is `hash` and whose first name is `name`: true only when walking them
    /// ([`Iterator::next`], [`Verdef::names`]) would find a definition named `name` with no error
    /// first. False tells nothing (no such entry, or a malformed one ahead of it): the caller walks.
    /// The hash is compared first, so the names of the other definitions are never read
    /// (LD-OBJECT-ONCE: the walk read every definition's name up to the one required).
    #[must_use]
    pub fn has_hashed(&self, hash: u32, name: &[u8]) -> bool {
        let be = self.rd.be;
        let b = self.rd.b;
        let (mut at, mut left) = (self.off, self.left);
        while left != 0 {
            let Some(e) = b.get(at..).and_then(<[u8]>::first_chunk::<VERDEF_SIZE>) else { return false };
            if field16(e, 0, be) != VER_CURRENT {
                return false;
            }
            left = left.saturating_sub(1);
            let def = at;
            if left > 0 {
                let next = field32(e, 16, be);
                let Some(n) = advance(def, next).filter(|_| next != 0) else { return false };
                at = n;
            }
            if field32(e, 8, be) == hash && field16(e, 6, be) != 0 && self.first_name_is(def, e, name) {
                return true;
            }
        }
        false
    }

    /// Whether the first name of the definition at `def` (entry `e`) reads without error and is
    /// `name` ([`Verdauxes`]' first step).
    fn first_name_is(&self, def: usize, e: &[u8; VERDEF_SIZE], name: &[u8]) -> bool {
        let be = self.rd.be;
        let Some(a) = advance(def, field32(e, 12, be)) else { return false };
        let Some(x) = self.rd.b.get(a..).and_then(<[u8]>::first_chunk::<8>) else { return false };
        if field16(e, 6, be) > 1 && (field32(x, 4, be) == 0 || advance(a, field32(x, 4, be)).is_none()) {
            return false;
        }
        self.dynm.string(u64::from(field32(x, 0, be))).is_ok_and(|s| s == name)
    }
}

impl<'a> Iterator for Verdefs<'a> {
    type Item = Result<Verdef<'a>, ElfError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.left == 0 {
            return None;
        }
        let r = self.step();
        self.done = r.is_err();
        Some(r)
    }
}

/// One name of a version definition (`Elf64_Verdaux`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verdaux<'a> {
    /// Offset of the entry in the table.
    pub offset: usize,
    /// The name.
    pub name: &'a [u8],
}

/// The names of one version definition; yields one error and then stops.
#[derive(Clone, Debug)]
pub struct Verdauxes<'a> {
    dynm: Dynamic<'a>,
    rd: Rd<'a>,
    /// Offset of the owning definition (for errors).
    at: usize,
    off: Option<usize>,
    left: u16,
    done: bool,
}

impl<'a> Verdauxes<'a> {
    fn step(&mut self) -> Result<Verdaux<'a>, ElfError> {
        let at = self.off.ok_or(ElfError::VersionTruncated(Table::Verdef, self.at))?;
        let bad = ElfError::VersionTruncated(Table::Verdef, at);
        let name = self.rd.u32(at).ok_or(bad)?;
        let next = self.rd.u32(at.checked_add(4).ok_or(bad)?).ok_or(bad)?;
        self.left = self.left.saturating_sub(1);
        if self.left > 0 {
            if next == 0 {
                return Err(ElfError::VersionCount(Table::Verdef, self.at));
            }
            self.off = Some(advance(at, next).ok_or(bad)?);
        }
        Ok(Verdaux { offset: at, name: self.dynm.string(u64::from(name))? })
    }
}

impl<'a> Iterator for Verdauxes<'a> {
    type Item = Result<Verdaux<'a>, ElfError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.left == 0 {
            return None;
        }
        let r = self.step();
        self.done = r.is_err();
        Some(r)
    }
}

/// One version requirement (`Elf64_Verneed`): a needed file and the versions required of it.
#[derive(Clone, Copy, Debug)]
pub struct Verneed<'a> {
    /// Offset of the entry in the table.
    pub offset: usize,
    /// `vn_cnt`: the number of required versions.
    pub cnt: u16,
    /// The file name (`vn_file`).
    pub file: &'a [u8],
    aux: u32,
    dynm: Dynamic<'a>,
    rd: Rd<'a>,
}

impl<'a> Verneed<'a> {
    /// The required versions (`Elf64_Vernaux`). Yields one error and stops.
    #[must_use]
    pub fn auxes(&self) -> Vernauxes<'a> {
        Vernauxes { dynm: self.dynm, rd: self.rd, at: self.offset, off: advance(self.offset, self.aux), left: self.cnt, done: false }
    }
}

/// The version requirements; yields one error and then stops.
#[derive(Clone, Debug)]
pub struct Verneeds<'a> {
    dynm: Dynamic<'a>,
    rd: Rd<'a>,
    off: usize,
    left: u64,
    done: bool,
}

impl<'a> Verneeds<'a> {
    fn step(&mut self) -> Result<Verneed<'a>, ElfError> {
        let at = self.off;
        let bad = ElfError::VersionTruncated(Table::Verneed, at);
        let f32 = |o: usize| self.rd.u32(at.checked_add(o)?);
        let version = self.rd.u16(at).ok_or(bad)?;
        let cnt = self.rd.u16(at.checked_add(2).ok_or(bad)?).ok_or(bad)?;
        let (file, aux, next) = (f32(4).ok_or(bad)?, f32(8).ok_or(bad)?, f32(12).ok_or(bad)?);
        if version != VER_CURRENT {
            return Err(ElfError::BadVersionRevision(Table::Verneed, version));
        }
        self.left = self.left.saturating_sub(1);
        if self.left > 0 {
            if next == 0 {
                return Err(ElfError::VersionCount(Table::Verneed, at));
            }
            self.off = advance(at, next).ok_or(bad)?;
        }
        Ok(Verneed { offset: at, cnt, file: self.dynm.string(u64::from(file))?, aux, dynm: self.dynm, rd: self.rd })
    }
}

impl<'a> Iterator for Verneeds<'a> {
    type Item = Result<Verneed<'a>, ElfError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.left == 0 {
            return None;
        }
        let r = self.step();
        self.done = r.is_err();
        Some(r)
    }
}

/// One required version (`Elf64_Vernaux`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vernaux<'a> {
    /// Offset of the entry in the table.
    pub offset: usize,
    /// `vna_hash`.
    pub hash: u32,
    /// `vna_flags` (`VER_FLG_WEAK` 2).
    pub flags: u16,
    /// `vna_other`: the version index `versym` entries use.
    pub other: u16,
    /// The version name.
    pub name: &'a [u8],
}

/// The required versions of one file; yields one error and then stops.
#[derive(Clone, Debug)]
pub struct Vernauxes<'a> {
    dynm: Dynamic<'a>,
    rd: Rd<'a>,
    at: usize,
    off: Option<usize>,
    left: u16,
    done: bool,
}

impl<'a> Vernauxes<'a> {
    fn step(&mut self) -> Result<Vernaux<'a>, ElfError> {
        let at = self.off.ok_or(ElfError::VersionTruncated(Table::Verneed, self.at))?;
        let bad = ElfError::VersionTruncated(Table::Verneed, at);
        let f32 = |o: usize| self.rd.u32(at.checked_add(o)?);
        let hash = self.rd.u32(at).ok_or(bad)?;
        let flags = self.rd.u16(at.checked_add(4).ok_or(bad)?).ok_or(bad)?;
        let other = self.rd.u16(at.checked_add(6).ok_or(bad)?).ok_or(bad)?;
        let (name, next) = (f32(8).ok_or(bad)?, f32(12).ok_or(bad)?);
        self.left = self.left.saturating_sub(1);
        if self.left > 0 {
            if next == 0 {
                return Err(ElfError::VersionCount(Table::Verneed, self.at));
            }
            self.off = Some(advance(at, next).ok_or(bad)?);
        }
        Ok(Vernaux { offset: at, hash, flags, other, name: self.dynm.string(u64::from(name))? })
    }
}

impl<'a> Iterator for Vernauxes<'a> {
    type Item = Result<Vernaux<'a>, ElfError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.left == 0 {
            return None;
        }
        let r = self.step();
        self.done = r.is_err();
        Some(r)
    }
}

/// One RELA relocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rela {
    /// `r_offset`: the address of the field to relocate.
    pub offset: u64,
    /// The symbol index (`r_info >> 32`).
    pub sym: u32,
    /// The raw type (`r_info & 0xffff_ffff`), for diagnostics only: code matches on `kind`.
    pub r_type: u32,
    /// The type as the architecture classifies it (an unknown type is `Unsupported`).
    pub kind: RelocKind,
    /// `r_addend`.
    pub addend: i64,
}

/// The entries of a RELA table, in order. The table was range-checked when made, so items cannot fail.
#[derive(Clone, Debug)]
pub struct Relas<'a, A: Arch> {
    rd: Rd<'a>,
    off: usize,
    arch: PhantomData<A>,
}

impl<'a, A: Arch> Relas<'a, A> {
    /// The number of entries in the table.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.rd.b.len().checked_div(RELA_SIZE).unwrap_or(0)
    }

    /// Calls `f(r_offset, r_addend)` for the leading run of `RELATIVE` entries (as `A` classifies
    /// them), at most `n` of them (a table's `DT_RELACOUNT` prefix), and leaves the iterator after
    /// the run: an entry of another type ends it there, and the iterator then yields that entry as
    /// usual, so a wrong count changes no result. The byte order is chosen once for the run, not per
    /// field (DYN-START-SPEED: the per-entry decode was most of a start's relocation pass).
    ///
    /// # Errors
    ///
    /// The first error of `f`; the iterator then stands on the entry `f` failed for.
    // Inlined into the pass that owns `f`: one call per table.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn relative_prefix<E>(&mut self, n: usize, f: impl FnMut(u64, i64) -> Result<(), E>) -> Result<(), E> {
        if self.rd.be { self.relative_run::<true, E>(n, f) } else { self.relative_run::<false, E>(n, f) }
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn relative_run<const BE: bool, E>(&mut self, n: usize, mut f: impl FnMut(u64, i64) -> Result<(), E>) -> Result<(), E> {
        let word = |b: &[u8; 8]| if BE { u64::from_be_bytes(*b) } else { u64::from_le_bytes(*b) };
        let rest = self.rd.b.get(self.off..).unwrap_or(&[]);
        // The entries the run accepted, added to the position once at the end (DYN-START-LINK-SHAPE:
        // a saturating update per entry was a sixth of the loop).
        let mut done = 0usize;
        let mut out = Ok(());
        for e in rest.chunks_exact(RELA_SIZE).take(n) {
            let Some((offset, e)) = e.split_first_chunk::<8>() else { break };
            let Some((info, e)) = e.split_first_chunk::<8>() else { break };
            let Some((addend, _)) = e.split_first_chunk::<8>() else { break };
            let r_type = u32::try_from(word(info) & 0xffff_ffff).unwrap_or(u32::MAX);
            if A::classify(r_type) != RelocKind::Relative {
                break;
            }
            if let Err(x) = f(word(offset), i64::from_ne_bytes(word(addend).to_ne_bytes())) {
                out = Err(x);
                break;
            }
            done = done.wrapping_add(1);
        }
        // `done` entries of `RELA_SIZE` bytes lie inside the table: no overflow.
        self.off = self.off.saturating_add(done.saturating_mul(RELA_SIZE));
        out
    }
}

impl<A: Arch> Iterator for Relas<'_, A> {
    type Item = Rela;
    // Inlined into each pass (one call per relocation otherwise; the start-up pass is the hot one).
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn next(&mut self) -> Option<Rela> {
        let at = self.off;
        let offset = self.rd.u64(at)?;
        let info = self.rd.u64(at.checked_add(8)?)?;
        let addend = i64::from_ne_bytes(self.rd.u64(at.checked_add(16)?)?.to_ne_bytes());
        self.off = at.checked_add(RELA_SIZE)?;
        let r_type = u32::try_from(info & 0xffff_ffff).ok()?;
        let sym = u32::try_from(info.checked_shr(32)?).ok()?;
        Some(Rela { offset, sym, r_type, kind: A::classify(r_type), addend })
    }

    /// The entry `n` places on, without decoding the ones before it (a pass that wants only a
    /// table's tail, after its `DT_RELACOUNT` prefix, starts there).
    fn nth(&mut self, n: usize) -> Option<Rela> {
        self.off = n.checked_mul(RELA_SIZE).and_then(|b| self.off.checked_add(b)).unwrap_or(usize::MAX);
        self.next()
    }
}

/// The bytes one RELR bitmap covers: 63 words of 8 bytes; the next bitmap starts after them.
const RELR_BITMAP_SPAN: u64 = 504;

/// The addresses a RELR table names, in order: an even entry is an address, an odd entry a bitmap of
/// the 63 words after the previous address or bitmap. Yields one error and then stops.
#[derive(Clone, Debug)]
pub struct Relrs<'a> {
    rd: Rd<'a>,
    idx: usize,
    /// Where the next bitmap's first bit points (`None`: no address entry yet; `Err`: past the address space).
    base: Option<Result<u64, ElfError>>,
    /// The current bitmap's remaining bits (bit 0 = `bit_base`).
    bits: u64,
    bit_base: u64,
    /// Entry index of the current bitmap (for errors).
    bit_idx: usize,
    done: bool,
}

impl Relrs<'_> {
    fn step(&mut self) -> Option<Result<u64, ElfError>> {
        loop {
            if self.bits != 0 {
                let k = self.bits.trailing_zeros();
                self.bits &= self.bits.wrapping_sub(1);
                let addr = u64::from(k).checked_mul(8).and_then(|d| self.bit_base.checked_add(d));
                return Some(addr.ok_or(ElfError::RelrOverflow(self.bit_idx)));
            }
            let i = self.idx;
            let e = self.rd.u64(i.checked_mul(RELR_SIZE)?)?;
            self.idx = i.checked_add(1)?;
            if e & 1 == 0 {
                self.base = Some(e.checked_add(8).ok_or(ElfError::RelrOverflow(i)));
                return Some(Ok(e));
            }
            let base = match self.base {
                None => return Some(Err(ElfError::RelrNoBase(i))),
                Some(Err(_)) => return Some(Err(ElfError::RelrOverflow(i))),
                Some(Ok(b)) => b,
            };
            self.bits = e.checked_shr(1).unwrap_or(0);
            self.bit_base = base;
            self.bit_idx = i;
            self.base = Some(base.checked_add(RELR_BITMAP_SPAN).ok_or(ElfError::RelrOverflow(i)));
        }
    }
}

impl Iterator for Relrs<'_> {
    type Item = Result<u64, ElfError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let r = self.step()?;
        self.done = r.is_err();
        Some(r)
    }
}
