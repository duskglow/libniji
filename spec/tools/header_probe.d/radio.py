"""header_probe.py candidates: radio (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def radio_names():
    """(names, tags) of the amateur-radio and legacy protocol headers (HEADERS-STAGE2-GAPS-C; net-tools names `struct
    sockaddr_ax25`): netax25/ax25.h, netrom/netrom.h, netrose/rose.h from the kernel UAPI headers linux/ax25.h,
    linux/netrom.h and linux/rose.h (their socket options, limits, address and control structures) and socket(2)'s
    SOL_<family> levels; netipx/ipx.h and neteconet/ec.h from the kernel UAPI headers linux/ipx.h and linux/if_ec.h of
    the kernels that had them (the IPX address, interface and route structures and frame types; the Econet
    address and transmit status codes); netash/ash.h from AF_ASH's address structure. A candidate is only a name to
    test: which header shows it, its value and its layout are probed."""
    ax = ["AX25_" + n for n in ("MTU", "MAX_DIGIS", "WINDOW", "T1", "N2", "T3", "T2", "BACKOFF", "EXTSEQ", "PIDINCL",
                                "IDLE", "PACLEN", "IAMDIGI", "KILL", "SET_RT_IPMODE", "NOUID_DEFAULT", "NOUID_BLOCK")]
    # Not candidates: the protocol-private ioctls (SIOCAX25*, SIOCNRDECOBS, SIOCRS*, SIOCAIPX*, SIOCIPX*) and the
    # member aliases sax25_uid and sipx_special, which the reference shows but which are no integer constant in a
    # unit including their header alone (type_probe.py finds no value; the generator needs one for every macro row):
    # stage2-gaps-j.py and stage2-gaps-k.py name the AX.25 and IPX ones, their templates define them.
    ax += ["SOL_AX25", "SOL_NETROM", "SOL_ROSE", "SOL_IPX", "SOL_ECONET", "SOL_ASH"]
    nr = ["NETROM_" + n for n in ("MTU", "T1", "T2", "N2", "T4", "IDLE", "NEIGH", "NODE")]
    ro = ["ROSE_" + n for n in ("MTU", "MAX_DIGIS", "DEFER", "T1", "T2", "T3", "IDLE", "QBITINCL", "HOLDBACK",
                                "DTE_ORIGINATED", "NUMBER_BUSY", "INVALID_FACILITY", "NETWORK_CONGESTION",
                                "OUT_OF_ORDER", "ACCESS_BARRED", "NOT_OBTAINABLE", "REMOTE_PROCEDURE",
                                "LOCAL_PROCEDURE", "SHIP_ABSENT")]
    ipx = ["IPX_" + n for n in ("NODE_LEN", "MTU", "FRAME_NONE", "FRAME_SNAP", "FRAME_8022", "FRAME_ETHERII",
                                "FRAME_8023", "FRAME_TR_8022", "SPECIAL_NONE", "PRIMARY", "INTERNAL", "DLTITF",
                                "CRTITF", "TYPE", "RT_SNAP", "RT_8022", "RT_BLUEBOOK", "RT_ROUTED", "ROUTE_NO_ROUTER")]
    ec = ["ECTYPE_" + n for n in ("PACKET_RECEIVED", "TRANSMIT_STATUS", "TRANSMIT_OK", "TRANSMIT_NOT_LISTENING",
                                  "TRANSMIT_NET_ERROR", "TRANSMIT_NO_CLOCK", "TRANSMIT_LINE_JAMMED",
                                  "TRANSMIT_NOT_PRESENT")]
    tags = ["ax25_address", "sockaddr_ax25", "full_sockaddr_ax25", "ax25_routes_struct", "ax25_route_opt_struct",
            "ax25_ctl_struct", "ax25_info_struct_deprecated", "ax25_info_struct", "ax25_fwd_struct", "nr_route_struct",
            "rose_address", "sockaddr_rose", "full_sockaddr_rose", "rose_route_struct", "rose_cause_struct",
            "rose_facilities_struct", "sockaddr_ipx", "ipx_route_definition", "ipx_interface_definition",
            "ipx_config_data", "ipx_route_def", "sockaddr_ec", "ec_addr", "sockaddr_ash"]
    # ax25_address and rose_address are typedef names (of untagged structures) in the kernel headers, so ordinary
    # names too.
    return ax + nr + ro + ipx + ec + ["ax25_address", "rose_address"], tags


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return radio_names()
