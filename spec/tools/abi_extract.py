#!/usr/bin/env python3
"""Regenerate the machine-readable parts of spec Appendix C (ABI data).

Sources, per the owner's decision on OD-1 (interface declarations, not implementation):
  * the installed public headers (glibc 2.39, x86_64), read only through the compiler:
    type layouts (DWARF of a probe that declares one object per type), sizes and alignments,
    static-initializer bytes, function prototypes (gcc -aux-info), numeric macro values;
  * the dynamic symbol tables of the installed libraries (readelf): symbol versions,
    default-vs-compat binding, and object sizes of exported data symbols;
  * spec/abi/versions-additions.tsv: the exports libniji adds beyond the reference, each with its
    decision record, merged into versions.tsv (versions.sha256 pins the reference rows alone);
  * spec/abi/prototypes-additions.tsv: the prototypes of those exports, the same way, merged into
    prototypes.tsv (prototypes.sha256 pins the reference rows alone).

Run on the reference system (Ubuntu 24.04, glibc 2.39):  python3 spec/tools/abi_extract.py
Outputs go to spec/abi/. Nothing here reads glibc source code. `--versions-only` regenerates
versions.tsv and versions.sha256 alone; `--prototypes-only` prototypes.tsv and prototypes.sha256 alone.
The full run and `--empty-versions-only` record what they did as a fragment,
spec/abi/provenance.d/<STORY-ID>.txt (`--story STORY-ID`, required for them; D-X.172): spec/abi/PROVENANCE
is generated from the fragments and the tables by `cargo xtask records-gen`, never written here.
"""
import csv, os, re, subprocess, sys, tempfile, collections, hashlib

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "abi")
LIBDIR = "/lib/x86_64-linux-gnu"

HEADERS = """stdio.h stdio_ext.h stdlib.h string.h strings.h wchar.h wctype.h ctype.h locale.h
langinfo.h monetary.h iconv.h uchar.h math.h fenv.h complex.h setjmp.h signal.h ucontext.h
pthread.h threads.h semaphore.h time.h sys/time.h sys/times.h sys/timeb.h sys/timex.h
sys/resource.h sys/utsname.h sys/sysinfo.h sys/stat.h sys/statfs.h sys/statvfs.h dirent.h
fts.h ftw.h glob.h regex.h wordexp.h fnmatch.h search.h termios.h sys/ioctl.h pty.h utmp.h
utmpx.h pwd.h grp.h shadow.h gshadow.h netdb.h resolv.h arpa/inet.h arpa/nameser.h
netinet/in.h netinet/ether.h net/if.h ifaddrs.h sys/socket.h sys/un.h poll.h sys/epoll.h
sys/select.h sys/uio.h aio.h mqueue.h sys/ipc.h sys/msg.h sys/sem.h sys/shm.h spawn.h
sys/wait.h dlfcn.h link.h mntent.h ar.h utime.h fstab.h getopt.h argp.h obstack.h printf.h malloc.h err.h
error.h syslog.h fmtmsg.h sys/mman.h fcntl.h unistd.h sched.h sys/sysmacros.h sys/inotify.h
sys/signalfd.h sys/timerfd.h sys/eventfd.h sys/xattr.h sys/random.h sys/auxv.h execinfo.h
inttypes.h stdint.h stddef.h stdarg.h envz.h argz.h libgen.h nl_types.h libintl.h
thread_db.h proc_service.h sys/ptrace.h sys/prctl.h gnu/libc-version.h sys/single_threaded.h sys/rseq.h
stdbit.h sys/pidfd.h sys/mount.h sys/fanotify.h sys/quota.h sys/swap.h sys/klog.h
sys/reboot.h ttyent.h aliases.h rpc/netdb.h ulimit.h sys/profil.h sys/gmon.h mcheck.h nss.h
sys/platform/x86.h sys/fsuid.h sys/personality.h sys/io.h sys/file.h sys/sendfile.h
sys/user.h sys/procfs.h sys/vlimit.h sys/vtimes.h sys/ustat.h
elf.h netinet/ip.h netinet/ip6.h netinet/tcp.h netinet/udp.h netinet/ip_icmp.h netinet/icmp6.h
net/ethernet.h net/if_arp.h netpacket/packet.h utime.h ieee754.h
net/route.h netinet/if_ether.h arpa/tftp.h sys/mtio.h""".split()

