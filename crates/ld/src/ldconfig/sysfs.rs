//! [`Fs`] over the kernel's system calls ([`Sys`]): the file system of the shipped, freestanding
//! `ldconfig` (D-31.2). With `-r ROOT` every path is resolved by `openat2` with `RESOLVE_IN_ROOT`
//! below a descriptor of ROOT, so a symlink or `..` anywhere in the tree stays inside it and no
//! privilege is needed (D-31.9). A path's last component is acted on through a descriptor of its
//! parent (resolved the same way), so links are made, read and removed inside the root too.

use alloc::vec::Vec;

use liella_kernel::errno::Errno;
use liella_kernel::fd::Fd;
use liella_kernel::path::KPathBuf;
use liella_kernel::sys::Sys;
use liella_kernel::uapi::{
    OpenHow, AT_EMPTY_PATH, AT_SYMLINK_NOFOLLOW, O_CLOEXEC, O_CREAT, O_DIRECTORY, O_NOFOLLOW, O_PATH, O_RDONLY,
    O_TRUNC, O_WRONLY, RESOLVE_IN_ROOT, S_IFDIR, S_IFLNK, S_IFMT, S_IFREG,
};

use super::{extend, push, Fs, Kind, Meta, Stage, Text};

const ENOMEM: i32 = 12;
const EINVAL: i32 = 22;

/// Bytes of one `getdents64` call.
const DIRENT_BUF: usize = 8 * 1024;
/// Bytes of a symlink target read at once (the kernel's `PATH_MAX`).
const LINK_MAX: usize = 4096;

/// A descriptor closed when dropped.
struct Owned<'s, S: Sys>(&'s S, Fd);

impl<S: Sys> Drop for Owned<'_, S> {
    fn drop(&mut self) {
        let _ = self.0.close(self.1);
    }
}

/// The file system through `sys`, inside `root` when one is given.
pub struct SysFs<'s, S: Sys> {
    sys: &'s S,
    root: Vec<u8>,
    /// The descriptor paths are resolved from: ROOT's, or the working directory.
    base: Fd,
    /// `RESOLVE_IN_ROOT` with a root, else 0.
    resolve: u64,
    /// The file [`Fs::read_at`] read last and its descriptor: the scan reads one file's headers in several
    /// calls, so it is opened once.
    last: Option<(Vec<u8>, Fd)>,
    /// The text of an error number (`strerror`'s), `None` for one without text.
    text: fn(i32) -> Option<&'static str>,
    /// Why the root could not be opened: every operation then fails with it.
    broken: Option<i32>,
}

fn errno(e: Errno) -> i32 {
    e.get()
}

fn kpath(path: &[u8]) -> Result<KPathBuf, i32> {
    KPathBuf::new(path).map_err(errno)
}

fn meta(mode: u32, dev: u64, ino: u64) -> Meta {
    let kind = match mode & S_IFMT {
        S_IFREG => Kind::File,
        S_IFDIR => Kind::Dir,
        S_IFLNK => Kind::Link,
        _ => Kind::Other,
    };
    Meta { kind, dev, ino }
}

/// `path` split into its parent directory and its last component (`None` when the last component is
/// empty, `.` or `..`: such a path names a directory, never a link, so it is resolved whole).
fn split(path: &[u8]) -> Option<(&[u8], &[u8])> {
    let (dir, name) = match path.iter().rposition(|&c| c == b'/') {
        Some(0) => (&b"/"[..], path.get(1..)?),
        Some(i) => (path.get(..i)?, path.get(i.wrapping_add(1)..)?),
        None => (&b"."[..], path),
    };
    (!matches!(name, b"" | b"." | b"..")).then_some((dir, name))
}

impl<'s, S: Sys> SysFs<'s, S> {
    /// The file system through `sys`; with `root`, every path resolved inside it. When `root` cannot be
    /// opened as a directory, every operation fails with that error, as for a path that does not exist.
    pub fn new(sys: &'s S, root: Option<&[u8]>, text: fn(i32) -> Option<&'static str>) -> Self {
        let mut fs = SysFs { sys, root: Vec::new(), base: Fd::CWD, resolve: 0, last: None, text, broken: None };
        if let Some(r) = root.filter(|r| !r.is_empty()) {
            fs.resolve = RESOLVE_IN_ROOT;
            let opened = extend(&mut fs.root, r).map_err(|_| ENOMEM).and_then(|()| kpath(r)).and_then(|p| {
                sys.openat(Fd::CWD, p.as_kstr(), O_PATH | O_DIRECTORY | O_CLOEXEC, 0).map_err(errno)
            });
            match opened {
                Ok(fd) => fs.base = fd,
                Err(e) => fs.broken = Some(e),
            }
        }
        fs
    }

