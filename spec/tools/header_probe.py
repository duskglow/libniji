#!/usr/bin/env python3
"""Probe which names each public header makes visible under each feature-test configuration.

    python3 spec/tools/header_probe.py [--jobs N] [--headers a.h,b.h] [--out DIR] [--include DIR]

`--include DIR` probes libniji's headers (P2 of plan/headers.md section 3): every compile gets `-nostdinc -isystem DIR`.

Run on the reference system (Appendix C.0), like abi_extract.py. Writes `ftm-configs.tsv` (the
configurations of plan/headers.md section 2.3) and `visibility.tsv` (Appendix C.12) into spec/abi/
(or --out). `--headers` limits the run to some headers, for trying the probe; a table for the spec
is always written by a full run.

Clean room (plan/headers.md section 2.2, ruling R-H0). The probe compiles translation units of its
own that include one public header and then test candidate names. It reads exactly two things back
from the compiler: the exit status, and the LINE NUMBERS of errors located in the probe's own file
(each test sits alone on its own line, so "line L has an error" is the result of test L). It never
reads diagnostic message text, `-E` or `-dM` output, or any header text.

Candidates are the names the existing tables already hold: prototypes.tsv functions, constants.tsv
macros, versions.tsv data objects, layouts.tsv types and tags, and the type and tag names spelled in
the prototypes.tsv declarations and layouts.tsv member types, and the identifiers the functional spec
writes in backticks. Per (header, configuration):

* macro       `#ifdef N` / `#error` / `#endif` errors (before any `#undef`);
* then every candidate is `#undef`ed and the ordinary name space is tested:
  - declared  C `__typeof__(N) *p;`, C++ `using ::N;` compiles;
  - typedef   C `N *p;` declares (C++: `typedef N t;` compiles and `struct`/`union`/`enum N *`
              all fail, so a class or enum name is not a typedef);
  - function  C: `(0, N)` has the type `__typeof__(N) *` (a function designator decays);
              C++: an ordinary name that is neither an enumerator nor a decay-stable object
              (an overload set lands here too);
  - enumerator `enum { e = (N) };` compiles (an integer constant expression);
  - object    a declared ordinary name that is none of the above;
* struct, union, enum  `sizeof(struct N)` (and `union`, `enum`) compiles: a complete tag.

A header whose bare `#include` fails in a configuration gets one row of kind `include-error`.
`configs` is `*` (every configuration), the ids, or `!` and the ids it is NOT visible in, whichever is
shorter. Rows of header `-` are what the empty translation unit already shows (compiler predefines and the
implicitly included stdc-predef.h); header rows leave those (name, kind, configuration) out.
"""
import argparse, csv, os, re, shutil, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
ABI = os.path.join(ROOT, "spec", "abi")
INVENTORY = os.path.join(ROOT, "plan", "headers-inventory.tsv")

