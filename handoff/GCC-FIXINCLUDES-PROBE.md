# GCC-FIXINCLUDES-PROBE (handoff)

State: branch `task/gcc-fixincludes-probe`, report only (no header or code change). Full report:
`handoff/GCC-FIXINCLUDES-PROBE-report.md`. Scripts: `handoff/gcc-fixincludes-probe/`.

Answer: fixincludes is a legacy gcc mechanism. It also rewrites stock glibc's `pthread.h`; upstream gcc turns it off
for musl, and Arch and Void turn it off entirely. libniji's headers draw four real rewrites (stdio.h, wchar.h,
stdlib.h, unistd.h). The `va_list` break comes from gcc's `stdio_va_list` sed together with libniji's
`_VA_LIST_DEFINED` guard, which gcc's `<stdarg.h>` honours. Every snapshot that ships headers (24.2 onwards) is
affected. Stage 1 never saw libniji headers (21.4 ships none).

Decisions in force: none recorded (research only).

RULING NEEDED (owner): which option.
- (1) libniji's generator emits `va_list`/`size_t`/`wchar_t` through the compiler's `__need_*` protocol. Tested:
  0 rewrites, same compile results.
- (2) Aqours adds `--disable-fixincludes`.
- (3) Upstream gcc.

Recommend (1), with (2) as a cheap extra.

Not to redo:
- Builds and outputs are on mei under `~/work/gcc-fixincludes-probe/` (build/, trees/, out-v2/, variant/).
- Logs: `~/logs/gcc-fixincludes-probe-{build2,trees-v2,compile2,distro}.log`.

Next step if (1) is ruled: change headers/stdio.h.in:20-27, headers/wchar.h.d/020-va-list.frag and printf.h.in:14-16,
plus the generator's size_t/wchar_t typedef emission (xtask/src/headers.rs guard/emit, around :654) for stdlib.h,
unistd.h and sys/types.h. Regenerate with `cargo xtask headers-gen`, and add a test that runs fixincl over the
installed tree and expects no "Fixed:" lines (scripts here).

Plan rows to read: none specific.