# (label, C type expression). Labels are the names used in Appendix C.
TYPES = [
 ("FILE", "FILE"), ("fpos_t", "fpos_t"), ("cookie_io_functions_t", "cookie_io_functions_t"),
 ("va_list", "va_list"), ("max_align_t", "max_align_t"),
 ("div_t", "div_t"), ("ldiv_t", "ldiv_t"), ("lldiv_t", "lldiv_t"), ("imaxdiv_t", "imaxdiv_t"),
 ("struct drand48_data", "struct drand48_data"), ("struct random_data", "struct random_data"),
 ("mbstate_t", "mbstate_t"), ("struct __locale_struct", "struct __locale_struct"),
 ("struct lconv", "struct lconv"),
 ("jmp_buf", "jmp_buf"), ("sigjmp_buf", "sigjmp_buf"), ("struct __jmp_buf_tag", "struct __jmp_buf_tag"),
 ("sigset_t", "sigset_t"), ("siginfo_t", "siginfo_t"), ("struct sigaction", "struct sigaction"),
 ("stack_t", "stack_t"), ("struct sigstack", "struct sigstack"), ("ucontext_t", "ucontext_t"),
 ("mcontext_t", "mcontext_t"), ("union sigval", "union sigval"), ("struct sigevent", "struct sigevent"),
 ("pthread_t", "pthread_t"), ("pthread_attr_t", "pthread_attr_t"), ("pthread_mutex_t", "pthread_mutex_t"),
 ("pthread_mutexattr_t", "pthread_mutexattr_t"), ("pthread_cond_t", "pthread_cond_t"),
 ("pthread_condattr_t", "pthread_condattr_t"), ("pthread_rwlock_t", "pthread_rwlock_t"),
 ("pthread_rwlockattr_t", "pthread_rwlockattr_t"), ("pthread_spinlock_t", "pthread_spinlock_t"),
 ("pthread_barrier_t", "pthread_barrier_t"), ("pthread_barrierattr_t", "pthread_barrierattr_t"),
 ("pthread_key_t", "pthread_key_t"), ("pthread_once_t", "pthread_once_t"),
 ("struct _pthread_cleanup_buffer", "struct _pthread_cleanup_buffer"),
 ("__pthread_unwind_buf_t", "__pthread_unwind_buf_t"), ("struct __cancel_jmp_buf_tag", "struct __cancel_jmp_buf_tag"),
 ("sigevent_t", "sigevent_t"),
 ("struct sigcontext", "struct sigcontext"),
 ("struct _fpstate", "struct _fpstate"), ("struct _fpxreg", "struct _fpxreg"), ("struct _xmmreg", "struct _xmmreg"),
 ("struct _libc_fpstate", "struct _libc_fpstate"),
 ("struct _libc_fpxreg", "struct _libc_fpxreg"),
 ("struct _libc_xmmreg", "struct _libc_xmmreg"),
 ("struct __pthread_mutex_s", "struct __pthread_mutex_s"),
 ("struct __pthread_rwlock_arch_t", "struct __pthread_rwlock_arch_t"),
 ("struct __pthread_cond_s", "struct __pthread_cond_s"),
 ("struct __pthread_internal_list", "struct __pthread_internal_list"),
 ("__sigset_t", "__sigset_t"),
 ("cpu_set_t", "cpu_set_t"),
 ("struct sched_param", "struct sched_param"), ("sem_t", "sem_t"),
 ("union ieee754_float", "union ieee754_float"), ("union ieee754_double", "union ieee754_double"),
 ("union ieee854_long_double", "union ieee854_long_double"),
 ("thrd_t", "thrd_t"), ("mtx_t", "mtx_t"), ("cnd_t", "cnd_t"), ("tss_t", "tss_t"), ("once_flag", "once_flag"),
 ("struct timespec", "struct timespec"), ("struct timeval", "struct timeval"), ("struct tm", "struct tm"),
 ("struct itimerval", "struct itimerval"), ("struct itimerspec", "struct itimerspec"),
 ("struct timezone", "struct timezone"), ("struct timex", "struct timex"),
 ("struct ntptimeval", "struct ntptimeval"), ("struct tms", "struct tms"), ("struct timeb", "struct timeb"),
 ("struct rusage", "struct rusage"), ("struct rlimit", "struct rlimit"), ("struct utsname", "struct utsname"),
 ("struct sysinfo", "struct sysinfo"), ("struct stat", "struct stat"), ("struct statx", "struct statx"), ("struct statx_timestamp", "struct statx_timestamp"),
 ("struct statfs", "struct statfs"), ("struct statvfs", "struct statvfs"),
 ("struct dirent", "struct dirent"), ("struct dirent64", "struct dirent64"),
 ("FTS", "FTS"), ("FTSENT", "FTSENT"), ("struct FTW", "struct FTW"), ("glob_t", "glob_t"),
 ("regex_t", "regex_t"), ("regmatch_t", "regmatch_t"), ("struct re_registers", "struct re_registers"),
 ("wordexp_t", "wordexp_t"), ("ENTRY", "ENTRY"), ("struct hsearch_data", "struct hsearch_data"),
 ("struct qelem", "struct qelem"),
 ("struct termios", "struct termios"), ("struct winsize", "struct winsize"),
 ("struct utmp", "struct utmp"), ("struct utmpx", "struct utmpx"), ("struct lastlog", "struct lastlog"),
 ("struct exit_status", "struct exit_status"),
 ("struct passwd", "struct passwd"), ("struct group", "struct group"), ("struct spwd", "struct spwd"),
 ("struct sgrp", "struct sgrp"), ("struct hostent", "struct hostent"), ("struct netent", "struct netent"),
 ("struct servent", "struct servent"), ("struct protoent", "struct protoent"), ("struct rpcent", "struct rpcent"),
 ("struct addrinfo", "struct addrinfo"), ("struct gaicb", "struct gaicb"), ("struct aliasent", "struct aliasent"),
 ("struct __res_state", "struct __res_state"), ("ns_msg", "ns_msg"), ("ns_rr", "ns_rr"), ("HEADER", "HEADER"),
 ("struct in_addr", "struct in_addr"), ("struct in6_addr", "struct in6_addr"),
 ("struct sockaddr", "struct sockaddr"), ("struct sockaddr_storage", "struct sockaddr_storage"),
 ("__SOCKADDR_ARG", "__SOCKADDR_ARG"), ("__CONST_SOCKADDR_ARG", "__CONST_SOCKADDR_ARG"),
 ("struct sockaddr_in", "struct sockaddr_in"), ("struct sockaddr_in6", "struct sockaddr_in6"),
 ("struct sockaddr_un", "struct sockaddr_un"), ("struct ether_addr", "struct ether_addr"),
 ("struct ifaddrs", "struct ifaddrs"), ("struct if_nameindex", "struct if_nameindex"),
 ("struct ifreq", "struct ifreq"), ("struct ip_mreq", "struct ip_mreq"),
 ("struct group_source_req", "struct group_source_req"), ("struct ip6_mtuinfo", "struct ip6_mtuinfo"),
 ("struct msghdr", "struct msghdr"), ("struct cmsghdr", "struct cmsghdr"), ("struct mmsghdr", "struct mmsghdr"),
 ("struct iovec", "struct iovec"), ("struct linger", "struct linger"), ("struct ucred", "struct ucred"),
 ("struct pollfd", "struct pollfd"), ("struct epoll_event", "struct epoll_event"), ("fd_set", "fd_set"),
 ("struct aiocb", "struct aiocb"), ("struct aioinit", "struct aioinit"),
 ("struct mq_attr", "struct mq_attr"), ("struct ipc_perm", "struct ipc_perm"),
 ("struct msqid_ds", "struct msqid_ds"), ("struct semid_ds", "struct semid_ds"), ("struct sembuf", "struct sembuf"),
 ("struct shmid_ds", "struct shmid_ds"),
 ("posix_spawnattr_t", "posix_spawnattr_t"), ("posix_spawn_file_actions_t", "posix_spawn_file_actions_t"),
 ("Dl_info", "Dl_info"), ("struct dl_phdr_info", "struct dl_phdr_info"), ("struct link_map", "struct link_map"),
 ("struct r_debug", "struct r_debug"), ("struct dl_find_object", "struct dl_find_object"),
 ("Dl_serinfo", "Dl_serinfo"), ("Dl_serpath", "Dl_serpath"), ("struct cpuid_feature", "struct cpuid_feature"), ("struct prof", "struct prof"), ("La_x86_64_regs", "La_x86_64_regs"),
 ("La_x86_64_retval", "La_x86_64_retval"), ("La_x86_64_xmm", "La_x86_64_xmm"), ("La_x86_64_ymm", "La_x86_64_ymm"),
 ("La_x86_64_zmm", "La_x86_64_zmm"), ("La_x86_64_vector", "La_x86_64_vector"),
 ("struct mntent", "struct mntent"), ("struct fstab", "struct fstab"), ("struct ttyent", "struct ttyent"),
 ("struct option", "struct option"), ("struct argp", "struct argp"), ("struct argp_option", "struct argp_option"),
 ("struct argp_state", "struct argp_state"), ("struct argp_child", "struct argp_child"),
 ("struct obstack", "struct obstack"), ("struct _obstack_chunk", "struct _obstack_chunk"), ("struct printf_info", "struct printf_info"),
 ("struct mallinfo", "struct mallinfo"), ("struct mallinfo2", "struct mallinfo2"),
 ("fenv_t", "fenv_t"), ("femode_t", "femode_t"), ("fexcept_t", "fexcept_t"),
 ("struct inotify_event", "struct inotify_event"), ("struct signalfd_siginfo", "struct signalfd_siginfo"),
 ("struct rseq", "struct rseq"), ("struct user_regs_struct", "struct user_regs_struct"),
 ("struct user_fpregs_struct", "struct user_fpregs_struct"), ("struct ustat", "struct ustat"),
 ("struct vtimes", "struct vtimes"), ("struct vlimit", "struct vlimit"), ("struct dqblk", "struct dqblk"),
 ("struct cpu_features", "struct cpu_features"),
 # plan/headers.md section 2.1 (slice 3): large-file, protocol, ELF and debug types.
 ("struct flock", "struct flock"), ("struct flock64", "struct flock64"), ("struct stat64", "struct stat64"),
 ("struct statfs64", "struct statfs64"), ("struct statvfs64", "struct statvfs64"),
 ("struct rlimit64", "struct rlimit64"), ("struct f_owner_ex", "struct f_owner_ex"),
 ("struct file_handle", "struct file_handle"),
 ("struct ip", "struct ip"), ("struct iphdr", "struct iphdr"), ("struct ip6_hdr", "struct ip6_hdr"),
 ("struct tcphdr", "struct tcphdr"), ("struct tcp_info", "struct tcp_info"), ("struct udphdr", "struct udphdr"),
 ("struct icmphdr", "struct icmphdr"), ("struct icmp", "struct icmp"), ("struct icmp6_hdr", "struct icmp6_hdr"),
 ("struct res_sym", "struct res_sym"), ("struct ether_header", "struct ether_header"), ("struct arphdr", "struct arphdr"),
 ("struct sockaddr_ll", "struct sockaddr_ll"), ("struct ip_mreqn", "struct ip_mreqn"),
 ("struct ipv6_mreq", "struct ipv6_mreq"), ("struct in_pktinfo", "struct in_pktinfo"),
 ("struct in6_pktinfo", "struct in6_pktinfo"),
 # HEADERS-TABLES-NET: the structures of slice 13's network headers (header_probe.py net_names() names the sources).
 *[(f"struct {t}", f"struct {t}") for t in (
     "ip_timestamp", "rtentry", "in6_rtmsg", "arpreq", "arpreq_old", "arpd_request", "ether_arp", "icmp_ra_addr",
     "icmp6_filter", "nd_router_solicit", "nd_router_advert", "nd_neighbor_solicit", "nd_neighbor_advert",
     "nd_redirect", "nd_opt_hdr", "nd_opt_prefix_info", "nd_opt_rd_hdr", "nd_opt_mtu", "nd_opt_adv_interval",
     "nd_opt_home_agent_info", "nd_opt_route_info", "nd_opt_rdnss", "nd_opt_dnssl", "mld_hdr", "icmp6_router_renum",
     "rr_pco_match", "rr_pco_use", "rr_result", "tftphdr")],
 # HEADERS-BUSYBOX-GAPS: struct ifconf (netdevice(7)), the packet socket and tape structures (header_probe.py
 # busybox_names() names the sources); a tag the reference does not define is dropped by available_types().
 *[(f"struct {t}", f"struct {t}") for t in (
     "ifconf", "ifaddr", "packet_mreq", "sockaddr_pkt", "tpacket_req", "tpacket_stats", "tpacket_auxdata", "tpacket_hdr",
     "mtop", "mtget", "mtpos", "mtconfiginfo", "mt_tape_info")],
 # HEADERS-STAGE2-GAPS-C: the archive member header of ar(5) (elfutils) and XSH <utime.h>'s struct utimbuf (bzip2, cmake).
 ("struct ar_hdr", "struct ar_hdr"), ("struct utimbuf", "struct utimbuf"),
 ("Elf32_Ehdr", "Elf32_Ehdr"), ("Elf64_Ehdr", "Elf64_Ehdr"), ("Elf32_Phdr", "Elf32_Phdr"), ("Elf64_Phdr", "Elf64_Phdr"),
 ("Elf32_Shdr", "Elf32_Shdr"), ("Elf64_Shdr", "Elf64_Shdr"), ("Elf32_Sym", "Elf32_Sym"), ("Elf64_Sym", "Elf64_Sym"),
 ("Elf32_Dyn", "Elf32_Dyn"), ("Elf64_Dyn", "Elf64_Dyn"), ("Elf32_Rel", "Elf32_Rel"), ("Elf64_Rel", "Elf64_Rel"),
 ("Elf32_Rela", "Elf32_Rela"), ("Elf64_Rela", "Elf64_Rela"), ("Elf32_Nhdr", "Elf32_Nhdr"), ("Elf64_Nhdr", "Elf64_Nhdr"),
 ("Elf64_Chdr", "Elf64_Chdr"), ("Elf64_auxv_t", "Elf64_auxv_t"), ("Elf64_Verdef", "Elf64_Verdef"),
 ("Elf64_Verdaux", "Elf64_Verdaux"), ("Elf64_Verneed", "Elf64_Verneed"), ("Elf64_Vernaux", "Elf64_Vernaux"),
 ("struct user", "struct user"), ("struct elf_prstatus", "struct elf_prstatus"),
 ("struct elf_prpsinfo", "struct elf_prpsinfo"), ("struct elf_siginfo", "struct elf_siginfo"),
 # thread_db.h and proc_service.h (SL-HEADERS-UNSHIPPED-REST): the tags are the C++ names of typedefs.tsv's rows.
 ("struct td_thrhandle", "struct td_thrhandle"), ("struct td_thr_events", "struct td_thr_events"),
 ("struct td_notify", "struct td_notify"), ("struct td_event_msg", "struct td_event_msg"),
 ("struct td_ta_stats", "struct td_ta_stats"), ("struct td_thrinfo", "struct td_thrinfo"),
 ("struct td_eventbuf", "struct td_eventbuf"), ("td_eventbuf_t", "td_eventbuf_t"),
 ("td_err_e", "td_err_e"), ("td_event_e", "td_event_e"), ("td_notify_e", "td_notify_e"),
 ("td_thr_state_e", "td_thr_state_e"), ("td_thr_type_e", "td_thr_type_e"), ("ps_err_e", "ps_err_e"),
 # The sys/procfs.h typedefs a debugger names (td_thr_getgregs, ps_lgetregs; core files) that no prototype spells.
 ("prgregset_t", "prgregset_t"), ("elf_fpregset_t", "elf_fpregset_t"), ("prstatus_t", "prstatus_t"),
 ("prpsinfo_t", "prpsinfo_t"),
]