# id, language, flags, what it probes (plan/headers.md section 2.3). The order is the table order.
CONFIGS = [
    ("c89", "c", "-std=c89", "ISO C90, __STRICT_ANSI__"),
    ("c99", "c", "-std=c99", "ISO C99, __STRICT_ANSI__"),
    ("c11", "c", "-std=c11", "ISO C11, __STRICT_ANSI__"),
    ("c17", "c", "-std=c17", "ISO C17, __STRICT_ANSI__"),
    ("c2x", "c", "-std=c2x", "ISO C2X, __STRICT_ANSI__"),
    ("gnu17", "c", "-std=gnu17", "compiler default mode, no feature-test macro"),
    ("gnu2x", "c", "-std=gnu2x", "GNU C2X mode, no feature-test macro"),
    ("posix1", "c", "-std=c17 -D_POSIX_C_SOURCE=1", "POSIX.1-1990"),
    ("posix2", "c", "-std=c17 -D_POSIX_C_SOURCE=2", "POSIX.2-1992"),
    ("posix199309", "c", "-std=c17 -D_POSIX_C_SOURCE=199309L", "POSIX.1b"),
    ("posix199506", "c", "-std=c17 -D_POSIX_C_SOURCE=199506L", "POSIX.1c"),
    ("posix200112", "c", "-std=c17 -D_POSIX_C_SOURCE=200112L", "POSIX.1-2001"),
    ("posix200809", "c", "-std=c17 -D_POSIX_C_SOURCE=200809L", "POSIX.1-2008"),
    ("xopen", "c", "-std=c17 -D_XOPEN_SOURCE", "_XOPEN_SOURCE defined empty"),
    ("xopen500", "c", "-std=c17 -D_XOPEN_SOURCE=500", "SUSv2"),
    ("xopen600", "c", "-std=c17 -D_XOPEN_SOURCE=600", "SUSv3"),
    ("xopen700", "c", "-std=c17 -D_XOPEN_SOURCE=700", "SUSv4"),
    ("xopenext", "c", "-std=c17 -D_XOPEN_SOURCE -D_XOPEN_SOURCE_EXTENDED", "X/Open UNIX extended"),
    ("isoc99", "c", "-std=c89 -D_ISOC99_SOURCE", "C99 names in C90 mode"),
    ("isoc11", "c", "-std=c99 -D_ISOC11_SOURCE", "C11 names in C99 mode"),
    ("isoc2x", "c", "-std=c17 -D_ISOC2X_SOURCE", "C2X names in C17 mode"),
    ("default", "c", "-std=c17 -D_DEFAULT_SOURCE", "_DEFAULT_SOURCE under strict C17"),
    ("bsd", "c", "-std=c17 -D_BSD_SOURCE", "_BSD_SOURCE under strict C17"),
    ("svid", "c", "-std=c17 -D_SVID_SOURCE", "_SVID_SOURCE under strict C17"),
    ("gnu", "c", "-std=gnu17 -D_GNU_SOURCE", "_GNU_SOURCE"),
    ("lfs64", "c", "-std=gnu17 -D_LARGEFILE64_SOURCE", "the *64 names"),
    ("fob64", "c", "-std=gnu17 -D_FILE_OFFSET_BITS=64", "64-bit file offsets"),
    ("time64", "c", "-std=gnu17 -D_FILE_OFFSET_BITS=64 -D_TIME_BITS=64", "64-bit time"),
    ("time64only", "c", "-std=gnu17 -D_TIME_BITS=64", "_TIME_BITS=64 without _FILE_OFFSET_BITS=64"),
    ("dynstack", "c", "-std=gnu17 -D_DYNAMIC_STACK_SIZE_SOURCE", "run-time stack-size macros"),
    ("reentrant", "c", "-std=gnu17 -D_REENTRANT", "_REENTRANT"),
    ("fortify1", "c", "-std=gnu17 -O2 -D_FORTIFY_SOURCE=1", "fortify level 1 at -O2"),
    ("fortify2", "c", "-std=gnu17 -O2 -D_FORTIFY_SOURCE=2", "fortify level 2 at -O2"),
    ("fortify3", "c", "-std=gnu17 -O2 -D_FORTIFY_SOURCE=3", "fortify level 3 at -O2"),
    ("cxx98", "c++", "-std=c++98", "C++98 (g++ predefines _GNU_SOURCE)"),
    ("cxx11", "c++", "-std=c++11", "C++11"),
    ("cxx14", "c++", "-std=c++14", "C++14"),
    ("cxx17", "c++", "-std=c++17", "C++17"),
    ("cxx20", "c++", "-std=c++20", "C++20"),
    ("cxx23", "c++", "-std=c++23", "C++23"),
]
COMPILER = {"c": "gcc", "c++": "g++"}
BASE = ["-fsyntax-only", "-w", "-fno-builtin", "-fmax-errors=0", "-fno-diagnostics-show-caret", "-fdiagnostics-plain-output",
        "-fno-diagnostics-show-option", "-fdiagnostics-color=never"]
IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
KINDS = ("macro", "typedef", "function", "object", "enumerator", "struct", "union", "enum", "include-error")


def tsv(name):
    with open(os.path.join(ABI, name), newline="") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def c_integer_names():
    """The names C 7.20 (<stdint.h>) and C 7.8.1 (<inttypes.h>) give the integer types of each width: the
    exact-width, minimum-width and fastest types (7.20.1.1-3), the integer-constant macros (7.20.4) and the
    format macros (7.8.1). No other table spells them all (ruling #6047)."""
    out = ["INTMAX_C", "UINTMAX_C"]
    kinds = ["", "LEAST", "FAST"]
    for n in (8, 16, 32, 64):
        out += [f"int{n}_t", f"uint{n}_t", f"int_least{n}_t", f"uint_least{n}_t", f"int_fast{n}_t", f"uint_fast{n}_t",
                f"INT{n}_C", f"UINT{n}_C"]
    sizes = [f"{k}{n}" for k in kinds for n in (8, 16, 32, 64)] + ["MAX", "PTR"]
    # C23 7.8.1 adds the binary conversions `b` and `B` (PRIb*, PRIB*, SCNb*).
    out += [f"PRI{c}{s}" for c in "diouxXbB" for s in sizes]
    out += [f"SCN{c}{s}" for c in "diouxXb" if c != "X" for s in sizes]
    return out


