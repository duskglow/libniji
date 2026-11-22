# DL-RUNPATH-SECURE-TEST: dlopen's secure-mode `$ORIGIN` rule, tested without root

State: branch `task/dl-runpath-secure-test`, cut from origin/next e72318002. Head and job results:
handoff/reports/DL-RUNPATH-SECURE-TEST.json and the hand-back. Test only: no loader code changed, and the test
found no defect.

## Premise (challenge #7440, accepted by the coordinator)

DL-RUNPATH-2 had already tested the program's `$ORIGIN` in secure mode, so this story adds only the missing parts.
The secure-mode coverage of dlopen's `$ORIGIN` (rulings #7094, #7190), all in `crates/ld/tests/runpath_search.rs`:

| what | test | plants |
|---|---|---|
| program `$ORIGIN` (DT_RUNPATH list and `$ORIGIN` name): untrusted skipped, trusted searched, both searched outside secure mode; set-group-ID copy | `secure_dlopen_origin_only_trusted` (DL-RUNPATH-2) | `no-secure-rule` (dlopen's context drops the flag; handoff/DL-RUNPATH-4-plants.txt) |
| caller library `$ORIGIN` (its DT_RUNPATH `$ORIGIN/lib/cdep` and a `$ORIGIN` name it opens): searched outside secure mode; skipped from an untrusted directory; skipped when only the program is trusted; searched when only the library is trusted | `secure_dlopen_caller_origin_only_trusted` (this story) | `secure-origin-drop`, `secure-origin-invert` (the rule in `search::expand`; handoff/DL-RUNPATH-SECURE-TEST-plants.txt) |

## How the test works (no root, no seam in the shipped loader)

- The secure flag: the test program writes 1 into the `AT_SECURE` entry of its own auxiliary vector, which is where
  dlopen reads it (`Env::read_with`). This is the same seam DL-RUNPATH-2 uses. It is the process's own memory, so
  no environment variable, file or tunable can turn secure mode on or off.
- The caller: `bin/libsec.so` (`sec_open` = one `dlopen` call). It has DT_RUNPATH `$ORIGIN/lib/cdep` and sits
  beside the program. An argument `via:PATH` of the SECURE program opens it by an absolute path, and the program
  sends later names through it.
- The trusted directory: a default directory (`Native::SYSTEM_DIRS`, no symbolic link) with the corpus's bin/
  bound over it in `unshare -r -m`. Inside the namespace the same files are reachable both under the trusted
  directory and under the corpus path. So one run checks a trusted program with an untrusted caller (the program's
  libB.so found, the caller's libD.so NULL), and a second run checks the reverse.
- Shared helpers: `secure_setup` (builds `secure` and `libsec.so`), `trusted_bind_target` (prints a SKIP when the
  namespace or the directory is unavailable), and `in_namespace`.

## Set-user-ID probe for the owner (needs root; nothing was run as root)

This extends the program-`$ORIGIN` recipe in handoff/DL-RUNPATH.md ("Secure mode") to the caller. Build as the
user in an absolute scratch directory D that is not on a nosuid file system. secure.c = the `SECURE` constant of
runpath_search.rs, libsec.c = `SEC_CALLER`, d.c = `int libd(void){return 4;}`, b.c = `int libb(void){return 2;}`.

    cc -o D/probe D/secure.c -Wl,--enable-new-dtags,-rpath,'$ORIGIN/niji-origin-sub'
    cc -shared -fPIC -o D/libsec.so D/libsec.c -Wl,--enable-new-dtags,-rpath,'$ORIGIN/niji-origin-cdep'
    cc -shared -fPIC -o D/libD.so D/d.c
    cc -shared -fPIC -o D/libB.so D/b.c

As root:

    install -d D/u/niji-origin-sub D/u/niji-origin-cdep
    install -m 644 D/libB.so D/u/niji-origin-sub/ && install -m 644 D/libD.so D/u/niji-origin-cdep/
    install -m 644 D/libsec.so D/u/libsec.so && install -o root -m 4755 D/probe D/u/probe
    T=/usr/lib/x86_64-linux-gnu
    install -d $T/niji-origin-sub $T/niji-origin-cdep
    install -m 644 D/libB.so $T/niji-origin-sub/ && install -m 644 D/libD.so $T/niji-origin-cdep/
    install -m 644 D/libsec.so $T/libniji-origin-sec.so && install -o root -m 4755 D/probe $T/niji-origin-probe

As the user (libniji's expected output follows each command):

1. `D/u/probe keep libB.so via:D/u/libsec.so libD.so '$ORIGIN/niji-origin-cdep/libD.so'`. Expected: `secure 1`,
   libB.so and both caller names NULL (program and caller untrusted).
2. `$T/niji-origin-probe keep libB.so via:D/u/libsec.so libD.so '$ORIGIN/niji-origin-cdep/libD.so'`. Expected:
   libB.so `$T/niji-origin-sub/libB.so`, both caller names NULL (only the program is trusted).
3. `D/u/probe keep libB.so via:$T/libniji-origin-sec.so libD.so '$ORIGIN/niji-origin-cdep/libD.so'`. Expected:
   libB.so NULL, both caller names `$T/niji-origin-cdep/libD.so` (only the caller is trusted).
4. Repeat steps 1 to 3 with libniji copies of probe and libsec.so, built as the test builds them: `-nostdlib`, the
   libniji ld.so as `--dynamic-linker`, `niji-crt1.o`, and the corpus's libc by path. Compare with the reference.

Clean up as root: `rm -rf D/u $T/niji-origin-sub $T/niji-origin-cdep $T/libniji-origin-sec.so $T/niji-origin-probe`.

## Next step

Land. Then the owner's probe above, which settles the reference's caller-`$ORIGIN` behaviour.