# Static initializers whose bit patterns are ABI (programs embed them).
INITIALIZERS = [
 ("pthread_mutex_t", "PTHREAD_MUTEX_INITIALIZER"),
 ("pthread_mutex_t", "PTHREAD_RECURSIVE_MUTEX_INITIALIZER_NP"),
 ("pthread_mutex_t", "PTHREAD_ERRORCHECK_MUTEX_INITIALIZER_NP"),
 ("pthread_mutex_t", "PTHREAD_ADAPTIVE_MUTEX_INITIALIZER_NP"),
 ("pthread_rwlock_t", "PTHREAD_RWLOCK_INITIALIZER"),
 ("pthread_rwlock_t", "PTHREAD_RWLOCK_WRITER_NONRECURSIVE_INITIALIZER_NP"),
 ("pthread_cond_t", "PTHREAD_COND_INITIALIZER"),
 ("pthread_once_t", "PTHREAD_ONCE_INIT"),
 ("once_flag", "ONCE_FLAG_INIT"),
 ("mbstate_t", "{0}"),
]

LIBS = ["libc.so.6", "libm.so.6", "libmvec.so.1", "libpthread.so.0", "libdl.so.2", "librt.so.1", "libutil.so.1",
        "libanl.so.1", "libresolv.so.2", "libnsl.so.1", "libthread_db.so.1", "libBrokenLocale.so.1",
        "libc_malloc_debug.so.0", "ld-linux-x86-64.so.2"]

# Target selection. The default is the native x86_64 reference (programs are built and run).
# `--arch aarch64 --sysroot DIR` probes an unpacked, not installed, arm64 reference (ledger A-5):
# compile-only, because nothing built for aarch64 runs here. Sizes, alignments and initializer
# bytes are object-file constants read back with readelf; members come from DWARF as for x86_64.
ARCH = "x86_64"
CC = ["gcc"]
SYSROOT = ""
TRIPLET = "x86_64-linux-gnu"
SUFFIX = ""

def configure(argv):
    global ARCH, CC, SYSROOT, TRIPLET, SUFFIX, LIBDIR, LIBS
    if "--arch" not in argv:
        return
    ARCH = argv[argv.index("--arch") + 1]
    if ARCH != "aarch64":
        sys.exit(f"--arch {ARCH}: only aarch64 is supported")
    if "--sysroot" not in argv:
        sys.exit("--arch aarch64 needs --sysroot DIR (the unpacked arm64 libc6 + libc6-dev)")
    SYSROOT = os.path.abspath(argv[argv.index("--sysroot") + 1])
    TRIPLET = "aarch64-linux-gnu"
    CC = ["aarch64-linux-gnu-gcc", f"--sysroot={SYSROOT}"]
    SUFFIX = "-aarch64"
    LIBDIR = next((d for d in (f"{SYSROOT}/lib/{TRIPLET}", f"{SYSROOT}/usr/lib/{TRIPLET}")
                   if os.path.exists(os.path.join(d, "libc.so.6"))), None)
    if LIBDIR is None:
        sys.exit(f"{SYSROOT}: no lib/{TRIPLET}/libc.so.6")
    LIBS = [l if not l.startswith("ld-linux") else "ld-linux-aarch64.so.1" for l in LIBS]

def cross():
    return ARCH != "x86_64"

def story(argv):
    """The story id of `--story ID` (capitals, digits, '-'): the name of the provenance fragment a run writes."""
    sid = argv[argv.index("--story") + 1] if "--story" in argv and argv.index("--story") + 1 < len(argv) else ""
    if not re.fullmatch(r"[A-Z0-9][A-Z0-9-]*", sid):
        sys.exit("this run records its provenance: pass --story STORY-ID (capitals, digits and '-')")
    return sid

def write_fragment(sid, text):
    """Write spec/abi/provenance.d/<sid>.txt (D-X.172), ordered after every fragment there; `text` is one
    paragraph. A fragment of the same story is replaced, keeping its place."""
    d = os.path.join(OUT, "provenance.d")
    path = os.path.join(d, f"{sid}.txt")
    orders = {}
    for n in sorted(os.listdir(d)):
        if n.endswith(".txt"):
            m = re.match(r"---\norder: (\d+)\n", open(os.path.join(d, n)).read())
            if m:
                orders[n] = int(m.group(1))
    order = orders.get(f"{sid}.txt", max(orders.values(), default=0) + 10)
    body = "\n".join(l.rstrip() for l in text.strip().splitlines() if l.strip())
    with open(path, "w", newline="\n") as f:
        f.write(f"---\norder: {order}\n---\n{body}\n")
    print(f"{path}:\n{body}")

