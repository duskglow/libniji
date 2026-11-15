# Header and reserved-name corpus scan (SL-HEADERS-1, part a)

Date: 2026-11-15. Tool: `validation/headers_corpus.py` (README: `validation/headers-corpus-README.md`). TSV: `headers-corpus-2026-11-15.tsv`.

Source: Devuan excalibur `main/source/Sources.xz`, chain verified (InRelease with gpgv and the pinned keyring, then each index and every source file against its SHA-256). Program sources only: any `lib/`, `gnulib/` or `gnulib-tests/` path component was skipped unread (log below). Counts are `occurrences`, `files`, `packages` (distinct source packages).

## Sets

* **essential**: 20 source packages named, 19 scanned: base-files, base-passwd, bash, coreutils, dash, debianutils, diffutils, dpkg, findutils, glibc, grep, gzip, hostname, init-system-helpers, ncurses, perl, sed, sysvinit, tar, util-linux
* **build-essential**: 7 source packages named, 6 scanned: binutils, build-essential, dpkg, gcc-14, gcc-defaults, glibc, make-dfsg
* **smoke**: 15 source packages named, 15 scanned: bash, coreutils, cppunit, diffutils, findutils, gawk, grep, lua5.4, make-dfsg, perl, python3.13, sed, sqlite3, xz-utils, zlib

Excluded by name or not in main:

* `glibc` 2.41-12+deb13u4: C library source package (by name)

Scanned 116418 source files in 31 packages.

## Set `all`: top counts

### `#include <...>` (by packages, top 25)

| name | occurrences | files | packages |
|---|---|---|---|
| `stdlib.h` | 3791 | 3728 | 27 |
| `stdio.h` | 3274 | 3224 | 27 |
| `string.h` | 2482 | 2437 | 27 |
| `unistd.h` | 1240 | 1205 | 27 |
| `errno.h` | 896 | 880 | 26 |
| `ctype.h` | 382 | 375 | 26 |
| `sys/types.h` | 834 | 814 | 25 |
| `sys/stat.h` | 494 | 481 | 25 |
| `stdarg.h` | 1281 | 1276 | 24 |
| `limits.h` | 759 | 749 | 24 |
| `stddef.h` | 818 | 783 | 22 |
| `fcntl.h` | 474 | 465 | 22 |
| `stdint.h` | 1496 | 1487 | 21 |
| `signal.h` | 385 | 361 | 21 |
| `time.h` | 305 | 287 | 21 |
| `assert.h` | 1175 | 1166 | 19 |
| `sys/time.h` | 210 | 192 | 19 |
| `dirent.h` | 123 | 123 | 19 |
| `inttypes.h` | 223 | 216 | 18 |
| `sys/wait.h` | 108 | 106 | 18 |
| `locale.h` | 110 | 103 | 17 |
| `getopt.h` | 251 | 251 | 16 |
| `sys/param.h` | 151 | 143 | 16 |
| `setjmp.h` | 111 | 106 | 16 |
| `wchar.h` | 118 | 111 | 15 |

### direct `bits/` and `gnu/` includes

| name | occurrences | files | packages |
|---|---|---|---|
| `bits/long-double.h` | 2 | 2 | 2 |
| `bits/wordsize.h` | 2 | 2 | 2 |
| `bits/c++config.h` | 120 | 120 | 1 |
| `bits/move.h` | 40 | 40 | 1 |
| `bits/extc++.h` | 37 | 37 | 1 |
| `bits/requires_hosted.h` | 37 | 37 | 1 |
| `bits/functexcept.h` | 30 | 30 | 1 |
| `bits/stdc++.h` | 28 | 23 | 1 |
| `bits/stl_algobase.h` | 19 | 19 | 1 |
| `bits/stl_function.h` | 17 | 17 | 1 |
| `bits/exception_defines.h` | 16 | 16 | 1 |
| `bits/stl_pair.h` | 16 | 16 | 1 |
| `bits/concept_check.h` | 15 | 15 | 1 |
| `bits/functional_hash.h` | 15 | 15 | 1 |
| `bits/stl_iterator_base_types.h` | 15 | 15 | 1 |
| `bits/c++0x_warning.h` | 14 | 14 | 1 |
| `bits/version.h` | 14 | 14 | 1 |
| `bits/cpp_type_traits.h` | 10 | 10 | 1 |
| `bits/stl_iterator.h` | 10 | 10 | 1 |
| `bits/allocator.h` | 9 | 9 | 1 |
| `bits/c++locale_internal.h` | 9 | 9 | 1 |
| `bits/stl_algo.h` | 8 | 8 | 1 |
| `bits/stl_iterator_base_funcs.h` | 8 | 8 | 1 |
| `bits/chrono.h` | 7 | 7 | 1 |
| `bits/cxxabi_forced.h` | 7 | 7 | 1 |
| `bits/ptr_traits.h` | 7 | 7 | 1 |
| `bits/gthr.h` | 6 | 6 | 1 |
| `bits/invoke.h` | 6 | 6 | 1 |
| `bits/stdtr1c++.h` | 6 | 6 | 1 |
| `bits/atomic_lockfree_defines.h` | 5 | 5 | 1 |
| `bits/atomic_word.h` | 5 | 5 | 1 |
| `bits/largefile-config.h` | 5 | 5 | 1 |
| `bits/stl_construct.h` | 5 | 5 | 1 |
| `bits/unique_ptr.h` | 5 | 5 | 1 |
| `bits/utility.h` | 5 | 5 | 1 |
| `bits/iterator_concepts.h` | 4 | 4 | 1 |
| `bits/localefwd.h` | 4 | 4 | 1 |
| `bits/memoryfwd.h` | 4 | 4 | 1 |
| `bits/ranges_base.h` | 4 | 4 | 1 |
| `bits/shared_ptr.h` | 4 | 4 | 1 |
| `bits/alloc_traits.h` | 3 | 3 | 1 |
| `bits/exception_ptr.h` | 3 | 3 | 1 |
| `bits/libc-lock.h` | 3 | 3 | 1 |
| `bits/locale_classes.h` | 3 | 3 | 1 |
| `bits/predefined_ops.h` | 3 | 3 | 1 |
| `bits/refwrap.h` | 3 | 3 | 1 |
| `bits/std_mutex.h` | 3 | 3 | 1 |
| `bits/stl_uninitialized.h` | 3 | 3 | 1 |
| `bits/streambuf_iterator.h` | 3 | 3 | 1 |
| `bits/uniform_int_dist.h` | 3 | 3 | 1 |
| `bits/uses_allocator.h` | 3 | 3 | 1 |
| `bits/allocated_ptr.h` | 2 | 2 | 1 |
| `bits/atomic_base.h` | 2 | 2 | 1 |
| `bits/atomic_wait.h` | 2 | 2 | 1 |
| `bits/boost_concept_check.h` | 2 | 2 | 1 |
| `bits/c++io.h` | 2 | 2 | 1 |
| `bits/char_traits.h` | 2 | 2 | 1 |
| `bits/codecvt.h` | 2 | 2 | 1 |
| `bits/compatibility.h` | 2 | 1 | 1 |
| `bits/cxxabi_init_exception.h` | 2 | 2 | 1 |

