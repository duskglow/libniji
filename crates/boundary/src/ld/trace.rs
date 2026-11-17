//! The dependency list (`--list`, `ldd`, `LD_TRACE_LOADED_OBJECTS`; DL-8, D-27.16): the search
//! of program start, printed instead of run.

use niji_ld::arch::{Arch, Native};
use niji_ld::program;
use niji_ld::dlopen::Refusal;
use niji_ld::search::{self, PathBuf, Scope};
use niji_ld::{elf, map};
use super::List;
use super::image::{SysMapper, Unusable, dir_of, dyn_string, dynamic_of, open_candidate, open_whole, read_cache};
use super::io::{exit, fail_loading, write_all};

/// Objects a dependency list (`--list`) holds (the program included); more is refused (D-27.16).
const LIST_OBJECTS_MAX: usize = 256;
/// Bytes of resolved paths a dependency list holds; more is refused (D-27.16).
const PATHS_MAX: usize = 64 * 1024;

/// One object of the dependency list: the name it was needed as, its `DT_SONAME`, its file (empty
/// when not found), its resolved path (a span of the path store) and the object that needed it.
#[derive(Clone, Copy, Default)]
struct Obj {
    name: &'static [u8],
    soname: &'static [u8],
    file: &'static [u8],
    path: (usize, usize),
    parent: usize,
    bias: u64,
}

/// `v` as 16 lower-case hexadecimal digits.
fn hex16(v: u64) -> [u8; 16] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digit = |n: u8| HEX.get(usize::from(n & 0xf)).copied().unwrap_or(b'0');
    let mut out = [b'0'; 16];
    for (pair, byte) in out.chunks_exact_mut(2).zip(v.to_be_bytes()) {
        if let [hi, lo] = pair {
            *hi = digit(byte.wrapping_shr(4));
            *lo = digit(byte);
        }
    }
    out
}

