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


def units(ctx):
    """Spec clause: syslog(3) (the priority is a facility or'ed with a level; HEADERS-STAGE2-GAPS-C, eudev): LOG_MAKEPRI
    ors an already shifted facility with a level (LOG_USER | LOG_ERR = 11), LOG_PRI keeps the level bits and LOG_FAC
    the facility number; each argument is parenthesised (`?:` and `|` bind looser than `|` and `&`), the results have
    the promoted argument's type, and they work in `#if`."""
    body = ("#include <syslog.h>\n"
            "#if LOG_MAKEPRI (LOG_USER, LOG_ERR) != 11 || LOG_PRI (0x10 | 0x0b) != 3 || LOG_FAC (0x400 | 8) != 1\n"
            "#error \"LOG_MAKEPRI/LOG_PRI/LOG_FAC in #if\"\n#endif\n"
            "typedef char niji_s14_log[LOG_MAKEPRI (1 ? LOG_USER : 0, LOG_ERR) == 11 && LOG_FAC (LOG_USER | LOG_ERR) == 1"
            " && LOG_FAC (LOG_MAKEPRI (LOG_LOCAL7, LOG_DEBUG)) == 23 && LOG_PRI (LOG_MAIL | LOG_WARNING) == LOG_WARNING"
            " && sizeof (LOG_PRI (7L)) == sizeof (long) && sizeof (LOG_FAC (8L)) == sizeof (long) ? 1 : -1];\n"
            "int niji_s14_tu;\n")
    return [(cid, "S14 syslog.h priority macros", body) for cid in ("c89", "gnu17", "cxx98", "cxx17")]
