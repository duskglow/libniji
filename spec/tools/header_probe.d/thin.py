"""header_probe.py candidates: thin (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def thin_names():
    """(names, tags) of the completeness pass over the thin headers (HEADERS-STAGE2-GAPS-C-2: the triage counts of
    non-reserved macros showed these headers with fewer names than the reference system's). Each group names its
    specification; a candidate is only a name to test, which header shows it, its value and its form are probed.
    * utmp.h, lastlog.h, utmpx.h: utmp(5) and utmpx.h of XSH (record types, field sizes, the file-name macros and
      the compatibility member names).
    * sys/queue.h: queue(3) (the list, singly-linked list, simple queue, tail queue and circular queue families).
    * sys/timex.h: adjtimex(2) (the mode bits ADJ_*, the legacy MOD_* names, the status bits STA_*, the clock
      states TIME_*) and the kernel UAPI header linux/timex.h.
    * sys/pidfd.h: pidfd_open(2), pidfd_send_signal(2) and the kernel UAPI header linux/pidfd.h.
    * netinet/udp.h: udp(7) and the kernel UAPI header linux/udp.h (socket options, encapsulation types)."""
    out = ["UT_LINESIZE", "UT_NAMESIZE", "UT_HOSTSIZE", "EMPTY", "RUN_LVL", "BOOT_TIME", "NEW_TIME", "OLD_TIME",
           "INIT_PROCESS", "LOGIN_PROCESS", "USER_PROCESS", "DEAD_PROCESS", "ACCOUNTING", "ut_name", "ut_time",
           "ut_xtime", "ut_addr", "ut_addr_v6", "UTMP_FILE", "UTMP_FILENAME", "WTMP_FILE", "WTMP_FILENAME",
           "UTMPX_FILE", "UTMPX_FILENAME", "WTMPX_FILE", "WTMPX_FILENAME", "_PATH_UTMP", "_PATH_WTMP", "_PATH_UTMPX",
           "_PATH_WTMPX", "_PATH_LASTLOG", "ll_time", "ll_line", "ll_host"]
    ops = {
        "LIST": ["EMPTY", "ENTRY", "FIRST", "FOREACH", "HEAD", "HEAD_INITIALIZER", "INIT", "INSERT_AFTER",
                 "INSERT_BEFORE", "INSERT_HEAD", "NEXT", "REMOVE", "SWAP", "FOREACH_FROM", "FOREACH_SAFE",
                 "FOREACH_FROM_SAFE", "PREV", "LAST"],
        "SLIST": ["EMPTY", "ENTRY", "FIRST", "FOREACH", "HEAD", "HEAD_INITIALIZER", "INIT", "INSERT_AFTER",
                  "INSERT_HEAD", "NEXT", "REMOVE", "REMOVE_AFTER", "REMOVE_HEAD", "FOREACH_SAFE", "FOREACH_PREVPTR",
                  "SWAP", "FOREACH_FROM", "FOREACH_FROM_SAFE"],
        "STAILQ": ["CONCAT", "EMPTY", "ENTRY", "FIRST", "FOREACH", "HEAD", "HEAD_INITIALIZER", "INIT", "INSERT_AFTER",
                   "INSERT_HEAD", "INSERT_TAIL", "LAST", "NEXT", "REMOVE", "REMOVE_AFTER", "REMOVE_HEAD",
                   "FOREACH_SAFE", "SWAP", "FOREACH_FROM", "FOREACH_FROM_SAFE"],
        "TAILQ": ["CONCAT", "EMPTY", "ENTRY", "FIRST", "FOREACH", "FOREACH_REVERSE", "HEAD", "HEAD_INITIALIZER",
                  "INIT", "INSERT_AFTER", "INSERT_BEFORE", "INSERT_HEAD", "INSERT_TAIL", "LAST", "NEXT", "PREV",
                  "REMOVE", "FOREACH_SAFE", "FOREACH_REVERSE_SAFE", "SWAP", "FOREACH_FROM", "FOREACH_FROM_SAFE",
                  "FOREACH_REVERSE_FROM", "FOREACH_REVERSE_FROM_SAFE"],
        "CIRCLEQ": ["EMPTY", "ENTRY", "FIRST", "FOREACH", "FOREACH_REVERSE", "HEAD", "HEAD_INITIALIZER", "INIT",
                    "INSERT_AFTER", "INSERT_BEFORE", "INSERT_HEAD", "INSERT_TAIL", "LAST", "LOOP_NEXT", "LOOP_PREV",
                    "NEXT", "PREV", "REMOVE"],
    }
    out += [f"{fam}_{o}" for fam, os_ in ops.items() for o in os_]
    out += ["QMD_TRACE_ELEM", "TRACEBUF", "TRASHIT", "QUEUE_MACRO_DEBUG"]
    out += ["ADJ_" + n for n in ("OFFSET", "FREQUENCY", "MAXERROR", "ESTERROR", "STATUS", "TIMECONST", "TAI",
                                 "SETOFFSET", "MICRO", "NANO", "TICK", "OFFSET_SINGLESHOT", "OFFSET_SS_READ")]
    out += ["MOD_" + n for n in ("OFFSET", "FREQUENCY", "MAXERROR", "ESTERROR", "STATUS", "TIMECONST", "CLKB",
                                 "CLKA", "TAI", "MICRO", "NANO")]
    out += ["STA_" + n for n in ("PLL", "PPSFREQ", "PPSTIME", "FLL", "INS", "DEL", "UNSYNC", "FREQHOLD", "PPSSIGNAL",
                                 "PPSJITTER", "PPSWANDER", "PPSERROR", "CLOCKERR", "NANO", "MODE", "CLK")]
    out += ["TIME_" + n for n in ("OK", "INS", "DEL", "OOP", "WAIT", "ERROR", "BAD")]
    out += ["NTP_API", "MAXPHASE", "MAXFREQ", "MINSEC", "MAXSEC", "MAXTC", "SHIFT_USEC", "SHIFT_PLL",
            "SHIFT_FLL", "PPS_FAVG", "PPS_FREQ", "PPS_TIME"]
    out += ["PIDFD_" + n for n in ("NONBLOCK", "THREAD", "SIGNAL_THREAD", "SIGNAL_THREAD_GROUP",
                                   "SIGNAL_PROCESS_GROUP", "GET_CGROUP_NAMESPACE", "GET_IPC_NAMESPACE",
                                   "GET_MNT_NAMESPACE", "GET_NET_NAMESPACE", "GET_PID_NAMESPACE",
                                   "GET_PID_FOR_CHILDREN_NAMESPACE", "GET_TIME_NAMESPACE",
                                   "GET_TIME_FOR_CHILDREN_NAMESPACE", "GET_USER_NAMESPACE", "GET_UTS_NAMESPACE",
                                   "GET_INFO", "INFO_PID", "INFO_CREDS", "INFO_CGROUPID", "INFO_EXIT", "INFO_SIZE_VER0",
                                   "SELF", "SELF_THREAD", "SELF_THREAD_GROUP", "SELF_PROCESS", "EXITED")]
    out += ["UDP_" + n for n in ("CORK", "ENCAP", "NO_CHECK6_TX", "NO_CHECK6_RX", "SEGMENT", "GRO", "MAX_SEGMENTS",
                                 "ENCAP_ESPINUDP_NON_IKE", "ENCAP_ESPINUDP", "ENCAP_L2TPINUDP", "ENCAP_GTP0",
                                 "ENCAP_GTP1U", "ENCAP_RXRPC", "ENCAP_ESPINTCP")]
    out += ["SOL_UDP", "SOL_UDPLITE", "UDPLITE_SEND_CSCOV", "UDPLITE_RECV_CSCOV", "UDP_ENCAP_SPECIAL"]
    # arpa/telnet.h: the TELNET commands (RFC 854), the option codes of the IANA TELNET option registry (TELOPT_*, RFCs
    # 856-1184, 1572), the subnegotiation qualifiers and the LINEMODE and flow-control values (RFCs 1073, 1091, 1184).
    out += ["IAC", "DONT", "DO", "WONT", "WILL", "SB", "GA", "EL", "EC", "AYT", "AO", "IP", "BREAK", "DM", "NOP", "SE",
            "EOR", "ABORT", "SUSP", "xEOF", "SYNCH", "TELCMD_FIRST", "TELCMD_LAST", "TELCMD_OK", "TELCMD", "telcmds",
            "TELOPT_FIRST", "TELOPT_LAST", "TELOPT_OK", "TELOPT", "telopts", "TELQUAL_IS", "TELQUAL_SEND",
            "TELQUAL_INFO", "TELQUAL_REPLY", "TELQUAL_NAME", "LFLOW_OFF", "LFLOW_ON", "LFLOW_RESTART_ANY",
            "LFLOW_RESTART_XON", "LM_MODE", "LM_FORWARDMASK", "LM_SLC", "MODE_EDIT", "MODE_TRAPSIG", "MODE_ACK",
            "MODE_SOFT_TAB", "MODE_LIT_ECHO", "MODE_MASK", "MODE_FLOW", "MODE_INBIN", "MODE_OUTBIN", "MODE_FORCE",
            "SLC_SYNCH", "SLC_BRK", "SLC_IP", "SLC_AO", "SLC_AYT", "SLC_EOR", "SLC_ABORT", "SLC_EOF", "SLC_SUSP",
            "SLC_EC", "SLC_EL", "SLC_EW", "SLC_RP", "SLC_LNEXT", "SLC_XON", "SLC_XOFF", "SLC_FORW1", "SLC_FORW2",
            "SLC_MCL", "SLC_MCR", "SLC_MCWL", "SLC_MCWR", "SLC_MCBOL", "SLC_MCEOL", "SLC_INSRT", "SLC_OVER",
            "SLC_ECR", "SLC_EWR", "SLC_EBOL", "SLC_EEOL", "NSLC", "SLC_NOSUPPORT", "SLC_CANTCHANGE", "SLC_VARIABLE",
            "SLC_DEFAULT", "SLC_LEVELBITS", "SLC_FUNC", "SLC_FLAGS", "SLC_VALUE", "SLC_ACK", "SLC_FLUSHIN",
            "SLC_FLUSHOUT", "OLD_ENV_VAR", "OLD_ENV_VALUE", "NEW_ENV_VAR", "NEW_ENV_VALUE", "ENV_ESC", "ENV_USERVAR",
            "AUTH_WHO_CLIENT", "AUTH_WHO_SERVER", "AUTH_WHO_MASK", "AUTH_HOW_ONE_WAY", "AUTH_HOW_MUTUAL",
            "AUTH_HOW_MASK", "AUTHTYPE_NULL", "AUTHTYPE_KERBEROS_V4", "AUTHTYPE_KERBEROS_V5", "AUTHTYPE_SPX",
            "AUTHTYPE_MINK", "AUTHTYPE_CNT", "AUTHTYPE_TEST", "ENCRYPT_IS", "ENCRYPT_SUPPORT", "ENCRYPT_REPLY",
            "ENCRYPT_START", "ENCRYPT_END", "ENCRYPT_REQSTART", "ENCRYPT_REQEND", "ENCRYPT_ENC_KEYID",
            "ENCRYPT_DEC_KEYID", "ENCRYPT_CNT", "ENCTYPE_ANY", "ENCTYPE_DES_CFB64", "ENCTYPE_DES_OFB64",
            "ENCTYPE_CNT"]
    out += ["TELOPT_" + n for n in (
        "BINARY", "ECHO", "RCP", "SGA", "NAMS", "STATUS", "TM", "RCTE", "NAOL", "NAOP", "NAOCRD", "NAOHTS", "NAOHTD",
        "NAOFFD", "NAOVTS", "NAOVTD", "NAOLFD", "XASCII", "LOGOUT", "BM", "DET", "SUPDUP", "SUPDUPOUTPUT", "SNDLOC",
        "TTYPE", "EOR", "TUID", "OUTMRK", "TTYLOC", "3270REGIME", "X3PAD", "NAWS", "TSPEED", "LFLOW", "LINEMODE",
        "XDISPLOC", "OLD_ENVIRON", "AUTHENTICATION", "ENCRYPT", "NEW_ENVIRON", "EXOPL", "ENVIRON")]
    # arpa/nameser.h: the resource-record types, classes, opcodes and response codes of the IANA DNS parameters
    # registry (RFC 1035 3.2.2-3.2.4, 1996, 2136, 2671, 4034, 6891, ...), each spelled by the three conventional
    # prefixes (T_<TYPE>, ns_t_<type>; C_<CLASS>, ns_c_<class>).
    types = ("A NS MD MF CNAME SOA MB MG MR NULL WKS PTR HINFO MINFO MX TXT RP AFSDB X25 ISDN RT NSAP NSAP_PTR SIG KEY "
             "PX GPOS AAAA LOC NXT EID NIMLOC SRV ATMA NAPTR KX CERT A6 DNAME SINK OPT APL DS SSHFP IPSECKEY RRSIG NSEC "
             "DNSKEY DHCID NSEC3 NSEC3PARAM TLSA HIP SPF TKEY TSIG IXFR AXFR MAILB MAILA ANY ZXFR CAA URI DLV").split()
    out += [f"T_{t}" for t in types] + [f"ns_t_{t.lower()}" for t in types] + ["ns_t_invalid", "ns_t_max"]
    classes = "IN CHAOS HS NONE ANY".split()
    out += [f"C_{c}" for c in classes] + [f"ns_c_{c.lower()}" for c in classes] + ["ns_c_2", "ns_c_invalid",
                                                                                  "ns_c_max"]
    out += ["ns_o_query", "ns_o_iquery", "ns_o_status", "ns_o_notify", "ns_o_update", "ns_o_max", "QUERY", "IQUERY",
            "STATUS", "NS_NOTIFY_OP", "NS_UPDATE_OP", "ns_r_noerror", "ns_r_formerr", "ns_r_servfail", "ns_r_nxdomain",
            "ns_r_notimpl", "ns_r_refused", "ns_r_yxdomain", "ns_r_yxrrset", "ns_r_nxrrset", "ns_r_notauth",
            "ns_r_notzone", "ns_r_max", "ns_r_badvers", "ns_r_badsig", "ns_r_badkey", "ns_r_badtime", "NOERROR",
            "FORMERR", "SERVFAIL", "NXDOMAIN", "NOTIMP", "REFUSED", "YXDOMAIN", "YXRRSET", "NXRRSET", "NOTAUTH",
            "NOTZONE", "ns_s_qd", "ns_s_zn", "ns_s_an", "ns_s_pr", "ns_s_ns", "ns_s_ud", "ns_s_ar", "ns_s_max",
            "ns_f_qr", "ns_f_opcode", "ns_f_aa", "ns_f_tc", "ns_f_rd", "ns_f_ra", "ns_f_z", "ns_f_ad", "ns_f_cd",
            "ns_f_rcode", "ns_f_max", "ns_uop_delete", "ns_uop_add", "ns_uop_max", "ns_cert_pkix", "ns_cert_spki",
            "ns_cert_pgp", "ns_cert_url", "ns_cert_oid", "ns_kt_rsa", "ns_kt_dh", "ns_kt_dsa", "ns_kt_private",
            "NS_PACKETSZ", "NS_MAXDNAME", "NS_MAXCDNAME", "NS_MAXLABEL", "NS_HFIXEDSZ", "NS_QFIXEDSZ", "NS_RRFIXEDSZ",
            "NS_INT32SZ", "NS_INT16SZ", "NS_INT8SZ", "NS_INADDRSZ", "NS_IN6ADDRSZ", "NS_CMPRSFLGS", "NS_DEFAULTPORT",
            "NS_MAXMSG", "NS_TYPE_ELT", "NS_MAXMSG", "NS_TSIG_FUDGE", "NS_TSIG_TCP_COUNT", "NS_TSIG_ALG_HMAC_MD5",
            "NS_TSIG_ERROR_NO_TSIG", "NS_TSIG_ERROR_NO_SPACE", "NS_TSIG_ERROR_FORMERR", "NS_SIG_TYPE", "NS_SIG_ALG",
            "NS_SIG_LABELS", "NS_SIG_OTTL", "NS_SIG_EXPIR", "NS_SIG_SIGNED", "NS_SIG_FOOT", "NS_SIG_SIGNER",
            "NS_NXT_BITS", "NS_NXT_BIT_SET", "NS_NXT_BIT_CLEAR", "NS_NXT_MAX", "NS_KEY_TYPEMASK", "NS_KEY_TYPE_AUTH_CONF",
            "NS_KEY_TYPE_CONF_ONLY", "NS_KEY_TYPE_AUTH_ONLY", "NS_KEY_TYPE_NO_KEY", "NS_KEY_NO_AUTH", "NS_KEY_NO_CONF",
            "NS_KEY_RESERVED2", "NS_KEY_EXTENDED_FLAGS", "NS_KEY_RESERVED4", "NS_KEY_RESERVED5", "NS_KEY_NAME_TYPE",
            "NS_KEY_NAME_USER", "NS_KEY_NAME_ENTITY", "NS_KEY_NAME_ZONE", "NS_KEY_NAME_RESERVED",
            "NS_KEY_RESERVED8", "NS_KEY_RESERVED9", "NS_KEY_RESERVED10", "NS_KEY_RESERVED11", "NS_KEY_SIGNATORYMASK",
            "NS_KEY_PROT_TLS", "NS_KEY_PROT_EMAIL", "NS_KEY_PROT_DNSSEC", "NS_KEY_PROT_IPSEC", "NS_KEY_PROT_ANY",
            "NS_ALG_MD5RSA", "NS_ALG_DH", "NS_ALG_DSA", "NS_ALG_DSS", "NS_ALG_EXPIRE_ONLY", "NS_ALG_PRIVATE_OID",
            "PACKETSZ", "MAXDNAME", "MAXCDNAME", "MAXLABEL", "HFIXEDSZ", "QFIXEDSZ", "RRFIXEDSZ", "INT32SZ",
            "INT16SZ", "INT8SZ", "INADDRSZ", "IN6ADDRSZ", "INDIR_MASK", "NAMESERVER_PORT", "GETSHORT", "GETLONG",
            "PUTSHORT", "PUTLONG", "NS_GET16", "NS_GET32", "NS_PUT16", "NS_PUT32", "ns_msg_id", "ns_msg_base",
            "ns_msg_end", "ns_msg_size", "ns_msg_count", "ns_rr_name", "ns_rr_type", "ns_rr_class", "ns_rr_ttl",
            "ns_rr_rdlen", "ns_rr_rdata", "ns_name_ntop", "ns_name_pton", "ns_name_unpack", "ns_name_pack",
            "ns_name_uncompress", "ns_name_compress", "ns_name_skip", "ns_name_rollback"]
    return out, ["__ns_msg", "__ns_rr", "__ns_updrec", "ns_tsig_key", "ns_sect", "ns_flag", "ns_opcode", "ns_rcode",
                 "ns_update_operation", "ns_cert_types", "ns_type", "ns_class", "ns_msg", "ns_rr", "ns_updrec",
                 "ns_tcp_tsig_state", "ns_key_types", "ns_key_name_types"]


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return thin_names()
