"""header_probe.py candidates: packages (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""
from header_probe import ABI, tsv  # noqa: F401


def package_names():
    """Names that package builds against libniji's headers failed on (HEADERS-SNAPSHOT-GAPS: the snapshot of
    2026-11-24 built tree, xxhash and a static busybox), with the members of the same families that a program's
    own source or the reference system's installed documentation names: the BSD integer type names (tree's
    `u_long`; busybox's `caddr_t`, `loff_t`, `ulong`), the regex translate-table type (busybox's grep), and the
    login-accounting and other default path macros (busybox's sources, utmp(5), utmpx(5) and the installed
    documentation), and the BSD signal-handler type name `sig_t` (apt's configure checks for it or `sighandler_t`).
    HEADERS-APT-GAPS (apt 3.0.4's build): the DNS message header type `HEADER` (apt's SRV lookup; RFC 1035 4.1.1),
    and the siginfo_t member names of the seccomp and memory-protection-key reports (apt's methods read
    `si_syscall`; sigaction(2) lists `si_call_addr`, `si_syscall`, `si_arch`, `si_lower`, `si_upper`, `si_pkey`).
    A candidate is only a name to test: which header shows it, and its type or value, are probed."""
    out = ["u_char", "u_short", "u_int", "u_long", "ushort", "uint", "ulong", "quad_t", "u_quad_t",
           "u_int8_t", "u_int16_t", "u_int32_t", "u_int64_t", "caddr_t", "daddr_t", "loff_t", "register_t",
           "RE_TRANSLATE_TYPE", "UTMP_FILE", "UTMPX_FILE", "WTMP_FILE", "WTMPX_FILE", "sig_t",
           "HEADER", "si_call_addr", "si_syscall", "si_arch", "si_lower", "si_upper", "si_pkey"]
    out += [f"_PATH_{p}" for p in ("BSHELL", "BTMP", "DEFPATH", "DEV", "FSTAB", "GROUP", "GSHADOW", "KLOG", "LOG",
                                   "LOGIN", "MNTTAB", "MOUNTED", "MTAB", "PASSWD", "SERVICES", "SHADOW", "TMP",
                                   "TTY", "TTYS", "UTMP", "UTMPX", "WTMP", "WTMPX")]
    # HEADERS-STAGE2-GAPS-A: perl's locale.c names the nl_langinfo items of the locale categories (LC_NAME,
    # LC_ADDRESS, LC_TELEPHONE, LC_MEASUREMENT, LC_IDENTIFICATION) and gnutls's tests the item macros; every item
    # name spec/abi/langinfo.tsv records (an alias after `/`) is a candidate, with the three item macros.
    out += ["_NL_ITEM", "_NL_ITEM_CATEGORY", "_NL_ITEM_INDEX"]
    out += [n for r in tsv("langinfo.tsv") for n in r["name"].split("/")]
    # HEADERS-FEATURES-FORMS (Yohane stage 2): python3.13's fcntlmodule.c (F_OWNER_TID, F_OWNER_PID, F_OWNER_PGRP:
    # fcntl(2) F_GETOWN_EX), util-linux's lib/monotonic.c (TIMESPEC_TO_TIMEVAL, and its inverse), fstrm (SUN_LEN,
    # unix(7)), openssl's ktls.h (SOL_TCP) and the other socket option levels of socket(7), tcp(7), udp(7), ip(7),
    # ipv6(7), packet(7), netlink(7), raw(7), and the kernel's UAPI headers (SOL_TLS, SOL_XDP, SOL_MPTCP, ...).
    out += ["F_OWNER_TID", "F_OWNER_PID", "F_OWNER_PGRP", "TIMESPEC_TO_TIMEVAL", "TIMEVAL_TO_TIMESPEC", "SUN_LEN"]
    out += [f"SOL_{s}" for s in ("SOCKET", "IP", "IPV6", "ICMPV6", "TCP", "UDP", "UDPLITE", "RAW", "DECNET", "X25",
                                 "PACKET", "ATM", "AAL", "IRDA", "NETBEUI", "LLC", "DCCP", "NETLINK", "TIPC", "RXRPC",
                                 "PPPOL2TP", "BLUETOOTH", "PNPIPE", "RDS", "IUCV", "CAIF", "ALG", "NFC", "KCM", "TLS",
                                 "XDP", "MPTCP", "MCTP", "SMC", "VSOCK", "SCTP", "AX25", "ROSE", "NETROM", "IPX",
                                 "ATALK", "HCI", "L2CAP", "SCO", "RFCOMM")]
    # HEADERS-STAGE2-GAPS-D (Yohane stage 2): perl's pp_sys.c casts to `__priority_which_t`, perl and heimdal read
    # `struct hostent`'s `h_addr`; glib calls IN_MULTICAST, with the other address-class tests of RFC 791 / RFC 1112
    # (class A to D, experimental, bad class); google-perftools waits with `__WALL` (wait(2) names __WALL, __WCLONE,
    # __WNOTHREAD); icu reads `__timezone` (exported with `__daylight` and `__tzname`, versions.tsv). perl's Socket
    # module defines its IN6ADDR_ANY / IN6ADDR_LOOPBACK constants where POSIX's IN6ADDR_ANY_INIT / IN6ADDR_LOOPBACK_INIT are.
    out += ["__priority_which_t", "h_addr", "IN_CLASSA", "IN_CLASSB", "IN_CLASSC", "IN_CLASSD", "IN_MULTICAST",
            "IN_EXPERIMENTAL", "IN_BADCLASS", "__WALL", "__WCLONE", "__WNOTHREAD", "__timezone", "__daylight", "__tzname",
            "IN6ADDR_ANY_INIT", "IN6ADDR_LOOPBACK_INIT"]
    # NL-LOCALE-NAME-ABI: perl's perl.h tests `_NL_LOCALE_NAME` to choose how it learns a locale object's names, and
    # the interpreter structure's layout with it. (`static_assert`, which perl.h also tests, is a stage2_names candidate.)
    out += ["_NL_LOCALE_NAME"]
    # HEADERS-GETTEXT-CLANG (Yohane's stage 2 on snapshot 2026-11-25.2): the struct utmp compatibility member names
    # sysvinit and procps use (utmp(5): `ut_name`, `ut_time`, `ut_addr`; `ut_xtime` beside them) and the login
    # accounting file name macros of the same header; psutil's struct ifaddrs union member names (getifaddrs(3):
    # `ifa_broadaddr`, `ifa_dstaddr`); ruby's register type `greg_t` (getcontext(3)'s mcontext_t member type); and
    # the libc revision test of gettext's AM_GNU_GETTEXT (gettext.m4: `__GNU_GETTEXT_SUPPORTED_REVISION`).
    out += ["ut_name", "ut_time", "ut_xtime", "ut_addr", "UTMP_FILENAME", "WTMP_FILENAME", "ifa_broadaddr",
            "ifa_dstaddr", "greg_t", "__GNU_GETTEXT_SUPPORTED_REVISION"]
    return out


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return package_names(), ()
