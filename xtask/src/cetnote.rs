//! The shadow-stack property of every shipped object (D-X.164, SHSTK-OPT-IN).
//!
//! An x86-64 object says that its code is shadow-stack-correct (no `ret` to an address other than
//! its caller's) with `GNU_PROPERTY_X86_FEATURE_1_AND` (0xc0000002) bit SHSTK (2) in a
//! `.note.gnu.property` note (x86-64 psABI, "Program Property"; Linux gABI extensions). The link
//! editor ANDs the bit over its inputs, so one unmarked input leaves a program or library
//! unmarked, and the C library enables the shadow stack only for a process whose objects all
//! have it.
//!
//! The pinned stable rustc has no flag that emits the note, so `cargo xtask so` adds it:
//! * every ELF member of a published archive without the note gets one ([`mark_archive`], from
//!   [`crate::so::publish`]): a `.note.gnu.property` section holding FEATURE_1_AND = SHSTK only
//!   (IBT is not claimed: Rust code has no `endbr64` at its indirect-branch targets);
//! * the linked products are linked with `-z shstk` ([`LINK_FLAG`]);
//! * [`check_release`] runs after the last product: every ELF file and every archive member the
//!   packages ship (the `file` rows of [`SHIPPED_MANIFEST`], `tools/deb/manifest.tsv`) has the
//!   bit, or the build fails. What else lands in `target/niji` (test programs, the Rust dylibs of
//!   `cargo xtask rust-dylib`, the by-path copy of libc.so.6) is not shipped and not checked.
//!
//! What makes the claim true is the audit of the hand-written assembly (D-21.5: setjmp/longjmp,
//! the context functions and vfork handle the shadow stack; the rest keeps call and return
//! paired). An existing note without SHSTK is an error, never overwritten: that object's code
//! has to be looked at.

use std::fs;
use std::path::Path;

