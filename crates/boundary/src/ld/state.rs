//! The loader's shared start-time state: the interpreter's own base, the table of start-time
//! objects `link` records for `dlopen` (DL-16a), and the bounds the parts share.

use core::sync::atomic::{AtomicU64, AtomicUsize};

/// The run-time address of the interpreter's own ELF header (its load bias: `ld.so` is linked at 0),
/// stored once its image is relocated.
pub(super) static OWN_BASE: AtomicUsize = AtomicUsize::new(0);

/// Objects one start can load (the program included).
pub(super) const OBJECTS_MAX: usize = 64;

/// What `dlopen` needs to see the start-time objects again (DL-16a, D-27.34; nothing is allocated at
/// start): per object, in load order, its run-time program header table address and count, its
/// bias and its file's (`st_dev`, `st_ino`) (the program's: none, `u64::MAX`).
pub(super) static START_OBJS: [[AtomicU64; 5]; OBJECTS_MAX] = [const { [const { AtomicU64::new(0) }; 5] }; OBJECTS_MAX];
/// Start-time objects recorded in [`START_OBJS`] (0: `link` did not run).
pub(super) static START_N: AtomicUsize = AtomicUsize::new(0);
/// Index of `ld.so` itself among them (`usize::MAX`: not among them).
pub(super) static START_INTERP: AtomicUsize = AtomicUsize::new(usize::MAX);
/// The page size.
pub(super) static START_PAGE: AtomicU64 = AtomicU64::new(0);
/// The program's `argc`, `argv`, `envp` and auxiliary vector as `run_init` found them on the initial
/// stack, for `dlopen` (its constructors' arguments, its search's environment). Recorded, never
/// re-read from the stack: after `ld.so PROGRAM` the start's words are rearranged (argv[0] dropped).
pub(super) static START_ARGS: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];

/// Words the loader copies of the auxiliary vector (128 pairs, `AT_NULL` included).
pub(super) const AUXV_WORDS_MAX: usize = 256;
/// Words per program header.
pub(super) const PHDR_WORDS: usize = niji_rt::start::PHDR_WORDS;
