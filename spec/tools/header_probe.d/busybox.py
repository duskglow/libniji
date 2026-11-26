"""header_probe.py candidates: busybox (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def busybox_names():
    """(names, tags) a static busybox build against libniji's headers failed on (HEADERS-BUSYBOX-GAPS: networking/arp.c,
    arping.c, libbb/xconnect.c, miscutils/mt.c), with their families: the struct ifreq and struct ifconf member names
    of netdevice(7) and the kernel UAPI <linux/if.h> (`ifr_name`, `ifc_req`, ...); the packet socket constants and
    structures of packet(7) and <linux/if_packet.h>; the tape operation, status and option names and structures of
    st(4) and <linux/mtio.h>. A candidate is only a name to test: which header shows it, and its type or value, are
    probed."""
    out = ["ifr_name", "ifr_hwaddr", "ifr_addr", "ifr_dstaddr", "ifr_broadaddr", "ifr_netmask", "ifr_flags",
           "ifr_metric", "ifr_mtu", "ifr_map", "ifr_slave", "ifr_data", "ifr_ifindex", "ifr_bandwidth", "ifr_qlen",
           "ifr_newname", "ifr_settings", "ifc_buf", "ifc_req"]
    out += [f"PACKET_{p}" for p in (
        "HOST", "BROADCAST", "MULTICAST", "OTHERHOST", "OUTGOING", "LOOPBACK", "USER", "KERNEL", "FASTROUTE",
        "ADD_MEMBERSHIP", "DROP_MEMBERSHIP", "RECV_OUTPUT", "RX_RING", "STATISTICS", "COPY_THRESH", "AUXDATA",
        "ORIGDEV", "VERSION", "HDRLEN", "RESERVE", "TX_RING", "LOSS", "VNET_HDR", "TX_TIMESTAMP", "TIMESTAMP",
        "FANOUT", "TX_HAS_OFF", "QDISC_BYPASS", "ROLLOVER_STATS", "FANOUT_DATA", "IGNORE_OUTGOING", "VNET_HDR_SZ",
        "MR_MULTICAST", "MR_PROMISC", "MR_ALLMULTI", "MR_UNICAST")]
    out += ["MTRESET", "MTFSF", "MTBSF", "MTFSR", "MTBSR", "MTWEOF", "MTREW", "MTOFFL", "MTNOP", "MTRETEN", "MTBSFM",
            "MTFSFM", "MTEOM", "MTERASE", "MTRAS1", "MTRAS2", "MTRAS3", "MTSETBLK", "MTSETDENSITY", "MTSEEK", "MTTELL",
            "MTSETDRVBUFFER", "MTFSS", "MTBSS", "MTWSM", "MTLOCK", "MTUNLOCK", "MTLOAD", "MTUNLOAD", "MTCOMPRESSION",
            "MTSETPART", "MTMKPART", "MTWEOFI", "MTIOCTOP", "MTIOCGET", "MTIOCPOS", "MTIOCGETCONFIG",
            "MTIOCSETCONFIG", "MT_TAPE_INFO", "DEFTAPE"]
    out += [f"MT_IS{t}" for t in (
        "UNKNOWN", "QIC02", "WT5150", "ARCHIVE_5945L2", "CMSJ500", "TDC3610", "ARCHIVE_VP60I", "ARCHIVE_2150L",
        "ARCHIVE_2060L", "ARCHIVESC499", "QIC02_ALL_FEATURES", "WT5099EEN24", "TEAC_MT2ST", "EVEREX_FT40A", "DDS1",
        "DDS2", "ONSTREAM_SC", "SCSI1", "SCSI2", "FTAPE_UNKNOWN", "FTAPE_FLAG")]
    out += [f"GMT_{g}" for g in ("EOF", "BOT", "EOT", "SM", "EOD", "WR_PROT", "ONLINE", "D_6250", "D_1600", "D_800",
                                 "DR_OPEN", "IM_REP_EN", "CLN")]
    out += [f"MT_ST_{s}" for s in (
        "BLKSIZE_SHIFT", "BLKSIZE_MASK", "DENSITY_SHIFT", "DENSITY_MASK", "SOFTERR_SHIFT", "SOFTERR_MASK", "OPTIONS",
        "BOOLEANS", "SETBOOLEANS", "CLEARBOOLEANS", "WRITE_THRESHOLD", "DEF_BLKSIZE", "DEF_OPTIONS", "TIMEOUTS",
        "SET_TIMEOUT", "SET_LONG_TIMEOUT", "SET_CLN", "BUFFER_WRITES", "ASYNC_WRITES", "READ_AHEAD", "DEBUGGING",
        "TWO_FM", "FAST_MTEOM", "AUTO_LOCK", "DEF_WRITES", "CAN_BSR", "NO_BLKLIMS", "CAN_PARTITIONS", "SCSI2LOGICAL",
        "SYSV", "NOWAIT", "SILI", "NOWAIT_EOF", "CLEAR_DEFAULT", "DEF_DENSITY", "DEF_COMPRESSION", "DEF_DRVBUFFER",
        "HPLOADER_OFFSET")]
    tags = ["ifconf", "ifaddr", "packet_mreq", "sockaddr_pkt", "tpacket_req", "tpacket_stats", "tpacket_auxdata",
            "tpacket_hdr", "mtop", "mtget", "mtpos", "mtconfiginfo", "mt_tape_info"]
    return out, tags


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return busybox_names()
