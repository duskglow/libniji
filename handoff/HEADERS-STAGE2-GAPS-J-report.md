# HEADERS-STAGE2-GAPS-J: report

Branch task/headers-stage2-gaps-j (base origin/next dbb8c346f, no merge). Decision D-X.179. All jobs on keke (Zen 1
host), role ~/w/n1.

## Commits (all mine)
* 073ff35ea (leg 1): items 1, 2, 3, 5: macro-values.tsv + generator + dup_values.py; nested-tags.tsv; ANYADDR; tables.
* da93299eb, 5b38c470c (leg 2): item 4 tables and templates (ax25, netrom, rose, if_ppp, ppp_defs).
* 4dba9aeca: emit_nested defines a member's structure only where no included header shows it (`nested_tag_covered`):
  <net/if_ppp.h> repeated the kernel's struct compstat and did not compile alone; 21 other headers lose only
  libniji-guarded repeats (no include cycle among them). header_check sanitizer unit.
* 14b1f1b1c: typedefs.tsv item-4 rows in order (static abi-types); dup-listed.tsv shrunk by dup.py --rows.
* 2c0a17239, 893568eca: plants file.
* dd26b37f3: the rows test's data (enum NPmode enumerators, __kernel_old_time_t) and C++ nested tag spelling.

## Proof
* Probe (p1.sh, C and C++, against stock 2.39 /usr/include and both libniji trees, libc-include and
  target/niji/include): 0 differing lines (items 1-5: values, forms, types, whod::whoent, ANYADDR, tables by run,
  sizes 48/36/76, the 11 PPPIOC* and 10 SIOCAX25*/SIOCNR* values); include closure depth 1-2 equal.
  Jobs headers-stage2-gaps-j-gen4 (4dba9aeca's tree).
* dup_values audit: diff=0 (gen4: 56 headers, 6583 values; gate1: 53 headers on libc-include, 6532).
* header_check, the whole harness: 0 failed of 26728 (gate1, on 893568eca's tree; 893568eca = 4dba9aeca + tables +
  plants file).
* System-flavour .debs from the role tree (4dba9aeca), job headers-stage2-gaps-j-pf1:
  * inetutils 2.6 (trixie source, mmdebstrap chroot): builds on stock (rc 0) and with libniji's libc6, libc-bin,
    libc6-dev, libc-dev-bin swapped in (rc 0, all debs incl. talkd).
  * llvm-19 compiler-rt 19.1.7 sanitizer_platform_limits_posix.cpp, clang-19 and g++: the ax25/netrom/ppp gaps are
    gone; first further stop, both compilers: `<netipx/ipx.h>` `SIOCAIPXITFCRT`, `SIOCAIPXPRISLT`, `SIOCIPXCFGDATA`
    undeclared (lines 944, 945, 953). Not in this story's scope: a follow-up story.
* Gate: affected escalated to FULL (crates/abi, headers/nested-tags.tsv): the changed crates' tests instead.
  gate1: cargo test -p niji-abi ok (400 + 5 passed); cargo test -p xtask 691 passed, 1 failed (the rows test,
  fixed in dd26b37f3, rerun in fin1); headers-gen --check ok.
* RESULTS-FINAL (on c0b977b7b's tree): fin2 (job headers-stage2-gaps-j-fin2): kernel_spelled rc 0 (1506 names,
  no change), headers-gen rc 0 (nothing changed), rows test rc 0, clippy rc 0, static rc 0, whole header_check
  0 failed of 26728. plants-2 (headers-stage2-gaps-j-plants2): 9 plants, 9 CAUGHT, 0 SURVIVED, 0 WARN (results in
  handoff/HEADERS-STAGE2-GAPS-J-plants.txt). precheck: see the hand-back.

## Informational
* PPPIOC* are macros.tsv literals (unsigned long); stock spells them as _IOR/_IOW forms, unusable in `#if`
  (form.py report-mode FORM-DIFF, as MTIOCTOP). A forms-of-ioctl-requests follow-up.
* header_check compiles only: a dropped tsptype/ripcmds entry still compiles (the arrays are sized); the entries are
  proven by the run probe, not by a gate unit.
* A generator mutant that breaks nested-tag placement makes headers-gen recurse until its stack overflows (plants-1
  j-nested-tag): a loud failure, not a wrong header; not changed.
* The deb build (pf1) ran beside the gate1 tests on keke: separate work tree and target dir, load ~3 of 60.
* Leg 1: one `sed -i` on its own new file by mistake (checked, .bak removed); a stray local `python3 -` killed by PID.
