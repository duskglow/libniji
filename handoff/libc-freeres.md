# LIBC-FREERES handoff (LIBC-FREERES-2: final)

Branch `task/libc-freeres-2` (continues `task/libc-freeres`), code head b811f0c0d; later commits are handoff/ and one
docs row. Role mei:~/w/s3. Scratch: local `~/w/.scratch/LIBC-FREERES{,-2}/`, mei `~/work/LIBC-FREERES/`.

## State: done

* `__libc_freeres` (crates/boundary/src/posix/s03_memory.rs) releases, in order: every stream's library-allocated
  buffer (`stdio::release_buffers` -> `stdio_compat::release_buffers`, cold + inline(never): pending output flushed
  first, kept on a write error; a reading stream with nothing read ahead goes idle and releases; read-ahead or pushback
  keeps the buffer), then the calling thread's getpw*/netdb table, resolver state and dlerror record
  (`users/resolv/dlerr::thread_ending`). Each part is left as before first use; a second call releases nothing.
* Records: D-29.7 (active; replaces ruling #1317's inert design), COMPAT-11 withdrawn, differences row rewritten (the
  mid-run caveat for getpw*/netdb results added in LIBC-FREERES-2, as D-29.7 already said).
* Tests: `s29_4_c_1` (release, re-allocation, kept read-ahead, EOF, ungetc), `s29_4_c_1_counted` (per-thread count of
  library blocks from the C malloc family is zero after the call); the two share a lock (a freeres call on one test
  thread releases another thread's stream buffer, whose per-thread count then never sees it: one baseline flake, fixed
  in b811f0c0d, cause understood).
* Plants: handoff/LIBC-FREERES-plants.txt, 5 of 5 CAUGHT (job libc-freeres-plants-3, content = b811f0c0d).

## Memcheck table (valgrind 3.24, trixie root, `--show-leak-kinds=all`; reachable bytes/blocks, errors, lost)

| program | stock | libniji before (base 0495c4684) | libniji after |
|---|---|---|---|
| /bin/true | 0 B / 0 | 0 B / 0 | 0 B / 0 |
| puts | 0 B / 0 | 4,104 B / 1 | 0 B / 0 |
| cxx (iostream + throw) | 0 B / 0 | 4,104 B / 1 | 0 B / 0 |
| mix (setlocale, iconv, getaddrinfo, dlopen, localtime, getpwuid) | 0 B / 0 | 5,080 B / 3 | 0 B / 0 |

Errors 0 and lost 0 in every cell. libniji columns run with libniji's libc6-dbg installed; without it valgrind stops
at start-up (no `strcmp` in ld.so's symbols) before and after alike: known, tracked by the VALGRIND-LDSO-SYMBOLS line.
Jobs: libc-freeres-vg-before, libc-freeres-vg-after (1578c120b), libc-freeres-2-vg-after-2 (final code head; the same table
as libc-freeres-2-vg-after, which left no tested-content record because its command named no `w/<role>`).

## Gate and start-up checks

* mei gates and `cargo xtask test` fail 6 locale tests (catalogs_products, locale_data x2, locale_products,
  printf_corpus_products, s7_2_f_0) since locales-all replaced mei's locale archive (incident #7846, fix briefed as
  LOCALE-PER-DIR-LOAD). Evidence they are not this change: the reference's own `setlocale(6, de_DE.UTF-8@euro)` fails
  in them, and none touches stdio buffers or `__libc_freeres`. Every other test passed.
* `NIJI_FULL_PROFILE=1` hot_text_products: 33 passed (libc-freeres-hottext). No new function is in a layout bin
  (`target/product/fast/layout-bins.tsv`): the new code is cold and outside the hot group; hot functions unchanged.

## Open

* RULING NEEDED: reversal of ruling #1317 (inert `__libc_freeres`), recommend accept: memory checkers call it only at
  exit; a mid-run call keeps every stream's data; only the caller's earlier getpw*/netdb results become invalid.

## Not to redo
The survey: libc-visible allocations are those through `caller_alloc` (the program's malloc); the plan caches use the
internal allocator (`TryBox`, not seen by valgrind). Valgrind recipe: `~/work/LIBC-FREERES/vgfree.sh <role> <tag>`.

## Next step
None for this story beyond landing (after #7846's fix lets mei gates pass the 6 locale tests).
