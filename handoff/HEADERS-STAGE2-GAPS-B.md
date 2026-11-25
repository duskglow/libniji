# HEADERS-STAGE2-GAPS-B (Yohane stage 2: openssh, apt, binutils, krb5, systemd, apparmor, babeltrace, avahi)

**State:** branch `task/headers-stage2-gaps-b` (from origin/next e125f7e34), leg 3 ends at 88ad51cb2 (pushed). Role
keke:~/w/n2. Local scratch `~/w/.scratch/HEADERS-STAGE2-GAPS-B/` (regen3/gate3/proof/proof2/probe3.sh, overlay.py,
ncc, splice.py, sortmacros.py, security-private.txt); on keke `~/work/headers-stage2-gaps-b/`. Decisions: none new.

## Done
* Legs 1-2: attr_probe sentinel + format_arg, macro-aliases.tsv + aliases.py, AF_x as PF_x, __RES, RTLD_DI_* +
  Dl_serinfo, IN6ADDR_*_INIT, ELF 32/64 pairs, header_check units (s10 format_arg, s12 initializers, s16 dl_requests).
* Leg 3: 78d949bc0 "Spell a macro as its alias where the reference shows the alias" (macro_alias returns a condition when
  the alias is narrower: SO_RCVTIMEO is SO_RCVTIMEO_OLD where shown, value elsewhere; macros.tsv re-sorted, abi-types
  was red); 941fcbf01 "Regenerate headers" (resolv.h __RES, sys/socket.h SO_*, limits.h _XOPEN_IOV_MAX, sys/mount.h
  OPEN_TREE_CLOEXEC); 88ad51cb2 attr_probe `s` lines differing only by -Wformat-extra-args = known difference
  SENTINEL_FORMAT (s19 test was red: printf/fprintf/sprintf/scanf/fscanf/sscanf carry format in libniji, reference
  relies on builtins; visible only under -fno-builtin: INFORMATIONAL), and header_probe stage2_names + UDP_* (12),
  SOL_UDP, TCP_ENCAP_ESPINTCP, ELFMAG (coordinator notes 11:58Z, 12:07Z).
* Jobs: check1 (SUPERSEDED: ALIAS-DIFF SO_*, abi-types, private-paths; all fixed), regen3 green, gate3 on 941fcbf01
  (header_check 0 failed of 19652, clippy, static OK, cargo test 983 passed / 1 failed = s19, fixed in 88ad51cb2;
  its drift check printed "not a git repository": role dir has no .git, use a checksum instead), plants1 on 941fcbf01
  (running at 12:12Z; caught so far: format-arg-dropped, alias-narrow-dropped, alias-after-enum).
* Proofs on 941fcbf01 headers (proof1/proof2, ncc = -nostdinc + libc-include + host headers minus libc6-dev, linked
  with the host's libc): systemd af list 48 = 48 names equal; krb5 1.21.3 configure + make util/support OK; openssh
  10.0p1 --without-openssl --without-zlib, make -k ssh sshd: 2 files fail, getrrsetbyname.c (HEADER, landed on next
  with HEADERS-APT-GAPS-2) and misc.c (TCP_NODELAY: HEADERS-NET-SYS-NAMES item 1, merging next per coordinator #8568).

## Exact next steps (leg 4)
1. plants1: `~/bin/jobwait headers-stage2-gaps-b-plants1` (it runs from ~/w/n2: sync is BUSY until it ends).
2. Sync 88ad51cb2; job `probe3` again (`~/work/headers-stage2-gaps-b/probe3.sh`, label e.g. ...-probe3c): visibility
   rows (vis3/visibility.tsv) and values (tp3/*.tsv) of UDP_*/SOL_UDP/TCP_ENCAP_ESPINTCP/ELFMAG. Splice with
   `splice.py vis` / `splice.py rows` (splice only, never whole regeneration); ELFMAG's macros.tsv row is a string
   (`char[5]`, as `_PATH_BSHELL`'s); keep macros.tsv sorted by name (byte order). Add a header_check unit if one fits
   (s12 `initializers` style or the udp.h slice) and a plant for it.
3. Merge origin/next (coordinator allows; conflicts in generated files: regenerate, never hand-merge), `cargo xtask
   headers-gen`, rsync libc-include/ and crates/abi/ back, commit.
4. On the final head: gate (headers-gen drift by checksum, `cargo test -p niji-abi -p xtask`, full header_check, clippy,
   static; affected escalates FULL: write `FULL: the train's (generated tables)`), plants (new dir plants-2, add the
   new plant), proof.sh + proof2.sh (openssh expected 0 failing files if NET-SYS-NAMES landed; otherwise name misc.c).
5. precheck `--jobs <labels> --head <sha>`, report json, handback-check, hand-back with security-private.txt verbatim.

## Open / for the hand-back
* RULING NEEDED: RTLD_DI_CONFIGADDR/PROFILENAME/PROFILEOUT declared (reference enumerators) but our dlinfo refuses
  them (-1, dlerror); recommend keep declared. Others served: LMID, LINKMAP, ORIGIN, SERINFO, SERINFOSIZE, TLS_MODID,
  TLS_DATA, PHDR.
* INFORMATIONAL: -fno-builtin format attribute difference (above); CHAR_MAX unconditional; ELF32_ST_*/ELF64_R_* macros
  absent both halves; diagattrs pure drift 182 cells; pthread_create nonnull 1,2,3,4 vs 1,3.
* Process note: leg 3 ran `sed -i` twice by mistake (no effect either time: a no-op on this note, /dev/null).
