"""header_probe.py candidates: stage2-gaps-g (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def stage2g_names():
    """Names stage 2 package builds against libniji's headers failed on (HEADERS-STAGE2-GAPS-G: systemd, eudev,
    mariadb and libtpms on the byte-order names; libgcc on the real-time signal bounds; glib2.0 on the ptrace
    options; dwarves on the obstack interface; postgresql-17 on the locale case mappings; valgrind on the ELF note
    names; shadow on the shadow file path; sysvinit and bsd-mailx on the control-character macros; nftables on the
    IGMP message types; net-tools on the FDDI sizes), with their families as ptrace(2), elf(5), obstack (the GNU C
    Library manual), RFC 1112 / RFC 2236 and the FDDI frame sizes name them. A candidate is only a name to test:
    which header shows it, and its kind, are probed."""
    out = ["__BYTE_ORDER", "__LITTLE_ENDIAN", "__BIG_ENDIAN", "__PDP_ENDIAN", "__FLOAT_WORD_ORDER",
           "__bswap_16", "__bswap_32", "__bswap_64", "__SIGRTMIN", "__SIGRTMAX", "__toupper_l", "__tolower_l",
           "SHADOW", "CTRL", "CCEQ"]
    out += ["PTRACE_O_" + n for n in ("TRACESYSGOOD", "TRACEFORK", "TRACEVFORK", "TRACECLONE", "TRACEEXEC",
                                      "TRACEVFORKDONE", "TRACEEXIT", "TRACESECCOMP", "EXITKILL", "SUSPEND_SECCOMP",
                                      "MASK")]
    out += ["PTRACE_EVENT_" + n for n in ("FORK", "VFORK", "CLONE", "EXEC", "VFORK_DONE", "EXIT", "SECCOMP", "STOP")]
    out += ["obstack_" + n for n in (
        "init", "begin", "specify_allocation", "specify_allocation_with_arg", "free", "alloc", "copy", "copy0",
        "blank", "grow", "grow0", "1grow", "ptr_grow", "int_grow", "finish", "object_size", "room", "1grow_fast",
        "ptr_grow_fast", "int_grow_fast", "blank_fast", "base", "next_free", "alignment_mask", "chunk_size",
        "empty_p", "make_room", "memory_used", "chunkfun", "freefun", "alloc_failed_handler", "exit_failure")]
    out += ["ELF_NOTE_" + n for n in ("SOLARIS", "GNU", "FDO", "PAGESIZE_HINT", "ABI", "OS_LINUX", "OS_GNU",
                                      "OS_SOLARIS2", "OS_FREEBSD", "GNU_PROPERTY_SECTION_NAME")]
    out += ["NT_GNU_" + n for n in ("ABI_TAG", "HWCAP", "BUILD_ID", "GOLD_VERSION", "PROPERTY_TYPE_0")]
    out += ["IGMP_" + n for n in (
        "MINLEN", "MEMBERSHIP_QUERY", "V1_MEMBERSHIP_REPORT", "V2_MEMBERSHIP_REPORT", "V2_LEAVE_GROUP", "DVMRP",
        "PIM", "TRACE", "MTRACE_RESP", "MTRACE", "MAX_HOST_REPORT_DELAY", "TIMER_SCALE", "DELAYING_MEMBER",
        "IDLE_MEMBER", "LAZY_MEMBER", "SLEEPING_MEMBER", "AWAKENING_MEMBER", "v1_ROUTER", "v2_ROUTER",
        "HOST_MEMBERSHIP_QUERY", "HOST_MEMBERSHIP_REPORT", "HOST_NEW_MEMBERSHIP_REPORT", "HOST_LEAVE_MESSAGE")]
    out += ["FDDI_K_" + n for n in ("ALEN", "8022_HLEN", "SNAP_HLEN", "8022_ZLEN", "SNAP_ZLEN", "8022_DLEN",
                                    "SNAP_DLEN", "LLC_ZLEN", "LLC_LEN", "OUI_LEN")]
    # inetutils ping (icmp.h) on n_time, with the network-order types beside it (RELEASE-SNAPSHOT-1126B).
    out += ["n_short", "n_long", "n_time"]
    return out


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return stage2g_names(), ("fanotify_event_metadata", "ethhdr", "igmp", "fddihdr", "_ftsent64", "fddi_8022_1_hdr",
                             "fddi_8022_2_hdr", "fddi_snap_hdr")
