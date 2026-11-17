//! Process primitives every part of the loader uses: writes to a descriptor, `exit_group`, reads
//! of the initial stack and of C strings, and the fatal start-up diagnostics.

use core::ffi::c_void;
use niji_ld::program;
use crate::conv;
use crate::ffi_sys::{niji_sys_exit_group, niji_sys_write};

/// The longest argument read (an option is short; a longer argument is only echoed, cut here).
const ARG_MAX_READ: usize = 4096;

/// Write all of `bytes` to `fd` (short writes continued; errors give up: there is nowhere to report them).
pub(crate) fn write_all(fd: i64, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let n = niji_sys_write(fd, bytes.as_ptr().cast::<c_void>(), i64::try_from(bytes.len()).unwrap_or(i64::MAX));
        match usize::try_from(n) {
            Ok(n) if n > 0 => bytes = bytes.get(n..).unwrap_or(&[]),
            _ => return,
        }
    }
}

/// End the process with `status`.
pub(crate) fn exit(status: i32) -> ! {
    loop {
        niji_sys_exit_group(i64::from(status));
    }
}

/// The word at `addr` on the initial stack.
pub(crate) fn stack_word(addr: usize) -> u64 {
    // UNSAFE(slice): one word of the initial stack (argc, argv[], envp[], auxv).
    #[allow(unsafe_code)]
    // SAFETY: the kernel laid out argc, argv[] and NULL, envp[] and NULL and the auxiliary vector
    // ending in AT_NULL contiguously from the entry stack pointer (⚑ KERN, execve); callers walk
    // that layout in order and never past its terminators.
    let w = unsafe { conv::words_at(addr, 1) };
    w.first().copied().unwrap_or(0)
}

/// The NUL-terminated string at `addr` (an argv element), without its NUL, at most [`ARG_MAX_READ`] bytes.
pub(crate) fn c_string(addr: usize) -> &'static [u8] {
    let mut len = 0usize;
    while len < ARG_MAX_READ {
        // UNSAFE(slice): one byte of an argument string.
        #[allow(unsafe_code)]
        // SAFETY: argv strings are NUL-terminated in the kernel's copy on the initial stack
        // (⚑ KERN); bytes are read one at a time up to and excluding the NUL.
        let b = unsafe { conv::bytes_at(addr.wrapping_add(len), 1) };
        if b.first().is_none_or(|&b| b == 0) {
            break;
        }
        len = len.wrapping_add(1);
    }
    // UNSAFE(slice): the argument string measured above.
    #[allow(unsafe_code)]
    // SAFETY: the `len` bytes just read one by one are mapped and stay unchanged for the process.
    unsafe { conv::bytes_at(addr, len) }
}

/// End the process with `msg` on standard error and status 127 (design §1 "Failure").
pub(super) fn refuse(msg: &[u8]) -> ! {
    write_all(2, msg);
    exit(program::FAILURE)
}

/// The low 32 bits of `w`.
pub(super) fn lo32(w: u64) -> u32 {
    u32::try_from(w & 0xffff_ffff).unwrap_or(0)
}

/// Print `parts` after `ld.so: ` on standard error and end with `status`.
pub(super) fn fail_with(parts: &[&[u8]], status: i32) -> ! {
    write_all(2, b"ld.so: ");
    parts.iter().for_each(|p| write_all(2, p));
    exit(status)
}

/// End the start for an object that cannot be loaded, as the reference does (DL-START-SEARCH
/// probe): `<prog>: error while loading shared libraries: <what>: <text>`, status 127.
pub(super) fn fail_loading(prog: &[u8], what: &[u8], text: &[u8]) -> ! {
    for part in [prog, &b": error while loading shared libraries: "[..], what, b": ", text, b"\n"] {
        write_all(2, part);
    }
    exit(program::FAILURE)
}
