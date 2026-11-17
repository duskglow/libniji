//! Program start's link (DL-6 to DL-10): the `DT_NEEDED` search from the program and the
//! preloads (`LD_PRELOAD`, `/etc/ld.so.preload`), mapping, versions, relocation and symbol
//! binding (`STB_GNU_UNIQUE` included), the TLS layout, and the state `dlopen` reads later.

use core::sync::atomic::Ordering::{Relaxed, Release};
use niji_kernel::auxv::Auxv;
use niji_kernel::uapi::AT_EXECFN;
use niji_ld::arch::{Arch, Native};
use niji_ld::program;
use niji_ld::dlopen::Refusal;
use niji_ld::search::{self, CacheRef, PathBuf, Scope};
use niji_ld::{elf, init, map, start};
use niji_ld::tls::{self as ltls, Module, Placed};
use crate::raw;
use super::{List, __libc_stack_end, register_rseq, report_nonshared};
use super::image::{FileId, ProcImage, SysMapper, Unusable, dir_of, dyn_string, dynamic_of, file_mode, image_view, open_candidate, open_file, own_object, parse, read_cache, read_image, scratch};
use super::initfini::run_init;
use super::io::{c_string, fail_loading, fail_with, write_all};
use super::state::{OBJECTS_MAX, START_INTERP, START_N, START_OBJS, START_PAGE};
use super::tls::{INITIAL_TAIL, install_tls, map_area};

