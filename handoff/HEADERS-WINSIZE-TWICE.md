# HEADERS-WINSIZE-TWICE: sys/mount.h must not define struct winsize a second time

State: branch `task/headers-winsize-twice`, code commit b73ee32ef (head: the commit carrying this note). COMPLETE.

## Cause

`headers/sys/mount.h.in`, `sys/mtio.h.in` and `sys/raw.h.in` (slice 15) included `<sys/ioctl.h>` when
`__has_include` found one. libniji ships no `<sys/ioctl.h>` (slice 7's tail), so on any include path that has one
(header_check P8 adds the host's `/usr/include` for the kernel UAPI headers) the include reached another C library's
header. Its `struct winsize` carries no `__NIJI_D_struct_winsize` guard, so `@@emit`'s own definition (visibility.tsv
shows `struct winsize` in all three headers) became a second definition. P5 and P1 never saw it: their include path is
libniji's tree, the compiler's and a directory of links to the kernel's `linux`, `asm`, `asm-generic` only.

## Fix (data only: templates and slice 15's check module)

* The three templates drop the `__has_include` block. Each still emits every name visibility.tsv gives it, so nothing
  a program sees from libniji's tree changes (P5/P1 compile the same text). When slice 7 generates `<sys/ioctl.h>`,
  these headers include it unconditionally (template comments say so).
* `spec/tools/header_checks/s15.py` gains `problems(ctx)`: a slice 15 template may include only a header libniji
  generates (`headers/<name>.in`), a kernel UAPI header (`linux/`, `asm/`, `asm-generic/`) or a compiler header. P8
  has a pair for sys/mount.h only; this check is what covers sys/mtio.h and sys/raw.h.
* Not chosen: guarding the emission with `!__has_include(<sys/ioctl.h>)`. It would still pull another libc's header
  into libniji's, and it would hide the names when the found header is not that libc's either.
* Not chosen: dropping `struct winsize` from sys/mount.h. visibility.tsv shows it there, so a program would lose a name.

## Results (mei, role s1)

| check | before (origin/next 8e34f9c72) | after |
|---|---|---|
| header_check whole tree | 8 failed of 11659 (all P8 sys/mount.h + linux/mount.h, 4 modes x 2 orders, `redefinition of 'struct winsize'`) | 0 failed of 11659 (`headers-winsize-twice-hc-final`, also `headers-gen --check` ok) |
| each header alone, host include path after libniji's, C17/C17+GNU/gnu17/C++17 (scratch probe `p5c89.sh`) | 12 of 12 failed (mount, mtio, raw: struct winsize redefined) | 12 of 12 pass |
| P5 under `-std=c89 -pedantic-errors` (libniji tree as `-I`), 3 modes x 3 headers | 3 failed | the same 3 failed (untouched headers, INFORMATIONAL below) |
| plants (`headers-winsize-twice-plants`) | - | 2 of 2 CAUGHT (ws-mount-ioctl by P8, ws-mtio-ioctl by the s15 include check); no WARN |
| clippy `--workspace --all-targets --all-features -D warnings`, `cargo xtask static` | - | clean |

`cargo xtask affected --list --base origin/next`: `FULL: the train's` (generated headers changed), 0 test targets
selected; the gate is precheck plus header_check above.

## INFORMATIONAL (outside this change, the same on origin/next)

* `-std=c89 -pedantic-errors -D_GNU_SOURCE`: `<sys/mount.h>` fails in `fcntl.h` (zero-size array `f_handle` of
  `struct file_handle`).
* `-std=c89 -pedantic-errors` with `-D_GNU_SOURCE` or `-D_DEFAULT_SOURCE`: `<sys/mtio.h>` fails in `sys/types.h:155`
  (`long long`, -Wlong-long).
* I did not probe whether the reference compiles these units, so whether they are defects is open. P5's c89
  `-pedantic-errors` clause (plan/headers.md section 3) is in no header_check unit.

## Open / next

* Decision record: none written (budget). If wanted, `ops reserve --story HEADERS-WINSIZE-TWICE D-<headers section>`
  for one line: "a generated header never includes a header libniji does not ship (no `__has_include` fallback to
  another libc); checked per slice 15 by header_checks/s15.py".
* The include check could cover every slice (it lives in s15.py because check modules are per slice; moving it into
  header_check.py is a generator change).
* SL-HEADERS-S12-6 and HEADERS-GEN-S10-HOOKS's deferred header_check.py plants can now run (the baseline is green).

Plan rows to read: plan/headers.md sections 3 (P5, P8) and 4 ("Adding a slice").
