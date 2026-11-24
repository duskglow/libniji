# libniji: release notes

libniji was called Liella until November 2026.

Each section's known limitations are those of its own snapshot; later sections say what was fixed. The newest
snapshot's list is in `snapshot-2026-11-24.1` below.

Changes on `next` since the newest snapshot are one file each in [release-notes.d/](release-notes.d/README.md);
`cargo xtask release-notes` prints them assembled. This file holds released sections only.

## libniji: twenty-eighth pre-release, 2026-11-24 (`snapshot-2026-11-24.1`)

`main` at commit `88c0518f`, tagged `snapshot-2026-11-24.1`. Changes since
[`snapshot-2026-11-23.1`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-23.1).

- **[#12](https://github.com/duskglow/libniji/issues/12), `gencat`:** `libc-dev-bin` now ships `/usr/bin/gencat`. The
  catalogs it writes are byte-identical to stock `gencat`'s for the same input, its exit status matches stock's for
  malformed and missing input, and `catopen`/`catgets` read catalogs back as stock does, by path and through
  `NLSPATH`.
- **[#21](https://github.com/duskglow/libniji/issues/21), address space of a long directory walk:** the heap's small
  size classes are finer, so the issue's `find` over 200,000 entries needs 23,584 KiB more than over 3 files, down from
  32,320 in 23.1 (stock: 26,976). The address space at start-up is not yet reduced in this snapshot; that change
  landed after this commit.
- **Shadow stack (Intel CET):** libniji's `libc.so.6`, `libm.so.6`, loader and start files now carry the shadow-stack
  property, so programs linked against them are marked. Turning the shadow stack on is opt-in through
  `GLIBC_TUNABLES`. Neither CPU class we test on has the feature, so these checks cover only the tunables' behaviour
  without it, which equals stock's in every row.
- **[#14](https://github.com/duskglow/libniji/issues/14), locales:** a locale compiled from the shipped `en_US` source
  now formats dates and times as stock's does; compiled-locale read-back differs from stock in 0 of 37 checked lines
  (23.1: 8).
- **Start-up** costs about 2,100 fewer user instructions per dynamic start than 23.1 (about 1%); page faults are
  unchanged.

### Packages

The same two flavours as before, built by `tools/deb/build.sh` for Debian 13 (trixie) and Devuan 6 (excalibur), amd64,
attached with `SHA256SUMS`. GitHub does not allow `~` in asset names, so the `niji6` files are published with `.` in its
place. `SHA256SUMS` lists the names as published, and the package versions inside the files are unchanged.

- **Beside glibc** (`niji6`, `niji6-dev`), version `0.0~snapshot20261124.1+88c0518f6ba4`. It sorts above 23.1's.
- **As the system C library** (`libc6`, `libc-bin`, `libc6-dev`, `libc-dev-bin`, `locales`, `libc-l10n`,
  `libc6-dbg`), version `2.39+niji20261124.1-1`, **experimental**. `libc-dev-bin` adds `gencat`.

Checks on this commit:

| Check | Result |
|---|---|
| Every check of 23.1 | equal or better |
| [#12](https://github.com/duskglow/libniji/issues/12): stock `gencat` vs libniji's, three inputs | byte-identical; read-back equal to stock |
| [#21](https://github.com/duskglow/libniji/issues/21): the issue's `find` formula | passes 5 of 5; 23,584 KiB (23.1: 32,320; stock 26,976) |
| Shadow-stack tunables, dynamic and static, on CPUs without the feature | equal to stock |
| [#14](https://github.com/duskglow/libniji/issues/14): bash's command, and compiled-locale read-back | compiles; 0 of 37 lines differ from stock |

**The system flavour still has no C headers.** Its `libc6-dev` replaces Debian's, so after a system install no C
program that includes a C library header compiles. Use the flavour beside glibc (which keeps the system's headers) to
compile. The generated headers are in their last step and will come in a following snapshot.

### Start-up

Per dynamic program, ASLR off, median of 101 runs, user instructions / page faults:

| Program | CPU class | glibc 2.39 | 23.1 | this snapshot |
|---|---|---|---|---|
| empty `main` | Skylake-SP | 107,752 / 49 | 156,623 / 52 | 154,535 / 52 |
| three libraries | Skylake-SP | 143,094 / 65 | 231,770 / 67 | 229,676 / 67 |
| `clock_gettime` | Skylake-SP | 109,862 / 50 | 167,599 / 55 | 165,510 / 55 |
| empty `main` | Zen 1 | 101,533 / 48 | 156,382 / 52 | 154,296 / 52 |

### Known limitations

- **[#21](https://github.com/duskglow/libniji/issues/21), the rest:** the address space at start-up is still larger
  than stock's (8,640 KiB against 3,988 in the issue's probe); the reduction landed after this commit.
- **[#6](https://github.com/duskglow/libniji/issues/6), the rest:** some character-set names convert differently from
  stock, and some names stock lists have no libniji converter.
- **`$ORIGIN` in set-user-ID and set-group-ID programs:** expanded only for libraries in the default system library
  directories. This is deliberate.
- **Thread debugging of dynamically linked programs** waits for the loader's debugger interface (`_r_debug`).
- The system flavour ships no C headers, as above.

## libniji: twenty-seventh pre-release, 2026-11-23 (`snapshot-2026-11-23.1`)

`main` at commit `80d1f809`, tagged `snapshot-2026-11-23.1`. Changes since
[`snapshot-2026-11-22.3`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-22.3).

- **[#25](https://github.com/duskglow/libniji/issues/25), C++ thread exit:** C++ programs built with `niji-g++` now
  unwind a thread's stack on `pthread_exit` and on acting on a cancellation request, as with g++ and stock glibc:
  destructors run, and `catch (abi::__forced_unwind&)` with a rethrow works. A program linked `-static-libgcc` with no
  `libgcc_s.so.1` loaded no longer hangs at such an exit: it stops with a message naming the cause and the fix, with
  stock's exit status.
- **[#24](https://github.com/duskglow/libniji/issues/24):** every interface that reports where a loaded object came
  from (`dladdr`, `dl_iterate_phdr`, the link map) gives the same path as stock.
- **[#11](https://github.com/duskglow/libniji/issues/11):** the C, POSIX and C.UTF-8 locales answer `LC_IDENTIFICATION`
  queries as stock does; the issue's reproducer prints `[ISO]`. The contact fields stay empty by design.
- **[#14](https://github.com/duskglow/libniji/issues/14), locales:** `locales` now ships 287 locale sources, the
  `SUPPORTED` list and charmaps, generated by libniji rather than copied from glibc, so bash's own `localedef` command
  works on a system with only libniji's packages. `localedef` now accepts the POSIX symbolic character names, searches
  the current directory and `I18NPATH`, fills the categories a source leaves out with defaults, writes the locale
  archive, and transliterates strings the target charmap lacks.
- **Dynamic start-up costs about 40% less.** User instructions per dynamic start fall by 39% to 40% against 22.3 (an
  empty `main`: 260,857 -> 156,623; glibc 107,752), from handling each loaded object once, a faster relative-relocation
  loop and validating each symbol once. Every start-up row takes one more page fault than 22.3; that is being traced.
- **`ld.so` run as a program** now runs programs that need shared objects, and takes `--library-path`.
- **Thread debugging of static programs:** `libthread_db.so.1` lets debuggers list the threads of a statically linked
  libniji program, show each thread's backtrace and read its thread-local variables, live or from a core file (tested
  with gdb 15.1).
- **Under valgrind,** nothing is still reachable at exit ("All heap blocks were freed"), and stdio streams allocate
  through the program's `malloc`.
- **[#21](https://github.com/duskglow/libniji/issues/21):** the issue's probe and `find` formula give the same numbers
  as 22.3.

### Packages

The same two flavours as before, built by `tools/deb/build.sh` for Debian 13 (trixie) and Devuan 6 (excalibur), amd64,
attached with `SHA256SUMS`. GitHub does not allow `~` in asset names, so the `niji6` files are published with `.` in its
place. `SHA256SUMS` lists the names as published, and the package versions inside the files are unchanged.

- **Beside glibc** (`niji6`, `niji6-dev`), version `0.0~snapshot20261123.1+80d1f809b2d1`. It sorts above 22.3's.
  `niji6` adds `libthread_db.so.1`; `niji6-dev` adds the `libthread_db.so` link and `niji-unwinder.o`.
- **As the system C library** (`libc6`, `libc-bin`, `libc6-dev`, `libc-dev-bin`, `locales`, `libc-l10n`,
  `libc6-dbg`), version `2.39+niji20261123.1-1`, **experimental**.

Checks on this commit:

| Check | Result |
|---|---|
| Every check of 22.3 | equal or better |
| [#25](https://github.com/duskglow/libniji/issues/25): the issue's three cases built with `niji-g++` | unwind; output equal to g++ with stock |
| [#24](https://github.com/duskglow/libniji/issues/24), [#11](https://github.com/duskglow/libniji/issues/11), `ld.so --library-path` | equal to stock |
| [#14](https://github.com/duskglow/libniji/issues/14): bash's exact command on a system with only libniji's packages | compiles, as on stock |
| Dynamic start-up | **-39% to -40% user instructions**, one more page fault per start |

**The system flavour has no C headers yet.** Its `libc6-dev` replaces Debian's, so after a system install no C program
that includes a C library header compiles. Use the flavour beside glibc (which keeps the system's headers) to compile.

### Start-up

Per dynamic program, ASLR off, median of 101 runs, user instructions / page faults:

| Program | CPU class | glibc 2.39 | 22.3 | this snapshot |
|---|---|---|---|---|
| empty `main` | Skylake-SP | 107,752 / 49 | 260,857 / 51 | 156,623 / 52 |
| three libraries | Skylake-SP | 143,094 / 65 | 371,146 / 66 | 231,770 / 67 |
| `clock_gettime` | Skylake-SP | 109,862 / 50 | 274,330 / 54 | 167,599 / 55 |
| empty `main` | Zen 1 | 101,533 / 48 | 260,618 / 51 | 156,382 / 52 |

### Known limitations

- **Date and time formats of the shipped locale sources:** a locale compiled from the shipped `en_US` source formats
  `%c` in the form `Sun, Nov 22, 2026, 02:05:09 PM` where stock's `en_US.UTF-8` uses `Sun 22 Nov 2026 02:05:09 PM UTC`. The
  shipped sources' time, number and money formats are being compared with stock's.
- **[#6](https://github.com/duskglow/libniji/issues/6), the rest:** 35 character-set names convert differently from
  stock, and 222 names stock lists have no libniji converter.
- **`$ORIGIN` in set-user-ID and set-group-ID programs:** expanded only for libraries in the default system library
  directories. This is deliberate.
- **[#21](https://github.com/duskglow/libniji/issues/21), the rest:** the address space at start-up is larger than
  stock's.
- **Thread debugging of dynamically linked programs** waits for the loader's debugger interface (`_r_debug`).
- The system flavour ships no C headers, as above.

## libniji: twenty-sixth pre-release, 2026-11-22 (`snapshot-2026-11-22.3`)

`main` at commit `2a76723f`, tagged `snapshot-2026-11-22.3`. Changes since
[`snapshot-2026-11-22.2`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-22.2).

- **`localedef` ([#14](https://github.com/duskglow/libniji/issues/14)):** `libc-bin` ships `localedef`. It compiles a
  locale from a source and a charmap into the directory form, and the result reads back as stock glibc's locale of the
  same definition. The `locales` package ships no locale sources or charmaps yet, so they must be supplied.
- **Dynamic start-up costs another fifth less:** 18% to 23% fewer user instructions per dynamic start than 22.2 (an
  empty `main`: 320,731 -> 260,857; glibc 107,752), from reading each object's program headers once.
- **[#21](https://github.com/duskglow/libniji/issues/21), heap:** a thread that moves to another NUMA node costs at
  most 4,352 KiB more address space; under an address-space limit the worst case falls from 45,648 to 43,600 KiB.

Packages: beside glibc `0.0~snapshot20261122.3+2a76723f2ed4`. Known limitations: no locale sources or charmaps in
`locales`; `localedef` writes only the categories a source defines, takes only `<Uxxxx>` names, does not search the
current directory or `I18NPATH`, and has no locale archive; under valgrind stdout's buffer is still reachable at exit;
allocations the library keeps to itself do not go through a replaced `malloc`; [#6](https://github.com/duskglow/libniji/issues/6):
35 character-set names convert differently from stock and 222 have no converter; `$ORIGIN` in set-user-ID and
set-group-ID programs is expanded only for the default system library directories (deliberate); `ld.so` run as a
program runs only position-independent programs that need no shared objects; #21: the start-up address space is larger
than stock's; `-static-libgcc` without `libgcc_s`: a C++ thread exit hangs; the system flavour ships no C headers.

## libniji: twenty-fifth pre-release, 2026-11-22 (`snapshot-2026-11-22.2`)

`main` at commit `302dd7c3`, tagged `snapshot-2026-11-22.2`. Changes since
[`snapshot-2026-11-22.1`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-22.1).

- **The start-up page-fault regression is fixed:** 21.6 to 22.1 took 5 or 6 more page faults per dynamic program than
  21.5; this snapshot takes 5 fewer than 22.1 (a program with three libraries is still 1 above 21.5), because the
  character-set tables hold offsets instead of addresses. User instructions per start fall by another ~3,340.
- **valgrind leak records reach your code ([#13](https://github.com/duskglow/libniji/issues/13)):** with the whole
  `libc6-dbg`, a leak record's stack runs down to `main`, as on stock glibc; helgrind gives stock's results on threaded
  programs.
- **Timed waits with a deadline in the past** (`pthread_rwlock_timedwrlock`, `pthread_mutex_timedlock`, `sem_timedwait`
  and the others) return `ETIMEDOUT` when the deadline's seconds are negative, instead of never returning.
- **`strncmp`** on AVX2 machines no longer risks reading past the end of a string when another thread changes the
  strings during the call.

## libniji: twenty-fourth pre-release, 2026-11-22 (`snapshot-2026-11-22.1`)

`main` at commit `9548c734`, tagged `snapshot-2026-11-22.1`. Changes since
[`snapshot-2026-11-21.7`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-21.7).

- **Dynamic start-up costs about half what it did:** about 55% fewer user instructions than 21.7 (an empty `main`:
  324,068 instead of 726,461 on Skylake-SP), one page fault fewer. The C library binds its calls to its own functions
  at build time (symbolic relocations in `libc.so.6`: 288 down to 55). Still about three times glibc's start-up.
- **Replacing `malloc` still works:** the `malloc` family and the library's data (`stdin`, `environ`, `optarg`, ...)
  stay replaceable; a replacement gets every allocation of memory the C library hands to the caller. Allocations the
  library keeps to itself are a known limitation.
- **`dlopen` in set-user-ID programs:** the secure-mode `$ORIGIN` rule now has its own tests (behaviour unchanged).

## libniji: twenty-third pre-release, 2026-11-21 (`snapshot-2026-11-21.7`)

`main` at commit `2c64fde8`, tagged `snapshot-2026-11-21.7`. Changes since
[`snapshot-2026-11-21.6`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-21.6).

- **valgrind on everyday programs ([#13](https://github.com/duskglow/libniji/issues/13)):** `memcheck` replaces
  libniji's string functions with its own, as on stock glibc; `ls /`, `find`, `sort` and a program calling every
  replaced string function report 0 errors with the whole `libc6-dbg` installed (the last snapshot: 12, 71, 12 and
  others). A real read past the end of a string is still reported.
- **More character sets ([#6](https://github.com/duskglow/libniji/issues/6)):** `iconv` converts CP949 and UHC (built
  in) and opens `WINDOWS-1252`, `windows-1251`, `KOI8R` and `ISO885915`; `iconv -l` lists 984 names (stock 1,180).
- **`dlopen` and `$ORIGIN`:** a library that calls `dlopen` with a bare name has its own `DT_RUNPATH` searched,
  including `$ORIGIN` elements, as on stock glibc.
- **Start-up:** about 16,500 fewer user instructions per dynamic program.
- **Correction to 21.6's notes:** `getaddrinfo_a` does not lose requests under heavy load; the loss reported there was
  the test's fixed deadline running out on a loaded machine.

## libniji: twenty-second pre-release, 2026-11-21 (`snapshot-2026-11-21.6`)

`main` at commit `97bf3542`, tagged `snapshot-2026-11-21.6`. Changes since
[`snapshot-2026-11-21.5`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-21.5).

- **CJK character sets ([#6](https://github.com/duskglow/libniji/issues/6)):** `iconv` converts BIG5, EUC-KR, CP932,
  CP936 and GB18030, built into the C library, byte for byte as stock for each set's whole code space.
- **valgrind with the whole debug package ([#13](https://github.com/duskglow/libniji/issues/13)):** with `libc6-dbg`
  installed, `memcheck` runs programs and reports the same error summaries as stock for `/bin/true`, C, C++ and a
  leaking program. Leak records do not yet show the program's frames below the C library.
- **More shared objects per process:** 600 libraries load (the last snapshot stopped at 510).
- **Fewer fixed limits:** exit handlers, `addseverity` levels and `getaddrinfo_a`'s request queue are limited only by
  memory.

## libniji: twenty-first pre-release, 2026-11-21 (`snapshot-2026-11-21.5`)

`main` at commit `cd1bbabd`, tagged `snapshot-2026-11-21.5`. Changes since
[`snapshot-2026-11-21.4`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-21.4).

- **`zic` and `zdump` ([#14](https://github.com/duskglow/libniji/issues/14), partly):** `libc-bin` ships both; against
  glibc 2.39's `zic` the compiled files decode the same for 591 of 598 zones, the other seven give the same readings,
  and `zdump -v` over the installed zones is identical. `zic` refuses names that would write outside its output
  directory. Not yet: `localedef`.
- **Closing, flushing and seeking stdio streams ([#20](https://github.com/duskglow/libniji/issues/20)):** `fclose` of a
  partly read seekable stream gives the unread input back; a failed flush sets the error indicator; a custom stream
  without a seek function reports `ESPIPE`. These follow POSIX and are recorded as differences.
- **Faster wide-character reads:** `fgetwc_unlocked` of a 2-byte character takes about 120 instructions instead of 640.
- **The debugger interface:** `_r_debug` is now the extended 48-byte structure with `r_version` 2.

## libniji: twentieth pre-release, 2026-11-21 (`snapshot-2026-11-21.4`)

`main` at commit `e4d7011e`, tagged `snapshot-2026-11-21.4`. Changes since
[`snapshot-2026-11-21.3`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-21.3).

- **Start files ([#27](https://github.com/duskglow/libniji/issues/27)):** `crt1.o` and its variants, `crti.o` and
  `crtn.o` carry the CET properties (IBT and shadow stack) and no longer add a second executable segment or 64 KiB of
  padding to every program.
- **valgrind ([#13](https://github.com/duskglow/libniji/issues/13), partly):** the loader's debug file has the local
  `strcmp` and `strlen` symbols valgrind needs, so `memcheck` starts for C and C++ programs. Not yet with the whole
  `libc6-dbg` installed (fixed in the next snapshot but one).
- **Fewer kernel transitions at start-up:** 8 fewer system calls per start, 7% to 11% fewer kernel instructions.

## libniji: nineteenth pre-release, 2026-11-21 (`snapshot-2026-11-21.3`)

`main` at commit `fe0899b7`, tagged `snapshot-2026-11-21.3`. Changes since
[`snapshot-2026-11-21.2`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-21.2).

- **A character cut short by end of input ([#23](https://github.com/duskglow/libniji/issues/23)):** a wide read cut
  short by end of input fails with `EILSEQ` and consumes the pending bytes, as on stock glibc. The other two kinds in
  #23 stay as recorded differences (a byte read right after a wide read on a wide-oriented stream, and sequences above
  U+10FFFF, which libniji rejects as RFC 3629 requires).
- **More shared objects per process:** the loader's tables for object paths and dependency lists grow with use; a
  process opening 300 libraries with long paths now works as on stock glibc.

## libniji: eighteenth pre-release, 2026-11-21 (`snapshot-2026-11-21.2`)

`main` at commit `71597d2a`, tagged `snapshot-2026-11-21.2`. Changes since
[`snapshot-2026-11-21.1`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-21.1).

- **malloc under an address-space limit ([#21](https://github.com/duskglow/libniji/issues/21), partly):** the heap's
  reservations grow with use; 56,021 KiB of 100-byte blocks fit within a 65,536 KiB margin (stock 58,456; the last
  snapshot 43,575), and `find` over 200,000 entries passes the issue's `ulimit -v` test 5 times out of 5.
- **Faster dynamic start-up:** about 5,500 fewer user instructions, 5 fewer page faults, 6 fewer system calls.
- **libresolv carries its own code:** code only `libresolv.so.2` uses is no longer in `libc.so.6`.
- **Complete debug files:** `libc6-dbg` carries debug information for every object, `iconv` and `locale` included.

## libniji: seventeenth pre-release, 2026-11-21 (`snapshot-2026-11-21.1`)

`main` at commit `15a2d648`, tagged `snapshot-2026-11-21.1`. Changes since
[`snapshot-2026-11-20.5`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-20.5).

- **`libm.so` and `libmvec` ([#28](https://github.com/duskglow/libniji/issues/28)):** `libm.so` in `libc6-dev` is a
  linker script, as on stock glibc, so `-lm` links `libmvec.so.1` only when a program uses a vector math function.
- **Character set names ([#6](https://github.com/duskglow/libniji/issues/6)):** `iconv_open` and `iconv` accept and
  refuse the same spellings as stock for 79 of the 81 we check (20.5: 67).
- **Thread cancellation in static programs ([#25](https://github.com/duskglow/libniji/issues/25)):** a thread
  cancelled or exited in a `-static` program runs its cleanups and destructors in stock's order (the C++ case hung).
- **Cut-short multibyte sequences ([#23](https://github.com/duskglow/libniji/issues/23)):** all 13 cases of the issue's
  check match stock (20.5: 11).
- The `niji6` package version carries the snapshot's serial, so same-day snapshots sort in order.

## libniji: sixteenth pre-release, 2026-11-20 (`snapshot-2026-11-20.5`)

`main` at commit `305065b0`, tagged `snapshot-2026-11-20.5`. Changes since
[`snapshot-2026-11-20.4`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-20.4).

- **Full library paths ([#24](https://github.com/duskglow/libniji/issues/24)):** a library found at start through the
  program's `$ORIGIN` is named by its full path in `dl_iterate_phdr`, `dladdr`, `dlinfo(RTLD_DI_LINKMAP)` and
  `_r_debug`, as on stock glibc.
- **Faster dynamic start-up:** about 42,000 fewer instructions per start (about 5%), one system call fewer.
- **Secure-mode `$ORIGIN`:** in a setuid or otherwise secure process, `$ORIGIN` comes from `/proc/self/exe`; if that
  cannot be read, it expands to nothing.

## libniji: fifteenth pre-release, 2026-11-20 (`snapshot-2026-11-20.4`)

`main` at commit `0e5a87e9`, tagged `snapshot-2026-11-20.4`. Changes since
[`snapshot-2026-11-20.3`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-20.3).

- **Thread exit and cancellation unwind C++ frames ([#25](https://github.com/duskglow/libniji/issues/25)):** on
  `pthread_exit` or cancellation, destructors of C++ locals run, `catch (abi::__forced_unwind &)` and `catch (...)`
  handlers are entered, and a held `std::lock_guard` is released; all ten cases checked behave as on stock glibc.
- **Start-up:** one page fault fewer per program start.

## libniji: fourteenth pre-release, 2026-11-20 (`snapshot-2026-11-20.3`)

`main` at commit `b732d9b4`, tagged `snapshot-2026-11-20.3`. Changes since
[`snapshot-2026-11-20.2`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-20.2).

- **Byte reads on a wide-oriented stream ([#23](https://github.com/duskglow/libniji/issues/23)):** `getc`, `fread`,
  `fgets`, `getdelim`, `ungetc` and `fputc` on a stream used for wide characters work on its bytes in order; util-linux
  `col` passes the issue's input through byte for byte. A multibyte sequence cut short by a byte that cannot continue
  it still lost its bytes (fixed in 21.1).
- **Executable stacks can be allowed (COMPAT-19):** programs, start-up libraries and `dlopen`ed libraries that ask for
  an executable stack are still refused by default, now with a message naming the object and how to allow it.
  `GLIBC_TUNABLES=niji.compat.execstack=1` accepts them and makes the stacks executable (not honoured in a secure
  process). See `docs/tunables.md`. A per-object allowlist is planned.

## libniji: thirteenth pre-release, 2026-11-20 (`snapshot-2026-11-20.2`)

`main` at commit `3f2a8452`, tagged `snapshot-2026-11-20.2`. Changes since
[`snapshot-2026-11-20.1`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-20.1).

- **Complete AddressSanitizer reports ([#26](https://github.com/duskglow/libniji/issues/26)):** ASan reports the main
  thread as `thread T0` and prints the allocation stack and the `SUMMARY` line, with exit status 1; 20 of 20 sampled
  gcc 14 ASan tests pass (14 of 20 on 20.1). LSan, TSan (ASLR off) and UBSan behave as on stock glibc.
- A sanitizer stack shows one more C library frame between `main` and `__libc_start_main` than on glibc.

## libniji: twelfth pre-release, 2026-11-20 (`snapshot-2026-11-20.1`)

`main` at commit `50abc44e`, tagged `snapshot-2026-11-20.1`. Changes since
[`snapshot-2026-11-19.2`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-19.2).

- **Sanitizers start ([#26](https://github.com/duskglow/libniji/issues/26)):** `ld.so` provides the private loader
  function GCC's sanitizer runtimes look up (`_dl_get_tls_static_info@@NIJI_PRIVATE`); programs built with
  `-fsanitize=address`, `leak` or `thread` reach `main`. ASan's report was incomplete (fixed in 20.2).
- **Dynamic sections readable ([#22](https://github.com/duskglow/libniji/issues/22)):** the dynamic sections of the
  program's libraries and of `dlopen`'d libraries can be read through the addresses the loader reports.
- **Dynamic start-up much faster:** an empty dynamically linked `main` runs in about 0.78 million instructions and 59
  page faults, down from 8.15 million and 76 (glibc 2.39: about 0.11 million and 49).

## libniji: eleventh pre-release, 2026-11-19 (`snapshot-2026-11-19.2`)

`main` at commit `d496d4c2`, tagged `snapshot-2026-11-19.2`. Changes since
[`snapshot-2026-11-19.1`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-19.1).

- **Debugger rendezvous: `_r_debug` exported ([#19](https://github.com/duskglow/libniji/issues/19)):** `ld.so`
  exports `_r_debug@GLIBC_2.2.5` (40 bytes) and keeps a program's copy current. Programs that copy it, such as Debian's
  `gnat1`, start; `gdb` lists shared libraries at `main` and stops at a breakpoint in a `dlopen`'d library. Keeping the
  program's copy current is a recorded difference with a compatibility entry (`COMPAT-35`).
- **`dl_iterate_phdr` names, in part ([#24](https://github.com/duskglow/libniji/issues/24)):** the loader and the C
  library are reported by full path, and a start-time library found through `$ORIGIN` is reported.
- C headers in progress: the build's headers are generated from libniji's ABI tables (not yet shipped).

## libniji: tenth pre-release, 2026-11-19 (`snapshot-2026-11-19.1`)

`main` at commit `fb63eeec`, tagged `snapshot-2026-11-19.1`. Changes since
[`snapshot-2026-11-18.2`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-18.2).

- **Canonical PLT entries honoured ([#10](https://github.com/duskglow/libniji/issues/10)):** the loader binds every
  library's references to a non-PIE executable's canonical PLT entry, so `&f` is the same address everywhere, as on
  glibc.
- **`strptime` number conversions take no sign ([#18](https://github.com/duskglow/libniji/issues/18)):** `%F`, `%Y`,
  `%s` and the others no longer accept a leading `+` or `-`; `%z` keeps its sign.
- C headers in progress (build only).

## libniji: ninth pre-release, 2026-11-18 (`snapshot-2026-11-18.2`)

`main` at commit `557dfdaa`, tagged `snapshot-2026-11-18.2`. Changes since
[`snapshot-2026-11-18.1`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-18.1).

- **Dynamic-section addresses relocated ([#22](https://github.com/duskglow/libniji/issues/22)):** the `d_ptr` entries
  (`DT_STRTAB`, `DT_SYMTAB`, `DT_GNU_HASH` and others) hold run-time addresses, as on glibc, for the program read
  through `_DYNAMIC` and for every object reached through `dl_iterate_phdr`; a PIE reading `DT_RUNPATH` through
  `DT_STRTAB` finds `$ORIGIN/`.
- More generated C headers in `niji6-dev`, with fixes to which declarations a header makes visible.

## libniji: eighth pre-release, 2026-11-18 (`snapshot-2026-11-18.1`)

`main` at commit `923df67e`, tagged `snapshot-2026-11-18.1`. Changes since
[`snapshot-2026-11-17.3`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-17.3).

- **More than 63 shared libraries ([#9](https://github.com/duskglow/libniji/issues/9)):** the loader no longer stops at
  a fixed number of start-time objects; programs needing 70 and 200 libraries start and run, directly and through
  `dlopen`, with output identical to stock glibc.
- **`getsockopt` and `setsockopt` with a NULL buffer ([#17](https://github.com/duskglow/libniji/issues/17)):** a NULL
  option buffer is passed to the kernel, which decides the result.
- **Character sets for `iconv` ([#6](https://github.com/duskglow/libniji/issues/6)):** `libc6` ships 43 charmaps in
  `/usr/share/i18n/charmaps/`; charmaps load without a fixed cap.
- **`/usr/bin/locale`** is part of `libc-bin`; `ldconfig` no longer caps its inputs; the named-semaphore table grows
  with use.
- `sigprocmask` with a null set follows POSIX.

## libniji: seventh pre-release, 2026-11-17 (`snapshot-2026-11-17.3`)

The day's third snapshot: `main` at commit `20dbed16`, tagged `snapshot-2026-11-17.3`. A snapshot of a work in progress,
not an alpha or a release candidate, and not for production. Changes since
[`snapshot-2026-11-17.2`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-17.2):

- **`ldconfig` and development symlinks ([#16](https://github.com/duskglow/libniji/issues/16)):** when a library's `.so`
  development link is an alias of its soname (libcmocka's `libcmockery.so`), `ldconfig` leaves the soname link on the
  real file instead of creating a loop; its links and output match Debian's own `ldconfig` in a trixie root.
- **`ldd` and `ld.so --list`** print the same paths and interpreter line as the reference; `ldd -v` still differs in the
  version lists it prints, as documented.
- **Symbol-less version nodes** (`GLIBC_ABI_DT_RELR` and others) are exported, so programs whose binaries require them load.
- **`/usr/bin/iconv`** is part of `libc-bin`, with libniji's own converters; more character sets follow (#6).
- `pthread_rwlock_tryrdlock` and `pthread_rwlock_trywrlock` return `EBUSY` when the lock is held.

Packages: the same two flavours (beside glibc: `niji6`, `niji6-dev`, version `0.0~snapshot20261117+20dbed16c07e`; as the
system C library, experimental: `2.39+niji20261117.3-1`, including `libc6-dbg`). Checks: empty root 19 pass, none fail;
trixie root with libniji swapped in 65 commands, 63 identical to stock glibc, 2 known differences (`ldd -v`), none
unexpected; lintian 10 errors (unchanged) and 59 warnings per suite (both new ones on `/usr/bin/iconv`).

Known gaps specific to this snapshot: the system flavour has no C headers yet (its `libc6-dev` replaces Debian's, so
compile with the flavour beside glibc); `libc6-dbg` has no debug file for `/usr/bin/iconv` (17 of 18 objects resolve).
Other limitations are as in the previous snapshot (#9, #19, 512 objects per process, no `dlmopen`, lazy binding,
`LD_AUDIT` or `libthread_db`, no security audit, Linux x86-64 only). Next: more `iconv` character sets, the loader
queue (#9, #10, #19), fixes for Yohane Linux (#17, #20, #21, #18), and the generated C headers.

## libniji: sixth pre-release, 2026-11-17 (`snapshot-2026-11-17.2`)

The day's second snapshot: `main` at commit `e4010743`, tagged `snapshot-2026-11-17.2`. Changes since
[`snapshot-2026-11-17`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-17). Snapshots now follow the
fixes for [Yohane Linux](https://github.com/duskglow/aqours) as they land, so several may appear in a day; the later ones
carry a `.N` suffix.

- **The vector math library, `libmvec.so.1`**, ships in `libc6` and `niji6`, with `libmvec.a` and `libmvec.so` in the
  development packages, so programs built against glibc's vector math entry points (Debian's graphviz) load and run.
  Each lane is computed by libniji's scalar `libm` function, so results match the scalar functions exactly; the vector
  entry points are not yet faster than the scalar ones.
- **A debug package, `libc6-dbg`** (system flavour, optional): the debug files of the shared objects, which ship
  stripped. Debuggers find each by build-id (all 17 objects resolve); full debug information is present for some objects
  so far and completing the rest is queued.
- `pow` special cases follow IEEE 754 and C23: a signalling-NaN operand gives a quiet NaN and raises invalid, and
  `pow(-1, y)` for a huge even or odd integer `y` returns ±1.

Packages: beside glibc `0.0~snapshot20261117+e4010743214e`; system flavour `2.39+niji20261117.2-1` (experimental).
Checks: empty root 19 pass; trixie root with libniji swapped in 63 commands, 56 identical, 7 known differences (the
`ldd` output), none unexpected; lintian 10 errors and 57 warnings per suite. The system flavour has no C headers yet.
Known gaps: #9 (more than 63 shared libraries at start), #19 (no `_r_debug` rendezvous). The `ldconfig` fix (#16) and the
`ldd` trace output came in the next snapshot.

## libniji: fifth pre-release, 2026-11-17 (`snapshot-2026-11-17`)

This is the fifth public pre-release, and the first under the project's new name, **libniji**. `main` is promoted to
commit `30d07e74`, tagged `snapshot-2026-11-17`. Like the earlier ones, it is a snapshot of a work in progress, not an
alpha or a release candidate, and it should not be used in production. These notes cover the changes since
[`snapshot-2026-11-16`](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-16).

### The new name

Nothing had been released, so the old names have no compatibility aliases. The C library's own Debian package names
(`libc6`, `libc-bin`, ...), `libc.so.6` and the loader's name do not change. The table of old and new names (packages,
the version suffix, the install prefix, the native interface library, headers, version nodes, macros, variables,
tunables and run-time names) is in
[`docs/install-packages.md`](install-packages.md#names-before-libniji), and on the
[GitHub release](https://github.com/duskglow/libniji/releases/tag/snapshot-2026-11-17).

### Summary

Four issues found by [Yohane Linux](https://github.com/duskglow/aqours) (the distribution built on libniji, formerly
Aqours) in its first stage are fixed or partly fixed:

- **[#7](https://github.com/duskglow/libniji/issues/7):** binaries built against older C libraries, such as Debian's
  `gawk`, start: `libm.so.6` and `libc.so.6` now export the old `GLIBC_2.2.5` versions they bind (`fmod`, `fmodf`, `exp`,
  `exp10`, `pow`, `hypot`, `hypotf`, `log`, `log2`, `pthread_sigmask`).
- **[#5](https://github.com/duskglow/libniji/issues/5):** the system packages ship `libresolv.so.2` and `libresolv.so`,
  so Debian programs that need it (`libkrb5` and its users) load. `libresolv.a` stays an empty archive:
  its functions are in `libc.a`, so a static `-lresolv` link works.
- **[#8](https://github.com/duskglow/libniji/issues/8):** `getaddrinfo` returns `EAI_AGAIN` ("Temporary failure in name
  resolution") when no nameserver can be reached, as the reference does, instead of `EAI_SYSTEM`.
- **[#6](https://github.com/duskglow/libniji/issues/6), partly:** `iconv` converts EUC-JP (a round trip is byte for byte
  identical), and 58 of the 1180 character-set names the reference lists open. ISO-8859-15 and most of the others do
  not open yet, and Debian's own `iconv` program does not start on libniji yet. The issue stays open; the work to cover
  the rest is under way.

Also new: **`ldd`** ships in `libc-bin`; the development package carries the remaining development links and archives
(`libBrokenLocale`, `libg.a`, `libmcheck.a`, `libpthread_nonshared.a`); and `libc6-dev`'s control fields now pull in
`linux-libc-dev`, `libcrypt-dev` and `rpcsvc-proto` and provide a versioned `libc-dev`, as Debian's do.

### Packages

The same two flavours as before, built by `tools/deb/build.sh` for Debian 13 (trixie) and Devuan 6 (excalibur), amd64,
attached with `SHA256SUMS`. GitHub does not allow `~` in asset names, so the `niji6` files are published with `.` in its
place. `SHA256SUMS` lists the names as published, and the package versions inside the files are unchanged.

- **Beside glibc** (`niji6`, `niji6-dev`), version `0.0~snapshot20261117+30d07e74b559`.
- **As the system C library** (`libc6`, `libc-bin`, `libc6-dev`, `libc-dev-bin`, `locales`, `libc-l10n`), version
  `2.39+niji20261117.1-1`, **experimental**.

Checks on this commit:

| Check | Result |
|---|---|
| Empty root | 19 checks pass, none fail |
| Trixie root with libniji swapped in | 63 commands: 56 identical to stock glibc, 7 known differences, none unexpected. The 7 are `ldd` output: library paths spelled `/usr/lib/...` where the reference prints `/lib/...`, the interpreter line, and `ldd -v`'s blocks. A fix is in progress |
| The Yohane issues | #7, #5 and #8 checked with Debian's own binaries in the trixie root; #6 as described above |
| Symbol check | Symbol sets as in the last snapshot, plus the new old-version exports and `libresolv.so.2` |
| Lintian | 10 errors and 40 warnings per suite (7 and 48 before). The 3 new errors flag the new empty archives (`libg.a`, `libmcheck.a`, `libpthread_nonshared.a`), which are empty on purpose. 8 warnings fewer because `libc6` now declares its self-provided `libc.so.6` prerequisite, 2 fewer with the added development links, and 2 new ones on `ldd` |

**The system flavour has no C headers yet.** Its `libc6-dev` replaces Debian's, so a root using the system packages
cannot compile C until libniji's own headers land. The flavour beside glibc keeps the system's headers.

Still true: swapping into a stock trixie root needs `--force-depends`, because some trixie packages require a C
library version of at least 2.41 and libniji presents 2.39.

### Known limitations

- **[#9](https://github.com/duskglow/libniji/issues/9):** a program that needs more than 63 shared libraries at start
  fails before `main` with "too many objects". A fix (no fixed limit) is queued at high priority.
- at most 512 loaded objects per process in all (start-time objects plus `dlopen`);
- no `dlmopen`, no lazy binding, no `LD_AUDIT`, and no debugger rendezvous (`_r_debug`; it landed in
  `snapshot-2026-11-19.2`, [#19](https://github.com/duskglow/libniji/issues/19)) or `libthread_db` yet, so
  debuggers cannot follow shared libraries or threads in a running program;
- no vector math library (`libmvec.so.1`) in this snapshot (it has landed since);
- no C headers of libniji's own yet;
- Rust `std` programs are not supported yet;
- no security audit yet;
- Linux x86-64 only.

### What's next

- **The loader:** issue #9 (no limit on start-time objects), the trace output of `ldd` and `ld.so --list`, a split of
  the loader into modules, and the debugger rendezvous.
- **`iconv`:** the remaining character sets (#6).
- **Packages:** `libmvec.so.1`, `libc6-dbg`, `libthread_db.so.1`.
- **The header generator** and the first generated C headers.

<!-- History: the entries below describe what was released under the project's earlier names and keep
     them. Add a new release above this line. -->

## Liella (now libniji): fourth pre-release, 2026-11-16 (`snapshot-2026-11-16`)

This is the fourth public pre-release: `main` promoted to commit `23b9dfdb`, tagged `snapshot-2026-11-16`. Like the
earlier ones, it is a snapshot of a work in progress, not an alpha or a release candidate, and it should not be used
in production. These notes cover the changes since
[`snapshot-2026-11-15`](https://github.com/duskglow/liella/releases/tag/snapshot-2026-11-15).

**A name change is coming.** Liella will be renamed **libniji** before its first versioned release. This is the last
snapshot under the old name. The next one is published as libniji, with a table mapping the old names to the new ones
(packages, paths, macros, version names). Nothing has been released yet, so the old names get no compatibility aliases.

### Summary

- **Dynamically linked C++ programs** built against the distribution's shared `libstdc++.so.6` now run with Liella as
  the system C library, including an exception thrown across a shared library.
- **`LD_PRELOAD` and the preload file** are honoured at program start. `fakeroot` works in a Debian root running on
  Liella.
- **`getconf` and `getent`** are part of Liella and ship in `libc-bin`.
- **`apt`** runs in a Debian 13 (trixie) root with Liella swapped in for glibc. That root now runs 52 commands with the
  same output and status as on stock glibc (47 before).
- **Start-up page-fault bounds** tightened again.

### Dynamic linking

- **C++ unique symbols.** `libstdc++` relies on `STB_GNU_UNIQUE` symbols (106 in Debian's build). The loader now binds
  them at start by the order of the global scope, with no fixed table. Before, a fixed 64-entry table overflowed, and the
  failure was reported as "malformed symbol tables". `dlopen`'s table of unique symbols now grows as needed, with no cap,
  and running out of memory is reported as such.
- **Preloading** works as ld.so(8) describes. The objects named in `LD_PRELOAD` (separated by spaces or colons) load
  first, then those named in the preload file: `/etc/ld.so.preload` for the system packages, and
  `/usr/lib/liella/etc/ld.so.preload` for the packages that sit beside glibc. They load after the program and before
  its libraries, so their symbols interpose, and `dlsym(RTLD_NEXT, ...)` reaches the next definition. A name without a
  slash is searched like a `DT_NEEDED` name. An object that cannot be preloaded prints the reference's message, and
  the program runs anyway. In a set-user-ID or set-group-ID
  program, `LD_PRELOAD` accepts only a name without a slash, from the trusted directories, and only for a library that
  is itself set-user-ID. See
  [`docs/install-packages.md`](https://github.com/duskglow/liella/blob/snapshot-2026-11-16/docs/install-packages.md).

### C library utilities

`libc-bin` now ships two more utilities beside `ldconfig`:

- **`getconf`**, with its table of names generated from Liella's ABI tables. Its output matches the reference in 25
  recorded transcripts.
- **`getent`**, dynamically linked and using Liella's own name services (`files` and `dns`).

Both are compared against the reference with the transcript harness, in private roots.

### Packages

The same two flavours as before, built by `tools/deb/build.sh` for Debian 13 (trixie) and Devuan 6 (excalibur), amd64,
attached with `SHA256SUMS`. GitHub does not allow `~` in asset names, so the `liella6` files are published with `.` in
its place. `SHA256SUMS` lists the names as published, and the package versions inside the files are unchanged.

- **Beside glibc** (`liella6`, `liella6-dev`), version `0.0~snapshot20261116+23b9dfdb5880`.
- **As the system C library** (`libc6`, `libc-bin`, `libc6-dev`, `libc-dev-bin`, `locales`, `libc-l10n`), version
  `2.39+liella20261116.1-1`, **experimental**.

Checks on this commit:

| Check | Result |
|---|---|
| Empty root | 15 checks pass, including a C++ `iostream` program and an exception thrown across a shared library, both against the shared `libstdc++.so.6` |
| Trixie root with Liella swapped in | 52 of 52 commands identical to stock glibc. New: `apt-get check`, `apt-cache policy`, and two `fakeroot` cases |
| Symbol check | All symbol sets as in the last snapshot |
| Lintian | 7 errors and 48 warnings per suite. The errors are the same as in the last snapshot; the 3 new warnings are on the new utilities (no manual pages yet for `getconf` and `getent`) |

Still true: swapping into a stock trixie root needs `--force-depends`, and dpkg leaves glibc's `/var/cache/ldconfig`
behind.

**Known issue in the release check, not in the packages.** The package-layout check (`tools/deb/check-system.sh`) at
this commit does not yet list `/usr/bin/` and `/usr/bin/getent` for `libc-bin`, so it reports two problems on these
packages. The packages are as intended. The check is fixed on `next`.

### Start-up layout

The start-up page-fault bounds of each product profile (`true`, `locale`, the tools, `localtime`) were tightened to
the level the 64 KiB window bins reach, and two other CPU classes measure the same. Bounds only ever tighten.

### C headers

Liella's own C headers are being generated from its ABI tables. Since the last snapshot, the tables of which names
each header makes visible (by feature-test level), which calls each header declares, and which C++ declarations are
`noexcept` have landed. The generator and the first headers come next.

### Known limitations

Unchanged from [`snapshot-2026-11-15`](https://github.com/duskglow/liella/releases/tag/snapshot-2026-11-15), except
that `LD_PRELOAD` is now provided:

- at most 512 loaded objects per process;
- no `dlmopen`, no lazy binding, no `LD_AUDIT` and no `libthread_db` yet;
- no C headers of Liella's own yet;
- Rust `std` programs are not supported yet;
- no security audit yet;
- Linux x86-64 only.

### What's next

- **The rename to libniji.**
- **[Yohane Linux](https://github.com/duskglow/aqours)** (formerly Aqours), the Linux distribution built on Liella,
  starts its first stage from this snapshot.
- **More utilities:** `locale`, `localedef` and the time-zone tools.
- **The loader's trace output** (`ld.so --list`, `LD_TRACE_LOADED_OBJECTS`) is being made to match the reference
  exactly.
- **The header generator** and the first generated C headers.

## Liella: third pre-release, 2026-11-15 (`snapshot-2026-11-15`)

This is the third public pre-release: `main` promoted to commit `bade54e0`, tagged `snapshot-2026-11-15`. Like the
earlier two, it is a snapshot of a work in progress, not an alpha or a release candidate, and it should not be used
in production. These notes cover the changes since `snapshot-2026-11-07`.

Ratios are Liella/glibc 2.39 on the same machine: below 1.00 means Liella takes less, above 1.00 more.

### Summary

- **The dynamic loader is complete enough to run real programs.** Liella's `ld.so` loads shared objects, finds them
  by name at start-up, starts `libc.so.6`, runs constructors and destructors, creates threads with thread-local
  storage, and provides `dlopen`, `dlsym`, `dlvsym`, `dlclose`, `dladdr`, `dladdr1` and `dlinfo`.
- **Liella can be the system C library.** New experimental *system* packages (`libc6`, `libc-bin`, `libc6-dev` and
  friends) replace glibc's. In a Debian 13 (trixie) root with Liella swapped in for glibc, 47 commands of the
  distribution's own programs (`ls`, `sort`, `grep`, `sed`, `awk`, `bash`, `perl`, `python3`, `find`) produce the
  same output and status as on stock glibc.
- **`ldconfig`** is now part of Liella, along with its dpkg trigger.
- **Start-up layout**: the build traces what it has just built, and start-up code is placed in fixed 64 KiB windows,
  so unrelated changes no longer move start-up page-fault counts.
- **`libm`**: on an AMD Zen 3 host the median time ratio of the `libm` rows fell from 6.19 to 1.98.
- **`regcomp`** compiles patterns at any nesting depth and has no fixed size cap.
- **Reference packages are attached to this release**, with `SHA256SUMS`.

### Dynamic linking: what `ld.so` does now

`cargo xtask so` builds `target/liella/ld-linux-x86-64.so.2`. Since the 11-07 snapshot it:

- **maps every loadable object through one validated reservation** and accepts any number of writable `PT_LOAD`
  segments, keeping the bounds, no-W+X, RELRO and secure-mode checks;
- **finds `DT_NEEDED` libraries by name at program start** through the same search as `ld.so --list` and `dlopen`:
  `DT_RPATH`, `LD_LIBRARY_PATH` (ignored in secure mode), `DT_RUNPATH`, the loader cache (`/etc/ld.so.cache`), then
  the default directories. A program whose dependencies are all named by path does no extra work. Liella's own
  libraries (`libc.so.6`, `libm.so.6`, ...) resolve only from Liella's library directory, or from `LD_LIBRARY_PATH`
  for testing a build, never through `RPATH`/`RUNPATH` or the cache (a recorded difference);
- **starts `libc.so.6`**: runs libc's early initialisation from `ld.so`, refuses a `libc.so.6` from a different build,
  runs IFUNC resolvers once in a deferred pass, and runs constructors and destructors in the reference's order;
- **creates threads with TLS** in dynamically linked programs;
- **`dlopen`, `dlsym`, `dlvsym`**: the loader's readers are lock-free and safe against a concurrent `dlopen`;
  `dl_iterate_phdr` callbacks run with no loader lock held; the TLS blocks of `dlopen`ed objects are placed at `dlopen`;
- **`dlclose`** unloads objects. A C++ `thread_local` destructor that is still pending keeps its object loaded, and the
  object unloads after that destructor runs;
- **`dladdr`, `dladdr1` and `dlinfo`**.

**Smoke test with real programs.** Debian trixie sources built against Liella with their own test suites (zlib,
xz-utils, sqlite3, grep, sed, diffutils, findutils, coreutils, gawk, bash, perl, python3 and cppunit), plus programs that
load and unload plug-ins repeatedly (Lua modules, a C++ library with `thread_local` state). The three behaviour
findings of the first run (`readdir` on a removed directory, `regcomp` refusing deep nesting, `errno` after a failed
`setlocale`) are fixed, and so is a crash the second run found in `dlclose` with a pending `thread_local` destructor.

### Packages

Both flavours are built by `tools/deb/build.sh` for Debian 13 (trixie) and Devuan 6 (excalibur), amd64, and are
attached to this release with `SHA256SUMS`. Two builds of the same commit give byte-identical packages. GitHub does not allow `~` in asset names, so
the `liella6` files are published with `.` where their Debian version has `~` (`0.0.snapshot...` for
`0.0~snapshot...`); `SHA256SUMS` lists the names as published, and the package versions inside are unchanged. See
[`docs/install-packages.md`](install-packages.md).

**Beside glibc (`liella6`, `liella6-dev`)**, version `0.0~snapshot20261115+bade54e077b7`: everything under
`/usr/lib/liella`; glibc and every existing program are untouched, and purging the packages leaves the system as it
was. A program selects Liella through its interpreter: `liella-gcc` / `liella-g++` link with
`--dynamic-linker=/usr/lib/liella/lib64/ld-linux-x86-64.so.2`.

**As the system C library (`libc6`, `libc-bin`, `libc6-dev`, `libc-dev-bin`, `locales`, `libc-l10n`)**, version
`2.39+liella20261115.2-1`, **experimental**: these replace glibc's packages, for building a system on Liella (the
Aqours project (now Yohane Linux), below). They ship the loader at `/lib64/ld-linux-x86-64.so.2`, the multiarch library directory,
`ldconfig` and its dpkg trigger, dpkg `symbols` and `shlibs` files so packages built against Liella get correct
dependencies, and the start files, so plain `gcc` links against Liella. Validated on this commit in an empty root (C and
C++ programs start through the system loader) and in a trixie root with Liella swapped in (the 47 commands above).
- The version sorts above every `libc6 (>= 2.39)` dependency and below 2.40, so a package that needs a glibc 2.40 or
  2.41 symbol honestly does not install. Swapping into a stock trixie root therefore needs `--force-depends`
  (trixie's `util-linux` depends on `libc6 (>= 2.41)`).
- A swap leaves glibc's `/var/cache/ldconfig` behind (not owned by Liella's `libc-bin`).
- Liella has no C headers of its own yet: `libc6-dev` programs compile with the system's glibc headers.

### Start-up layout

The build traces the products it has just built (under `gdb`) and groups the code and data the trace touched; no
function-name list is committed any more. Start-up text is placed in 64 KiB windows planned per program mode, each
window ending on a window boundary, so a change outside a window cannot move that window's page faults. The start-up
page-fault bounds only tighten (the release `localtime` bound went from 42 to 40 in this snapshot).
- A product build needs a tracer; `LIELLA_LAYOUT_TRACE=ungrouped` builds an ungrouped product without one
  ([`docs/plants.md`](plants.md)).
- A per-change byte budget per window, replacing page-fault counts as the per-change check, is next.

### `libm`

Timing sweep of every `libm` row on AMD Zen 3 (Ryzen 5 5500 class), `next` at `bc146cce`, 2026-11-13, beside the
11-07 sweep on the same host ([`results/timing-sweep-zen3-libm-1113-table.md`](../results/timing-sweep-zen3-libm-1113-table.md)):
176 rows, 28 faster, 7 parity, 141 slower. The median time ratio fell from 6.19 to 1.98 and the geometric mean from
9.76 to 3.25; 21 rows moved from slower to faster or parity. Complex, `long double` and the error and gamma families
are still far slower than glibc.

- **Quick first phases** for the `exp`, `log`, trigonometric, inverse trigonometric and hyperbolic families and `pow`,
  and `float` functions computed in one pass in `double`.
- **Slow tails removed**: `hypot` has one correctly rounded path for every finite input (with an FMA build), and
  `lgamma`/`lgammaf` answer edge arguments without the slow path.
- **Hardware square root through a safe interface**: the math kernels stay free of `unsafe` at no cost.
- Correct rounding is kept; each family's accuracy contract is a recorded difference.

### Regular expressions

- **`regcomp` at any nesting depth**: the parse tree is an arena, the parser uses explicit stacks, and the compiler
  recurses natively for 64 levels and continues deeper patterns with an explicit frame stack.
- **No fixed size cap**: every compiler allocation is fallible and bounded by the process's own memory limits;
  exhaustion is `REG_ESPACE`, never an abort.
- **Cheaper repetition scopes**, and back-references at 22,913 instructions per call on the back-reference row
  (glibc 21,970).

### glibc 2.41 functions

`sched_getattr` and `sched_setattr` (glibc 2.41) are implemented. Liella's ABI level stays glibc 2.39 until the
2.40/2.41 additions are complete; in the dpkg `symbols` file these two depend on this Liella build or later.

### Compatibility flags and differences

- The decision, compatibility and saturation registers are one file per record under `records/`; the tables in `docs/`
  are generated from them (`cargo xtask records-gen`).
- **Compatibility flags**: 30 registered (COMPAT-1 to COMPAT-30). A row is a promise to design the flag, not an
  implemented flag.
- **Differences from glibc** ([`docs/differences-from-glibc.md`](differences-from-glibc.md)), new loader rows
  among them: a failing `dlopen` runs no IFUNC resolver of the new set; malformed objects are refused before anything
  is mapped; objects that ask for an executable stack are refused; `dlsym`, `dlvsym` and `dlclose` refuse a handle
  `dlopen` did not return or one already closed; `dlopen` in a statically linked program fails cleanly; Liella's own
  libraries resolve only from its own directory.

### Changes users may notice

- A failed `setlocale` sets `errno` (`ENOENT` for a name, `EINVAL` for an unknown category).
- `dlopen` with any of the eight high mode bits `0x01000000`-`0x80000000` fails with `invalid mode parameter`: they
  are reserved for Liella, and public loader features will come as functions, not through these bits.
- An opt-in link interface registers exit and fork handlers without `libc_nonshared.a`
  ([`docs/install-packages.md`](install-packages.md)); the default is unchanged.

### Known limitations

- **At most 512 loaded objects per process**, the start-time objects included; the `dlopen` that would load one more
  returns `NULL`. glibc has no fixed limit; growing the table is planned.
- **`dlmopen` and link-map namespaces are not provided yet.**
- **No lazy binding**: symbols are bound at load time.
- **Not available yet**: `LD_PRELOAD`, `LD_AUDIT`, `libthread_db` (debugger support), `libc_malloc_debug`; `dlopen`
  in statically linked programs (fails cleanly).
- **No C headers of Liella's own yet** (in progress, below).
- **Running unmodified glibc-linked binaries** that need compatibility-only symbol versions is not supported yet.
- **Rust `std` programs** are not supported yet.
- **No security audit yet.**
- **Platforms**: Linux x86-64 only.

### What's next

- **[Aqours](https://github.com/duskglow/aqours)**: a Linux distribution based on Devuan, built on Liella, under
  active development. The loader and the system packages in this snapshot are what its first stage needed.
- **Liella as the system C library**: Liella's own C headers (generated from its ABI tables and the standards; the
  first slices have landed), the remaining C library utilities (`ldd`, `getent`, `getconf`, `iconv`, `locale`,
  `localedef`) and the time-zone tools, and the glibc 2.40/2.41 additions programs need.
- **Start-up layout**: the per-window byte budget as the per-change check.
- **Loader**: growing the 512-object table, `dlopen` in static programs, lazy binding, `LD_PRELOAD`, `libthread_db`.
- **`libm`**: the remaining slow families.

## Second pre-release, 2026-11-07

This is the second public pre-release: the `next` branch promoted to `main` around 2026-11-07. Like
the first, it is a public work in progress, not an alpha or a release candidate, and it should not
be used in production. It is tagged `snapshot-2026-11-07`: a snapshot of the development tree, not a
version.

Every figure below says where it comes from and when it was measured. "The measured tree" is `next`
at commit `136b5246` (2026-11-07), the tree of this pre-release; timing figures name the tree they were
measured on. Work still on its way is named "in progress", without figures. Ratios are Liella/glibc 2.39 on the same
machine: below 1.00 means Liella takes less, above 1.00 more. A timing verdict is "faster" below
0.95, "slower" above 1.05 and "parity" between.

### Summary

- **Dynamic linking (phase 14) has started, and dynamically linked programs still do not run.**
  Liella now builds an `ld.so` that relocates itself and starts a program that needs no shared
  object. It does not load shared objects yet.
- **Static PIE** programs (`-static-pie`) are supported.
- **`libm`**: `exp` and `log` have a new quick first phase, the x87 `sinl` and `cosl` take about
  half of glibc's cycles, and the complex `_Float128` functions (milestone 5) are done. Most `libm`
  rows are still slower than glibc's, `long double` and complex ones by far.
- **Strings, memory and locale**: most timed rows are at parity with glibc or faster on the three
  classes of processor timed.
- **Symbols**: 557 of the 4,305 names of the glibc 2.39 ABI are not defined yet (716 on 2026-10-24).

### Landed late in this run

These landed on `next` on 2026-11-07, after the timing sweeps below were measured:

- **`expf`, `exp2f`**: a one-pass `float` path had left a dependency between consecutive calls
  (a conversion into a register it did not clear first). Fixed: booked timing on the N100, `expf`
  3.11 to 1.66 and `exp2f` 3.13 to 1.67 (Liella/glibc, before/after the fix); on Ivy Bridge `expf`
  2.51 to 1.24.
- **Quick first phases** for `exp2`, `exp10`, `log2`, `log10`, `log1p`, `sin`, `cos`, `tan` and
  `pow`. In progress: `expm1`, the inverse hyperbolic functions. No sweep figures for these yet.
- **Wide-character classification** in the built-in `C` and `C.UTF-8` locales reads one two-stage
  table of class words, and `towctrans` recognises the `toupper`/`tolower` descriptors directly.
- **Start-up page faults**: a statically linked coreutils `printf` (`printf '%s %d %x\n' a 1 1`,
  address randomisation off, one warm-up and five identical runs) takes 61 page faults on the
  measured tree and 62 on the 2026-10-24 tree, on the same host as the test figures below. Run-time
  start-up functions are kept in the hot text group again, and a build check now fails when a hot-text
  key matches no function.

### Tests and symbols on the measured tree

**Tests** (`cargo xtask status` on the measured tree, 2026-11-07, Intel Xeon Platinum 8151 (Skylake-SP),
Linux 6.8): 5,484 tests are specified and 577 of them are still pending (stubs for behaviour not yet
implemented): 85 belong to phase 14, 316 to phase 15, 173 to the mathematics track and 3 are
listed deferrals.

**Symbols** (`cargo xtask so && cargo xtask symcov` on the measured tree, 2026-11-07, the same
host): the glibc 2.39 ABI table (`spec/abi/versions.tsv`) has 4,305 names (4,600 version rows), and
557 of those names are not defined by any Liella product yet:

| Class | Names | Planned in |
|---|---:|---|
| dynamic linking: `ld.so` 33, the `dl*` functions of `libc.so.6` 11, `libthread_db` 40, `libc_malloc_debug` 26 | 110 | Phase 14 |
| compatibility-only versions in libraries Liella ships (`libc.so.6` 111, `libm.so.6` 37, `libresolv.so.2` 11) | 159 | Phase 15 |
| legacy tier, default version (`__libc_sa_len`, `bindresvport`) | 2 | Phase 15 |
| `GLIBC_PRIVATE` (internal to glibc's own libraries: `libc.so.6` 278, `libnsl.so.1` 6, `libresolv.so.2` 2) | 286 | out of scope |

Against the 2026-10-24 table: the 18 complex `_Float128` names are defined (`libm` milestone 5 is
closed); `libnsl.so.1` is now built (below), so its 121 public names are defined and its 6
`GLIBC_PRIVATE` names moved to the out-of-scope row; 17 ONC RPC names of `libc.so.6` are now
exported (below).

### Dynamic linking (phase 14): what `ld.so` does today

`cargo xtask so` now also builds `target/liella/ld-linux-x86-64.so.2`. Today it:

- **relocates itself** before it reads any word a relocation writes. The build checks this: it
  follows every function `ld.so`'s entry reaches before the relocation pass and refuses the product
  if one of them reads a relocated word (D-27.13);
- **is a product of its own**: it links, passes the build's link checks, and answers `--version`
  and `--help` (its own text; [`docs/differences-from-glibc.md`](differences-from-glibc.md));
- **starts a position-independent program that needs no shared object, both ways**: as the
  program's interpreter (`PT_INTERP`), and as `ld.so PROGRAM ARGS`. It applies the program's
  relative relocations (RELA and RELR) and its read-only-after-relocation segment, then enters it.
  The tests start such a program 100 times each way, half of them with address randomisation off
  (D-27.14);
- **clears the environment in secure mode**: it copies the auxiliary vector, decides secure mode
  once from `AT_SECURE`, removes the variables on its unsecure list in secure mode, and only then
  reads its own `LD_*` settings and tunables (D-27.14).

What it does not do yet: load shared objects (a program that names one in `DT_NEEDED` is refused,
with exit status 127), so `libc.so.6` itself is not loaded and **dynamically linked programs still
do not run against Liella**; `dlopen`, `dlsym` and the other `dl*` functions; lazy binding; TLS for
modules; `LD_PRELOAD`, `LD_LIBRARY_PATH` and `LD_AUDIT`; `libthread_db`; `libc_malloc_debug`.
The phase's story list is
[`plan/06-phases/17-phase-14-dynamic-linking.md`](../plan/06-phases/17-phase-14-dynamic-linking.md).

### Static PIE

Programs can be linked against `libc.a` as static position-independent executables, with the start
file `liella-rcrt1.o` (D-27.10). The start code relocates the program with the same self-relocator
as `ld.so`, then records the load bias once (D-27.12). `dl_iterate_phdr`, `_dl_find_object`,
`backtrace`, `backtrace_symbols` and the `%n` read-only check use that bias. One conformance
program starts a C++ static PIE at 100 random load addresses and checks these interfaces and an
exception thrown through three frames; another checks the `backtrace` addresses and text.

### `libm`

Since 2026-10-24:

- **Quick first phase for `exp` and `log`** (D-10.51; D-10.54 for `exp`, D-10.60 for `log`): a short
  computation that returns the correctly rounded result when its error test decides it, and
  otherwise hands the argument to the earlier fast path, in both builds (with and without FMA). Time ratios from the sweep
  tool on idle timing hosts, 2026-11-07, on the code that landed:

  | Function | Ivy Bridge (no FMA) | N100 | Before (11-07 sweep, Ivy Bridge / N100) |
  |---|---:|---:|---:|
  | `exp` | 1.89 | 2.17 | 2.90 / 3.35 |
  | `log` | 1.94 | 2.11 | 5.18 / 5.88 |

  The Ivy Bridge `exp` figure had a large round-to-round spread (0.68); read it as approximate.
  Both functions remain about twice as slow as glibc's on these two classes.
- **x87 `sinl`, `cosl` and `sincosl`**: a new tier in integer arithmetic (D-10.52, D-10.56). Cycles
  per call against glibc (`perf stat`, three interleaved rounds, idle hosts, 2026-11-07): Ivy Bridge
  `sinl` 0.48, `cosl` 0.48, `sincosl` 0.74; N100 `sinl` 0.55, `cosl` 0.56, `sincosl` 0.81. Large
  arguments (10^10, 10^300) measure the same. The 11-07 sweep, before this tier, timed `sinl` and
  `cosl` at 112 (Ivy Bridge) and 88 (N100).
- **`float` `expf` and `exp2f`**: a single pass in `double` arithmetic. Not yet timed on the timing
  hosts.
- **Fast tiers that landed on 2026-11-05 and 11-06**: `double` `sin`, `cos`, `tan` and `sincos`;
  FMA builds of `exp2`, `log2`, `log10` and `pow`; `expm1`, the hyperbolic functions and `cbrt`;
  inverse trigonometric functions and `hypot`; Bessel, gamma and error functions; `float`
  hyperbolic functions. Examples from the 11-07 sweep against the 11-05 snapshot on Ivy Bridge:
  `sin` 4.04 to 2.22, `cos` 3.92 to 2.13, `tan` 4.96 to 2.83, `sinhf` 11.8 to 0.66, `tgammaf`
  1,270 to 0.58.
- **Milestone 5**: the 18 complex `_Float128` functions are implemented.
- **In progress**: quick phases for `exp2`, `exp10`, `expm1`, `log2`, `log10`, `log1p`, `sin`, `cos`,
  `tan`, `pow` and `cexp`.

**Where `libm` stands** (timing sweep of every `libm` row, `next` at `66b8d3ba`, 2026-11-07, Intel
Xeon E3-1265L v2 (Ivy Bridge) and Intel N100; [`results/timing-sweep-libm-1107-table.md`](../results/timing-sweep-libm-1107-table.md)).
This sweep ran before the `exp`, `log`, `sinl`/`cosl` and `expf`/`exp2f` changes above. Of 166
rows, by time ratio:

| Time ratio | Ivy Bridge | N100 |
|---|---:|---:|
| 1.05 or less (faster or parity) | 6 | 6 |
| over 1.05 to 2 | 23 | 22 |
| over 2 to 5 | 39 | 49 |
| over 5 to 20 | 52 | 43 |
| over 20 to 100 | 9 | 18 |
| over 100 | 37 | 28 |
| median | 6.98 | 5.83 |

Median by precision (Ivy Bridge / N100): `float` 2.27 / 2.66, `double` 3.62 / 3.52, `_Float128`
10.4 / 7.5, `long double` 115 / 84. The complex functions have a median of 274 / 239. The rows
over 100 are mostly `long double` and complex functions.

### Strings, memory and locale

Timing sweep 1106E (`next` at `34c98f61`, 2026-11-06, every non-`libm` row; Ivy Bridge (Xeon
E3-1265L v2) and N100; [`results/timing-sweep-1106e-table.md`](../results/timing-sweep-1106e-table.md)):

| Rows | Ivy Bridge faster / parity / slower, median | N100 faster / parity / slower, median |
|---|---|---|
| all 407 judged rows | 252 / 99 / 56 | 313 / 74 / 20 |
| string and memory (`cbench_memref`, 117) | 62 / 33 / 22, 0.94 | 89 / 24 / 4, 0.73 |
| locale (`cbench_locale`, 33) | 19 / 9 / 5, 0.88 | 24 / 4 / 5, 0.80 |
| hot paths (`cbench_hot`, 58) | 32 / 17 / 9, 0.92 | 49 / 9 / 0, 0.67 |

The first sweep on an AMD Zen 3 (Ryzen 5 5500 class, `next` at `66b8d3ba`, 2026-11-07;
[`results/timing-sweep-zen3-1107-table.md`](../results/timing-sweep-zen3-1107-table.md)) judged 376
non-`libm` rows: 274 faster, 64 parity, 38 slower, median 0.75.

The work behind them: AVX2 and SSE2 kernels and short-length entries for `memcmp`, `__memcmpeq`,
`strcmp`, `strncmp`, `strcasecmp`, `memset`, `memchr`, `rawmemchr`, `strstr` and the copy
functions; `nl_langinfo` answered from a table per locale view; faster `wctype` lookups; lower
first-call cost for `setlocale`, `strcoll`, the time-zone load and `bindtextdomain`; directory walks
(`nftw`, `readdir`) and `glob` sorting. The per-row tables are in
[`docs/benchmarks.md`](benchmarks.md) and `results/`.

The workload count of real programs (the 2026-10-24 figure below) was not repeated on the measured
tree.

### Changes users may notice

- **`libnsl.so.1`** is built now, as a placeholder whose NIS/YP and NIS+ functions fail (121
  compatibility-only names, so only programs already linked against it bind them; D-29.6).
- **The ONC RPC client, port-mapper calls and server** are failing stubs; 17 of these names that
  were not exported before are exported now, at their compatibility versions (D-29.5).
- **Static C++ programs** that throw an exception without using threads run.
- **stdio**: a read function entered with the stream's error indicator already set leaves `errno`
  unchanged.

Deliberate differences from glibc are listed with their reasons in
[`docs/differences-from-glibc.md`](differences-from-glibc.md).

### For contributors: tests and CI

Every change must now prove its tests with plants (deliberately broken copies of the changed code)
run by `cargo xtask plants` ([`docs/plants.md`](plants.md)); `cargo xtask affected` selects the
tests a change can reach; the timing sweep tool (`xtask sweep`, [`docs/benchmarks.md`](benchmarks.md)
section 6) times every benchmark row against glibc with interleaved rounds on idle hosts.

### What does not work yet

- **Dynamic linking (phase 14)**: dynamically linked programs do not run (above).
- **Legacy tier and binary compatibility (phase 15)**: running unmodified glibc-linked binaries has
  not been demonstrated; 159 compatibility-only symbol versions and two legacy default symbols are
  not defined
  ([`plan/06-phases/18-phase-15-legacy-tier-and.md`](../plan/06-phases/18-phase-15-legacy-tier-and.md)).
- **Rust `std` programs** are not supported yet; Rust programs are `#![no_std]`
  ([`docs/toolchain.md`](toolchain.md) section 8).
- **Security audit**: the library has not had one; it is planned after phase 15.
- **Platforms**: Linux x86-64 only.

### Processors

The measured tree's status and symbol figures come from an AMD EPYC 7551 (Zen 1) with Linux 6.8.
Timing figures come from Intel Xeon E3 v2 (Ivy Bridge), Intel N100 and AMD Zen 3 hosts, as named
with each table. Tests run on the processor classes listed in
[`CONTRIBUTING.md`](../CONTRIBUTING.md) ("Tested processors").

## Previous release: soft release, 2026-10-24

This is a soft release: the current tree, published in a fresh repository. It is a public work
in progress, not an alpha or a release candidate, and it should not be used in production.

Released 2026-10-24 as the first commit of this repository, cut from the development tree; that
commit's message names the development commit it was cut from. It had no tag when it was published;
on 2026-11-07 it was tagged `snapshot-2026-10-24`. Every
figure below says where it comes from and when it was measured.

### What Liella is

Liella is a clean-room implementation of the GNU C library ABI, glibc 2.39, for x86-64 Linux,
written in Rust. It aims to export glibc's symbols with glibc's calling conventions, data
layouts and symbol versions, so that programs built against glibc's headers can use it. The
contract is [`glibc-functional-spec.md`](../glibc-functional-spec.md); the ABI data is in
`spec/abi/`; the design, phases and decision register are in
[`implementation-plan.md`](../implementation-plan.md) and `plan/`.

Licence: LGPL-2.1-or-later, the same as glibc ([`LICENSE`](../LICENSE)).

The clean-room record, including where it fell short in the early phases and what was
rewritten, is in [`docs/clean-room.md`](clean-room.md) and the README's "Clean-room record".

### What works

Phases 1 to 13 of the plan are implemented: the kernel boundary, strings, locale, math, stdio,
the file system, sockets, name services, processes, signals, threads, time and IPC. Programs
are linked **statically** against Liella's `libc.a` (`cargo xtask so` builds `libc.a`,
`libc.so.6` and `liella-crt1.o` into `target/liella/`). See [`README.md`](../README.md) and
[`docs/interfaces.md`](interfaces.md).

**Tests** (`cargo xtask status` on the release tree, 2026-10-24, AMD Zen 1): 5,020 tests are
specified and 591 of them are still pending (stubs for behaviour not yet implemented). Most of
the pending tests belong to phases 14 and 15 (below).

**Symbols** (`cargo xtask so && cargo xtask symcov` on the release tree, 2026-10-24, AMD Zen 1;
the per-name list is [`docs/spec-symbol-gaps.md`](spec-symbol-gaps.md)): the glibc 2.39 ABI table
(`spec/abi/versions.tsv`) has 4,305 names (4,600 version rows), and 716 of those names are not
defined by any Liella product yet:

| Class | Names | Planned in |
|---|---:|---|
| dynamic linking / `ld.so`, `libthread_db`, `libc_malloc_debug`, placeholder libraries | 113 | Phase 14 |
| compatibility-only versions in libraries Liella ships (`libc.so.6` 128, `libm.so.6` 37, `libresolv.so.2` 11) | 176 | Phase 15 |
| `libnsl.so.1`, which Liella does not replace yet | 127 | Phase 15 |
| legacy tier, default version (`__libc_sa_len`, `bindresvport`) | 2 | Phase 15 |
| `libm`: the complex `_Float128` functions | 18 | Track M milestone 5 |
| `GLIBC_PRIVATE` (internal to glibc's own libraries) | 280 | out of scope |

So "phases 1 to 13 implemented" has one stated exception: the 18 complex `_Float128` names of
`libm` milestone 5. Their `float`, `double`, `long double` and real `_Float128` counterparts, and
the other complex functions, are implemented.

### What does not work yet

- **Dynamic linking (phase 14): not done.** There is no working `ld.so` yet. No dynamically
  linked program has been run against Liella's `libc.so.6`; such programs are linked and
  checked, not executed. `LD_PRELOAD`, `LD_LIBRARY_PATH`, `LD_AUDIT`, `dlopen` and
  `libthread_db` are not available. The phase has a story list
  ([`plan/06-phases/17-phase-14-dynamic-linking.md`](../plan/06-phases/17-phase-14-dynamic-linking.md));
  by the plan's own record, its design (`plan/ph14/design.md`) is proposed with no open
  question ruled yet, and `crates/ld` and `crates/thread-db` are skeletons.
- **Legacy tier and binary compatibility (phase 15): not done.** Running unmodified
  glibc-linked binaries is the project's aim and has not been demonstrated. Not done yet: the
  NIS client library `libnsl.so.1` (§29.2), 176 compatibility-only symbol versions and two
  legacy default symbols (the symbol table above). The binary-compatibility corpus does not
  exist yet
  ([`plan/06-phases/18-phase-15-legacy-tier-and.md`](../plan/06-phases/18-phase-15-legacy-tier-and.md)).
- **`libm` milestone 5**: the 18 complex `_Float128` names above.
- **Rust `std` programs** are not supported yet; Rust programs are `#![no_std]`
  ([`docs/toolchain.md`](toolchain.md) section 8).
- **Security audit**: the library has not had one. A security pass is planned after phase 15
  ([`plan/06-phases/19-security-pass-after-phase-15.md`](../plan/06-phases/19-security-pass-after-phase-15.md)).
- **Platforms**: Linux x86-64 only. Building on Windows is not supported (use WSL or Linux).

### How it was validated

- **Differential tests against glibc.** Liella's behaviour is checked by black-box comparison
  with an installed glibc 2.39: oracle programs run on the host glibc and their results are
  compared with Liella's. The specified tests and their state are counted by
  `cargo xtask status` (above). Every change lands only after the full gate passes in CI. The
  landing gate on the release tree (2026-10-24, Intel Xeon Scalable 1st generation, Linux 6.8)
  ran 11,236 tests with 0 failures and 1,198 ignored, and 114 of 114
  conformance programs.
- **Real programs.** A workload set of real programs (coreutils, gawk, sed, grep, tar, gzip,
  ...) is built statically against Liella, and its output is compared with the same programs
  on glibc ([`validation/README.md`](../validation/README.md)). The workload gate
  (`validation/wl_gate.sh`) fails on deterministic counters (user instructions, page faults,
  system calls), never on wall time. **Preliminary** (2026-10-24, the release tree, AMD Zen 1):
  each of the 30 workloads ran no more user-space instructions as a Liella static build than
  with the host's glibc 2.39; the geometric mean ratio was 0.821. Outputs match glibc's except
  one documented difference (`ls -l` in `en_US.UTF-8`, POSIX punctuation collation, D-4.42).
  This is one host and one counter, not a release benchmark.
- **Fuzzing.** 20 libFuzzer targets cover the parsers of untrusted input (name-service files,
  DNS messages, regex, printf/scanf formats, locale and charmap sources, iconv, glob, wordexp
  and others; [`docs/fuzzing.md`](fuzzing.md)). The latest campaign (2026-10-23, AMD Zen 1)
  ran every target for up to 30 minutes, about 823 million executions in all: 16 targets ran
  the full time with no finding; `regex`, `regex_dfa`, `glob` and `locale` stopped early on
  slow inputs (time-outs), which are open work.
- **Plants (mutation checks).** Changes are checked by planting deliberate faults in the
  changed code and confirming the direct tests catch them, before the change lands. The tree
  does not keep a running total.
- **Kernels and CPUs.** Linux 6.8 on every test host, on the processor classes listed in
  [`CONTRIBUTING.md`](../CONTRIBUTING.md) ("Tested processors"): Intel Xeon Scalable 1st
  generation (AVX-512), AMD Zen 1, Intel Haswell-EP and Broadwell-EP (AVX2), Intel Ivy Bridge
  (AVX without AVX2) and Intel N100.

### Deliberate differences from glibc and compatibility flags

Where Liella deliberately behaves differently from glibc, the difference and the reason for it
(correctness, performance, memory or security) are listed in
[`docs/differences-from-glibc.md`](differences-from-glibc.md), by area (behaviour,
mathematics, character sets).

Behaviour that a user could notice is listed first, in the section
[Deliberate user-facing incompatibilities](differences-from-glibc.md#deliberate-user-facing-incompatibilities).

[`docs/compat-flags.md`](compat-flags.md) registers the differences that a program might
depend on, and how a flag would restore glibc's behaviour (COMPAT-1 to COMPAT-12). Most are
registered designs, not implemented flags; COMPAT-12 (UDP RPC replies from another address) is
implemented as the tunable `liella.compat.rpc_reply_any_source`. COMPAT-3 (the
binding of WEAK exports) is resolved for `libc.so.6` and still registered for `libc.a`.

### Reporting a security issue

Please report privately, not in a public issue or pull request. Use GitHub private
vulnerability reporting on this repository. See [`SECURITY.md`](../SECURITY.md) for what to
include and what to expect.

### Contributing

See [`CONTRIBUTING.md`](../CONTRIBUTING.md), the code map in
[`docs/codemap/README.md`](codemap/README.md), [`AI_POLICY.md`](../AI_POLICY.md) (Liella was
written with Claude Code under the direction of a human maintainer) and
[`CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md). Contributions are accepted under
LGPL-2.1-or-later. Please do not paste glibc source into a security report (`SECURITY.md`).

### History

The project's full development history is kept in a separate historical repository.
References of the form `liella-historic:<path>` in this tree point into it.
