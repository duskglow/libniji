//! Slice 6 (plan/headers.md section 4: minimal C program): functions its headers declare that link from elsewhere.
//! Listed by `xtask/build.rs` (plan/headers.md section 4, "Adding a slice").
use super::super::Slice;

/// No default-version export of the shared library provides these: `atexit`, `at_quick_exit` and `pthread_atfork`
/// come from the static archive that links into every program, and `crypt` from the crypt library (plan/headers.md
/// section 1: `crypt.h` is not libniji's, `unistd.h` declares the function).
pub(super) const SLICE: Slice = Slice { markers: &[], inline: &[], linked_elsewhere: &["atexit", "at_quick_exit", "pthread_atfork", "crypt"] };
