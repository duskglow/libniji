"""header_probe.py candidates: stage2-gaps-c (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""
from header_probe import public_headers, guard_candidates  # noqa: F401


def stage2_names():
    """(names, tags) of HEADERS-STAGE2-GAPS-C (Yohane stage 2: coreutils with gmp, bzip2, cmake, elfutils, bsd-mailx,
    capnproto): the include guards of every public header (guard_candidates); the archive header of ar(5) (`struct
    ar_hdr`, ARMAG, SARMAG, ARFMAG); the BSD default path macros (bsd-mailx names `_PATH_DEVNULL`; the rest of the
    family the BSD paths(3) set and the FHS name); the d_type conversions `DTTOIF` and `IFTODT` (capnproto);
    `struct utimbuf` of XSH <utime.h> (bzip2, cmake)."""
    out = [g for h in public_headers() for g in guard_candidates(h)]
    out += ["ARMAG", "SARMAG", "ARFMAG", "DTTOIF", "IFTODT"]
    # Coordinator note (Yohane 12:25Z): C11's `static_assert` of <assert.h> (elogind), the syslog(3) priority macros
    # (eudev); the mount-option and file-system-type names of getmntent(3) / fstab(5) (busybox's swaponoff).
    out += ["static_assert", "LOG_PRI", "LOG_MAKEPRI", "LOG_FAC", "LOG_PRIMASK", "LOG_FACMASK", "LOG_NFACILITIES",
            "LOG_MASK", "LOG_UPTO"]
    out += [f"MNTTYPE_{t}" for t in ("IGNORE", "NFS", "SWAP")]
    out += [f"MNTOPT_{o}" for o in ("DEFAULTS", "RO", "RW", "SUID", "NOSUID", "NOAUTO", "AUTO", "EXEC", "NOEXEC",
                                    "DEV", "NODEV", "USER", "NOUSER", "SYNC", "ASYNC")]
    out += [f"_PATH_{p}" for p in (
        "CONSOLE", "CSHELL", "DEVDB", "DEVNULL", "DEVZERO", "DRUM", "HEQUIV", "KMEM", "LASTLOG", "MAILDIR", "MAN", "MEM",
        "NOLOGIN", "PRESERVE", "RWHODIR", "SENDMAIL", "SHELLS", "STDPATH", "UNIX", "VI", "VARDB", "VARRUN", "VARTMP",
        "VARLOG", "VARSPOOL", "BOOT", "CP", "ECHO", "MV", "NOLOGIN_TXT", "RSH", "SH", "TMPDIR", "AUDIT", "HOSTS",
        "NETWORKS", "PROTOCOLS", "RESCONF", "LOCALE", "ZONEINFO", "CONSOLE_DEV", "ETC", "DEVDB_DIR", "MKDIR")]
    return out, ["ar_hdr", "utimbuf"]


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return stage2_names()