/// `GNU_PROPERTY_X86_FEATURE_1_AND`.
const FEATURE_1_AND: u32 = 0xc000_0002;
/// Its SHSTK bit.
pub const SHSTK: u32 = 2;
/// The link-editor option that sets SHSTK in a linked output whatever its inputs carry.
pub const LINK_FLAG: &str = "-Wl,-z,shstk";
const SECTION: &[u8] = b".note.gnu.property";
const SHT_NOTE: u32 = 7;
const SHF_ALLOC: u64 = 2;

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    d.get(at..at.checked_add(2)?).map(|b| u16::from_le_bytes([b[0], b[1]]))
}
fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at.checked_add(4)?).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn u64_at(d: &[u8], at: usize) -> Option<u64> {
    d.get(at..at.checked_add(8)?).map(|b| u64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
}
fn us(v: u64) -> Option<usize> {
    usize::try_from(v).ok()
}

/// One section header: (name offset, type, flags, file offset, size).
fn shdr(obj: &[u8], shoff: usize, i: usize) -> Option<(u32, u32, u64, usize, usize)> {
    let h = shoff.checked_add(i.checked_mul(64)?)?;
    Some((u32_at(obj, h)?, u32_at(obj, h + 4)?, u64_at(obj, h + 8)?, us(u64_at(obj, h + 24)?)?, us(u64_at(obj, h + 32)?)?))
}

/// The SHSTK-relevant bits of an `NT_GNU_PROPERTY_TYPE_0` note area (8-byte aligned on ELFCLASS64):
/// the AND of every FEATURE_1_AND found, 0 for a note without one.
fn note_bits(notes: &[u8]) -> Result<u32, String> {
    let (mut at, mut bits) = (0usize, None::<u32>);
    while at + 12 <= notes.len() {
        let (namesz, descsz, ty) = (u32_at(notes, at).unwrap_or(0) as usize, u32_at(notes, at + 4).unwrap_or(0) as usize, u32_at(notes, at + 8).unwrap_or(0));
        let name = at + 12;
        let desc = (name + namesz + 7) & !7;
        let end = (desc + descsz + 7) & !7;
        if desc + descsz > notes.len() {
            return Err("truncated note".into());
        }
        if ty == 5 && notes.get(name..name + namesz) == Some(b"GNU\0") {
            let mut p = desc;
            let mut found = 0;
            while p + 8 <= desc + descsz {
                let (pt, sz) = (u32_at(notes, p).unwrap_or(0), u32_at(notes, p + 4).unwrap_or(0) as usize);
                if pt == FEATURE_1_AND && sz == 4 {
                    found = u32_at(notes, p + 8).unwrap_or(0);
                }
                p = (p + 8 + sz + 7) & !7;
            }
            bits = Some(bits.unwrap_or(u32::MAX) & found);
        }
        at = end;
    }
    Ok(bits.unwrap_or(0))
}

/// The FEATURE_1_AND bits of an ELF64 little-endian file: from its `PT_GNU_PROPERTY` segment
/// (a linked file) or its `.note.gnu.property` sections (a relocatable object). `Ok(None)`: no
/// property note at all; `Err`: not such an ELF file, or a malformed one.
pub fn feature_bits(obj: &[u8]) -> Result<Option<u32>, String> {
    if !obj.starts_with(b"\x7fELF\x02\x01") {
        return Err("not an ELF64 little-endian file".into());
    }
    let ty = u16_at(obj, 16).ok_or("short header")?;
    if ty != 1 {
        // Linked: the PT_GNU_PROPERTY segment (0x6474e553).
        let (phoff, phnum) = (us(u64_at(obj, 32).ok_or("short header")?).ok_or("phoff")?, usize::from(u16_at(obj, 56).ok_or("short header")?));
        for i in 0..phnum {
            let h = phoff + i * 56;
            if u32_at(obj, h) == Some(0x6474_e553) {
                let (off, size) = (us(u64_at(obj, h + 8).ok_or("phdr")?).ok_or("phdr")?, us(u64_at(obj, h + 32).ok_or("phdr")?).ok_or("phdr")?);
                return note_bits(obj.get(off..off + size).ok_or("PT_GNU_PROPERTY past the end")?).map(Some);
            }
        }
        return Ok(None);
    }
    let (shoff, shnum, shstrndx) = (us(u64_at(obj, 40).ok_or("short header")?).ok_or("shoff")?, usize::from(u16_at(obj, 60).ok_or("short header")?), usize::from(u16_at(obj, 62).ok_or("short header")?));
    let (_, _, _, stroff, strsize) = shdr(obj, shoff, shstrndx).ok_or("no section-name table")?;
    let strtab = obj.get(stroff..stroff + strsize).ok_or("section-name table past the end")?;
    let mut bits = None::<u32>;
    for i in 0..shnum {
        let (name, t, _, off, size) = shdr(obj, shoff, i).ok_or("section header past the end")?;
        let n = strtab.get(name as usize..).and_then(|s| s.split(|&b| b == 0).next()).unwrap_or(b"");
        if t == SHT_NOTE && n == SECTION {
            let b = note_bits(obj.get(off..off + size).ok_or("note past the end")?)?;
            bits = Some(bits.unwrap_or(u32::MAX) & b);
        }
    }
    Ok(bits)
}

/// The note this module adds: owner "GNU", `NT_GNU_PROPERTY_TYPE_0`, FEATURE_1_AND = SHSTK.
fn note() -> Vec<u8> {
    let mut n = Vec::with_capacity(32);
    for w in [4u32, 16, 5] {
        n.extend_from_slice(&w.to_le_bytes());
    }
    n.extend_from_slice(b"GNU\0");
    for w in [FEATURE_1_AND, 4, SHSTK, 0] {
        n.extend_from_slice(&w.to_le_bytes());
    }
    n
}

/// `obj` (an ELF64 relocatable without a property note) with a `.note.gnu.property` section
/// appended: the note, a copy of the section-name table with the new name, and a copy of the
/// section headers with one more, all past the old end; no existing section moves or changes
/// index, so symbols, relocations and groups stay valid.
pub fn add_note(obj: &[u8]) -> Result<Vec<u8>, String> {
    if u16_at(obj, 16) != Some(1) || u16_at(obj, 58) != Some(64) {
        return Err("not an ELF64 relocatable object".into());
    }
    let (shoff, shnum, shstrndx) = (us(u64_at(obj, 40).ok_or("short header")?).ok_or("shoff")?, usize::from(u16_at(obj, 60).ok_or("short header")?), usize::from(u16_at(obj, 62).ok_or("short header")?));
    // Extended numbering (shnum 0 or names index SHN_XINDEX) or a full table: not produced here.
    if shnum == 0 || shnum >= 0xff00 || shstrndx >= shnum {
        return Err(format!("{shnum} sections (names at {shstrndx}): extended section numbering is not handled"));
    }
    let (_, _, _, stroff, strsize) = shdr(obj, shoff, shstrndx).ok_or("no section-name table")?;
    let headers = obj.get(shoff..shoff + shnum * 64).ok_or("section headers past the end")?;
    let mut out = obj.to_vec();
    out.resize((out.len() + 7) & !7, 0);
    let note_off = out.len();
    out.extend_from_slice(&note());
    let names_off = out.len();
    out.extend_from_slice(obj.get(stroff..stroff + strsize).ok_or("section-name table past the end")?);
    let name = u32::try_from(strsize).map_err(|_| "section-name table too large")?;
    out.extend_from_slice(SECTION);
    out.push(0);
    let names_size = out.len() - names_off;
    out.resize((out.len() + 7) & !7, 0);
    let new_shoff = out.len();
    out.extend_from_slice(headers);
    let names = new_shoff + shstrndx * 64;
    out[names + 24..names + 32].copy_from_slice(&(names_off as u64).to_le_bytes());
    out[names + 32..names + 40].copy_from_slice(&(names_size as u64).to_le_bytes());
    let mut h = [0u8; 64];
    h[0..4].copy_from_slice(&name.to_le_bytes());
    h[4..8].copy_from_slice(&SHT_NOTE.to_le_bytes());
    h[8..16].copy_from_slice(&SHF_ALLOC.to_le_bytes());
    h[24..32].copy_from_slice(&(note_off as u64).to_le_bytes());
    h[32..40].copy_from_slice(&32u64.to_le_bytes());
    h[48..56].copy_from_slice(&8u64.to_le_bytes());
    out.extend_from_slice(&h);
    out[40..48].copy_from_slice(&(new_shoff as u64).to_le_bytes());
    out[60..62].copy_from_slice(&u16::try_from(shnum + 1).map_err(|_| "too many sections")?.to_le_bytes());
    Ok(out)
}

/// One archive member: its header's name field, its long name resolved, and its bytes.
struct Member {
    header: Vec<u8>,
    name: String,
    body: Vec<u8>,
}

fn members(data: &[u8]) -> Result<Vec<Member>, String> {
    if !data.starts_with(b"!<arch>\n") {
        return Err("not an ar archive".into());
    }
    let (mut pos, mut out, mut longnames) = (8usize, Vec::new(), Vec::new());
    while pos + 60 <= data.len() {
        let size = String::from_utf8_lossy(&data[pos + 48..pos + 58]).trim().parse::<usize>().map_err(|_| format!("bad member header at byte {pos}"))?;
        let body = pos + 60;
        if &data[pos + 58..pos + 60] != b"`\n" || body + size > data.len() {
            return Err(format!("bad member header at byte {pos}"));
        }
        let raw = String::from_utf8_lossy(&data[pos..pos + 16]).trim_end().to_string();
        let bytes = data[body..body + size].to_vec();
        if raw == "//" {
            longnames.clone_from(&bytes);
        }
        let name = match raw.strip_prefix('/').and_then(|n| n.parse::<usize>().ok()) {
            Some(off) => longnames.get(off..).and_then(|s| s.split(|&b| b == b'\n').next()).map_or(raw.clone(), |s| String::from_utf8_lossy(s).trim_end_matches('/').to_string()),
            None => raw.trim_end_matches('/').to_string(),
        };
        out.push(Member { header: data[pos..pos + 60].to_vec(), name, body: bytes });
        pos = body + size + (size & 1);
    }
    Ok(out)
}

/// The ELF members of an archive (not the symbol index or the long-name table).
fn is_object(m: &Member) -> bool {
    m.body.starts_with(b"\x7fELF")
}

/// Gives every ELF member of the archive `data` without a property note the SHSTK note
/// ([`add_note`]); returns the new archive (`None`: nothing to add) and the number added. The
/// symbol index is dropped from a rewritten archive (its member offsets change): the caller
/// rebuilds it (`ar s`). A member whose note lacks SHSTK is an error.
pub fn mark_archive_bytes(data: &[u8]) -> Result<(Option<Vec<u8>>, usize), String> {
    let mut ms = members(data)?;
    let mut added = 0;
    for m in ms.iter_mut().filter(|m| is_object(m)) {
        match feature_bits(&m.body).map_err(|e| format!("{}: {e}", m.name))? {
            Some(b) if b & SHSTK != 0 => {}
            Some(b) => return Err(format!("{}: its property note has x86 features {b:#x}, without SHSTK: audit its code (D-X.164)", m.name)),
            None => {
                m.body = add_note(&m.body).map_err(|e| format!("{}: {e}", m.name))?;
                added += 1;
            }
        }
    }
    if added == 0 {
        return Ok((None, 0));
    }
    let mut out = b"!<arch>\n".to_vec();
    for m in ms.iter().filter(|m| !(m.header.starts_with(b"/ ") || m.header.starts_with(b"/SYM64/ "))) {
        let mut h = m.header.clone();
        h[48..58].copy_from_slice(format!("{:<10}", m.body.len()).as_bytes());
        out.extend_from_slice(&h);
        out.extend_from_slice(&m.body);
        if m.body.len() % 2 == 1 {
            out.push(b'\n');
        }
    }
    Ok((Some(out), added))
}

/// [`mark_archive_bytes`] on the archive file at `path`, rewritten (with its symbol index
/// rebuilt by `ar s`) only when a member got a note. Returns the number of notes added.
pub fn mark_archive(path: &Path) -> Result<usize, String> {
    let data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (out, added) = mark_archive_bytes(&data).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(out) = out {
        fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))?;
        let ar = std::env::var("AR").unwrap_or_else(|_| "ar".into());
        let s = std::process::Command::new(&ar).arg("s").arg(path).status().map_err(|e| format!("{ar}: {e}"))?;
        if !s.success() {
            return Err(format!("{ar} s {}: {s}", path.display()));
        }
    }
    Ok(added)
}

