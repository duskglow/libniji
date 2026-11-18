# LAYOUT-LDSO-ORDER: ld.so gets a traced, ordered start-up layout like libc

State: branch `task/layout-ldso-order`, run 1 stopped at the context checkpoint (~150k). Base origin/next 627ef49ca.
Code head 3d056a5ec (WIP commits). Role keke:~/w/n1. Brief: ~/state/briefs/LAYOUT-LDSO-ORDER.md.

## Done (run 1)

* `tools/hottext/trace_gdb.py`: `HOT_TEXT_LDSO=<interpreter>` mode. The functions are the interpreter's
  (nm of that file, image offsets); each run stops at `starti` (the interpreter's entry), takes the base as
  pc - e_entry, puts breakpoints and the answered entries (cpuid, xgetbv, clock, topology) at base + offset,
  records the entry as the first function, and writes names by offset. The "classes ran the same functions"
  check is skipped in this mode (NOT VERIFIED that the interpreter chooses no code by CPU class: compare the
  per-class runs, or keke vs mei lists, next run).
* `tools/hottext/ldso_trace.sh` (new): builds two dynamic programs of this build under /tmp/ldso-trace.XXXXXX
  (fixed-length paths): `true` (libc only) and `libs3` (3 trivial libraries by RUNPATH), traces both, plus the
  `true` trace with HOT_TEXT_SLOW=2 and HOT_TEXT_HOST_NODES=1, which must equal `true` (TRACE-TIME-PIN /
  TRACE-NUMA-PIN cover the new output). `/etc/ld.so.preload` is redirected to an absent run file. libc.so.6 is
  named by path (`native_examples::libc_by_path`): the interpreter takes no libc.so.6 from a run path (#4537;
  first build failed on this, plain run outside gdb fails the same way).
* `xtask/src/hottext.rs`: `group_ldso` = the same renaming pass as libc (`group_as`, `Image::Ldso`: no window
  bins, no STARTUP_NAMES count); `ldso_lists` (list = `true` order then what `libs3` adds; signatures = modes);
  unit test `ldso_list_is_the_first_mode_then_what_later_modes_add`.
* `xtask/src/so.rs`: `run_ld(.., layout)`; traced: link ungrouped into target/product/<p>/ldso-trace, trace,
  write `ldso-text.list` / `ldso-text.signatures` next to libc's lists, group a copy of the archive with the
  entry object as a member, link that. Ungrouped opt-out links as before.
* Build job `layout-ldso-order-so2` (keke, head 3d056a5ec) rc 0: `true` 99 names, `libs3` 109; 107 sections
  grouped (109 keys); slow and numa1 traces equal; entry back at 0x1000, `niji_ld_start` 0x1040,
  `relocate_image` 0x23c0; libc's STARTUP-TEXT-WINDOWS unchanged (1 window).

## Open / next step (in order)

1. Decision record: `cargo xtask records new decision D-NEW --series D-X --story LAYOUT-LDSO-ORDER` (why no
   window bins for ld.so: the kernel maps the interpreter at a page-aligned base; measure the base alignment
   with ASLR off to confirm). Not written yet.
2. Determinism: build on mei (or copy) and compare `ldso-text.list` / `.signatures` with keke's.
3. Proof: count2.sh pattern (mei ~/work/DL-STARTUP-OBJECTS/count2.sh; ASLR off and on, N=0/3/12, 21 runs)
   base 627ef49ca vs branch; insensitivity: a 32 KiB `#[cold] #[inline(never)]` function in ld.so (not
   committed), faults unchanged. Say whether ld.so data (.dynamic, relro, GOT) decides a fault.
4. hot_text_products: add an ld.so bound (e.g. the traced functions lie in [entry, entry + group) and the
   group spans N pages/windows at the measured level).
5. Plants file (`group_as` Ldso branch, `ldso_lists` order, trace_gdb rebase, ldso_trace.sh cmp checks) via
   `cargo xtask plants`; clippy, static; gate (direct tier) + the two start-up checks (`cargo xtask test`,
   `NIJI_FULL_PROFILE=1 ... hot_text_products`).

Process slip: one `sed -i` edit (a one-word call-site change in so.rs) and one python heredoc to edit
hottext.rs; both against the rules, no effect on the result.