def section_bytes(obj, section):
    """The contents of one section of an object file, from `readelf -x` (hex dump)."""
    text = sh(["readelf", "-x", section, "-W", obj])
    out = bytearray()
    for line in text.splitlines():
        m = re.match(r"\s+0x[0-9a-f]+ ", line)
        if m:
            out += bytes.fromhex(line[m.end():m.end() + 35].replace(" ", ""))
    if not out:
        raise SystemExit(f"{obj}: section {section} has no data:\n{text}")
    return bytes(out)

def sh(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, check=True, **kw).stdout

def prelude():
    return "#define _GNU_SOURCE 1\n#define _DYNAMIC_STACK_SIZE_SOURCE 1\n" + \
           "".join(f"#include <{h}>\n" for h in HEADERS if os.path.exists(SYSROOT + "/usr/include/" + h) or
                   os.path.exists(f"{SYSROOT}/usr/include/{TRIPLET}/" + h))

def available_types(tmp):
    """Keep only types the headers actually declare (probe each one)."""
    ok = []
    for label, expr in TYPES:
        src = prelude() + f"extern char probe[sizeof({expr})];\n"
        p = os.path.join(tmp, "t.c")
        open(p, "w").write(src)
        r = subprocess.run(CC + ["-std=gnu17", "-fsyntax-only", p], capture_output=True, text=True)
        if r.returncode == 0:
            ok.append((label, expr))
        else:
            print(f"  skip: {label} (not declared by the installed headers)", file=sys.stderr)
    return ok

def sizes(tmp, types):
    if cross():
        # One 8-byte little-endian pair (sizeof, _Alignof) per type, in a section of its own.
        src = prelude() + '__attribute__((used, section(".probe_sz"))) const unsigned long long probe_sz[] = {\n'
        for label, expr in types:
            src += f"  sizeof({expr}), _Alignof({expr}),\n"
        src += "};\n"
        p = os.path.join(tmp, "sz.c"); open(p, "w").write(src)
        o = os.path.join(tmp, "sz.o")
        sh(CC + ["-std=gnu17", "-O0", "-c", p, "-o", o])
        raw = section_bytes(o, ".probe_sz")
        vals = [int.from_bytes(raw[i:i + 8], "little") for i in range(0, len(raw), 8)]
        if len(vals) != 2 * len(types):
            raise SystemExit(f"sz.o: {len(vals)} values for {len(types)} types")
        return {label: (vals[2 * i], vals[2 * i + 1]) for i, (label, _) in enumerate(types)}
    src = prelude() + "#include <stdio.h>\nint main(void){\n"
    for label, expr in types:
        src += f'  printf("%s\\t%zu\\t%zu\\n", "{label}", sizeof({expr}), _Alignof({expr}));\n'
    src += "  return 0;}\n"
    p = os.path.join(tmp, "sz.c"); open(p, "w").write(src)
    sh(["gcc", "-std=gnu17", "-O0", p, "-o", os.path.join(tmp, "sz")])
    rows = [l.split("\t") for l in sh([os.path.join(tmp, "sz")]).splitlines()]
    return {r[0]: (int(r[1]), int(r[2])) for r in rows}

# --- DWARF member extraction -------------------------------------------------
def dwarf(tmp, types):
    src = prelude()
    for i, (label, expr) in enumerate(types):
        src += f"{expr} probe_{i};\n"
    p = os.path.join(tmp, "dw.c"); open(p, "w").write(src)
    o = os.path.join(tmp, "dw.o")
    sh(CC + ["-std=gnu17", "-g", "-gdwarf-4", "-O0", "-fno-eliminate-unused-debug-types", "-c", p, "-o", o])
    text = sh(["readelf", "--debug-dump=info", "-W", o])
    dies = {}      # offset -> dict
    stack = []     # (depth, offset)
    cur = None
    for line in text.splitlines():
        m = re.match(r"\s*<(\d+)><([0-9a-f]+)>: Abbrev Number: (\d+)(?: \((\w+)\))?", line)
        if m:
            depth, off, tag = int(m.group(1)), int(m.group(2), 16), m.group(4)
            if tag is None:
                stack = [s for s in stack if s[0] < depth]
                cur = None
                continue
            cur = {"tag": tag, "children": [], "attrs": {}}
            dies[off] = cur
            stack = [s for s in stack if s[0] < depth]
            if stack:
                dies[stack[-1][1]]["children"].append(off)
            stack.append((depth, off))
            continue
        a = re.match(r"\s*<[0-9a-f]+>\s+(DW_AT_\w+)\s*:\s*(.*)$", line)
        if a and cur is not None:
            name, val = a.group(1), a.group(2).strip()
            # Strip DWARF form annotations: "(strp) (offset: 0x1a): _flags", "(string) int".
            if "(offset:" in val and "):" in val:
                val = val.split("):", 1)[1].strip()
            val = re.sub(r"^\((?:string|strp|line_strp|data\d|sdata|udata|ref\d|ref_udata|flag_present|exprloc)\)\s*", "", val)
            cur["attrs"][name] = val
    return dies

def num(v, default=0):
    for tok in reversed((v or "").split()):
        try:
            return int(tok, 0)
        except ValueError:
            continue
    return default

def ref(v):
    m = re.search(r"<0x([0-9a-f]+)>", v or "")
    return int(m.group(1), 16) if m else None

def tname(dies, off, depth=0):
    if off is None:
        return "void"
    d = dies.get(off)
    if d is None or depth > 12:
        return "?"
    tag, at = d["tag"], d["attrs"]
    nm = at.get("DW_AT_name")
    if tag in ("DW_TAG_base_type", "DW_TAG_typedef"):
        return nm
    if tag in ("DW_TAG_structure_type", "DW_TAG_union_type", "DW_TAG_enumeration_type"):
        kw = {"DW_TAG_structure_type": "struct", "DW_TAG_union_type": "union", "DW_TAG_enumeration_type": "enum"}[tag]
        return f"{kw} {nm}" if nm else f"{kw} <anon>"
    if tag == "DW_TAG_pointer_type":
        return tname(dies, ref(at.get("DW_AT_type")), depth + 1) + " *"
    if tag in ("DW_TAG_const_type", "DW_TAG_volatile_type", "DW_TAG_restrict_type", "DW_TAG_atomic_type"):
        q = tag[7:-5]
        return f"{q} " + tname(dies, ref(at.get("DW_AT_type")), depth + 1)
    if tag == "DW_TAG_array_type":
        dims = ""
        for c in d["children"]:
            ca = dies[c]["attrs"]
            if "DW_AT_upper_bound" in ca:
                dims += f"[{num(ca['DW_AT_upper_bound']) + 1}]"
            elif "DW_AT_count" in ca:
                dims += f"[{num(ca['DW_AT_count'])}]"
            else:
                dims += "[]"
        return tname(dies, ref(at.get("DW_AT_type")), depth + 1) + dims
    if tag == "DW_TAG_subroutine_type":
        return "fn(...)"
    return tag

def resolve(dies, off, depth=0):
    """Follow typedef/cv chains to the underlying struct or union DIE."""
    while off is not None and depth < 20:
        d = dies[off]
        if d["tag"] in ("DW_TAG_structure_type", "DW_TAG_union_type"):
            return off
        if d["tag"] in ("DW_TAG_typedef", "DW_TAG_const_type", "DW_TAG_volatile_type"):
            off = ref(d["attrs"].get("DW_AT_type")); depth += 1; continue
        if d["tag"] == "DW_TAG_array_type":   # jmp_buf, sigjmp_buf are arrays of a struct
            off = ref(d["attrs"].get("DW_AT_type")); depth += 1; continue
        return None
    return None

def resolve_base(dies, off, depth=0):
    """Follow typedef/cv chains to the underlying base or enumeration type DIE (a bit-field's storage unit type)."""
    while off is not None and depth < 20 and dies[off]["tag"] in ("DW_TAG_typedef", "DW_TAG_const_type", "DW_TAG_volatile_type"):
        off = ref(dies[off]["attrs"].get("DW_AT_type")); depth += 1
    return off