### identity macros

| name | occurrences | files | packages |
|---|---|---|---|
| `__GLIBC__` | 483 | 129 | 14 |
| `__GLIBC_MINOR__` | 100 | 42 | 8 |
| `__GNU_LIBRARY__` | 72 | 37 | 5 |
| `__GLIBC_PREREQ` | 35 | 16 | 2 |

### `__USE_*`

| name | occurrences | files | packages |
|---|---|---|---|
| `__USE_MINGW_ANSI_STDIO` | 21 | 11 | 7 |
| `__USE_GNU` | 33 | 12 | 6 |
| `__USE_LARGEFILE64` | 22 | 10 | 3 |
| `__USE_FILE_OFFSET64` | 20 | 11 | 3 |
| `__USE_MISC` | 4 | 4 | 3 |
| `__USE_FORTIFY_LEVEL` | 11 | 3 | 2 |
| `__USE_EXTERN_INLINES` | 9 | 4 | 2 |
| `__USE_INLINE__` | 3 | 3 | 2 |
| `__USE_XOPEN2K` | 2 | 2 | 2 |
| `__USE_INIT_FINI__` | 12 | 12 | 1 |
| `__USE_RUNCOMMAND__` | 5 | 2 | 1 |
| `__USE_ISOC11` | 4 | 1 | 1 |
| `__USE_TIME_BITS64` | 3 | 2 | 1 |
| `__USE_FIXED_PROTOTYPES__` | 2 | 1 | 1 |
| `__USE_BSD` | 1 | 1 | 1 |
| `__USE_ISOC99` | 1 | 1 | 1 |
| `__USE_POSIX2` | 1 | 1 | 1 |
| `__USE_POSIX_IMPLICITLY` | 1 | 1 | 1 |
| `__USE_STRING_INLINES` | 1 | 1 | 1 |
| `__USE_SYSTEM_ENDIAN_H__` | 1 | 1 | 1 |
| `__USE_XOPEN2K8` | 1 | 1 | 1 |

### `sys/cdefs.h` vocabulary

| name | occurrences | files | packages |
|---|---|---|---|
| `__P` | 1158 | 57 | 6 |
| `__glibc_unlikely` | 733 | 28 | 5 |
| `__glibc_likely` | 103 | 29 | 5 |
| `__attribute_warn_unused_result__` | 93 | 14 | 5 |
| `__THROW` | 57 | 13 | 4 |
| `__attribute_nonnull__` | 45 | 9 | 4 |
| `__attribute_maybe_unused__` | 23 | 9 | 4 |
| `__attribute_pure__` | 20 | 10 | 4 |
| `__nonnull` | 15 | 7 | 4 |
| `__REDIRECT` | 31 | 8 | 3 |
| `__ASMNAME` | 28 | 5 | 3 |
| `__NTH` | 16 | 7 | 3 |
| `__BEGIN_DECLS` | 15 | 9 | 3 |
| `__END_DECLS` | 15 | 9 | 3 |
| `__extern_inline` | 12 | 7 | 3 |
| `__ASMNAME2` | 9 | 5 | 3 |
| `__STRING` | 8 | 5 | 3 |
| `__CONCAT` | 7 | 6 | 3 |
| `__attribute_const__` | 7 | 5 | 3 |
| `__attribute_deprecated_with_message__` | 3 | 3 | 3 |
| `__glibc_has_attribute` | 38 | 6 | 2 |
| `__glibc_clang_prereq` | 16 | 4 | 2 |
| `__LDBL_REDIR1_NTH` | 14 | 4 | 2 |
| `__THROWNL` | 14 | 4 | 2 |
| `__REDIRECT_NTH` | 13 | 5 | 2 |
| `__LDBL_REDIR1` | 12 | 4 | 2 |
| `__extern_always_inline` | 12 | 4 | 2 |
| `__glibc_safe_len_cond` | 12 | 4 | 2 |
| `__glibc_has_extension` | 10 | 4 | 2 |
| `__LDBL_REDIR` | 8 | 4 | 2 |
| `__LDBL_REDIR2_DECL` | 8 | 4 | 2 |
| `__LDBL_REDIR_DECL` | 8 | 4 | 2 |
| `__LDBL_REDIR_NTH` | 8 | 4 | 2 |
| `__NTHNL` | 8 | 4 | 2 |
| `__REDIRECT_LDBL` | 8 | 4 | 2 |
| `__REDIRECT_NTH_LDBL` | 8 | 4 | 2 |
| `__attribute_artificial__` | 8 | 4 | 2 |
| `__attribute_deprecated__` | 8 | 4 | 2 |
| `__attribute_noinline__` | 8 | 6 | 2 |
| `__glibc_has_builtin` | 8 | 4 | 2 |
| `__glibc_macro_warning` | 8 | 4 | 2 |
| `__glibc_safe_or_unknown_len` | 8 | 4 | 2 |
| `__glibc_unsafe_len` | 8 | 4 | 2 |
| `__glibc_unsigned_or_positive` | 8 | 4 | 2 |
| `__wur` | 8 | 4 | 2 |
| `__bos` | 7 | 4 | 2 |
| `__bos0` | 7 | 4 | 2 |
| `__LDBL_REDIR1_DECL` | 6 | 4 | 2 |
| `__REDIRECT_NTHNL` | 6 | 4 | 2 |
| `__attribute_alloc_align__` | 6 | 4 | 2 |
| `__attribute_alloc_size__` | 6 | 4 | 2 |
| `__attribute_copy__` | 6 | 2 | 2 |
| `__attribute_deprecated_msg__` | 6 | 4 | 2 |
| `__attribute_format_arg__` | 6 | 4 | 2 |
| `__attribute_format_strfmon__` | 6 | 4 | 2 |
| `__attribute_malloc__` | 6 | 4 | 2 |
| `__attribute_nonstring__` | 6 | 4 | 2 |
| `__attribute_returns_twice__` | 6 | 4 | 2 |
| `__attribute_used__` | 6 | 4 | 2 |
| `__glibc_objsize` | 6 | 4 | 2 |

### reserved `__*_t` names (top 25)

