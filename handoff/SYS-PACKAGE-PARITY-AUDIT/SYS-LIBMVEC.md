# SYS-LIBMVEC: ship libmvec.so.1 (audit row 2; the ledger's M-0/M-1, re-prioritised)

Size: M. Kind: code. From plan/package-parity-audit.md.

Evidence: 18 of 4000 sampled trixie packages (about 104 archive-wide) have `NEEDED libmvec.so.1`
(`consumers-users.tsv`: guitarix, splash, xfoil, raster3d, …) and 12 more depend on those. They cannot start on a
libniji system. The reference exports 216 symbols at version nodes up to GLIBC_2.35 (`ref-dynsyms.tsv`, soname
`libmvec.so.1`; names and versions only).

1. `libmvec.so.1` with every (symbol, version) pair the reference exports, under the x86-64 vector function ABI
   (lane layout and masking from the published vector function ABI document). A first version may compute each lane
   with libniji's scalar function: the accuracy target is the vector ABI's documented bound, and libniji's scalar
   functions are tighter. Faster vector kernels are later work with their own measurements.
2. `libmvec.so` and `libmvec.a` in libc6-dev (`ref-files.tsv:1981`).
3. Ruling R4 stays as it is: libniji's headers declare no `simd` variants, so rebuilt packages do not import it.

Proof: each pair exported (readelf against the table); a test per vector width calling through the ABI against the
scalar results; two of the sampled Debian binaries above run in the swapped root; plants.