def standard_names():
    """Names that standards and man pages give headers whose names no other table holds (slice 17): the POSIX
    <tar.h> and <cpio.h> constants (XBD 13), the exit codes of sysexits(3), the queue macros of list(3), slist(3),
    stailq(3), tailq(3) and circleq(3), and the obstack macros of the GNU C Library manual (Obstacks). Also (slice 8): names the standards give that no other table spells: the integer type of the signal-handler flag
    (`sig_atomic_t`, C 7.14 and POSIX <signal.h>), the notification structure's type name (`sigevent_t`,
    POSIX <signal.h>), and the mutex kinds of pthread_mutexattr_settype (POSIX: NORMAL, ERRORCHECK, RECURSIVE,
    DEFAULT) and the GNU timed kind `PTHREAD_MUTEX_TIMED_NP`, the name the layout notes of the pthread
    types use for kind 0 (crates/abi/src/layout_init.rs). Also the other pthread attribute values that are
    enumeration constants, which constants.tsv (macros only) cannot hold: the priority protocols
    (pthread_mutexattr_setprotocol(3): PTHREAD_PRIO_NONE, _INHERIT, _PROTECT), the robustness values
    (pthread_mutexattr_setrobust(3): PTHREAD_MUTEX_STALLED, _ROBUST and their `_NP` spellings) and the lock
    preference kinds (pthread_rwlockattr_setkind_np(3): PTHREAD_RWLOCK_PREFER_READER_NP, _PREFER_WRITER_NP,
    _PREFER_WRITER_NONRECURSIVE_NP, _DEFAULT_NP)."""
    out = ["TMAGIC", "TMAGLEN", "TVERSION", "TVERSLEN", "REGTYPE", "AREGTYPE", "LNKTYPE", "SYMTYPE", "CHRTYPE",
           "BLKTYPE", "DIRTYPE", "FIFOTYPE", "CONTTYPE", "TSUID", "TSGID", "TSVTX", "TUREAD", "TUWRITE", "TUEXEC",
           "TGREAD", "TGWRITE", "TGEXEC", "TOREAD", "TOWRITE", "TOEXEC", "MAGIC"]
    out += [f"C_I{c}" for c in ("RUSR", "WUSR", "XUSR", "RGRP", "WGRP", "XGRP", "ROTH", "WOTH", "XOTH", "SUID", "SGID",
                                "SVTX", "SDIR", "SFIFO", "SREG", "SBLK", "SCHR", "SCTG", "SLNK", "SSOCK")]
    out += [f"EX_{c}" for c in ("OK", "_BASE", "USAGE", "DATAERR", "NOINPUT", "NOUSER", "NOHOST", "UNAVAILABLE", "SOFTWARE",
                                "OSERR", "OSFILE", "CANTCREAT", "IOERR", "TEMPFAIL", "PROTOCOL", "NOPERM", "CONFIG", "_MAX")]
    common = ["ENTRY", "EMPTY", "FIRST", "NEXT", "HEAD", "HEAD_INITIALIZER", "INIT", "FOREACH", "REMOVE"]
    more = {"LIST": ["INSERT_AFTER", "INSERT_BEFORE", "INSERT_HEAD"],
            "SLIST": ["INSERT_AFTER", "INSERT_HEAD", "REMOVE_AFTER", "REMOVE_HEAD"],
            "STAILQ": ["CONCAT", "INSERT_AFTER", "INSERT_HEAD", "INSERT_TAIL", "LAST", "REMOVE_AFTER", "REMOVE_HEAD"],
            "TAILQ": ["CONCAT", "FOREACH_REVERSE", "INSERT_AFTER", "INSERT_BEFORE", "INSERT_HEAD", "INSERT_TAIL", "LAST", "PREV"],
            "CIRCLEQ": ["FOREACH_REVERSE", "INSERT_AFTER", "INSERT_BEFORE", "INSERT_HEAD", "INSERT_TAIL", "LAST",
                        "LOOP_NEXT", "LOOP_PREV", "PREV"]}
    for q, ms in more.items():
        out += [f"{q}_{m}" for m in common + ms]
    out += ["SLIST_FOREACH_SAFE", "LIST_FOREACH_SAFE", "STAILQ_FOREACH_SAFE", "TAILQ_FOREACH_SAFE", "CIRCLEQ_FOREACH_SAFE"]
    out += [f"obstack_{m}" for m in ("init", "alloc", "copy", "copy0", "free", "blank", "grow", "grow0", "1grow", "ptr_grow",
                                     "int_grow", "finish", "object_size", "room", "make_room", "blank_fast", "1grow_fast",
                                     "ptr_grow_fast", "int_grow_fast", "base", "next_free", "chunk_size",
                                     "alignment_mask", "chunk_alloc", "chunk_free")]
    out +=["sig_atomic_t", "sigevent_t", "PTHREAD_MUTEX_NORMAL", "PTHREAD_MUTEX_RECURSIVE",
            "PTHREAD_MUTEX_ERRORCHECK", "PTHREAD_MUTEX_DEFAULT", "PTHREAD_MUTEX_TIMED_NP",
            "PTHREAD_PRIO_NONE", "PTHREAD_PRIO_INHERIT", "PTHREAD_PRIO_PROTECT",
            "PTHREAD_MUTEX_STALLED", "PTHREAD_MUTEX_ROBUST", "PTHREAD_MUTEX_STALLED_NP", "PTHREAD_MUTEX_ROBUST_NP",
            "PTHREAD_RWLOCK_PREFER_READER_NP", "PTHREAD_RWLOCK_PREFER_WRITER_NP",
            "PTHREAD_RWLOCK_PREFER_WRITER_NONRECURSIVE_NP", "PTHREAD_RWLOCK_DEFAULT_NP"]
    return out