    /// Opens `path`, resolved from the base (inside the root when there is one).
    fn open(&self, path: &[u8], flags: u32) -> Result<Owned<'s, S>, i32> {
        self.open_raw(path, flags).map(|fd| Owned(self.sys, fd))
    }

    /// [`Self::open`]'s descriptor, which the caller closes.
    fn open_raw(&self, path: &[u8], flags: u32) -> Result<Fd, i32> {
        if let Some(e) = self.broken {
            return Err(e);
        }
        let p = kpath(path)?;
        let how = OpenHow { flags: u64::from(flags | O_CLOEXEC), mode: 0, resolve: self.resolve };
        self.sys.openat2(self.base, p.as_kstr(), &how).map_err(errno)
    }

    /// The parent of `path` opened as a directory, and the last component's name.
    fn parent(&self, path: &[u8]) -> Result<(Owned<'s, S>, KPathBuf), i32> {
        let (dir, name) = split(path).ok_or(EINVAL)?;
        Ok((self.open(dir, O_PATH | O_DIRECTORY)?, kpath(name)?))
    }

    fn stat_fd(&self, fd: &Owned<'s, S>) -> Result<Meta, i32> {
        let st = self.sys.fstatat(fd.1, kpath(b"")?.as_kstr(), AT_EMPTY_PATH).map_err(errno)?;
        Ok(meta(st.mode, st.dev, st.ino))
    }

    /// Writes all of `data` to `fd`.
    fn write_all(&self, fd: Fd, mut data: &[u8]) -> Result<(), i32> {
        while !data.is_empty() {
            match self.sys.write(fd, data) {
                Ok(0) => return Err(Errno::EIO.get()),
                Ok(n) => data = data.get(n..).unwrap_or_default(),
                Err(Errno::EINTR) => {}
                Err(e) => return Err(errno(e)),
            }
        }
        Ok(())
    }

    /// The descriptor [`Fs::read_at`] reads `path` through.
    fn reader(&mut self, path: &[u8]) -> Result<Fd, i32> {
        if let Some((p, fd)) = &self.last {
            if p.as_slice() == path {
                return Ok(*fd);
            }
        }
        self.forget();
        let mut p = Vec::new();
        extend(&mut p, path).map_err(|_| ENOMEM)?;
        let fd = self.open_raw(path, O_RDONLY)?;
        self.last = Some((p, fd));
        Ok(fd)
    }

    /// Closes [`Self::reader`]'s descriptor.
    fn forget(&mut self) {
        if let Some((_, fd)) = self.last.take() {
            let _ = self.sys.close(fd);
        }
    }
}

impl<S: Sys> Drop for SysFs<'_, S> {
    fn drop(&mut self) {
        self.forget();
        if self.base != Fd::CWD {
            let _ = self.sys.close(self.base);
        }
    }
}

