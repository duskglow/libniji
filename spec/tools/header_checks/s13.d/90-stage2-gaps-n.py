"""Slice 13 checks: stage2-gaps-n (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md). Yohane's R-H7 gaps (HEADERS-STAGE2-GAPS-N; black-box probes of the reference, no header
text read): macro definition order as gcc's -fdump-go-spec reads it (libgo's mksysinfo.sh), the C cleanup functions
in C++ without exceptions (compiler-rt's tsan runtime), QCMD (systemd 257) and __cpu_mask (elogind 255).

Stand-alone against the build host's own C library headers (the expectations are the reference's, so they hold there
too; the macro-order sweep is libniji's rule, stricter than the reference, whose own headers define IP_RECVRETOPTS
before IP_RETOPTS and SSIZE_MAX before LONG_MAX, so it is skipped there): python3 spec/tools/header_checks/s13.d/90-stage2-gaps-n.py --reference
"""
import concurrent.futures
import os
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
from header_check import expand, kernel_flags  # noqa: E402

# quotactl(2)'s command word: values (quota.h's Q_* and *QUOTA), the int type (an unsigned operand makes it unsigned),
# full parenthesization, and use in #if.
QCMD_UNIT = """#include <sys/quota.h>
#if !defined QCMD || QCMD (1, 2) != 258
#error "QCMD missing or not usable in #if"
#endif
typedef char niji_gn_qcmd[QCMD (1 | 2, 3) == 771 && QCMD (1, 0x1ff) == 511 && QCMD (1, 2) * 2 == 516
  && (QCMD (1, 2) & 1) == 0 && sizeof (QCMD (1, 2)) == sizeof (int) && QCMD (1, 2) - 1000 < 0
  && QCMD (1u, 2) - 1000 > 0 && QCMD (0x100, PRJQUOTA) == 0x10002 ? 1 : -1];
"""
# The element type of cpu_set_t's bit array, unsigned long (C++: overloading tells it from unsigned long long).
CPU_MASK_C = "typedef char niji_gn_cm[sizeof (__cpu_mask) == sizeof (unsigned long) && (__cpu_mask) -1 > 0 ? 1 : -1];\n"
CPU_MASK_CXX = ("int niji_gn_f (unsigned long);\nchar niji_gn_f (unsigned long long);\n"
                "typedef char niji_gn_cm[sizeof (niji_gn_f ((__cpu_mask) 0)) == sizeof (int) ? 1 : -1];\n")
CANCEL = ("__pthread_register_cancel", "__pthread_unregister_cancel", "__pthread_unwind_next",
          "__pthread_register_cancel_defer", "__pthread_unregister_cancel_restore")
CLEANUP = """#include <pthread.h>
static void niji_gn_h (void *) {}
void niji_gn_f () { pthread_cleanup_push (niji_gn_h, 0); pthread_cleanup_pop (1); }
void niji_gn_g () { pthread_cleanup_push_defer_np (niji_gn_h, 0); pthread_cleanup_pop_restore_np (0); }
"""


def units(ctx):
    """No spec clause beyond quotactl(2) and the reference's headers (HEADERS-STAGE2-GAPS-N): (1) QCMD in <sys/quota.h>
    wherever the header compiles: values, int type, parenthesization, #if; (2) __cpu_mask, unsigned long, in each
    header that shows it. Defects: systemd's QCMD_FIXED not compiling, QCMD's operands regrouped or its type changed,
    __cpu_mask missing (elogind's test-sizeof.c) or another type."""
    shown = {}
    for r in ctx.vis:
        shown.setdefault((r["header"], r["name"]), set()).update(expand(r["configs"], ctx.ids))
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    out = [(cid, "GAPS-N QCMD", QCMD_UNIT) for cid in sorted(shown.get(("sys/quota.h", "QCMD"), ()))]
    for h in ("sched.h", "pthread.h", "spawn.h", "thread_db.h"):
        for cid in sorted(shown.get((h, "__cpu_mask"), ())):
            out.append((cid, f"GAPS-N {h} __cpu_mask", f"#include <{h}>\n" + (CPU_MASK_CXX if lang[cid] == "c++" else CPU_MASK_C)))
    return out


