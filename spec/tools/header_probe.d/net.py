"""header_probe.py candidates: net (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def net_names():
    """(names, tags) of slice 13's network headers that no other table spells (HEADERS-TABLES-NET). Each group names
    its source; a candidate is only a name to test: whether a header shows it, and its value and type, are probed.
    * IPv4 (netinet/ip.h): the type-of-service bits of ip(7) (IP_TOS) and RFC 1349, the precedence levels of RFC 791
      3.1, the codepoints of RFC 2474 (class selectors), RFC 2597 (AF), RFC 3246 (EF), RFC 8622 (LE) and the ECN field
      of RFC 3168 5; the option numbers, classes and security levels of RFC 791 3.1 and RFC 2113 (router alert); the
      timestamp option flags of RFC 791; the header constants of RFC 791 (version, time to live, fragment flags).
    * ICMP (netinet/ip_icmp.h): the message types and codes of RFC 792, RFC 950 (address mask), RFC 1256 (router
      discovery), RFC 1191 and RFC 1812 5.2.7.1 (unreachable codes), RFC 8335 (extended echo), in the spellings of the
      kernel UAPI header linux/icmp.h and in the type-and-code spellings ICMP_<TYPE>_<CODE> of the same messages.
    * ICMPv6 and neighbour discovery (netinet/icmp6.h): RFC 3542 2.2 and 3 (types, codes, ND messages, flags and
      options, ICMP6_FILTER, MLD), RFC 4443 3.1 (codes 5 and 6), RFC 2894 (router renumbering), RFC 4191 (route
      preference), RFC 6275 7 (mobile IPv6 options), RFC 8106 (DNS options).
    * Routes (net/route.h): the kernel UAPI headers linux/route.h and linux/ipv6_route.h (struct rtentry, struct
      in6_rtmsg, RTF_*, RTMSG_*), linux/in_route.h (RTCF_*).
    * ARP (net/if_arp.h, netinet/if_ether.h): arp(7) (struct arpreq, ATF_*), the kernel UAPI header linux/if_arp.h
      (struct arpreq_old), RFC 826 (the Ethernet ARP packet, struct ether_arp).
    * TELNET (arpa/telnet.h): the commands of RFC 854, the option numbers of the IANA TELNET options registry
      (RFC 855 ff.), the subnegotiation qualifiers of RFC 1091 and RFC 1572, the LINEMODE names of RFC 1184, the
      toggle-flow-control values of RFC 1372, the authentication and encryption names of RFC 2941 and RFC 2946.
    * FTP (arpa/ftp.h): the reply classes, representation types, formats, structures and transfer modes of RFC 959,
      the block-mode descriptors of RFC 959 3.4.2. TFTP (arpa/tftp.h): the opcodes and error codes of RFC 1350 and
      RFC 2347, its segment size.
    * SLIP (net/if_slip.h) and PPP compression (net/ppp-comp.h): the kernel UAPI headers linux/if_slip.h and
      linux/ppp-comp.h.
    Not candidates: names the reference shows that are no integer constant in a unit including their header alone
    (type_probe.py finds no value; the generator needs one for every macro row): ETHER_MAP_IP_MULTICAST (a
    function-like macro), ICMP_TSLEN, and SIOCSKEEPALIVE, SIOCGKEEPALIVE, SIOCSOUTFILL, SIOCGOUTFILL, SIOCSLEASE,
    SIOCGLEASE of net/if_slip.h. They wait for a template definition (handoff/HEADERS-TABLES-NET.md)."""
    n = []
    tos = ["LOWDELAY", "THROUGHPUT", "RELIABILITY", "MINCOST", "LOWCOST", "TOS_MASK", "PREC_MASK", "ECN_MASK", "DSCP_MASK",
           "CLASS_MASK", "ECN_NOT_ECT", "ECN_ECT0", "ECN_ECT1", "ECN_CE", "CE", "ECT", "DSCP_EF", "DSCP_LE", "CLASS_DEFAULT"]
    tos += [f"PREC_{p}" for p in ("NETCONTROL", "INTERNETCONTROL", "CRITIC_ECP", "FLASHOVERRIDE", "FLASH", "IMMEDIATE",
                                  "PRIORITY", "ROUTINE")]
    tos += [f"DSCP_AF{c}{d}" for c in "1234" for d in "123"] + [f"CLASS_CS{i}" for i in range(8)] + [f"DSCP_CS{i}" for i in range(8)]
    n += [f"IPTOS_{t}" for t in tos]
    opt = ["COPY", "CLASS_MASK", "NUMBER_MASK", "CONTROL", "RESERVED1", "DEBMEAS", "MEASUREMENT", "RESERVED2", "EOL", "END",
           "NOP", "NOOP", "RR", "TS", "TIMESTAMP", "SECURITY", "SEC", "LSRR", "SATID", "SID", "SSRR", "RA", "OPTVAL", "OLEN",
           "OFFSET", "MINOFF", "TS_TSONLY", "TS_TSANDADDR", "TS_PRESPEC"]
    opt += [f"SECUR_{s}" for s in ("UNCLASS", "CONFID", "EFTO", "MMMM", "RESTR", "SECRET", "TOPSECRET", "PROG")]
    n += [f"IPOPT_{o}" for o in opt]
    n += ["IPVERSION", "IP_MAXPACKET", "MAXTTL", "IPDEFTTL", "IPFRAGTTL", "IPTTLDEC", "IP_MSS", "IP_RF", "IP_DF", "IP_MF",
          "IP_OFFMASK", "MAX_IPOPTLEN"]
    icmp = ["ECHOREPLY", "DEST_UNREACH", "UNREACH", "SOURCE_QUENCH", "SOURCEQUENCH", "REDIRECT", "ECHO", "ROUTERADVERT",
            "ROUTERSOLICIT", "TIME_EXCEEDED", "TIMXCEED", "PARAMETERPROB", "PARAMPROB", "TIMESTAMP", "TSTAMP",
            "TIMESTAMPREPLY", "TSTAMPREPLY", "INFO_REQUEST", "IREQ", "INFO_REPLY", "IREQREPLY", "ADDRESS", "MASKREQ",
            "ADDRESSREPLY", "MASKREPLY", "ALTHOSTADDR", "MAXTYPE", "NET_UNREACH", "HOST_UNREACH", "PROT_UNREACH",
            "PORT_UNREACH", "FRAG_NEEDED", "SR_FAILED", "NET_UNKNOWN", "HOST_UNKNOWN", "HOST_ISOLATED", "NET_ANO",
            "HOST_ANO", "NET_UNR_TOS", "HOST_UNR_TOS", "PKT_FILTERED", "PREC_VIOLATION", "PREC_CUTOFF", "REDIR_NET",
            "REDIR_HOST", "REDIR_NETTOS", "REDIR_HOSTTOS", "EXC_TTL", "EXC_FRAGTIME", "MINLEN", "MASKLEN",
            "ADVLENMIN", "FILTER", "EXT_ECHO", "EXT_ECHOREPLY", "ROUTERADVERT_NORMAL", "ROUTERADVERT_NOROUTE_COMMON"]
    icmp += [f"UNREACH_{c}" for c in ("NET", "HOST", "PROTOCOL", "PORT", "NEEDFRAG", "SRCFAIL", "NET_UNKNOWN",
                                      "HOST_UNKNOWN", "ISOLATED", "NET_PROHIB", "HOST_PROHIB", "TOSNET", "TOSHOST",
                                      "FILTER_PROHIB", "HOST_PRECEDENCE", "PRECEDENCE_CUTOFF")]
    icmp += [f"REDIRECT_{c}" for c in ("NET", "HOST", "TOSNET", "TOSHOST")]
    icmp += ["TIMXCEED_INTRANS", "TIMXCEED_REASS", "PARAMPROB_OPTABSENT", "PARAMPROB_ERRATPTR", "PARAMPROB_LENGTH"]
    n += [f"ICMP_{c}" for c in icmp] + ["NR_ICMP_TYPES", "NR_ICMP_UNREACH"]
    n += ["ICMP6_" + c for c in ("DST_UNREACH", "PACKET_TOO_BIG", "TIME_EXCEEDED", "PARAM_PROB", "INFOMSG_MASK",
                                 "ECHO_REQUEST", "ECHO_REPLY", "MEMBERSHIP_QUERY", "MEMBERSHIP_REPORT",
                                 "MEMBERSHIP_REDUCTION", "ROUTER_RENUMBERING", "DST_UNREACH_NOROUTE", "DST_UNREACH_ADMIN",
                                 "DST_UNREACH_BEYONDSCOPE", "DST_UNREACH_NOTNEIGHBOR", "DST_UNREACH_ADDR",
                                 "DST_UNREACH_NOPORT", "DST_UNREACH_POLICY", "DST_UNREACH_REJECT",
                                 "TIME_EXCEED_TRANSIT", "TIME_EXCEED_REASSEMBLY", "PARAMPROB_HEADER",
                                 "PARAMPROB_NEXTHEADER", "PARAMPROB_OPTION", "FILTER", "FILTER_PASS", "FILTER_BLOCK",
                                 "FILTER_PASSONLY", "FILTER_BLOCKOTHERS", "RR_FLAGS_TEST", "RR_FLAGS_REQRESULT",
                                 "RR_FLAGS_FORCEAPPLY", "RR_FLAGS_SPECSITE", "RR_FLAGS_PREVDONE",
                                 "RR_PCOUSE_RAFLAGS_ONLINK", "RR_PCOUSE_RAFLAGS_AUTO", "RR_PCOUSE_FLAGS_DECRVLTIME",
                                 "RR_PCOUSE_FLAGS_DECRPLTIME", "RR_RESULT_FLAGS_OOB", "RR_RESULT_FLAGS_FORBIDDEN")]
    n += ["MLD_LISTENER_QUERY", "MLD_LISTENER_REPORT", "MLD_LISTENER_REDUCTION", "RPM_PCO_ADD", "RPM_PCO_CHANGE",
          "RPM_PCO_SETGLOBAL", "RPM_PCO_MAX"]
    n += ["ND_" + c for c in ("ROUTER_SOLICIT", "ROUTER_ADVERT", "NEIGHBOR_SOLICIT", "NEIGHBOR_ADVERT", "REDIRECT",
                              "RA_FLAG_MANAGED", "RA_FLAG_OTHER", "RA_FLAG_HOME_AGENT", "RA_FLAG_PRF_MASK",
                              "RA_FLAG_PRF_HIGH", "RA_FLAG_PRF_MEDIUM", "RA_FLAG_PRF_LOW", "RA_FLAG_PRF_INVALID",
                              "NA_FLAG_ROUTER", "NA_FLAG_SOLICITED", "NA_FLAG_OVERRIDE", "OPT_SOURCE_LINKADDR",
                              "OPT_TARGET_LINKADDR", "OPT_PREFIX_INFORMATION", "OPT_REDIRECTED_HEADER", "OPT_MTU",
                              "OPT_ADV_INTERVAL", "OPT_ADVINTERVAL", "OPT_HOME_AGENT_INFO", "OPT_HOMEAGENT_INFO",
                              "OPT_ROUTE_INFO", "OPT_RDNSS", "OPT_DNSSL", "OPT_PI_FLAG_ONLINK", "OPT_PI_FLAG_AUTO",
                              "OPT_PI_FLAG_RADDR", "OPT_PI_FLAG_ROUTER")]
    n += ["RTF_" + c for c in ("UP", "GATEWAY", "HOST", "REINSTATE", "DYNAMIC", "MODIFIED", "MTU", "MSS", "WINDOW", "IRTT",
                               "REJECT", "STATIC", "XRESOLVE", "NOFORWARD", "THROW", "NOPMTUDISC", "DEFAULT", "ALLONLINK",
                               "ADDRCONF", "PREFIX_RT", "ANYCAST", "NONEXTHOP", "EXPIRES", "ROUTEINFO", "CACHE", "FLOW",
                               "POLICY", "PREF_MASK", "PCPU", "LOCAL", "LINKRT", "MAC", "INTERFACE", "MULTICAST",
                               "BROADCAST", "NAT", "ADDRCLASSMASK")]
    n += ["RTCF_" + c for c in ("DEAD", "ONLINK", "NOPMTUDISC", "NOTIFY", "DIRECTDST", "REDIRECTED", "TPROXY", "FAST",
                                "MASQ", "SNAT", "DOREDIRECT", "DIRECTSRC", "DNAT", "BROADCAST", "MULTICAST", "REJECT",
                                "LOCAL", "NAT")]
    n += ["RTMSG_NEWDEVICE", "RTMSG_DELDEVICE", "RTMSG_NEWROUTE", "RTMSG_DELROUTE", "IP6_RT_PRIO_USER",
          "IP6_RT_PRIO_ADDRCONF", "IP6_RT_PRIO_KERN"]
    n += ["ATF_" + c for c in ("COM", "PERM", "PUBL", "USETRAILERS", "NETMASK", "DONTPUB", "MAGIC")]
    n += ["MAX_ADDR_LEN"]
    tel = ["IAC", "DONT", "DO", "WONT", "WILL", "SB", "GA", "EL", "EC", "AYT", "AO", "IP", "BREAK", "DM", "NOP", "SE",
           "EOR", "ABORT", "SUSP", "xEOF", "SYNCH", "TELCMD_FIRST", "TELCMD_LAST", "NTELOPTS", "TELOPT_FIRST", "TELOPT_LAST"]
    tel += ["TELOPT_" + o for o in ("BINARY", "ECHO", "RCP", "SGA", "NAMS", "STATUS", "TM", "RCTE", "NAOL", "NAOP",
                                    "NAOCRD", "NAOHTS", "NAOHTD", "NAOFFD", "NAOVTS", "NAOVTD", "NAOLFD", "XASCII",
                                    "LOGOUT", "BM", "DET", "SUPDUP", "SUPDUPOUTPUT", "SNDLOC", "TTYPE", "EOR", "TUID",
                                    "OUTMRK", "TTYLOC", "3270REGIME", "X3PAD", "NAWS", "TSPEED", "LFLOW", "LINEMODE",
                                    "XDISPLOC", "OLD_ENVIRON", "ENVIRON", "AUTHENTICATION", "ENCRYPT", "NEW_ENVIRON",
                                    "TN3270E", "XAUTH", "CHARSET", "RSP", "COMPORT", "SLE", "STARTTLS", "KERMIT",
                                    "SEND_URL", "FORWARD_X", "EXOPL")]
    tel += ["TELQUAL_" + q for q in ("IS", "SEND", "INFO", "REPLY", "NAME")]
    tel += ["LFLOW_OFF", "LFLOW_ON", "LFLOW_RESTART_ANY", "LFLOW_RESTART_XON", "LM_MODE", "LM_FORWARDMASK", "LM_SLC",
            "MODE_EDIT", "MODE_TRAPSIG", "MODE_ACK", "MODE_SOFT_TAB", "MODE_LIT_ECHO", "MODE_MASK", "MODE_FLOW",
            "MODE_ECHO", "MODE_INBIN", "MODE_OUTBIN", "MODE_FORCE", "NSLC", "SLC_LEVELBITS", "SLC_FUNC", "SLC_FLAGS",
            "SLC_VALUE", "SLC_ACK", "SLC_FLUSHIN", "SLC_FLUSHOUT", "SLC_NOSUPPORT", "SLC_CANTCHANGE", "SLC_VARIABLE",
            "SLC_DEFAULT"]
    tel += ["SLC_" + s for s in ("SYNCH", "BRK", "IP", "AO", "AYT", "EOR", "ABORT", "EOF", "SUSP", "EC", "EL", "EW", "RP",
                                 "LNEXT", "XON", "XOFF", "FORW1", "FORW2", "MCL", "MCR", "MCWL", "MCWR", "MCBOL", "MCEOL",
                                 "INSRT", "OVER", "ECR", "EWR", "EBOL", "EEOL")]
    tel += ["ENV_VALUE", "ENV_VAR", "ENV_ESC", "ENV_USERVAR", "OLD_ENV_VAR", "OLD_ENV_VALUE", "NEW_ENV_VAR", "NEW_ENV_VALUE"]
    tel += ["AUTH_WHO_CLIENT", "AUTH_WHO_SERVER", "AUTH_WHO_MASK", "AUTH_HOW_ONE_WAY", "AUTH_HOW_MUTUAL", "AUTH_HOW_MASK",
            "AUTH_ENCRYPT_OFF", "AUTH_ENCRYPT_MASK", "AUTHTYPE_CNT", "AUTHTYPE_TEST"]
    tel += ["AUTHTYPE_" + a for a in ("NULL", "KERBEROS_V4", "KERBEROS_V5", "SPX", "MINK", "SRA", "RSA", "SSL")]
    tel += ["ENCRYPT_" + e for e in ("IS", "SUPPORT", "REPLY", "START", "END", "REQSTART", "REQEND", "ENC_KEYID",
                                     "DEC_KEYID", "CNT")]
    tel += ["ENCTYPE_ANY", "ENCTYPE_DES_CFB64", "ENCTYPE_DES_OFB64", "ENCTYPE_CNT"]
    n += tel
    n += ["PRELIM", "COMPLETE", "CONTINUE", "TRANSIENT", "ERROR", "TYPE_A", "TYPE_E", "TYPE_I", "TYPE_L", "FORM_N",
          "FORM_T", "FORM_C", "STRU_F", "STRU_R", "STRU_P", "MODE_S", "MODE_B", "MODE_C", "REC_ESC", "REC_EOR", "REC_EOF",
          "BLK_EOR", "BLK_EOF", "BLK_ERRORS", "BLK_RESTART", "BLK_BYTECOUNT"]
    n += ["SEGSIZE", "RRQ", "WRQ", "DATA", "ACK", "OACK", "EUNDEF", "ENOTFOUND", "EACCESS", "ENOSPACE", "EBADOP", "EBADID",
          "EEXISTS", "ENOUSER", "EOPTNEG"]
    n += ["SL_MODE_SLIP", "SL_MODE_CSLIP", "SL_MODE_KISS", "SL_MODE_SLIP6", "SL_MODE_CSLIP6", "SL_MODE_AX25",
          "SL_MODE_ADAPTIVE", "SL_OPT_SIXBIT", "SL_OPT_ADAPTIVE"]
    n += ["CCP_CONFREQ", "CCP_CONFACK", "CCP_TERMREQ", "CCP_TERMACK", "CCP_RESETREQ", "CCP_RESETACK",
          "CCP_MAX_OPTION_LENGTH", "CCP_HDRLEN", "CCP_OPT_MINLEN", "CI_BSD_COMPRESS", "CILEN_BSD_COMPRESS",
          "BSD_CURRENT_VERSION", "BSD_MIN_BITS", "BSD_MAX_BITS", "CI_DEFLATE", "CI_DEFLATE_DRAFT", "CILEN_DEFLATE",
          "DEFLATE_MIN_SIZE", "DEFLATE_MAX_SIZE", "DEFLATE_METHOD_VAL", "DEFLATE_CHK_SEQUENCE", "CI_MPPE", "CILEN_MPPE",
          "CI_PREDICTOR_1", "CILEN_PREDICTOR_1", "CI_PREDICTOR_2", "CILEN_PREDICTOR_2", "COMP_PROTO_LEN"]
    tags = ["ip_timestamp", "rtentry", "in6_rtmsg", "arpreq", "arpreq_old", "arpd_request", "ether_arp", "icmp_ra_addr",
            "icmp6_filter", "nd_router_solicit", "nd_router_advert", "nd_neighbor_solicit", "nd_neighbor_advert",
            "nd_redirect", "nd_opt_hdr", "nd_opt_prefix_info", "nd_opt_rd_hdr", "nd_opt_mtu", "nd_opt_adv_interval",
            "nd_opt_home_agent_info", "nd_opt_route_info", "nd_opt_rdnss", "nd_opt_dnssl", "mld_hdr",
            "icmp6_router_renum", "rr_pco_match", "rr_pco_use", "rr_result", "tftphdr", "compressor"]
    return n, tags


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return net_names()
