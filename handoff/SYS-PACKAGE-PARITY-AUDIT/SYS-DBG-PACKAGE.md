# SYS-DBG-PACKAGE: a libc6-dbg package from the same build (audit row 12)

Size: S. Kind: code (packaging). From plan/package-parity-audit.md.

Yohane Linux asked for it; trixie has 1 binary package that Depends on libc6-dbg and 4 source packages that
Build-Depend on it (`consumers-packages.tsv`, exact). The reference package provides `libc-dbg`, `libc6-dbgsym`
(`ref-control.tsv:154`) and holds detached debug files under `/usr/lib/debug/.build-id/` (`ref-files.tsv`, names
only).

1. Keep the debug information of the system build (build-id notes on every shared object and program), split it
   with objcopy into `.build-id/xx/yyyy.debug` files, and package them as `libc6-dbg` (`Depends: libc6 (= version)`,
   `Provides: libc-dbg, libc6-dbgsym`, `Multi-Arch: same`).
2. The stripped libc6 stays byte-identical in the parts the products checks pin (layout contracts).

Proof: gdb in the swapped root shows source lines for a libc function with libc6-dbg installed; build twice gives
identical .debs; check-system.sh checks the fields; plants.
