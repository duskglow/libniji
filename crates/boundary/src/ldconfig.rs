//! The shipped `ldconfig` program (spec §31.2; D-31.2: freestanding and static, no C library):
//! `niji_machine_ldconfig_entry` (`machine/entry.S`, `-DNIJI_LDCONFIG`) calls
//! [`niji_ldconfig_start`] with the initial stack pointer and the run-time address of the
//! program's own ELF header. It relocates the static-PIE image (the loader's pass 1), reads argv,
//! runs `niji_ld::ldconfig` over [`SysFs`] (the kernel's system calls; `-r` through
//! `openat2(RESOLVE_IN_ROOT)`, D-31.9) and writes what it printed. Its global allocator is
//! [`Arena`]: one short-lived, single-threaded process needs no general heap.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use niji_kernel::sys::Sys;
use niji_kernel::uapi::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE};
use niji_ld::arch::Native;
use niji_ld::ldconfig::{self, sysfs::SysFs};

use crate::ld::{c_string, exit, stack_word, write_all};
use crate::sys::CSys;

/// The entry of `ldconfig` (called by `niji_machine_ldconfig_entry` with the initial stack pointer
/// `sp` and the run-time address `ehdr` of the image's own ELF header).
// UNSAFE(export): the Rust side of the ldconfig entry primitive; the program's only entry.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn niji_ldconfig_start(sp: usize, ehdr: usize) -> ! {
    if crate::bootstrap::relocate_image(ehdr).is_err() {
        write_all(2, b"ldconfig: cannot relocate the program\n");
        exit(1)
    }
    // Every argument the kernel passed is read (its own limits bound them); a list that does not fit in memory is
    // the out-of-memory report below.
    let argc = usize::try_from(stack_word(sp)).unwrap_or(0);
    let mut args: alloc::vec::Vec<&'static [u8]> = alloc::vec::Vec::new();
    if args.try_reserve_exact(argc).is_err() {
        write_all(2, b"ldconfig: out of memory\n");
        exit(1)
    }
    for i in 0..argc {
        let p = stack_word(sp.wrapping_add(8).wrapping_add(i.wrapping_mul(8)));
        if ldconfig::push(&mut args, c_string(usize::try_from(p).unwrap_or(0))).is_err() {
            write_all(2, b"ldconfig: out of memory\n");
            exit(1)
        }
    }
    let argv0 = args.first().copied().unwrap_or(b"ldconfig");
    let rest = args.get(1..).unwrap_or_default();
    let mut fs = SysFs::new(&CSys, ldconfig::root_of(rest), niji::diag::description);
    let r = ldconfig::run::<Native, _>(argv0, rest, &mut fs, ldconfig::GENERATOR);
    drop(fs);
    write_all(1, &r.out.buf);
    write_all(2, &r.err.buf);
    exit(i32::from(r.status))
}

/// The panic handler of `ldconfig` (a panic is a bug): a fixed message, status 127. It formats
/// nothing.
pub fn panicked(_info: &core::panic::PanicInfo<'_>) -> ! {
    write_all(2, b"ldconfig: internal error\n");
    exit(127)
}

/// Bytes the arena asks the kernel for at once (bigger requests get a mapping of their own size).
const CHUNK: usize = 1 << 20;
/// The page size the mappings are rounded to.
const PAGE: usize = 4096;

/// The free space of the current mapping, `NEXT..END`, and the start of the block handed out last
/// (the only one that can grow in place or be given back).
static NEXT: AtomicUsize = AtomicUsize::new(0);
static END: AtomicUsize = AtomicUsize::new(0);
static LAST: AtomicUsize = AtomicUsize::new(0);

/// The global allocator of `ldconfig`: a bump arena over anonymous mappings. The last block grows
/// and shrinks in place and is returned when freed; other freed blocks stay until the process ends
/// (ldconfig runs once and exits; a run over a full system's libraries stays within a few MB).
pub struct Arena;

/// `size` bytes at `align` (a power of two), 0 without memory.
fn arena_alloc(size: usize, align: usize) -> usize {
    let mask = align.wrapping_sub(1);
    let next = NEXT.load(Relaxed);
    if let Some(start) = next.checked_add(mask).map(|n| n & !mask) {
        if start.checked_add(size).is_some_and(|e| e <= END.load(Relaxed)) && next != 0 {
            LAST.store(start, Relaxed);
            NEXT.store(start.wrapping_add(size), Relaxed);
            return start;
        }
    }
    let Some(need) = size.checked_add(mask).and_then(|n| n.checked_add(PAGE - 1)).map(|n| n & !(PAGE - 1)) else { return 0 };
    let len = need.max(CHUNK);
    let Ok(region) = CSys.mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, None, 0) else { return 0 };
    let base = region.addr();
    let start = base.wrapping_add(mask) & !mask;
    // A request bigger than a chunk gets a mapping of its own; the current one keeps serving the small ones.
    if len == CHUNK {
        NEXT.store(start.wrapping_add(size), Relaxed);
        END.store(base.wrapping_add(len), Relaxed);
        LAST.store(start, Relaxed);
    }
    start
}

// UNSAFE(allocator): implementing `GlobalAlloc` is the allocator category itself.
// SAFETY: blocks lie in never-unmapped private mappings and never overlap (only the last block, once freed or
// resized by its owner, is reused); sizes and alignments are honoured, exhaustion is null; one thread.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Arena {
    // UNSAFE(allocator): the trait's required method.
    #[allow(unsafe_code)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        core::ptr::with_exposed_provenance_mut(arena_alloc(layout.size(), layout.align()))
    }

    // UNSAFE(allocator): the trait's required method.
    #[allow(unsafe_code)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let a = ptr.expose_provenance();
        if a == LAST.load(Relaxed) && a.wrapping_add(layout.size()) == NEXT.load(Relaxed) {
            NEXT.store(a, Relaxed);
        }
    }

    // UNSAFE(allocator): the trait's provided method, overridden to grow the last block in place.
    #[allow(unsafe_code)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let a = ptr.expose_provenance();
        let in_place = a == LAST.load(Relaxed)
            && a.wrapping_add(layout.size()) == NEXT.load(Relaxed)
            && a.checked_add(new_size).is_some_and(|e| e <= END.load(Relaxed));
        if in_place {
            NEXT.store(a.wrapping_add(new_size), Relaxed);
            return ptr;
        }
        let new = arena_alloc(new_size, layout.align());
        if new == 0 {
            return core::ptr::null_mut();
        }
        let dst: *mut u8 = core::ptr::with_exposed_provenance_mut(new);
        // UNSAFE(allocator): moving a block's bytes to its new place.
        // SAFETY: `ptr` is a live block of `layout.size()` bytes (the trait's contract) and `dst` a
        // fresh block of `new_size` bytes that overlaps no live block; the copy is the smaller size.
        unsafe { core::ptr::copy_nonoverlapping(ptr, dst, layout.size().min(new_size)) };
        dst
    }
}
