# SYS-CONTROL-PARITY: package relations and conffiles of the system flavour (audit rows 7, 8, 25)

Size: S. Kind: code (tools/deb/system.sh and its checks). From plan/package-parity-audit.md.

1. libc6-dev `Depends`: add `linux-libc-dev`, `libcrypt-dev`, `rpcsvc-proto` (`ref-control.tsv:165`). Builds that
   include `<linux/…>`, `<crypt.h>` or run `rpcgen` rely on libc6-dev pulling them in.
2. libc6-dev `Provides: libc-dev (= <our version>)` instead of the unversioned form (`ours-control.tsv:81`): an
   unversioned Provides satisfies no versioned relation (`libc-dev (>= 2.38)`).
3. Check from the spec whether libniji reads `/etc/bindresvport.blacklist` and `/etc/default/nss`; ship each one it
   reads as a libc-bin conffile (`ref-control.tsv:32`); `/etc/locale.alias` goes with SL-LOCALEDEF.
4. `Multi-Arch: foreign` on libc-l10n; `Priority` values as the reference has them (`ref-control.tsv:69`).

Proof: `tools/deb/check-system.sh` checks each field (plants on each); `apt-get build-dep` in the swapped root for a
package that needs `linux-libc-dev` only through libc6-dev; rerun `tools/deb/parity.sh` and the rows are gone.