/// `ld.so --list PROGRAM` and `LD_TRACE_LOADED_OBJECTS` (ld.so(8), design §19, DL-8): find and map
/// (through [`map::load`]) every object the program needs, breadth first in `DT_NEEDED` order, each
/// once, and write one line per object as `ldd` shows them; nothing is relocated or run. The lines
/// are written once the walk is complete. A dependency that is not found is a `NAME => not found`
/// line (status 0) when tracing; with `strict` (`--list`) it is the loader's error on stderr, nothing
/// on stdout, status 127, as the reference's `ld.so --list` does (DL-8 probe).
pub(super) fn list(env: &List<'_>, prog: &'static [u8], strict: bool) -> ! {
    let mut path = PathBuf::default();
    let fail = |what: &[u8], msg: &[u8]| -> ! {
        for part in [&b"ld.so: "[..], what, b": ", msg] {
            write_all(2, part);
        }
        exit(program::FAILURE)
    };
    let Some((file, fd)) = (if path.push(prog) { path.c_str().and_then(open_whole) } else { None }) else {
        fail(prog, b"cannot open shared object file\n")
    };
    let _ = crate::ffi_sys::niji_sys_close(fd);
    let interp = elf::Elf::parse::<Native>(file).ok().and_then(|e| e.interp().ok().flatten());
    if dynamic_of(file).is_none() {
        write_all(1, b"\tnot a dynamic executable\n");
        exit(1)
    }
    // The cache, once (R14-3 (a): `<prefix>/etc/ld.so.cache`); unreadable or inconsistent = none.
    let cache = read_cache();
    let ctx = env.ctx(cache.as_ref().map(|(_, c)| c).into());
    let mut objs = [Obj::default(); LIST_OBJECTS_MAX];
    let mut store = [0u8; PATHS_MAX];
    let first = store.get_mut(..prog.len()).unwrap_or_else(|| fail(prog, b"path too long\n"));
    first.copy_from_slice(prog);
    let mut used = prog.len();
    if let Some(o) = objs.first_mut() {
        *o = Obj { name: prog, file, path: (0, prog.len()), ..Obj::default() };
    }
    let mut count = 1usize;
    let mut i = 0usize;
    while i < count {
        let me = i;
        i = i.wrapping_add(1);
        let Some(d) = objs.get(me).and_then(|o| dynamic_of(o.file)) else { continue };
        let (old, free) = store.split_at_mut(used);
        let old: &[u8] = old;
        let mut wrote = 0usize;
        // The requester chain: this object, then each object that loaded it, up to the program.
        let mut chain = [Scope::default(); LIST_OBJECTS_MAX];
        let mut depth = 0usize;
        let mut k = me;
        while let (Some(slot), Some(o)) = (chain.get_mut(depth), objs.get(k)) {
            let od = dynamic_of(o.file);
            let origin = old.get(o.path.0..o.path.0.saturating_add(o.path.1)).map(dir_of);
            *slot = Scope { rpath: od.as_ref().and_then(|x| dyn_string(x, elf::dt::RPATH)), runpath: od.as_ref().and_then(|x| dyn_string(x, elf::dt::RUNPATH)), origin };
            depth = depth.wrapping_add(1);
            if k == 0 {
                break;
            }
            k = o.parent;
        }
        let chain = chain.get(..depth).unwrap_or(&[]);
        for name in d.needed().flatten() {
            let known = objs.get(..count).unwrap_or(&[]).iter().any(|o| o.name == name || (!o.soname.is_empty() && o.soname == name));
            if name == Native::INTERP.as_bytes() || known {
                continue;
            }
            if count >= LIST_OBJECTS_MAX {
                fail(name, b"too many shared objects\n");
            }
            let mut hit: Option<(&'static [u8], i64)> = None;
            let mut refused: Option<Refusal> = None;
            let mut hit_path = PathBuf::default();
            search::search::<Native>(name, chain, &ctx, &mut |p: &mut PathBuf| {
                // The program-start rule: another machine's object or class is skipped, the search
                // goes on; any other unusable file ends the list as it ends a start.
                let Some(c) = p.c_str() else { return false };
                let found = match open_candidate(c, env.page) {
                    Err(Unusable::Skip) => return false,
                    Err(Unusable::Refused(r)) => {
                        refused = Some(r);
                        None
                    }
                    Ok((fd, _, f, _)) => Some((f, i64::from(fd))),
                };
                hit_path.clear();
                hit_path.push(p.as_bytes());
                hit = found;
                true
            });
            if let Some(r) = refused {
                let (text, by_path) = r.at_start();
                fail_loading(prog, if by_path { hit_path.as_bytes() } else { name }, text)
            }
            let Some((f, fd)) = hit else {
                if strict {
                    fail_loading(prog, name, niji_ld::dlopen::FILE_MISSING)
                }
                if let Some(o) = objs.get_mut(count) {
                    *o = Obj { name, parent: me, ..Obj::default() };
                }
                count = count.wrapping_add(1);
                continue;
            };
            let found = hit_path.as_bytes();
            let loaded = map::load::<Native, _>(&mut SysMapper { fd: i32::try_from(fd).unwrap_or(-1) }, f, env.page);
            let _ = crate::ffi_sys::niji_sys_close(fd);
            let loaded = loaded.unwrap_or_else(|e| fail(found, e.message()));
            let at = wrote;
            let dst = free.get_mut(at..at.saturating_add(found.len())).unwrap_or_else(|| fail(found, b"too many shared objects\n"));
            dst.copy_from_slice(found);
            wrote = wrote.saturating_add(found.len());
            let soname = dynamic_of(f).and_then(|x| dyn_string(&x, elf::dt::SONAME)).unwrap_or(b"");
            if let Some(o) = objs.get_mut(count) {
                *o = Obj { name, soname, file: f, path: (used.saturating_add(at), found.len()), parent: me, bias: loaded.bias };
            }
            count = count.wrapping_add(1);
        }
        used = used.saturating_add(wrote);
    }
    if let Some(v) = env.vdso.filter(|&v| v != 0) {
        for part in [&b"\tlinux-vdso.so.1 (0x"[..], &hex16(v), b")\n"] {
            write_all(1, part);
        }
    }
    for o in objs.get(1..count).unwrap_or(&[]) {
        if o.file.is_empty() {
            for part in [&b"\t"[..], o.name, b" => not found\n"] {
                write_all(1, part);
            }
        } else {
            let found = store.get(o.path.0..o.path.0.saturating_add(o.path.1)).unwrap_or(&[]);
            for part in [&b"\t"[..], o.name, b" => ", found, b" (0x", &hex16(o.bias), b")\n"] {
                write_all(1, part);
            }
        }
    }
    let interp = interp.unwrap_or(Native::INTERP.as_bytes());
    let base = u64::try_from(env.base).unwrap_or(0);
    for part in [&b"\t"[..], interp, b" (0x", &hex16(base), b")\n"] {
        write_all(1, part);
    }
    exit(0)
}