| name | occurrences | files | packages |
|---|---|---|---|
| `__time64_t` | 53 | 6 | 4 |
| `__locale_t` | 19 | 12 | 4 |
| `__pid_t` | 10 | 6 | 4 |
| `__uint128_t` | 246 | 72 | 3 |
| `__wchar_t` | 5 | 3 | 3 |
| `__re_size_t` | 20 | 10 | 2 |
| `__int64_t` | 9 | 7 | 2 |
| `__need_ptrdiff_t` | 9 | 2 | 2 |
| `__need_size_t` | 9 | 2 | 2 |
| `__need_wchar_t` | 9 | 2 | 2 |
| `__need_wint_t` | 9 | 3 | 2 |
| `__off_t` | 9 | 5 | 2 |
| `__kernel_time_t` | 8 | 5 | 2 |
| `__re_long_size_t` | 8 | 2 | 2 |
| `__ptr_t` | 6 | 4 | 2 |
| `__wint_t` | 4 | 2 | 2 |
| `__ino64_t` | 3 | 3 | 2 |
| `__kernel_gid_t` | 3 | 3 | 2 |
| `__kernel_off_t` | 3 | 3 | 2 |
| `__kernel_uid_t` | 3 | 3 | 2 |
| `__int128_t` | 378 | 89 | 1 |
| `__vector_type_t` | 163 | 8 | 1 |
| `__gthread_mutex_t` | 148 | 35 | 1 |
| `__sanitizer_sigset_t` | 101 | 18 | 1 |
| `__int_for_sizeof_t` | 89 | 10 | 1 |

### FILE members

| name | occurrences | files | packages |
|---|---|---|---|
| `_flags` | 22 | 9 | 5 |
| `_IO_write_base` | 8 | 5 | 2 |
| `_IO_read_end` | 5 | 4 | 2 |
| `_IO_write_ptr` | 4 | 4 | 2 |
| `_IO_read_ptr` | 3 | 3 | 2 |
| `_IO_save_base` | 3 | 3 | 2 |
| `_fileno` | 3 | 2 | 2 |
| `_flags2` | 30 | 1 | 1 |
| `_IO_read_base` | 6 | 3 | 1 |
| `_offset` | 5 | 1 | 1 |
| `_IO_write_end` | 4 | 3 | 1 |
| `_IO_backup_base` | 2 | 2 | 1 |
| `_IO_buf_base` | 2 | 2 | 1 |
| `_IO_buf_end` | 2 | 2 | 1 |
| `_IO_save_end` | 2 | 2 | 1 |

## Set `essential`: top counts

### `#include <...>` (by packages, top 25)

| name | occurrences | files | packages |
|---|---|---|---|
| `stdio.h` | 750 | 742 | 17 |
| `stdlib.h` | 611 | 603 | 17 |
| `unistd.h` | 579 | 571 | 17 |
| `string.h` | 540 | 532 | 17 |
| `errno.h` | 486 | 479 | 17 |
| `sys/stat.h` | 285 | 276 | 17 |
| `ctype.h` | 221 | 221 | 17 |
| `sys/types.h` | 461 | 455 | 16 |
| `limits.h` | 166 | 163 | 15 |
| `stdarg.h` | 99 | 99 | 15 |
| `fcntl.h` | 241 | 239 | 14 |
| `getopt.h` | 241 | 241 | 13 |
| `signal.h` | 139 | 139 | 13 |
| `stddef.h` | 132 | 125 | 13 |
| `dirent.h` | 87 | 87 | 13 |
| `stdint.h` | 181 | 178 | 11 |
| `time.h` | 137 | 134 | 11 |
| `sys/time.h` | 91 | 86 | 11 |
| `inttypes.h` | 84 | 82 | 11 |
| `sys/wait.h` | 59 | 59 | 11 |
| `wchar.h` | 58 | 56 | 10 |
| `pwd.h` | 57 | 57 | 10 |
| `locale.h` | 48 | 47 | 10 |
| `config.h` | 593 | 593 | 9 |
| `sys/ioctl.h` | 73 | 72 | 9 |

### direct `bits/` and `gnu/` includes

| name | occurrences | files | packages |
|---|---|---|---|
| `bits/libc-lock.h` | 3 | 3 | 1 |
| `bits/long-double.h` | 1 | 1 | 1 |
| `bits/sigcontext.h` | 1 | 1 | 1 |
| `bits/wordsize.h` | 1 | 1 | 1 |

### identity macros

| name | occurrences | files | packages |
|---|---|---|---|
| `__GLIBC__` | 287 | 57 | 8 |
| `__GLIBC_MINOR__` | 39 | 17 | 6 |
| `__GNU_LIBRARY__` | 9 | 7 | 1 |

### `__USE_*`

| name | occurrences | files | packages |
|---|---|---|---|
| `__USE_MINGW_ANSI_STDIO` | 10 | 6 | 4 |
| `__USE_GNU` | 14 | 4 | 3 |
| `__USE_FORTIFY_LEVEL` | 6 | 2 | 1 |
| `__USE_RUNCOMMAND__` | 5 | 2 | 1 |
| `__USE_EXTERN_INLINES` | 3 | 2 | 1 |
| `__USE_TIME_BITS64` | 3 | 2 | 1 |
| `__USE_INLINE__` | 2 | 2 | 1 |
| `__USE_ISOC99` | 1 | 1 | 1 |
| `__USE_MISC` | 1 | 1 | 1 |
| `__USE_XOPEN2K` | 1 | 1 | 1 |

### `sys/cdefs.h` vocabulary