/// What in `data` (an ELF file or an archive) lacks SHSTK: one line per object.
pub fn unmarked(data: &[u8]) -> Result<Vec<String>, String> {
    let bad = |bits: Option<u32>| bits.is_none_or(|b| b & SHSTK == 0);
    if data.starts_with(b"\x7fELF") {
        return Ok(if bad(feature_bits(data)?) { vec![String::new()] } else { vec![] });
    }
    let mut out = Vec::new();
    for m in members(data)?.iter().filter(|m| is_object(m)) {
        if bad(feature_bits(&m.body).map_err(|e| format!("{}: {e}", m.name))?) {
            out.push(m.name.clone());
        }
    }
    Ok(out)
}

/// The packages' manifest: every file they ship is a row, and a `file` row names its product.
pub const SHIPPED_MANIFEST: &str = "tools/deb/manifest.tsv";

/// The products the packages ship, read from the manifest's text: the source (column 5) of every
/// `file` row (column 4), each once, sorted. Comment and blank lines are skipped; a row with fewer
/// than five columns is an error (the manifest's own lint refuses it too).
pub fn shipped_products(manifest: &str) -> Result<Vec<String>, String> {
    let mut out = std::collections::BTreeSet::new();
    for (n, line) in manifest.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 {
            return Err(format!("{SHIPPED_MANIFEST}:{}: {} columns, at least 5 expected", n + 1, cols.len()));
        }
        if cols[3] == "file" {
            out.insert(cols[4].to_string());
        }
    }
    Ok(out.into_iter().collect())
}

