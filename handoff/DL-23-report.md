# DL-23 report (run DL-23-2)

Branch task/dl-23-2 (from task/dl-23 377ca800c; base origin/next fb63eeec8, not merged: LAYOUT-DYN-PHASE-3
not landed, DYN_TRUE_BOUND unchanged at 89/86). Role mei:~/w/s1 (Skylake-SP host).

## Commits (DL-23-2)
- d4f139abf Place ld.so's link-map nodes first in its .lbss (xtask `move_cold_bss_first`, `LDSO_LBSS_FIRST`)
- 51ab958ef Register COMPAT-35 for the program's live copy of _r_debug
- 7e05fed9d WIP: Test link-map names of path-named dependencies and an unnamed ld.so
- 01d982707 Look ld.so's symbols up in its handle when the program does not name it (NOLD test fix; D-27.62 tests)
- e0271d963 Fix the D-27.62 tests list and date COMPAT-35 (its body also says it adds the first_order
  plant; that plant went in with 01d982707: wording error, pushed, not rewritten)
- 528936530 Keep the first thread-local module entries on the start's .lbss page
- d38498d03 Read thread-local module entries through the accessor in dl_iterate_phdr
- then the handoff/report commit(s).

## Coordinator rulings, item by item
1. #6735 live COPY: COMPAT-35 registered only (row and D-27.62 name it; why registered-only is in the record).
2. #6736 faults. Two pages found (pagemap at `main` + exact counter, leg 1): the link-map nodes' page and the
   vDSO header page. The nodes now lie first in `.lbss` on the page `.bss` already shares. That moved
   THREAD_MODS (thread-local module table) 90 KB on, so a start with a PT_TLS object faulted a new page
   (measured: +2 on TLS rows). Fix: the first 16 module entries sit in front of the nodes in the same static
   (`dlopen::NodeSpace`), reached only through `ld::thread_mod`. Final head d38498d03, exact counts
   (execcount, 301 runs, ASLR off, spread <= 30 instr, faults spread 0), base = fb63eeec8 rebuilt:

   | row | base instr | branch instr | base faults | branch faults |
   |---|---|---|---|---|
   | pie0 (PIE, no libraries) | 8959154 | 8964161 | 79 | 80 |
   | pie3 (3 DT_NEEDED libraries) | 9189085 | 9196948 | 95 | 96 |
   | tlsp (program PT_TLS) | 8962224 | 8967260 | 79 | 80 |
   | tlsl (library PT_TLS, initial-exec) | 9053749 | 9060020 | 85 | 86 |
   | clk (one clock_gettime call) | 9003181 | 9008592 | 83 | 83 |
   | copy0 (-no-pie, COPY of _r_debug; branch only) | - | 9029614 | - | 84 |

   The remaining +1 is the vDSO header page, read to fill the vDSO link map's l_addr/l_ld (every source of
   the vDSO's PT_DYNAMIC is in that page). libc reads the same page on its first vDSO call
   (`sys.rs` `resolve_vdso`), so a program that calls clock_gettime/time/gettimeofday/getcpu pays it
   anyway: clk row 83/83. Rejected: a lazy fill (l_ld 0 for walkers and debuggers until filled), a syscall
   read (hides the fault from the counter, costs more). -> RULING NEEDED.
   hot_text_products (NIJI_FULL_PROFILE, job dl-23-2-hot1, a RAM copy of the synced d38498d03 tree): 28
   passed, 0 failed (DYN_TRUE_BOUND untouched).
3. #6737 r_version: man-pages (both hosts) do not document r_debug/r_version; the glibc manual is not
   installed on the hosts; documentation silent -> kept 1 (probe), RULING NEEDED (clause s27_4_c_0 says 2).
4. RUNPATH, dlopen(ld.so path, RTLD_NOLOAD): out of scope, untouched.
5. Plants: survivors now covered: check 13 (a dependency named by path in a library's DT_NEEDED has that path
   as l_name) and `rendezvous_lists_ld_so_the_program_does_not_name` (ld.so's l_addr/l_ld, check 8).

## New finding (RULING NEEDED)
A program that does not name ld.so on its link line (the usual case) has ld.so outside its global scope
under libniji: libniji's libc.so.6 has no DT_NEEDED on ld-linux-x86-64.so.2 (the reference's has one), so
`dlsym(RTLD_DEFAULT, "_r_debug")` / `"_dl_debug_state"` return NULL there. Found by the NOLD test; the test
now looks those symbols up in ld.so's handle. Recommend: a loader story with #6739/#6740 (ld.so in the global
scope, by libc's DT_NEEDED or by the start), not this branch.

## Tests and jobs (mei)
- dl-23-2-test2: clippy --all-features -D warnings, cargo xtask static, xtask so, cargo test -p niji-ld,
  -p niji-boundary --test dl_tls_threads, -p xtask: 35 suites, 719 passed, 0 failed.
- dl-23-2-count5: the table above. dl-23-2-startup1: cargo xtask test + full-profile hot_text_products.
- dl-23-2-plants3: handoff/DL-23-plants.txt, 12 plants; 5 CAUGHT, none survived when the leg ended
  (still running). dl-23-2-startup1 (`cargo xtask test`, 6842 tests, then the full-profile check
  again) still running. Both run from the role, so the role cannot be synced to the handoff-only head
  until they end: then sync and rerun precheck (its only red was the head, d38498d03 vs the final head).
- dl-23-2-precheck1 (head 41f1f854b): every check ok except head (role at d38498d03) and the WAITs.
- Superseded: dl-23-2-rdv1 (NOLD test red, fixed in 01d982707), rdv2/rdv3/rdv4 (diagnosis, older head),
  lint1 (records red, fixed), static2, lint2, pm2/tls1/tls2 (older heads), test1 (dlclose check 26 red,
  fixed in d38498d03), plants1/plants2 (stopped: older heads), pm3 (empty log: the base directory was
  removed mid-job), count3/count4 (counter or base path missing).

## INFORMATIONAL
- /mnt/zram/claude/dl-23 (DL-23's base products and counter) was removed around 14:15 by someone else
  during job dl-23-2-pm3; base rebuilt from ~/wl/src/fb63eeec8 into /mnt/zram/claude/dl-23-2/base-niji.
- sync_role returns BUSY while a plants job runs from the role (its cwd); plants and syncs cannot overlap.
- Process slips this run: one empty no-op heredoc command, one `sed -i` on a scratch copy of a script
  (not in the tree); the e0271d963 body wording above.