def enumerators(dies):
    """(tag, enumerator, value) of every named enumeration the headers define, enumerators in declaration order.

    An enumeration without a tag that a typedef names directly (`typedef enum { ... } NAME;`: the DWARF typedef's
    type is the unnamed enumeration) gets the tag `typedef NAME` (HEADERS-GEN-SHARED, ruling #7643): no other row
    could name it, and the generator emits it as that typedef of an unnamed enumeration.
    """
    named_by = {}
    for d in dies.values():
        if d["tag"] == "DW_TAG_typedef" and d["attrs"].get("DW_AT_name"):
            t = ref(d["attrs"].get("DW_AT_type"))
            e = dies.get(t) if t is not None else None
            if e is not None and e["tag"] == "DW_TAG_enumeration_type" and not e["attrs"].get("DW_AT_name"):
                named_by.setdefault(t, set()).add(d["attrs"]["DW_AT_name"])
    rows, seen = [], set()
    for off, d in dies.items():
        nm = d["attrs"].get("DW_AT_name")
        if d["tag"] != "DW_TAG_enumeration_type" or "DW_AT_declaration" in d["attrs"]:
            continue
        if not nm:
            names = named_by.get(off, set())
            if len(names) != 1:
                # An enumeration no typedef names (its enumerators are plain constants), or one two typedefs name
                # (no single tag): left to visibility.tsv and macros.tsv.
                continue
            nm = f"typedef {names.pop()}"
        if nm in seen:
            raise SystemExit(f"enumerators: enum {nm} is defined twice")
        seen.add(nm)
        tag = nm if nm.startswith("typedef ") else f"enum {nm}"
        for c in d["children"]:
            e = dies[c]
            if e["tag"] == "DW_TAG_enumerator":
                rows.append((tag, e["attrs"]["DW_AT_name"], num(e["attrs"].get("DW_AT_const_value"))))
    return sorted(rows, key=lambda r: r[0])

def members(dies, off, prefix="", base=0, out=None, depth=0):
    out = [] if out is None else out
    d = dies[off]
    for c in d["children"]:
        m = dies[c]
        if m["tag"] != "DW_TAG_member":
            continue
        at = m["attrs"]
        name = at.get("DW_AT_name", "")
        o = num(at.get("DW_AT_data_member_location", "0"))
        t = ref(at.get("DW_AT_type"))
        full = f"{prefix}{name}" if name else prefix.rstrip(".") or "<anon>"
        bits = ""
        if "DW_AT_bit_size" in at:
            # `bits W@P`: width W, first bit P counted from the low bit of the storage unit at the row's byte offset
            # (psABI little-endian). DWARF 4 gives the unit (data_member_location, byte_size) and the bit_offset of
            # the field's high bit from the unit's high bit; DWARF 5 gives data_bit_offset from the aggregate start.
            w = num(at["DW_AT_bit_size"])
            if "DW_AT_data_bit_offset" in at:
                dbo = num(at["DW_AT_data_bit_offset"])
                unit = num(at.get("DW_AT_byte_size")) or num(dies[resolve_base(dies, t)]["attrs"].get("DW_AT_byte_size"))
                o = dbo // (8 * unit) * unit
                pos = dbo - 8 * o
            else:
                pos = 8 * num(at.get("DW_AT_byte_size")) - num(at.get("DW_AT_bit_offset")) - w
            bits = f"bits {w}@{pos}"
        inner = resolve(dies, t)
        out.append((base + o, full, tname(dies, t), bits))
        at_row = len(out) - 1
        tag = dies[t]["tag"] if t in dies else ""
        # Descend into every aggregate member at every depth (anonymous, named, and the first element of an array of
        # aggregates: `sort_list.addr` is the member of `sort_list[0]`); never through a pointer.
        if inner is not None and tag != "DW_TAG_pointer_type" and depth < 16:
            members(dies, inner, f"{full}." if name else prefix, base + o, out, depth + 1)
        if not name and inner is not None:
            # An anonymous member's own members follow it at this level (flattened, as C names them):
            # the last column gives its byte size and how many of the following rows at this level are
            # its members (`size N rows M`), so a reader can rebuild the nesting.
            level = prefix.count(".") + 1
            n = sum(1 for r in out[at_row + 1:] if r[1].count(".") + 1 == level)
            out[at_row] = out[at_row][:3] + (f"size {num(dies[inner]['attrs'].get('DW_AT_byte_size'))} rows {n}",)
    return out

def accepts(tmp, expr, arg, decl="", std="gnu17", pre=None):
    """Whether a call passes an argument of type `arg` to a parameter of type `expr` (return code only)."""
    src = (prelude() if pre is None else pre) + decl + f"void probe_tu ({expr});\nvoid probe_tu_call ({arg} p) {{ probe_tu (p); }}\n"
    p = os.path.join(tmp, "tu.c")
    open(p, "w").write(src)
    return subprocess.run(CC + [f"-std={std}", "-Werror", "-fsyntax-only", p], capture_output=True, text=True).returncode == 0

def transparent(tmp, expr):
    """Whether the union type `expr` is a transparent union. DWARF does not record the attribute; the GCC manual
    (Common Type Attributes, transparent_union) does: a parameter of such a union with a pointer member accepts a
    `void *` argument, which no ordinary union parameter accepts. The head row's last column then says
    `transparent_union` (abi-gen reads no head-row column past the alignment)."""
    return accepts(tmp, expr, "void *")

# The address structures a transparent union parameter may take: the tags the tables record (typedefs.tsv) and
# the ones the Linux man pages document (ddp(7) sockaddr_atalk, netlink(7) sockaddr_nl, packet(7) sockaddr_pkt,
# vsock(7) sockaddr_vm, x25(7) sockaddr_x25). The generic `struct sockaddr` (POSIX <sys/socket.h>) comes first:
# the first member is the one a transparent union argument is passed as (GCC manual, transparent_union).
TU_CANDIDATES = ["sockaddr", "sockaddr_atalk", "sockaddr_in", "sockaddr_in6", "sockaddr_ll", "sockaddr_nl",
                 "sockaddr_pkt", "sockaddr_storage", "sockaddr_un", "sockaddr_vm", "sockaddr_x25"]

def tu_members(tmp, expr):
    """The members of the transparent union `expr`. GCC's DWARF gives the union no member DIEs; the GCC manual
    says such a parameter accepts an argument of any member's type, so each candidate pointer that a call passes
    under -Werror is a member (`const struct T *` where a pointer to const is accepted). The members' names are
    the implementation's own (reserved, never observable): the rows name each `__tu_<tag>`. With the head row
    mark: ` restrict` when the name, where it is a macro (strict C), is a restrict pointer to the first
    member's type (`_Generic` on a pointer to an object of it)."""
    rows = []
    for t in TU_CANDIDATES:
        decl = f"struct {t};\n"
        ty = next((q + f"struct {t} *" for q in ("const ", "") if accepts(tmp, expr, f"{q}struct {t} *", decl)), None)
        if ty is not None:
            rows.append((f"__tu_{t}", ty))
    restrict = ""
    if rows:
        strict = "".join(l + "\n" for l in prelude().splitlines() if not l.startswith("#define"))
        src = strict + f"{expr} probe_x;\nint probe_y = _Generic (&probe_x, {rows[0][1]}restrict *: 1);\n"
        p = os.path.join(tmp, "tr.c")
        open(p, "w").write(src)
        if subprocess.run(CC + ["-std=c17", "-fsyntax-only", p], capture_output=True, text=True).returncode == 0:
            restrict = " restrict"
    return rows, restrict

def layouts(tmp, types, szmap):
    dies = dwarf(tmp, types)
    varmap = {}
    for off, d in dies.items():
        if d["tag"] == "DW_TAG_variable" and d["attrs"].get("DW_AT_name", "").startswith("probe_"):
            varmap[int(d["attrs"]["DW_AT_name"][6:])] = ref(d["attrs"].get("DW_AT_type"))
    rows = []
    for i, (label, expr) in enumerate(types):
        size, align = szmap[label]
        s = resolve(dies, varmap.get(i))
        if s is not None and dies[s]["tag"] == "DW_TAG_union_type" and transparent(tmp, expr):
            tu, restrict = tu_members(tmp, expr)
            rows.append((label, "", size, align, "", "transparent_union" + restrict))
            rows.extend((label, n, 0, "", ty, "") for n, ty in tu)
            continue
        rows.append((label, "", size, align, "", ""))
        if s is None:
            continue
        for o, name, ty, bits in members(dies, s):
            rows.append((label, name, o, "", ty, bits))
    return rows, enumerators(dies)