/// Load the program's dependencies, check their versions, relocate everything and protect the RELRO
/// pages (DL-10, design §6, D-27.17). A `DT_NEEDED` name with `/` is opened as given; one without is
/// found by [`search::search`], the search `--list` and `dlopen` use (DL-START-SEARCH): the requester
/// chain is each object's first requester up to the program, `$ORIGIN` the directory of the path an
/// object was opened by (the program's: from `origin`, looked up at the first search), and the
/// cache is read only when a search reaches it. The program is read from its memory image
/// ([`image_view`], ruling #4229, D-27.18), never from its file; `name` names it in messages (its
/// path for `ld.so PROGRAM`, else `argv[0]`).
// Indices stay below `n`, which never passes OBJECTS_MAX (checked before each store).
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::too_many_arguments)]
pub(super) fn link(prog: &start::Program, bias: usize, page: u64, name: &'static [u8], sp: usize, env: &List<'_>, origin: Origin<'_>, preloads: &Preloads<'_>) -> usize {
    use niji_ld::reloc::{self, Obj, Objects};
    let prog_name = name;
    let mut origin_at: Option<usize> = None;
    // The search state, used only for a name without `/`: each object's first requester and the
    // path it was opened by (empty: unknown, or the program's, from `origin`); the store for the
    // paths found (address space mapped at the first search hit, so it never runs out); the cache.
    let mut parent = [0u8; OBJECTS_MAX];
    let mut paths: [&'static [u8]; OBJECTS_MAX] = [&[]; OBJECTS_MAX];
    let mut store: &'static mut [u8] = &mut [];
    let mut store_map = (0usize, 0usize);
    let cache: search::LazyCache<'_> = core::cell::OnceCell::new();
    let mut maps = [(0usize, 0usize); OBJECTS_MAX];
    // The scratch mappings of the two image views: the program's, then the interpreter's own.
    let mut views = [(0usize, 0usize); 6];
    let (prog_views, own_views) = views.split_at_mut(3);
    let (head, view) = image_view(prog, bias, name, prog_views);
    // The program's own file is not known (no file is read): (dev, ino) that no file has.
    let mut ids: [FileId; OBJECTS_MAX] = [(u64::MAX, u64::MAX); OBJECTS_MAX];
    // Each object's run-time program header table (address, count), for `dlopen` (DL-16a).
    let mut phdrs = [(prog.phdr, prog.phnum); OBJECTS_MAX];
    let image = niji_ld::elf::Elf::from_image::<Native>(head, view).unwrap_or_else(|_| fail_with(&[name, b": malformed program headers\n"], program::FAILURE));
    let dynm = image.dynamic().ok().flatten().unwrap_or_else(|| fail_with(&[name, b": no valid dynamic section\n"], program::FAILURE));
    let first = Obj { elf: image, dynm, bias: u64::try_from(bias).unwrap_or(0), name, tls: None };
    let mut objs = [first; OBJECTS_MAX];
    let mut n = 1usize;
    let mut q = 0usize;
    // The interpreter itself, when an object names it (`__tls_get_addr`): already relocated, in the
    // scope with its exports, never relocated again (D-27.20); read from its memory image.
    let mut interp: Option<usize> = None;
    // The DT_NEEDED links (dependent, dependency) in the order each object names them (DL-11).
    let mut edges = [(0u8, 0u8); EDGES_MAX];
    let mut ne = 0usize;
    let mut edge = |from: usize, to: usize, name: &[u8]| {
        let (Ok(a), Ok(b)) = (u8::try_from(from), u8::try_from(to)) else { fail_with(&[name, b": too many objects\n"], program::FAILURE) };
        let Some(slot) = edges.get_mut(ne) else { fail_with(&[name, b": too many DT_NEEDED entries\n"], program::FAILURE) };
        *slot = (a, b);
        ne += 1;
    };
    // The preloads (DL-LD-PRELOAD) come first, as the program's (q == 0) names before its own
    // DT_NEEDED: after the program and before its dependencies in the scope, linked by no edge.
    let mut pending = preloads.names();
    while q < n {
        let needed = objs[q].dynm;
        let pre_names = core::iter::from_fn(|| if q == 0 { pending.next() } else { None }).map(|(p, from_env)| Ok((p, Some(from_env))));
        for name in pre_names.chain(needed.needed().map(|r| r.map(|p| (p, None)))) {
            let (name, pre) = name.unwrap_or_else(|_| fail_with(&[objs[q].name, b": malformed DT_NEEDED\n"], program::FAILURE));
            let rule = pre.map(|from_env| start::preload_rule(name, from_env, env.secure));
            if rule == Some(start::PreloadRule::Skip) {
                continue;
            }
            let bare = !name.contains(&b'/');
            // A name already loaded; a bare name also matches a loaded object's DT_SONAME (as the
            // dependency list and `dlopen` match it).
            if let Some(j) = objs[..n].iter().position(|o| o.name == name || (bare && dyn_string(&o.dynm, elf::dt::SONAME) == Some(name))) {
                if pre.is_none() {
                    edge(q, j, name);
                }
                continue;
            }
            let mut is_interp = name == Native::INTERP.as_bytes();
            if is_interp && pre.is_some() {
                continue;
            }
            if n == OBJECTS_MAX {
                fail_with(&[name, b": too many objects\n"], program::FAILURE);
            }
            let mut found = None;
            if (bare || pre.is_some()) && !is_interp {
                // The requester chain: this object, then each first requester up to the program.
                let mut chain = [Scope::default(); OBJECTS_MAX];
                let (mut depth, mut k) = (0usize, q);
                loop {
                    let at = if k == 0 {
                        let o = *origin_at.get_or_insert_with(|| origin.at());
                        if o == 0 { &[][..] } else { c_string(o) }
                    } else {
                        paths[k]
                    };
                    let d = &objs[k].dynm;
                    chain[depth] = Scope { rpath: dyn_string(d, elf::dt::RPATH), runpath: dyn_string(d, elf::dt::RUNPATH), origin: (!at.is_empty()).then(|| dir_of(at)) };
                    depth += 1;
                    if k == 0 || depth == OBJECTS_MAX {
                        break;
                    }
                    k = usize::from(parent[k]);
                }
                let ctx = env.ctx(CacheRef::Lazy(&cache, read_cache));
                let mut refused: Option<Refusal> = None;
                let mut at_path = PathBuf::default();
                let set_uid_only = rule == Some(start::PreloadRule::TrustedSetUid);
                let mut try_open = |p: &mut PathBuf| {
                    let Some(c) = p.c_str() else { return false };
                    match open_candidate(c, page) {
                        Err(Unusable::Skip) => return false,
                        Err(Unusable::Refused(r)) => refused = Some(r),
                        Ok((fd, at, file, _)) if set_uid_only && !start::preload_mode_admits(file_mode(fd)) => {
                            let _ = crate::ffi_sys::niji_sys_close(i64::from(fd));
                            let _ = raw::munmap(at, file.len());
                            return false;
                        }
                        Ok(hit) => found = Some(hit),
                    }
                    at_path.clear();
                    at_path.push(p.as_bytes());
                    true
                };
                if set_uid_only {
                    // Secure mode, an LD_PRELOAD name: each trusted directory itself, nothing else.
                    let mut cand = PathBuf::default();
                    search::trusted_dirs::<Native>(&mut |dir| {
                        cand.clear();
                        cand.push(dir) && cand.push(b"/") && cand.push(name) && try_open(&mut cand)
                    });
                } else {
                    search::search::<Native>(name, &chain[..depth], &ctx, &mut try_open);
                }
                if let Some(r) = refused {
                    if let Some(from_env) = pre {
                        preloads.ignored(name, from_env, r.at_preload());
                        continue;
                    }
                    let (text, by_path) = r.at_start();
                    fail_loading(prog_name, if by_path { at_path.as_bytes() } else { name }, text)
                }
                if found.is_none() {
                    if let Some(from_env) = pre {
                        preloads.ignored(name, from_env, start::PRELOAD_MISSING);
                        continue;
                    }
                    fail_loading(prog_name, name, niji_ld::dlopen::FILE_MISSING)
                }
                // Kept for its `$ORIGIN` (the store is address space for OBJECTS_MAX full paths).
                if store_map.0 == 0 {
                    let len = OBJECTS_MAX * search::PATH_MAX;
                    let (at, buf) = scratch(len, prog_name);
                    (store_map, store) = ((at, len), buf);
                }
                let p = at_path.as_bytes();
                if p.len() > store.len() {
                    fail_with(&[p, b": too many objects\n"], program::FAILURE);
                }
                let (kept, rest) = core::mem::take(&mut store).split_at_mut(p.len());
                kept.copy_from_slice(p);
                store = rest;
                paths[n] = kept;
            }
            let id = if is_interp {
                (u64::MAX, u64::MAX)
            } else {
                let (fd, at, file, id) = found.unwrap_or_else(|| {
                    // The string table entry is NUL-terminated in the mapped file.
                    open_file(name.as_ptr().expose_provenance()).unwrap_or_else(|| fail_loading(prog_name, name, niji_ld::dlopen::FILE_MISSING))
                });
                found = Some((fd, at, file, id));
                // `ld.so` by any name it was found by is the start-time `ld.so` (ruling #5126).
                is_interp = bare && dynamic_of(file).and_then(|d| dyn_string(&d, elf::dt::SONAME)) == Some(Native::INTERP.as_bytes());
                id
            };
            // Another path to an object already loaded (L14-4): the same object, loaded once.
            let again = if is_interp { interp } else { ids[..n].iter().position(|x| *x == id) };
            if is_interp || again.is_some() {
                if let Some((fd, at, file, _)) = found {
                    let _ = crate::ffi_sys::niji_sys_close(i64::from(fd));
                    let _ = raw::munmap(at, file.len());
                }
            }
            if pre.is_some() && (is_interp || again.is_some()) {
                continue;
            }
            if let Some(j) = again {
                edge(q, j, name);
                continue;
            }
            if is_interp {
                objs[n] = own_object(name, own_views);
                paths[n] = &[];
                parent[n] = u8::try_from(q).unwrap_or(0);
                edge(q, n, name);
                interp = Some(n);
                n += 1;
                continue;
            }
            let Some((fd, at, file, id)) = found else { continue };
            if !bare && pre.is_none() {
                paths[n] = name;
            }
            let loaded = map::load::<Native, _>(&mut SysMapper { fd }, file, page);
            let _ = crate::ffi_sys::niji_sys_close(i64::from(fd));
            let loaded = loaded.unwrap_or_else(|e| fail_with(&[name, b": ", e.message()], program::FAILURE));
            maps[n] = (at, file.len());
            ids[n] = id;
            phdrs[n] = (usize::try_from(loaded.phdr).unwrap_or(0), usize::from(loaded.phnum));
            objs[n] = parse(file, name, loaded.bias);
            parent[n] = u8::try_from(q).unwrap_or(0);
            if pre.is_none() {
                edge(q, n, name);
            }
            n += 1;
        }
        q += 1;
    }
    for (k, o) in objs[..n].iter().enumerate() {
        if Some(k) != interp {
            report_nonshared(&o.elf, o.name);
        }
    }
    // Static TLS of the initially loaded objects (design §7, R14-4, D-27.20).
    let mut mods: [Option<Module>; OBJECTS_MAX] = [None; OBJECTS_MAX];
    for (k, (m, o)) in mods.iter_mut().zip(&objs[..n]).enumerate() {
        if Some(k) == interp {
            continue;
        }
        *m = o.elf.find(niji_ld::elf::pt::TLS).map(|ph| Module::of(&ph));
        if m.is_some_and(|m| !ltls::image_in_load(&o.elf, &m)) {
            fail_with(&[o.name, b": the TLS image lies outside the object's segments\n"], program::FAILURE);
        }
    }
    let mut placed: [Option<Placed>; OBJECTS_MAX] = [None; OBJECTS_MAX];
    let area = ltls::layout::<Native>(&mods[..n], &mut placed[..n], INITIAL_TAIL, ltls::SURPLUS).unwrap_or_else(|e| {
        let at = match e {
            ltls::TlsError::Align(k) | ltls::TlsError::Size(k) => k,
            _ => 0,
        };
        fail_with(&[objs.get(at).map_or(&b"?"[..], |o| o.name), b": malformed or too large TLS segment\n"], program::FAILURE)
    });
    // Room in every thread's dtv for the modules `dlopen` may add (DL-16d): address space only.
    let area = u64::try_from(ltls::MODULES_MAX).ok().and_then(|s| area.with_dtv_slots(s)).unwrap_or_else(|| fail_with(&[b"ld.so: the TLS area is too large\n"], program::FAILURE));
    // The objects ld.so exports (R14-12, D-27.26): published before any object is relocated, so
    // a program's copy relocation of one reads its final value.
    let area_base = map_area(&area);
    __libc_stack_end.store(sp, Release);
    let rseq_size = register_rseq(area_base.wrapping_add(usize::try_from(area.tp).unwrap_or(0)));
    for (o, p) in objs[..n].iter_mut().zip(&placed[..n]) {
        o.tls = *p;
    }
    let objs = &objs[..n];
    if let Err(e) = reloc::check_versions(objs) {
        let by = |i: usize| objs.get(i).map_or(&b"?"[..], |o| o.name);
        match e {
            reloc::VersionError::NoVersions { by: b, file, version } => fail_with(&[file, b": version `", version, b"' not found (the object has no version information; required by ", by(b), b")\n"], program::FAILURE),
            reloc::VersionError::Missing { by: b, file, version } => fail_with(&[file, b": version `", version, b"' not found (required by ", by(b), b")\n"], 1),
            reloc::VersionError::Elf(b, _) => fail_with(&[by(b), b": malformed version tables\n"], program::FAILURE),
        }
    }
    let first_sym = niji_ld::symbols::Object::new(objs[0].dynm).unwrap_or_else(|_| fail_with(&[objs[0].name, b": malformed symbol tables\n"], program::FAILURE));
    let mut syms = [first_sym; OBJECTS_MAX];
    for (s, o) in syms.iter_mut().zip(objs).skip(1) {
        *s = niji_ld::symbols::Object::new(o.dynm).unwrap_or_else(|_| fail_with(&[o.name, b": malformed symbol tables\n"], program::FAILURE));
    }
    // The C library's private interface (NIJI_PRIVATE, L14-10, D-27.22): a library from another
    // build is refused before any of its code runs or any relocation is applied.
    let early = start::private::find(objs, &syms[..n], interp, |at, out| read_image(usize::try_from(at).unwrap_or(0), out)).unwrap_or_else(|e| {
        let (start::private::PrivateError::Mismatch(k) | start::private::PrivateError::Malformed(k)) = e;
        fail_with(&[objs.get(k).map_or(&b"?"[..], |o| o.name), e.message()], program::FAILURE)
    });
    let global: [usize; OBJECTS_MAX] = core::array::from_fn(|i| i);
    let set = Objects { objs, syms: &syms[..n], global: &global[..n] };
    // The start's lookups all search the global scope, whose order decides each unique name
    // (`ScopeOrder`): no table to fill, so no count of unique names can make the start fail.
    let mut unique = niji_ld::symbols::ScopeOrder;
    // Reverse load order (L14-8), the interpreter left out: it relocated itself at its start.
    let relocated = (0..n).rev().filter(|&k| Some(k) != interp).try_for_each(|k| reloc::relocate::<Native, _>(&set, k, &mut ProcImage, page, &mut unique).map_err(|error| reloc::Failure { obj: k, error }));
    if let Err(f) = relocated {
        let name = objs.get(f.obj).map_or(&b"?"[..], |o| o.name);
        match f.error {
            reloc::RelocError::Undefined(sym) => fail_with(&[name, b": ", f.error.message(), sym, b"\n"], program::FAILURE),
            e => fail_with(&[name, b": ", e.message()], program::FAILURE),
        }
    }
    if let Err(i) = reloc::protect_relro(objs, &mut ProcImage, page) {
        fail_with(&[objs.get(i).map_or(&b"?"[..], |o| o.name), b": cannot protect the RELRO segment\n"], program::FAILURE);
    }
    // The images are copied after relocation: a `.tdata` word may hold a relocated address.
    let biases: [u64; OBJECTS_MAX] = core::array::from_fn(|k| objs.get(k).map_or(0, |o| o.bias));
    install_tls(&mods[..n], &biases[..n], &placed[..n], &area, area_base);
    // The interpreter, when in the list, ran its own start already: no hooks of its own here.
    let hooks: [init::Hooks; OBJECTS_MAX] = core::array::from_fn(|i| objs.get(i).filter(|_| Some(i) != interp).map_or_else(init::Hooks::default, |o| init::hooks(&o.dynm, o.bias, i == 0)));
    // The cache, when a search read it, and the path store: no longer needed.
    let cache_map = cache.get().and_then(Option::as_ref).map_or((0, 0), |(f, _)| (f.as_ptr().expose_provenance(), f.len()));
    let preload_map = (preloads.file.as_ptr().expose_provenance(), preloads.file.len());
    for &(at, len) in maps[1..n].iter().chain(&views).chain([&store_map, &cache_map, &preload_map]).filter(|m| m.1 != 0) {
        let _ = raw::munmap(at, len);
    }
    for (k, (slot, o)) in START_OBJS.iter().zip(objs).enumerate() {
        let (phdr, phnum) = phdrs[k];
        let words = [u64::try_from(phdr).unwrap_or(0), u64::try_from(phnum).unwrap_or(0), o.bias, ids[k].0, ids[k].1];
        slot.iter().zip(words).for_each(|(w, v)| w.store(v, Relaxed));
    }
    START_INTERP.store(interp.unwrap_or(usize::MAX), Relaxed);
    START_PAGE.store(page, Relaxed);
    START_N.store(n, Release);
    run_init(&hooks[..n], &edges[..ne], sp, early.map(|f| f.early_init), rseq_size)
}

/// Most `DT_NEEDED` links over all objects of one start.
const EDGES_MAX: usize = 1024;

/// Where a kernel-started program's `$ORIGIN` comes from: the path it was run by (`ld.so PROGRAM`,
/// nonzero), else `AT_EXECFN` of `aux`, read only when a search needs it (a program whose
/// dependencies are all named by path never looks it up).
#[derive(Clone, Copy)]
pub(super) struct Origin<'a> {
    pub(super) path: usize,
    pub(super) aux: Auxv<'a>,
}

