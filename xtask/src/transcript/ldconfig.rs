//! Liella's `ldconfig` as a hosted command (spec §31.2, plan/utilities.md UTIL-3): the engine in
//! `liella_ld::ldconfig` over a std file system. This binary runs it when started with argv[0] `ldconfig` (the
//! transcript runner and the packaging do that). The freestanding product with a syscall file system is a later
//! story; the engine is the same.
//!
//! `-r ROOT` is resolved here, without chroot (no privilege): every path is walked inside ROOT component by
//! component, `..` stops at ROOT, and a symlink's absolute target starts again at ROOT, so nothing a configuration
//! or a link names can reach outside it. More than 40 symlinks on one path is `ELOOP`.

use liella_ld::arch::X86_64;
use liella_ld::ldconfig::{self, Fs, Kind, Meta, Stage, Text};
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::path::PathBuf;
use std::process::ExitCode;

/// The generator text Liella's caches carry (tag 0, spec Appendix D.3.1).
pub(crate) const GENERATOR: &[u8] = ldconfig::GENERATOR;

/// Symlinks followed on one path before `ELOOP` (the kernel's limit).
const LINKS_MAX: u32 = 40;
const ELOOP: i32 = 40;
const EINVAL: i32 = 22;
const EIO: i32 = 5;

/// The host file system, inside `root` when one is given.
pub(crate) struct HostFs {
    root: Vec<u8>,
}

fn errno(e: &std::io::Error) -> i32 {
    e.raw_os_error().unwrap_or(EIO)
}

fn meta(m: &std::fs::Metadata) -> Meta {
    let ft = m.file_type();
    let kind = if ft.is_symlink() {
        Kind::Link
    } else if ft.is_dir() {
        Kind::Dir
    } else if ft.is_file() {
        Kind::File
    } else {
        Kind::Other
    };
    Meta { kind, dev: m.dev(), ino: m.ino() }
}

impl HostFs {
    pub(crate) fn new(root: Option<&[u8]>) -> Self {
        HostFs { root: root.unwrap_or_default().to_vec() }
    }

    /// The host path for `path`: itself without a root; inside the root otherwise, every symlink on the way
    /// (and the last component's too when `follow`) resolved inside it.
    fn host(&self, path: &[u8], follow: bool) -> Result<PathBuf, i32> {
        if self.root.is_empty() {
            return Ok(PathBuf::from(OsStr::from_bytes(path)));
        }
        let mut todo: Vec<Vec<u8>> = path.split(|&c| c == b'/').filter(|c| !c.is_empty()).rev().map(<[u8]>::to_vec).collect();
        let mut done: Vec<Vec<u8>> = Vec::new();
        let mut links = 0u32;
        let at = |done: &[Vec<u8>]| {
            let mut p = self.root.clone();
            for c in done {
                p.push(b'/');
                p.extend_from_slice(c);
            }
            PathBuf::from(OsString::from_vec(p))
        };
        while let Some(c) = todo.pop() {
            match c.as_slice() {
                b"." => continue,
                b".." => {
                    done.pop();
                    continue;
                }
                _ => {}
            }
            done.push(c);
            if todo.is_empty() && !follow {
                break;
            }
            let here = at(&done);
            let Ok(m) = std::fs::symlink_metadata(&here) else { continue };
            if !m.file_type().is_symlink() {
                continue;
            }
            links += 1;
            if links > LINKS_MAX {
                return Err(ELOOP);
            }
            let target = std::fs::read_link(&here).map_err(|e| errno(&e))?;
            let t = target.as_os_str().as_bytes();
            done.pop();
            if t.first() == Some(&b'/') {
                done.clear();
            }
            todo.extend(t.split(|&c| c == b'/').filter(|c| !c.is_empty()).rev().map(<[u8]>::to_vec));
        }
        Ok(at(&done))
    }
}

use std::os::unix::ffi::OsStringExt;

impl Fs for HostFs {
    fn root(&self) -> &[u8] {
        &self.root
    }

    fn stat(&mut self, path: &[u8]) -> Result<Meta, i32> {
        std::fs::metadata(self.host(path, true)?).map(|m| meta(&m)).map_err(|e| errno(&e))
    }

