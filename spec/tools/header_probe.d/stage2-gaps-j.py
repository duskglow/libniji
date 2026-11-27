"""header_probe.py candidates: stage2-gaps-j (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def stage2_gaps_j_names():
    """(names, tags) of HEADERS-STAGE2-GAPS-J (Yohane stage 2, llvm-19's sanitizer_platform_limits_posix.cpp).

    * <netax25/ax25.h>, <netrom/netrom.h>: the parameter structures and their ioctl requests the sanitizer sizes and
      names (`struct ax25_parms_struct`, `struct nr_parms_struct`, SIOCAX25GETPARMS ..., SIOCNRGETPARMS ...), the
      index names of the AX.25 parameter vector and the kernel UAPI names of <linux/ax25.h> and <linux/netrom.h>.
    * <net/if_ppp.h>, <net/ppp_defs.h>: `struct ppp_stats` and the PPPIOC* requests the sanitizer names, with the
      kernel UAPI names of <linux/ppp-ioctl.h> and <linux/ppp_defs.h> (the statistics structures, the NP modes, the
      SC_* flags) and the interface request structures of the PPP statistics ioctls.
    The probe settles which of these the reference defines, where and in which configurations."""
    ax25 = ["SIOCAX25GETPARMS", "SIOCAX25SETPARMS", "SIOCAX25GETUID", "SIOCAX25ADDUID", "SIOCAX25DELUID",
            "SIOCAX25NOUID", "SIOCAX25BPQADDR", "SIOCAX25OPTRT", "SIOCAX25CTLCON", "SIOCAX25GETINFO",
            "SIOCAX25GETINFOOLD", "SIOCAX25ADDFWD", "SIOCAX25DELFWD", "SIOCAX25DEVCTL", "AX25_MTU", "AX25_MAX_VALUES",
            "AX25_DIGI_INBAND", "AX25_DIGI_XBAND", "sax25_uid"]
    fields = ["IPDEFMODE", "AXDEFMODE", "NETROM", "TEXT", "BACKOFF", "CONMODE", "WINDOW", "EWINDOW", "T1", "T2", "T3",
              "N2", "DIGI", "IDLE", "PACLEN", "IPMAXQUEUE", "MAXQUEUE"]
    ax25 += [f"AX25_VALUES_{f}" for f in fields] + [f"AX25_DEF_{f}" for f in fields]
    netrom = ["SIOCNRGETPARMS", "SIOCNRSETPARMS", "SIOCNRDECOBS", "SIOCNRRTCTL", "SIOCNRCTLCON", "NETROM_MTU",
              "NETROM_T1", "NETROM_T2", "NETROM_N2", "NETROM_T4", "NETROM_IDLE", "NETROM_KILL", "NETROM_NEIGH",
              "NETROM_NODE"]
    ppp = ["PPPIOCGFLAGS", "PPPIOCSFLAGS", "PPPIOCGASYNCMAP", "PPPIOCSASYNCMAP", "PPPIOCGUNIT", "PPPIOCGRASYNCMAP",
           "PPPIOCSRASYNCMAP", "PPPIOCGMRU", "PPPIOCSMRU", "PPPIOCSMAXCID", "PPPIOCGXASYNCMAP", "PPPIOCSXASYNCMAP",
           "PPPIOCXFERUNIT", "PPPIOCSCOMPRESS", "PPPIOCGNPMODE", "PPPIOCSNPMODE", "PPPIOCSPASS", "PPPIOCSACTIVE",
           "PPPIOCGDEBUG", "PPPIOCSDEBUG", "PPPIOCGIDLE", "PPPIOCGIDLE32", "PPPIOCGIDLE64", "PPPIOCNEWUNIT",
           "PPPIOCATTACH", "PPPIOCDETACH", "PPPIOCSMRRU", "PPPIOCCONNECT", "PPPIOCDISCONN", "PPPIOCATTCHAN",
           "PPPIOCGCHAN", "PPPIOCGL2TPSTATS", "PPPIOCBRIDGECHAN", "PPPIOCUNBRIDGECHAN", "SIOCGPPPSTATS",
           "SIOCGPPPVER", "SIOCGPPPCSTATS", "ifr__name", "stats_ptr", "NUM_NP", "NPmode", "NPMODE_PASS",
           "NPMODE_DROP", "NPMODE_ERROR", "NPMODE_QUEUE", "ext_accm", "PPP_VERSION", "PPP_MAGIC", "PPP_MTU",
           "PPP_MAXMRU", "PPP_MAXMTU", "PPP_MINMRU", "PPP_MRU", "PPP_HDRLEN", "PPP_FCSLEN", "PPP_ADDRESS",
           "PPP_CONTROL", "PPP_PROTOCOL", "PPP_ALLSTATIONS", "PPP_UI", "PPP_FLAG", "PPP_ESCAPE", "PPP_TRANS",
           "PPP_IP", "PPP_AT", "PPP_IPX", "PPP_VJC_COMP", "PPP_VJC_UNCOMP", "PPP_MP", "PPP_IPV6", "PPP_COMPFRAG",
           "PPP_COMP", "PPP_MPLS_UC", "PPP_MPLS_MC", "PPP_IPCP", "PPP_ATCP", "PPP_IPXCP", "PPP_IPV6CP",
           "PPP_CCPFRAG", "PPP_CCP", "PPP_MPLSCP", "PPP_LCP", "PPP_PAP", "PPP_LQR", "PPP_CHAP", "PPP_CBCP",
           "PPP_INITFCS", "PPP_GOODFCS", "PPP_FCS", "SC_MASK", "SC_FLAG_BITS"]
    ppp += [f"SC_{f}" for f in (
        "COMP_PROT", "COMP_AC", "COMP_TCP", "NO_TCP_CCID", "REJ_COMP_AC", "REJ_COMP_TCP", "CCP_OPEN", "CCP_UP",
        "ENABLE_IP", "LOOP_TRAFFIC", "MULTILINK", "MP_SHORTSEQ", "COMP_RUN", "DECOMP_RUN", "MP_XSHORTSEQ", "DEBUG",
        "LOG_INPKT", "LOG_OUTPKT", "LOG_RAWIN", "LOG_FLUSH", "SYNC", "MUST_COMP", "XMIT_BUSY", "RCV_ODDP",
        "RCV_EVNP", "RCV_B7_1", "RCV_B7_0", "DC_FERROR", "DC_ERROR")]
    names = ax25 + netrom + ppp
    tags = ["ax25_parms_struct", "ax25_bpqaddr_struct", "nr_parms_struct", "nr_route_struct", "nr_ctl_struct",
            "ppp_stats", "pppstat", "vjstat", "compstat", "ppp_comp_stats", "ppp_idle", "ppp_idle32", "ppp_idle64",
            "npioctl", "ppp_option_data", "pppol2tp_ioc_stats", "ifpppstatsreq", "ifpppcstatsreq", "NPmode"]
    return names, tags


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return stage2_gaps_j_names()
