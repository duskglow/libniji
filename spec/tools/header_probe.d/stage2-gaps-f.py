"""header_probe.py candidates: stage2-gaps-f (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def stage2f_names():
    """Names stage 2 package builds against libniji's headers failed on (HEADERS-STAGE2-GAPS-F: util-linux's
    sys-utils/hwclock-cmos.c, inetutils' libtelnet/auth.c and libtelnet/encrypt.c), with their families: the x86 port
    I/O functions of outb(2) (byte, word and long, their pausing `_p` forms and the string forms), and the name
    tables of the TELNET options, commands, line-mode special characters, authentication types and encryption types
    with their range and lookup macros. A candidate is only a name to test: which header shows it, and its kind, are
    probed."""
    out = [f"{d}{w}{p}" for d in ("in", "out") for w in "bwl" for p in ("", "_p")]
    out += [f"{d}s{w}" for d in ("in", "out") for w in "bwl"]
    for m in ("AUTHTYPE_NAME", "ENCTYPE_NAME", "ENCRYPT_NAME", "SLC_NAME", "TELCMD", "TELOPT"):
        out += [m, m + "_OK"]
    out += ["TELOPT_FIRST", "TELOPT_LAST", "NTELOPTS", "SLC_NAMELIST", "SLC_NAMES", "authtype_names", "enctype_names",
            "encrypt_names", "slc_names", "telcmds", "telopts"]
    # The reserved scalar type names of <sys/types.h> no other table spells (util-linux and others name __int8_t ..
    # __uint64_t): the exact-width and least-width integers and the remaining system-type spellings.
    out += [f"__{u}int{w}_t" for u in ("", "u") for w in (8, 16, 32, 64)]
    out += [f"__{u}int_least{w}_t" for u in ("", "u") for w in (8, 16, 32, 64)]
    out += ["__intptr_t", "__loff_t", "__rlim_t", "__rlim64_t", "__timer_t", "__sig_atomic_t", "__suseconds64_t"]
    return out


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return stage2f_names(), ()
