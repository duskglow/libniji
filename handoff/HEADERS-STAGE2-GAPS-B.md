# HEADERS-STAGE2-GAPS-B (Yohane stage 2: openssh, apt, binutils, krb5, systemd, apparmor, babeltrace, avahi)

**State:** branch `task/headers-stage2-gaps-b` (from origin/next e125f7e34). Leg 1 head d687e907a (WIP). Role keke:~/w/n2.
Probe scripts and downloads: scratch `/home/claude/w/.scratch/HEADERS-STAGE2-GAPS-B/` (p1..p4.sh, tsvdiff.py,
merge_diag.py); on keke `~/work/headers-stage2-gaps-b/`. Decisions in force: none new (every change follows a probe).

## Done in leg 1 (committed d687e907a; `libc-include/` NOT regenerated yet)

1. **openssh / namespace (premise corrected):** stock `<signal.h>` DOES declare execl/execle/execlp, read, sysconf,
   getopt, optarg, _exit (environ under _GNU_SOURCE) under `_GNU_SOURCE` and `_DYNAMIC_STACK_SIZE_SOURCE`, as ours
   (probe p1: conflicting redeclaration after the include). Only diff: ours also shows `__sysconf` (reserved; stock
   hides it). Real cause: our `__sentinel__ (0)` spelling; an empty `#define __sentinel__` (openssh's compat) breaks
   it. Stock survives it, and with `-fno-builtin` stock gives NO missing-sentinel warning: the warning is gcc's
   builtin, the reference declarations carry no sentinel. Fix: attr_probe sentinel tests (+ new zero-null `s0`
   baseline) in a `-fno-builtin` unit; diagattrs.tsv sentinel cells of execl/execle/execlp emptied.
   Spelling robustness (probe p2, empty `#define X` before 15 headers): niji-only breaks were `__sentinel__` (fixed),
   `__strftime__`, `__aligned__` (open, low value: reserved names). The step-1 sweep was not done (its premise fell).
2. **avahi / format_arg:** stock (probe p4, -fno-builtin, own format-attributed pf): gettext(1), dgettext(2),
   dcgettext(2), ngettext(1)+(2), dngettext(2)+(3), dcngettext(2)+(3). attr_probe `fa` tests + `format_arg` column;
   headers.rs `attr_text` emits `__format_arg__ (j)` per parameter. No other char*-returning function has one.
3. **systemd/apparmor / AF_* form:** new `spec/tools/header_checks/aliases.py` (black box: `#undef Y; #define Y
   sentinel`, `_Static_assert ((X) != sentinel)`; only same-valued names; direct alias = the followed name whose own
   followers are the rest). `spec/abi/macro-aliases.tsv` (header, name, alias; 2481 rows, every header; table job
   3m51 on keke, 16 compilers). sys/socket.h: AF_x=PF_x, PF_FILE=PF_LOCAL, MSG_TRYHARD=MSG_DONTROUTE,
   SO_DETACH_BPF=SO_DETACH_FILTER, SO_GET_FILTER=SO_ATTACH_FILTER, SO_RCVTIMEO/SNDTIMEO/TIMESTAMP*=*_OLD.
   headers.rs loads it; enums.rs `macro_alias` (alias used when visible wherever the macro is, mask test as
   enum_macro); item "macro": enum_macro, else macro_alias, else value. Module default: ENFORCED (sys/socket.h);
   `NIJI_ALIAS_ALL=1` probes every header.

## INFORMATIONAL found
* diagattrs.tsv `pure` column: 182 cells differ from what attr_probe produces today, with the ORIGINAL probe too
  (job headers-stage2-gaps-b-attrorig) - pre-existing drift, not taken.
* pthread_create: diagattrs nonnull `1,2,3,4`; stock warns only for args 1 and 3 (probe p3, direct calls; arg 4 not
  validly probed). A NULL attr warns under libniji (avahi's log shows it). Worth a follow-up (re-probe nonnull).

## Exact next steps (leg 2)
1. Sync; on keke `cargo xtask headers-gen` (check `git status` lists only libc-include changes + expected); check
   `grep -n "define AF_INET\|define AF_FILE" libc-include/sys/socket.h`, gettext format_arg, no sentinel.
   Also provenance: `spec/abi/provenance.d/` needs a fragment for macro-aliases.tsv? (`cargo xtask static` tells;
   then `cargo xtask records-gen`). Add `("macro-aliases.tsv", "rows", &[])` to provenance.rs TABLES if required.
2. Missing names (none is in any table yet): `__RES` (resolv.h, probe the value), `RTLD_DI_*` (dlfcn.h; check
   which requests our `dlinfo` serves: grep the dlinfo implementation; declare only served ones or RULING),
   `IN6ADDR_ANY_INIT`/`IN6ADDR_LOOPBACK_INIT` (netinet/in.h, brace initializers, check struct in6_addr's first member
   chain), elf.h 32/64 pairs (Elf32_Chdr, Elf64_Sword, + full gABI set). Method: add names to
   `spec/tools/header_probe.py` package_names() (comment the story), run with `--names FILE --headers ... --out DIR`,
   splice visibility rows; values/layouts by the macros/layout probes (see handoff/HEADERS-BUSYBOX-GAPS.md).
3. Tests: header_check run (aliases module FAILs on sys/socket.h), compile probes C and C++; plants in
   handoff/HEADERS-STAGE2-GAPS-B-plants.txt (attr_text format_arg line, macro_alias mask test, s0 baseline).
4. Proof builds on keke: openssh configure+make ssh/sshd, krb5 src/util/support, systemd af list preprocessing.
5. Gate (`cargo xtask affected` likely FULL: generated-table inputs -> precheck + header_check job), plants, report.