def math_names():
    """The identifiers of the math group of headers (slice 11), which no other table spells: the types and
    macros of C 7.12 (<math.h>: float_t, double_t, HUGE_VAL*, SNAN*, FP_FAST_*), of the TS 18661 and C2X extended
    suffixes (F32 ... F64X), the POSIX XSI constants (M_E ... M_SQRT1_2), the SVID error names and the SVID
    <values.h> constants, C 7.3 (<complex.h>: _Complex_I, CMPLX*), C 7.6 (<fenv.h>) and the x87 control-word names
    of <fpu_control.h>. Only names: their visibility, types and values are probed (ruling #6047 for the pattern)."""
    suffixes = ["", "f", "l", "f32", "f64", "f128", "f32x", "f64x"]
    out = ["isgreaterequal", "isless", "islessequal", "islessgreater", "float_t", "double_t", "MAXFLOAT", "HUGE", "X_TLOSS", "DOMAIN", "SING", "OVERFLOW", "UNDERFLOW", "TLOSS",
           "PLOSS", "exception", "matherr", "_LIB_VERSION", "_IEEE_", "_SVID_", "_XOPEN_", "_POSIX_", "_ISOC_"]
    out += [f"HUGE_VAL{s.upper()}" for s in suffixes] + [f"SNAN{s.upper()}" for s in suffixes]
    out += ["FP_FAST_FMA", "FP_FAST_FMAF", "FP_FAST_FMAL"]
    for op in ("ADD", "SUB", "MUL", "DIV", "FMA", "SQRT"):
        out += [f"FP_FAST_{p}{op}{s}" for p in ("F", "D") for s in ("", "L") if not (p == "D" and s == "")]
    for c in ("E", "LOG2E", "LOG10E", "LN2", "LN10", "PI", "PI_2", "PI_4", "1_PI", "2_PI", "2_SQRTPI", "SQRT2", "SQRT1_2"):
        out += [f"M_{c}{s}" for s in suffixes]
    out += ["complex", "imaginary", "_Complex_I", "_Imaginary_I", "I"] + [f"CMPLX{s.upper()}" for s in suffixes]
    out += ["fenv_t", "fexcept_t", "femode_t", "FE_DFL_ENV", "FE_NOMASK_ENV", "FE_DFL_MODE", "FE_SNANS_ALWAYS_SIGNAL",
            "FE_ENABLE", "FE_DISABLE"]
    out += ["_FPU_MASK_IM", "_FPU_MASK_DM", "_FPU_MASK_ZM", "_FPU_MASK_OM", "_FPU_MASK_UM", "_FPU_MASK_PM",
            "_FPU_EXTENDED", "_FPU_DOUBLE", "_FPU_SINGLE", "_FPU_RC_NEAREST", "_FPU_RC_DOWN", "_FPU_RC_UP",
            "_FPU_RC_ZERO", "_FPU_RESERVED", "_FPU_DEFAULT", "_FPU_IEEE", "_FPU_GETCW", "_FPU_SETCW", "fpu_control_t",
            "__fpu_control"]
    out += ["BITSPERBYTE", "BITS", "MAXSHORT", "MAXINT", "MAXLONG", "MINSHORT", "MININT", "MINLONG", "HIBITS", "HIBITL",
            "HIBITI", "DMAXEXP", "FMAXEXP", "DMINEXP", "FMINEXP", "DSIGNIF", "FSIGNIF", "DMAXPOWTWO", "FMAXPOWTWO",
            "_DEXPLEN", "_FEXPLEN", "_HIDDENBIT", "LN_MAXDOUBLE", "LN_MINDOUBLE", "MAXDOUBLE", "MINDOUBLE", "MINFLOAT"]
    return out


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
    # HEADERS-GETTEXT-CLANG (Yohane's stage 2 on snapshot 2026-11-25.2): the struct utmp compatibility member names
    # sysvinit and procps use (utmp(5): `ut_name`, `ut_time`, `ut_addr`; `ut_xtime` beside them) and the login
    # accounting file name macros of the same header; psutil's struct ifaddrs union member names (getifaddrs(3):
    # `ifa_broadaddr`, `ifa_dstaddr`); ruby's register type `greg_t` (getcontext(3)'s mcontext_t member type); and
    # the libc revision test of gettext's AM_GNU_GETTEXT (gettext.m4: `__GNU_GETTEXT_SUPPORTED_REVISION`).
    out += ["ut_name", "ut_time", "ut_xtime", "ut_addr", "UTMP_FILENAME", "WTMP_FILENAME", "ifa_broadaddr",
            "ifa_dstaddr", "greg_t", "__GNU_GETTEXT_SUPPORTED_REVISION"]
    return out


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