| name | occurrences | files | packages |
|---|---|---|---|
| `__attribute_warn_unused_result__` | 49 | 9 | 4 |
| `__glibc_unlikely` | 369 | 13 | 3 |
| `__glibc_likely` | 43 | 12 | 3 |
| `__attribute_nonnull__` | 24 | 6 | 3 |
| `__attribute_maybe_unused__` | 13 | 6 | 3 |
| `__THROW` | 31 | 7 | 2 |
| `__P` | 15 | 3 | 2 |
| `__attribute__visibility__` | 671 | 3 | 1 |
| `__attribute__warn_unused_result__` | 486 | 3 | 1 |
| `__attribute__format__` | 41 | 3 | 1 |
| `__attribute__noreturn__` | 40 | 5 | 1 |
| `__attribute__pure__` | 23 | 2 | 1 |
| `__attribute__deprecated__` | 20 | 3 | 1 |
| `__glibc_has_attribute` | 20 | 4 | 1 |
| `__attribute__malloc__` | 15 | 2 | 1 |
| `__ASMNAME` | 13 | 2 | 1 |
| `__REDIRECT` | 13 | 3 | 1 |
| `__NTH` | 9 | 3 | 1 |
| `__BEGIN_DECLS` | 8 | 5 | 1 |
| `__END_DECLS` | 8 | 5 | 1 |
| `__attribute__always_inline__` | 8 | 3 | 1 |
| `__attribute__unused__` | 8 | 2 | 1 |
| `__glibc_clang_prereq` | 8 | 2 | 1 |
| `__LDBL_REDIR1_NTH` | 7 | 2 | 1 |
| `__THROWNL` | 7 | 2 | 1 |
| `__attribute__format__null_ok__` | 7 | 2 | 1 |
| `__attribute_pure__` | 7 | 4 | 1 |
| `__LDBL_REDIR1` | 6 | 2 | 1 |
| `__REDIRECT_NTH` | 6 | 2 | 1 |
| `__attribute_noreturn__` | 6 | 2 | 1 |
| `__extern_always_inline` | 6 | 2 | 1 |
| `__glibc_safe_len_cond` | 6 | 2 | 1 |
| `__glibc_has_extension` | 5 | 2 | 1 |
| `__nonnull` | 5 | 2 | 1 |
| `__LDBL_REDIR` | 4 | 2 | 1 |
| `__LDBL_REDIR2_DECL` | 4 | 2 | 1 |
| `__LDBL_REDIR_DECL` | 4 | 2 | 1 |
| `__LDBL_REDIR_NTH` | 4 | 2 | 1 |
| `__NTHNL` | 4 | 2 | 1 |
| `__REDIRECT_LDBL` | 4 | 2 | 1 |
| `__REDIRECT_NTH_LDBL` | 4 | 2 | 1 |
| `__attribute__nonnull__` | 4 | 2 | 1 |
| `__attribute_artificial__` | 4 | 2 | 1 |
| `__attribute_deprecated__` | 4 | 2 | 1 |
| `__attribute_noinline__` | 4 | 3 | 1 |
| `__bos` | 4 | 2 | 1 |
| `__bos0` | 4 | 2 | 1 |
| `__extern_inline` | 4 | 3 | 1 |
| `__glibc_has_builtin` | 4 | 2 | 1 |
| `__glibc_macro_warning` | 4 | 2 | 1 |
| `__glibc_safe_or_unknown_len` | 4 | 2 | 1 |
| `__glibc_unsafe_len` | 4 | 2 | 1 |
| `__glibc_unsigned_or_positive` | 4 | 2 | 1 |
| `__wur` | 4 | 2 | 1 |
| `__ASMNAME2` | 3 | 2 | 1 |
| `__CONCAT` | 3 | 3 | 1 |
| `__LDBL_REDIR1_DECL` | 3 | 2 | 1 |
| `__REDIRECT_NTHNL` | 3 | 2 | 1 |
| `__STRING` | 3 | 2 | 1 |
| `__attribute_alloc_align__` | 3 | 2 | 1 |

### reserved `__*_t` names (top 25)

| name | occurrences | files | packages |
|---|---|---|---|
| `__time64_t` | 29 | 4 | 2 |
| `__pid_t` | 4 | 2 | 2 |
| `__locale_t` | 2 | 2 | 2 |
| `__strtol_t` | 11 | 3 | 1 |
| `__re_size_t` | 10 | 5 | 1 |
| `__uint128_t` | 7 | 1 | 1 |
| `__need_wint_t` | 5 | 2 | 1 |
| `__re_long_size_t` | 4 | 1 | 1 |
| `__need_time_t` | 3 | 2 | 1 |
| `__ptr_t` | 3 | 2 | 1 |
| `__int64_t` | 2 | 1 | 1 |
| `__need_locale_t` | 2 | 1 | 1 |
| `__need_ptrdiff_t` | 2 | 1 | 1 |
| `__need_size_t` | 2 | 1 | 1 |
| `__need_wchar_t` | 2 | 1 | 1 |
| `__priority_which_t` | 2 | 1 | 1 |
| `__fsword_t` | 1 | 1 | 1 |
| `__ino64_t` | 1 | 1 | 1 |
| `__kernel_off_t` | 1 | 1 | 1 |
| `__kernel_time_t` | 1 | 1 | 1 |
| `__need___off64_t` | 1 | 1 | 1 |
| `__need_clock_t` | 1 | 1 | 1 |
| `__need_error_t` | 1 | 1 | 1 |
| `__need_mbstate_t` | 1 | 1 | 1 |
| `__need_off_t` | 1 | 1 | 1 |

### FILE members

| name | occurrences | files | packages |
|---|---|---|---|
| `_flags` | 18 | 7 | 3 |
| `_offset` | 5 | 1 | 1 |
| `_IO_write_base` | 2 | 2 | 1 |
| `_IO_write_ptr` | 2 | 2 | 1 |
| `_IO_read_end` | 1 | 1 | 1 |
| `_IO_read_ptr` | 1 | 1 | 1 |
| `_IO_save_base` | 1 | 1 | 1 |
| `_fileno` | 1 | 1 | 1 |

## Set `build-essential`: top counts

### `#include <...>` (by packages, top 25)

| name | occurrences | files | packages |
|---|---|---|---|
| `stdlib.h` | 2926 | 2882 | 4 |
| `stdio.h` | 2282 | 2251 | 4 |
| `string.h` | 1642 | 1609 | 4 |
| `stdint.h` | 1273 | 1267 | 4 |
| `stdarg.h` | 1133 | 1128 | 4 |
| `assert.h` | 915 | 910 | 4 |
| `stddef.h` | 568 | 545 | 4 |
| `unistd.h` | 558 | 538 | 4 |
| `limits.h` | 551 | 546 | 4 |
| `stdbool.h` | 472 | 468 | 4 |
| `errno.h` | 350 | 343 | 4 |
| `sys/types.h` | 317 | 311 | 4 |
| `signal.h` | 225 | 202 | 4 |
| `fcntl.h` | 179 | 177 | 4 |
| `sys/stat.h` | 174 | 172 | 4 |
| `time.h` | 132 | 118 | 4 |
| `config.h` | 128 | 128 | 4 |
| `inttypes.h` | 118 | 113 | 4 |
| `sys/time.h` | 94 | 83 | 4 |
| `ctype.h` | 93 | 86 | 4 |
| `sys/param.h` | 76 | 73 | 4 |
| `sys/wait.h` | 57 | 55 | 4 |
| `locale.h` | 51 | 46 | 4 |
| `dirent.h` | 44 | 44 | 4 |
| `sys/file.h` | 19 | 19 | 4 |

### direct `bits/` and `gnu/` includes

