//! LAYOUT-GOT-ORDER (D-X.140): the GOT-indirect operands of `add`/`cmp`/... in `libc.a` are marked relaxable, so a
//! static program has no `.got` for its start-up to read.
//!
//! The compiler reaches a static of another crate through the GOT (the library is position-independent code), and
//! when it folds that address into an arithmetic or compare instruction (`add foo@GOTPCREL(%rip), %rdx`) it emits a
//! plain `R_X86_64_GOTPCREL`; GNU ld relaxes only the `R_X86_64_GOTPCRELX` / `R_X86_64_REX_GOTPCRELX` forms (x86-64
//! psABI, "Optimize GOTPCRELX Relocations"). Those few operands (160 sites, 25 slots in the products of 2026-11-20)
//! kept a 200-byte `.got` in every static program, right behind `.data.rel.ro` (GNU ld's default script), so its
//! page position moved with `.data.rel.ro`'s size mod 4096 and start-up read it on a page of its own (D-X.138's
//! `.got` data bin; LAYOUT-BYTE-BUDGETS-3 step 3: a 0x148-byte `.data.rel.ro` change pushed it across a page).
//!
//! The psABI's form for such an instruction is `R_X86_64_REX_GOTPCRELX`: the relocation sits on the 32-bit
//! displacement of a RIP-relative memory operand of a REX.W-prefixed `adc add and cmp or sbb sub xor test`. This pass
//! rewrites the type of exactly those (the instruction bytes are checked, nothing else is touched), so a non-PIE
//! static link turns each into `<op> $foo, %reg` (same length, one memory read fewer) and the program has no GOT. A
//! position-independent link (`libc.so.6`, which is linked before this pass anyway, or a static PIE) keeps the GOT
//! slot: an immediate cannot hold an address there. A target placed beyond the signed 32-bit range in a non-PIE link
//! makes GNU ld refuse the link ("failed to convert GOTPCREL relocation; relink with --no-relax"), never miscompile;
//! the library's small-code-model `R_X86_64_PC32` references already need its data within that range.
//!
//! The archive is patched in place: only relocation-type bytes change, so member sizes, offsets and the archive
//! symbol index stay valid.

use std::fs;
use std::path::Path;

/// `R_X86_64_GOTPCREL`.
const GOTPCREL: u32 = 9;
/// `R_X86_64_REX_GOTPCRELX`.
const REX_GOTPCRELX: u32 = 42;
/// `SHT_RELA`.
const SHT_RELA: u32 = 4;
/// `SHT_NOBITS`.
const SHT_NOBITS: u32 = 8;

/// What [`mark_archive`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Marked {
    /// Relocations rewritten to `R_X86_64_REX_GOTPCRELX`.
    pub marked: usize,
    /// Plain `R_X86_64_GOTPCREL` relocations left as they are (not on an instruction the psABI form covers).
    pub left: usize,
    /// Archive members changed.
    pub members: usize,
}