/// The build check: every product the packages ship ([`SHIPPED_MANIFEST`]'s `file` rows), when it
/// is an ELF file or an archive, has SHSTK in the file or in every member. A listed product that
/// is missing from `release` is an error (the check cannot vouch for it). Files the packages do
/// not ship (test programs, the Rust dylibs, the by-path copy of libc.so.6) are not read. Returns
/// the number of ELF files and archives checked.
pub fn check_release(root: &Path, release: &Path) -> Result<usize, String> {
    let manifest = root.join(SHIPPED_MANIFEST);
    let text = fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    check_products(release, &shipped_products(&text)?)
}

/// [`check_release`] over the named products of `release`.
fn check_products(release: &Path, products: &[String]) -> Result<usize, String> {
    let (mut checked, mut bad) = (0usize, Vec::new());
    for rel in products {
        let p = release.join(rel);
        let data = match fs::read(&p) {
            Ok(d) => d,
            Err(e) => {
                bad.push(format!("{rel}: {e}"));
                continue;
            }
        };
        if !(data.starts_with(b"\x7fELF") || data.starts_with(b"!<arch>\n")) {
            continue;
        }
        match unmarked(&data) {
            Ok(v) => {
                checked += 1;
                bad.extend(v.into_iter().map(|m| if m.is_empty() { rel.clone() } else { format!("{rel}({m})") }));
            }
            Err(e) => bad.push(format!("{rel}: {e}")),
        }
    }
    if bad.is_empty() {
        Ok(checked)
    } else {
        Err(format!("{} shipped objects lack the SHSTK property (D-X.164): {}", bad.len(), bad.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal ELF64 relocatable: the null section, `.text` (4 bytes) and the names.
    fn object() -> Vec<u8> {
        let mut o = vec![0u8; 64];
        o[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        o[16..18].copy_from_slice(&1u16.to_le_bytes());
        o[18..20].copy_from_slice(&62u16.to_le_bytes());
        o[52..54].copy_from_slice(&64u16.to_le_bytes());
        o[58..60].copy_from_slice(&64u16.to_le_bytes());
        o.extend_from_slice(&[0xc3, 0x90, 0x90, 0x90]);
        let names = o.len();
        o.extend_from_slice(b"\0.text\0.shstrtab\0");
        let names_size = o.len() - names;
        o.resize((o.len() + 7) & !7, 0);
        let shoff = o.len();
        let sh = |name: u32, ty: u32, off: usize, size: usize| {
            let mut h = [0u8; 64];
            h[0..4].copy_from_slice(&name.to_le_bytes());
            h[4..8].copy_from_slice(&ty.to_le_bytes());
            h[24..32].copy_from_slice(&(off as u64).to_le_bytes());
            h[32..40].copy_from_slice(&(size as u64).to_le_bytes());
            h
        };
        let hs = [sh(0, 0, 0, 0), sh(1, 1, 64, 4), sh(7, 3, names, names_size)];
        for h in hs {
            o.extend_from_slice(&h);
        }
        o[40..48].copy_from_slice(&(shoff as u64).to_le_bytes());
        o[60..62].copy_from_slice(&3u16.to_le_bytes());
        o[62..64].copy_from_slice(&2u16.to_le_bytes());
        o
    }

    fn archive(bodies: &[(&str, &[u8])]) -> Vec<u8> {
        let mut a = b"!<arch>\n".to_vec();
        for (name, body) in bodies {
            a.extend_from_slice(format!("{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n", format!("{name}/"), 0, 0, 0, 644, body.len()).as_bytes());
            a.extend_from_slice(body);
            if body.len() % 2 == 1 {
                a.push(b'\n');
            }
        }
        a
    }

    /// no spec clause (build check, D-X.164): an object without a note gets one that reads back as
    /// SHSTK only, its existing sections unchanged; a second pass adds nothing.
    #[test]
    fn a_note_is_added_once_and_reads_back_as_shstk() {
        let o = object();
        assert_eq!(feature_bits(&o).unwrap(), None);
        let m = add_note(&o).unwrap();
        assert_eq!(feature_bits(&m).unwrap(), Some(SHSTK));
        assert_eq!(&m[64..68], &o[64..68], "the code moved");
        let a = archive(&[("a.o", &o), ("b.o", &m)]);
        let (out, added) = mark_archive_bytes(&a).unwrap();
        assert_eq!(added, 1);
        let out = out.unwrap();
        assert!(unmarked(&out).unwrap().is_empty());
        assert_eq!(mark_archive_bytes(&out).unwrap(), (None, 0));
    }

    /// no spec clause (build check, D-X.164): the check names an unmarked member, and a note that
    /// lacks SHSTK is refused rather than overwritten.
    #[test]
    fn the_check_names_an_unmarked_member_and_refuses_a_note_without_shstk() {
        let o = object();
        let a = archive(&[("good.o", &add_note(&o).unwrap()), ("bare.o", &o)]);
        assert_eq!(unmarked(&a).unwrap(), vec!["bare.o".to_string()]);
        let mut ibt_only = add_note(&o).unwrap();
        let n = ibt_only.windows(32).position(|w| w == note().as_slice()).unwrap() + 24;
        ibt_only[n..n + 4].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(feature_bits(&ibt_only).unwrap(), Some(1));
        let e = mark_archive_bytes(&archive(&[("ibt.o", &ibt_only)])).unwrap_err();
        assert!(e.contains("ibt.o") && e.contains("without SHSTK"), "{e}");
    }

    /// no spec clause (build check, D-X.164): the check reads exactly the products the packages'
    /// manifest ships: an unmarked file it does not list (a test product) passes, an unmarked
    /// listed one fails, a listed one that is missing fails, a non-ELF listed one is skipped.
    #[test]
    fn the_check_reads_the_products_the_manifest_ships() {
        let manifest = "# comment\n\nboth\tlibc6\tusr/lib/a.so\tfile\ta.so\t0644\tunneeded\t-\n\
                        both\tlibc6\tusr/lib/a.so.1\tlink\tlink-target.so\t-\tnone\t-\n\
                        both\tlibc6\tusr/share/t\ttext\tt.txt\t0644\tnone\t-\n\
                        system\tlibc-bin\tusr/bin/s\tfile\ts\t0755\tnone\t-\n\
                        both\tlibc6\tusr/lib/b/a.so\tfile\ta.so\t0644\tunneeded\t-\n";
        assert_eq!(shipped_products(manifest).unwrap(), vec!["a.so".to_string(), "s".to_string()]);
        assert!(shipped_products("both\tlibc6\tx\tfile\n").unwrap_err().contains("4 columns"));
        let dir = std::env::temp_dir().join(format!("niji-cetnote-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("rust-test")).unwrap();
        let o = object();
        fs::write(dir.join("a.so"), add_note(&o).unwrap()).unwrap();
        fs::write(dir.join("s"), b"#!/bin/sh\n").unwrap();
        fs::write(dir.join("rust-test/unshipped.so"), &o).unwrap();
        let products = shipped_products(manifest).unwrap();
        assert_eq!(check_products(&dir, &products).unwrap(), 1);
        fs::write(dir.join("a.so"), &o).unwrap();
        let e = check_products(&dir, &products).unwrap_err();
        assert!(e.contains("1 shipped objects") && e.contains("a.so"), "{e}");
        fs::remove_file(dir.join("a.so")).unwrap();
        let e = check_products(&dir, &products).unwrap_err();
        assert!(e.contains("a.so:"), "{e}");
        let _ = fs::remove_dir_all(&dir);
    }
}