def _cmd(ctx, lang, flags, work):
    cc = "g++" if lang == "c++" else "gcc"
    cmd = [cc, *flags.split()]
    if ctx.inc:
        own = subprocess.run([cc, "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
        cmd += ["-nostdinc", "-isystem", ctx.inc, "-isystem", own, *kernel_flags(work)]
    return cmd


def _cc(ctx, lang, flags, body, work, extra=()):
    src = os.path.join(work, "gn.cc" if lang == "c++" else "gn.c")
    with open(src, "w") as f:
        f.write(body)
    r = subprocess.run(_cmd(ctx, lang, flags, work) + ["-fsyntax-only", *extra, src], capture_output=True, text=True,
                       timeout=300)
    return r.returncode, r.stderr


DEFINE = re.compile(r"^#define (\w+)(\()?(?:[^)]*\))? ?(.*)$")
IDENT = re.compile(r"[A-Za-z_]\w*")


MARKER = re.compile(r'^# \d+ "([^"]*)"')
KERNEL_DIRS = ("linux", "asm", "asm-generic", "sound", "mtd", "rdma", "scsi/fc", "video", "drm", "misc", "xen")


def own_file(path, inc):
    """A file of the C library's headers (not the kernel UAPI's, which both libraries take as they are)."""
    base = os.path.realpath(inc or "/usr/include")
    p = os.path.realpath(path)
    if not p.startswith(base + os.sep):
        if inc or not p.startswith("/usr/include/x86_64-linux-gnu/"):
            return False
        base = "/usr/include/x86_64-linux-gnu"
    rel = os.path.relpath(p, base)
    return not any(rel == d or rel.startswith(d + os.sep) for d in KERNEL_DIRS)


def order_problems(text, inc):
    """The object-like macros of `gcc -E -dD` output, defined in the C library's own headers, whose (last) definition
    names a macro that was not defined then and is an object-like macro later: gcc's -fdump-go-spec drops such a macro
    (it keeps a macro only when every name in its body is already one), so libgo's syscall package loses it (AF_INET
    as PF_INET before PF_INET)."""
    defined, pending, own, cache = set(), {}, False, {}
    for line in text.splitlines():
        mk = MARKER.match(line)
        if mk:
            f = mk.group(1)
            own = cache[f] if f in cache else cache.setdefault(f, not f.startswith("<") and own_file(f, inc))
            continue
        if line.startswith("#undef "):
            n = line.split()[1]
            defined.discard(n)
            pending.pop(n, None)
            continue
        m = DEFINE.match(line)
        if not m:
            continue
        name, fn, body = m.group(1), m.group(2), m.group(3)
        if fn:
            defined.discard(name)
            pending.pop(name, None)
            continue
        body = re.sub(r'"(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'', "", body)
        if own:
            pending[name] = {i for i in IDENT.findall(re.sub(r"\b\d\w*", "", body)) if i != name and i not in defined}
        else:
            pending.pop(name, None)
        defined.add(name)
    return sorted(f"{n} -> {', '.join(sorted(p & defined))}" for n, p in pending.items() if p & defined)


# The macro-order sweep: every public header alone, the usual feature-macro sets.
ORDER_FLAGS = ("-std=gnu17", "-std=gnu17 -D_GNU_SOURCE", "-std=c17", "-std=c17 -D_POSIX_C_SOURCE=200809L",
               "-std=c17 -D_XOPEN_SOURCE=700", "-std=c99 -D_DEFAULT_SOURCE -D_FILE_OFFSET_BITS=64")


def _headers(ctx):
    hs = {r["header"] for r in ctx.vis if r["header"] != "-" and not r["header"].startswith(("bits/", "niji/"))}
    return sorted(h for h in hs if os.path.isfile(os.path.join(ctx.inc or "/usr/include", h)))


def problems(ctx):
    """No spec clause beyond the reference's headers and gcc's -fdump-go-spec (HEADERS-STAGE2-GAPS-N): (1) no public
    header, alone, in six feature-macro sets, defines an object-like macro in terms of one defined after it; (2) the
    report's repro (<sys/socket.h>, <errno.h>, _GNU_SOURCE) dumps _AF_INET, _AF_PACKET and _ENOTSUP; (3) in C++ without
    exceptions <pthread.h> declares the five cleanup functions (C++98 .. 23) and the tsan pattern compiles; with
    exceptions it declares none (a program's own object of that name is its first). Defects: libgo's syscall package
    missing AF_* / ENOTSUP, compiler-rt's tsan_platform_linux.cpp not compiling, a declaration C++ with exceptions
    does not have in the reference."""
    out = []
    with tempfile.TemporaryDirectory(prefix="niji-gaps-n-") as work:
        jobs = []
        for flags in (ORDER_FLAGS if ctx.inc else ()):
            for h in _headers(ctx):
                src = os.path.join(work, h.replace("/", "_") + ".c")
                if not os.path.exists(src):
                    with open(src, "w") as f:
                        f.write(f"#include <{h}>\n")
                jobs.append((h, flags, _cmd(ctx, "c", flags, work) + ["-E", "-dD", src]))

        def run(job):
            h, flags, cmd = job
            r = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
            return h, flags, (order_problems(r.stdout, ctx.inc) if r.returncode == 0 else [])
        with concurrent.futures.ThreadPoolExecutor(max_workers=min(16, os.cpu_count() or 2)) as ex:
            for h, flags, bad in ex.map(run, jobs):
                out += [f"GAPS-N macro order [{h} {flags}]: {b}" for b in bad]
        src = os.path.join(work, "go.c")
        with open(src, "w") as f:
            f.write("#define _GNU_SOURCE\n#include <sys/socket.h>\n#include <errno.h>\n")
        go = os.path.join(work, "go.go")
        r = subprocess.run(_cmd(ctx, "c", "-std=gnu99", work) + [f"-fdump-go-spec={go}", "-c", src, "-o", os.devnull],
                           capture_output=True, text=True, timeout=300)
        dump = open(go).read() if r.returncode == 0 and os.path.exists(go) else ""
        for n in ("_AF_INET", "_AF_PACKET", "_ENOTSUP", "_PF_PACKET"):
            if not re.search(rf"^const {n} = ", dump, re.M):
                out.append(f"GAPS-N go dump: no `const {n}` (rc {r.returncode}): {r.stderr.strip()[:200]}")
        for std in ("c++98", "c++11", "c++17", "c++23"):
            for n in CANCEL:
                rc, err = _cc(ctx, "c++", f"-std={std} -fno-exceptions", f"#include <pthread.h>\nvoid *niji_gn_p = (void *) &{n};\n", work)
                if rc:
                    out.append(f"GAPS-N C++ -fno-exceptions {std}: {n} not declared: {err.strip()[:200]}")
                rc, err = _cc(ctx, "c++", f"-std={std}", f"#include <pthread.h>\nint {n};\n", work)
                if rc:
                    out.append(f"GAPS-N C++ with exceptions {std}: {n} declared: {err.strip()[:200]}")
            rc, err = _cc(ctx, "c++", f"-std={std} -fno-exceptions", CLEANUP, work, ("-Wall", "-Werror"))
            if rc:
                out.append(f"GAPS-N C++ -fno-exceptions {std}: cleanup macros: {err.strip()[:300]}")
    return out


if __name__ == "__main__":
    import types
    import csv
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "..")

    def tsv(rel):
        with open(os.path.join(root, rel), newline="") as f:
            return list(csv.DictReader((x for x in f if not x.startswith("#")), delimiter="\t"))
    cfgs = tsv("spec/abi/ftm-configs.tsv")
    ref = "--reference" in sys.argv
    ctx = types.SimpleNamespace(ids=[c["id"] for c in cfgs], cfgs=cfgs, vis=tsv("spec/abi/visibility.tsv"),
                                inc=None if ref else os.path.abspath(os.path.join(root, "libc-include")))
    by_id = {c["id"]: c for c in cfgs}
    rejected = {r["id"] for r in tsv("headers/levels.tsv") if r["reject"] == "1"}
    fails = problems(ctx)
    with tempfile.TemporaryDirectory(prefix="niji-gaps-n-u-") as w:
        for cid, what, body in units(ctx):
            if cid in rejected:
                continue
            c = by_id[cid]
            rc, err = _cc(ctx, c["language"], c["flags"] + (" -Wno-cpp" if cid in ("bsd", "svid") else ""), body, w)
            if rc:
                fails.append(f"{what} {cid}: rc {rc}: {' | '.join(err.strip().splitlines()[:3])[:300]}")
    for f in fails:
        print(f"FAIL {f}")
    print(f"gaps-n: {len(fails)} failed")
    sys.exit(1 if fails else 0)
