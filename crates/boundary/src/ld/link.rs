//! Program start's link (DL-6 to DL-10): the `DT_NEEDED` search from the program and the
//! preloads (`LD_PRELOAD`, `/etc/ld.so.preload`), mapping, versions, relocation and symbol
//! binding (`STB_GNU_UNIQUE` included), the TLS layout, the state `dlopen` reads later, and the
//! debugger rendezvous's start-time link maps (DL-23; [`super::debug`] fills the structure).

use core::sync::atomic::Ordering::{Relaxed, Release};
use niji_kernel::auxv::Auxv;
use niji_kernel::uapi::AT_EXECFN;
use niji_ld::arch::{Arch, Native};
use niji_ld::program;
use niji_ld::dlopen::Refusal;
use niji_ld::search::{self, CacheRef, PathBuf, Scope};
use niji_ld::{boot, elf, init, map, start};
use niji_ld::tls::{self as ltls, Module, Placed};
use crate::raw;
use super::table::{PathStore, Table, INLINE};
use super::{debug, dlopen};
use super::entry::{List, __libc_stack_end, register_rseq, report_nonshared};
use super::exec_stack::exec_stack_dependency;
use super::image::{FileId, ProcImage, SysMapper, Unusable, dir_of, dyn_string, dynamic_of, file_mode, image_view, open_candidate, open_file, own_object, parse, read_cache, read_image};
use super::initfini::run_init;
use super::io::{c_string, fail_loading, fail_with, no_memory, write_all};
use super::state::{OWN_BASE, START_INTERP, START_N, START_OBJS, START_PAGE, START_SPILL};
use super::tls::{INITIAL_TAIL, install_tls, map_area};

