"""header_probe.py candidates: stage2-gaps-e (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def stage2e_names():
    """Names stage 2 package builds against libniji's headers failed on (HEADERS-STAGE2-GAPS-E: libnftnl and
    libnetfilter-conntrack, inetutils' tftpsubs.c, keyutils' dns.afsdb.c, libev's ev_select.c, libnsl, libcap-ng's
    filecap.c), with their families: the IP protocol numbers of ip(7) and the kernel UAPI <linux/in.h>, with
    `IPPROTO_MAX`; the `struct tftphdr` member names of RFC 1350 packets; the `ns_msg`/`ns_rr` accessors and the
    byte-order macros of the BIND resolver interface (resolver(3)); `fd_mask` and `NFDBITS` of select(2); the
    reserved scalar type names `__u_char` ... `__u_quad_t`; and the names another header's template writes itself
    that the include sweep found shown by other headers (`S_ISDIR` through <ftw.h>, the timer macros of
    <sys/time.h>, ...). A candidate is only a name to test: which header shows it, and its kind, are probed."""
    out = [f"IPPROTO_{p}" for p in (
        "IP", "ICMP", "IGMP", "IPIP", "TCP", "EGP", "PUP", "UDP", "IDP", "TP", "DCCP", "IPV6", "RSVP", "GRE", "ESP",
        "AH", "MTP", "BEETPH", "ENCAP", "PIM", "COMP", "L2TP", "SCTP", "UDPLITE", "MPLS", "ETHERNET", "RAW", "MPTCP",
        "SMC", "MAX", "HOPOPTS", "ROUTING", "FRAGMENT", "ICMPV6", "NONE", "DSTOPTS", "MH")]
    out += ["th_opcode", "th_block", "th_code", "th_stuff", "th_data", "th_msg", "th_u", "th_padding"]
    out += [f"ns_msg_{m}" for m in ("id", "base", "end", "size", "count", "getflag")]
    out += [f"ns_rr_{m}" for m in ("name", "type", "class", "ttl", "rdlen", "rdata", "nsec")]
    out += ["NS_GET16", "NS_GET32", "NS_PUT16", "NS_PUT32", "GETSHORT", "GETLONG", "PUTSHORT", "PUTLONG"]
    out += ["fd_mask", "__fd_mask", "NFDBITS", "__NFDBITS", "FD_SETSIZE", "__FD_SETSIZE", "__FDELT", "__FDMASK"]
    out += ["__u_char", "__u_short", "__u_int", "__u_long", "__quad_t", "__u_quad_t", "u_char", "u_short", "u_int",
            "u_long", "quad_t", "u_quad_t"]
    out += ["S_ISBLK", "S_ISCHR", "S_ISDIR", "S_ISFIFO", "S_ISLNK", "S_ISREG", "S_ISSOCK", "S_TYPEISMQ",
            "S_TYPEISSEM", "S_TYPEISSHM", "S_TYPEISTMO", "st_atime", "st_ctime", "st_mtime", "timeradd",
            "timerclear", "timercmp", "timerisset", "timersub", "TCSETS2", "bswap_16", "bswap_32", "bswap_64",
            "index", "rindex"]
    return out


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return stage2e_names(), ()
