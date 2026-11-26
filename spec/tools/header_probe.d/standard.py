"""header_probe.py candidates: standard (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""
from header_probe import tsv  # noqa: F401


def standard_names():
    """Names that standards and man pages give headers whose names no other table holds (slice 17): the POSIX
    <tar.h> and <cpio.h> constants (XBD 13), the exit codes of sysexits(3), the queue macros of list(3), slist(3),
    stailq(3), tailq(3) and circleq(3), and the obstack macros of the GNU C Library manual (Obstacks). Also (slice 8): names the standards give that no other table spells: the integer type of the signal-handler flag
    (`sig_atomic_t`, C 7.14 and POSIX <signal.h>), the notification structure's type name (`sigevent_t`,
    POSIX <signal.h>), and the mutex kinds of pthread_mutexattr_settype (POSIX: NORMAL, ERRORCHECK, RECURSIVE,
    DEFAULT) and the GNU timed kind `PTHREAD_MUTEX_TIMED_NP`, the name the layout notes of the pthread
    types use for kind 0 (crates/abi/src/layout_init.rs). Also the other pthread attribute values that are
    enumeration constants, which constants.tsv (macros only) cannot hold: the priority protocols
    (pthread_mutexattr_setprotocol(3): PTHREAD_PRIO_NONE, _INHERIT, _PROTECT), the robustness values
    (pthread_mutexattr_setrobust(3): PTHREAD_MUTEX_STALLED, _ROBUST and their `_NP` spellings) and the lock
    preference kinds (pthread_rwlockattr_setkind_np(3): PTHREAD_RWLOCK_PREFER_READER_NP, _PREFER_WRITER_NP,
    _PREFER_WRITER_NONRECURSIVE_NP, _DEFAULT_NP)."""
    out = ["TMAGIC", "TMAGLEN", "TVERSION", "TVERSLEN", "REGTYPE", "AREGTYPE", "LNKTYPE", "SYMTYPE", "CHRTYPE",
           "BLKTYPE", "DIRTYPE", "FIFOTYPE", "CONTTYPE", "TSUID", "TSGID", "TSVTX", "TUREAD", "TUWRITE", "TUEXEC",
           "TGREAD", "TGWRITE", "TGEXEC", "TOREAD", "TOWRITE", "TOEXEC", "MAGIC"]
    out += [f"C_I{c}" for c in ("RUSR", "WUSR", "XUSR", "RGRP", "WGRP", "XGRP", "ROTH", "WOTH", "XOTH", "SUID", "SGID",
                                "SVTX", "SDIR", "SFIFO", "SREG", "SBLK", "SCHR", "SCTG", "SLNK", "SSOCK")]
    out += [f"EX_{c}" for c in ("OK", "_BASE", "USAGE", "DATAERR", "NOINPUT", "NOUSER", "NOHOST", "UNAVAILABLE", "SOFTWARE",
                                "OSERR", "OSFILE", "CANTCREAT", "IOERR", "TEMPFAIL", "PROTOCOL", "NOPERM", "CONFIG", "_MAX")]
    common = ["ENTRY", "EMPTY", "FIRST", "NEXT", "HEAD", "HEAD_INITIALIZER", "INIT", "FOREACH", "REMOVE"]
    more = {"LIST": ["INSERT_AFTER", "INSERT_BEFORE", "INSERT_HEAD"],
            "SLIST": ["INSERT_AFTER", "INSERT_HEAD", "REMOVE_AFTER", "REMOVE_HEAD"],
            "STAILQ": ["CONCAT", "INSERT_AFTER", "INSERT_HEAD", "INSERT_TAIL", "LAST", "REMOVE_AFTER", "REMOVE_HEAD"],
            "TAILQ": ["CONCAT", "FOREACH_REVERSE", "INSERT_AFTER", "INSERT_BEFORE", "INSERT_HEAD", "INSERT_TAIL", "LAST", "PREV"],
            "CIRCLEQ": ["FOREACH_REVERSE", "INSERT_AFTER", "INSERT_BEFORE", "INSERT_HEAD", "INSERT_TAIL", "LAST",
                        "LOOP_NEXT", "LOOP_PREV", "PREV"]}
    for q, ms in more.items():
        out += [f"{q}_{m}" for m in common + ms]
    out += ["SLIST_FOREACH_SAFE", "LIST_FOREACH_SAFE", "STAILQ_FOREACH_SAFE", "TAILQ_FOREACH_SAFE", "CIRCLEQ_FOREACH_SAFE"]
    out += [f"obstack_{m}" for m in ("init", "alloc", "copy", "copy0", "free", "blank", "grow", "grow0", "1grow", "ptr_grow",
                                     "int_grow", "finish", "object_size", "room", "make_room", "blank_fast", "1grow_fast",
                                     "ptr_grow_fast", "int_grow_fast", "base", "next_free", "chunk_size",
                                     "alignment_mask", "chunk_alloc", "chunk_free")]
    out +=["sig_atomic_t", "sigevent_t", "PTHREAD_MUTEX_NORMAL", "PTHREAD_MUTEX_RECURSIVE",
            "PTHREAD_MUTEX_ERRORCHECK", "PTHREAD_MUTEX_DEFAULT", "PTHREAD_MUTEX_TIMED_NP",
            "PTHREAD_PRIO_NONE", "PTHREAD_PRIO_INHERIT", "PTHREAD_PRIO_PROTECT",
            "PTHREAD_MUTEX_STALLED", "PTHREAD_MUTEX_ROBUST", "PTHREAD_MUTEX_STALLED_NP", "PTHREAD_MUTEX_ROBUST_NP",
            "PTHREAD_RWLOCK_PREFER_READER_NP", "PTHREAD_RWLOCK_PREFER_WRITER_NP",
            "PTHREAD_RWLOCK_PREFER_WRITER_NONRECURSIVE_NP", "PTHREAD_RWLOCK_DEFAULT_NP"]
    return out


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return standard_names(), ()