| name | occurrences | files | packages |
|---|---|---|---|
| `bits/c++config.h` | 120 | 120 | 1 |
| `bits/move.h` | 40 | 40 | 1 |
| `bits/extc++.h` | 37 | 37 | 1 |
| `bits/requires_hosted.h` | 37 | 37 | 1 |
| `bits/functexcept.h` | 30 | 30 | 1 |
| `bits/stdc++.h` | 28 | 23 | 1 |
| `bits/stl_algobase.h` | 19 | 19 | 1 |
| `bits/stl_function.h` | 17 | 17 | 1 |
| `bits/exception_defines.h` | 16 | 16 | 1 |
| `bits/stl_pair.h` | 16 | 16 | 1 |
| `bits/concept_check.h` | 15 | 15 | 1 |
| `bits/functional_hash.h` | 15 | 15 | 1 |
| `bits/stl_iterator_base_types.h` | 15 | 15 | 1 |
| `bits/c++0x_warning.h` | 14 | 14 | 1 |
| `bits/version.h` | 14 | 14 | 1 |
| `bits/cpp_type_traits.h` | 10 | 10 | 1 |
| `bits/stl_iterator.h` | 10 | 10 | 1 |
| `bits/allocator.h` | 9 | 9 | 1 |
| `bits/c++locale_internal.h` | 9 | 9 | 1 |
| `bits/stl_algo.h` | 8 | 8 | 1 |
| `bits/stl_iterator_base_funcs.h` | 8 | 8 | 1 |
| `bits/chrono.h` | 7 | 7 | 1 |
| `bits/cxxabi_forced.h` | 7 | 7 | 1 |
| `bits/ptr_traits.h` | 7 | 7 | 1 |
| `bits/gthr.h` | 6 | 6 | 1 |
| `bits/invoke.h` | 6 | 6 | 1 |
| `bits/stdtr1c++.h` | 6 | 6 | 1 |
| `bits/atomic_lockfree_defines.h` | 5 | 5 | 1 |
| `bits/atomic_word.h` | 5 | 5 | 1 |
| `bits/largefile-config.h` | 5 | 5 | 1 |
| `bits/stl_construct.h` | 5 | 5 | 1 |
| `bits/unique_ptr.h` | 5 | 5 | 1 |
| `bits/utility.h` | 5 | 5 | 1 |
| `bits/iterator_concepts.h` | 4 | 4 | 1 |
| `bits/localefwd.h` | 4 | 4 | 1 |
| `bits/memoryfwd.h` | 4 | 4 | 1 |
| `bits/ranges_base.h` | 4 | 4 | 1 |
| `bits/shared_ptr.h` | 4 | 4 | 1 |
| `bits/alloc_traits.h` | 3 | 3 | 1 |
| `bits/exception_ptr.h` | 3 | 3 | 1 |
| `bits/locale_classes.h` | 3 | 3 | 1 |
| `bits/predefined_ops.h` | 3 | 3 | 1 |
| `bits/refwrap.h` | 3 | 3 | 1 |
| `bits/std_mutex.h` | 3 | 3 | 1 |
| `bits/stl_uninitialized.h` | 3 | 3 | 1 |
| `bits/streambuf_iterator.h` | 3 | 3 | 1 |
| `bits/uniform_int_dist.h` | 3 | 3 | 1 |
| `bits/uses_allocator.h` | 3 | 3 | 1 |
| `bits/allocated_ptr.h` | 2 | 2 | 1 |
| `bits/atomic_base.h` | 2 | 2 | 1 |
| `bits/atomic_wait.h` | 2 | 2 | 1 |
| `bits/boost_concept_check.h` | 2 | 2 | 1 |
| `bits/c++io.h` | 2 | 2 | 1 |
| `bits/char_traits.h` | 2 | 2 | 1 |
| `bits/codecvt.h` | 2 | 2 | 1 |
| `bits/compatibility.h` | 2 | 1 | 1 |
| `bits/cxxabi_init_exception.h` | 2 | 2 | 1 |
| `bits/hash_bytes.h` | 2 | 2 | 1 |
| `bits/hashtable.h` | 2 | 2 | 1 |
| `bits/hashtable_policy.h` | 2 | 2 | 1 |

### identity macros

| name | occurrences | files | packages |
|---|---|---|---|
| `__GLIBC__` | 178 | 60 | 4 |
| `__GNU_LIBRARY__` | 47 | 25 | 3 |
| `__GLIBC_MINOR__` | 61 | 25 | 2 |
| `__GLIBC_PREREQ` | 35 | 16 | 2 |

### `__USE_*`

| name | occurrences | files | packages |
|---|---|---|---|
| `__USE_LARGEFILE64` | 16 | 7 | 2 |
| `__USE_FILE_OFFSET64` | 14 | 8 | 2 |
| `__USE_INIT_FINI__` | 12 | 12 | 1 |
| `__USE_GNU` | 7 | 5 | 1 |
| `__USE_EXTERN_INLINES` | 6 | 2 | 1 |
| `__USE_MINGW_ANSI_STDIO` | 4 | 2 | 1 |
| `__USE_FIXED_PROTOTYPES__` | 2 | 1 | 1 |
| `__USE_MISC` | 2 | 2 | 1 |
| `__USE_BSD` | 1 | 1 | 1 |
| `__USE_STRING_INLINES` | 1 | 1 | 1 |
| `__USE_XOPEN2K8` | 1 | 1 | 1 |

### `sys/cdefs.h` vocabulary

| name | occurrences | files | packages |
|---|---|---|---|
| `__P` | 1124 | 50 | 3 |
| `__attribute_pure__` | 10 | 4 | 2 |
| `__nonnull` | 5 | 3 | 2 |
| `__glibc_likely` | 29 | 11 | 1 |
| `__glibc_unlikely` | 16 | 9 | 1 |
| `__REDIRECT` | 6 | 2 | 1 |
| `__extern_inline` | 5 | 2 | 1 |
| `__BEGIN_DECLS` | 4 | 2 | 1 |
| `__END_DECLS` | 4 | 2 | 1 |
| `__attribute__vcond_v16hfv16hfge_a` | 4 | 1 | 1 |
| `__ASMNAME2` | 3 | 1 | 1 |
| `__ASMNAME` | 2 | 1 | 1 |
| `__CONCAT` | 2 | 1 | 1 |
| `__NTH` | 2 | 2 | 1 |
| `__STRING` | 2 | 1 | 1 |
| `__THROW` | 2 | 2 | 1 |
| `__attribute____atomic_add_dispatch` | 1 | 1 | 1 |
| `__attribute____exchange_and_add_dispatch0` | 1 | 1 | 1 |
| `__attribute__foo_v4df` | 1 | 1 | 1 |
| `__attribute__mask_slp_int64_t_8_2` | 1 | 1 | 1 |
| `__attribute__s` | 1 | 1 | 1 |
| `__attribute__vcond_v16hfv16hfge` | 1 | 1 | 1 |
| `__attribute__vcond_v16hfv16hfmax` | 1 | 1 | 1 |
| `__attribute_blah__` | 1 | 1 | 1 |
| `__attribute_const__` | 1 | 1 | 1 |
| `__attribute_deprecated_with_message__` | 1 | 1 | 1 |
| `__attribute_unavailable_with_message__` | 1 | 1 | 1 |

