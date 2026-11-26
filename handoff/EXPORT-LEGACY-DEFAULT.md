# EXPORT-LEGACY-DEFAULT: bindresvport and __libc_sa_len at their default versions

State: branch `task/export-legacy-default` (from origin/next `dff0cc5c1`); HEAD named in the report json.
Decisions in force: D-15.7 (bindresvport port order, unchanged). No new decision row: the export follows the
reference's own versions (`spec/abi/versions.tsv`), and `__libc_sa_len` follows the probe below.

## What changed

- `bindresvport` (crates/boundary/src/posix/s29_rpc_clnt.rs): `internal_entry!` `niji_compat_bindresvport` became
  `export! { "GLIBC_2.2.5"; bindresvport }`. Body unchanged.
- `__libc_sa_len` (crates/boundary/src/posix/s15_sockets.rs): new `export!` at GLIBC_2.2.5, table `SA_LEN`.
- Tests: `sym_bindresvport`, `sym___libc_sa_len` (crates/boundary/tests/s29_legacy_symbols.rs) replace their stubs;
  new `crates/core/tests/legacy_default_products.rs` + `tests/oracle/legacy_default_diff.c` (reference vs libniji
  `libc.so.6` vs `libc.a`, same output; the dynamic program must bind both `@GLIBC_2.2.5`); `userns_paths.rs` drops
  its `-Dbindresvport=niji_compat_bindresvport` rename.
- Docs: docs/spec-symbol-gaps.md row (e) 2 -> 0 (total 698 -> 696, the page's 11-04 snapshot), the two rows out of
  docs/spec-symbol-gaps.tsv; release note docs/release-notes.d/EXPORT-LEGACY-DEFAULT.md. The differences rows naming
  `bindresvport` (rcmd family, ONC RPC client) say it "works as before": still true.

## Probe (stock glibc 2.39, Ubuntu 2.39-0ubuntu8.9, x86-64, Skylake-SP host)

Exports: `__libc_sa_len@@GLIBC_2.2.5` FUNC GLOBAL; `bindresvport@@GLIBC_2.2.5` FUNC GLOBAL (default, no compat
version). `bindresvport6` is declared by `<netinet/in.h>` but not exported (already a differences row).
Headers: `bindresvport` declared in `<netinet/in.h>`; `__libc_sa_len` declared by no installed header (no header gap).

`__libc_sa_len(af)` over all 65536 families: 1->110, 2->16, 3->16, 4->16, 5->16, 10->28, 11->28, 17->20, 18->32,
19->16, every other 0. Called through an `unsigned int` prototype: 0x10002 -> 16, 0x20000 -> 0 (only the low 16 bits
count; libniji takes `c_ushort`).

`bindresvport` (unprivileged / inside `unshare -r -m -n`):

| case | unprivileged | in namespace |
|---|---|---|
| NULL sin | -1 EACCES | 0, port in 512..1023 |
| sin AF_INET, port 4000 given | -1 EACCES | 0, sin_port = bound port |
| sin family AF_INET6 / AF_UNSPEC | -1 EAFNOSUPPORT | -1 EAFNOSUPPORT |
| AF_INET6 socket, NULL sin | -1 EINVAL | -1 EINVAL |
| already bound socket | -1 EACCES | -1 EINVAL |
| bad fd | -1 EBADF | -1 EBADF |
| every port 1..1023 held | - | -1 EADDRINUSE |

INFORMATIONAL: after a failed call with a caller's `sin`, stock leaves a port number in `sin->sin_port`; libniji
writes `sin` only on success. No caller can use the value; not changed (D-15.7 unchanged per the brief).

## Next step

None for this story once the gate and plants are green. `dlmopen` (the other default name of the R-H7 report) stays
with DL-22.
