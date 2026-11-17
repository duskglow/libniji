# Hot-text tracer (PAGEFAULTS-2): run under `gdb -batch -x trace_gdb.py --args <static program> ...`.
# Puts a one-shot breakpoint on every function of OUR static binary, runs it, and writes the names of
# the functions that executed (every name at each hit address) to $HOT_TEXT_OUT, one per line, in
# first-touch order (HOT-ORDER: the list keeps that order and the link lays the group out by it).
# libniji products only (the binary is linked against libc.a); never point it at glibc.
#
# LAYOUT-TRACE-HOST-CANONICAL (LT-4): the trace is a function of the program, not of the host.
#  * CPU: the program never sees the host's CPU. Its two CPU entries (`niji_machine_cpuid`,
#    `niji_machine_xgetbv0`; every CPUID / XCR0 read of the library goes through them) are answered
#    by the tracer from a fixed virtual CPU, and the program runs once per CPU class (CLASSES below:
#    every combination of the two decisions that choose code, AVX2 and ERMS). The runs are merged into
#    one first-touch order (merge()): the first class's order, then every function only another class
#    ran, right after the function that class ran before it. So every candidate a class decision can
#    pick is in the list, whichever the build host would have picked.
#  * Environment: the program's environment is exactly the HOT_TEXT_ENV_<NAME>=<value> variables
#    given to gdb, as NAME=value in name order (nothing of the host's or gdb's own environment);
#    cwd `/`; address randomisation off; stdin $HOT_TEXT_STDIN, stdout /dev/null; the program runs
#    from a copy at a path of fixed length (argv[0]), so the work directory's path does not reach it.
#  HOT_TEXT_CLASSES (comma list of CLASSES names, default all) picks the classes; `host` alone runs the
#  host's own CPU (no virtual CPU: a diagnosis aid, never a list the build groups).
#
# TRACE-TIME-PIN: the trace is a function of the program, not of how fast it ran. Under gdb a program
# runs far slower than live, and the library times its once-a-second file checks (crates/legacy/src/
# recheck.rs: `now`, `due`, and the TSC gate in front of them): a real clock let a slow trace run a
# check the live program never runs (mktime's zone-file stat). So every time source the library
# reads is answered from one virtual clock (VCLOCK below), in every class (`host` too), and its own
# code never runs:
#  * `niji_machine_rdtsc` (the TSC gate's counter): a virtual counter, TSC_STEP ticks per read;
#  * the vDSO's clock_gettime, gettimeofday and time, at the library's vDSO call entries
#    (`niji_machine_vcall2` / `niji_machine_vcall3`, when their target is one of these: the other
#    vDSO calls, getcpu and clock_getres, run as they are);
#  * the system call fallbacks `niji_sys_clock_gettime`, `niji_sys_gettimeofday`, `niji_sys_time`.
#  Every clock read advances the virtual clock by CLOCK_STEP_NS (1 us): two reads always differ, and
#  a program's reads are far under the one-second interval apart, as they are live. The clock restarts
#  at every run of the program. HOT_TEXT_SLOW=<seconds> (the check in regen.sh) sleeps that long at
#  every time read: a source answered from the host's time then shows as a different trace.
#
# TRACE-NUMA-PIN: the trace is a function of the program, not of the host's NUMA topology. The heap
# (D-3.20) asks once how many nodes the process may allocate on and, when more than one, reads the
# calling thread's node at its slow paths (D-3.12, D-3.21) and places each shard's memory on its node.
# Every trace sees one virtual topology, in every class (`host` too): VNODES (2) allowed nodes, the
# program running on node 0.
#  * The node count: `get_mempolicy(MPOL_F_MEMS_ALLOWED)` at the library's generic system call entry
#    `niji_sys_syscall` (crates/boundary/src/raw.rs allowed_node_count, its one caller) is answered
#    with nodes 0..VNODES-1; the call never reaches the kernel. Other system calls run as they are.
#  * The node read: the rseq area's `node_id` (written by the kernel, no entry to answer at) or the
#    vDSO's getcpu. The program runs pinned to node 0's CPUs (pin_node0), so the kernel itself answers
#    0 on every host (a host without NUMA has one node, 0: nothing to pin). A program that moved
#    between nodes, or ran on node 1, would build another shard and trace other functions.
#  Why two nodes pinned to node 0 (measured, handoff/TRACE-NUMA-PIN.md): the layout traced so keeps
#  the NUMA path the 2-node build hosts run live in the start-up group, and a 1-node host runs a
#  subset of it, with no bound of hot_text_products exceeded on either.
#  HOT_TEXT_HOST_NODES=<n> (test-only; the check in regen.sh) simulates a host that allows n nodes:
#  the tracer answers the node count as that host would, and the virtual topology answers over it,
#  so the trace must not change.
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import time

