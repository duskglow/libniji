//! The shipped `localedef` program (spec §31.8; freestanding and static like ldconfig, D-31.2):
//! `niji_machine_localedef_entry` (`machine/entry.S`, `-DNIJI_LOCALEDEF`) calls
//! [`niji_localedef_start`] with the initial stack pointer and the run-time address of the
//! program's own ELF header. It relocates the static-PIE image, reads argv, runs
//! `niji_locale::prog::run` over [`Disk`] (the kernel's system calls, through ldconfig's
//! [`SysFs`]) and writes what it printed. Its global allocator is ldconfig's arena.

use alloc::vec::Vec;

use niji_kernel::fd::Fd;
use niji_kernel::path::KPathBuf;
use niji_kernel::sys::Sys;
use niji_kernel::uapi::{LOCK_EX, O_CLOEXEC, O_DIRECTORY, O_RDONLY};
use niji_ld::ldconfig::sysfs::SysFs;
use niji_ld::ldconfig::Fs;
use niji_locale::prog::{self, Host};
use niji_locale::TryVec;

use crate::ld::{c_string, exit, stack_word, write_all};
use crate::sys::CSys;

/// The file system of `localedef`: reads and atomic replacements through [`SysFs`] (paths from
/// the working directory), directories with `mkdirat`.
struct Disk<'s> {
    fs: SysFs<'s, CSys>,
    /// `I18NPATH`'s value, when the environment has it.
    i18npath: Option<&'static [u8]>,
    /// `DEFAULTCHARSET`'s value, when the environment has it (`validlocale`).
    defaultcharset: Option<&'static [u8]>,
    /// `PATH`'s value, when the environment has it (`validlocale` names itself by it).
    path: Option<&'static [u8]>,
    /// The directories whose lock this process holds.
    locked: Vec<Vec<u8>>,
}

impl Host for Disk<'_> {
    fn read(&mut self, path: &[u8], max: usize) -> Result<TryVec<u8>, i32> {
        let mut out = Vec::new();
        self.fs.read_file(path, max, &mut out)?;
        Ok(TryVec::from(out))
    }

    fn mkdir(&mut self, path: &[u8]) -> Result<(), i32> {
        let p = KPathBuf::new(path).map_err(|e| e.get())?;
        CSys.mkdirat(Fd::CWD, p.as_kstr(), 0o777).map_err(|e| e.get())
    }

    fn write(&mut self, path: &[u8], data: &[u8]) -> Result<(), i32> {
        self.fs.replace_file(path, data).map_err(|(_, e)| e)
    }

    fn read_stdin(&mut self, max: usize) -> Result<TryVec<u8>, i32> {
        let mut out = TryVec::new();
        let mut chunk = [0u8; 4096];
        while out.len() < max {
            let n = CSys.read(Fd::STDIN, &mut chunk).map_err(|e| e.get())?;
            if n == 0 {
                break;
            }
            out.extend_from_slice(chunk.get(..n).unwrap_or_default()).map_err(|_| 12)?; // ENOMEM
        }
        Ok(out)
    }
    fn lock(&mut self, dir: &[u8]) -> Result<(), i32> {
        let p = KPathBuf::new(dir).map_err(|e| e.get())?;
        // A flock lock belongs to its open file description: locking the directory again through a second
        // descriptor would wait for this process's own lock (locale-gen runs the localedef command once per
        // locale in one process, D-31.36). A directory already locked is held already.
        if self.locked.iter().any(|d| d.as_slice() == dir) {
            return Ok(());
        }
        let fd = CSys.openat(Fd::CWD, p.as_kstr(), O_RDONLY | O_DIRECTORY | O_CLOEXEC, 0).map_err(|e| e.get())?;
        // The descriptor stays open, and the lock held, until the program exits.
        CSys.flock(fd, LOCK_EX).map_err(|e| e.get())?;
        self.locked.push(dir.to_vec());
        Ok(())
    }
    fn i18npath(&self) -> Option<&[u8]> {
        self.i18npath
    }
    fn remove(&mut self, path: &[u8]) -> Result<(), i32> {
        self.fs.remove(path)
    }
    fn exists(&mut self, path: &[u8]) -> bool {
        self.fs.stat(path).is_ok()
    }
    fn env(&self, name: &[u8]) -> Option<&[u8]> {
        match name {
            b"DEFAULTCHARSET" => self.defaultcharset,
            b"PATH" => self.path,
            _ => None,
        }
    }
    fn emit(&mut self, fd: u8, bytes: &[u8]) -> bool {
        write_all(i64::from(fd), bytes);
        true
    }

    fn error_text(&self, errno: i32) -> &'static [u8] {
        niji::diag::description(errno).map_or(&b"Unknown error"[..], |t| t.trim_end_matches('\0').as_bytes())
    }
}

/// The entry of `localedef` (called by `niji_machine_localedef_entry` with the initial stack
/// pointer `sp` and the run-time address `ehdr` of the image's own ELF header).
// UNSAFE(export): the Rust side of the localedef entry primitive; the program's only entry.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn niji_localedef_start(sp: usize, ehdr: usize) -> ! {
    if crate::bootstrap::relocate_image(ehdr).is_err() {
        write_all(2, b"localedef: cannot relocate the program\n");
        exit(4)
    }
    let argc = usize::try_from(stack_word(sp)).unwrap_or(0);
    let mut args: TryVec<&'static [u8]> = TryVec::new();
    if args.reserve_exact(argc).is_err() {
        write_all(2, b"localedef: memory exhausted\n");
        exit(4)
    }
    for i in 0..argc {
        let p = stack_word(sp.wrapping_add(8).wrapping_add(i.wrapping_mul(8)));
        if args.push(c_string(usize::try_from(p).unwrap_or(0))).is_err() {
            write_all(2, b"localedef: memory exhausted\n");
            exit(4)
        }
    }
    let rest = args.as_slice().get(1..).unwrap_or_default();
    // The environment follows argv's terminating null word; the last I18NPATH entry wins.
    let mut i18npath = None;
    let mut defaultcharset = None;
    let mut path = None;
    let mut at = sp.wrapping_add(16).wrapping_add(argc.wrapping_mul(8));
    loop {
        let p = usize::try_from(stack_word(at)).unwrap_or(0);
        if p == 0 {
            break;
        }
        if let Some(v) = c_string(p).strip_prefix(b"I18NPATH=") {
            i18npath = Some(v);
        }
        if let Some(v) = c_string(p).strip_prefix(b"DEFAULTCHARSET=") {
            defaultcharset = Some(v);
        }
        if let Some(v) = c_string(p).strip_prefix(b"PATH=") {
            path = Some(v);
        }
        at = at.wrapping_add(8);
    }
    let mut disk = Disk { fs: SysFs::new(&CSys, None, niji::diag::description), i18npath, defaultcharset, path, locked: Vec::new() };
    // argv[0] chooses the command: locale-gen, update-locale and validlocale are this program (D-31.36).
    let argv0 = args.as_slice().first().copied().unwrap_or_default();
    let r = match niji_locale::cmds::by_name(argv0, rest, &mut disk) {
        Some(r) => r,
        None => prog::run(rest, &mut disk),
    };
    drop(disk);
    write_all(1, r.out.as_slice());
    write_all(2, r.err.as_slice());
    exit(i32::from(r.status))
}

/// The panic handler of `localedef` (a panic is a bug): a fixed message, status 127.
pub fn panicked(_info: &core::panic::PanicInfo<'_>) -> ! {
    write_all(2, b"localedef: internal error\n");
    exit(127)
}