/// Load the program's dependencies, check their versions, relocate everything and protect the RELRO
/// pages (DL-10, design §6, D-27.17). A `DT_NEEDED` name with `/` is opened as given; one without is
/// found by [`search::search`], the search `--list` and `dlopen` use (DL-START-SEARCH): the requester
/// chain is each object's first requester up to the program, `$ORIGIN` the directory of the path an
/// object was opened by (the program's: from `origin`, looked up at the first search), and the
/// cache is read only when a search reaches it. The program is read from its memory image
/// ([`image_view`], ruling #4229, D-27.18), never from its file; `name` names it in messages (its
/// path for `ld.so PROGRAM`, else `argv[0]`).
// Indices stay below `n`; every per-object table is reserved to `n + 1` entries before each store.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn link(prog: &start::Program, bias: usize, page: u64, name: &'static [u8], sp: usize, env: &List<'_>, origin: Origin<'_>, preloads: &Preloads<'_>) -> usize {
    use niji_ld::reloc::{self, Obj, Objects};
    let prog_name = name;
    let mut origin_at: Option<usize> = None;
    // The search state, used only for a name without `/`: each object's first requester and the
    // path it was opened by (empty: unknown, or the program's, from `origin`); the store for the
    // paths found (address space mapped at the first search hit, so it never runs out); the cache.
    let mut parent: Table<usize, INLINE> = Table::new(0);
    let mut paths: Table<&'static [u8], INLINE> = Table::new(&[]);
    let mut store = PathStore::default();
    let cache: search::LazyCache<'_> = core::cell::OnceCell::new();
    let mut maps: Table<(usize, usize), INLINE> = Table::new((0, 0));
    // The scratch mappings of the two image views: the program's, then the interpreter's own.
    let mut views = [(0usize, 0usize); 6];
    let (prog_views, own_views) = views.split_at_mut(3);
    let (head, view) = image_view(prog, bias, name, prog_views, false);
    // The program's own file is not known (no file is read): (dev, ino) that no file has.
    let mut ids: Table<FileId, INLINE> = Table::new((u64::MAX, u64::MAX));
    // Each object's run-time program header table (address, count), for `dlopen` (DL-16a).
    let mut phdrs: Table<(usize, usize), INLINE> = Table::new((prog.phdr, prog.phnum));
    let image = niji_ld::elf::Elf::from_image::<Native>(head, view).unwrap_or_else(|_| fail_with(&[name, b": malformed program headers\n"], program::FAILURE));
    let dynm = image.dynamic().ok().flatten().unwrap_or_else(|| fail_with(&[name, b": no valid dynamic section\n"], program::FAILURE));
    let first = Obj { elf: image, dynm, bias: u64::try_from(bias).unwrap_or(0), name, tls: None };
    let mut objs: Table<Obj<'static>, INLINE> = Table::new(first);
    // Entry 0, the program, in every per-object table (inline: nothing can fail).
    let grown = [parent.reserve(1), paths.reserve(1), maps.reserve(1), ids.reserve(1), phdrs.reserve(1), objs.reserve(1)];
    if grown.iter().any(Result::is_err) {
        no_memory(name);
    }
    let mut n = 1usize;
    let mut q = 0usize;
    // The interpreter itself, when an object names it (`__tls_get_addr`): already relocated, in the
    // scope with its exports, never relocated again (D-27.20); read from its memory image.
    let mut interp: Option<usize> = None;
    // The DT_NEEDED links (dependent, dependency) in the order each object names them (DL-11).
    let mut edges: Table<(usize, usize), EDGES_INLINE> = Table::new((0, 0));
    let mut ne = 0usize;
    let mut edge = |from: usize, to: usize, name: &[u8]| {
        edges.reserve(ne + 1).unwrap_or_else(|_| no_memory(name));
        edges[ne] = (from, to);
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
            // Room for object `n` in every per-object table (each holds INLINE entries without a mapping).
            let grown = [parent.reserve(n + 1), paths.reserve(n + 1), maps.reserve(n + 1), ids.reserve(n + 1), phdrs.reserve(n + 1), objs.reserve(n + 1)];
            if grown.iter().any(Result::is_err) {
                no_memory(name);
            }
            let mut found = None;
            if (bare || pre.is_some()) && !is_interp {
                // The requester chain: this object, then each first requester up to the program.
                let mut chain: Table<Scope<'_>, INLINE> = Table::new(Scope::default());
                let (mut depth, mut k) = (0usize, q);
                loop {
                    chain.reserve(depth + 1).unwrap_or_else(|_| no_memory(name));
                    let at = if k == 0 {
                        let o = *origin_at.get_or_insert_with(|| origin.at());
                        if o == 0 { &[][..] } else { c_string(o) }
                    } else {
                        paths[k]
                    };
                    let d = &objs[k].dynm;
                    chain[depth] = Scope { rpath: dyn_string(d, elf::dt::RPATH), runpath: dyn_string(d, elf::dt::RUNPATH), origin: (!at.is_empty()).then(|| dir_of(at)) };
                    depth += 1;
                    // Each first requester was loaded before its object: the chain ends at the program.
                    if k == 0 || depth > n {
                        break;
                    }
                    k = parent[k];
                }
                let ctx = env.ctx(CacheRef::Lazy(&cache, read_cache));
                let mut refused: Option<Refusal> = None;
                let mut at_path = PathBuf::default();
                let set_uid_only = rule == Some(start::PreloadRule::TrustedSetUid);
                let mut try_open = |p: &mut PathBuf| {
                    let Some(c) = p.c_str() else { return false };
                    match open_candidate(c, page, env) {
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
                // Kept for its `$ORIGIN`.
                paths[n] = store.keep(at_path.as_bytes()).unwrap_or_else(|_| no_memory(name));
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
                parent[n] = q;
                edge(q, n, name);
                interp = Some(n);
                n += 1;
                continue;
            }
            let Some((fd, at, file, id)) = found else { continue };
            if !bare && pre.is_none() {
                // Kept (NUL-terminated) as its link map's name: the DT_NEEDED string is in a mapping
                // released below.
                paths[n] = store.keep(name).unwrap_or_else(|_| no_memory(name));
            }
            let loaded = match map::load::<Native, _>(&mut SysMapper { fd }, file, page) {
                Err(map::MapError::ExecutableStack) => exec_stack_dependency(fd, file, page, name, prog_name, env),
                other => other,
            };
            let _ = crate::ffi_sys::niji_sys_close(i64::from(fd));
            let loaded = loaded.unwrap_or_else(|e| fail_with(&[name, b": ", e.message()], program::FAILURE));
            maps[n] = (at, file.len());
            ids[n] = id;
            phdrs[n] = (usize::try_from(loaded.phdr).unwrap_or(0), usize::from(loaded.phnum));
            objs[n] = parse(file, name, loaded.bias);
            parent[n] = q;
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
    let mut mods: Table<Option<Module>, INLINE> = Table::new(None);
    let mut placed: Table<Option<Placed>, INLINE> = Table::new(None);
    if mods.reserve(n).is_err() || placed.reserve(n).is_err() {
        no_memory(prog_name);
    }
    for (k, (m, o)) in mods.iter_mut().zip(&objs[..n]).enumerate() {
        if Some(k) == interp {
            continue;
        }
        *m = o.elf.find(niji_ld::elf::pt::TLS).map(|ph| Module::of(&ph));
        if m.is_some_and(|m| !ltls::image_in_load(&o.elf, &m)) {
            fail_with(&[o.name, b": the TLS image lies outside the object's segments\n"], program::FAILURE);
        }
    }
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
    rendezvous_start(&objs[..n], &paths[..n], interp, env.vdso.and_then(|v| usize::try_from(v).ok()).unwrap_or(0));
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
    // The program's view: its canonical PLT entries define their functions (DL-CANONICAL-PLT).
    let mut first_sym = niji_ld::symbols::Object::new(objs[0].dynm).unwrap_or_else(|_| fail_with(&[objs[0].name, b": malformed symbol tables\n"], program::FAILURE));
    first_sym.mark_program();
    let mut syms: Table<niji_ld::symbols::Object<'static>, INLINE> = Table::new(first_sym);
    let mut global: Table<usize, INLINE> = Table::new(0);
    if syms.reserve(n).is_err() || global.reserve(n).is_err() {
        no_memory(prog_name);
    }
    for (s, o) in syms.iter_mut().zip(objs).skip(1) {
        *s = niji_ld::symbols::Object::new(o.dynm).unwrap_or_else(|_| fail_with(&[o.name, b": malformed symbol tables\n"], program::FAILURE));
    }
    // The C library's private interface (NIJI_PRIVATE, L14-10, D-27.22): a library from another
    // build is refused before any of its code runs or any relocation is applied.
    let early = start::private::find(objs, &syms[..n], interp, |at, out| read_image(usize::try_from(at).unwrap_or(0), out)).unwrap_or_else(|e| {
        let (start::private::PrivateError::Mismatch(k) | start::private::PrivateError::Malformed(k)) = e;
        fail_with(&[objs.get(k).map_or(&b"?"[..], |o| o.name), e.message()], program::FAILURE)
    });
    for (k, g) in global.iter_mut().enumerate() {
        *g = k;
    }
    let set = Objects { objs, syms: &syms[..n], global: &global[..n] };
    // The start's lookups all search the global scope, whose order decides each unique name
    // (`ScopeOrder`): no table to fill, so no count of unique names can make the start fail.
    let mut unique = niji_ld::symbols::ScopeOrder;
    // Reverse load order (L14-8), the interpreter left out: it relocated itself at its start.
    let relocated = (0..n).rev().filter(|&k| Some(k) != interp).try_for_each(|k| reloc::relocate::<Native, _>(&set, k, &mut ProcImage, page, &mut unique).map_err(|error| reloc::Failure { obj: k, error }));
    if let Err(f) = relocated {
        let name = objs.get(f.obj).map_or(&b"?"[..], |o| o.name);
        match f.error {
            reloc::RelocError::Undefined(sym) | reloc::RelocError::IndirectExternAccess(sym) => fail_with(&[name, b": ", f.error.message(), sym, b"\n"], program::FAILURE),
            e => fail_with(&[name, b": ", e.message()], program::FAILURE),
        }
    }
    // Each dynamic array's address entries as run-time addresses, before any user code (DL-DYNAMIC-RELOC);
    // the interpreter published its own at its start.
    for (_, o) in objs.iter().enumerate().filter(|&(k, _)| Some(k) != interp) {
        reloc::publish_dynamic(o, &mut ProcImage);
    }
    if let Err(i) = reloc::protect_relro(objs, &mut ProcImage, page) {
        fail_with(&[objs.get(i).map_or(&b"?"[..], |o| o.name), b": cannot protect the RELRO segment\n"], program::FAILURE);
    }
    // The program's headers are read here, before their views are unmapped below.
    debug::keep_copy(|| {
        reloc::relro_range(&objs[0], page).map_or((0, 0), |(at, len)| (usize::try_from(at).unwrap_or(0), usize::try_from(at.saturating_add(len)).unwrap_or(usize::MAX)))
    });
    // The images are copied after relocation: a `.tdata` word may hold a relocated address.
    let mut biases: Table<u64, INLINE> = Table::new(0);
    let mut hooks: Table<init::Hooks, INLINE> = Table::new(init::Hooks::default());
    if biases.reserve(n).is_err() || hooks.reserve(n).is_err() {
        no_memory(prog_name);
    }
    for (b, o) in biases.iter_mut().zip(objs) {
        *b = o.bias;
    }
    install_tls(&mods[..n], &biases[..n], &placed[..n], &area, area_base);
    // The interpreter, when in the list, ran its own start already: no hooks of its own here.
    for (i, (h, o)) in hooks.iter_mut().zip(objs).enumerate() {
        if Some(i) != interp {
            *h = init::hooks(&o.dynm, o.bias, i == 0);
        }
    }
    // The cache, when a search read it, is no longer needed; the path store is kept: its paths are the
    // start-time link maps' names (DL-23).
    let cache_map = cache.get().and_then(Option::as_ref).map_or((0, 0), |(f, _)| (f.as_ptr().expose_provenance(), f.len()));
    let preload_map = (preloads.file.as_ptr().expose_provenance(), preloads.file.len());
    for &(at, len) in maps[1..n].iter().chain(&views).chain([&cache_map, &preload_map]).filter(|m| m.1 != 0) {
        let _ = raw::munmap(at, len);
    }
    let words = |k: usize, o: &Obj<'_>| {
        let (phdr, phnum) = phdrs[k];
        [u64::try_from(phdr).unwrap_or(0), u64::try_from(phnum).unwrap_or(0), o.bias, ids[k].0, ids[k].1]
    };
    if n <= INLINE {
        for (k, (slot, o)) in START_OBJS.iter().zip(objs).enumerate() {
            slot.iter().zip(words(k, o)).for_each(|(w, v)| w.store(v, Relaxed));
        }
    } else {
        // More objects than the static records hold: all of them in a kept table instead.
        let mut rows: Table<[u64; 5], 0> = Table::new([0; 5]);
        rows.reserve(n).unwrap_or_else(|_| no_memory(prog_name));
        for (k, (row, o)) in rows.iter_mut().zip(objs).enumerate() {
            *row = words(k, o);
        }
        START_SPILL.keep(rows, n).unwrap_or_else(|_| no_memory(prog_name));
    }
    START_INTERP.store(interp.unwrap_or(usize::MAX), Relaxed);
    START_PAGE.store(page, Relaxed);
    START_N.store(n, Release);
    debug::consistent();
    run_init(&hooks[..n], &edges[..ne], sp, early.map(|f| f.early_init), rseq_size)
}

/// `DT_DEBUG`: the dynamic entry the loader points at the debugger rendezvous.
const DT_DEBUG: u64 = 21;

/// The debugger rendezvous at the start (DL-23, [`debug`]): link the start-time objects' link maps
/// in load order (the program `""`, the vDSO `linux-vdso.so.1` when `vdso` (`AT_SYSINFO_EHDR`) is
/// not 0, each other object by the path the start opened it by, `ld.so` at its place, or last when
/// no object named it, by the program's `PT_INTERP`): the order `dl_iterate_phdr` reports. Then
/// fill `_r_debug`, `DT_DEBUG` and call `r_brk` with `RT_ADD`. Before relocation: a program's `COPY`
/// of `_r_debug` then reads the filled structure. Pointer writes only: no allocation, no file read.
// Indices stay below `objs.len()` (== paths.len()). Run once, out of `link`'s line: `link`'s code
// stays as it was apart from the call.
#[allow(clippy::indexing_slicing)]
#[cold]
#[inline(never)]
fn rendezvous_start(objs: &[niji_ld::reloc::Obj<'static>], paths: &[&'static [u8]], interp: Option<usize>, vdso: usize) {
    let n = objs.len();
    let own_base = OWN_BASE.load(Relaxed);
    let empty = c"".as_ptr().expose_provenance();
    let ld_of = |o: &niji_ld::reloc::Obj<'_>| usize::try_from(o.bias.wrapping_add(o.elf.find(elf::pt::DYNAMIC).map_or(0, |p| p.vaddr))).unwrap_or(0);
    // The previous link map: its address (0: none yet) and its start-time index (None: the vDSO's).
    let mut prev: (usize, Option<usize>) = (0, None);
    let mut put = |k: usize, l_addr: usize, name: usize, l_ld: usize| {
        let at = dlopen::start_node(k, l_addr, name, l_ld, prev.0);
        if at == 0 {
            return 0;
        }
        match prev {
            (0, _) => {}
            (_, Some(p)) => dlopen::link_next(p, at),
            (_, None) => debug::vdso_next(at),
        }
        prev = (at, Some(k));
        if k == 0 && vdso != 0 {
            let v = debug::vdso_node(vdso, at);
            dlopen::link_next(0, v);
            prev = (v, None);
        }
        at
    };
    let mut head = 0;
    for (k, o) in objs.iter().enumerate() {
        let name = if Some(k) == interp {
            interp_name(&objs[0])
        } else if k == 0 || paths[k].is_empty() {
            empty
        } else {
            paths[k].as_ptr().expose_provenance()
        };
        let at = put(k, usize::try_from(o.bias).unwrap_or(0), name, ld_of(o));
        if k == 0 {
            head = at;
        }
    }
    if interp.is_none() {
        put(n, own_base, interp_name(&objs[0]), own_dynamic(own_base));
    }
    let dt_debug = niji_ld::reloc::dynamic_word(&objs[0], DT_DEBUG).map_or(0, |w| usize::try_from(w).unwrap_or(0));
    debug::start(head, own_base, dt_debug);
}

/// `ld.so`'s `l_ld`: `base` plus its `PT_DYNAMIC`'s `p_vaddr`, read from its own mapped header table
/// as the `dl_iterate_phdr` walk reads it (no view, no parse: a few words per start); 0 when the
/// headers are malformed or name no `PT_DYNAMIC`.
fn own_dynamic(base: usize) -> usize {
    use niji_ld::elf::{EHDR_SIZE, PHDR_SIZE};
    let mut header = [0u8; EHDR_SIZE];
    read_image(base, &mut header);
    let Some((phoff, phnum)) = boot::own_headers(&header) else { return 0 };
    let table = base.wrapping_add(usize::try_from(phoff).unwrap_or(0));
    let mut ph = [0u8; PHDR_SIZE];
    for i in 0..phnum {
        read_image(table.wrapping_add(i.wrapping_mul(PHDR_SIZE)), &mut ph);
        let word = |o: usize| ph.get(o..o.saturating_add(8)).and_then(|b| <[u8; 8]>::try_from(b).ok()).map_or(0, u64::from_ne_bytes);
        if word(0) & 0xffff_ffff == u64::from(elf::pt::DYNAMIC) {
            return base.wrapping_add(usize::try_from(word(16)).unwrap_or(0));
        }
    }
    0
}

/// `ld.so`'s link map name: the program's `PT_INTERP` string in its memory image when a loaded
/// segment holds it, NUL-terminated; else `""`.
fn interp_name(prog: &niji_ld::reloc::Obj<'_>) -> usize {
    let empty = c"".as_ptr().expose_provenance();
    let Some(ph) = prog.elf.find(elf::pt::INTERP).filter(|ph| ph.filesz > 0) else { return empty };
    let end = ph.vaddr.saturating_add(ph.filesz);
    if !prog.elf.program_headers().any(|s| s.p_type == elf::pt::LOAD && s.vaddr <= ph.vaddr && end <= s.vaddr.saturating_add(s.filesz)) {
        return empty;
    }
    let Ok(at) = usize::try_from(prog.bias.wrapping_add(ph.vaddr)) else { return empty };
    let Ok(len) = usize::try_from(ph.filesz) else { return empty };
    let mut last = [0u8; 1];
    read_image(at.wrapping_add(len).wrapping_sub(1), &mut last);
    if last[0] == 0 { at } else { empty }
}

/// `DT_NEEDED` links one start holds without a mapping (more grow the table): twice [`INLINE`].
const EDGES_INLINE: usize = 2 * INLINE;

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