    fn lstat(&mut self, path: &[u8]) -> Result<Meta, i32> {
        std::fs::symlink_metadata(self.host(path, false)?).map(|m| meta(&m)).map_err(|e| errno(&e))
    }

    fn list(&mut self, dir: &[u8], out: &mut Vec<Vec<u8>>) -> Result<(), i32> {
        for e in std::fs::read_dir(self.host(dir, true)?).map_err(|e| errno(&e))? {
            out.push(e.map_err(|e| errno(&e))?.file_name().into_vec());
        }
        Ok(())
    }

    fn read_at(&mut self, path: &[u8], offset: u64, buf: &mut [u8]) -> Result<usize, i32> {
        let f = std::fs::File::open(self.host(path, true)?).map_err(|e| errno(&e))?;
        let mut got = 0;
        while got < buf.len() {
            match f.read_at(&mut buf[got..], offset + got as u64) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(errno(&e)),
            }
        }
        Ok(got)
    }

    fn read_file(&mut self, path: &[u8], max: usize, out: &mut Vec<u8>) -> Result<(), i32> {
        use std::io::Read;
        let f = std::fs::File::open(self.host(path, true)?).map_err(|e| errno(&e))?;
        f.take(max as u64).read_to_end(out).map_err(|e| errno(&e))?;
        Ok(())
    }

    fn read_link(&mut self, path: &[u8], out: &mut Vec<u8>) -> Result<(), i32> {
        let t = std::fs::read_link(self.host(path, false)?).map_err(|e| errno(&e))?;
        out.extend_from_slice(t.as_os_str().as_bytes());
        Ok(())
    }

    fn replace_link(&mut self, target: &[u8], link: &[u8]) -> Result<(), i32> {
        // A new link under a temporary name in the same directory, renamed over the old one: a reader sees the old
        // link or the new one, never none (the pending D-31 record of UTIL-LDCONFIG-3, rule 1).
        let dst = self.host(link, false)?;
        let name = dst.file_name().ok_or(EINVAL)?;
        let mut tmp_name = OsString::from(".");
        tmp_name.push(name);
        tmp_name.push(".ldconfig-new");
        let tmp = dst.with_file_name(tmp_name);
        let _ = std::fs::remove_file(&tmp);
        std::os::unix::fs::symlink(OsStr::from_bytes(target), &tmp).map_err(|e| errno(&e))?;
        std::fs::rename(&tmp, &dst).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            errno(&e)
        })
    }

    fn remove(&mut self, path: &[u8]) -> Result<(), i32> {
        std::fs::remove_file(self.host(path, false)?).map_err(|e| errno(&e))
    }

    fn replace_file(&mut self, path: &[u8], data: &[u8]) -> Result<(), (Stage, i32)> {
        let dst = self.host(path, false).map_err(|e| (Stage::Create, e))?;
        let mut tmp = dst.clone().into_os_string();
        tmp.push("~");
        let tmp = PathBuf::from(tmp);
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o644).open(&tmp).map_err(|e| (Stage::Create, errno(&e)))?;
        let wrote = f.write_all(data).and_then(|()| f.sync_all());
        drop(f);
        if let Err(e) = wrote {
            let _ = std::fs::remove_file(&tmp);
            return Err((Stage::Write, errno(&e)));
        }
        std::fs::rename(&tmp, &dst).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            (Stage::Rename, errno(&e))
        })
    }

    fn error_text(&self, errno: i32, out: &mut Text) {
        let text = std::io::Error::from_raw_os_error(errno).to_string();
        out.bytes(text.split(" (os error").next().unwrap_or_default().as_bytes());
    }
}

/// `ldconfig` with `args` (after argv[0]); `argv0` is the program name used in messages.
pub(crate) fn ldconfig(argv0: &OsString, args: &[OsString]) -> ExitCode {
    let args: Vec<&[u8]> = args.iter().map(|a| a.as_bytes()).collect();
    let mut fs = HostFs::new(ldconfig::root_of(&args));
    let r = ldconfig::run::<X86_64, _>(argv0.as_bytes(), &args, &mut fs, GENERATOR);
    let _ = std::io::stdout().write_all(&r.out.buf).and_then(|()| std::io::stdout().flush());
    let _ = std::io::stderr().write_all(&r.err.buf);
    ExitCode::from(r.status)
}
