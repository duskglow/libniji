//! XTASK-STALE-PRODUCTS: a test link under `target/niji-test` (`cargo xtask so`) is used only when what it loads is
//! there too. A link left by an earlier build (its loader `target/niji/ld-linux-x86-64.so.2` removed since, or a
//! role directory moved so the interpreter path names nothing) would otherwise fail with `ENOENT` at `execve`
//! instead of being skipped with the reason.

use std::path::{Path, PathBuf};

/// The files an ELF executable names by path for the loader: its interpreter (`PT_INTERP`) and each `DT_NEEDED`
/// entry holding a `/` (the test links name `libc-by-path.so` so). An error when `elf` is not a 64-bit
/// little-endian ELF file or a table points outside it.
pub fn loads(elf: &[u8]) -> Result<Vec<PathBuf>, String> {
    let word = |at: usize, n: usize| -> Result<u64, String> {
        let b = elf.get(at..at.checked_add(n).ok_or("offset overflow")?).ok_or_else(|| format!("offset {at:#x} outside the file"))?;
        Ok(b.iter().rev().fold(0u64, |v, &x| (v << 8) | u64::from(x)))
    };
    let size = |v: u64| usize::try_from(v).map_err(|e| e.to_string());
    if elf.get(..6) != Some(b"\x7fELF\x02\x01".as_slice()) {
        return Err("not a 64-bit little-endian ELF file".into());
    }
    let (phoff, phentsize, phnum) = (size(word(0x20, 8)?)?, size(word(0x36, 2)?)?, size(word(0x38, 2)?)?);
    let mut out = Vec::new();
    let (mut loads, mut dynamic) = (Vec::new(), None);
    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        let (kind, offset, vaddr, filesz) = (word(ph, 4)?, size(word(ph + 8, 8)?)?, word(ph + 16, 8)?, size(word(ph + 32, 8)?)?);
        match kind {
            1 => loads.push((vaddr, offset, filesz)),
            2 => dynamic = Some((offset, filesz)),
            3 => {
                let s = elf.get(offset..offset + filesz).ok_or("PT_INTERP outside the file")?;
                out.push(PathBuf::from(String::from_utf8_lossy(s.split(|&b| b == 0).next().unwrap_or_default()).into_owned()));
            }
            _ => {}
        }
    }
    let Some((offset, filesz)) = dynamic else { return Ok(out) };
    let entries: Vec<(u64, u64)> = (offset..offset + filesz).step_by(16).map(|at| Ok((word(at, 8)?, word(at + 8, 8)?))).collect::<Result<_, String>>()?;
    let entries: Vec<(u64, u64)> = entries.into_iter().take_while(|&(tag, _)| tag != 0).collect();
    let Some(&(_, strtab)) = entries.iter().find(|&&(tag, _)| tag == 5) else { return Ok(out) };
    let strtab = loads
        .iter()
        .find(|&&(vaddr, _, filesz)| strtab >= vaddr && strtab - vaddr < filesz as u64)
        .map(|&(vaddr, offset, _)| offset + size(strtab - vaddr).unwrap_or(usize::MAX))
        .ok_or("DT_STRTAB in no PT_LOAD")?;
    for &(_, at) in entries.iter().filter(|&&(tag, _)| tag == 1) {
        let s = elf.get(strtab.checked_add(size(at)?).ok_or("DT_NEEDED offset overflow")?..).ok_or("DT_NEEDED outside the file")?;
        let name = String::from_utf8_lossy(s.split(|&b| b == 0).next().unwrap_or_default()).into_owned();
        if name.contains('/') {
            out.push(PathBuf::from(name));
        }
    }
    Ok(out)
}

/// `link` when it is built and every file it [`loads`] is there; otherwise why it is to be skipped (the caller prints
/// `SKIPPED: <why>`). A link that is no ELF executable fails the test (a broken product, not a missing one).
pub fn usable(link: &Path) -> Result<PathBuf, String> {
    let Ok(elf) = std::fs::read(link) else {
        return Err(format!("{} is not built (run `cargo xtask so` first)", link.display()));
    };
    let wanted = loads(&elf).unwrap_or_else(|e| panic!("{}: {e}", link.display()));
    let missing: Vec<String> = wanted.iter().filter(|p| !p.exists()).map(|p| p.display().to_string()).collect();
    if !missing.is_empty() {
        return Err(format!("{} is left from an earlier build: it loads {}, which is gone (run `cargo xtask so` to link it again)", link.display(), missing.join(" and ")));
    }
    Ok(link.to_path_buf())
}
