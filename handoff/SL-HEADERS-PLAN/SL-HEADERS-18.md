# SL-HEADERS-18: fortify

Id: SL-HEADERS-18
Kind: code
Priority: 90
Size: M
Host: -
Stacks-on: SL-HEADERS-4, -6, -7, -10, -12
Model: opus
Effort: high
BUDGET: 90 tool calls / 180k

Proposed by SL-HEADERS-PLAN (plan/headers.md §4, slice 18; owner ruling R4; pending decision
records/pending/SL-HEADERS-PLAN-1.toml).

## The change

`_FORTIFY_SOURCE` levels 1 to 3 in the landed headers. The wrappers use GCC's object-size builtins (GCC manual, Object Size Checking) and the `_chk` argument shapes of Appendix C.9. Read Debian's default level from `dpkg-buildflags` on the excalibur builder and record it.

## Proof

P3 at -O2 for every configuration in `callsites.tsv`, with every `*_chk` export reached. A deliberate overflow aborts as the spec says. P7 built with `dpkg-buildflags` CPPFLAGS and CFLAGS. Plants: one wrapper removed, and one size argument off by one.

## Rules

Standing rules of rules/thread_common.txt apply. Clean room: never read the reference's header text in
any form. Probes read only diagnostics that name the probe's own identifiers, and the probe's own
objects (plan/headers.md §2.2). ABI facts reach a header only through the `spec/abi` tables. No lab host
names outside handoff/. Source compatibility is best effort (owner, 2026-11-14; plan/headers.md §0): the ABI of what is built is exact (P1, P3), but a package that fails to compile only because it relies on a glibc-internal name gets a small source change (an Aqours patch, listed in your results), not a new glibc-internal name. `__LIELLA__` is always defined.

JOBS: gate, plants