def initializers(tmp):
    if cross():
        # Each initialised object in a section of its own; the section's bytes are the object's.
        src = prelude()
        for i, (ty, init) in enumerate(INITIALIZERS):
            src += f'__attribute__((used, section(".probe_init{i}"))) const {ty} probe_init{i} = {init};\n'
        p = os.path.join(tmp, "init.c"); open(p, "w").write(src)
        o = os.path.join(tmp, "init.o")
        sh(CC + ["-std=gnu17", "-O0", "-c", p, "-o", o])
        rows = []
        for i, (ty, init) in enumerate(INITIALIZERS):
            b = section_bytes(o, f".probe_init{i}")
            rows.append([ty, init, str(len(b)), "all-zero" if not any(b) else "nonzero", b.hex()])
        return rows
    src = prelude() + "#include <stdio.h>\n#include <string.h>\nint main(void){\n"
    for ty, init in INITIALIZERS:
        var = re.sub(r"\W", "_", init)
        rhs = init if init.startswith("{") or init.startswith("*") else init
        src += f"  {{ {ty} v = {rhs}; const unsigned char *p = (const unsigned char *)&v; size_t n = sizeof v, z = 1;\n"
        src += f'    for (size_t i = 0; i < n; i++) z &= p[i] == 0;\n'
        src += f'    printf("%s\\t%s\\t%zu\\t%s\\t", "{ty}", "{init}", n, z ? "all-zero" : "nonzero");\n'
        src += '    for (size_t i = 0; i < n; i++) printf("%02x", p[i]); printf("\\n"); }\n'
    src += "  return 0;}\n"
    p = os.path.join(tmp, "init.c"); open(p, "w").write(src)
    sh(["gcc", "-std=gnu17", p, "-o", os.path.join(tmp, "init")])
    return [l.split("\t") for l in sh([os.path.join(tmp, "init")]).splitlines()]

def prototypes(tmp, src=None, flags=("-std=gnu17",)):
    p = os.path.join(tmp, "proto.c"); open(p, "w").write(prelude() if src is None else src)
    aux = os.path.join(tmp, "aux.txt")
    subprocess.run(["gcc", *flags, "-c", p, "-o", os.path.join(tmp, "proto.o"), "-aux-info", aux], check=True)
    rows = {}
    for line in open(aux, errors="replace"):
        m = re.match(r"/\* (\S+):(\d+):\S+ \*/ (.*);$", line.strip())
        if not m:
            continue
        decl = m.group(3)
        n = re.search(r"(\w+)\s*\((?!\*)", decl)
        if not n:
            continue
        name = n.group(1)
        hdr = m.group(1).replace("/usr/include/x86_64-linux-gnu/", "").replace("/usr/include/", "")
        rows.setdefault(name, (hdr, re.sub(r"\s+", " ", decl)))
    return rows

def tsv_rows(name):
    with open(os.path.join(OUT, name), newline="") as f:
        return list(csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE))

def prototypes_missing(tmp, seen):
    """Rows for the functions visibility.tsv declares in some C configuration that the `_GNU_SOURCE` pass of
    prototypes() (the names `seen`) does not see (`gets`: C89/C99 only; `bsd_signal`: X/Open 500/600 only). Each
    such name is compiled in the first C configuration of ftm-configs.tsv that declares it, with that configuration's
    flags and the headers that show it there, and its `-aux-info` line is taken. Names declared in no C configuration
    (the C++ function forms of the type-generic macros) or only under _FORTIFY_SOURCE get no row here."""
    have = set(seen)
    have |= {r["name"] for r in tsv_rows("typedefs.tsv") if r["kind"] == "function"}
    # Not the fortify configurations: their `__*_chk` functions are the fortify layer's, not ordinary prototypes.
    cfgs = [r for r in tsv_rows("ftm-configs.tsv") if r["language"] == "c" and "_FORTIFY_SOURCE" not in r["flags"]]
    ids = [r["id"] for r in tsv_rows("ftm-configs.tsv")]
    def shown(configs, cid):
        if configs == "*":
            return True
        if configs.startswith("!"):
            return cid not in configs[1:].split(",") and cid in ids
        return cid in configs.split(",")
    vis = [r for r in tsv_rows("visibility.tsv") if r["kind"] == "function" and r["header"] != "-" and r["name"] not in have]
    plan = {}
    for n in sorted({r["name"] for r in vis}):
        for c in cfgs:
            hs = sorted(r["header"] for r in vis if r["name"] == n and shown(r["configs"], c["id"]))
            if hs:
                plan.setdefault(c["id"], (c["flags"].split(), []))[1].append((n, hs[0]))
                break
    rows = {}
    for cid, (flags, names) in sorted(plan.items()):
        src = "".join(f"#include <{h}>\n" for h in sorted({h for _, h in names}))
        got = prototypes(tmp, src, flags)
        for n, _ in names:
            if n in got:
                rows[n] = got[n]
            else:
                print(f"prototypes-missing: {n}: no declaration in configuration {cid}", file=sys.stderr)
    return rows

def symbols():
    out = []
    for lib in LIBS:
        path = os.path.join(LIBDIR, lib)
        if not os.path.exists(path):
            continue
        text = sh(["readelf", "--dyn-syms", "-W", path])
        for line in text.splitlines():
            f = line.split()
            if len(f) < 8 or not f[0].rstrip(":").isdigit():
                continue
            num, value, size, typ, bind, vis, ndx, name = f[:8]
            if ndx == "UND" or typ in ("SECTION", "FILE"):
                continue
            if "@" not in name:
                continue
            base, _, ver = name.partition("@")
            default = ver.startswith("@")
            ver = ver.lstrip("@")
            if base == ver:        # version-definition symbols (GLIBC_2.2.5 ABS)
                continue
            # Interface metadata only: a function's code size and whether it is an IFUNC are
            # implementation details, so functions are reported as FUNC with no size. Object
            # sizes are ABI (old binaries have them baked in via copy relocations).
            is_obj = typ in ("OBJECT", "TLS")
            nbytes = (int(size, 0) if size.startswith("0x") else int(size)) if is_obj else ""
            out.append((lib, base, ver, "default" if default else "compat", typ if is_obj else "FUNC", bind, nbytes))
    return out

def empty_versions():
    """The version definitions of each library that no exported symbol carries: (library, version,
    parent). versions.tsv has rows for symbols only, so it cannot show these; a program can still
    require one (the link editor records GLIBC_ABI_DT_RELR for packed relative relocations)."""
    out = []
    for lib in LIBS:
        path = os.path.join(LIBDIR, lib)
        if not os.path.exists(path):
            continue
        defs, parent, cur = [], {}, None
        for line in sh(["readelf", "-V", "-W", path]).splitlines():
            m = re.search(r"Flags: (\S+)\s+Index: \d+\s+Cnt: \d+\s+Name: (\S+)", line)
            if m:
                cur = m.group(2) if m.group(1) != "BASE" else None
                if cur:
                    defs.append(cur)
                continue
            m = re.search(r"Parent 1: (\S+)", line)
            if m and cur:
                parent[cur] = m.group(1)
        used = set()
        for line in sh(["readelf", "--dyn-syms", "-W", path]).splitlines():
            f = line.split()
            if len(f) < 8 or not f[0].rstrip(":").isdigit() or f[6] == "UND" or "@" not in f[7]:
                continue
            base, _, ver = f[7].partition("@")
            ver = ver.lstrip("@")
            if base != ver:        # not the node's own ABS symbol
                used.add(ver)
        out += [(lib, d, parent.get(d, "")) for d in defs if d not in used]
    return out

