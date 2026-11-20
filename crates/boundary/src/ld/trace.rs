//! The dependency list (`--list`, `ldd`, `LD_TRACE_LOADED_OBJECTS`; DL-8, D-27.16): the search
//! of program start, printed instead of run.

use niji_ld::arch::{Arch, Native};
use niji_ld::program;
use niji_ld::dlopen::Refusal;
use niji_ld::search::{self, PathBuf, Scope};
use niji_ld::{elf, map};
use super::table::{PathStore, Table, INLINE};
use super::List;
use super::image::{SysMapper, Unusable, dir_of, dyn_string, dynamic_of, open_candidate, open_whole, read_cache};
use super::io::{exit, fail_loading, write_all};

/// One object of the dependency list: the name it was needed as, its `DT_SONAME`, its file (empty
/// when not found), its resolved path (a span of the path store) and the object that needed it.
#[derive(Clone, Copy, Default)]
struct Obj {
    name: &'static [u8],
    soname: &'static [u8],
    file: &'static [u8],
    /// The path it was found by (the program's: as given; none: not found, or the interpreter).
    path: &'static [u8],
    parent: usize,
    bias: u64,
    /// The interpreter's place in a list (it is never mapped again).
    interp: bool,
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
/// on stdout, status 127, as the reference's `ld.so --list` does (DL-8 probe). The interpreter's
/// line is listed only when an object names the interpreter in `DT_NEEDED` (libniji's own
/// libraries name it last), at its breadth-first place; a program with no `DT_NEEDED` is
/// `statically linked`, status 0 (DL-TRACE-INTERP probe, spec/behaviour/dynlink.md). `invoked` is
/// the loader's own path when it was run as a program, `None` when the kernel started it.
pub(super) fn list(env: &List<'_>, prog: &'static [u8], strict: bool, invoked: Option<&'static [u8]>) -> ! {
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
    // The interpreter's line: the program's PT_INTERP, else the path the loader was run by; the
    // path is the loader as run (`NAME => PATH` when the two differ), as the reference prints it.
    let interp_path = invoked.or(interp).unwrap_or(Native::INTERP.as_bytes());
    let interp_name = interp.unwrap_or(interp_path);
    let Some(top) = dynamic_of(file) else {
        write_all(1, b"\tnot a dynamic executable\n");
        exit(1)
    };
    if top.needed().flatten().next().is_none() {
        // No DT_NEEDED (a static PIE, a library that needs nothing): as the reference, status 0.
        write_all(1, b"\tstatically linked\n");
        exit(0)
    }
    // The cache, once (R14-3 (a): `<prefix>/etc/ld.so.cache`); unreadable or inconsistent = none.
    let cache = read_cache();
    let ctx = env.ctx(cache.as_ref().map(|(_, c)| c).into());
    let mut objs: Table<Obj, INLINE> = Table::new(Obj::default());
    let mut store = PathStore::default();
    objs.reserve(1).unwrap_or_else(|_| fail(prog, b"cannot allocate memory\n"));
    if let Some(o) = objs.first_mut() {
        *o = Obj { name: prog, file, path: prog, ..Obj::default() };
    }
    let mut count = 1usize;
    let mut i = 0usize;
    while i < count {
        let me = i;
        i = i.wrapping_add(1);
        let Some(d) = objs.get(me).and_then(|o| dynamic_of(o.file)) else { continue };
        // The requester chain: this object, then each object that loaded it, up to the program.
        let mut chain: Table<Scope<'_>, INLINE> = Table::new(Scope::default());
        let mut depth = 0usize;
        let mut k = me;
        while let Some(o) = objs.get(k) {
            chain.reserve(depth.wrapping_add(1)).unwrap_or_else(|_| fail(o.name, b"cannot allocate memory\n"));
            let Some(slot) = chain.get_mut(depth) else { break };
            let od = dynamic_of(o.file);
            let origin = Some(dir_of(o.path));
            *slot = Scope { rpath: od.as_ref().and_then(|x| dyn_string(x, elf::dt::RPATH)), runpath: od.as_ref().and_then(|x| dyn_string(x, elf::dt::RUNPATH)), origin };
            depth = depth.wrapping_add(1);
            if k == 0 {
                break;
            }
            k = o.parent;
        }
        let chain = chain.get(..depth).unwrap_or(&[]);
        // libniji's own libraries run only under this loader: each names it after its own
        // DT_NEEDED entries, where the reference's libc and libm name theirs (DL-TRACE-INTERP probe).
        let own = me != 0 && objs.get(me).is_some_and(|o| !o.file.is_empty() && niji_ld::search::pinned(o.name));
        for name in d.needed().flatten().chain(core::iter::once(Native::INTERP.as_bytes()).filter(|_| own)) {
            objs.reserve(count.wrapping_add(1)).unwrap_or_else(|_| fail(name, b"cannot allocate memory\n"));
            if name == Native::INTERP.as_bytes() || name == interp_name || name == interp_path {
                // The interpreter: listed once, at its breadth-first place, only when an object needs it.
                if !objs.get(..count).unwrap_or(&[]).iter().any(|o| o.interp) {
                    if let Some(o) = objs.get_mut(count) {
                        *o = Obj { name, interp: true, parent: me, ..Obj::default() };
                    }
                    count = count.wrapping_add(1);
                }
                continue;
            }
            let known = objs.get(..count).unwrap_or(&[]).iter().any(|o| o.name == name || (!o.soname.is_empty() && o.soname == name));
            if known {
                continue;
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
            let path = store.keep(found).unwrap_or_else(|_| fail(found, b"cannot allocate memory\n"));
            let soname = dynamic_of(f).and_then(|x| dyn_string(&x, elf::dt::SONAME)).unwrap_or(b"");
            if let Some(o) = objs.get_mut(count) {
                *o = Obj { name, soname, file: f, path, parent: me, bias: loaded.bias, interp: false };
            }
            count = count.wrapping_add(1);
        }
    }
    let vdso = || {
        if let Some(v) = env.vdso.filter(|&v| v != 0) {
            for part in [&b"\tlinux-vdso.so.1 (0x"[..], &hex16(v), b")\n"] {
                write_all(1, part);
            }
        }
    };
    // The vDSO's line comes first, after the interpreter's only when the interpreter is the first
    // object listed (the program's first need), as the reference orders them.
    let mut vdso_done = false;
    let base = u64::try_from(env.base).unwrap_or(0);
    for o in objs.get(1..count).unwrap_or(&[]) {
        if !o.interp && !vdso_done {
            vdso();
            vdso_done = true;
        }
        if o.interp {
            write_all(1, b"\t");
            write_all(1, interp_name);
            if interp_name != interp_path {
                write_all(1, b" => ");
                write_all(1, interp_path);
            }
            for part in [&b" (0x"[..], &hex16(base), b")\n"] {
                write_all(1, part);
            }
        } else if o.file.is_empty() {
            for part in [&b"\t"[..], o.name, b" => not found\n"] {
                write_all(1, part);
            }
        } else {
            for part in [&b"\t"[..], o.name, b" => ", o.path, b" (0x", &hex16(o.bias), b")\n"] {
                write_all(1, part);
            }
        }
    }
    if !vdso_done {
        vdso();
    }
    exit(0)
}
