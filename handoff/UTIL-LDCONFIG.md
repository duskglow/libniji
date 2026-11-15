# UTIL-LDCONFIG (UTIL-3, SL-LDCONFIG): ldconfig

**State.** Branch `task/util-ldconfig-5` (run 5; stacked on run 4's `task/util-ldconfig-4`), role mei `~/w/s1`.
Head: see `handoff/reports/UTIL-LDCONFIG-5.json`. Engine, shipped program and packaging exist (runs 1-4). Run 5
merged origin/next (CI conflict) and did the foreign-class probe (#5922) and its engine change. NOT done: `-l`
(#5916), the package rebuild and checks on the merged head, root-system.sh, the end-to-end loader differential,
plan §5 counters.

## Done

* Runs 1-3: engine `crates/ld/src/ldconfig/{mod,conf,scan}.rs`, hosted front end `xtask/src/transcript/ldconfig.rs`,
  transcripts, `crates/ld/tests/ldconfig.rs`, plants `handoff/UTIL-LDCONFIG-3-plants.txt`.
* Run 4: the shipped static-PIE `target/liella/ldconfig` (`crates/ldconfig-bin`, boundary `src/ldconfig.rs`, entry.S
  `-DLIELLA_LDCONFIG`, `ldconfig/sysfs.rs` with `openat2(RESOLVE_IN_ROOT)`), libc-bin ships `/usr/sbin/ldconfig` and
  runs it on the trigger; records D-31.11, D-31.12. Gate util-ldconfig-4-gate (FULL, 32 suites ok) and plants
  util-ldconfig-4-plants2 (3/3 caught), both rc 0.
* Run 5:
  * Merge of origin/next: `tools/deb/check-system.sh` conflict resolved by keeping both sides (libc-bin
    `/usr/sbin/ldconfig`; SL-CRT's `crt[in].o` for libc6-dev). Lint on the merge green (util-ldconfig-5-lint1).
  * Transcript harness: `elf: ... class=x86-64|i386|x32` (ELF32 images, `elf32_image` in xtask/src/transcript/run.rs;
    docs/transcripts.md).
  * Probe `validation/transcripts/ldconfig/foreign-class.transcript` (recorded against the reference, committed):
    i386 and x32 objects are linked like native ones, cached with flags `0x3` / `0x803`, no message.
  * Engine: `Arch::LDCONFIG_TYPES` (x86-64: ELF64 EM_X86_64 0x303, ELF32 EM_386 0x3, ELF32 EM_X86_64 0x803);
    `elf_info` reads the ELF32 layout; each cache entry carries its file's flags.
  * Same soname, mixed types in one directory: the reference's cache flags and its message `libraries <a> and <b>
    in directory <dir> have same soname but different type.` follow directory order (probed: ext4 and tmpfs private
    roots, both creation orders; the tmpfs runs swapped flags 0x3/0x303 and which directory got the message). Not a
    committable transcript (file-system dependent). Liella: highest version wins, winner's flags, message in every
    order. Pending record `records/pending/UTIL-LDCONFIG-5-1.toml`; test
    `foreign_class_objects_are_recorded_with_their_type`. RULING NEEDED (see report).
  * Unsafe condition #5923 checked: every unsafe site in `crates/boundary/src/ldconfig.rs` has its UNSAFE note
    (static green). The policy lists ldconfig.rs under export, allocator AND slice: the audit classifies the
    `unsafe fn alloc/dealloc/realloc` and the `unsafe {}` copy syntactically as `slice` and accepts them as
    `allocator` only for files in both lists (policy.rs, as heap.rs and ld.rs); removing `slice` would fail them.
  * Plants `handoff/UTIL-LDCONFIG-5-plants.txt` (7).

## Open questions

1. Mixed-type same-soname rule (pending record UTIL-LDCONFIG-5-1): ruling requested.
2. Is the mixed-type message printed without `-v`? Only probed with `-v`; Liella prints it always.
3. Include-cycle message has no reference counterpart recorded. Arena peak RSS on a full /usr/lib not measured.

## Next steps (run 6), in order

1. Read util-ldconfig-5-gate and util-ldconfig-5-plants (mei); fix any red.
2. Package rebuild and checks on the merged head: `tools/deb/build.sh --flavour system` needs a git tree (it
   exports HEAD with git archive) and the role `~/w/s1` is not one; build from a checkout or use
   `--products DIR --version V` with products built in the system layout, then `check-system.sh` and
   `root-system.sh` (the trigger must write `$r/etc/ld.so.cache`: add a pass line).
3. `-l` (#5916: implement, probe its messages and status; no differences row).
4. The end-to-end loader differential over a cache the binary wrote; plan §5 counters.

Read: this note, plan/utilities.md §2.3/§4/§5, spec §31.2 and D.3, docs/transcripts.md, D-31.2/D-31.9/D-31.12.
