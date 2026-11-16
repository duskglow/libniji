# UTIL-LDD (UTIL-4): ldd

**State.** Branch `task/util-ldd` (from origin/next `4a0a545ef`), role keke `~/w/n3`. Code head `48c567897`; this
note and `handoff/reports/UTIL-LDD.json` are the commit after it. INCOMPLETE (context checkpoint): code and
transcripts done and green; plants, gate, decision record and the initramfs consumer check are not done.

## On the branch

* `csys/util/ldd.c`: ldd as a static PIE over `libc.a` (like getconf). It reads the ELF headers itself: not ELF64
  x86-64 / no `PT_DYNAMIC` -> `\tnot a dynamic executable` (stderr, 1); no `DT_NEEDED` -> `\tstatically linked`
  (stdout, 0); otherwise it runs Liella's loader (`LDD_RTLD`, default `/lib64/ld-linux-x86-64.so.2`) on the
  operand with `LD_TRACE_LOADED_OBJECTS=1` and prints the listing. It never runs the operand (D-31.7). `-v` reads
  `DT_VERNEED` of the program and of each listed object itself. `-d/-r/-u` refused (exit 1) per spec §31.3 rule.
  A program whose `PT_INTERP` is not `/lib64/ld-linux-x86-64.so.2` gets a note on stderr (D-31.7).
* `xtask/src/so.rs` `run_ldd`: `target/liella/ldd` (shipped) and `target/liella-test/ldd` (runs this tree's
  loader); `transcript/mod.rs` maps `ldd` to the test link.
* Harness: `cc: <path> <flags>` + C source block (programs and libraries built by the host `cc` at setup; the
  transcript is SKIPPED with the reason when there is no cc) and `mask-addresses: yes` (only the final
  ` (0x<hex>)` of a line becomes `@ADDR@`; unit test `mask_addresses_masks_only_a_final_hex_group`).
  docs/transcripts.md NOT yet updated for the two directives.
* `validation/transcripts/ldd/`: 28 transcripts recorded from the reference on keke (generator kept in scratch,
  not committed): no operand, missing (relative `./FILE`, absolute), directory, text, script, cut ELF, i386 ELF,
  static, static-pie, 0/1/several needed (with a transitive one), `=> not found`, library operand, two operands,
  operands with failures, `-v`, `--verb`, `-v` with not found, `-v` with two operands, `--`, option after a file,
  `-x`, `-vd`, `-`, `--foo`, `--ver` (ambiguous). Test programs use no libc (`-nostdlib`), since Liella's loader
  pins `libc.so.6` to its own prefix and so cannot list the reference's libc.
* libc-bin ships `/usr/bin/ldd` (`tools/deb/system.sh`, stripped; `check-system.sh` allows the path).

## Probe-settled (reference, black box)

Header `<file>:` for every operand when there are several, also for failing ones; missing operand message names
`./FILE` when FILE has no slash; `ldd: ./.: not regular file`; options only before the first file (an option after it
is a file); no bundling; unique long prefixes, `option `--ver' is ambiguous` (no Try line), exit 1; `--help` and
`--version` win at once. `-v`: the blank line and `\tVersion information:` appear only when some object has version
needs; objects without needs get no header; the program is named as shown (`./x` or as given). The interpreter
line is listed only when some object names the interpreter in `DT_NEEDED` (no-libc programs have none); for an
operand without `PT_INTERP` (a library) the reference prints no loader line. `-d`/`-r` on a clean program print the
plain list; `-u` printed nothing for a program with an unused lib (not explained).

## Open / next step (in order)

1. Decision record `records new decision D-NEW --series D-31 --story UTIL-LDD`: ldd drops the loader's
   interpreter line when nothing needs it (Liella's loader prints it always in trace mode: the loader fix belongs to
   a DL story; recommend one), lazy version header, identity strings as getent (`ldd (Liella) 2.39`, no bug lines).
2. docs/transcripts.md: document `cc:` and `mask-addresses:`.
3. Plants file `handoff/UTIL-LDD-plants.txt` on ldd.c (cmd: relink as `~/work/UTIL-LDD/rec.sh` does, then
   `cargo xtask transcript check validation/transcripts/ldd`) and on `mask_addresses`; run via `cargo xtask plants`.
4. Gate: `cargo xtask clippy`, `cargo xtask static`, `cargo test -p xtask` (transcripts.rs runs these transcripts),
   then the initramfs-tools `copy_exec` consumer check in a private root (not started).
Read: plan/utilities.md §2.3/§4/§5, spec §31.3, D-31.7, this note, docs/transcripts.md.

## Rule slips (no effect on the tree)

One empty heredoc (`python3 - <<'X'`) ran nothing; two python edits through `-c` lost quotes (fixed with Edit).