impl<S: Sys> Fs for SysFs<'_, S> {
    fn root(&self) -> &[u8] {
        &self.root
    }

    fn stat(&mut self, path: &[u8]) -> Result<Meta, i32> {
        let f = self.open(path, O_PATH)?;
        self.stat_fd(&f)
    }

    fn lstat(&mut self, path: &[u8]) -> Result<Meta, i32> {
        if split(path).is_none() {
            return self.stat(path);
        }
        let (dir, name) = self.parent(path)?;
        let st = self.sys.fstatat(dir.1, name.as_kstr(), AT_SYMLINK_NOFOLLOW).map_err(errno)?;
        Ok(meta(st.mode, st.dev, st.ino))
    }

    fn list(&mut self, dir: &[u8], out: &mut Vec<Vec<u8>>) -> Result<(), i32> {
        let d = self.open(dir, O_RDONLY | O_DIRECTORY)?;
        let mut buf = [0u8; DIRENT_BUF];
        loop {
            let n = self.sys.getdents64(d.1, &mut buf).map_err(errno)?;
            if n == 0 {
                return Ok(());
            }
            // Each record: d_ino (8), d_off (8), d_reclen (2), d_type (1), the NUL-terminated name.
            let mut at = 0usize;
            while at < n {
                let rec = buf.get(at..n).unwrap_or_default();
                let len = rec.get(16..18).and_then(|b| <[u8; 2]>::try_from(b).ok()).map_or(0, |b| usize::from(u16::from_ne_bytes(b)));
                let Some(name) = rec.get(19..len) else { return Err(Errno::EIO.get()) };
                let name = name.split(|&c| c == 0).next().unwrap_or_default();
                if !matches!(name, b"." | b"..") {
                    let mut v = Vec::new();
                    extend(&mut v, name).map_err(|_| ENOMEM)?;
                    push(out, v).map_err(|_| ENOMEM)?;
                }
                at = at.saturating_add(len);
            }
        }
    }

    fn read_at(&mut self, path: &[u8], offset: u64, buf: &mut [u8]) -> Result<usize, i32> {
        let fd = self.reader(path)?;
        let mut got = 0usize;
        while got < buf.len() {
            let at = u64::try_from(got).ok().and_then(|g| offset.checked_add(g)).and_then(|a| i64::try_from(a).ok()).ok_or(EINVAL)?;
            match self.sys.pread(fd, buf.get_mut(got..).unwrap_or_default(), at) {
                Ok(0) => break,
                Ok(n) => got = got.saturating_add(n),
                Err(Errno::EINTR) => {}
                Err(e) => return Err(errno(e)),
            }
        }
        Ok(got)
    }

    fn read_file(&mut self, path: &[u8], max: usize, out: &mut Vec<u8>) -> Result<(), i32> {
        let f = self.open(path, O_RDONLY)?;
        let mut chunk = [0u8; 4096];
        while out.len() < max {
            let want = chunk.len().min(max.saturating_sub(out.len()));
            match self.sys.read(f.1, chunk.get_mut(..want).unwrap_or_default()) {
                Ok(0) => break,
                Ok(n) => extend(out, chunk.get(..n).unwrap_or_default()).map_err(|_| ENOMEM)?,
                Err(Errno::EINTR) => {}
                Err(e) => return Err(errno(e)),
            }
        }
        Ok(())
    }

    fn read_link(&mut self, path: &[u8], out: &mut Vec<u8>) -> Result<(), i32> {
        let (dir, name) = self.parent(path)?;
        let mut buf = [0u8; LINK_MAX];
        let n = self.sys.readlinkat(dir.1, name.as_kstr(), &mut buf).map_err(errno)?;
        extend(out, buf.get(..n).unwrap_or_default()).map_err(|_| ENOMEM)
    }

    fn replace_link(&mut self, target: &[u8], link: &[u8]) -> Result<(), i32> {
        // A new link under a temporary name in the same directory, renamed over the old one: a reader sees the
        // old link or the new one, never none.
        let (dir, name) = self.parent(link)?;
        let base = split(link).map_or(&b""[..], |(_, n)| n);
        let mut tmp = Vec::new();
        for part in [&b"."[..], base, b".ldconfig-new"] {
            extend(&mut tmp, part).map_err(|_| ENOMEM)?;
        }
        let tmp = kpath(&tmp)?;
        let _ = self.sys.unlinkat(dir.1, tmp.as_kstr(), 0);
        self.sys.symlinkat(kpath(target)?.as_kstr(), dir.1, tmp.as_kstr()).map_err(errno)?;
        self.sys.renameat2(dir.1, tmp.as_kstr(), dir.1, name.as_kstr(), 0).map_err(|e| {
            let _ = self.sys.unlinkat(dir.1, tmp.as_kstr(), 0);
            errno(e)
        })
    }

    fn remove(&mut self, path: &[u8]) -> Result<(), i32> {
        let (dir, name) = self.parent(path)?;
        self.sys.unlinkat(dir.1, name.as_kstr(), 0).map_err(errno)
    }

    fn replace_file(&mut self, path: &[u8], data: &[u8]) -> Result<(), (Stage, i32)> {
        let (dir, name) = self.parent(path).map_err(|e| (Stage::Create, e))?;
        let base = split(path).map_or(&b""[..], |(_, n)| n);
        let mut tmp = Vec::new();
        extend(&mut tmp, base).and_then(|()| extend(&mut tmp, b"~")).map_err(|_| (Stage::Create, ENOMEM))?;
        let tmp = kpath(&tmp).map_err(|e| (Stage::Create, e))?;
        let flags = O_WRONLY | O_CREAT | O_TRUNC | O_NOFOLLOW | O_CLOEXEC;
        let fd = self.sys.openat(dir.1, tmp.as_kstr(), flags, 0o644).map_err(|e| (Stage::Create, errno(e)))?;
        let f = Owned(self.sys, fd);
        let wrote = self.write_all(f.1, data).and_then(|()| self.sys.fsync(f.1).map_err(errno));
        drop(f);
        if let Err(e) = wrote {
            let _ = self.sys.unlinkat(dir.1, tmp.as_kstr(), 0);
            return Err((Stage::Write, e));
        }
        self.sys.renameat2(dir.1, tmp.as_kstr(), dir.1, name.as_kstr(), 0).map_err(|e| {
            let _ = self.sys.unlinkat(dir.1, tmp.as_kstr(), 0);
            (Stage::Rename, errno(e))
        })
    }

    fn error_text(&self, errno: i32, out: &mut Text) {
        match (self.text)(errno) {
            Some(t) => out.bytes(t.trim_end_matches('\0').as_bytes()),
            None => out.bytes(b"Unknown error ").num(u64::from(errno.unsigned_abs())),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::split;

    // no spec clause: the parent/name split every link operation relies on.
    #[test]
    fn split_names_the_last_component() {
        assert_eq!(split(b"/lib/libc.so.6"), Some((&b"/lib"[..], &b"libc.so.6"[..])));
        assert_eq!(split(b"/x"), Some((&b"/"[..], &b"x"[..])));
        assert_eq!(split(b"x"), Some((&b"."[..], &b"x"[..])));
        assert_eq!(split(b"/lib/"), None);
        assert_eq!(split(b"/lib/.."), None);
        assert_eq!(split(b"/"), None);
    }
}