### reserved `__*_t` names (top 25)

| name | occurrences | files | packages |
|---|---|---|---|
| `__kernel_gid_t` | 3 | 3 | 2 |
| `__kernel_uid_t` | 3 | 3 | 2 |
| `__wchar_t` | 3 | 2 | 2 |
| `__int128_t` | 378 | 89 | 1 |
| `__uint128_t` | 226 | 67 | 1 |
| `__vector_type_t` | 163 | 8 | 1 |
| `__gthread_mutex_t` | 148 | 35 | 1 |
| `__sanitizer_sigset_t` | 101 | 18 | 1 |
| `__int_for_sizeof_t` | 89 | 10 | 1 |
| `__gthread_key_t` | 74 | 21 | 1 |
| `__gthread_recursive_mutex_t` | 73 | 13 | 1 |
| `__gthread_t` | 69 | 12 | 1 |
| `__uint32_t` | 65 | 21 | 1 |
| `__gthread_cond_t` | 64 | 12 | 1 |
| `__void_t` | 54 | 21 | 1 |
| `__enable_if_t` | 53 | 23 | 1 |
| `__remove_cvref_t` | 49 | 14 | 1 |
| `__conditional_t` | 48 | 23 | 1 |
| `__type_identity_t` | 48 | 17 | 1 |
| `__gthread_once_t` | 47 | 26 | 1 |
| `__platform_wait_t` | 45 | 3 | 1 |
| `__fixed_size_storage_t` | 40 | 6 | 1 |
| `__iter_key_t` | 40 | 6 | 1 |
| `__intrinsic_type_t` | 37 | 3 | 1 |
| `__sve_vector_type_t` | 37 | 1 | 1 |

### FILE members

| name | occurrences | files | packages |
|---|---|---|---|
| `_IO_read_base` | 6 | 3 | 1 |
| `_IO_write_base` | 6 | 3 | 1 |
| `_IO_read_end` | 4 | 3 | 1 |
| `_IO_write_end` | 4 | 3 | 1 |
| `_flags` | 3 | 1 | 1 |
| `_IO_backup_base` | 2 | 2 | 1 |
| `_IO_buf_base` | 2 | 2 | 1 |
| `_IO_buf_end` | 2 | 2 | 1 |
| `_IO_read_ptr` | 2 | 2 | 1 |
| `_IO_save_base` | 2 | 2 | 1 |
| `_IO_save_end` | 2 | 2 | 1 |
| `_IO_write_ptr` | 2 | 2 | 1 |
| `_fileno` | 2 | 1 | 1 |

## Set `smoke`: top counts

### `#include <...>` (by packages, top 25)

| name | occurrences | files | packages |
|---|---|---|---|
| `stdio.h` | 587 | 571 | 15 |
| `string.h` | 414 | 406 | 15 |
| `stdlib.h` | 367 | 354 | 15 |
| `unistd.h` | 298 | 288 | 15 |
| `sys/types.h` | 286 | 276 | 14 |
| `errno.h` | 204 | 202 | 14 |
| `stddef.h` | 144 | 137 | 14 |
| `ctype.h` | 128 | 127 | 14 |
| `limits.h` | 90 | 88 | 14 |
| `stdarg.h` | 78 | 78 | 14 |
| `sys/stat.h` | 93 | 90 | 13 |
| `stdint.h` | 73 | 73 | 13 |
| `time.h` | 68 | 65 | 13 |
| `assert.h` | 232 | 229 | 12 |
| `fcntl.h` | 128 | 121 | 12 |
| `signal.h` | 89 | 88 | 11 |
| `inttypes.h` | 38 | 38 | 11 |
| `locale.h` | 36 | 35 | 11 |
| `sys/time.h` | 46 | 43 | 10 |
| `wchar.h` | 33 | 32 | 10 |
| `sys/wait.h` | 22 | 22 | 10 |
| `config.h` | 286 | 283 | 9 |
| `windows.h` | 119 | 116 | 9 |
| `io.h` | 57 | 57 | 9 |
| `dirent.h` | 19 | 19 | 9 |

### direct `bits/` and `gnu/` includes

| name | occurrences | files | packages |
|---|---|---|---|
| `bits/alltypes.h` | 1 | 1 | 1 |
| `bits/long-double.h` | 1 | 1 | 1 |
| `bits/wordsize.h` | 1 | 1 | 1 |

### identity macros

| name | occurrences | files | packages |
|---|---|---|---|
| `__GLIBC__` | 47 | 27 | 6 |
| `__GNU_LIBRARY__` | 28 | 10 | 2 |
| `__GLIBC_MINOR__` | 4 | 4 | 2 |

### `__USE_*`

| name | occurrences | files | packages |
|---|---|---|---|
| `__USE_MINGW_ANSI_STDIO` | 11 | 5 | 4 |
| `__USE_GNU` | 13 | 4 | 3 |
| `__USE_INLINE__` | 3 | 3 | 2 |
| `__USE_FILE_OFFSET64` | 6 | 3 | 1 |
| `__USE_LARGEFILE64` | 6 | 3 | 1 |
| `__USE_FORTIFY_LEVEL` | 5 | 1 | 1 |
| `__USE_RUNCOMMAND__` | 5 | 2 | 1 |
| `__USE_ISOC11` | 4 | 1 | 1 |
| `__USE_MISC` | 1 | 1 | 1 |
| `__USE_POSIX2` | 1 | 1 | 1 |
| `__USE_POSIX_IMPLICITLY` | 1 | 1 | 1 |
| `__USE_SYSTEM_ENDIAN_H__` | 1 | 1 | 1 |
| `__USE_XOPEN2K` | 1 | 1 | 1 |

### `sys/cdefs.h` vocabulary