import gdb

binp = gdb.current_progspace().filename
out_path = os.environ["HOT_TEXT_OUT"]
stdin_path = os.environ.get("HOT_TEXT_STDIN", "/dev/null")


def fail(msg):
    sys.stderr.write("trace_gdb: %s\n" % msg)
    sys.stderr.flush()
    os._exit(2)


# The virtual CPUs (Intel SDM vol. 2A, CPUID; bit numbers as crates/rt/src/cpu.rs reads them).
L1D_BASE = (1 << 0) | (1 << 4) | (1 << 8) | (1 << 15) | (1 << 23) | (1 << 24) | (1 << 25) | (1 << 26)  # FPU TSC CX8 CMOV MMX FXSR SSE SSE2
L1C_V3 = (1 << 0) | (1 << 9) | (1 << 12) | (1 << 13) | (1 << 19) | (1 << 20) | (1 << 22) | (1 << 23) | (1 << 26) | (1 << 27) | (1 << 28) | (1 << 29)
L7B_V3 = (1 << 3) | (1 << 5) | (1 << 8)  # BMI1 AVX2 BMI2
L7B_ERMS = 1 << 9
E1C_V3 = (1 << 0) | (1 << 5)  # LAHF LZCNT
E1D = (1 << 11) | (1 << 20) | (1 << 29)  # SYSCALL NX LM
# name -> (leaf 1 ECX, leaf 7 EBX, 80000001h ECX, XCR0, XSAVE area size)
CLASSES = {
    "v3-erms": (L1C_V3, L7B_V3 | L7B_ERMS, E1C_V3, 0x7, 832),
    "v3": (L1C_V3, L7B_V3, E1C_V3, 0x7, 832),
    "base-erms": (0, L7B_ERMS, 0, 0, 0),
    "base": (0, 0, 0, 0, 0),
}
# The host features a class's code paths execute (lscpu / /proc/cpuinfo flag names).
NEEDS = {"v3-erms": ("avx", "avx2", "fma", "bmi1", "bmi2", "movbe", "f16c", "abm"), "v3": ("avx", "avx2", "fma", "bmi1", "bmi2", "movbe", "f16c", "abm")}


def cpuid(cls, leaf, sub):
    l1c, l7b, e1c, xcr0, xsize = CLASSES[cls]
    if leaf == 0:
        return (0xD, 0x756E6547, 0x6C65746E, 0x49656E69)  # max leaf 0xd, "GenuineIntel"
    if leaf == 1:
        return (0x000306F0, 0x00010800, l1c, L1D_BASE)
    if leaf == 7 and sub == 0:
        return (0, l7b, 0, 0)
    if leaf == 0xD and sub == 0 and xcr0:
        return (xcr0, xsize, xsize, 0)
    if leaf == 0x80000000:
        return (0x80000008, 0, 0, 0)
    if leaf == 0x80000001:
        return (0, 0, e1c, E1D)
    return (0, 0, 0, 0)


