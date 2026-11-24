---
section: changes
---
- **`gencat` in `libc-dev-bin` (GitHub issue #12):** `/usr/bin/gencat` compiles message-catalog sources into the
  catalogs `catopen` and `catgets` read. For the same source its catalog is byte-identical to glibc's `gencat`'s, so
  catalogs built with either are interchangeable; the options, the `-H` header and the diagnostics match too
  (`validation/transcripts/gencat/`).
