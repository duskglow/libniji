# HEADERS-STAGE2-GAPS-B (Yohane stage 2: openssh, apt, binutils, krb5, systemd, apparmor, babeltrace, avahi; gnu-efi)

**State:** branch `task/headers-stage2-gaps-b`, code head 0e9c6ec87 (merged origin/next ecc8c95e3 at 533c1677f). Role
keke:~/w/n2. Scratch `~/w/.scratch/HEADERS-STAGE2-GAPS-B/`; on keke `~/work/headers-stage2-gaps-b/`. No new decision
rows (all local, table-driven; forms settled by black-box probes recorded in the check modules). Full report below.

## Done (all legs)
* openssh (item 1): premise corrected in leg 1: the reference's `<signal.h>` does declare execl/execle/execlp under
  _GNU_SOURCE; the failure was libniji's `__sentinel__ (0)` spelling, which openssh's compat `#define __sentinel__`
  broke. The reference carries no sentinel attribute (its warning comes from compiler builtins): attr_probe judges
  sentinel in a -fno-builtin unit; libniji declares none. Namespace sweep (P2: header_probe --include libc-include +
  probe_compare visibility) on the 15 headers this story touches: 38 differences, all identical on origin/next's
  headers (same sweep on ecc8c95e3): none introduced here; listed under INFORMATIONAL.
* Forms (item 2): AF_x as PF_x and the aliases as aliases via macro-aliases.tsv (aliases.py probe); a macro is spelled
  as its alias only where the alias is shown (SO_RCVTIMEO as SO_RCVTIMEO_OLD). aliases check: 66 reference aliases,
  0 differences for sys/socket.h (MSG_*, SOL_*, SO_* included).
* Names (item 3): `__RES` 19991006; RTLD_DI_* / RTLD_DL_* enumerators + Dl_serinfo/Dl_serpath; IN6ADDR_ANY_INIT /
  IN6ADDR_LOOPBACK_INIT; ELF 32/64 type pairs (gABI set); leg 4: netinet/udp.h UDP_* (12) + SOL_UDP, ELFMAG (char[5]);
  coordinator 12:33Z: the 20 ELF32_/ELF64_ R_/ST_/M_ accessors (elf.h, link.h, sys/auxv.h), forms from 89 probed uses.
  TCP_ENCAP_ESPINTCP, UDP_ENCAP_RXRPC and the unsized ELF_* accessors: not shown by any reference header, left out.
* Attributes (item 4): format_arg on gettext family (gettext 1, dgettext/dcgettext 2, ngettext 1,2, dngettext/
  dcngettext 2,3).
* Tests: header_check units s10 format_arg, s12 initializers (IN6ADDR, __RES, ELFMAG), s16 dl_requests and elf_info;
  9 plants in handoff/HEADERS-STAGE2-GAPS-B-plants.txt.

## Jobs (keke)
* Final head 0e9c6ec87: gate5 (header_check 0 failed of 20285, aliases 0 differences, clippy + static OK, cargo test
  1000 passed / 1 failed: gencat transcripts, cause the role's missing target/niji, gencat's interpreter dangling), gate5b
  (cargo xtask so + xtask transcripts: 10 passed), plants3 (9 plants), proof5 (systemd af 48 = 48 names; krb5 configure + make
  util/support OK; openssh 10.0p1 make -k ssh sshd: only misc.c fails, TCP_NODELAY = HEADERS-NET-SYS-NAMES item 1, not
  on next yet; getrrsetbyname.c now builds with next's HEADER).
* Superseded: plants1 (941fcbf01, 6/6 CAUGHT), gate4 + plants2 (1db1e31b9; stopped when the ELF accessors were added;
  gate4's cargo test saw target/niji-test/gencat vanish mid-run while plants2 ran beside it on the same role: my
  error, one build per role at a time). The same failure in gate5 alone showed the real cause (above).

## Open / for the hand-back
* RULING NEEDED: RTLD_DI_CONFIGADDR/PROFILENAME/PROFILEOUT are declared (reference enumerators) but libniji's dlinfo
  refuses them (-1, dlerror); recommend keep declared. Served: LMID, LINKMAP, ORIGIN, SERINFO, SERINFOSIZE,
  TLS_MODID, TLS_DATA, PHDR.
* INFORMATIONAL (P2 sweep, inherited from next): sys/param.h macros (MAX MIN howmany roundup powerof2 setbit clrbit
  isset isclr) shown by arpa/nameser.h and resolv.h; siginfo_t shown in posix1/posix2/xopen by signal.h, resolv.h,
  arpa/nameser.h; sys/mount.h FSCONFIG_* enumerators + fsconfig_command; signal.h __sysconf (gnu, dynstack, C++);
  resolv.h fwrite_unlocked macro (fortify); libintl.h gettext/dgettext/dngettext/ngettext/LC_GLOBAL_LOCALE macros
  missing under fortify; link.h La_x86_64_vector also in C++. -fno-builtin format attribute difference; diagattrs
  drift 182 cells; pthread_create nonnull 1,2,3,4 vs 1,3.

* Coordinator 13:45Z (netcat-openbsd), NOT DONE: libniji's libresolv.so.2 exports __b64_ntop/__b64_pton and
  visibility.tsv shows both as resolv.h functions, but libniji's resolv.h declares neither, and b64_ntop/b64_pton are
  not in header_probe's candidates. Next: add them to stage2_names, probe their form (macro or function, which header),
  find why headers-gen skips the __b64_* prototypes, add a unit and a plant.

## Next step
b64_ntop/b64_pton (above) as a follow-up story; plants3 verifies the final head; the openssh misc.c failure
clears when HEADERS-NET-SYS-NAMES lands.
