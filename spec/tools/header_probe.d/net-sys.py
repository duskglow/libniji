"""header_probe.py candidates: net-sys (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def net_sys_names():
    """(names, tags) the rest of a static busybox build against libniji's headers failed on (HEADERS-NET-SYS-NAMES:
    networking/ftpd.c, nbd-client.c, ping.c, traceroute.c, route.c, zcip.c, udhcp/d6_*.c, sysklogd/*.c,
    util-linux/eject.c, ipcs.c, setarch.c, swaponoff.c), with their families. Each group names its source; a candidate
    is only a name to test: which header shows it, and its form, type and value, are probed.
    * TCP (netinet/tcp.h): the socket options of tcp(7) and the kernel UAPI header linux/tcp.h, the header flags and
      option kinds and lengths of RFC 9293 3.1-3.2, RFC 7323 (window scale, timestamps) and RFC 2018 (SACK), the
      connection states of RFC 9293 3.3.2, the tcp_info option bits of linux/tcp.h.
    * The member names of struct icmp (busybox ping.c and traceroute.c, the fields of RFC 792, RFC 1191 and RFC 1256),
      struct icmp6_hdr, struct ip6_hdr, struct mld_hdr and the neighbour discovery messages (RFC 3542 2.1, 2.2 and 3),
      struct ether_arp (RFC 826, busybox zcip.c) and struct rtentry (busybox route.c's `rt_mss`); the IPv6 extension
      headers, options and fragment bits of RFC 3542 2.2 and RFC 8200 4.
    * syslog.h under SYSLOG_NAMES (busybox logger.c and syslogd.c): the name tables and their entry type, the
      internal priority and mark values; the priority and facility encoders of syslog(3).
    * SCSI (scsi/scsi.h, scsi/sg.h): the operation codes, status, sense keys, message codes and peripheral device types
      of the T10 SCSI Primary and Block Commands, the kernel SCSI ioctls (busybox eject.c's ALLOW_MEDIUM_REMOVAL and
      START_STOP), and the SCSI generic interface of sg(4) and the Linux SCSI Generic HOWTO (sg_io_hdr_t, SG_*).
    * System V IPC (sys/msg.h, sys/shm.h, sys/sem.h): the IPC_INFO structures and names of msgctl(2), shmctl(2) and
      semctl(2) (busybox ipcs.c: msginfo, shminfo, shm_info, seminfo, msg_cbytes).
    * sys/personality.h: the execution domains and flags of personality(2) and the kernel UAPI header
      linux/personality.h (busybox setarch.c's PER_LINUX32).
    * mntent.h: the mount option and type names of getmntent(3) and fstab(5) (busybox swaponoff.c's MNTOPT_NOAUTO)."""
    tcp = ["NODELAY", "MAXSEG", "CORK", "KEEPIDLE", "KEEPINTVL", "KEEPCNT", "SYNCNT", "LINGER2", "DEFER_ACCEPT",
           "WINDOW_CLAMP", "INFO", "QUICKACK", "CONGESTION", "MD5SIG", "COOKIE_TRANSACTIONS", "THIN_LINEAR_TIMEOUTS",
           "THIN_DUPACK", "USER_TIMEOUT", "REPAIR", "REPAIR_QUEUE", "QUEUE_SEQ", "REPAIR_OPTIONS", "FASTOPEN",
           "TIMESTAMP", "NOTSENT_LOWAT", "CC_INFO", "SAVE_SYN", "SAVED_SYN", "REPAIR_WINDOW", "FASTOPEN_CONNECT", "ULP",
           "MD5SIG_EXT", "FASTOPEN_KEY", "FASTOPEN_NO_COOKIE", "ZEROCOPY_RECEIVE", "INQ", "CM_INQ", "TX_DELAY",
           "AO_ADD_KEY", "AO_DEL_KEY", "AO_INFO", "AO_GET_KEYS", "AO_REPAIR", "REPAIR_ON", "REPAIR_OFF",
           "REPAIR_OFF_NO_WP", "NO_QUEUE", "RECV_QUEUE", "SEND_QUEUE", "QUEUES_NR", "MSS", "MAXWIN", "MAX_WINSHIFT",
           "MSS_DEFAULT", "MSS_DESIRED", "MD5SIG_MAXKEYLEN", "MD5SIG_FLAG_PREFIX", "MD5SIG_FLAG_IFINDEX",
           "COOKIE_MIN", "COOKIE_MAX", "COOKIE_PAIR_SIZE", "COOKIE_IN_ALWAYS", "COOKIE_OUT_NEVER", "S_DATA_IN",
           "S_DATA_OUT", "ESTABLISHED", "SYN_SENT", "SYN_RECV", "FIN_WAIT1", "FIN_WAIT2", "TIME_WAIT", "CLOSE",
           "CLOSE_WAIT", "LAST_ACK", "LISTEN", "CLOSING", "CA_Open", "CA_Disorder", "CA_CWR", "CA_Recovery", "CA_Loss"]
    out = [f"TCP_{t}" for t in tcp] + ["SOL_TCP", "TH_FIN", "TH_SYN", "TH_RST", "TH_PUSH", "TH_ACK", "TH_URG"]
    out += [f"TCPOPT_{o}" for o in ("EOL", "NOP", "MAXSEG", "WINDOW", "SACK_PERMITTED", "SACK", "TIMESTAMP",
                                    "TSTAMP_HDR")]
    out += [f"TCPOLEN_{o}" for o in ("MAXSEG", "WINDOW", "SACK_PERMITTED", "TIMESTAMP", "TSTAMP_APPA")]
    out += [f"TCPI_OPT_{o}" for o in ("TIMESTAMPS", "SACK", "WSCALE", "ECN", "ECN_SEEN", "SYN_DATA", "USEC_TS")]
    out += [f"icmp_{m}" for m in ("pptr", "gwaddr", "id", "seq", "void", "pmvoid", "nextmtu", "num_addrs", "wpa",
                                  "lifetime", "otime", "rtime", "ttime", "ip", "radv", "mask", "data")]
    out += [f"icmp6_{m}" for m in ("data32", "data16", "data8", "pptr", "mtu", "id", "seq", "maxdelay")]
    out += [f"mld_{m}" for m in ("type", "code", "cksum", "maxdelay", "reserved")]
    for p, ms in (("nd_rs", ("type", "code", "cksum", "reserved")), ("nd_ns", ("type", "code", "cksum", "reserved")),
                  ("nd_na", ("type", "code", "cksum", "flags_reserved")), ("nd_rd", ("type", "code", "cksum", "reserved")),
                  ("nd_ra", ("type", "code", "cksum", "curhoplimit", "flags_reserved", "router_lifetime")),
                  ("rr", ("type", "code", "cksum", "seqnum")), ("ip6", ("vfc", "flow", "plen", "nxt", "hlim", "hops")),
                  ("arp", ("hrd", "pro", "hln", "pln", "op")), ("rt", ("mss",))):
        out += [f"{p}_{m}" for m in ms]
    out += [f"IP6OPT_{o}" for o in ("PAD1", "PADN", "JUMBO", "NSAP_ADDR", "TUNNEL_LIMIT", "ROUTER_ALERT", "TYPE_SKIP",
                                    "TYPE_DISCARD", "TYPE_FORCEICMP", "TYPE_ICMP", "MUTABLE", "JUMBO_LEN")]
    out += ["IP6_ALERT_MLD", "IP6_ALERT_RSVP", "IP6_ALERT_AN", "IP6F_OFF_MASK", "IP6F_RESERVED_MASK", "IP6F_MORE_FRAG"]
    out += ["CODE", "prioritynames", "facilitynames", "INTERNAL_NOPRI", "INTERNAL_MARK", "LOG_MAKEPRI", "LOG_FAC",
            "LOG_PRI", "LOG_FACMASK", "LOG_PRIMASK", "LOG_NFACILITIES", "SYSLOG_NAMES"]
    out += ["TEST_UNIT_READY", "REZERO_UNIT", "REQUEST_SENSE", "FORMAT_UNIT", "READ_BLOCK_LIMITS", "REASSIGN_BLOCKS",
            "READ_6", "WRITE_6", "SEEK_6", "READ_REVERSE", "WRITE_FILEMARKS", "SPACE", "INQUIRY",
            "RECOVER_BUFFERED_DATA", "MODE_SELECT", "RESERVE", "RELEASE", "COPY", "ERASE", "MODE_SENSE", "START_STOP",
            "RECEIVE_DIAGNOSTIC", "SEND_DIAGNOSTIC", "ALLOW_MEDIUM_REMOVAL", "SET_WINDOW", "READ_CAPACITY", "READ_10",
            "WRITE_10", "SEEK_10", "WRITE_VERIFY", "VERIFY", "SEARCH_HIGH", "SEARCH_EQUAL", "SEARCH_LOW", "SET_LIMITS",
            "PRE_FETCH", "READ_POSITION", "SYNCHRONIZE_CACHE", "LOCK_UNLOCK_CACHE", "READ_DEFECT_DATA", "MEDIUM_SCAN",
            "COMPARE", "COPY_VERIFY", "WRITE_BUFFER", "READ_BUFFER", "UPDATE_BLOCK", "READ_LONG", "WRITE_LONG",
            "CHANGE_DEFINITION", "WRITE_SAME", "READ_TOC", "LOG_SELECT", "LOG_SENSE", "MODE_SELECT_10",
            "MODE_SENSE_10", "MOVE_MEDIUM", "READ_12", "WRITE_12", "WRITE_VERIFY_12", "SEARCH_HIGH_12",
            "SEARCH_EQUAL_12", "SEARCH_LOW_12", "READ_ELEMENT_STATUS", "SEND_VOLUME_TAG", "WRITE_LONG_2", "READ_16",
            "WRITE_16", "SERVICE_ACTION_IN", "REPORT_LUNS", "GOOD", "CHECK_CONDITION", "CONDITION_GOOD", "BUSY",
            "INTERMEDIATE_GOOD", "INTERMEDIATE_C_GOOD", "RESERVATION_CONFLICT", "COMMAND_TERMINATED", "QUEUE_FULL",
            "STATUS_MASK", "NO_SENSE", "RECOVERED_ERROR", "NOT_READY", "MEDIUM_ERROR", "HARDWARE_ERROR",
            "ILLEGAL_REQUEST", "UNIT_ATTENTION", "DATA_PROTECT", "BLANK_CHECK", "COPY_ABORTED", "ABORTED_COMMAND",
            "VOLUME_OVERFLOW", "MISCOMPARE", "TYPE_DISK", "TYPE_TAPE", "TYPE_PROCESSOR", "TYPE_WORM", "TYPE_ROM",
            "TYPE_SCANNER", "TYPE_MOD", "TYPE_MEDIUM_CHANGER", "TYPE_ENCLOSURE", "TYPE_NO_LUN", "COMMAND_COMPLETE",
            "EXTENDED_MESSAGE", "EXTENDED_MODIFY_DATA_POINTER", "EXTENDED_SDTR", "EXTENDED_EXTENDED_IDENTIFY",
            "EXTENDED_WDTR", "SAVE_POINTERS", "RESTORE_POINTERS", "DISCONNECT", "INITIATOR_ERROR", "ABORT",
            "MESSAGE_REJECT", "NOP", "MSG_PARITY_ERROR", "LINKED_CMD_COMPLETE", "LINKED_FLG_CMD_COMPLETE",
            "BUS_DEVICE_RESET", "INITIATE_RECOVERY", "RELEASE_RECOVERY", "SIMPLE_QUEUE_TAG", "HEAD_OF_QUEUE_TAG",
            "ORDERED_QUEUE_TAG", "SCSI_REMOVAL_PREVENT", "SCSI_REMOVAL_ALLOW"]
    out += [f"SCSI_IOCTL_{i}" for i in ("GET_IDLUN", "TAGGED_ENABLE", "TAGGED_DISABLE", "PROBE_HOST", "GET_BUS_NUMBER",
                                        "SEND_COMMAND", "TEST_UNIT_READY", "BENCHMARK_COMMAND", "SYNC", "START_UNIT",
                                        "STOP_UNIT", "DOORLOCK", "DOORUNLOCK", "GET_PCI")]
    out += ["sg_io_hdr_t", "sg_scsi_id_t", "sg_req_info_t", "sg_iovec_t", "Sg_io_hdr", "Sg_scsi_id", "Sg_req_info"]
    out += [f"SG_{s}" for s in (
        "INTERFACE_ID_ORIG", "DXFER_NONE", "DXFER_TO_DEV", "DXFER_FROM_DEV", "DXFER_TO_FROM_DEV", "FLAG_DIRECT_IO",
        "FLAG_LUN_INHIBIT", "FLAG_NO_DXFER", "INFO_OK_MASK", "INFO_OK", "INFO_CHECK", "INFO_DIRECT_IO_MASK",
        "INFO_INDIRECT_IO", "INFO_DIRECT_IO", "INFO_MIXED_IO", "EMULATED_HOST", "SET_TRANSFORM", "GET_TRANSFORM",
        "SET_RESERVED_SIZE", "GET_RESERVED_SIZE", "GET_SCSI_ID", "SET_FORCE_LOW_DMA", "GET_LOW_DMA",
        "SET_FORCE_PACK_ID", "GET_PACK_ID", "GET_NUM_WAITING", "GET_SG_TABLESIZE", "GET_VERSION_NUM", "SCSI_RESET",
        "SCSI_RESET_NOTHING", "SCSI_RESET_DEVICE", "SCSI_RESET_BUS", "SCSI_RESET_HOST", "IO", "GET_REQUEST_TABLE",
        "SET_KEEP_ORPHAN", "GET_KEEP_ORPHAN", "MAX_QUEUE", "SCATTER_SZ", "DEFAULT_RETRIES", "DEF_FORCE_LOW_DMA",
        "DEF_FORCE_PACK_ID", "DEF_KEEP_ORPHAN", "DEF_RESERVED_SIZE", "MAX_SENSE", "SET_TIMEOUT", "GET_TIMEOUT",
        "GET_COMMAND_Q", "SET_COMMAND_Q", "SET_DEBUG", "NEXT_CMD_LEN", "DEFAULT_TIMEOUT", "DEF_COMMAND_Q",
        "DEF_UNDERRUN_FLAG", "GET_ACCESS_COUNT")]
    out += ["MSG_STAT", "MSG_INFO", "MSG_STAT_ANY", "MSG_COPY", "msg_cbytes", "SHM_STAT", "SHM_INFO", "SHM_STAT_ANY",
            "SHM_DEST", "SHM_LOCKED", "SHM_NORESERVE", "SHM_HUGE_SHIFT", "SHM_HUGE_MASK", "SHM_HUGE_2MB",
            "SHM_HUGE_1GB", "SHM_EXEC", "SEM_INFO", "SEM_STAT_ANY", "SEMVMX", "SEMMNI"]
    out += [f"PER_{p}" for p in ("LINUX", "LINUX_32BIT", "LINUX_FDPIC", "SVR4", "SVR3", "SCOSVR3", "OSR5", "WYSEV386",
                                 "ISCR4", "BSD", "SUNOS", "XENIX", "LINUX32", "LINUX32_3GB", "IRIX32", "IRIXN32",
                                 "IRIX64", "RISCOS", "SOLARIS", "UW7", "OSF4", "HPUX", "MASK")]
    out += ["UNAME26", "FDPIC_FUNCPTRS", "MMAP_PAGE_ZERO", "ADDR_COMPAT_LAYOUT", "READ_IMPLIES_EXEC", "ADDR_LIMIT_32BIT",
            "SHORT_INODE", "WHOLE_SECONDS", "STICKY_TIMEOUTS", "ADDR_LIMIT_3GB"]
    out += ["MNTTAB", "MOUNTED", "MNTTYPE_IGNORE", "MNTTYPE_NFS", "MNTTYPE_SWAP", "MNTOPT_DEFAULTS", "MNTOPT_RO",
            "MNTOPT_RW", "MNTOPT_SUID", "MNTOPT_NOSUID", "MNTOPT_NOAUTO"]
    tags = ["_code", "tcp_md5sig", "tcp_repair_opt", "tcp_repair_window", "tcp_cookie_transactions",
            "tcp_zerocopy_receive", "tcp_diag_md5sig", "tcp_info", "ip6_hdrctl", "ip6_ext", "ip6_hbh", "ip6_dest",
            "ip6_rthdr", "ip6_rthdr0", "ip6_frag", "ip6_opt", "ip6_opt_jumbo", "ip6_opt_nsap", "ip6_opt_tunnel",
            "ip6_opt_router", "sg_io_hdr", "sg_scsi_id", "sg_req_info", "sg_iovec", "sg_header", "ccs_modesel_head",
            "msginfo", "shminfo", "shm_info", "seminfo"]
    return out, tags


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return net_sys_names()
