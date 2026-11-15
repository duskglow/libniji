# UTIL-LDCONFIG (UTIL-3, SL-LDCONFIG): ldconfig

**State.** Branch `task/util-ldconfig-6` (run 6; stacked on run 5's `task/util-ldconfig-5`), role mei `~/w/s2`.
Head: see `handoff/reports/UTIL-LDCONFIG-6.json`. Engine, shipped program, packaging and `-l` exist. NOT done: the
package rebuild and checks (blocked, see Open questions 1), the end-to-end loader differential, plan §5 counters.

## Done

* Runs 1-3: engine `crates/ld/src/ldconfig/{mod,conf,scan}.rs`, hosted front end `xtask/src/transcript/ldconfig.rs`,
  transcripts, `crates/ld/tests/ldconfig.rs`, plants `handoff/UTIL-LDCONFIG-3-plants.txt`.
* Run 4: the shipped static-PIE `target/liella/ldconfig` (`crates/ldconfig-bin`, boundary `src/ldconfig.rs`, entry.S
  `-DLIELLA_LDCONFIG`, `ldconfig/sysfs.rs` with `openat2(RESOLVE_IN_ROOT)`); libc-bin ships `/usr/sbin/ldconfig` and
  runs it on the trigger; D-31.11, D-31.12.
* Run 5: merge of origin/next; ELF32 `elf:` items in the transcript harness; foreign-class probe
  (`foreign-class.transcript`); i386/x32 objects cached with flags 0x3/0x803; mixed-type same-soname rule
  (pending record UTIL-LDCONFIG-5-1, accepted by ruling #5932: deterministic in every listing order).
* Run 6:
  * Run 5's surviving plant `elf32-phdr-offset` was a test gap, not an equivalent mutant: the test ELF images
    placed every segment at `p_vaddr == p_offset`. Both builders in `crates/ld/tests/ldconfig.rs` now link at a
    bias (ELF-32 0x8000, ELF-64 0x200000; `DT_STRTAB` a virtual address). Plant run util-ldconfig-6-plants: 8/8
    caught, including elf32-phdr-offset and a new elf64-phdr-offset.
  * `-l` (#5916) implemented (`Run::libs` in scan.rs), every behaviour probed against the reference and committed
    as transcripts `validation/transcripts/ldconfig/l-{links,bad,same-soname,root-cache,edge,x}.transcript`:
    operands in argument order, link `<operand dir>/<soname>` -> file name, the later operand of one soname wins,
    no directory scanned and no cache written (with or without `-N`, under `-r` too: operand taken inside the
    root), `-X` reports without linking, `-v` lines without a directory header, the dynamic linker's soname linked
    silently; messages `Cannot lstat <op>: <err>`, `Ignored file <op> since it is not a regular file.`,
    `file <op> is truncated` + empty line, `No link created since soname could not be found for <op>`; exit 0.
    `--help` line now matches the reference.
  * Probe `n-not-link.transcript`: a regular file at a soname path prints `<path> is not a symbolic link` and an
    empty line (the engine printed `..., not replaced`); fixed for both modes (shared `link_at`).
  * `tools/deb/root-system.sh`: new PASS line, the root's own `ldconfig -p` lists libc.so.6 from the cache the
    trigger wrote. NOT VERIFIED: no packages could be built (Open questions 1).
  * Plants `handoff/UTIL-LDCONFIG-6-plants.txt` (13: run 5's 7, elf64-phdr-offset, five on `-l` and the message):
    util-ldconfig-6-plants2 on code head 3693c5a88, 13/13 CAUGHT, no WARN. Lint/tests job util-ldconfig-6-check1:
    clippy, static, 9 engine tests, 20/20 ldconfig transcripts ok against the reference. Gate util-ldconfig-6-gate2
    (escalated to FULL by build-config paths) left running at hand-back.

## Open questions

1. Package build blocked outside this story: `tools/deb/build.sh --flavour system` stops in `tools/deb/symbols.py`
   with `no minimum version rule for node LIELLA_OWNER_1` (job util-ldconfig-6-pkg, rc 1). The node came from
   DL-NONSHARED-OPTOUT on origin/next; symbols.py on next has no rule for it; this branch changes neither.
   Needs a packaging story to choose the dependency for LIELLA_OWNER_<n> (recommended: like LIELLA_NATIVE nodes,
   an alternative only Liella's libc6 satisfies, since no other libc provides the node).
2. After packages build again: does purge leave `/etc/ld.so.cache` behind (the trigger now writes it; root-system's
   purge check allows only nsswitch.conf)? If so, libc-bin needs a postrm that removes the cache on purge.
3. Not probed: `-l` with `-p`, a foreign-machine (non-x86) ELF operand (treated as "no soname"), a relative
   operand that exists. Mixed-type message without `-v`. Arena peak RSS on a full /usr/lib.

## Next steps (run 7), in order

1. Read util-ldconfig-6-gate2 (FULL by escalation) and util-ldconfig-6-plants2; fix any red.
2. When symbols.py handles LIELLA_OWNER_1: run `~/work/UTIL-LDCONFIG-6/pkg.sh <commit> <epoch> <RAM dir>` (on mei;
   build.sh --flavour system with LIELLA_DEB_COMMIT/SOURCE_DATE_EPOCH, check-system.sh, root-system.sh); answer Open
   question 2.
3. The end-to-end loader differential over a cache the binary wrote; plan §5 counters.

Read: this note, plan/utilities.md §2.3/§4/§5, spec §31.2 and D.3, docs/transcripts.md, D-31.2/D-31.9/D-31.12.