| name | occurrences | files | packages |
|---|---|---|---|
| `__attribute_warn_unused_result__` | 48 | 8 | 4 |
| `__glibc_unlikely` | 352 | 8 | 3 |
| `__P` | 50 | 7 | 3 |
| `__glibc_likely` | 35 | 8 | 3 |
| `__attribute_nonnull__` | 23 | 5 | 3 |
| `__attribute_maybe_unused__` | 12 | 5 | 3 |
| `__THROW` | 27 | 5 | 2 |
| `__attribute__visibility__` | 671 | 3 | 1 |
| `__attribute__warn_unused_result__` | 486 | 3 | 1 |
| `__attribute__format__` | 41 | 3 | 1 |
| `__attribute__noreturn__` | 40 | 5 | 1 |
| `__attribute__pure__` | 23 | 2 | 1 |
| `__attribute__deprecated__` | 20 | 3 | 1 |
| `__glibc_has_attribute` | 18 | 2 | 1 |
| `__attribute__malloc__` | 15 | 2 | 1 |
| `__ASMNAME` | 13 | 2 | 1 |
| `__REDIRECT` | 12 | 3 | 1 |
| `__attribute__always_inline__` | 8 | 3 | 1 |
| `__attribute__unused__` | 8 | 2 | 1 |
| `__glibc_clang_prereq` | 8 | 2 | 1 |
| `__LDBL_REDIR1_NTH` | 7 | 2 | 1 |
| `__REDIRECT_NTH` | 7 | 3 | 1 |
| `__THROWNL` | 7 | 2 | 1 |
| `__attribute__format__null_ok__` | 7 | 2 | 1 |
| `__LDBL_REDIR1` | 6 | 2 | 1 |
| `__extern_always_inline` | 6 | 2 | 1 |
| `__glibc_safe_len_cond` | 6 | 2 | 1 |
| `__NTH` | 5 | 2 | 1 |
| `__glibc_has_extension` | 5 | 2 | 1 |
| `__nonnull` | 5 | 2 | 1 |
| `__LDBL_REDIR` | 4 | 2 | 1 |
| `__LDBL_REDIR2_DECL` | 4 | 2 | 1 |
| `__LDBL_REDIR_DECL` | 4 | 2 | 1 |
| `__LDBL_REDIR_NTH` | 4 | 2 | 1 |
| `__NTHNL` | 4 | 2 | 1 |
| `__REDIRECT_LDBL` | 4 | 2 | 1 |
| `__REDIRECT_NTH_LDBL` | 4 | 2 | 1 |
| `__attribute__nonnull__` | 4 | 2 | 1 |
| `__attribute_artificial__` | 4 | 2 | 1 |
| `__attribute_deprecated__` | 4 | 2 | 1 |
| `__attribute_noinline__` | 4 | 3 | 1 |
| `__glibc_has_builtin` | 4 | 2 | 1 |
| `__glibc_macro_warning` | 4 | 2 | 1 |
| `__glibc_safe_or_unknown_len` | 4 | 2 | 1 |
| `__glibc_unsafe_len` | 4 | 2 | 1 |
| `__glibc_unsigned_or_positive` | 4 | 2 | 1 |
| `__wur` | 4 | 2 | 1 |
| `__ASMNAME2` | 3 | 2 | 1 |
| `__BEGIN_DECLS` | 3 | 2 | 1 |
| `__END_DECLS` | 3 | 2 | 1 |
| `__LDBL_REDIR1_DECL` | 3 | 2 | 1 |
| `__REDIRECT_NTHNL` | 3 | 2 | 1 |
| `__STRING` | 3 | 2 | 1 |
| `__attribute_alloc_align__` | 3 | 2 | 1 |
| `__attribute_alloc_size__` | 3 | 2 | 1 |
| `__attribute_const__` | 3 | 2 | 1 |
| `__attribute_copy__` | 3 | 1 | 1 |
| `__attribute_deprecated_msg__` | 3 | 2 | 1 |
| `__attribute_format_arg__` | 3 | 2 | 1 |
| `__attribute_format_strfmon__` | 3 | 2 | 1 |

### reserved `__*_t` names (top 25)

| name | occurrences | files | packages |
|---|---|---|---|
| `__time64_t` | 22 | 2 | 2 |
| `__pid_t` | 4 | 2 | 2 |
| `__locale_t` | 3 | 2 | 2 |
| `__uint128_t` | 13 | 4 | 1 |
| `__re_size_t` | 10 | 5 | 1 |
| `__re_long_size_t` | 4 | 1 | 1 |
| `__ptr_t` | 3 | 2 | 1 |
| `__off_t` | 2 | 1 | 1 |
| `__priority_which_t` | 2 | 1 | 1 |
| `__wchar_t` | 2 | 1 | 1 |
| `__wint_t` | 2 | 1 | 1 |
| `__fsword_t` | 1 | 1 | 1 |
| `__ino64_t` | 1 | 1 | 1 |
| `__time32_t` | 1 | 1 | 1 |

### FILE members

| name | occurrences | files | packages |
|---|---|---|---|
| `_flags` | 3 | 3 | 2 |
| `_flags2` | 30 | 1 | 1 |
| `_fileno` | 1 | 1 | 1 |

## Skip log (directories never read)

184 skipped directories across the sets.