classes = [c for c in os.environ.get("HOT_TEXT_CLASSES", ",".join(CLASSES)).split(",") if c]
virtual = classes != ["host"]
if virtual:
    for c in classes:
        if c not in CLASSES:
            fail("HOT_TEXT_CLASSES: unknown class %r (classes: %s, or host)" % (c, " ".join(CLASSES)))
    with open("/proc/cpuinfo") as f:
        flags = set(next((ln.split(":", 1)[1].split() for ln in f if ln.startswith("flags")), []))
    for c in classes:
        missing = [n for n in NEEDS.get(c, ()) if n not in flags]
        if missing:
            fail("class %s runs code this host cannot execute (missing %s): trace on an x86-64-v3 host" % (c, " ".join(missing)))

# The names at one address (aliases: `bcmp`, `__memcmpeq`, `niji_machine_equal`) are written in
# byte order (LAYOUT-TRACE-HOST-CANONICAL-3): nm sorts by the caller's locale (en_US ignores the
# underscores, C does not), so its order was a host input; nm runs in the C locale and the names are
# sorted here as well.
names = {}
nm_env = dict(os.environ, LC_ALL="C")
nm = subprocess.run(["nm", "--defined-only", binp], capture_output=True, text=True, check=True, env=nm_env).stdout
for line in nm.splitlines():
    p = line.split()
    if len(p) == 3 and p[1] in "tTwWiI":
        names.setdefault(int(p[0], 16), []).append(p[2])
names = {a: sorted(ns) for a, ns in names.items()}
by_name = {n: a for a, ns in names.items() for n in ns}
emul = {}
if virtual:
    for n, kind in (("niji_machine_cpuid", "cpuid"), ("niji_machine_xgetbv0", "xgetbv")):
        if n not in by_name:
            fail("%s: no %s: the virtual CPU cannot answer for it" % (binp, n))
        emul[by_name[n]] = kind
# TRACE-TIME-PIN: the library's time entries (header). A program that never reads the time links none.
TIME_ENTRIES = {"niji_machine_rdtsc": "rdtsc", "niji_machine_vcall2": "vcall", "niji_machine_vcall3": "vcall",
                "niji_sys_clock_gettime": "clock_gettime", "niji_sys_gettimeofday": "gettimeofday", "niji_sys_time": "time"}
clocks = {by_name[n]: kind for n, kind in TIME_ENTRIES.items() if n in by_name}
slow = float(os.environ.get("HOT_TEXT_SLOW", "0") or 0)
CLOCK_STEP_NS = 1000
TSC_STEP = 1000
MONO0 = 1000 * 10**9  # the monotonic clocks' start (any value above 0: 0 is the library's "never checked")
REALTIME0 = 1759579200 * 10**9  # the wall clock's start (the probes' own date, 2025-10-04)
REALTIME_IDS = (0, 5, 8, 11)  # CLOCK_REALTIME, _COARSE, _ALARM, CLOCK_TAI
MONO_IDS = (1, 2, 3, 4, 6, 7, 9)  # MONOTONIC, PROCESS/THREAD_CPUTIME, MONOTONIC_RAW, _COARSE, BOOTTIME, _ALARM
VCLOCK = {}
# TRACE-NUMA-PIN: the virtual topology (header). get_mempolicy is system call 239 on x86-64; the
# library passes it to `niji_sys_syscall(nr, a0..a5)`: nr in rdi, the mask in rdx, maxnode in rcx,
# the flags in r9.
TOPO_ENTRY = "niji_sys_syscall"
topo = {by_name[TOPO_ENTRY]} if TOPO_ENTRY in by_name else set()
NR_GET_MEMPOLICY = 239
MPOL_F_MEMS_ALLOWED = 4
VNODES = 2
VIRTUAL_NODES = True
HOST_NODES = os.environ.get("HOT_TEXT_HOST_NODES", "")
if HOST_NODES and not HOST_NODES.isdigit():
    fail("HOT_TEXT_HOST_NODES: want a node count, got %r" % HOST_NODES)


def nodes_answer():
    """The allowed-node count the program is told: the virtual topology's. Without it, the host's:
    the simulated host's (HOT_TEXT_HOST_NODES), else None (the kernel answers)."""
    host = int(HOST_NODES) if HOST_NODES else None
    return VNODES if VIRTUAL_NODES else host