def guard_candidates(header):
    """The include-guard names a program may test for `header` (HEADERS-STAGE2-GAPS-C; gmp's <gmp.h> declares its
    `FILE *` functions only when it sees a stdio guard): the conventional shapes, generated from the header's path
    (`sys/types.h` -> `_SYS_TYPES_H`, `__SYS_TYPES_H`, ...). A candidate is only a name to test: which headers define
    it, and its value, are probed. xtask/src/headers.rs `guard_candidates` spells the same list."""
    p = re.sub(r"[^A-Za-z0-9]", "_", header[:-2] if header.endswith(".h") else header).upper()
    return [f"_{p}_H", f"__{p}_H", f"_{p}_H_", f"__{p}_H__", f"{p}_H", f"_{p}_H_INCLUDED", f"_{p}_INCLUDED"]


def stage2_names():
    """(names, tags) of HEADERS-STAGE2-GAPS-C (Yohane stage 2: coreutils with gmp, bzip2, cmake, elfutils, bsd-mailx,
    capnproto): the include guards of every public header (guard_candidates); the archive header of ar(5) (`struct
    ar_hdr`, ARMAG, SARMAG, ARFMAG); the BSD default path macros (bsd-mailx names `_PATH_DEVNULL`; the rest of the
    family the BSD paths(3) set and the FHS name); the d_type conversions `DTTOIF` and `IFTODT` (capnproto);
    `struct utimbuf` of XSH <utime.h> (bzip2, cmake)."""
    out = [g for h in public_headers() for g in guard_candidates(h)]
    out += ["ARMAG", "SARMAG", "ARFMAG", "DTTOIF", "IFTODT"]
    # Coordinator note (Yohane 12:25Z): C11's `static_assert` of <assert.h> (elogind), the syslog(3) priority macros
    # (eudev); the mount-option and file-system-type names of getmntent(3) / fstab(5) (busybox's swaponoff).
    out += ["static_assert", "LOG_PRI", "LOG_MAKEPRI", "LOG_FAC", "LOG_PRIMASK", "LOG_FACMASK", "LOG_NFACILITIES",
            "LOG_MASK", "LOG_UPTO"]
    out += [f"MNTTYPE_{t}" for t in ("IGNORE", "NFS", "SWAP")]
    out += [f"MNTOPT_{o}" for o in ("DEFAULTS", "RO", "RW", "SUID", "NOSUID", "NOAUTO", "AUTO", "EXEC", "NOEXEC",
                                    "DEV", "NODEV", "USER", "NOUSER", "SYNC", "ASYNC")]
    out += [f"_PATH_{p}" for p in (
        "CONSOLE", "CSHELL", "DEVDB", "DEVNULL", "DEVZERO", "DRUM", "HEQUIV", "KMEM", "LASTLOG", "MAILDIR", "MAN", "MEM",
        "NOLOGIN", "PRESERVE", "RWHODIR", "SENDMAIL", "SHELLS", "STDPATH", "UNIX", "VI", "VARDB", "VARRUN", "VARTMP",
        "VARLOG", "VARSPOOL", "BOOT", "CP", "ECHO", "MV", "NOLOGIN_TXT", "RSH", "SH", "TMPDIR", "AUDIT", "HOSTS",
        "NETWORKS", "PROTOCOLS", "RESCONF", "LOCALE", "ZONEINFO", "CONSOLE_DEV", "ETC", "DEVDB_DIR", "MKDIR")]
    return out, ["ar_hdr", "utimbuf"]


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
    # member alias sax25_uid, which the reference shows but which are no integer constant in a unit including their
    # header alone (type_probe.py finds no value; the generator needs one for every macro row): a later story.
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
    """(ordinary names, tag names) from the existing spec/abi tables."""
    names, tags = set(), set()
    names.update(r["name"] for r in tsv("prototypes.tsv"))
    names.update(r["macro"] for r in tsv("constants.tsv"))
    names.update(r["symbol"] for r in tsv("versions.tsv") if r["type"] == "OBJECT")
    spelled = [r["declaration"] for r in tsv("prototypes.tsv")]
    for r in tsv("layouts.tsv"):
        spelled.append(r["member_type"])
        t = r["type"]
        m = re.match(r"^(struct|union|enum)\s+(\w+)$", t)
        if m:
            tags.add(m.group(2))
        elif IDENT.match(t):
            names.add(t)
    for text in spelled:
        text = text or ""
        for m in re.finditer(r"\b(struct|union|enum)\s+([A-Za-z_]\w*)", text):
            tags.add(m.group(2))
        cleaned = re.sub(r"\b(struct|union|enum)\s+[A-Za-z_]\w*", " ", text)
        names.update(re.findall(r"[A-Za-z_]\w*", cleaned))
    # Identifiers the functional spec writes in backticks (`errno`, `HUGE_VAL`, `pid_t`, `S_ISDIR()`): names
    # the integer and prototype tables cannot hold.
    with open(os.path.join(ROOT, "glibc-functional-spec.md"), encoding="utf-8") as f:
        names.update(re.findall(r"`([A-Za-z_][A-Za-z0-9_]*)(?:\(\))?`", f.read()))
    names.update(c_integer_names())
    names.update(standard_names())
    names.update(math_names())
    names.update(package_names())
    # The public type name of each reserved one (`__blksize_t` spells struct stat's member, `blksize_t` is the
    # name XBD <sys/types.h> gives programs): a header that shows the reserved type may show its public name.
    names.update(n[2:] for n in list(names) if n.startswith("__") and n.endswith("_t") and not n.startswith("___"))
    net, net_tags = net_names()
    names.update(net)
    tags.update(net_tags)
    bb, bb_tags = busybox_names()
    names.update(bb)
    tags.update(bb_tags)
    names.update(stage2e_names())
    s2, s2_tags = stage2_names()
    names.update(s2)
    tags.update(s2_tags)
    rn, r_tags = radio_names()
    names.update(rn)
    tags.update(r_tags)
    tn, tn_tags = thin_names()
    names.update(tn)
    tags.update(tn_tags)
    ns, ns_tags = net_sys_names()
    names.update(ns)
    tags.update(ns_tags)
    tags.update(["exception", "ieee754_float", "ieee754_double", "ieee754_long_double", "ieee854_long_double"])
    # The enumerators and enumeration tags of enumerators.tsv (HEADERS-ENUM-FOLLOWUPS): `_REG_NOMATCH`, the
    # enumerator the macro REG_NOMATCH names, its typedef `reg_errcode_t`, and `ns_t_a` of `enum __ns_type` are
    # spelled in no other table.
    for r in tsv("enumerators.tsv"):
        if r["tag"].startswith("enum "):
            tags.add(r["tag"][len("enum "):])
        elif r["tag"].startswith("typedef "):
            names.add(r["tag"][len("typedef "):])
        names.add(r["name"])
    # The reserved names headers-gen emits by policy (headers/reserved.tsv `emit`, rulings R-H2, R-H3): the generator
    # declares them only where this table shows them, so they are probed whether or not another table spells them
    # (slice 20: `__locale_t`, which the C++ library's locale support names, appears in no prototype).
    with open(os.path.join(ROOT, "headers", "reserved.tsv"), newline="") as f:
        names.update(r["name"] for r in csv.DictReader((l for l in f if not l.startswith("#")), delimiter="\t")
                     if r["policy"] == "emit")
    names = {n for n in names if IDENT.match(n) and not n.startswith("__builtin")}
    tags = {n for n in tags if IDENT.match(n)}
    return sorted(names), sorted(tags)