def macros(tmp):
    p = os.path.join(tmp, "m.c"); open(p, "w").write(prelude())
    defs = sh(["gcc", "-std=gnu17", "-dM", "-E", p])
    names = sorted({m.group(1) for m in re.finditer(r"^#define (\w+) ", defs, re.M) if not m.group(1).startswith("__")})
    # Evaluate each name that expands to an integer constant expression, or to a pointer with a
    # fixed integer value (SIG_ERR, MAP_FAILED, RTLD_NEXT). The type check rejects floating
    # macros, which would record a truncated value; running each probe twice (below) rejects
    # string and function-name macros, which would record an address. A chunk that does not compile is bisected until
    # the offending names are isolated, so a bad name never drops the others (an earlier
    # version skipped such a chunk whole, losing e.g. the _SC_* enum names; ERRATA E10).
    def evaluate(part, tag):
        src = prelude() + "#include <stdio.h>\n"
        for n in part:
            src += (f"#ifdef {n}\n_Static_assert(__builtin_classify_type(({n}) + 0) == 1 || __builtin_classify_type({n}) == 5, \"integer\");\n"
                    f"static const long long v_{n} = (long long)({n});\n#endif\n")
        src += "int main(void){\n"
        for n in part:
            src += f'#ifdef {n}\n printf("%s\\t%lld\\n", "{n}", v_{n});\n#endif\n'
        src += "return 0;}\n"
        q = os.path.join(tmp, f"mv{tag}.c"); open(q, "w").write(src)
        r = subprocess.run(["gcc", "-std=gnu17", "-w", q, "-o", q[:-2]], capture_output=True, text=True)
        if r.returncode == 0:
            # Run twice: under ASLR an address (a function name such as `basename`) differs
            # between runs, a constant does not.
            first = [l.split("\t") for l in sh([q[:-2]]).splitlines()]
            again = dict(l.split("\t") for l in sh([q[:-2]]).splitlines())
            return [row for row in first if again.get(row[0]) == row[1]]
        if len(part) == 1:
            return []
        half = len(part) // 2
        return evaluate(part[:half], tag + "a") + evaluate(part[half:], tag + "b")
    rows = []
    chunk = 400
    for i in range(0, len(names), chunk):
        rows += evaluate(names[i:i + chunk], str(i))
    return rows

def write(name, header, rows):
    with open(os.path.join(OUT, name), "w", newline="\n") as f:
        f.write("\t".join(header) + "\n")
        for r in rows:
            f.write("\t".join(str(x) for x in r) + "\n")

VERSIONS_HEAD = ["library", "symbol", "version", "binding", "type", "scope", "size"]
ADDITIONS = "versions-additions.tsv"
ADDITIONS_HEAD = ["arch"] + VERSIONS_HEAD + ["decision"]

def additions():
    """The exports libniji adds beyond the reference (spec/abi/versions-additions.tsv), for this ARCH.

    The reference's symbol tables cannot give them, so a regeneration takes them from that file: one row per
    export, `all` or one architecture, and the decision record that adds it (for example GLIBC_2.41's
    sched_getattr/sched_setattr, owner ruling R3, D-24.12). `cargo xtask static` (abi-versions) checks the file
    and that the committed versions tables are what this script writes."""
    path = os.path.join(OUT, ADDITIONS)
    if not os.path.exists(path):
        sys.exit(f"{path}: missing; it lists the exports libniji adds beyond the reference, which a regeneration "
                 f"would otherwise drop")
    lines = open(path).read().splitlines()
    if not lines or lines[0].split("\t") != ADDITIONS_HEAD:
        sys.exit(f"{path}: the header must be {' '.join(ADDITIONS_HEAD)}")
    rows = []
    for n, line in enumerate(lines[1:], 2):
        f = line.split("\t")
        if len(f) != len(ADDITIONS_HEAD):
            sys.exit(f"{path}:{n}: {len(f)} columns, want {len(ADDITIONS_HEAD)}")
        if f[0] not in ("all", "x86_64", "aarch64"):
            sys.exit(f"{path}:{n}: arch `{f[0]}` is none of all, x86_64, aarch64")
        if not re.fullmatch(r"D-[0-9A-Z]+\.[0-9]+", f[-1]):
            sys.exit(f"{path}:{n}: {f[2]}: the decision column names no decision record (D-<section>.<n>)")
        if f[0] in ("all", ARCH):
            rows.append(tuple(f[1:-1]))
    return rows

def merged(ref, adds):
    """The reference rows with each addition after the last row of its library (at the end for a library the
    reference lacks), in the additions file's order. An addition must not repeat a reference row's
    library, symbol and version."""
    keys = {tuple(str(x) for x in r[:3]) for r in ref}
    out = list(ref)
    for a in adds:
        if a[:3] in keys:
            sys.exit(f"{ADDITIONS}: {a[0]} {a[1]}@{a[2]} is already a row of the reference (or repeated)")
        keys.add(a[:3])
        last = max((i for i, r in enumerate(out) if r[0] == a[0]), default=len(out) - 1)
        out.insert(last + 1, a)
    return out

def write_versions(syms):
    """versions{SUFFIX}.tsv: the reference rows merged with the additions. versions{SUFFIX}.sha256: the sha256 of
    the reference rows alone, rendered as write() renders them (header included), and their count, so that the
    static check can tell a regenerated row from a hand-written one without the reference system."""
    ref_text = "\t".join(VERSIONS_HEAD) + "\n" + "".join("\t".join(str(x) for x in r) + "\n" for r in syms)
    rows = merged(syms, additions())
    write(f"versions{SUFFIX}.tsv", VERSIONS_HEAD, rows)
    with open(os.path.join(OUT, f"versions{SUFFIX}.sha256"), "w", newline="\n") as f:
        f.write(f"{hashlib.sha256(ref_text.encode()).hexdigest()}\t{len(syms)}\n")
    return rows

PROTO_HEAD = ["name", "header", "declaration"]
PROTO_ADDITIONS = "prototypes-additions.tsv"
PROTO_ADDITIONS_HEAD = PROTO_HEAD + ["decision"]

def proto_additions():
    """The prototypes of the functions libniji adds beyond the reference (spec/abi/prototypes-additions.tsv): one row
    per function, a prototypes.tsv row plus the decision record that adds it (for example GLIBC_2.41's
    sched_getattr/sched_setattr, owner ruling R3, D-24.12). The reference's headers cannot declare them, so a
    regeneration takes them from that file. `cargo xtask static` (abi-prototypes) checks the file and that the
    committed prototypes.tsv is what this script writes."""
    path = os.path.join(OUT, PROTO_ADDITIONS)
    if not os.path.exists(path):
        sys.exit(f"{path}: missing; it lists the prototypes libniji adds beyond the reference, which a regeneration "
                 f"would otherwise drop")
    lines = open(path).read().splitlines()
    if not lines or lines[0].split("\t") != PROTO_ADDITIONS_HEAD:
        sys.exit(f"{path}: the header must be {' '.join(PROTO_ADDITIONS_HEAD)}")
    rows = []
    for n, line in enumerate(lines[1:], 2):
        f = line.split("\t")
        if len(f) != len(PROTO_ADDITIONS_HEAD):
            sys.exit(f"{path}:{n}: {len(f)} columns, want {len(PROTO_ADDITIONS_HEAD)}")
        if not re.fullmatch(r"D-[0-9A-Z]+\.[0-9]+", f[-1]):
            sys.exit(f"{path}:{n}: {f[0]}: the decision column names no decision record (D-<section>.<n>)")
        rows.append(tuple(f[:-1]))
    return rows

def write_prototypes(ref):
    """prototypes.tsv: the reference rows `ref` (name, header, declaration), sorted by name, with each addition of
    prototypes-additions.tsv before the first row whose name sorts after its own. prototypes.sha256: the sha256 of
    the reference rows alone, rendered as write() renders them (header included), and their count."""
    ref = sorted(ref)
    ref_text = "\t".join(PROTO_HEAD) + "\n" + "".join("\t".join(r) + "\n" for r in ref)
    rows = list(ref)
    names = {r[0] for r in ref}
    for a in proto_additions():
        if a[0] in names:
            sys.exit(f"{PROTO_ADDITIONS}: {a[0]} is already a row of the reference (or repeated)")
        names.add(a[0])
        at = next((i for i, r in enumerate(rows) if r[0] > a[0]), len(rows))
        rows.insert(at, a)
    write("prototypes.tsv", PROTO_HEAD, rows)
    with open(os.path.join(OUT, "prototypes.sha256"), "w", newline="\n") as f:
        f.write(f"{hashlib.sha256(ref_text.encode()).hexdigest()}\t{len(ref)}\n")
    return rows

def reference_prototypes(tmp):
    """The reference's prototypes: the `_GNU_SOURCE` pass and the functions only other configurations declare
    (HEADERS-TABLES-COMPLETE), as (name, header, declaration) rows."""
    pro = prototypes(tmp)
    pro.update(prototypes_missing(tmp, pro))
    return [(k, *v) for k, v in pro.items()]

def exports_diff(syms):
    """Exports present on only one of x86_64 (versions.tsv) and this arch, by library and name."""
    norm = lambda lib: "ld-linux" if lib.startswith("ld-linux") else lib
    mine = {(norm(r[0]), r[1]) for r in syms}
    x86 = set()
    for line in open(os.path.join(OUT, "versions.tsv")).read().splitlines()[1:]:
        f = line.split("\t")
        x86.add((norm(f[0]), f[1]))
    return sorted([(l, s, "x86_64") for l, s in x86 - mine] + [(l, s, ARCH) for l, s in mine - x86])