def answer_nodes():
    """At `niji_sys_syscall`'s stop: answer get_mempolicy(MPOL_F_MEMS_ALLOWED) (header); any other
    call runs."""
    if reg("rdi") != NR_GET_MEMPOLICY or not reg("r9") & MPOL_F_MEMS_ALLOWED:
        return
    n = nodes_answer()
    if n is None:
        return
    mask, maxnode = reg("rdx"), reg("rcx")
    words = (maxnode - 1 + 63) // 64  # the kernel writes maxnode - 1 bits, in whole longs
    if n >= 64 or words < 1 or words > 1024:
        fail("get_mempolicy: a %d-node answer does not fit a mask of %d bits" % (n, maxnode - 1))
    if mask:
        gdb.selected_inferior().write_memory(mask, struct.pack("<%dQ" % words, (1 << n) - 1, *([0] * (words - 1))))
    gdb.execute("set $rax = 0")
    leave_entry()
    TOPO_ANSWERS[0] += 1


TOPO_ANSWERS = [0]  # node counts answered, all runs (printed at the end: regen.sh checks start-up asked one)


def node0_cpus():
    """The CPUs of node 0 this process may run on, or None on a host without NUMA (one node)."""
    try:
        with open("/sys/devices/system/node/node0/cpulist") as f:
            text = f.read().strip()
    except FileNotFoundError:
        return None
    cpus = set()
    for part in filter(None, text.split(",")):
        lo, _, hi = part.partition("-")
        cpus.update(range(int(lo), int(hi or lo) + 1))
    cpus &= os.sched_getaffinity(0)
    if not cpus:
        fail("node 0 has no CPU this process may run on (%r): the trace cannot run on node 0" % text)
    return cpus


NODE0 = node0_cpus()


def pin_node0(pid):
    """Pin the program (its threads and children inherit it) to node 0's CPUs (header)."""
    if NODE0 is not None and pid:
        os.sched_setaffinity(pid, NODE0)

# The program's copy at a fixed-length path (argv[0] is the path gdb runs).
stage = tempfile.mkdtemp(prefix="niji-trace.", dir="/tmp")
prog = os.path.join(stage, "p")
shutil.copy(binp, prog)
env = sorted((k[len("HOT_TEXT_ENV_"):], v) for k, v in os.environ.items() if k.startswith("HOT_TEXT_ENV_"))
for cmd in ["set confirm off", "set pagination off", "set breakpoint always-inserted on", "set startup-with-shell off",
            "set disable-randomization on", "file " + prog, "set cwd /", "unset environment"] + \
           ["set environment %s=%s" % kv for kv in env]:
    gdb.execute(cmd, to_string=True)
# HOT_TEXT_FOLLOW_FORK=child (TIMING-COV-FORK-COW-FAULTS): trace the parent up to its first fork,
# then the child to its exit. `fork` does not copy the page tables of file-backed text, so a fork
# child faults on every 64 KiB text window its path touches; the parent's later path is not traced.
if os.environ.get("HOT_TEXT_FOLLOW_FORK") == "child":
    gdb.execute("set follow-fork-mode child")
# With startup-with-shell off, gdb ignores `run < file > file` (HOT-ORDER-3: p_cat read an empty stdin
# and never reached fwrite), so the inferior inherits gdb's own stdin/stdout: point them at the files.
for fd, path, mode in ((0, stdin_path, os.O_RDONLY), (1, "/dev/null", os.O_WRONLY)):
    f = os.open(path, mode)
    os.dup2(f, fd)
    os.close(f)


def reg(name):
    return int(gdb.selected_frame().read_register(name)) & 0xFFFFFFFFFFFFFFFF


def answer(kind, cls):
    """Return from the CPU entry at pc with the virtual CPU's answer (its own code never runs)."""
    inf = gdb.selected_inferior()
    if kind == "cpuid":
        regs = cpuid(cls, reg("rdi") & 0xFFFFFFFF, reg("rsi") & 0xFFFFFFFF)
        inf.write_memory(reg("rdx"), struct.pack("<4I", *regs))
    else:
        gdb.execute("set $rax = %d" % CLASSES[cls][3])
    leave_entry()


