"""header_probe.py candidates: stage2-gaps-i (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def stage2_gaps_i_names():
    """(names, tags) of HEADERS-STAGE2-GAPS-I (Yohane stage 2 on snapshot 26.3, R-H7).

    * <stdlib.h>'s GNU `comparison_fn_t` (GNU C Library manual, "Comparison Functions"; systemd and elogind
      `sort-util.h`).
    * The System V terminal structure `struct termio` (termio(7); llvm-19's sanitizer `sizeof (struct termio)`
      with <termios.h>, <sys/ioctl.h> and the kernel UAPI headers, not <termio.h>).
    * <protocols/talkd.h>: the names inetutils' talk and talkd use (talk/*.c, talkd/*.c: CTL_MSG, CTL_RESPONSE,
      the request types, the answers, the protocol limits) and the sockaddr form its messages carry.
    * <protocols/rwhod.h>, <protocols/timed.h>, <protocols/routed.h>: the packet formats of rwhod(8), timed(8) (the
      TSP message types) and routed(8) (RIP version 1, RFC 1058: commands, the route entry, the limits), with the
      member-name macros of their unions.
    The probe settles which of these the reference defines, where and in which configurations."""
    talkd = ["CTL_MSG", "CTL_RESPONSE", "TALK_VERSION", "NAME_SIZE", "TTY_SIZE", "MAX_LIFE", "RING_WAIT",
             "LEAVE_INVITE", "LOOK_UP", "DELETE", "ANNOUNCE", "SUCCESS", "NOT_HERE", "FAILED", "MACHINE_UNKNOWN",
             "PERMISSION_DENIED", "UNKNOWN_REQUEST", "BADVERSION", "BADADDR", "BADCTLADDR"]
    rwhod = ["WHODVERSION", "WHODTYPE_STATUS", "_PATH_RWHODIR"]
    timed = ["TSPVERSION", "ANYADDR", "TSPTYPENUMBER", "TSPTYPES", "tsptype", "tsp_time", "tsp_hopcnt"]
    timed += [f"TSP_{t}" for t in (
        "ANY", "ADJTIME", "ACK", "MASTERREQ", "MASTERACK", "SETTIME", "MASTERUP", "SLAVEUP", "ELECTION", "ACCEPT",
        "REFUSE", "CONFLICT", "RESOLVE", "QUIT", "DATE", "DATEREQ", "DATEACK", "TRACEON", "TRACEOFF", "MSITE",
        "MSITEREQ", "TEST", "SETDATE", "SETDATEREQ", "LOOP")]
    routed = ["RIPVERSION", "RIPCMD_REQUEST", "RIPCMD_RESPONSE", "RIPCMD_TRACEON", "RIPCMD_TRACEOFF", "RIPCMD_MAX",
              "RIPCMDS", "ripcmds", "rip_nets", "rip_tracefile", "HOPCNT_INFINITY", "MAXPACKETSIZE", "TIMER_RATE",
              "SUPPLY_INTERVAL", "MIN_WAITTIME", "MAX_WAITTIME", "EXPIRE_TIME", "GARBAGE_TIME"]
    names = ["comparison_fn_t"] + talkd + rwhod + timed + routed
    tags = ["termio", "osockaddr", "whod", "whoent", "outmp", "tsp", "rip", "netinfo"]
    return names, tags


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return stage2_gaps_i_names()