def main_cross():
    # Package identity of the reference (RL-3): the first line of its Debian changelog (documentation).
    ch = os.path.join(SYSROOT, "usr/share/doc/libc6/changelog.Debian.gz")
    ver = sh(["sh", "-c", f"gzip -dc '{ch}' | head -1"]).strip() if os.path.exists(ch) else "(no changelog)"
    types, lay, ini = [], [], []
    layouts_note = ""
    with tempfile.TemporaryDirectory() as tmp:
        # The headers must compile as a set before any type is probed: a missing header (the
        # kernel UAPI asm/ headers are not part of libc6-dev) would otherwise drop every type
        # silently instead of failing.
        p = os.path.join(tmp, "pre.c"); open(p, "w").write(prelude())
        r = subprocess.run(CC + ["-std=gnu17", "-fsyntax-only", p], capture_output=True, text=True)
        if r.returncode != 0:
            first = next((l for l in r.stderr.splitlines() if "error" in l), r.stderr.strip())
            if "--symbols-only" not in sys.argv:
                sys.exit(f"the reference headers do not compile: {first}\n(--symbols-only writes the symbol tables alone)")
            layouts_note = f"Layouts and initializers NOT generated: the reference headers do not compile ({first.split(': ', 1)[-1]}).\n"
        elif "--symbols-only" in sys.argv:
            layouts_note = "Layouts and initializers not generated (--symbols-only).\n"
        else:
            types = available_types(tmp)
            szmap = sizes(tmp, types)
            lay, _ = layouts(tmp, types, szmap)
            write(f"layouts{SUFFIX}.tsv", ["type", "member", "offset_or_size", "align", "member_type", "bitfield"], lay)
            ini = initializers(tmp)
            write(f"initializers{SUFFIX}.tsv", ["type", "initializer", "size", "zero", "bytes_hex"], ini)
    syms = symbols()
    diff = exports_diff(write_versions(syms))
    write(f"exports-diff{SUFFIX}.tsv", ["library", "symbol", "only_on"], diff)
    oldest = sorted({r[2] for r in syms if r[2].startswith("GLIBC_2.")},
                    key=lambda v: [int(x) for x in v[6:].split(".")])[0]
    with open(os.path.join(OUT, f"PROVENANCE{SUFFIX}"), "w", newline="\n") as f:
        f.write(f"Generated by spec/tools/abi_extract.py --arch {ARCH} --sysroot <ref> (compile-only probes,\n"
                f"{CC[0]}; readelf of the object files and of the reference's shared objects).\n"
                f"Reference: Ubuntu arm64 libc6 + libc6-dev, unpacked, not installed (RL-3): {ver}\n"
                f"{layouts_note}Types: {len(types)}  layout rows: {len(lay)}  initializers: {len(ini)}\n"
                f"Versioned symbol rows: {len(syms)}  oldest version node: {oldest}  "
                f"exports on one arch only: {len(diff)}\n")
    print(open(os.path.join(OUT, f"PROVENANCE{SUFFIX}")).read())

def main():
    configure(sys.argv)
    if cross():
        return main_cross()
    os.makedirs(OUT, exist_ok=True)
    ver = sh(["ldd", "--version"]).splitlines()[0]
    if "--constants-only" in sys.argv:
        # Regenerate constants.tsv alone (the other tables are unchanged by the bisection fix).
        with tempfile.TemporaryDirectory() as tmp:
            mac = macros(tmp)
        write("constants.tsv", ["macro", "value"], mac)
        print(f"{ver}\nconstants: {len(mac)}")
        return
    if "--prototypes-only" in sys.argv or "--prototypes-missing" in sys.argv:
        # Regenerate prototypes.tsv (reference rows plus prototypes-additions.tsv) and prototypes.sha256 alone (slice
        # 7 added utime.h to HEADERS; the other tables do not depend on it). `--prototypes-missing` (which once added
        # only the rows of the functions other configurations declare to the committed table) is the same run now.
        with tempfile.TemporaryDirectory() as tmp:
            pro = reference_prototypes(tmp)
        rows = write_prototypes(pro)
        print(f"{ver}\nPrototypes: {len(pro)}  additions: {len(rows) - len(pro)}")
        return
    if "--empty-versions-only" in sys.argv:
        # Write empty-versions.tsv alone and record it in the story's provenance fragment (SYS-ABI-DT-RELR; D-X.172).
        sid = story(sys.argv)
        emp = empty_versions()
        write("empty-versions.tsv", ["library", "version", "parent"], emp)
        write_fragment(sid, f"empty-versions.tsv: abi_extract.py --empty-versions-only ({ver}): {len(emp)} version "
                            f"definitions no exported symbol carries.")
        print(f"{ver}\nempty versions: {len(emp)}")
        return
    if "--prototypes-fortify" in sys.argv:
        # The `__*_chk` declarations the headers make only under _FORTIFY_SOURCE (plan/headers.md slice 18; C.9
        # "Signatures per chk-prototypes.tsv"): the `_GNU_SOURCE` prelude compiled at -O2 with _FORTIFY_SOURCE=3,
        # the `-aux-info` lines of `__*_chk` names only, in their own table so prototypes.tsv and the tables keyed on
        # it (noexcept.tsv, callsites.tsv) keep their contents. Names GCC provides as builtins and the headers do
        # not declare (`__memcpy_chk`) get no row. The `__*_2` entries a call without its mode argument reaches
        # (`__open_2`, `__openat64_2`; spec 11.1 and 25.3, "fortify") are declared the same way and get rows too.
        with tempfile.TemporaryDirectory() as tmp:
            got = prototypes(tmp, None, ("-std=gnu17", "-O2", "-D_FORTIFY_SOURCE=3"))
        rows = sorted((k, *v) for k, v in got.items() if re.fullmatch(r"__\w+_chk|__\w+_2", k))
        write("chk-prototypes.tsv", ["name", "header", "declaration"], rows)
        print(f"{ver}\nchk prototypes: {len(rows)}")
        return
    if "--versions-only" in sys.argv:
        # Regenerate versions.tsv (reference rows plus versions-additions.tsv) and versions.sha256 alone
        # (SYS-LIBMVEC added libmvec.so.1 to LIBS; the other tables do not depend on the library list).
        syms = symbols()
        rows = write_versions(syms)
        print(f"{ver}\nversioned symbol rows: {len(syms)}  additions: {len(rows) - len(syms)}")
        return
    sid = None if "--layouts-only" in sys.argv else story(sys.argv)
    with tempfile.TemporaryDirectory() as tmp:
        types = available_types(tmp)
        szmap = sizes(tmp, types)
        lay, enums = layouts(tmp, types, szmap)
        write("layouts.tsv", ["type", "member", "offset_or_size", "align", "member_type", "bitfield"], lay)
        write("enumerators.tsv", ["tag", "name", "value"], enums)
        if "--layouts-only" in sys.argv:
            # Regenerate layouts.tsv and enumerators.tsv alone (both from the one DWARF probe; slice 3 added types,
            # HEADERS-TABLES-COMPLETE every depth, bit positions and the enumerators; the other tables are unchanged).
            print(f"{ver}\nTypes: {len(types)}  layout rows: {len(lay)}  enumerators: {len(enums)}")
            return
        ini = initializers(tmp)
        write("initializers.tsv", ["type", "initializer", "size", "zero", "bytes_hex"], ini)
        pro = reference_prototypes(tmp)
        write_prototypes(pro)
        mac = macros(tmp)
        write("constants.tsv", ["macro", "value"], mac)
    syms = symbols()
    write_versions(syms)
    emp = empty_versions()
    write("empty-versions.tsv", ["library", "version", "parent"], emp)
    write_fragment(sid, f"Full run of spec/tools/abi_extract.py on the reference system ({sid}).\n{ver}\n"
                        f"Types: {len(types)}  layout rows: {len(lay)}  initializers: {len(ini)}\n"
                        f"Prototypes: {len(pro)}  constants: {len(mac)}  versioned symbol rows: {len(syms)}  "
                        f"empty version definitions: {len(emp)}")

if __name__ == "__main__":
    main()
