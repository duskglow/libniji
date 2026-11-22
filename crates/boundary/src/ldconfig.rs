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

/// Bytes the arena asks the kernel for at once for small blocks.
const CHUNK: usize = 1 << 20;
/// The page size the mappings are rounded to.
const PAGE: usize = 4096;
/// The smallest size class (2^4 bytes: room for the free-list link, and malloc's alignment).
const MIN_SHIFT: u32 = 4;
/// The largest size class (2^16 bytes); a bigger block gets a mapping of its own.
const MAX_SHIFT: u32 = 16;
/// The number of size classes.
const CLASSES: usize = 13; // MAX_SHIFT - MIN_SHIFT + 1
/// The free space of the current chunk, `NEXT..END`.
static NEXT: AtomicUsize = AtomicUsize::new(0);
static END: AtomicUsize = AtomicUsize::new(0);
/// Per size class, the first freed block (0: none); each freed block holds the address of the next.
static FREE: [AtomicUsize; CLASSES] = [const { AtomicUsize::new(0) }; CLASSES];

/// The global allocator of `ldconfig` and `localedef` (D-31.23): a block of at most 64 KiB is
/// rounded up to a power of two, taken from that size's free list or cut from a 1 MiB chunk at
/// its own alignment, and put back on the list when freed; a bigger block is a mapping of its own,
/// grown with `mremap` and unmapped when freed. One thread. (A bump arena that kept every freed
/// block but the last made localedef's peak RSS 218 to 338 MB per locale, #7535.)
pub struct Arena;

/// The size class of a block of `size` bytes at `align`, `None` for a block that gets its own mapping.
fn class_of(size: usize, align: usize) -> Option<usize> {
    let s = size.max(align).max(1 << MIN_SHIFT);
    if s > 1 << MAX_SHIFT {
        return None;
    }
    usize::try_from(s.next_power_of_two().trailing_zeros().saturating_sub(MIN_SHIFT)).ok()
}

/// The bytes of size class `c`.
fn class_size(c: usize) -> usize {
    u32::try_from(c).ok().and_then(|c| (1usize << MIN_SHIFT).checked_shl(c)).unwrap_or(0)
}

/// `len` rounded up to whole pages (`None` on overflow).
fn pages(len: usize) -> Option<usize> {
    len.checked_add(PAGE - 1).map(|n| n & !(PAGE - 1))
}

/// A block of size class `c`, aligned to its size; 0 without memory.
// UNSAFE(allocator): pops the free list, whose link lives in the freed block.
#[allow(unsafe_code)]
fn small_alloc(c: usize) -> usize {
    let Some(list) = FREE.get(c) else { return 0 };
    let head = list.load(Relaxed);
    if head != 0 {
        let p: *const usize = core::ptr::with_exposed_provenance(head);
        // UNSAFE(allocator): reading the free-list link stored in a freed block.
        // SAFETY: `head` is a block of at least 16 bytes, aligned to its size, that `dealloc` put on
        // this list after writing the next block's address in its first word; nothing else uses it.
        list.store(unsafe { p.read() }, Relaxed);
        return head;
    }
    let size = class_size(c);
    let mask = size.wrapping_sub(1);
    let next = NEXT.load(Relaxed);
    if next != 0 {
        if let Some(start) = next.checked_add(mask).map(|n| n & !mask) {
            if start.checked_add(size).is_some_and(|e| e <= END.load(Relaxed)) {
                NEXT.store(start.wrapping_add(size), Relaxed);
                return start;
            }
        }
    }
    // A new chunk; what is left of the old one (less than the largest class) is not used again.
    let Ok(region) = CSys.mmap(0, CHUNK, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, None, 0) else { return 0 };
    let base = region.addr();
    let start = base.wrapping_add(mask) & !mask;
    NEXT.store(start.wrapping_add(size), Relaxed);
    END.store(base.wrapping_add(CHUNK), Relaxed);
    start
}

