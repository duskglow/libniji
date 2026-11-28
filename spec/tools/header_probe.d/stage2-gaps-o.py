"""header_probe.py candidates: stage2-gaps-o (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def stage2_gaps_o_names():
    """(names, tags) of HEADERS-STAGE2-GAPS-O (Yohane stage 2, gcc-14's libgo: mksysinfo.sh copies every constant and
    struct size of gcc's -fdump-go-spec dump of sysinfo.c): the public names the reference's headers define and no
    table held, found as the names (only) of a Go-dump sweep of every public header in six feature-macro sets. The
    struct types shipped so far are candidates both ways (ordinary name and tag); the other types of the sweep are
    listed in handoff/HEADERS-STAGE2-GAPS-O.md. The probe settles which of these the
    reference defines, where, of which kind and in which configurations."""
    names = [
        "AC", "ASH_CHANNEL_ANY", "ASH_CHANNEL_CONTROL", "ASH_CHANNEL_REALTIME", "ATPORT_FIRST", "ATPORT_LAST",
        "ATPORT_RESERVED", "AX25_IPMAXQUEUE", "DDP_MAXHOPS", "DDP_MAXSZ", "EXTENDED_SAP", "FSTAB", "FSTAB_RO",
        "FSTAB_RQ", "FSTAB_RW", "FSTAB_SW", "FSTAB_XX", "GSHADOW", "ICMPV6_EXT_ECHO_REPLY",
        "ICMPV6_EXT_ECHO_REQUEST", "ICMP_AFI_IP", "ICMP_AFI_IP6", "ICMP_EXT_CODE_MAL_QUERY",
        "ICMP_EXT_CODE_MULT_IFS", "ICMP_EXT_CODE_NO_IF", "ICMP_EXT_CODE_NO_TABLE_ENT", "ICMP_EXT_ECHOREPLY_ACTIVE",
        "ICMP_EXT_ECHOREPLY_IPV4", "ICMP_EXT_ECHOREPLY_IPV6", "ICMP_EXT_ECHO_CTYPE_ADDR",
        "ICMP_EXT_ECHO_CTYPE_INDEX", "ICMP_EXT_ECHO_CTYPE_NAME", "IGMP_MRDISC_ADV", "IP6OPT_TYPE_MUTABLE",
        "IPPORT_BIFFUDP", "IPPORT_CMDSERVER", "IPPORT_DAYTIME", "IPPORT_DISCARD", "IPPORT_ECHO", "IPPORT_EFSSERVER",
        "IPPORT_EXECSERVER", "IPPORT_FINGER", "IPPORT_FTP", "IPPORT_LOGINSERVER", "IPPORT_MTP", "IPPORT_NAMESERVER",
        "IPPORT_NETSTAT", "IPPORT_RJE", "IPPORT_ROUTESERVER", "IPPORT_SMTP", "IPPORT_SUPDUP", "IPPORT_SYSTAT",
        "IPPORT_TELNET", "IPPORT_TFTP", "IPPORT_TIMESERVER", "IPPORT_TTYLINK", "IPPORT_USERRESERVED",
        "IPPORT_WHOIS", "IPPORT_WHOSERVER", "IPTOS_DSCP_VA", "LA_ACT_ADD", "LA_ACT_CONSISTENT", "LA_ACT_DELETE",
        "LA_FLG_BINDFROM", "LA_FLG_BINDTO", "LA_SER_CONFIG", "LA_SER_DEFAULT", "LA_SER_LIBPATH", "LA_SER_ORIG",
        "LA_SER_RUNPATH", "LA_SER_SECURE", "LA_SYMB_ALTVALUE", "LA_SYMB_DLSYM", "LA_SYMB_NOPLTENTER",
        "LA_SYMB_NOPLTEXIT", "LA_SYMB_STRUCTCALL", "LINK_MAX", "LLC_FRAME", "ND_OPT_RTR_ADV_INTERVAL",
        "NETROM_PACLEN", "NOTE_GNU_PROPERTY_SECTION_NAME", "NR_OPEN", "PERSISTENT_RESERVE_IN",
        "PERSISTENT_RESERVE_OUT", "PRIB16", "PRIB32", "PRIB8", "PRIBFAST8", "PRIBLEAST16", "PRIBLEAST32",
        "PRIBLEAST8", "PRIb16", "PRIb32", "PRIb8", "PRIbFAST8", "PRIbLEAST16", "PRIbLEAST32", "PRIbLEAST8",
        "PROTO_DNA_RT", "PROTO_IPX", "RELEASE_10", "RESERVE_10", "RTCF_LOG", "RTCF_VALVE", "RTMSG_AR_FAILED",
        "RTMSG_CONTROL", "RTMSG_DELRULE", "RTMSG_NEWRULE", "RTMSG_OVERRUN", "RT_ADD", "RT_CLASS_DEFAULT",
        "RT_CLASS_LOCAL", "RT_CLASS_MAIN", "RT_CLASS_MAX", "RT_CLASS_UNSPEC", "RT_CONSISTENT", "RT_DELETE",
        "SCM_IUCV_TRGCLS", "SCNb16", "SCNb32", "SCNb8", "SCNbFAST8", "SCNbLEAST16", "SCNbLEAST32", "SCNbLEAST8",
        "SC_ESCAPED", "SC_FLUSH", "SC_VJ_RESET", "SG_BIG_BUFF", "SHAPER_GET_DEV", "SHAPER_GET_SPEED",
        "SHAPER_MAXSLIP", "SHAPER_QLEN", "SHAPER_SET_DEV", "SHAPER_SET_SPEED", "SOCK_BUF_LOCK_MASK",
        "SOCK_RCVBUF_LOCK", "SOCK_SNDBUF_LOCK", "SOCK_TXREHASH_DEFAULT", "SOCK_TXREHASH_DISABLED",
        "SOCK_TXREHASH_ENABLED", "SO_IPRMDATA_MSG", "SO_MSGLIMIT", "SO_MSGSIZE", "TR_ALEN", "TR_MAXRIFLEN",
        "TR_RCF_BROADCAST", "TR_RCF_BROADCAST_MASK", "TR_RCF_DIR_BIT", "TR_RCF_FRAME2K", "TR_RCF_LEN_MASK",
        "TR_RCF_LIMITED_BROADCAST", "TR_RII", "UI_CMD", "Val_GNU_MIPS_ABI_FP_64", "Val_GNU_MIPS_ABI_FP_64A",
        "Val_GNU_MIPS_ABI_FP_ANY", "Val_GNU_MIPS_ABI_FP_DOUBLE", "Val_GNU_MIPS_ABI_FP_MAX",
        "Val_GNU_MIPS_ABI_FP_OLD_64", "Val_GNU_MIPS_ABI_FP_SINGLE", "Val_GNU_MIPS_ABI_FP_SOFT",
        "Val_GNU_MIPS_ABI_FP_XX", "WRDE_NOSYS"
    ]
    types = [
        "group_filter", "group_req", "ip_msfilter", "ip_opts", "timestamp",
    ]
    return names + types, types


def candidates():
    return stage2_gaps_o_names()
