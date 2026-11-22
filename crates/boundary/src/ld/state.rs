//! The loader's shared start-time state: the interpreter's own base, the table of start-time
//! objects `link` records for `dlopen` (DL-16a; inline, then spilled to a mapping), and the bounds
//! the parts share.

use core::sync::atomic::{AtomicU64, AtomicUsize};
use core::sync::atomic::Ordering::Relaxed;
use super::table::{Kept, INLINE};

/// The run-time address of the interpreter's own ELF header (its load bias: `ld.so` is linked at 0),
/// stored once its image is relocated.
pub(super) static OWN_BASE: AtomicUsize = AtomicUsize::new(0);

/// What `dlopen` needs to see the start-time objects again (DL-16a, D-27.34; nothing is allocated at
/// start for up to [`INLINE`] objects): per object, in load order, its run-time program header table
/// address and count, its bias and its file's (`st_dev`, `st_ino`) (the program's: none, `u64::MAX`).
/// Read through [`start_obj`].
pub(super) static START_OBJS: [[AtomicU64; 5]; INLINE] = [const { [const { AtomicU64::new(0) }; 5] }; INLINE];
/// The same records for a start of more than [`INLINE`] objects (DL-STARTUP-OBJECTS), all of them.
pub(super) static START_SPILL: Kept<[u64; 5]> = Kept::new();

/// Start-time object `k`'s record ([`START_OBJS`]); zeros past the last.
pub(super) fn start_obj(k: usize) -> [u64; 5] {
    let spill = START_SPILL.get();
    if spill.is_empty() {
        START_OBJS.get(k).map_or([0; 5], |s| [0, 1, 2, 3, 4].map(|j| s.get(j).map_or(0, |w| w.load(Relaxed))))
    } else {
        spill.get(k).copied().unwrap_or([0; 5])
    }
}
/// Start-time objects recorded in [`START_OBJS`] (0: `link` did not run).
pub(super) static START_N: AtomicUsize = AtomicUsize::new(0);
/// Index of `ld.so` itself among them (`usize::MAX`: not among them).
pub(super) static START_INTERP: AtomicUsize = AtomicUsize::new(usize::MAX);
/// The page size.
pub(super) static START_PAGE: AtomicU64 = AtomicU64::new(0);
/// The program's `$ORIGIN` as the start looked it up (address and length in the start's path
/// store; length 0: not looked up, or unknown), and the path it was run by under `ld.so PROGRAM`
/// (0: started by the kernel), for `dlopen`'s own look-up (DL-RUNPATH, `link::program_origin`).
pub(super) static START_ORIGIN: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
pub(super) static START_RUN_PATH: AtomicUsize = AtomicUsize::new(0);
/// The program's `argc`, `argv`, `envp` and auxiliary vector as `run_init` found them on the initial
/// stack, for `dlopen` (its constructors' arguments, its search's environment). Recorded, never
/// re-read from the stack: after `ld.so PROGRAM` the start's words are rearranged (argv[0] dropped).
pub(super) static START_ARGS: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];

/// `ld.so --library-path PATH` (address and length + 1 of PATH; 0: not given) and `--inhibit-cache`
/// (1), kept by program mode for `dlopen`, whose search uses them as the start's did (probed).
static PROGRAM_OPTIONS: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];

/// Keep program mode's `--library-path` and `--inhibit-cache` for `dlopen` ([`program_options`]).
pub(super) fn keep_program_options(library_path: Option<&'static [u8]>, inhibit_cache: bool) {
    let [at, len, inhibit] = &PROGRAM_OPTIONS;
    if let Some(p) = library_path {
        at.store(p.as_ptr().expose_provenance(), Relaxed);
        len.store(p.len().wrapping_add(1), Relaxed);
    }
    inhibit.store(usize::from(inhibit_cache), Relaxed);
}

/// Program mode's `--library-path` (`Some` when given) and whether `--inhibit-cache` was given.
pub(super) fn program_options() -> (Option<&'static [u8]>, bool) {
    let [at, len, inhibit] = &PROGRAM_OPTIONS;
    let n = len.load(Relaxed);
    let path = n.checked_sub(1).map(|n| {
        // UNSAFE(slice): the `--library-path` value, one of the loader's own arguments.
        #[allow(unsafe_code)]
        // SAFETY: `keep_program_options` stored a NUL-terminated argument string of the initial
        // stack and its length; the stack's strings live for the process's lifetime.
        unsafe {
            crate::conv::bytes_at(at.load(Relaxed), n)
        }
    });
    (path, inhibit.load(Relaxed) != 0)
}

/// Words the loader copies of the auxiliary vector (128 pairs, `AT_NULL` included).
pub(super) const AUXV_WORDS_MAX: usize = 256;
/// Words per program header.
pub(super) const PHDR_WORDS: usize = niji_rt::start::PHDR_WORDS;
