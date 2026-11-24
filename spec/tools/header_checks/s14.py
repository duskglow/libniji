"""Slice 14 (plan/headers.md section 4): the checks only its headers need.

* `p1_skip`: rows of visibility.tsv whose P1 unit would assert what libniji's header does not show by design.
  macros.tsv gives each name one owning header, the first in name order that shows it; for these three that is
  aio.h, so P1 asserts them in aio.h alone (header_check.py: reserved names only in their owner).
  - `_SIGSET_NWORDS`, `_STRUCT_TIMESPEC`: headers/reserved.tsv `omit` (an internal size constant and an internal
    guard of the reference): no libniji header defines them, aio.h included.
  - `_POSIX_C_SOURCE`: its value is the configuration's (set from the feature-test macros the program defines), so
    the one macros.tsv value (200809) is not the value under `_XOPEN_SOURCE` 500 and 600 (P1 failed there).
Loaded by spec/tools/header_check.py (plan/headers.md section 4, "Adding a slice").
"""

p1_skip = [("aio.h", "_SIGSET_NWORDS"), ("aio.h", "_STRUCT_TIMESPEC"), ("aio.h", "_POSIX_C_SOURCE")]