def _compile(lang, flags, path):
    """(exit status, line numbers of errors located in `path`, count of errors located elsewhere, command)."""
    # Without macro-expansion tracking an error inside a macro's expansion is located at the line that uses the macro
    # where the compiler can do so; errors outside macros are located as before.
    cmd = [COMPILER[lang]] + flags.split() + BASE + ["-ftrack-macro-expansion=0", "-x", "c" if lang == "c" else "c++", path]
    p = subprocess.run(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    # Only the location prefix of error lines is matched; the message after it is never looked at.
    pat = re.compile(r"^(.+?):(\d+):\d+: (?:fatal )?error:")
    lines, elsewhere = set(), 0
    for raw in p.stderr.splitlines():
        m = pat.match(raw.decode("utf-8", "replace"))
        if m and m.group(1) == path:
            lines.add(int(m.group(2)))
        elif m:
            elsewhere += 1
    return p.returncode, lines, elsewhere, cmd


def _located(rc, lines, elsewhere):
    """Whether a compile's outcome is known line by line: it passed, or every error is on a line of the unit."""
    return rc == 0 or (lines and not elsewhere)


def error_lines(lang, flags, path):
    """(exit status, set of line numbers of errors located in `path`). Nothing else is read."""
    rc, lines, elsewhere, cmd = _compile(lang, flags, path)
    if not _located(rc, lines, elsewhere):
        raise RuntimeError(f"{os.path.basename(path)}: the unit failed outside its own test lines ({' '.join(cmd[:2])} ...)")
    return rc, lines


class Unit:
    """A probe translation unit: each test on its own line, remembered by line number."""

    def __init__(self, header):
        self.lines, self.tests = [], {}
        if header:
            self.lines.append(f"#include <{header}>")

    def raw(self, text):
        self.lines.append(text)

    def test(self, key, text):
        self.lines.append(text)
        self.tests[len(self.lines)] = key

    def _write(self, path, active):
        """Write the unit with only the test lines in `active` (the others blank, so line numbers do not move)."""
        with open(path, "w") as f:
            f.write("\n".join("" if i + 1 in self.tests and i + 1 not in active else t
                              for i, t in enumerate(self.lines)) + "\n")

    def run(self, lang, flags, path):
        """(exit status, keys of the tests that fail to compile)."""
        every = set(self.tests)
        self._write(path, every)
        rc, errs, elsewhere, cmd = _compile(lang, flags, path)
        if not _located(rc, errs, elsewhere):
            # An error located outside the unit (an object-like alias of a function-like macro, `GETLONG` for
            # `NS_GET32`, used as a value is reported at the alias's definition in the header, whatever the
            # tracking flag): halve the set of tests until each such failure is pinned to one test. A unit that
            # fails with no test at all is the header's own failure, as before.
            self._write(path, set())
            if not _located(*_compile(lang, flags, path)[:3]):
                raise RuntimeError(f"{os.path.basename(path)}: the unit failed outside its own test lines "
                                   f"({' '.join(cmd[:2])} ...)")
            errs = self._bisect(lang, flags, path, sorted(every))
            rc = 1
        return rc, {key for line, key in self.tests.items() if line in errs}

    def _bisect(self, lang, flags, path, active):
        """The lines of the tests in `active` that fail, each test of an unlocated failure found by halving."""
        self._write(path, set(active))
        rc, errs, elsewhere, _ = _compile(lang, flags, path)
        if _located(rc, errs, elsewhere):
            return errs
        if len(active) == 1:
            return set(active)
        half = len(active) // 2
        return self._bisect(lang, flags, path, active[:half]) | self._bisect(lang, flags, path, active[half:])


def keywords(lang, names, work):
    """Candidates the compiler itself reserves (keywords) in the widest mode of `lang`: excluded there."""
    u = Unit(None)
    for n in names:
        u.test(n, f"struct vis_kw_{n} {{ int {n}; }};")
    # -undef drops the compiler's predefined macros and -nostdinc the implicitly included stdc-predef.h, whose
    # macros (`_STDC_PREDEF_H`, `__STDC_ISO_10646__`) would otherwise fail this test and be taken for keywords.
    flags = "-std=gnu2x -undef -nostdinc" if lang == "c" else "-std=c++23 -undef -nostdinc"
    _, failed = u.run(lang, flags, os.path.join(work, f"kw.{lang}"))
    return failed


def probe(header, cfg, names, tags, work, macro_only=()):
    """{(name, kind)} visible after `#include <header>` (header None: the empty unit) in `cfg`.

    `macro_only` (the keywords of the language) get the `#ifdef` test alone: `#ifdef` takes any identifier, and a
    header can define one as a macro in an older mode (`true` and `false` from stdbool.h before C23)."""
    cid, lang, flags, _ = cfg
    tag = (header or "-").replace("/", "_")
    path = os.path.join(work, f"{tag}.{cid}.{'c' if lang == 'c' else 'cc'}")
    if header:
        rc = subprocess.run([COMPILER[lang]] + flags.split() + BASE + ["-x", "c" if lang == "c" else "c++", "-"],
                            input=f"#include <{header}>\n".encode(), stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL).returncode
        if rc != 0:
            return {("-", "include-error")}
    cxx = lang == "c++"
    # Pass A: macros, then declared ordinary names and complete tags.
    a = Unit(header)
    for n in list(names) + list(macro_only):
        a.raw(f"#ifdef {n}")
        a.test(("macro", n), "#error vis")
        a.raw("#endif")
    for n in names:
        a.raw(f"#undef {n}")
    for n in names:
        # Conflicts with any ordinary declaration of n (function, object, typedef, enumerator): the error
        # marks a declared name, so the many undeclared candidates cost no diagnostic at all.
        a.test(("decl", n), f"struct vis_g_{n}; extern {'' if cxx else 'struct '}vis_g_{n} {n};")
    for n in tags:
        for kw in ("struct", "union", "enum"):
            a.test((kw, n), f"void vis_{kw[0]}_{n}(void) {{ (void)sizeof({kw} {n}); }}")
    _, fa = a.run(lang, flags, path)
    out = {(n, "macro") for n in list(names) + list(macro_only) if ("macro", n) in fa}
    for n in tags:
        for kw in ("struct", "union", "enum"):
            if (kw, n) not in fa:
                out.add((n, kw))
    declared = [n for n in names if ("decl", n) in fa]
    if not declared:
        return out
    # Pass B: classify the declared names.
    b = Unit(header)
    for n in names:
        b.raw(f"#undef {n}")
    if cxx:
        b.raw("template<class A, class B> struct vis_same { enum { vis_v = 1 }; };")
        b.raw("template<class A> struct vis_same<A, A> { enum { vis_v = -1 }; };")
        b.raw("template<class T> T vis_decay(T);")
    for n in declared:
        if cxx:
            b.test(("type", n), f"void vis_t_{n}() {{ typedef {n} vis_x_{n}; }}")
            for kw in ("struct", "union", "enum"):
                b.test(("ptr-" + kw, n), f"void vis_p{kw[0]}_{n}() {{ {kw} {n} *vis_y_{n} = 0; (void)vis_y_{n}; }}")
            b.test(("obj", n), f"extern char vis_o_{n}[vis_same<__typeof__(vis_decay({n})), __typeof__(&{n})>::vis_v];")
        else:
            b.test(("type", n), f"void vis_t_{n}(void) {{ (void)sizeof({n} *); }}")
            b.test(("fn", n), f"extern char vis_f_{n}[__builtin_types_compatible_p(__typeof__((0, {n})), __typeof__(&{n})) ? 1 : -1];")
        b.test(("ice", n), f"enum {{ vis_e_{n} = ({n}) }};")
    _, fb = b.run(lang, flags, path)
    for n in declared:
        is_type = ("type", n) not in fb
        if cxx:
            if is_type:
                if all(("ptr-" + kw, n) in fb for kw in ("struct", "union", "enum")):
                    out.add((n, "typedef"))
                continue
            if ("ice", n) not in fb:
                out.add((n, "enumerator"))
            elif ("obj", n) not in fb:
                out.add((n, "object"))
            else:
                out.add((n, "function"))
        else:
            if is_type:
                out.add((n, "typedef"))
            elif ("fn", n) not in fb:
                out.add((n, "function"))
            elif ("ice", n) not in fb:
                out.add((n, "enumerator"))
            else:
                out.add((n, "object"))
    return out


def public_headers():
    with open(INVENTORY, newline="") as f:
        rows = list(csv.DictReader(f, delimiter="\t"))
    return sorted(r["header"] for r in rows if r["kind"] == "public" and r["header"].endswith(".h"))


def uapi_flags():
    """`-isystem` for the kernel UAPI directories (linux-libc-dev) that slice 9 headers include: $NIJI_UAPI (the tree
    `cargo xtask so` stages at target/niji/uapi) or, when unset, the build host's multiarch and plain include directories,
    placed after libniji's tree so the host's libc headers stay out of reach."""
    d = os.environ.get("NIJI_UAPI")
    if d:
        return ["-isystem", d]
    return ["-isystem", "/usr/include/x86_64-linux-gnu", "-isystem", "/usr/include"] if os.path.isdir("/usr/include/linux") else []

def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--headers", default="")
    ap.add_argument("--out", default=ABI)
    ap.add_argument("--names", default="", help="FILE: probe only the candidates it lists (whitespace-separated)")
    ap.add_argument("--include", default="", help="probe the headers under DIR (libniji's libc-include/) instead of the system's")
    args = ap.parse_args()
    if args.include:
        gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
        BASE.extend(["-nostdinc", "-isystem", os.path.abspath(args.include), "-isystem", gccinc, *uapi_flags()])
    headers = args.headers.split(",") if args.headers else public_headers()
    names, tags = candidates()
    if args.names:
        # A name's rows depend on that name's units only, so probing a subset gives exactly the full run's rows
        # for those names (rows to splice into the committed table).
        with open(args.names) as f:
            only = set(f.read().split())
        names, tags = [n for n in names if n in only], [t for t in tags if t in only]
    work = tempfile.mkdtemp(prefix="header_probe.")
    try:
        langs = sorted({c[1] for c in CONFIGS})
        kw = {lang: keywords(lang, sorted(set(names) | set(tags)), work) for lang in langs}
        per_lang = {lang: ([n for n in names if n not in kw[lang]], [t for t in tags if t not in kw[lang]]) for lang in langs}
        jobs = [(h, c) for h in [None] + headers for c in CONFIGS]
        with ThreadPoolExecutor(max_workers=args.jobs) as ex:
            results = list(ex.map(lambda hc: probe(hc[0], hc[1], *per_lang[hc[1][1]], work, sorted(kw[hc[1][1]])), jobs))
    finally:
        shutil.rmtree(work, ignore_errors=True)
    ids = [c[0] for c in CONFIGS]
    seen = {}
    for (h, c), res in zip(jobs, results):
        for name, kind in res:
            seen.setdefault((h or "-", name, kind), set()).add(c[0])
    base = {(n, k): cs for (h, n, k), cs in seen.items() if h == "-"}
    rows = []
    for (h, n, k), cs in seen.items():
        if h != "-":
            cs = cs - base.get((n, k), set())
        if cs:
            listed = ",".join(i for i in ids if i in cs)
            rest = "!" + ",".join(i for i in ids if i not in cs)
            rows.append((h, n, k, "*" if len(cs) == len(ids) else min(listed, rest, key=len)))
    rows.sort(key=lambda r: (r[0] != "-", r[0], r[1], KINDS.index(r[2])))
    os.makedirs(args.out, exist_ok=True)
    with open(os.path.join(args.out, "ftm-configs.tsv"), "w", newline="\n") as f:
        f.write("id\tlanguage\tflags\tprobes\n")
        for c in CONFIGS:
            f.write("\t".join(c) + "\n")
    with open(os.path.join(args.out, "visibility.tsv"), "w", newline="\n") as f:
        f.write("header\tname\tkind\tconfigs\n")
        for r in rows:
            f.write("\t".join(r) + "\n")
    excluded = {lang: len(kw[lang]) for lang in langs}
    print(f"headers {len(headers)}  configurations {len(CONFIGS)}  candidates {len(names)} names + {len(tags)} tags"
          f"  keywords excluded {excluded}  rows {len(rows)}")


if __name__ == "__main__":
    sys.exit(main())
