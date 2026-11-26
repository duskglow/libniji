# HEADERS-STAGE2-GAPS-G: handoff (run 1, leg 3: hand-back)

Branch task/headers-stage2-gaps-g from origin/next dff0cc5c1; role keke:~/w/r5. Code head da637ab15 (the final jobs
ran on it); later commits touch handoff/ only. No decision rows (local generator detail documented in
headers/reserved.tsv's header); black-box probes only. Plants file: handoff/HEADERS-STAGE2-GAPS-G-plants.txt (23).

## Done (all pushed)
Items 1-7 of the brief (legs 1-2; commits 072b0f243, 2c2f8cc2f, 2cf22e256, 4997f30fc, 77c3c4b0c): XSI strerror_r
(redirects.tsv), resolv.h `(__x) (` renames, __BYTE_ORDER family / __SIGRTMIN/MAX / __toupper_l / __tolower_l,
__bswap_16/32/64 static inline, kernel UAPI includes (fanotify, ethhdr, fddi), FTS64/FTSENT64 layouts, clang fortify
(no __builtin_va_arg_pack under clang), obstack macros, CTRL/CCEQ, PTRACE_O_*/EVENT_* (enumerators only), SHADOW,
ELF_NOTE_*/NT_GNU_*, IGMP. Leg 3 (be31db8cf, 0eec4d072) fixed what the first full test run found:
* DUP (header_checks/dup.py, list only shrinks): reserved.tsv policy `emit-once` (generator `Gen::item`: the name's
  definition in each header under one `__NIJI_D_m_<name>` guard) for the 5 byte-order names; CTRL fragments and a new
  elf.h / sys/auxv.h fragment (ELF_NOTE_FDO/GNU/SOLARIS) under one guard each. Policy readers (header_probe.py,
  probe_compare.py, header_check.py P1G) accept `emit-once`.
* macro-forms.tsv +95 rows from feature_probe.forms_batch on stock: __FLOAT_WORD_ORDER kept in 94 headers,
  __LITTLE_ENDIAN kept in sys/rseq.h (376 other pairs replaced = default).
* feature_probe.compiler: + header_probe.uapi_flags() (the forms probe could not compile headers that include
  <linux/*.h>, so every name read "replaced"): full forms check 210 headers, 0 differ.
* rows test (rows.rs): no prototypes.tsv redeclaration for a function bound to a redirects.tsv symbol (strerror_r).
* providers.tsv: __bswap_16/32/64 `none` (static inline, as inb). abi-gen regenerated (FTS64 layout rows).
* Coordinator note 18:03Z (RELEASE-SNAPSHOT-1126B): (a) CTRL through sys/ttydefaults.h, termios.h, sys/termios.h,
  termio.h, sys/ioctl.h: already as stock (probe of each, stock and libniji: CTRL ('d') == 4 compiles); (b) n_short
  (uint16_t), n_long and n_time (uint32_t) in <netinet/in_systm.h>, every configuration but time64only (header_probe
  candidates in header_probe.d/stage2-gaps-g.py; typedefs/visibility rows; s12.d unit: same types, C and C++).
* Plants: DUP plants must mutate a header dup.py's unit includes after the first definer (slice order): plants-2 had
  s2g-ctrl-once / s2g-elf-note-once SURVIVED in sys/ioctl.h / sys/auxv.h (first definers); moved to termios.h / elf.h.

## Proofs
* Package units (job units-3, script ~/work/headers-stage2-gaps-g/units.sh on keke; sources from the Debian/Devuan
  pools and the LLVM release in /dev/shm/claude/headers-stage2-gaps-g/pkgs): 15 units, stock rc 0 and libniji rc 0
  each: systemd + elogind sparse-endian.h (-Werror=undef), eudev libudev-device.c + libudev-hwdb.c, openssh
  sshbuf-misc.c + openbsd-compat/base64.c, liburing test/fsnotify.c, iptables nft-bridge.c nft-arp.c xtables-eb.c,
  fakechroot fts64.c, libssh ssh_strerror (misc.c's includes + the function), glib2.0 gsubprocess ptrace block,
  llvm-19 libunwind Unwind-seh.cpp + libunwind.cpp (clang-19, the log's flags: -O3 -D_FORTIFY_SOURCE=2, libniji's
  stdio.h confirmed by -H). Limits: no dev libraries, meson or cmake on the host and no root, so configure ran against
  the stock toolchain and only the unit compile switched; systemd/elogind/libssh/glib are the failing header or block
  with the unit's flags, not the whole unit (needs glib, openssl, meson config.h). fakechroot has no configure in
  its +ds tarball: config.h from the log's configure results; its fts.c is GNU C Library code, compiled as a black
  box and not read (clean room).
* Jobs on da637ab15 (keke): test-4 (cargo test -p xtask; niji-abi appendix_c_layouts; full header_check),
  plants-4 (23 plants), units-5 (the 15 package units). Superseded: plants-1/test-1 (77c3c4b0c: test-1 found DUP,
  forms, rows and abi-gen failures), test-2/plants-2 (0eec4d072: test green; 2 DUP plants survived, moved),
  test-3/plants-3 (stopped: typedefs.tsv order), units-1..4 (harness fixes / earlier heads). FULL escalated
  (generator inputs): `FULL: the train's`.

## INFORMATIONAL (found, not fixed here)
* TTYDEF_SPEED is B9600 in termios.h and 13 in sys/ioctl.h / sys/ttydefaults.h: both families together conflict
  (-Werror). termios.h before the kernel's asm/ioctls.h also conflicts in header_check's UAPI tree.
* 147 functions have warn_unused_result on stock only under fortify, on libniji always (diagattrs.tsv from the
  fortify=3 base); nothrow/leaf absent in C. Next story: per-config wur column + `__NIJI_WUR`.
* __asprintf (reserved, undeclared on libniji) is a clang macro on stock; obstack_empty_p's C++ type not verified;
  a program's 0-argument `__res_init()` macro: stock errors, libniji compiles.
* Item 1 sweep: gnu C getitimer/setitimer/getrlimit/setrlimit(64) take enum types on stock; getumask, bindresvport6,
  _argp_input, _option_is_short undeclared on libniji. Item 4 sweep: stock-only kernel include closures beyond the
  three fixed (errno.h -> linux/errno.h in 19 headers, linux/limits.h, falloc, close_range, sockios, param, ppp,
  if_slip, stat, quota, prctl, sysinfo, rseq, atalk): listed, not changed.
* feature_probe's forms probe had no UAPI tree before this branch: forms results of other kernel-including headers
  (KNOWN_KEPT in features.py) may have been vacuous on next; the full forms check is clean with the UAPI tree now, so
  KNOWN_KEPT may be removable (not tried).

## Next step
None for this story beyond landing. Follow-ups: the wur/nothrow story above; the item-4 kernel closure sweep.