/// Whether the plain `R_X86_64_GOTPCREL` at `off` of `code` (addend `addend`) is the displacement of a REX.W
/// arithmetic, compare or test instruction with a RIP-relative operand: `REX.W op modrm(00 reg 101) disp32`.
pub fn binop_operand(code: &[u8], off: u64, addend: i64) -> bool {
    let Ok(off) = usize::try_from(off) else { return false };
    if addend != -4 || off < 3 || off.checked_add(4).is_none_or(|end| end > code.len()) {
        return false;
    }
    let (rex, op, modrm) = (code[off - 3], code[off - 2], code[off - 1]);
    // adc add and cmp or sbb sub xor (the `r64, r/m64` forms) and test (`r/m64, r64`): none has an immediate after
    // the displacement, so the relocation is the instruction's last four bytes, as the psABI form requires.
    rex & 0xf8 == 0x48 && matches!(op, 0x03 | 0x0b | 0x13 | 0x1b | 0x23 | 0x2b | 0x33 | 0x3b | 0x85) && modrm & 0xc7 == 0x05
}

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    d.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn u64_at(d: &[u8], at: usize) -> Option<u64> {
    d.get(at..at + 8).map(|b| u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}

/// One section header: (type, offset, size, info).
fn section(obj: &[u8], shoff: usize, shentsize: usize, i: usize) -> Option<(u32, usize, usize, usize)> {
    let at = shoff.checked_add(i.checked_mul(shentsize)?)?;
    let ty = u32_at(obj, at + 4)?;
    let off = usize::try_from(u64_at(obj, at + 24)?).ok()?;
    let size = usize::try_from(u64_at(obj, at + 32)?).ok()?;
    let info = usize::try_from(u32_at(obj, at + 44)?).ok()?;
    Some((ty, off, size, info))
}

/// Marks the relaxable `R_X86_64_GOTPCREL` relocations of one x86-64 relocatable object in place. Anything that is
/// not a little-endian ELF64 x86-64 relocatable object, or whose headers do not fit in it, is left alone.
pub fn mark_object(obj: &mut [u8]) -> Result<Marked, String> {
    let mut m = Marked::default();
    // ELF64 (class 2), little-endian (1), ET_REL (1), EM_X86_64 (62).
    if !obj.starts_with(b"\x7fELF\x02\x01") || u16_at(obj, 16) != Some(1) || u16_at(obj, 18) != Some(62) {
        return Ok(m);
    }
    let bad = |what: &str| format!("malformed ELF object: {what}");
    let shoff = u64_at(obj, 0x28).and_then(|v| usize::try_from(v).ok()).ok_or_else(|| bad("e_shoff"))?;
    let shentsize = usize::from(u16_at(obj, 0x3a).ok_or_else(|| bad("e_shentsize"))?);
    let shnum = usize::from(u16_at(obj, 0x3c).ok_or_else(|| bad("e_shnum"))?);
    if shnum > 0 && shentsize < 64 {
        return Err(bad("section header entries shorter than 64 bytes"));
    }
    let mut edits = Vec::new();
    for i in 0..shnum {
        let (ty, off, size, info) = section(obj, shoff, shentsize, i).ok_or_else(|| bad("section header out of range"))?;
        if ty != SHT_RELA {
            continue;
        }
        let (tty, toff, tsize, _) = section(obj, shoff, shentsize, info).ok_or_else(|| bad("relocated section out of range"))?;
        let rela = obj.get(off..off.checked_add(size).ok_or_else(|| bad("rela size"))?).ok_or_else(|| bad("rela out of range"))?;
        let code: &[u8] = if tty == SHT_NOBITS { &[] } else { obj.get(toff..toff.checked_add(tsize).ok_or_else(|| bad("section size"))?).ok_or_else(|| bad("section out of range"))? };
        for (k, r) in rela.chunks_exact(24).enumerate() {
            let (Some(at), Some(info), Some(addend)) = (u64_at(r, 0), u64_at(r, 8), u64_at(r, 16)) else { continue };
            if info as u32 != GOTPCREL {
                continue;
            }
            if binop_operand(code, at, addend as i64) {
                edits.push(off + k * 24 + 8);
            } else {
                m.left += 1;
            }
        }
    }
    for at in &edits {
        obj[*at..*at + 4].copy_from_slice(&REX_GOTPCRELX.to_le_bytes());
    }
    m.marked = edits.len();
    Ok(m)
}

/// Marks every member of the ar archive `data` in place ([`mark_object`]).
pub fn mark_archive_bytes(data: &mut [u8]) -> Result<Marked, String> {
    if !data.starts_with(b"!<arch>\n") {
        return Err("not an ar archive".into());
    }
    let (mut pos, mut total) = (8usize, Marked::default());
    while pos + 60 <= data.len() {
        let size = String::from_utf8_lossy(&data[pos + 48..pos + 58]).trim().parse::<usize>().map_err(|_| format!("bad member header at byte {pos}"))?;
        let body = pos + 60;
        if &data[pos + 58..pos + 60] != b"`\n" || body + size > data.len() {
            return Err(format!("bad member header at byte {pos}"));
        }
        let m = mark_object(&mut data[body..body + size]).map_err(|e| format!("member at byte {pos}: {e}"))?;
        total.marked += m.marked;
        total.left += m.left;
        total.members += usize::from(m.marked > 0);
        pos = body + size + (size & 1);
    }
    Ok(total)
}

/// [`mark_archive_bytes`] on the archive file at `path`, rewritten only when something changed.
pub fn mark_archive(path: &Path) -> Result<Marked, String> {
    let mut data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let m = mark_archive_bytes(&mut data).map_err(|e| format!("{}: {e}", path.display()))?;
    if m.marked > 0 {
        fs::write(path, &data).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal ELF64 x86-64 relocatable object: `.text` holding `code`, `.rela.text` holding `relocs` (offset,
    /// type, addend), and the null section.
    fn object(code: &[u8], relocs: &[(u64, u32, i64)]) -> Vec<u8> {
        let mut o = vec![0u8; 64];
        o[..6].copy_from_slice(b"\x7fELF\x02\x01");
        o[6] = 1;
        o[16..18].copy_from_slice(&1u16.to_le_bytes());
        o[18..20].copy_from_slice(&62u16.to_le_bytes());
        let text = o.len();
        o.extend_from_slice(code);
        while o.len() % 8 != 0 {
            o.push(0);
        }
        let rela = o.len();
        for (at, ty, add) in relocs {
            o.extend_from_slice(&at.to_le_bytes());
            o.extend_from_slice(&((1u64 << 32) | u64::from(*ty)).to_le_bytes());
            o.extend_from_slice(&add.to_le_bytes());
        }
        let shoff = o.len();
        let mut sh = |ty: u32, off: usize, size: usize, info: u32| {
            let mut s = vec![0u8; 64];
            s[4..8].copy_from_slice(&ty.to_le_bytes());
            s[24..32].copy_from_slice(&(off as u64).to_le_bytes());
            s[32..40].copy_from_slice(&(size as u64).to_le_bytes());
            s[44..48].copy_from_slice(&info.to_le_bytes());
            s
        };
        let headers = [sh(0, 0, 0, 0), sh(1, text, code.len(), 0), sh(SHT_RELA, rela, relocs.len() * 24, 1)].concat();
        o.extend_from_slice(&headers);
        o[0x28..0x30].copy_from_slice(&(shoff as u64).to_le_bytes());
        o[0x3a..0x3c].copy_from_slice(&64u16.to_le_bytes());
        o[0x3c..0x3e].copy_from_slice(&3u16.to_le_bytes());
        o
    }

    fn archive(members: &[Vec<u8>]) -> Vec<u8> {
        let mut a = b"!<arch>\n".to_vec();
        for (i, m) in members.iter().enumerate() {
            a.extend_from_slice(format!("{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n", format!("m{i}.o/"), 0, 0, 0, 644, m.len()).as_bytes());
            a.extend_from_slice(m);
            if m.len() % 2 == 1 {
                a.push(b'\n');
            }
        }
        a
    }

    fn reloc_types(obj: &[u8], n: usize) -> Vec<u32> {
        let shoff = u64_at(obj, 0x28).unwrap() as usize;
        let rela = u64_at(obj, shoff + 2 * 64 + 24).unwrap() as usize;
        (0..n).map(|k| u32_at(obj, rela + k * 24 + 8).unwrap()).collect()
    }

    /// No spec clause (product build): the psABI's REX_GOTPCRELX form covers REX.W adc/add/and/cmp/or/sbb/sub/xor
    /// and test with a RIP-relative operand; a call, a mov without REX.W, a non-RIP operand or a displacement not
    /// at the instruction's end are not that form.
    #[test]
    fn binop_operand_is_the_psabi_form_only() {
        // add 0(%rip),%rdx: 48 03 15 disp32; cmp 0(%rip),%r12: 4c 3b 25 disp32; test %rax,0(%rip): 48 85 05.
        for code in [[0x48, 0x03, 0x15, 0, 0, 0, 0], [0x4c, 0x3b, 0x25, 0, 0, 0, 0], [0x48, 0x85, 0x05, 0, 0, 0, 0], [0x49, 0x2b, 0x0d, 0, 0, 0, 0]] {
            assert!(binop_operand(&code, 3, -4), "{code:x?}");
        }
        // call *0(%rip) (ff 15), add without REX.W (40 03 15), add 0(%rax) (modrm 00), a short section, addend -8.
        for (code, off, add) in [(&[0x41u8, 0xff, 0x15, 0, 0, 0, 0][..], 3, -4), (&[0x40, 0x03, 0x15, 0, 0, 0, 0], 3, -4), (&[0x48, 0x03, 0x10, 0, 0, 0, 0], 3, -4), (&[0x48, 0x03, 0x15, 0, 0, 0], 3, -4), (&[0x48, 0x03, 0x15, 0, 0, 0, 0], 3, -8), (&[0x03, 0x15, 0, 0, 0, 0], 2, -4)] {
            assert!(!binop_operand(code, off, add), "{code:x?} at {off}");
        }
    }

    /// No spec clause (product build): only the plain GOTPCREL relocations on a binop operand change type, in
    /// every member, and the archive keeps its size.
    #[test]
    fn mark_archive_rewrites_only_binop_gotpcrel() {
        // add 0(%rip),%rax; call *0(%rip); cmp 0(%rip),%rbx; mov 0(%rip),%rax (already REX_GOTPCRELX).
        let code = [0x48, 0x03, 0x05, 0, 0, 0, 0, 0xff, 0x15, 0, 0, 0, 0, 0x48, 0x3b, 0x1d, 0, 0, 0, 0, 0x48, 0x8b, 0x05, 0, 0, 0, 0];
        let relocs = [(3, GOTPCREL, -4), (9, GOTPCREL, -4), (16, GOTPCREL, -4), (23, REX_GOTPCRELX, -4)];
        let one = object(&code, &relocs);
        let mut a = archive(&[one.clone(), b"not an object".to_vec(), one.clone()]);
        let len = a.len();
        let m = mark_archive_bytes(&mut a).expect("marks");
        assert_eq!(m, Marked { marked: 4, left: 2, members: 2 });
        assert_eq!(a.len(), len);
        let body = 8 + 60;
        assert_eq!(reloc_types(&a[body..body + one.len()], 4), [REX_GOTPCRELX, GOTPCREL, REX_GOTPCRELX, REX_GOTPCRELX]);
        // A second pass finds nothing left to mark.
        assert_eq!(mark_archive_bytes(&mut a).expect("again").marked, 0);
    }

    /// No spec clause (product build): a truncated object is an error, never a write past its end.
    #[test]
    fn mark_object_rejects_headers_outside_the_object() {
        let mut o = object(&[0x48, 0x03, 0x05, 0, 0, 0, 0], &[(3, GOTPCREL, -4)]);
        let cut = o.len() - 10;
        assert!(mark_object(&mut o[..cut]).is_err());
    }
}