def leave_entry():
    """Return from the entry at pc to its caller (the entry's own code never runs)."""
    inf = gdb.selected_inferior()
    sp = reg("rsp")
    ret = struct.unpack("<Q", bytes(inf.read_memory(sp, 8)))[0]
    gdb.execute("set $rsp = %d" % (sp + 8))
    gdb.execute("set $pc = %d" % ret)


def vdso_time_fns():
    """The vDSO's clock_gettime, gettimeofday and time: {address: kind} in this run's process."""
    if "vdso" in VCLOCK:
        return VCLOCK["vdso"]
    inf, fns = gdb.selected_inferior(), {}
    with open("/proc/%d/maps" % inf.pid) as f:
        span = next((ln.split()[0] for ln in f if ln.rstrip().endswith("[vdso]")), None)
    if span is not None:
        lo, hi = (int(x, 16) for x in span.split("-"))
        img = bytes(inf.read_memory(lo, hi - lo))
        phoff, phent, phnum = struct.unpack_from("<Q", img, 0x20)[0], *struct.unpack_from("<HH", img, 0x36)
        loads = [struct.unpack_from("<IIQQ", img, phoff + i * phent) for i in range(phnum)]
        vaddr = min(p[3] for p in loads if p[0] == 1)  # PT_LOAD
        with tempfile.NamedTemporaryFile(prefix="niji-vdso.", dir="/tmp") as t:
            t.write(img)
            t.flush()
            out = subprocess.run(["nm", "-D", "--defined-only", t.name], capture_output=True, text=True, check=True, env=nm_env).stdout
        for line in out.splitlines():
            p = line.split()
            name = p[-1].split("@")[0].removeprefix("__vdso_")  # nm -D writes the version: clock_gettime@@LINUX_2.6
            if len(p) == 3 and name in ("clock_gettime", "gettimeofday", "time"):
                fns[lo - vaddr + int(p[0], 16)] = name
        if not fns:
            fail("the vDSO at %s exports no clock_gettime, gettimeofday or time: the virtual clock cannot find them" % span)
    VCLOCK["vdso"] = fns
    return fns


def time_read(kind):
    """At a time entry's stop: (kind, args) when this call reads the time, else None (a vDSO call
    entry calling another vDSO function)."""
    if kind == "vcall":
        kind = vdso_time_fns().get(reg("rdi"))
        return None if kind is None else (kind, (reg("rsi"), reg("rdx")))
    return kind, (reg("rdi"), reg("rsi"))


def virtual_ns(clock):
    """The virtual clock `clock` (a clockid_t) after one more read, or None for an id it has not."""
    clock = clock & 0xFFFFFFFF
    if clock not in REALTIME_IDS and clock not in MONO_IDS:
        return None
    VCLOCK["ns"] += CLOCK_STEP_NS
    return VCLOCK["ns"] + (REALTIME0 if clock in REALTIME_IDS else MONO0)