/// A block of `size` bytes at `align` in a mapping of its own; 0 without memory. Alignments up to a
/// page are the mapping's own; a bigger one maps the slack too (kept until the process ends).
fn large_alloc(size: usize, align: usize) -> usize {
    let slack = if align > PAGE { align } else { 0 };
    let Some(len) = size.checked_add(slack).and_then(pages) else { return 0 };
    let Ok(region) = CSys.mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, None, 0) else { return 0 };
    let mask = align.wrapping_sub(1);
    region.addr().wrapping_add(mask) & !mask
}

/// Give back the block at `a` of `size` bytes at `align`.
// UNSAFE(allocator): pushes on the free list, whose link lives in the freed block.
#[allow(unsafe_code)]
fn free_block(a: usize, size: usize, align: usize) {
    match class_of(size, align) {
        Some(c) => {
            let Some(list) = FREE.get(c) else { return };
            let p: *mut usize = core::ptr::with_exposed_provenance_mut(a);
            // UNSAFE(allocator): storing the free-list link in the block being freed.
            // SAFETY: `a` is a live block of class `c` (at least 16 bytes, aligned to its size) that
            // its owner gives back now and never uses again.
            unsafe { p.write(list.load(Relaxed)) };
            list.store(a, Relaxed);
        }
        None => {
            if let Some(len) = pages(size) {
                // The block's own pages (for an alignment above a page, the slack before it stays mapped).
                let _ = crate::raw::munmap(a, len);
            }
        }
    }
}

// UNSAFE(allocator): implementing `GlobalAlloc` is the allocator category itself.
// SAFETY: a small block is handed out once from a chunk or a free list and is on a list only while
// freed; a large block is a mapping no other block shares; sizes and alignments are honoured
// (class blocks are aligned to their size, which is at least the alignment); exhaustion is null;
// one thread.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Arena {
    // UNSAFE(allocator): the trait's required method.
    #[allow(unsafe_code)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let a = match class_of(layout.size(), layout.align()) {
            Some(c) => small_alloc(c),
            None => large_alloc(layout.size(), layout.align()),
        };
        core::ptr::with_exposed_provenance_mut(a)
    }
    // UNSAFE(allocator): the trait's required method.
    #[allow(unsafe_code)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        free_block(ptr.expose_provenance(), layout.size(), layout.align());
    }
    // UNSAFE(allocator): the trait's provided method, overridden: a block that stays in its class
    // stays put, and a large one grows or shrinks with `mremap`.
    #[allow(unsafe_code)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let a = ptr.expose_provenance();
        let (old, new) = (class_of(layout.size(), layout.align()), class_of(new_size, layout.align()));
        match (old, new) {
            (Some(o), Some(n)) if o == n => return ptr,
            (None, None) if layout.align() <= PAGE => {
                let (Some(old_len), Some(new_len)) = (pages(layout.size()), pages(new_size)) else { return core::ptr::null_mut() };
                if old_len == new_len {
                    return ptr;
                }
                return match crate::raw::mremap(a, old_len, new_len) {
                    Ok(moved) => core::ptr::with_exposed_provenance_mut(moved),
                    Err(_) => core::ptr::null_mut(),
                };
            }
            _ => {}
        }
        let fresh = match new {
            Some(c) => small_alloc(c),
            None => large_alloc(new_size, layout.align()),
        };
        if fresh == 0 {
            return core::ptr::null_mut();
        }
        let dst: *mut u8 = core::ptr::with_exposed_provenance_mut(fresh);
        // UNSAFE(allocator): moving a block's bytes to its new place.
        // SAFETY: `ptr` is a live block of `layout.size()` bytes (the trait's contract) and `dst` a
        // fresh block of `new_size` bytes that overlaps no live block; the copy is the smaller size.
        unsafe { core::ptr::copy_nonoverlapping(ptr, dst, layout.size().min(new_size)) };
        free_block(a, layout.size(), layout.align());
        dst
    }
}