impl Origin<'_> {
    /// The NUL-terminated path, or 0 when unknown.
    fn at(self) -> usize {
        if self.path != 0 { self.path } else { self.aux.get(AT_EXECFN).and_then(|a| usize::try_from(a).ok()).unwrap_or(0) }
    }
}

/// The objects program start preloads (DL-LD-PRELOAD): the `LD_PRELOAD` names, then the preload
/// file's (ld.so(8) order; probed), each with whether it came from `LD_PRELOAD`.
pub(super) struct Preloads<'a> {
    pub(super) env: &'static [u8],
    pub(super) file: &'static [u8],
    pub(super) file_path: &'a [u8],
}

impl Preloads<'_> {
    pub(super) fn names(&self) -> impl Iterator<Item = (&'static [u8], bool)> {
        start::preload_names(self.env).map(|n| (n, true)).chain(start::preload_file_names(self.file).map(|n| (n, false)))
    }

    /// The "cannot be preloaded" diagnostic for `name` (from `LD_PRELOAD` when `from_env`).
    fn ignored(&self, name: &[u8], from_env: bool, reason: &[u8]) {
        for part in start::preload_ignored(name, if from_env { start::PRELOAD_ENV } else { self.file_path }, reason) {
            write_all(2, part);
        }
    }
}