* `bash bash_5.2.37.orig.tar.xz:bash-5.2.37/lib`
* `bash bash_5.2.37.orig.tar.xz>bash-completion-2.5.tar.xz:bash-completion-2.5/test/fixtures/slackware/home/abc-4-i686-1.txz (nested archive unreadable: empty file)`
* `bash bash_5.2.37.orig.tar.xz>bash-completion-2.5.tar.xz:bash-completion-2.5/test/fixtures/slackware/home/tcl.d/tcl.tgz (nested archive unreadable: empty file)`
* `bash bash_5.2.37.orig.tar.xz>bash-completion-2.5.tar.xz:bash-completion-2.5/test/fixtures/tar/archive.tar.xz (nested archive unreadable: empty file)`
* `bash bash_5.2.37.orig.tar.xz>bash-completion-2.5.tar.xz:bash-completion-2.5/test/lib`
* `binutils binutils_2.44.orig.tar.xz:binutils-2.44/binutils/testsuite/lib`
* `binutils binutils_2.44.orig.tar.xz:binutils-2.44/gas/testsuite/lib`
* `binutils binutils_2.44.orig.tar.xz:binutils-2.44/gprofng/testsuite/lib`
* `binutils binutils_2.44.orig.tar.xz:binutils-2.44/ld/testsuite/ld-aarch64/lib`
* `binutils binutils_2.44.orig.tar.xz:binutils-2.44/ld/testsuite/lib`
* `binutils binutils_2.44.orig.tar.xz:binutils-2.44/libctf/testsuite/lib`
* `coreutils coreutils_9.7.orig.tar.gz:coreutils-9.7/gl/lib`
* `coreutils coreutils_9.7.orig.tar.gz:coreutils-9.7/gnulib-tests`
* `coreutils coreutils_9.7.orig.tar.gz:coreutils-9.7/lib`
* `cppunit cppunit_1.15.1.orig.tar.gz:cppunit-1.15.1/lib`
* `diffutils diffutils_3.10.orig.tar.xz:diffutils-3.10/gnulib-tests`
* `diffutils diffutils_3.10.orig.tar.xz:diffutils-3.10/lib`
* `dpkg dpkg_1.22.22.tar.xz:dpkg-1.22.22/lib`
* `dpkg dpkg_1.22.22.tar.xz:dpkg-1.22.22/tests/t-filtering/pkg-somefiles/test/lib`
* `findutils findutils_4.10.0.orig.tar.xz:findutils-4.10.0/gl/lib`
* `findutils findutils_4.10.0.orig.tar.xz:findutils-4.10.0/gnulib-tests`
* `findutils findutils_4.10.0.orig.tar.xz:findutils-4.10.0/lib`
* `gawk gawk_5.2.1.orig.tar.xz:gawk-5.2.1/awklib/eg/lib`
* `gawk gawk_5.2.1.orig.tar.xz:gawk-5.2.1/test/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz:gcc-14-14.2.0/newlib-4.4.0.20231231.tar.xz (nested C library archive, not opened)`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/gcc/testsuite/gcc.c-torture/execute/builtins/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/gcc/testsuite/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/libatomic/testsuite/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/libffi/testsuite/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/libgo/testsuite/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/libgomp/testsuite/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/libitm/testsuite/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/libphobos/testsuite/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/libstdc++-v3/testsuite/lib`
* `gcc-14 gcc-14_14.2.0.orig.tar.gz>gcc-14.2.0-dfsg.tar.xz:gcc-14.2.0/libvtv/testsuite/lib`
* `grep grep_3.11.orig.tar.xz:grep-3.11/gnulib-tests`
* `grep grep_3.11.orig.tar.xz:grep-3.11/lib`
* `gzip gzip_1.13.orig.tar.xz:gzip-1.13/lib`
* `make-dfsg make-dfsg_4.4.1.orig.tar.xz:make-4.4.1/lib`
* `perl perl_5.40.1.orig-regen-configure.tar.xz:metaconfig-5.40.1/dist/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Archive-Tar/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/AutoLoader/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/CPAN-Meta-Requirements/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/CPAN-Meta-YAML/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/CPAN-Meta-YAML/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/CPAN-Meta/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/CPAN-Meta/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/CPAN/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Compress-Raw-Bzip2/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Compress-Raw-Zlib/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Digest-SHA/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Digest/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Digest/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Encode/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/ExtUtils-Constant/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/ExtUtils-Install/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/ExtUtils-Install/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/ExtUtils-MakeMaker/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/ExtUtils-MakeMaker/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/ExtUtils-Manifest/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/ExtUtils-PL2Bat/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/File-Fetch/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/File-Path/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/File-Temp/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Getopt-Long/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/HTTP-Tiny/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/IO-Compress/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/IO-Socket-IP/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/IPC-Cmd/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/IPC-SysV/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/JSON-PP/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Locale-Maketext-Simple/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/MIME-Base64/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Math-BigInt-FastCalc/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Math-BigInt/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Memoize/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Module-Load-Conditional/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Module-Load/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Module-Loaded/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Module-Metadata/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Module-Metadata/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/NEXT/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Params-Check/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Perl-OSType/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/PerlIO-via-QuotedPrint/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Pod-Checker/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Pod-Escapes/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Pod-Perldoc/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Pod-Simple/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Pod-Simple/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Pod-Usage/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Scalar-List-Utils/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Term-ANSIColor/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Term-ANSIColor/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Term-Table/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Test-Harness/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Test-Harness/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Test-Simple/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Test-Simple/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Test2-Suite/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Test2-Suite/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Text-Balanced/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Text-ParseWords/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Text-Tabs/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Text-Tabs/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Tie-RefHash/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Time-Local/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/Time-Piece/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/autodie/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/autodie/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/bignum/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/experimental/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/libnet/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/parent/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/parent/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/perlfaq/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/podlators/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/podlators/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/cpan/version/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Attribute-Handlers/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Carp/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Data-Dumper/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Devel-SelfStubber/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Dumpvalue/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Dumpvalue/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Env/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Exporter/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/ExtUtils-CBuilder/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/ExtUtils-ParseXS/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/ExtUtils-ParseXS/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Filter-Simple/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Filter-Simple/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/FindBin/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/I18N-Collate/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/I18N-LangTags/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/IO/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Locale-Maketext/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Math-Complex/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Module-CoreList/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Net-Ping/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/PathTools/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Search-Dict/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/SelfLoader/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Term-Complete/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Term-ReadLine/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Test/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Text-Abbrev/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Thread-Queue/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Thread-Semaphore/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/Tie-File/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/autouse/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/autouse/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/base/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/base/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/constant/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/encoding-warnings/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/threads-shared/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/dist/threads/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/ExtUtils-Miniperl/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/File-DosGlob/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/File-Find/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/File-Find/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/FileCache/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/Hash-Util-FieldHash/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/Hash-Util/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/IPC-Open3/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/POSIX/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/Pod-Html/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/Pod-Html/t/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/Tie-Memoize/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/ext/VMS-Filespec/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/lib`
* `perl perl_5.40.1.orig.tar.xz:perl-5.40.1/t/lib`
* `python3.13 python3.13_3.13.5.orig.tar.xz:Python-3.13.5/Tools/msi/lib`
* `sed sed_4.9.orig.tar.xz:sed-4.9/gnulib-tests`
* `sed sed_4.9.orig.tar.xz:sed-4.9/lib`
* `sysvinit sysvinit_3.14-4devuan1.debian.tar.xz:debian/src/initscripts/lib`
* `sysvinit sysvinit_3.14-4devuan1.debian.tar.xz:debian/src/initscripts/usr/lib`
* `tar tar_1.35+dfsg.orig.tar.xz:tar-1.35+dfsg/lib`
* `util-linux util-linux_2.41.5.orig.tar.xz:util-linux-2.41.5/lib`
* `util-linux util-linux_2.41.5.orig.tar.xz:util-linux-2.41.5/tests/expected/lib`
* `util-linux util-linux_2.41.5.orig.tar.xz:util-linux-2.41.5/tests/ts/lib`
* `xz-utils xz-utils_5.8.1.orig.tar.xz:xz-5.8.1/lib`

Code-extension members over 48 MiB not scanned: 0.

## Limits

* FILE members other than `_IO_*` are counted only as `->name` or `.name` in files that also mention `FILE`, `stdin`, `stdout` or `stderr`; generic names (`_flags`, `_mode`, `_offset`, `_lock`) therefore over-count.
* `__*_t` includes names a package defines for itself and compiler names; read the package count with that in mind.
* Identifier counts include mentions in comments and `#ifdef` lines (a test for a name is a use of the name).
* Archives nested inside archives are not opened.
