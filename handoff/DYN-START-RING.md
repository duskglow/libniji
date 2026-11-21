# DYN-START-RING: cut the ring transitions of a dynamic start

State: branch `task/dyn-start-ring` from origin/next 0122a0d8a; role keke:~/w/n1 (Zen 1). Leg 1 ended with a
REFRESH: code is in, **one regression to explain before anything else** (below). Not ready to land.

## Commits (leg 1)

1. 771ed7154 `Install the internal signal's handler at the first thread creation` (owner ruling #7075): cancel.rs
   `install` is once-per-process (INSTALLED flag, cold `install_now`), called from thread.rs `create_inner` before the
   clone under `cfg(feature = "runtime")`; both runtime.rs start-up calls removed. conformance/c/s20_internal_signal.c:
   default action at start, handler after the first pthread_create, every later check unchanged.
2. fbe54cca4 `Take the loader's NULL-folding decision without reading mmap_min_addr`: dispatch.rs
   `resolve_fold_slow`: `cfg!(all(feature = "ld", not(feature = "export"))) || page_zero_unmappable(..)`. Reason:
   ld.so's dynamic symbol table has no copy/fill entries (checked: `readelf --dyn-syms` 0 matches), so only its own
   Rust code calls them, which never passes NULL with n > 0. Not the brief's "inline fixed-size stores" design; same
   folded paths as before, zero code growth. Recorded in records/pending/DYN-START-RING-2.toml (amends D-4.22).
3. 6f15029fe + amended fix `Map the start's object views from one arena and keep reservation slack`: image.rs view
   arena (`views_arm` in link, `views_take` reserves 1 GiB PROT_NONE|NORESERVE at the first view, 64 KiB slots,
   `unmap_view` no-op inside the open arena, `views_close` one munmap after link's unmap loop); `StartMapper`
   (keeps_slack true) for link's dependencies, exec_stack_dependency and `ld.so PROGRAM` load; map.rs
   `Mapper::keeps_slack` (default false) skips the two trims in `reserve`.
4. Uncommitted at refresh time -> committed as WIP: hot_text_products.rs (`start_calls` helper, rt_sigaction added
   to the removable-calls test, new `init_dynamic_start_system_calls_stay_within_their_bounds` with PLACEHOLDER
   bounds 1000: set them from the measured counts, `--nocapture` prints them), records/pending/DYN-START-RING-1..3,
   differences row (reserved signals: lazy handler sentence), handoff/DYN-START-RING-plants.txt (5 plants).

## Measurements (Zen 1, keke; ~/work/dyn-start-ring/rp.sh <name>: dsq-probe kit + static s0; median of 101, ASLR off;
equal-length product dirs base0 / aft01; logs ~/logs/dyn-start-ring-base0.log, ~/logs/dyn-start-ring-so2.log)

| row | base0 user / kernel / faults / syscalls (strace incl. execve) | aft01 | glibc 2.39 |
|---|---|---|---|
| n0 | 763,452 / 822,950 / 59 / 36 | 766,914 / 834,537 / 58 / 29 | 100,272 / 694,966 / 48 / 42 |
| n3 | 1,027,009 / 1,260,051 / 75 / 78 | 1,030,231 / 1,249,555 / 74 / 59 | 135,461 / 992,759 / 64 / 68 |
| nt | 798,566 / 852,228 / 63 / 36 | 802,028 / 863,841 / 62 / 29 | 102,411 / 703,640 / 50 / 42 |
| s0 (static) | 2,076 / 149,804 / 7 / 10 | 2,057 / 149,144 / 7 / 9 | gs0 132,974 / 288,868 / 24 / 15 |

aft01 n0 calls: mmap:12 openat:4 mprotect:2 arch_prctl:2 uname:1 set_tid_address:1 set_robust_list:1 rseq:1 munmap:1
fstat:1 close:1 exit_group:1 execve:1. **RATCHET RED: user +3.2..3.5k on every dynamic row, kernel +11.6k on n0/nt**
(n3 kernel -10.5k). Suspect: the 1 GiB arena (the hole it leaves after link moves every later mapping: TLS, thread
block, heap; new page-table pages; and something address-dependent in user code). Not yet proven.

## Exact next step

1. A/B on keke (one `cargo xtask so` each, ~20 min; copy products to a 5-char dir, `sh ~/work/dyn-start-ring/rp.sh
   <dir>`): (a) arena off (`views_arm();` removed), (b) arena on but VIEWS_LEN sized small (e.g. 64 MiB), to split the
   user and kernel deltas between the arena, the slack, the fold and the lazy signal. Profile before redesigning.
2. Likely fix if the hole is the cause: per-view reservation `mapped + VIEW_ALIGN` with the file at the aligned
   address and NO trims, unmapped with one munmap of the whole reservation at the end (3 calls per object instead
   of 5, no hole), or an arena sized from the files (fstat sizes) placed so nothing lands in its hole.
3. Then: set the three bounds in hot_text_products.rs from `--nocapture` output; clippy (all-features) + static;
   gate (`cargo xtask affected --run`), plants (`cargo xtask plants handoff/DYN-START-RING-plants.txt --dir
   /dev/shm/claude/dyn-start-ring/plants-1 --timeout 2400`), `cargo xtask test`, full-profile hot_text_products,
   Skylake-SP rows (mei), precheck, report json.

Not done (brief items left): uname, the early ARCH_SET_FS, the initial thread block/TLS merge, read_phdr per-header
user cost (60k on empty main), whole-file views replaced by header reads (needs a header-only parse mode; refusal
order in tools/probe/dl/gaps must be kept). Jobs so far: dyn-start-ring-so0 (base build), -base0 (base probe),
-so1 (red: unused import, superseded), -so2 (aft01 build + probe + hot_text_products init_dynamic_ 5 passed).