def answer_time(kind, args):
    """Return from a time entry with the virtual clock's answer (header)."""
    inf = gdb.selected_inferior()
    if kind == "rdtsc":
        VCLOCK["tsc"] += TSC_STEP
        gdb.execute("set $rax = %d" % VCLOCK["tsc"])
    elif kind == "clock_gettime":
        ns = virtual_ns(args[0])
        if ns is None:
            return  # an id the virtual clock has not: the call runs (EINVAL)
        if args[1]:
            inf.write_memory(args[1], struct.pack("<qq", ns // 10**9, ns % 10**9))
        gdb.execute("set $rax = 0")
    elif kind == "gettimeofday":
        ns = virtual_ns(0)
        if args[0]:
            inf.write_memory(args[0], struct.pack("<qq", ns // 10**9, ns % 10**9 // 1000))
        if args[1]:
            inf.write_memory(args[1], struct.pack("<ii", 0, 0))
        gdb.execute("set $rax = 0")
    else:  # time
        sec = virtual_ns(0) // 10**9
        if args[0]:
            inf.write_memory(args[0], struct.pack("<q", sec))
        gdb.execute("set $rax = %d" % sec)
    leave_entry()


# HOT_TEXT_REDIRECT (canon_host.sh): `<host path>=<file>` joined by `:`. At the entry of each path
# system call below the tracer points the path argument at the file (copied below the stack pointer).
# A file `@run/<name>` is <name> in a directory the tracer empties before every run of the program
# (ruling #5223: the name-service cache region, so every run creates it, whatever the host's
# /dev/shm holds).
redirect = {}
run_dir = os.path.join(stage, "run")  # same length on every host: stage is mkdtemp's fixed-length name
for item in filter(None, os.environ.get("HOT_TEXT_REDIRECT", "").split(":")):
    k, _, v = item.partition("=")
    if v.startswith("@run/"):
        v = os.path.join(run_dir, v[len("@run/"):])
    redirect[k.encode()] = v.encode() + b"\0"
# open stat lstat access unlink openat newfstatat unlinkat faccessat statx faccessat2: the path argument's
# register (unlink too, so no failure path removes a host file the redirected open did not create).
PATH_ARG = {2: "rdi", 4: "rdi", 6: "rdi", 21: "rdi", 87: "rdi", 257: "rsi", 262: "rsi", 263: "rsi", 269: "rsi", 332: "rsi", 439: "rsi"}
NOT_SYSCALL = (1 << 64) - 1  # %orig_rax at a stop that is not a system call's (a breakpoint's trap)
CATCH = "catch syscall open stat lstat access unlink openat newfstatat unlinkat faccessat statx faccessat2"


# Host files a traced program may read besides the redirected ones: the residual inputs canon_host.sh
# prints digests of (message catalogs and locale.alias, zone files), the kernel's own files, and the
# program's scratch files. Any other path a probe opens, stats or unlinks (absent ones included: a
# host that has one runs another path) is host state the list would follow: the trace fails.
HOST_OK = (b"/usr/share/locale/", b"/usr/share/locale-langpack/", b"/usr/share/zoneinfo/", b"/proc/", b"/sys/", b"/dev/null", b"/dev/urandom", b"/tmp/", run_dir.encode() + b"/")
host_reads = set()


def read_path(addr):
    """The NUL-terminated path at addr, read a page at a time (never past the page it ends in)."""
    inf, p = gdb.selected_inferior(), b""
    while len(p) < 4096:
        n = 4096 - (addr + len(p)) % 4096
        part = bytes(inf.read_memory(addr + len(p), n))
        if b"\0" in part:
            return p + part.split(b"\0", 1)[0]
        p += part
    return p


def redirect_path():
    """At a path system call's entry stop: point its path at the tree's file when redirected."""
    r = PATH_ARG.get(reg("orig_rax"))
    if r is None:
        return
    inf = gdb.selected_inferior()
    p = read_path(reg(r))
    if p not in redirect:
        full = p if p.startswith(b"/") else b"/" + p  # cwd is /
        if not full.startswith(HOST_OK):
            host_reads.add(full.decode(errors="replace"))
        return
    new = redirect[p]
    at = (reg("rsp") - 4096 - len(new)) & ~15  # below the red zone, in the mapped stack
    inf.write_memory(at, new)
    gdb.execute("set $%s = %d" % (r, at))


def run_one(cls):
    """One run of the program on class `cls`: the function start addresses in first-touch order."""
    made, once = [], {}
    VCLOCK.clear()
    VCLOCK.update(ns=0, tsc=1 << 32)
    every = set(clocks) | topo | (set(emul) if cls != "host" else set())  # entries answered at every call
    for addr in names:
        if addr in every:
            continue
        bp = gdb.Breakpoint("*0x%x" % addr, internal=True, temporary=True)
        bp.silent = True
        made.append(bp)
        once[addr] = bp
    for addr in sorted(every):
        bp = gdb.Breakpoint("*0x%x" % addr, internal=True)
        bp.silent = True
        made.append(bp)
    if redirect:
        shutil.rmtree(run_dir, ignore_errors=True)
        os.mkdir(run_dir)
        gdb.execute(CATCH, to_string=True)
    hit, seen, in_call = [], set(), set()
    gdb.execute("run", to_string=True)
    # The first stop is the program's first function (every function has a breakpoint), before its
    # first node read: from here on it runs on node 0 (TRACE-NUMA-PIN).
    pin_node0(gdb.selected_inferior().pid)
    while True:
        try:
            pc = int(gdb.selected_frame().pc())
        except gdb.error:
            break
        # A system call stop (the catchpoint) is told from a breakpoint by %orig_rax (the call's number
        # at a system call stop, -1 at a breakpoint's trap), never by the pc: the instruction after a
        # `syscall` can be a function's address (a label such as niji_machine_cancel_window_end)
        # whose one-shot breakpoint has not run yet. Entry and return stops alternate per thread.
        tid = gdb.selected_thread().ptid
        if redirect and reg("orig_rax") != NOT_SYSCALL and tid not in in_call:
            in_call.add(tid)
            redirect_path()
            bp = once.pop(pc, None)
            if bp is not None and bp.is_valid():
                # gdb cannot resume a system call stop over a breakpoint at its pc (the program ended
                # there: p_cold's first cancellable open): drop it, count the function at the return.
                bp.delete()
        elif redirect and tid in in_call:
            in_call.discard(tid)
            if pc in names and pc not in seen and pc not in once:
                seen.add(pc)
                hit.append(pc)
        elif pc in names and pc not in seen:  # one-shot breakpoints stop once each; the CPU entries at every call
            seen.add(pc)
            hit.append(pc)
        if pc in emul and cls != "host":
            answer(emul[pc], cls)
        elif pc in clocks and (not redirect or reg("orig_rax") == NOT_SYSCALL):  # its breakpoint, not a system call stop
            read = time_read(clocks[pc])
            if read is not None:
                if slow:
                    time.sleep(slow)
                answer_time(*read)
        elif pc in topo and (not redirect or reg("orig_rax") == NOT_SYSCALL):  # its breakpoint, not a system call stop
            answer_nodes()
        try:
            gdb.execute("continue", to_string=True)
        except gdb.error:
            break
    for bp in made:
        if bp.is_valid():
            bp.delete()
    for bp in gdb.breakpoints() or ():  # the catchpoint
        bp.delete()
    return hit


def merge(runs):
    """The first run's order; a function only a later run touched goes right after the function that
    run touched before it (or first, when it is that run's first), in that run's order."""
    order = list(runs[0])
    at = {a: i for i, a in enumerate(order)}
    for run in runs[1:]:
        prev = None
        for a in run:
            if a not in at:
                i = 0 if prev is None else order.index(prev) + 1
                order.insert(i, a)
                at = {x: k for k, x in enumerate(order)}
            prev = a
    return order


runs = [run_one(c) for c in classes]
sys.stderr.write("trace_gdb: node count answered %d times\n" % TOPO_ANSWERS[0])
shutil.rmtree(stage, ignore_errors=True)
if host_reads:
    fail("the program read host files the tree does not supply (canon_host.sh): %s" % " ".join(sorted(host_reads)))
# The virtual CPU reached the program: a class with AVX2 and one without never run the same code
# (start-up reads XCR0 only when CPUID shows OSXSAVE and AVX). Equal runs mean the program saw one CPU,
# the host's, and the list would follow it.
if virtual:
    for a in classes:
        for b in classes:
            if CLASSES[a][1] & L7B_V3 and not CLASSES[b][1] & L7B_V3 and runs[classes.index(a)] == runs[classes.index(b)]:
                fail("classes %s and %s ran the same functions: the program did not see the virtual CPU" % (a, b))
with open(out_path, "w") as f:
    for addr in merge(runs):
        for n in names.get(addr, []):
            f.write(n + "\n")
