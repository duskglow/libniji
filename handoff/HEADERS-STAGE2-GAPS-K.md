# HEADERS-STAGE2-GAPS-K: handoff (run 1, end of leg 2: code done, probes run; proof jobs next)

Branch task/headers-stage2-gaps-k, cut from origin/next 16e03637e, no merge. Role shiki:~/w/sh1 (Skylake-SP class, no
clang; clang-19 and the llvm-19 source on keke: ~/work/RELEASE-SNAPSHOT-1127A/u2/pkgs/compiler-rt/..., J's proof
script keke ~/work/headers-stage2-gaps-j/pf1.sh builds the system-flavour debs and compiles the unit).
Job scripts and outputs: shiki ~/work/headers-stage2-gaps-k/ (p1.sh/j1.sh probes + form.py; p2.sh/j2.sh pair probes;
kt.py kernel UAPI text of names; r1.sh TAG = headers-gen + j1 + j2). Local copies: the run's scratch dir.

## Done (commits 60f848637, then "Own the IPX requests")
* New table `spec/abi/macro-spellings.tsv` (name, spelling, source; 109 rows; table_keys.tsv line): the generator
  (xtask/src/headers.rs `spellings`, `macro_def`, `redefined`) emits the spelling in every header that shows the
  name, outright where macro-forms says so, kernel-spelled.tsv notwithstanding. Rows: 45 PPP names of
  linux/ppp-ioctl.h (all shared but SC_MASK), 10 linux/fs.h names (BLK*, BLOCK_SIZE, SYNC_FILE_RANGE_WRITE_AND_WAIT),
  49 linux/mtio.h names, TIOCSER_TEMT `0x01`; source `-`: MTIOCGETCONFIG/SETCONFIG `_IOR/_IOW('m', 4/5, struct
  mtconfiginfo)`, SC_MASK `266403839` (outright, as stock: redefinition with the kernel's either order),
  TTYDEF_SPEED `B9600` everywhere. Kernel texts from shiki /usr/include (kt.py; = what probes use).
* `<netipx/ipx.h>`: the four (SIOCPROTOPRIVATE+n) requests, `sipx_special` -> `sipx_port` (probe: offset 2 size 2),
  `@@owns`, 5 visibility rows (!time64only), probe candidates header_probe.d/stage2-gaps-k.py, radio.py note.
* header_checks/s13.d/60-stage2-gaps-k.py: pairs both orders (termios/termio/pty/sgtty/ttydefaults with sys/ioctl.h
  or termios.h, termios+asm/ioctls.h, if_ppp+linux/ppp-ioctl.h with SC_MASK undefined between, ipx+sys/ioctl.h),
  kernel respelling per header for every source!=`-` row, IPX values/parenthesis/sipx_special, TTYDEF_SPEED==B9600;
  `p1_skip` TTYDEF_SPEED in the 8 headers not showing B9600. form.py ENFORCED: the 26 ioctl requests.

## Probe results on the regenerated libc-include (job headers-stage2-gaps-k-gen2, tag k2)
* form.py: 274 -> 245 differing pairs; 0 FORM-DIFF lines for PPPIOC*/BLK*/MTIOC*.
* p2 (pairs, -Wsystem-headers): 0 DIFF except `asm/termbits.h + termios.h` (stock: 111/78 redefinitions; libniji
  fails with another diagnostic in that order: both reject the unit; pre-existing, not in scope -> INFORMATIONAL).
* p1: 28 lines left, all TTYDEF_IFLAG/OFLAG/LFLAG/CFLAG with `<sys/ioctl.h>` alone (stock: no value; libniji
  literal): form.py audit group E1, pre-existing report mode -> INFORMATIONAL (not this brief's redefinition item).
  IPX, PPP, termios pair items: 0 DIFF.
* kernel-spelled.tsv NOT regenerated (needs `cargo xtask so` for target/niji/uapi; the redefined() override makes
  it unnecessary; optional: after the gate's build run `python3 spec/tools/kernel_spelled.py` and commit if changed).

## Next step (leg 3)
1. On shiki sh1 (synced): `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo xtask
   static` (headers-gen --check runs there), `python3 spec/tools/header_check.py` slice 13 (or the affected tests).
2. Plants in docs/plants.md (format `id | path | pattern | cmd | msg:`): e.g. a TIOCSER_TEMT row spelled `1`, a PPP
   row text changed (`_IOR('t', 90, int)` -> `_IOR('t',90,int)` is NOT a plant: same tokens; use another nr), the
   IPX `+2` -> `+3`, sipx_special -> sipx_family, redefined() override removed. cmd: the header_check / affected run.
3. Gate: `cargo xtask affected --list` (expect FULL escalation for xtask/spec changes: then precheck + changed
   crates' tests, kind test); plants job; then llvm-19 sanitizer unit on keke (pf1.sh pattern), clang-19 and g++.
4. precheck --jobs, report json, handback-check.
Jobs so far (shiki): headers-stage2-gaps-k-probe1, -probe2 (base tree), -gen1 (failed: owns), -gen2 (k2 probes).
